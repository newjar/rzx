use serde::{Deserialize, Serialize};
use crate::core::{EncryptionAlgorithm, RZX_MAGIC, RZX_VERSION, RzxError, RzxResult};

/// RZX archive header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RzxHeader {
    /// Magic bytes, should be "RZX\0"
    pub magic: [u8; 4],

    /// RZX format version
    pub version: u16,

    /// Encryption algorithm used for the archive
    pub encryption_algorithm: EncryptionAlgorithm,

    /// Salt for key derivation (if encrypted)
    pub salt: Option<[u8; 16]>,

    /// Nonce for encryption (if encrypted)
    pub nonce: Option<[u8; 12]>,

    /// Offset to the metadata block
    pub metadata_offset: u64,

    /// Size of the metadata block in bytes
    pub metadata_size: u64,

    /// CRC32 checksum of the metadata block
    pub metadata_checksum: u32,
}

impl RzxHeader {
    /// Create a new RZX header
    #[allow(dead_code)]
pub fn new(
    encryption_algorithm: EncryptionAlgorithm,
    salt: Option<[u8; 16]>,
    nonce: Option<[u8; 12]>,
    metadata_offset: u64,
    metadata_size: u64,
    metadata_checksum: u32,
) -> Self {
    Self {
        magic: RZX_MAGIC,
        version: RZX_VERSION,
        encryption_algorithm,
        salt,
        nonce,
        metadata_offset,
        metadata_size,
        metadata_checksum,
    }
}

#[allow(dead_code)]
pub fn validate(&self) -> RzxResult<()> {
    if self.magic != RZX_MAGIC {
        return Err(RzxError::InvalidMagic);
    }
    if self.version > RZX_VERSION {
        return Err(RzxError::UnsupportedVersion(self.version));
    }
    if self.encryption_algorithm != EncryptionAlgorithm::None {
        if self.salt.is_none() || self.nonce.is_none() {
            return Err(RzxError::CorruptedArchive(
                "Encrypted archive missing salt or nonce".to_string(),
            ));
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn to_bytes(&self) -> RzxResult<Vec<u8>> {
    Ok(bincode::serialize(self)?)
}

#[allow(dead_code)]
pub fn from_bytes(bytes: &[u8]) -> RzxResult<Self> {
    let header: Self = bincode::deserialize(bytes)?;
    header.validate()?;
    Ok(header)
}

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialization() {
        let header = RzxHeader::new(EncryptionAlgorithm::None, None, None, 1024, 2048, 0x12345678);
        let bytes = header.to_bytes().unwrap();
        let deserialized = RzxHeader::from_bytes(&bytes).unwrap();

        assert_eq!(header.magic, deserialized.magic);
        assert_eq!(header.version, deserialized.version);
        assert_eq!(header.encryption_algorithm, deserialized.encryption_algorithm);
        assert_eq!(header.salt, deserialized.salt);
        assert_eq!(header.nonce, deserialized.nonce);
        assert_eq!(header.metadata_offset, deserialized.metadata_offset);
        assert_eq!(header.metadata_size, deserialized.metadata_size);
        assert_eq!(header.metadata_checksum, deserialized.metadata_checksum);
    }

    #[test]
    fn test_header_validation() {
        let header = RzxHeader::new(EncryptionAlgorithm::None, None, None, 0, 0, 0);
        assert!(header.validate().is_ok());

        let mut bad_magic = header.clone();
        bad_magic.magic = [0; 4];
        assert!(matches!(bad_magic.validate(), Err(RzxError::InvalidMagic)));

        let mut bad_version = header.clone();
        bad_version.version = RZX_VERSION + 1;
        assert!(matches!(bad_version.validate(), Err(RzxError::UnsupportedVersion(_))));

        let mut encrypted_no_salt = header.clone();
        encrypted_no_salt.encryption_algorithm = EncryptionAlgorithm::Aes256Gcm;
        encrypted_no_salt.salt = None;
        encrypted_no_salt.nonce = Some([0; 12]);
        assert!(matches!(encrypted_no_salt.validate(), Err(RzxError::CorruptedArchive(_))));

        let mut encrypted_no_nonce = header.clone();
        encrypted_no_nonce.encryption_algorithm = EncryptionAlgorithm::Aes256Gcm;
        encrypted_no_nonce.salt = Some([0; 16]);
        encrypted_no_nonce.nonce = None;
        assert!(matches!(encrypted_no_nonce.validate(), Err(RzxError::CorruptedArchive(_))));
    }
}