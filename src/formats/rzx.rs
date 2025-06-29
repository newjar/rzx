#[cfg(unix)]
use crc32fast::Hasher;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{File, OpenOptions, Permissions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::core::header::RzxHeader;
use crate::core::metadata::{ArchiveMetadata, FileEntry};
use crate::core::{
    CompressionAlgorithm, EncryptionAlgorithm, RzxError, RzxResult, NONCE_SIZE, RZX_MAGIC,
    RZX_VERSION, SALT_SIZE,
};

/// RZX archive writer
pub struct RzxWriter<W: Write + Seek> {
    writer: W,
    metadata: ArchiveMetadata,
    current_data_offset: u64,
    encryption_algorithm: EncryptionAlgorithm,
    encryption_key: Option<[u8; 32]>,
    salt: Option<[u8; 16]>,
    nonce: Option<[u8; 12]>,
}

impl<W: Write + Seek> RzxWriter<W> {
    /// Create a new RZX writer
    pub fn new(
        mut writer: W,
        encryption_algorithm: EncryptionAlgorithm,
        password: Option<&str>,
    ) -> RzxResult<Self> {
        let (salt, nonce, encryption_key) = if encryption_algorithm != EncryptionAlgorithm::None {
            let salt = crate::core::encryption::generate_salt();
            let nonce = crate::core::encryption::generate_nonce();
            let key = crate::core::encryption::derive_key(password.unwrap_or("").as_bytes(), &salt);
            (Some(salt), Some(nonce), Some(key))
        } else {
            (None, None, None)
        };

        // Write a placeholder header
        let header = RzxHeader::new(encryption_algorithm, salt, nonce, 0, 0, 0);
        writer.write_all(&header.to_bytes()?)?;

        Ok(Self {
            writer,
            metadata: ArchiveMetadata::new(),
            current_data_offset: 0,
            encryption_algorithm,
            encryption_key,
            salt,
            nonce,
        })
    }

    /// Add an entry to the archive (for sequential writing)
    pub fn add_entry_sequential(
        &mut self,
        mut file_entry: FileEntry,
        absolute_path: &Path,
        compression_level: u8,
    ) -> RzxResult<()> {
        if file_entry.is_directory {
            self.metadata.add_entry(file_entry);
            return Ok(());
        }

        let mut file = std::io::BufReader::new(File::open(absolute_path)?);
        let mut hasher = Hasher::new();
        let mut buffer = [0; 4096]; // Use a small buffer for reading chunks

        // Calculate checksum by reading in chunks
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        file_entry.checksum = hasher.finalize();

        // Rewind the file to the beginning for compression
        file.seek(SeekFrom::Start(0))?;

        file_entry.original_size = file.get_ref().metadata()?.len();
        file_entry.data_offset = self.current_data_offset;

        let mut compressed_buffer = Vec::new();
        crate::core::compression::compress(
            file_entry.compression,
            &mut file,
            &mut compressed_buffer,
            compression_level,
        )?;

        let mut encrypted_buffer = Vec::new();
        if self.encryption_algorithm != EncryptionAlgorithm::None {
            crate::core::encryption::encrypt::<&[u8], Vec<u8>>(
                self.encryption_algorithm,
                self.encryption_key.as_ref().unwrap(),
                self.nonce.as_ref().unwrap(),
                &mut compressed_buffer.as_ref(),
                &mut encrypted_buffer,
            )?;
        } else {
            encrypted_buffer = compressed_buffer;
        }

        file_entry.compressed_size = encrypted_buffer.len() as u64;
        self.writer.write_all(&encrypted_buffer)?;
        self.current_data_offset += encrypted_buffer.len() as u64;

        self.metadata.add_entry(file_entry);

        Ok(())
    }

    /// Finalize the archive writing
    pub fn finalize(&mut self) -> RzxResult<()> {
        self.metadata.sort_entries();
        self.metadata.validate()?;

        let metadata_bytes = self.metadata.to_bytes()?;
        let metadata_size = metadata_bytes.len() as u64;

        let mut hasher = Hasher::new();
        hasher.update(&metadata_bytes);
        let metadata_checksum = hasher.finalize();

        let metadata_offset = self.writer.seek(SeekFrom::Current(0))?;

        self.writer.write_all(&metadata_bytes)?;

        let header = RzxHeader::new(
            self.encryption_algorithm,
            self.salt,
            self.nonce,
            metadata_offset,
            metadata_size,
            metadata_checksum,
        );
        self.writer.seek(SeekFrom::Start(0))?;
        self.writer.write_all(&header.to_bytes()?)?;

        Ok(())
    }
}

/// RZX archive reader
pub struct RzxReader<R: Read + Seek> {
    reader: R,
    header: RzxHeader,
    metadata: ArchiveMetadata,
    encryption_key: Option<[u8; 32]>,
    data_section_start_offset: u64,
}

impl<R: Read + Seek> RzxReader<R> {
    /// Create a new RZX reader
    pub fn new(mut reader: R, password: Option<&str>) -> RzxResult<Self> {
        let mut magic_bytes = [0; 4];
        reader.read_exact(&mut magic_bytes)?;
        if magic_bytes != RZX_MAGIC {
            return Err(RzxError::InvalidMagic);
        }

        let mut version_bytes = [0; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version > RZX_VERSION {
            return Err(RzxError::UnsupportedVersion(version));
        }

        let mut encryption_algorithm_byte = [0; 1];
        reader.read_exact(&mut encryption_algorithm_byte)?;
        let encryption_algorithm = EncryptionAlgorithm::from_u8(encryption_algorithm_byte[0])
            .ok_or(RzxError::CorruptedArchive(
                "Invalid encryption algorithm byte".to_string(),
            ))?;

        let salt = if encryption_algorithm != EncryptionAlgorithm::None {
            let mut salt_tag = [0u8; 1];
            reader.read_exact(&mut salt_tag)?;
            if salt_tag[0] != 1 {
                return Err(RzxError::CorruptedArchive(format!("Invalid salt option tag for encrypted archive: expected 1, got {}", salt_tag[0])));
            }
            let mut salt_bytes = [0; SALT_SIZE];
            reader.read_exact(&mut salt_bytes)?;
            Some(salt_bytes)
        } else {
            let mut salt_tag = [0u8; 1];
            reader.read_exact(&mut salt_tag)?; // Read the None tag
            if salt_tag[0] != 0 {
                return Err(RzxError::CorruptedArchive(format!("Invalid salt option tag for unencrypted archive: expected 0, got {}", salt_tag[0])));
            }
            None
        };

        let nonce = if encryption_algorithm != EncryptionAlgorithm::None {
            let mut nonce_tag = [0u8; 1];
            reader.read_exact(&mut nonce_tag)?;
            if nonce_tag[0] != 1 {
                return Err(RzxError::CorruptedArchive(format!("Invalid nonce option tag for encrypted archive: expected 1, got {}", nonce_tag[0])));
            }
            let mut nonce_bytes = [0; NONCE_SIZE];
            reader.read_exact(&mut nonce_bytes)?;
            Some(nonce_bytes)
        } else {
            let mut nonce_tag = [0u8; 1];
            reader.read_exact(&mut nonce_tag)?; // Read the None tag
            if nonce_tag[0] != 0 {
                return Err(RzxError::CorruptedArchive(format!("Invalid nonce option tag for unencrypted archive: expected 0, got {}", nonce_tag[0])));
            }
            None
        };

        let encryption_key = if encryption_algorithm != EncryptionAlgorithm::None {
            let p = password.ok_or(RzxError::InvalidPassword)?;
            Some(crate::core::encryption::derive_key(
                p.as_bytes(),
                salt.as_ref().unwrap(),
            ))
        } else {
            None
        };

        let mut metadata_offset_bytes = [0; 8];
        reader.read_exact(&mut metadata_offset_bytes)?;
        let metadata_offset = u64::from_le_bytes(metadata_offset_bytes);

        let mut metadata_size_bytes = [0; 8];
        reader.read_exact(&mut metadata_size_bytes)?;
        let metadata_size = u64::from_le_bytes(metadata_size_bytes);

        let mut metadata_checksum_bytes = [0; 4];
        reader.read_exact(&mut metadata_checksum_bytes)?;
        let metadata_checksum = u32::from_le_bytes(metadata_checksum_bytes);

        let data_section_start_offset = reader.seek(SeekFrom::Current(0))?;

        let header = RzxHeader::new(
            encryption_algorithm,
            salt,
            nonce,
            metadata_offset,
            metadata_size,
            metadata_checksum,
        );

        // Debugging the metadata seek
        println!(
            "Debug RzxReader::new: header.metadata_offset = {}, header.metadata_size = {}",
            header.metadata_offset, header.metadata_size
        );
        let current_pos_before_meta_seek_info = reader.seek(SeekFrom::Current(0))?;
        let file_size_check = reader.seek(SeekFrom::End(0))?;
        println!(
            "Debug RzxReader::new: Archive current pos before meta seek info: {}, total size for check = {}",
            current_pos_before_meta_seek_info, file_size_check
        );
        // Restore position to where data_section_start_offset was determined,
        // as this is the logical end of fixed header before variable metadata seek.
        reader.seek(SeekFrom::Start(data_section_start_offset))?;


        if header.metadata_offset + header.metadata_size > file_size_check {
            eprintln!(
                "Error: Metadata section out of bounds. Offset: {}, Size: {}, ArchiveSize: {}",
                header.metadata_offset, header.metadata_size, file_size_check
            );
            return Err(RzxError::CorruptedArchive("Metadata section out of bounds".to_string()));
        }

        reader.seek(SeekFrom::Start(header.metadata_offset))?;
        println!("Debug RzxReader::new: Successfully seeked to metadata_offset: {}", header.metadata_offset);
        let mut metadata_bytes = vec![0; header.metadata_size as usize];
        reader.read_exact(&mut metadata_bytes)?;
        println!("Debug RzxReader::new: Successfully read {} metadata_bytes", metadata_bytes.len());

        let mut decrypted_metadata_bytes = Vec::new();
        if encryption_algorithm != EncryptionAlgorithm::None {
            crate::core::encryption::decrypt::<&[u8], Vec<u8>>(
                encryption_algorithm,
                encryption_key.as_ref().unwrap(),
                nonce.as_ref().unwrap(),
                &mut metadata_bytes.as_ref(),
                &mut decrypted_metadata_bytes,
            )?;
        } else {
            decrypted_metadata_bytes = metadata_bytes;
        }

        let mut hasher = Hasher::new();
        hasher.update(&decrypted_metadata_bytes);
        if hasher.finalize() != header.metadata_checksum {
            return Err(RzxError::ChecksumMismatch);
        }

        let metadata = ArchiveMetadata::from_bytes(&decrypted_metadata_bytes)?;

        Ok(Self {
            reader,
            header,
            metadata,
            encryption_key,
            data_section_start_offset,
        })
    }

    /// Get archive metadata
    pub fn metadata(&self) -> &ArchiveMetadata {
        &self.metadata
    }

    /// Extract a file from the archive
    pub fn extract_file(&mut self, entry: &FileEntry, output_path: &Path) -> RzxResult<()> {
        if entry.is_directory {
            std::fs::create_dir_all(output_path)?;
            return Ok(());
        }

        let total_archive_size = self.reader.seek(SeekFrom::End(0))?;
        self.reader.seek(SeekFrom::Start(0))?; // Reset seek to start for relative calculations if any part of reader needs it. Or rather, restore position before this block.
                                               // Better to record original position if needed, but for this specific function, we are about to seek to data.

        let absolute_target_offset = self.data_section_start_offset + entry.data_offset;
        println!(
            "Debug extract_file: data_section_start_offset: {}, entry.data_offset: {}, absolute_target_offset: {}",
            self.data_section_start_offset, entry.data_offset, absolute_target_offset
        );
        println!(
            "Debug extract_file: entry.compressed_size: {}, total_archive_size: {}",
            entry.compressed_size, total_archive_size
        );

        if absolute_target_offset + entry.compressed_size > total_archive_size {
            eprintln!(
                "Error: Read would go out of bounds. Offset: {}, CompressedSize: {}, ArchiveSize: {}",
                absolute_target_offset, entry.compressed_size, total_archive_size
            );
            return Err(RzxError::CorruptedArchive("File entry data out of bounds".to_string()));
        }

        let original_pos_before_seek = self.reader.seek(SeekFrom::Current(0))?; // Should be after header read, before metadata seek for this reader instance
        self.reader.seek(SeekFrom::Start(absolute_target_offset))?;
        let pos_after_seek = self.reader.seek(SeekFrom::Current(0))?;
        println!("Debug extract_file: Original pos: {}, Positioned to {} for entry data", original_pos_before_seek, pos_after_seek);


        // Test read a single byte
        // let mut test_byte = [0u8; 1];
        // match self.reader.read_exact(&mut test_byte) {
        //     Ok(_) => println!("Successfully read a test byte: {:?}", test_byte),
        //     Err(e) => {
        //         eprintln!("Failed to read test byte: {}", e);
        //         return Err(RzxError::Io(e));
        //     }
        // }
        // self.reader.seek(SeekFrom::Start(pos_after_seek))?; // Seek back

        let mut encrypted_data = Vec::new();
        let mut limited_reader = self.reader.by_ref().take(entry.compressed_size);
        limited_reader.read_to_end(&mut encrypted_data)?;

        let mut compressed_data = Vec::new();
        println!("Reader: Encrypted data size before decryption: {}", encrypted_data.len());
        if self.header.encryption_algorithm != EncryptionAlgorithm::None {
            crate::core::encryption::decrypt::<&[u8], Vec<u8>>(
                self.header.encryption_algorithm,
                self.encryption_key.as_ref().unwrap(),
                self.header.nonce.as_ref().unwrap(),
                &mut encrypted_data.as_ref(),
                &mut compressed_data,
            )?;
        } else {
            compressed_data = encrypted_data;
        }
        println!("Reader: Compressed data size after decryption: {}", compressed_data.len());

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut input_stream = std::io::BufReader::<&[u8]>::new(compressed_data.as_ref());
        {
            let mut output_file = File::create(output_path)?;
            crate::core::compression::decompress(
                entry.compression,
                &mut input_stream,
                &mut output_file,
            )?;
            // output_file is dropped here, ensuring it's flushed and closed.
        }

        // Re-calculate checksum to verify integrity after decompression
        let mut decompressed_data = Vec::new();
        let mut temp_file = File::open(output_path)?;
        temp_file.read_to_end(&mut decompressed_data)?;

        let calculated_checksum = {
            let mut hasher = Hasher::new();
            hasher.update(&decompressed_data);
            hasher.finalize()
        };

        if calculated_checksum != entry.checksum {
            // If checksum fails, it's useful to know what the actual checksum was
            eprintln!("Checksum mismatch for {:?}: expected {:x}, got {:x}", output_path, entry.checksum, calculated_checksum);
            return Err(RzxError::ChecksumMismatch);
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = Permissions::from_mode(entry.permissions);
            std::fs::set_permissions(output_path, permissions)?;
        }

        Ok(())
    }
}

/// Create a new RZX archive
pub fn create_archive(
    output_path: &Path,
    input_paths: &[PathBuf],
    compression_level: u8,
    algorithm: &str,
    encryption_algorithm: EncryptionAlgorithm,
    password: Option<&str>,
    exclude: &[String],
) -> RzxResult<()> {
    let _selected_algorithm = CompressionAlgorithm::from_str(algorithm)
        .ok_or_else(|| RzxError::UnsupportedCompression(0))?;

    let base_path = input_paths[0].parent().unwrap_or(Path::new("/"));

    let mut files_to_process: Vec<PathBuf> = Vec::new();
    for path in input_paths {
        if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let entry_path = entry.path().to_path_buf();
                let relative_path = entry_path.strip_prefix(base_path).map_err(|e| {
                    RzxError::Io(io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
                })?;
                if !exclude
                    .iter()
                    .any(|ex| relative_path.to_string_lossy().contains(ex))
                {
                    files_to_process.push(entry_path);
                }
            }
        } else {
            let relative_path = path.strip_prefix(base_path).map_err(|e| {
                RzxError::Io(io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
            })?;
            if !exclude
                .iter()
                .any(|ex| relative_path.to_string_lossy().contains(ex))
            {
                files_to_process.push(path.clone());
            }
        }
    }

    let pb_compress = ProgressBar::new(files_to_process.len() as u64);
    pb_compress.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")? // Added ?
        .progress_chars("#>- "));
    pb_compress.set_message("Preparing files for archiving...");

    let processed_files: Vec<(FileEntry, PathBuf)> = files_to_process
        .par_iter()
        .filter_map(|path| {
            pb_compress.inc(1);
            let relative_path = path.strip_prefix(base_path).ok()?;
            let mut file_entry = if path.is_dir() {
                FileEntry::new_directory(relative_path)
            } else {
                FileEntry::new(relative_path)
            };
            file_entry
                .set_metadata_from_fs(&std::fs::metadata(&path).ok()?)
                .ok()?;

            Some((file_entry, path.clone()))
        })
        .collect();

    pb_compress.finish_with_message("File preparation complete.");

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_path)?;

    let mut writer = RzxWriter::new(file, encryption_algorithm, password)?;

    let pb_write = ProgressBar::new(processed_files.len() as u64);
    pb_write.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
        .progress_chars("#>- "));
    pb_write.set_message("Writing archive...");

    for (file_entry, absolute_path) in processed_files {
        pb_write.set_message(format!("Adding {}", file_entry.display_name()));
        writer.add_entry_sequential(file_entry, &absolute_path, compression_level)?;
        pb_write.inc(1);
    }

    pb_write.finish_with_message("Archive writing complete.");

    writer.finalize()?;

    Ok(())
}

