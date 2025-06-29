use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use crate::core::{CompressionAlgorithm, FileType, RzxResult};

/// File entry in RZX archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path of the file in archive
    pub path: PathBuf,

    /// Original file size in bytes
    pub original_size: u64,

    /// Compressed size in bytes
    pub compressed_size: u64,

    /// Compression algorithm used for this file
    pub compression: CompressionAlgorithm,

    /// File type classification
    pub file_type: FileType,

    /// Offset in the data section where compressed data starts
    pub data_offset: u64,

    /// CRC32 checksum of original file
    pub checksum: u32,

    /// File permissions (Unix-style)
    pub permissions: u32,

    /// Creation time
    pub created: Option<SystemTime>,

    /// Last modification time
    pub modified: Option<SystemTime>,

    /// Last access time
    pub accessed: Option<SystemTime>,

    /// Whether this entry represents a directory
    pub is_directory: bool,

    /// Extended attributes (platform-specific)
    pub extended_attrs: HashMap<String, Vec<u8>>,
}

impl FileEntry {
    /// Create a new file entry from a path
    #[allow(dead_code)]
pub fn new<P: AsRef<Path>>(path: P) -> Self {
    let path = path.as_ref().to_path_buf();
    println!("Debug: FileEntry::new - input path: {:?}", path);
    let file_type = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(FileType::from_extension)
        .unwrap_or(FileType::Unknown);

    Self {
        path,
        original_size: 0,
        compressed_size: 0,
        compression: file_type.best_compression(),
        file_type,
        data_offset: 0,
        checksum: 0,
        permissions: 0o644, // Default Unix permissions
        created: None,
        modified: None,
        accessed: None,
        is_directory: false,
        extended_attrs: HashMap::new(),
    }
}

#[allow(dead_code)]
pub fn new_directory<P: AsRef<Path>>(path: P) -> Self {
    let mut entry = Self::new(path);
    entry.is_directory = true;
    entry.permissions = 0o755; // Default directory permissions
    entry.compression = CompressionAlgorithm::None; // Directories don't compress
    entry
}

#[allow(dead_code)]
pub fn compression_ratio(&self) -> f64 {
    if self.original_size == 0 {
        return 0.0;
    }

    100.0 * (1.0 - (self.compressed_size as f64 / self.original_size as f64))
}

#[allow(dead_code)]
pub fn should_compress(&self) -> bool {
    if self.is_directory {
        return false;
    }

    // Don't compress very small files (overhead not worth it)
    if self.original_size < 100 {
        return false;
    }

    // Don't compress already compressed formats
    matches!(self.file_type,
        FileType::Text | FileType::Binary | FileType::Document | FileType::Unknown
    )
}

#[allow(dead_code)]
pub fn optimize_compression(&mut self) {
    if !self.should_compress() {
        self.compression = CompressionAlgorithm::None;
        return;
    }

    // Choose algorithm based on file type and size
    self.compression = match self.file_type {
        FileType::Text | FileType::Document => {
            if self.original_size > 1024 * 1024 {
                CompressionAlgorithm::Lzma // Best compression for large text
            } else {
                CompressionAlgorithm::Deflate // Faster for small text
            }
        },
        FileType::Binary => CompressionAlgorithm::Zstd, // Good balance for binaries
        _ => CompressionAlgorithm::Deflate, // Safe default
    };
}

#[allow(dead_code)]
    pub fn set_metadata_from_fs(&mut self, metadata: &std::fs::Metadata) -> RzxResult<()> {
        self.is_directory = metadata.is_dir();
        if self.is_directory {
            self.original_size = 0;
        } else {
            self.original_size = metadata.len();
        }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        self.permissions = metadata.mode();
    }

    #[cfg(windows)]
    {
        // Windows doesn't have Unix-style permissions, use defaults
        self.permissions = if self.is_directory { 0o755 } else { 0o644 };
    }

    // Set timestamps
    if let Ok(created) = metadata.created() {
        self.created = Some(created);
    }

    if let Ok(modified) = metadata.modified() {
        self.modified = Some(modified);
    }

    if let Ok(accessed) = metadata.accessed() {
        self.accessed = Some(accessed);
    }

    Ok(())
}

#[allow(dead_code)]
pub fn display_name(&self) -> String {
    let mut name = self.path.to_string_lossy().to_string();
    if self.is_directory && !name.ends_with('/') {
        name.push('/');
    }
    name
}

#[allow(dead_code)]
pub fn validate(&self) -> RzxResult<()> {
    if self.path.as_os_str().is_empty() {
        return Err(crate::core::RzxError::CorruptedArchive("Empty file path".to_string()));
    }

    if self.is_directory && self.original_size > 0 {
        return Err(crate::core::RzxError::CorruptedArchive(
            "Directory with non-zero size".to_string()
        ));
    }

    if !self.is_directory && self.compressed_size == 0 && self.original_size > 0 {
        return Err(crate::core::RzxError::CorruptedArchive(
            "File with zero compressed size".to_string()
        ));
    }

    Ok(())
}
}

