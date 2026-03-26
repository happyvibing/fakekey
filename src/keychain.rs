use anyhow::{Context, Result};
use rand::Rng;
use security_framework::passwords::{get_generic_password, set_generic_password, delete_generic_password};

const KEYCHAIN_SERVICE: &str = "fakekey";
const KEYCHAIN_ACCOUNT: &str = "encryption-key";

/// Get or create the encryption key from macOS Keychain
pub fn get_or_create_encryption_key() -> Result<[u8; 32]> {
    // Try to read existing key from Keychain
    match get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(password_data) => {
            // Found existing key in Keychain
            if password_data.len() != 32 {
                anyhow::bail!("Invalid encryption key length in Keychain: expected 32 bytes, got {}", password_data.len());
            }
            
            let mut key = [0u8; 32];
            key.copy_from_slice(&password_data);
            Ok(key)
        }
        Err(_) => {
            // No existing key, generate new random key
            let mut key = [0u8; 32];
            let mut rng = rand::rng();
            rng.fill(&mut key);
            
            // Store in Keychain
            set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, &key)
                .with_context(|| "Failed to store encryption key in Keychain")?;
            
            Ok(key)
        }
    }
}

/// Delete the encryption key from macOS Keychain (for cleanup/reset)
#[allow(dead_code)]
pub fn delete_encryption_key() -> Result<()> {
    match delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(_) => Ok(()),
        Err(e) => {
            // If key doesn't exist, that's fine
            if e.to_string().contains("errSecItemNotFound") {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to delete encryption key from Keychain: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_or_create_key() {
        // This test will interact with actual macOS Keychain
        // Clean up first
        let _ = delete_encryption_key();
        
        // First call should create new key
        let key1 = get_or_create_encryption_key().unwrap();
        assert_eq!(key1.len(), 32);
        
        // Second call should retrieve same key
        let key2 = get_or_create_encryption_key().unwrap();
        assert_eq!(key1, key2);
        
        // Cleanup
        let _ = delete_encryption_key();
    }
}
