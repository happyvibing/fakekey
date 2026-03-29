use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AppConfig {
    pub proxy: ProxyConfig,
    pub api_keys: Vec<ApiKeyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub name: String,
    pub encrypted_key: String, // Encrypted real API key
    pub fake_key: String,
    #[serde(default)]
    pub endpoints: Vec<String>, // 具体的端点域名列表
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}


fn default_port() -> u16 {
    1155
}

fn default_log_level() -> String {
    "info".to_string()
}


impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            log_level: default_log_level(),
        }
    }
}



impl AppConfig {
    /// Return the data directory path (always ~/.fakekey)
    pub fn data_dir(&self) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".fakekey")
    }

    /// Load config from the default config file path
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let mut config: AppConfig = serde_json::from_str(&content)
            .with_context(|| "Failed to parse config file")?;
        
        // Decrypt real keys using keychain, filter out keys that fail to decrypt
        let mut valid_keys = Vec::new();
        let mut skipped_count = 0;
        
        for mut key_config in config.api_keys {
            if !key_config.encrypted_key.is_empty() {
                match hex::decode(&key_config.encrypted_key)
                    .with_context(|| "Failed to decode encrypted real key")
                    .and_then(|encrypted_key| {
                        crate::security::decrypt_data(&encrypted_key)
                            .with_context(|| "Failed to decrypt real key")
                    })
                    .and_then(|decrypted_key| {
                        String::from_utf8(decrypted_key)
                            .with_context(|| "Failed to convert decrypted key to string")
                    })
                {
                    Ok(decrypted) => {
                        key_config.encrypted_key = decrypted;
                        valid_keys.push(key_config);
                    }
                    Err(_) => {
                        eprintln!("⚠️  Warning: Skipping key '{}' - decryption failed (encryption key may have changed)", key_config.name);
                        skipped_count += 1;
                    }
                }
            } else {
                valid_keys.push(key_config);
            }
        }
        
        config.api_keys = valid_keys;
        
        if skipped_count > 0 {
            eprintln!("⚠️  {} key(s) were skipped due to decryption failures.", skipped_count);
            eprintln!("   Please re-add them using 'fakekey add' command.");
        }
        
        // Log config load if audit logger is available
        if let Ok(data_dir_env) = std::env::var("FAKEKEY_DATA_DIR") {
            let data_dir_path = std::path::PathBuf::from(data_dir_env);
            if let Ok(logger) = crate::audit::AuditLogger::new(&data_dir_path) {
                let _ = logger.log(
                    crate::audit::AuditEventType::ConfigLoad,
                    "Configuration loaded successfully".to_string(),
                    true,
                );
            }
        }
        
        Ok(config)
    }

    /// Save config to the default config file path (encrypting real keys)
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Create a copy and encrypt real keys using keychain
        let mut config_to_save = self.clone();
        
        for key_config in &mut config_to_save.api_keys {
            if !key_config.encrypted_key.is_empty() {
                let encrypted_key = crate::security::encrypt_data(
                    key_config.encrypted_key.as_bytes()
                )?;
                key_config.encrypted_key = hex::encode(encrypted_key);
            }
        }
        
        let content = serde_json::to_string_pretty(&config_to_save)?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        // Log config save if audit logger is available
        if let Ok(data_dir_env) = std::env::var("FAKEKEY_DATA_DIR") {
            let data_dir_path = std::path::PathBuf::from(data_dir_env);
            if let Ok(logger) = crate::audit::AuditLogger::new(&data_dir_path) {
                let _ = logger.log(
                    crate::audit::AuditEventType::ConfigSave,
                    "Configuration saved (real keys encrypted)".to_string(),
                    true,
                );
            }
        }
        
        Ok(())
    }

    /// Return the default config file path: ~/.fakekey/config.json
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".fakekey")
            .join("config.json")
    }

    /// Build a mapping from fake_key -> real_key for quick lookup
    pub fn build_key_map(&self) -> HashMap<String, String> {
        self.api_keys
            .iter()
            .map(|key_config| (key_config.fake_key.clone(), key_config.encrypted_key.clone()))
            .collect()
    }

    /// Find an API key config by name
    pub fn find_by_name(&self, name: &str) -> Option<&ApiKeyConfig> {
        self.api_keys.iter().find(|k| k.name == name)
    }

    /// Remove an API key config by name. Returns true if removed.
    pub fn remove_by_name(&mut self, name: &str) -> bool {
        let before = self.api_keys.len();
        self.api_keys.retain(|k| k.name != name);
        self.api_keys.len() < before
    }

    /// Check if any configured API keys might be used for the given domain
    /// This helps determine if MITM is needed for a domain
    pub fn needs_mitm_for_domain(&self, domain: &str) -> bool {
        if self.api_keys.is_empty() {
            return false;
        }

        // Extract domain without port for comparison
        let clean_domain = domain.split(':').next().unwrap_or(domain);

        // Check if any API key config has this domain in its endpoints list
        for key_config in &self.api_keys {
            if key_config.endpoints.contains(&clean_domain.to_string()) {
                return true;
            }
        }

        false
    }
}

