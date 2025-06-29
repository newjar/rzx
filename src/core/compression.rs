use crate::core::{CompressionAlgorithm, FileType, RzxResult};
use flate2::write::{DeflateEncoder, DeflateDecoder};
use flate2::Compression;

#[allow(dead_code)]
pub fn default_compression_level() -> u8 {
    6
}

#[allow(dead_code)]
pub fn select_compression_algorithm(file_type: FileType, file_size: u64) -> CompressionAlgorithm {
    if file_size < 100 {
        return CompressionAlgorithm::None;
    }

    match file_type {
        FileType::Text | FileType::Document => {
            if file_size > 1024 * 1024 {
                CompressionAlgorithm::Lzma // Best compression for large text
            } else {
                CompressionAlgorithm::Deflate // Faster for small text
            }
        }
        FileType::Binary => CompressionAlgorithm::Zstd, // Good balance for binaries
        FileType::Image | FileType::Audio | FileType::Video | FileType::Archive => {
            CompressionAlgorithm::None // Already compressed
        }
        FileType::Unknown => CompressionAlgorithm::Deflate, // Safe default
    }
}

#[allow(dead_code)]
pub fn compress<R: std::io::Read + std::io::BufRead, W: std::io::Write>(
    algorithm: CompressionAlgorithm,
    reader: &mut R,
    writer: &mut W,
    level: u8,
) -> RzxResult<()> {
    match algorithm {
        CompressionAlgorithm::None => {
            std::io::copy(reader, writer)?;
        }
        CompressionAlgorithm::Deflate => {
            let mut encoder = DeflateEncoder::new(writer, Compression::new(level as u32));
            std::io::copy(reader, &mut encoder)?;
            encoder.finish()?;
        }
        CompressionAlgorithm::Lzma => {
            lzma_rs::lzma_compress(reader, writer)?;
        }
        CompressionAlgorithm::Zstd => {
            zstd::stream::copy_encode(reader, writer, level as i32)?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn decompress<R: std::io::Read + std::io::BufRead, W: std::io::Write>(
    algorithm: CompressionAlgorithm,
    reader: &mut R,
    writer: &mut W,
) -> RzxResult<()> {
    match algorithm {
        CompressionAlgorithm::None => {
            std::io::copy(reader, writer)?;
        }
        CompressionAlgorithm::Deflate => {
            let mut decoder = DeflateDecoder::new(writer);
            std::io::copy(reader, &mut decoder)?;
            decoder.finish()?;
        }
        CompressionAlgorithm::Lzma => {
            lzma_rs::lzma_decompress(reader, writer)?;
        }
        CompressionAlgorithm::Zstd => {
            zstd::stream::copy_decode(reader, writer)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_decompression() {
        let data = b"hello world";

        for &alg in &[CompressionAlgorithm::Deflate, CompressionAlgorithm::Lzma, CompressionAlgorithm::Zstd] {
            let mut compressed_data = Vec::new();
            let mut reader = std::io::Cursor::new(data);
            compress(alg, &mut reader, &mut compressed_data, 6).unwrap();

            let mut decompressed_data = Vec::new();
            let mut reader = std::io::Cursor::new(compressed_data);
            decompress(alg, &mut reader, &mut decompressed_data).unwrap();
            assert_eq!(data.to_vec(), decompressed_data);
        }
    }
}
