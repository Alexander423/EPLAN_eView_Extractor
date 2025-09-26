use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Encrypted password representation for JSON serialization
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedPassword {
    /// Base64-encoded encrypted data
    pub data: String,
    /// Base64-encoded nonce/IV
    pub nonce: String,
    /// Indicates this is an encrypted password (for backward compatibility)
    pub encrypted: bool,
}

/// Password encryption/decryption functionality
pub struct PasswordCrypto;

impl PasswordCrypto {
    /// Generates a machine-specific encryption key
    pub fn get_machine_key() -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();

        // Collect machine-specific identifiers
        let mut machine_data = BTreeMap::new();

        // Add computer name
        if let Ok(computer_name) = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
        {
            machine_data.insert("computer_name", computer_name);
        }

        // Add username
        if let Ok(username) = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
        {
            machine_data.insert("username", username);
        }

        // Add Windows-specific system root
        if let Ok(system_root) = std::env::var("SystemRoot") {
            machine_data.insert("system_root", system_root);
        }

        // Add processor info if available
        if let Ok(processor) = std::env::var("PROCESSOR_IDENTIFIER") {
            machine_data.insert("processor", processor);
        }

        // Add OS info
        machine_data.insert("os", std::env::consts::OS.to_string());
        machine_data.insert("arch", std::env::consts::ARCH.to_string());

        // Create deterministic hash from machine data
        for (key, value) in machine_data {
            hasher.update(key.as_bytes());
            hasher.update(b":");
            hasher.update(value.as_bytes());
            hasher.update(b";");
        }

        // Add a fixed salt to make the key specific to this application
        hasher.update(b"eview_scraper_v1.0.0_password_key");

        let hash = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);

        Ok(key)
    }

    /// Encrypts a plaintext password
    pub fn encrypt_password(plaintext: &str) -> Result<EncryptedPassword> {
        if plaintext.is_empty() {
            return Ok(EncryptedPassword {
                data: String::new(),
                nonce: String::new(),
                encrypted: true,
            });
        }

        let key = Self::get_machine_key()
            .context("Failed to generate machine key")?;

        let cipher = Aes256Gcm::new(&key.into());
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok(EncryptedPassword {
            data: BASE64.encode(&ciphertext),
            nonce: BASE64.encode(&nonce),
            encrypted: true,
        })
    }

    /// Decrypts an encrypted password
    pub fn decrypt_password(encrypted: &EncryptedPassword) -> Result<String> {
        if !encrypted.encrypted {
            // This should not happen with proper usage
            return Err(anyhow::anyhow!("Password is not encrypted"));
        }

        if encrypted.data.is_empty() {
            return Ok(String::new());
        }

        let key = Self::get_machine_key()
            .context("Failed to generate machine key")?;

        let cipher = Aes256Gcm::new(&key.into());

        let ciphertext = BASE64.decode(&encrypted.data)
            .context("Failed to decode encrypted data")?;

        let nonce_bytes = BASE64.decode(&encrypted.nonce)
            .context("Failed to decode nonce")?;

        if nonce_bytes.len() != 12 {
            return Err(anyhow::anyhow!("Invalid nonce length"));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .context("Decrypted data is not valid UTF-8")
    }

    /// Checks if a string looks like it might be an encrypted password (JSON)
    pub fn is_likely_encrypted(text: &str) -> bool {
        text.trim().starts_with('{') &&
        text.contains("\"encrypted\"") &&
        text.contains("\"data\"") &&
        text.contains("\"nonce\"")
    }

    /// Migrates a plaintext password to encrypted format
    pub fn migrate_plaintext_password(plaintext: &str) -> Result<EncryptedPassword> {
        Self::encrypt_password(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let original = "test_password_123";

        let encrypted = PasswordCrypto::encrypt_password(original).unwrap();
        assert!(encrypted.encrypted);
        assert!(!encrypted.data.is_empty());
        assert!(!encrypted.nonce.is_empty());

        let decrypted = PasswordCrypto::decrypt_password(&encrypted).unwrap();
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_empty_password() {
        let empty = "";

        let encrypted = PasswordCrypto::encrypt_password(empty).unwrap();
        assert!(encrypted.encrypted);
        assert!(encrypted.data.is_empty());
        assert!(encrypted.nonce.is_empty());

        let decrypted = PasswordCrypto::decrypt_password(&encrypted).unwrap();
        assert_eq!(empty, decrypted);
    }

    #[test]
    fn test_machine_key_consistency() {
        let key1 = PasswordCrypto::get_machine_key().unwrap();
        let key2 = PasswordCrypto::get_machine_key().unwrap();
        assert_eq!(key1, key2, "Machine key should be consistent");
    }

    #[test]
    fn test_is_likely_encrypted() {
        let encrypted_json = r#"{"data":"dGVzdA==","nonce":"bm9uY2U=","encrypted":true}"#;
        assert!(PasswordCrypto::is_likely_encrypted(encrypted_json));

        let plaintext = "regular_password";
        assert!(!PasswordCrypto::is_likely_encrypted(plaintext));
    }
}