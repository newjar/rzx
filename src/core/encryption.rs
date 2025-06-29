use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use pbkdf2::hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use std::io::{Read, Write};

use crate::core::{EncryptionAlgorithm, RzxError, RzxResult};

pub const SALT_SIZE: usize = 16;
pub const NONCE_SIZE: usize = 12;
pub const KEY_SIZE: usize = 32;

/// Generates a random salt for key derivation.
pub fn generate_salt() -> [u8; SALT_SIZE] {
    let mut salt = [0u8; SALT_SIZE];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Generates a random nonce for AES-GCM.
pub fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

/// Derives an encryption key from a password and salt using PBKDF2.
pub fn derive_key(password: &[u8], salt: &[u8; SALT_SIZE]) -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    let _ = pbkdf2::<Hmac<Sha256>>(password, salt, 100_000, &mut key);
    key
}

/// Encrypts data using AES-256 GCM.
pub fn encrypt<R: Read, W: Write>(
    algorithm: EncryptionAlgorithm,
    key: &[u8; KEY_SIZE],
    nonce: &[u8; NONCE_SIZE],
    reader: &mut R,
    writer: &mut W,
) -> RzxResult<()> {
    match algorithm {
        EncryptionAlgorithm::None => {
            std::io::copy(reader, writer)?;
        }
        EncryptionAlgorithm::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key).unwrap();
            let nonce = Nonce::from_slice(nonce);

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;

            let ciphertext = cipher
                .encrypt(nonce, buffer.as_ref())
                .map_err(|e| RzxError::EncryptionError(e.to_string()))?;
            writer.write_all(&ciphertext)?;
        }
    }
    Ok(())
}

/// Decrypts data using AES-256 GCM.
pub fn decrypt<R: Read, W: Write>(
    algorithm: EncryptionAlgorithm,
    key: &[u8; KEY_SIZE],
    nonce: &[u8; NONCE_SIZE],
    reader: &mut R,
    writer: &mut W,
) -> RzxResult<()> {
    match algorithm {
        EncryptionAlgorithm::None => {
            std::io::copy(reader, writer)?;
        }
        EncryptionAlgorithm::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key).unwrap();
            let nonce = Nonce::from_slice(nonce);

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;

            let plaintext = cipher
                .decrypt(nonce, buffer.as_ref())
                .map_err(|e| RzxError::DecryptionError(e.to_string()))?;
            writer.write_all(&plaintext)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes256gcm_encryption_decryption() {
        let password = b"mysecretpassword";
        let salt = generate_salt();
        let nonce = generate_nonce();
        let key = derive_key(password, &salt);
        let plaintext = b"Hello, world!";

        let mut encrypted_data = Vec::new();
        encrypt(
            EncryptionAlgorithm::Aes256Gcm,
            &key,
            &nonce,
            &mut plaintext.as_ref(),
            &mut encrypted_data,
        )
        .unwrap();

        let mut decrypted_data = Vec::new();
        decrypt::<&[u8], Vec<u8>>(
            EncryptionAlgorithm::Aes256Gcm,
            &key,
            &nonce,
            &mut encrypted_data.as_ref(),
            &mut decrypted_data,
        )
        .unwrap();

        assert_eq!(plaintext.to_vec(), decrypted_data);
    }
}