/// Extract files from an RZX archive
pub fn extract_archive(
    archive_path: &Path,
    output_dir: &Path,
    password: Option<&str>,
    overwrite: bool,
) -> RzxResult<()> {
    let file = File::open(archive_path)?;
    let reader = RzxReader::new(file, password)?;

    let entries: Vec<FileEntry> = reader.metadata().entries.clone();

    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
        .progress_chars("#>- "));

    for entry in entries {
        let output_path = output_dir.join(&entry.path);
        println!("Debug: output_dir: {:?}", output_dir);
        println!("Debug: entry.path: {:?}", entry.path);
        println!("Debug: constructed output_path: {:?}", output_path);

        if output_path.exists() && !overwrite {
            pb.abandon_with_message(format!("Error: File already exists: {:?}", output_path));
            return Err(RzxError::Io(io::Error::new(io::ErrorKind::AlreadyExists, format!("File already exists: {:?}", output_path))));
        }

        pb.set_message(format!("Extracting {}", entry.display_name()));
        
        // Re-open the file for each entry, which is inefficient but safe for sequential processing.
        let file = File::open(archive_path)?;
        let mut thread_reader = RzxReader::new(file, password)?;

        thread_reader.extract_file(&entry, &output_path)?;
        pb.inc(1);
    }

    pb.finish_with_message("Extraction complete.");

    Ok(())
}
pub fn list_archive(archive_path: &Path, detailed: bool, password: Option<&str>) -> RzxResult<()> {
    let file = File::open(archive_path)?;
    let _reader = RzxReader::new(file, password)?;

    for entry in _reader.metadata().entries.iter() {
        if detailed {
            println!(
                "{} {} {} {} {}",
                entry.display_name(),
                entry.original_size,
                entry.compressed_size,
                entry.compression.as_str(),
                entry.checksum
            );
        } else {
            println!("{}", entry.display_name());
        }
    }

    Ok(())
}

