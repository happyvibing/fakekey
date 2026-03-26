use anyhow::{Context, Result};
use rand::Rng;

const SERVICE_NAME: &str = "fakekey";
const ACCOUNT_NAME: &str = "encryption-key";

//
// ============================================================================
// macOS Implementation - Uses system Keychain
// ============================================================================
//
#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use security_framework::passwords::{get_generic_password, set_generic_password, delete_generic_password};

    pub fn get_or_create_encryption_key() -> Result<[u8; 32]> {
        match get_generic_password(SERVICE_NAME, ACCOUNT_NAME) {
            Ok(password_data) => {
                if password_data.len() != 32 {
                    anyhow::bail!("Invalid encryption key length in Keychain: expected 32 bytes, got {}", password_data.len());
                }
                
                let mut key = [0u8; 32];
                key.copy_from_slice(&password_data);
                Ok(key)
            }
            Err(_) => {
                let mut key = [0u8; 32];
                let mut rng = rand::rng();
                rng.fill(&mut key);
                
                set_generic_password(SERVICE_NAME, ACCOUNT_NAME, &key)
                    .with_context(|| "Failed to store encryption key in macOS Keychain")?;
                
                Ok(key)
            }
        }
    }

    pub fn delete_encryption_key() -> Result<()> {
        match delete_generic_password(SERVICE_NAME, ACCOUNT_NAME) {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("errSecItemNotFound") {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Failed to delete encryption key from Keychain: {}", e))
                }
            }
        }
    }
}

//
// ============================================================================
// Linux Implementation - Uses Secret Service (libsecret/GNOME Keyring)
// ============================================================================
//
#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use secret_service::{SecretService, EncryptionType};
    use std::collections::HashMap;

    pub fn get_or_create_encryption_key() -> Result<[u8; 32]> {
        let ss = SecretService::connect(EncryptionType::Dh)
            .with_context(|| "Failed to connect to Secret Service (ensure gnome-keyring or KWallet is running)")?;
        
        let collection = ss.get_default_collection()
            .with_context(|| "Failed to access default keyring collection")?;
        
        if collection.is_locked()? {
            collection.unlock()
                .with_context(|| "Failed to unlock keyring (may require user authentication)")?;
        }

        // Search for existing key
        let mut search_attrs = HashMap::new();
        search_attrs.insert("service", SERVICE_NAME);
        search_attrs.insert("account", ACCOUNT_NAME);
        
        let items = collection.search_items(search_attrs.clone())
            .with_context(|| "Failed to search keyring")?;
        
        if let Some(item) = items.first() {
            let secret = item.get_secret()
                .with_context(|| "Failed to retrieve secret from keyring")?;
            let key_hex = String::from_utf8(secret)
                .with_context(|| "Invalid UTF-8 in stored key")?;
            let key_bytes = hex::decode(key_hex.trim())
                .with_context(|| "Failed to decode stored encryption key")?;
            
            if key_bytes.len() != 32 {
                anyhow::bail!("Invalid encryption key length in keyring: expected 32 bytes, got {}", key_bytes.len());
            }
            
            let mut key = [0u8; 32];
            key.copy_from_slice(&key_bytes);
            Ok(key)
        } else {
            // Generate new key
            let mut key = [0u8; 32];
            let mut rng = rand::rng();
            rng.fill(&mut key);
            
            let key_hex = hex::encode(key);
            collection.create_item(
                "FakeKey Encryption Key",
                search_attrs,
                key_hex.as_bytes(),
                true, // replace if exists
                "text/plain",
            ).with_context(|| "Failed to store encryption key in keyring")?;
            
            Ok(key)
        }
    }

    pub fn delete_encryption_key() -> Result<()> {
        let ss = SecretService::connect(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;
        
        if collection.is_locked()? {
            collection.unlock()?;
        }

        let mut search_attrs = HashMap::new();
        search_attrs.insert("service", SERVICE_NAME);
        search_attrs.insert("account", ACCOUNT_NAME);
        
        let items = collection.search_items(search_attrs)?;
        
        for item in items {
            item.delete()?;
        }
        
        Ok(())
    }
}

//
// ============================================================================
// Windows Implementation - Uses Windows Credential Manager
// ============================================================================
//
#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredReadW, CredWriteW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    fn target_name() -> Vec<u16> {
        format!("{}:{}", SERVICE_NAME, ACCOUNT_NAME)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    }

    pub fn get_or_create_encryption_key() -> Result<[u8; 32]> {
        let target = target_name();
        
        unsafe {
            let mut cred_ptr = std::ptr::null_mut();
            let result = CredReadW(
                PWSTR(target.as_ptr() as *mut u16),
                CRED_TYPE_GENERIC,
                0,
                &mut cred_ptr,
            );
            
            if result.is_ok() && !cred_ptr.is_null() {
                let cred = &*cred_ptr;
                let secret_slice = std::slice::from_raw_parts(
                    cred.CredentialBlob,
                    cred.CredentialBlobSize as usize,
                );
                let key_hex = String::from_utf8_lossy(secret_slice);
                let key_bytes = hex::decode(key_hex.trim())
                    .with_context(|| "Failed to decode stored encryption key")?;
                
                windows::Win32::Security::Credentials::CredFree(cred_ptr as *const _);
                
                if key_bytes.len() != 32 {
                    anyhow::bail!("Invalid encryption key length in Credential Manager: expected 32 bytes, got {}", key_bytes.len());
                }
                
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);
                Ok(key)
            } else {
                // Generate new key
                let mut key = [0u8; 32];
                let mut rng = rand::rng();
                rng.fill(&mut key);
                
                let key_hex = hex::encode(key);
                let key_bytes = key_hex.as_bytes();
                
                let mut comment: Vec<u16> = "FakeKey encryption key"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                
                let credential = CREDENTIALW {
                    Flags: 0,
                    Type: CRED_TYPE_GENERIC,
                    TargetName: PWSTR(target.as_ptr() as *mut u16),
                    Comment: PWSTR(comment.as_mut_ptr()),
                    CredentialBlobSize: key_bytes.len() as u32,
                    CredentialBlob: key_bytes.as_ptr() as *mut u8,
                    Persist: windows::Win32::Security::Credentials::CRED_PERSIST_LOCAL_MACHINE,
                    ..Default::default()
                };
                
                CredWriteW(&credential, 0)
                    .with_context(|| "Failed to store encryption key in Windows Credential Manager")?;
                
                Ok(key)
            }
        }
    }

    pub fn delete_encryption_key() -> Result<()> {
        let target = target_name();
        
        unsafe {
            let result = CredDeleteW(
                PWSTR(target.as_ptr() as *mut u16),
                CRED_TYPE_GENERIC,
                0,
            );
            
            if result.is_err() {
                let err = windows::core::Error::from_win32();
                if err.code() == ERROR_NOT_FOUND.to_hresult() {
                    return Ok(());
                }
                return Err(anyhow::anyhow!("Failed to delete encryption key from Credential Manager: {}", err));
            }
            
            Ok(())
        }
    }
}

//
// ============================================================================
// Public API - Platform-independent interface
// ============================================================================
//

/// Get or create the encryption key from platform-specific secure storage
/// - macOS: Keychain
/// - Linux: Secret Service (libsecret/GNOME Keyring/KWallet)
/// - Windows: Credential Manager
pub fn get_or_create_encryption_key() -> Result<[u8; 32]> {
    platform::get_or_create_encryption_key()
}

/// Delete the encryption key from platform-specific secure storage
#[allow(dead_code)]
pub fn delete_encryption_key() -> Result<()> {
    platform::delete_encryption_key()
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
