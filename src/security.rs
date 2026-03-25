use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use rcgen::KeyPair;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const NONCE_SIZE: usize = 12;

/// Encrypt data using AES-256-GCM with a password-derived key
pub fn encrypt_data(data: &[u8], password: &str) -> Result<Vec<u8>> {
    let key = derive_key(password);
    let cipher = Aes256Gcm::new(&key.into());

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    for byte in &mut nonce_bytes {
        *byte = rand::random();
    }
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt data using AES-256-GCM with a password-derived key
pub fn decrypt_data(encrypted: &[u8], password: &str) -> Result<Vec<u8>> {
    if encrypted.len() < NONCE_SIZE {
        anyhow::bail!("Invalid encrypted data: too short");
    }

    let key = derive_key(password);
    let cipher = Aes256Gcm::new(&key.into());

    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
}

/// Derive a 32-byte key from password using SHA-256
fn derive_key(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.finalize().into()
}

/// Encrypt and save config file
pub fn encrypt_config_file(config_yaml: &str, path: &Path, password: &str) -> Result<()> {
    let encrypted = encrypt_data(config_yaml.as_bytes(), password)?;
    fs::write(path, encrypted)
        .with_context(|| format!("Failed to write encrypted config to {}", path.display()))
}

/// Load and decrypt config file
pub fn decrypt_config_file(path: &Path, password: &str) -> Result<String> {
    let encrypted = fs::read(path)
        .with_context(|| format!("Failed to read encrypted config from {}", path.display()))?;
    let decrypted = decrypt_data(&encrypted, password)?;
    String::from_utf8(decrypted).with_context(|| "Invalid UTF-8 in decrypted config")
}

/// Mask sensitive data in strings for logging
pub fn mask_sensitive(text: &str, keywords: &[&str]) -> String {
    let mut result = text.to_string();
    for keyword in keywords {
        if let Some(start_pos) = result.find(keyword) {
            let value_start = start_pos + keyword.len();
            if value_start < result.len() {
                let after = &result[value_start..];
                if let Some(end_pos) = after.find(&[' ', '"', '\'', ',', '\n'][..]) {
                    let value = &after[..end_pos];
                    if value.len() > 8 {
                        let masked = format!("{}****{}", &value[..4], &value[value.len() - 4..]);
                        result.replace_range(value_start..value_start + value.len(), &masked);
                    }
                }
            }
        }
    }
    result
}

/// Derive encryption key from CA private key
pub fn derive_key_from_ca_key(ca_key_pem: &str) -> Result<[u8; 32]> {
    let key_pair = KeyPair::from_pem(ca_key_pem)
        .with_context(|| "Failed to parse CA private key")?;
    
    // Use the raw private key bytes to derive encryption key
    let key_der = key_pair.serialized_der();
    let mut hasher = Sha256::new();
    hasher.update(key_der);
    Ok(hasher.finalize().into())
}

/// Encrypt data using CA private key-derived key
pub fn encrypt_data_with_ca_key(data: &[u8], ca_key_pem: &str) -> Result<Vec<u8>> {
    let key = derive_key_from_ca_key(ca_key_pem)?;
    let cipher = Aes256Gcm::new(&key.into());

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    for byte in &mut nonce_bytes {
        *byte = rand::random();
    }
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt data using CA private key-derived key
pub fn decrypt_data_with_ca_key(encrypted: &[u8], ca_key_pem: &str) -> Result<Vec<u8>> {
    if encrypted.len() < NONCE_SIZE {
        anyhow::bail!("Invalid encrypted data: too short");
    }

    let key = derive_key_from_ca_key(ca_key_pem)?;
    let cipher = Aes256Gcm::new(&key.into());

    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
}

/// Load CA private key and encrypt config file
pub fn encrypt_config_file_with_ca_key(config_json: &str, config_path: &Path, ca_key_path: &Path) -> Result<()> {
    let ca_key_pem = fs::read_to_string(ca_key_path)
        .with_context(|| format!("Failed to read CA key from {}", ca_key_path.display()))?;
    
    let encrypted = encrypt_data_with_ca_key(config_json.as_bytes(), &ca_key_pem)?;
    fs::write(config_path, encrypted)
        .with_context(|| format!("Failed to write encrypted config to {}", config_path.display()))
}

/// Load CA private key and decrypt config file
pub fn decrypt_config_file_with_ca_key(config_path: &Path, ca_key_path: &Path) -> Result<String> {
    let ca_key_pem = fs::read_to_string(ca_key_path)
        .with_context(|| format!("Failed to read CA key from {}", ca_key_path.display()))?;
    
    let encrypted = fs::read(config_path)
        .with_context(|| format!("Failed to read encrypted config from {}", config_path.display()))?;
    let decrypted = decrypt_data_with_ca_key(&encrypted, &ca_key_pem)?;
    String::from_utf8(decrypted).with_context(|| "Invalid UTF-8 in decrypted config")
}

/// Get encryption password from environment or prompt (deprecated, kept for compatibility)
pub fn get_encryption_password() -> Result<String> {
    if let Ok(password) = std::env::var("FAKEKEY_PASSWORD") {
        Ok(password)
    } else {
        anyhow::bail!(
            "Encryption password required. Set FAKEKEY_PASSWORD environment variable."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let data = b"test data 12345";
        let password = "test_password";

        let encrypted = encrypt_data(data, password).unwrap();
        assert_ne!(encrypted, data);
        assert!(encrypted.len() > data.len());

        let decrypted = decrypt_data(&encrypted, password).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_wrong_password() {
        let data = b"test data";
        let encrypted = encrypt_data(data, "correct").unwrap();
        let result = decrypt_data(&encrypted, "wrong");
        assert!(result.is_err());
    }

    #[test]
    fn test_mask_sensitive() {
        let text = r#"{"api_key": "sk-proj-1234567890abcdef"}"#;
        let masked = mask_sensitive(text, &["api_key\": \""]);
        assert!(masked.contains("sk-p****cdef"));
    }

    #[test]
    fn test_derive_key_deterministic() {
        let key1 = derive_key("password123");
        let key2 = derive_key("password123");
        assert_eq!(key1, key2);

        let key3 = derive_key("different");
        assert_ne!(key1, key3);
    }
}