/// Generate a fake key from a real key.
/// Strategy: keep the same length, replace some trailing chars with `_fk` pattern.
pub fn generate_fake_key(real_key: &str) -> String {
    let key_len = real_key.len();
    
    if key_len < 8 {
        // For very short keys, just replace last 2 chars with _k
        let prefix = &real_key[..key_len.saturating_sub(2)];
        format!("{}_k", prefix)
    } else if key_len < 12 {
        // For short keys, replace last 3 chars with _fk
        let prefix = &real_key[..key_len - 3];
        format!("{}_fk", prefix)
    } else {
        // For normal keys, replace some characters in the middle with _fk_
        // Keep first 1/3 and last 1/3, replace middle 1/3 with _fk_
        let first_third = key_len / 3;
        let last_third = key_len - first_third;
        let prefix = &real_key[..first_third];
        let suffix = &real_key[last_third..];
        
        // Calculate how many chars we need for the middle part
        let middle_len = key_len - prefix.len() - suffix.len();
        let middle_pattern = if middle_len >= 3 {
            "_fk_".to_string()
        } else {
            "_k".to_string()
        };
        
        // Adjust to maintain exact length
        let mut result = format!("{}{}{}", prefix, middle_pattern, suffix);
        if result.len() > key_len {
            // Truncate if too long
            result.truncate(key_len);
        } else if result.len() < key_len {
            // Pad with random chars if too short
            let mut rng = rand::rng();
            while result.len() < key_len {
                let rand_char: char = rng.random_range(b'a'..=b'z') as char;
                result.push(rand_char);
            }
        }
        result
    }
}

/// Ensure uniqueness of the fake key among existing keys.
/// If there is a collision, modify the pattern while maintaining length.
pub fn generate_unique_fake_key(real_key: &str, existing_fake_keys: &[&str]) -> String {
    let mut fake = generate_fake_key(real_key);
    let mut attempts = 0;
    let key_len = real_key.len();
    
    while existing_fake_keys.iter().any(|k| k == &fake) && attempts < 100 {
        let mut rng = rand::rng();
        
        // Generate a unique variation while maintaining length
        if key_len < 8 {
            // For very short keys, use different single char
            let rand_char: char = rng.random_range(b'a'..=b'z') as char;
            let prefix = &real_key[..key_len.saturating_sub(2)];
            fake = format!("{}_{}", prefix, rand_char);
        } else if key_len < 12 {
            // For short keys, use different pattern
            let rand_char: char = rng.random_range(b'a'..=b'z') as char;
            let prefix = &real_key[..key_len - 4];
            fake = format!("{}_{}k", prefix, rand_char);
        } else {
            // For normal keys, modify the middle pattern
            let first_third = key_len / 3;
            let last_third = key_len - first_third;
            let prefix = &real_key[..first_third];
            let suffix = &real_key[last_third..];
            
            // Use different pattern with random chars
            let rand_char1: char = rng.random_range(b'a'..=b'z') as char;
            let rand_char2: char = rng.random_range(b'a'..=b'z') as char;
            let middle_pattern = format!("_{}_{}_", rand_char1, rand_char2);
            
            let mut result = format!("{}{}{}", prefix, middle_pattern, suffix);
            if result.len() > key_len {
                result.truncate(key_len);
            } else if result.len() < key_len {
                while result.len() < key_len {
                    let rand_char: char = rng.random_range(b'a'..=b'z') as char;
                    result.push(rand_char);
                }
            }
            fake = result;
        }
        
        attempts += 1;
    }
    fake
}

