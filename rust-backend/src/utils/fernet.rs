use crate::error::{AppError, AppResult};
use aes::Aes128;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
/// Fernet-compatible encryption/decryption for OAuth tokens
/// Compatible with Python's cryptography.fernet.Fernet
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use cbc::{Decryptor as Aes128CbcDec, Encryptor as Aes128CbcEnc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const FERNET_VERSION: u8 = 0x80;

pub struct Fernet {
    signing_key: [u8; 16],
    encryption_key: [u8; 16],
}

impl Fernet {
    /// Create a new Fernet instance from a base64url-encoded 32-byte key
    /// If the key is not 44 characters (32 bytes base64url encoded), it will be hashed with SHA-256
    pub fn new(key: &str) -> AppResult<Self> {
        let key_bytes = if key.len() == 44 {
            // Properly formatted Fernet key
            URL_SAFE_NO_PAD
                .decode(key.as_bytes())
                .map_err(|e| AppError::Auth(format!("Invalid Fernet key format: {}", e)))?
        } else {
            // Hash the key with SHA-256 to get 32 bytes
            let mut hasher = Sha256::new();
            hasher.update(key.as_bytes());
            hasher.finalize().to_vec()
        };

        if key_bytes.len() != 32 {
            return Err(AppError::Auth("Fernet key must be 32 bytes".to_string()));
        }

        // Split into signing key (first 16 bytes) and encryption key (last 16 bytes)
        let mut signing_key = [0u8; 16];
        let mut encryption_key = [0u8; 16];
        signing_key.copy_from_slice(&key_bytes[0..16]);
        encryption_key.copy_from_slice(&key_bytes[16..32]);

        Ok(Self {
            signing_key,
            encryption_key,
        })
    }

    /// Encrypt data and return base64url-encoded Fernet token
    pub fn encrypt(&self, data: &[u8]) -> AppResult<String> {
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Generate random IV (16 bytes)
        let mut iv = [0u8; 16];
        use rand::RngCore;
        rand::rng().fill_bytes(&mut iv);

        // Encrypt data with AES-128-CBC
        let mut buffer = data.to_vec();
        // Add PKCS7 padding
        let padding_len = 16 - (buffer.len() % 16);
        buffer.extend(vec![padding_len as u8; padding_len]);

        let mut cipher = cbc::Encryptor::<Aes128>::new(&self.encryption_key.into(), &iv.into());

        // Encrypt in place, block by block
        use aes::cipher::generic_array::{typenum::U16, GenericArray};
        for chunk in buffer.chunks_exact_mut(16) {
            let block: &mut GenericArray<u8, U16> = GenericArray::from_mut_slice(chunk);
            cipher.encrypt_block_mut(block);
        }

        let ciphertext = buffer;

        // Build Fernet token: version (1) | timestamp (8) | iv (16) | ciphertext | hmac (32)
        let mut token = Vec::new();
        token.push(FERNET_VERSION);
        token.extend_from_slice(&timestamp.to_be_bytes());
        token.extend_from_slice(&iv);
        token.extend_from_slice(&ciphertext);

        // Calculate HMAC-SHA256 over version | timestamp | iv | ciphertext
        let mut mac = HmacSha256::new_from_slice(&self.signing_key)
            .map_err(|e| AppError::Auth(format!("HMAC error: {}", e)))?;
        mac.update(&token);
        let hmac_result = mac.finalize().into_bytes();

        // Append HMAC to token
        token.extend_from_slice(&hmac_result);

        // Base64url encode the entire token
        Ok(URL_SAFE_NO_PAD.encode(&token))
    }

    /// Decrypt a base64url-encoded Fernet token
    pub fn decrypt(&self, token: &str) -> AppResult<Vec<u8>> {
        // Decode base64url
        let token_bytes = URL_SAFE_NO_PAD
            .decode(token.as_bytes())
            .map_err(|e| AppError::Auth(format!("Invalid Fernet token encoding: {}", e)))?;

        if token_bytes.len() < 57 {
            // 1 + 8 + 16 + 0 + 32 minimum
            return Err(AppError::Auth("Fernet token too short".to_string()));
        }

        // Extract components
        let version = token_bytes[0];
        if version != FERNET_VERSION {
            return Err(AppError::Auth("Invalid Fernet version".to_string()));
        }

        let timestamp_bytes = &token_bytes[1..9];
        let iv = &token_bytes[9..25];
        let ciphertext_end = token_bytes.len() - 32;
        let ciphertext = &token_bytes[25..ciphertext_end];
        let expected_hmac = &token_bytes[ciphertext_end..];

        // Verify HMAC
        let mut mac = HmacSha256::new_from_slice(&self.signing_key)
            .map_err(|e| AppError::Auth(format!("HMAC error: {}", e)))?;
        mac.update(&token_bytes[..ciphertext_end]);
        mac.verify_slice(expected_hmac)
            .map_err(|_| AppError::Auth("Invalid Fernet token signature".to_string()))?;

        // Optional: Check timestamp (TTL verification)
        // For now, we'll skip TTL check as Python implementation doesn't enforce it by default

        // Decrypt ciphertext
        let mut iv_array = [0u8; 16];
        iv_array.copy_from_slice(iv);

        let mut buffer = ciphertext.to_vec();

        let mut cipher =
            cbc::Decryptor::<Aes128>::new(&self.encryption_key.into(), &iv_array.into());

        // Decrypt in place, block by block
        use aes::cipher::generic_array::{typenum::U16, GenericArray};
        for chunk in buffer.chunks_exact_mut(16) {
            let block: &mut GenericArray<u8, U16> = GenericArray::from_mut_slice(chunk);
            cipher.decrypt_block_mut(block);
        }

        // Remove PKCS7 padding
        if buffer.is_empty() {
            return Err(AppError::Auth("Invalid ciphertext: empty".to_string()));
        }
        let padding_len = *buffer.last().unwrap() as usize;
        if padding_len == 0 || padding_len > 16 || padding_len > buffer.len() {
            return Err(AppError::Auth("Invalid padding".to_string()));
        }
        // Verify padding
        for i in (buffer.len() - padding_len)..buffer.len() {
            if buffer[i] != padding_len as u8 {
                return Err(AppError::Auth("Invalid padding bytes".to_string()));
            }
        }
        buffer.truncate(buffer.len() - padding_len);

        Ok(buffer)
    }

    /// Encrypt JSON data
    pub fn encrypt_json<T: serde::Serialize>(&self, data: &T) -> AppResult<String> {
        let json = serde_json::to_string(data)
            .map_err(|e| AppError::Auth(format!("JSON serialization error: {}", e)))?;
        self.encrypt(json.as_bytes())
    }

    /// Decrypt JSON data
    pub fn decrypt_json<T: serde::de::DeserializeOwned>(&self, token: &str) -> AppResult<T> {
        let plaintext = self.decrypt(token)?;
        let data = serde_json::from_slice(&plaintext)
            .map_err(|e| AppError::Auth(format!("JSON deserialization error: {}", e)))?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fernet_encryption_decryption() {
        let key = "cw_0x689RpI-jtRR7oE8h_eQsKImvJapLeSbXpwF4e4="; // Valid Fernet key
        let fernet = Fernet::new(key).unwrap();

        let data = b"Hello, World!";
        let encrypted = fernet.encrypt(data).unwrap();
        let decrypted = fernet.decrypt(&encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_fernet_with_short_key() {
        let key = "short_key"; // Will be hashed
        let fernet = Fernet::new(key).unwrap();

        let data = b"Test data";
        let encrypted = fernet.encrypt(data).unwrap();
        let decrypted = fernet.decrypt(&encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_fernet_json() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            access_token: String,
            refresh_token: String,
        }

        let key = "cw_0x689RpI-jtRR7oE8h_eQsKImvJapLeSbXpwF4e4=";
        let fernet = Fernet::new(key).unwrap();

        let data = TestData {
            access_token: "access123".to_string(),
            refresh_token: "refresh456".to_string(),
        };

        let encrypted = fernet.encrypt_json(&data).unwrap();
        let decrypted: TestData = fernet.decrypt_json(&encrypted).unwrap();

        assert_eq!(data, decrypted);
    }
}