/// Test integrity of an RZX archive
pub fn test_archive(archive_path: &Path, password: Option<&str>) -> RzxResult<()> {
    let file = File::open(archive_path)?;
    let _reader = RzxReader::new(file, password)?;

    // Metadata validation is done during RzxReader::new
    // Additional checks can be added here if needed

    println!("Archive {:?} integrity check passed.", archive_path);
    Ok(())
}

/// Show information about an RZX archive
pub fn show_archive_info(archive_path: &Path, password: Option<&str>) -> RzxResult<()> {
    let file = File::open(archive_path)?;
    let _reader = RzxReader::new(file, password)?;

    let metadata = _reader.metadata();

    println!("Archive Info:");
    println!("  Creator: {}", metadata.creator);
    println!("  Created At: {:?}", metadata.created_at);
    println!("  Modified At: {:?}", metadata.modified_at);
    println!("  Total Files: {}", metadata.file_count());
    println!("  Total Directories: {}", metadata.directory_count());
    println!("  Total Original Size: {}", metadata.total_original_size());
    println!(
        "  Total Compressed Size: {}",
        metadata.total_compressed_size()
    );
    println!(
        "  Overall Compression Ratio: {:.2}%",
        metadata.compression_ratio()
    );
    if let Some(comment) = &metadata.comment {
        println!("  Comment: {}", comment);
    }

    Ok(())
}