/// Expand ~ to the user's home directory
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix('~')
        && let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    PathBuf::from(path)
}

/// Initialize the data directory structure
pub fn init_data_dir(data_dir: &Path) -> Result<()> {
    let dirs_to_create = [
        data_dir.to_path_buf(),
        data_dir.join("certs"),
        data_dir.join("certs/ca"),
        data_dir.join("certs/cache"),
        data_dir.join("logs"),
    ];
    
    for dir in &dirs_to_create {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }

    // Set directory permissions on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o700);
        fs::set_permissions(data_dir, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_fake_key() {
        let real = "sk-proj-1234567890abcdefghijk";
        let fake = generate_fake_key(real);
        assert_eq!(fake.len(), real.len());
        assert!(fake.contains("_fk_")); // _fk_ pattern in the middle for long keys
        assert_ne!(fake, real);
    }

    #[test]
    fn test_generate_fake_key_short() {
        let real = "ab";
        let fake = generate_fake_key(real);
        assert!(fake.contains("_k")); // short keys use _k pattern
    }

    #[test]
    fn test_unique_fake_key() {
        let config = AppConfig {
            api_keys: vec![ApiKeyConfig {
                name: "my-openai-key".to_string(),
                encrypted_key: "sk-real".to_string(),
                fake_key: "sk-fake_fk".to_string(),
                endpoints: vec!["api.openai.com".to_string()],
                created_at: Utc::now(),
            }],
            ..Default::default()
        };
        let existing_fake_keys: Vec<_> = config.api_keys.iter().map(|k| k.fake_key.as_str()).collect();
        let fake_key = generate_unique_fake_key("sk-proj-1234567890abcdefghijk", &existing_fake_keys);
        assert_ne!(fake_key, "sk-proj-1234567890abcdefghijk");
        assert!(!existing_fake_keys.iter().any(|k| k == &fake_key));
    }

    #[test]
    fn test_build_key_map() {
        let config = AppConfig {
            api_keys: vec![ApiKeyConfig {
                name: "my-openai-key".to_string(),
                encrypted_key: "sk-real".to_string(),
                fake_key: "sk-fake_fk".to_string(),
                endpoints: vec!["api.openai.com".to_string()],
                created_at: Utc::now(),
            }],
            ..Default::default()
        };
        let map = config.build_key_map();
        assert_eq!(map.get("sk-fake_fk"), Some(&"sk-real".to_string()));
    }

    #[test]
    fn test_needs_mitm_for_domain() {
        let mut config = AppConfig::default();
        
        // Test with no API keys
        assert!(!config.needs_mitm_for_domain("api.openai.com"));
        
        // Add OpenAI key with endpoint
        config.api_keys.push(ApiKeyConfig {
            name: "openai-test".to_string(),
            encrypted_key: "sk-real".to_string(),
            fake_key: "sk-fake_fk".to_string(),
            endpoints: vec!["api.openai.com".to_string()],
            created_at: Utc::now(),
        });
        
        // Test OpenAI domains
        assert!(config.needs_mitm_for_domain("api.openai.com"));
        assert!(config.needs_mitm_for_domain("api.openai.com:443"));
        
        // Test non-OpenAI domains
        assert!(!config.needs_mitm_for_domain("api.github.com"));
        assert!(!config.needs_mitm_for_domain("googleapis.com"));
        assert!(!config.needs_mitm_for_domain("example.com"));
    }

    #[test]
    fn test_needs_mitm_multiple_endpoints() {
        let mut config = AppConfig::default();
        
        // Add API key with multiple endpoints
        config.api_keys.push(ApiKeyConfig {
            name: "multi-endpoint".to_string(),
            encrypted_key: "sk-real".to_string(),
            fake_key: "sk-fake_fk".to_string(),
            endpoints: vec![
                "api.openai.com".to_string(),
                "api.github.com".to_string(),
                "custom.example.com".to_string(),
            ],
            created_at: Utc::now(),
        });
        
        // Test all configured endpoints
        assert!(config.needs_mitm_for_domain("api.openai.com"));
        assert!(config.needs_mitm_for_domain("api.github.com"));
        assert!(config.needs_mitm_for_domain("custom.example.com"));
        
        // Test non-configured domains
        assert!(!config.needs_mitm_for_domain("googleapis.com"));
        assert!(!config.needs_mitm_for_domain("other.com"));
    }
}
