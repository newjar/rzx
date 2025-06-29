use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub mod compression;
pub mod encryption;
pub mod header;
pub mod metadata;

// Re-export main structures
pub use compression::*;
pub use encryption::*;
pub use header::*;
pub use metadata::*;

/// RZX Format Version
pub const RZX_VERSION: u16 = 1;

/// Magic bytes for RZX format: "RZX\0"
pub const RZX_MAGIC: [u8; 4] = [b'R', b'z', b'X', 0];

/// Compression algorithms supported by RZX
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    None = 0,
    Deflate = 1,
    Lzma = 2,
    Zstd = 3,
}

impl CompressionAlgorithm {
    #[allow(dead_code)]
pub fn from_str(s: &str) -> Option<Self> {
    match s.to_lowercase().as_str() {
        "none" => Some(Self::None),
        "deflate" => Some(Self::Deflate),
        "lzma" => Some(Self::Lzma),
        "zstd" => Some(Self::Zstd),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn as_str(&self) -> &'static str {
    match self {
        Self::None => "none",
        Self::Deflate => "deflate",
        Self::Lzma => "lzma",
        Self::Zstd => "zstd",
    }
}
}

/// Encryption algorithms supported by RZX
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EncryptionAlgorithm {
    None = 0,
    Aes256Gcm = 1,
}

impl EncryptionAlgorithm {
    #[allow(dead_code)]
pub fn from_str(s: &str) -> Option<Self> {
    match s.to_lowercase().as_str() {
        "none" => Some(Self::None),
        "aes256gcm" => Some(Self::Aes256Gcm),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn as_str(&self) -> &'static str {
    match self {
        Self::None => "none",
        Self::Aes256Gcm => "aes256gcm",
    }
}

#[allow(dead_code)]
pub fn from_u8(v: u8) -> Option<Self> {
    match v {
        0 => Some(Self::None),
        1 => Some(Self::Aes256Gcm),
        _ => None,
    }
}
}

/// Feature flags for RZX format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureFlags(pub u8);

impl FeatureFlags {
    #[allow(dead_code)]
pub const NONE: Self = Self(0);
#[allow(dead_code)]
pub const ENCRYPTED: Self = Self(1 << 0);
#[allow(dead_code)]
pub const COMPRESSED: Self = Self(1 << 1);
#[allow(dead_code)]
pub const HAS_COMMENT: Self = Self(1 << 2);
#[allow(dead_code)]
pub const EXTENDED_METADATA: Self = Self(1 << 3);

#[allow(dead_code)]
pub fn has_flag(&self, flag: Self) -> bool {
    (self.0 & flag.0) != 0
}

#[allow(dead_code)]
pub fn set_flag(&mut self, flag: Self) {
    self.0 |= flag.0;
}

#[allow(dead_code)]
pub fn clear_flag(&mut self, flag: Self) {
    self.0 &= !flag.0;
}
}

/// File type classification for smart compression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Text,
    Binary,
    Image,
    Audio,
    Video,
    Archive,
    Document,
    Unknown,
}

impl FileType {
    #[allow(dead_code)]
pub fn from_extension(ext: &str) -> Self {
    match ext.to_lowercase().as_str() {
        "txt" | "md" | "json" | "xml" | "html" | "csv" | "log" => Self::Text,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => Self::Image,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => Self::Audio,
        "mp4" | "avi" | "mkv" | "mov" | "webm" => Self::Video,
        "zip" | "rar" | "7z" | "tar" | "gz" => Self::Archive,
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" => Self::Document,
        "exe" | "dll" | "so" | "dylib" => Self::Binary,
        _ => Self::Unknown,
    }
}

#[allow(dead_code)]
pub fn best_compression(&self) -> CompressionAlgorithm {
    match self {
        Self::Text | Self::Document => CompressionAlgorithm::Lzma, // High compression for text
        Self::Binary => CompressionAlgorithm::Zstd,                // Fast for binaries
        Self::Image | Self::Audio | Self::Video | Self::Archive => CompressionAlgorithm::None, // Already compressed
        Self::Unknown => CompressionAlgorithm::Deflate, // Safe default
    }
}
}

/// Archive statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchiveStats {
    pub total_files: u32,
    pub total_dirs: u32,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub created_at: Option<SystemTime>,
    pub modified_at: Option<SystemTime>,
}

impl ArchiveStats {
    #[allow(dead_code)]
pub fn new() -> Self {
    Self {
        created_at: Some(SystemTime::now()),
        modified_at: Some(SystemTime::now()),
        ..Default::default()
    }
}

#[allow(dead_code)]
pub fn update_compression_ratio(&mut self) {
    if self.uncompressed_size > 0 {
        self.compression_ratio =
            1.0 - (self.compressed_size as f64 / self.uncompressed_size as f64);
    }
}

#[allow(dead_code)]
pub fn add_file(&mut self, original_size: u64, compressed_size: u64) {
    self.total_files += 1;
    self.uncompressed_size += original_size;
    self.compressed_size += compressed_size;
    self.update_compression_ratio();
    self.modified_at = Some(SystemTime::now());
}

#[allow(dead_code)]
pub fn add_directory(&mut self) {
    self.total_dirs += 1;
    self.modified_at = Some(SystemTime::now());
}
}

/// Error types for RZX operations
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum RzxError {
    #[error("Invalid RZX magic bytes")]
    InvalidMagic,

    #[error("Unsupported RZX version: {0}")]
    UnsupportedVersion(u16),

    #[error("Compression algorithm not supported: {0:?}")]
    UnsupportedCompression(u8),

    #[error("Archive is corrupted: {0}")]
    CorruptedArchive(String),

    #[error("File not found in archive: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("LZMA error: {0}")]
    Lzma(#[from] lzma_rs::error::Error),

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Progress bar template error: {0}")]
    ProgressBarTemplate(#[from] indicatif::style::TemplateError),
}

#[allow(dead_code)]
pub type RzxResult<T> = Result<T, RzxError>;
