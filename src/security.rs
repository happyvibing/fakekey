use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use rcgen::KeyPair;
use sha2::{Digest, Sha256};

const NONCE_SIZE: usize = 12;

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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_sensitive() {
        let text = r#"{"api_key": "sk-proj-1234567890abcdef"}"#;
        let masked = mask_sensitive(text, &["api_key\": \""]);
        assert!(masked.contains("sk-p****cdef"));
    }
}