/// Archive metadata containing all file entries and global information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// List of all file entries
    pub entries: Vec<FileEntry>,

    /// Archive comment
    pub comment: Option<String>,

    /// Creator information
    pub creator: String,

    /// Archive creation timestamp
    pub created_at: SystemTime,

    /// Last modification timestamp
    pub modified_at: SystemTime,

    /// Custom metadata fields
    pub custom_fields: HashMap<String, String>,
}

impl ArchiveMetadata {
    /// Create new archive metadata
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            entries: Vec::new(),
            comment: None,
            creator: format!("rzx-compressor v{}", env!("CARGO_PKG_VERSION")),
            created_at: now,
            modified_at: now,
            custom_fields: HashMap::new(),
        }
    }

    /// Add a file entry
    #[allow(dead_code)]
pub fn add_entry(&mut self, entry: FileEntry) {
    self.entries.push(entry);
    self.modified_at = SystemTime::now();
}

#[allow(dead_code)]
pub fn find_entry(&self, path: &Path) -> Option<&FileEntry> {
    self.entries.iter().find(|entry| entry.path == path)
}

#[allow(dead_code)]
pub fn find_entry_mut(&mut self, path: &Path) -> Option<&mut FileEntry> {
    self.entries.iter_mut().find(|entry| entry.path == path)
}

#[allow(dead_code)]
pub fn total_original_size(&self) -> u64 {
    self.entries.iter().map(|e| e.original_size).sum()
}

#[allow(dead_code)]
pub fn total_compressed_size(&self) -> u64 {
    self.entries.iter().map(|e| e.compressed_size).sum()
}

#[allow(dead_code)]
pub fn compression_ratio(&self) -> f64 {
    let original = self.total_original_size();
    if original == 0 {
        return 0.0;
    }

    let compressed = self.total_compressed_size();
    100.0 * (1.0 - (compressed as f64 / original as f64))
}

#[allow(dead_code)]
pub fn file_count(&self) -> usize {
    self.entries.iter().filter(|e| !e.is_directory).count()
}

#[allow(dead_code)]
pub fn directory_count(&self) -> usize {
    self.entries.iter().filter(|e| e.is_directory).count()
}

#[allow(dead_code)]
pub fn sort_entries(&mut self) {
    // Sort directories first, then by file type for better compression
    self.entries.sort_by(|a, b| {
        if a.is_directory && !b.is_directory {
            std::cmp::Ordering::Less
        } else if !a.is_directory && b.is_directory {
            std::cmp::Ordering::Greater
        } else {
            a.path.cmp(&b.path)
        }
    });
}

#[allow(dead_code)]
pub fn validate(&self) -> RzxResult<()> {
    for entry in &self.entries {
        entry.validate()?;
    }

    // Check for duplicate paths
    let mut paths = std::collections::HashSet::new();
    for entry in &self.entries {
        if !paths.insert(&entry.path) {
            return Err(crate::core::RzxError::CorruptedArchive(
                format!("Duplicate path: {:?}", entry.path)
            ));
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn to_bytes(&self) -> RzxResult<Vec<u8>> {
    if let Some(entry) = self.entries.first() {
        println!("Metadata: Checksum before serialization for {}: {:x}", entry.display_name(), entry.checksum);
    }
    Ok(bincode::serialize(self)?)
}

#[allow(dead_code)]
pub fn from_bytes(bytes: &[u8]) -> RzxResult<Self> {
    let deserialized: Self = bincode::deserialize(bytes)?;
    if let Some(entry) = deserialized.entries.first() {
        println!("Metadata: Checksum after deserialization for {}: {:x}", entry.display_name(), entry.checksum);
    }
    Ok(deserialized)
}
}

impl Default for ArchiveMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry_creation() {
        let entry = FileEntry::new("test.txt");
        assert_eq!(entry.path, PathBuf::from("test.txt"));
        assert_eq!(entry.file_type, FileType::Text);
        assert!(!entry.is_directory);
    }

    #[test]
    fn test_directory_entry() {
        let entry = FileEntry::new_directory("test_dir");
        assert!(entry.is_directory);
        assert_eq!(entry.permissions, 0o755);
    }

    #[test]
    fn test_compression_ratio() {
        let mut entry = FileEntry::new("test.txt");
        entry.original_size = 1000;
        entry.compressed_size = 300;

        assert!((entry.compression_ratio() - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_metadata_serialization() {
        let mut metadata = ArchiveMetadata::new();
        metadata.add_entry(FileEntry::new("test.txt"));

        let bytes = metadata.to_bytes().unwrap();
        let deserialized = ArchiveMetadata::from_bytes(&bytes).unwrap();

        assert_eq!(metadata.entries.len(), deserialized.entries.len());
        assert_eq!(metadata.entries[0].path, deserialized.entries[0].path);
    }
}
