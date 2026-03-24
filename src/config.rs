use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub proxy: ProxyConfig,
    pub api_keys: Vec<ApiKeyConfig>,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub service: String,
    pub real_key: String,
    pub fake_key: String,
    #[serde(default = "default_header_name")]
    pub header_name: String,
    #[serde(default)]
    pub scan_locations: Vec<ScanLocation>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "name")]
pub enum ScanLocation {
    #[serde(rename = "header")]
    Header(String),
    #[serde(rename = "url_param")]
    UrlParam(String),
    #[serde(rename = "json_body")]
    JsonBody(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub encrypt_config: bool,
}

fn default_port() -> u16 {
    1157
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_data_dir() -> String {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".fakekey").to_string_lossy().to_string()
}

fn default_header_name() -> String {
    "Authorization".to_string()
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            log_level: default_log_level(),
            data_dir: default_data_dir(),
            allowed_hosts: Vec::new(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            encrypt_config: false,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig::default(),
            api_keys: Vec::new(),
            security: SecurityConfig::default(),
        }
    }
}

impl AppConfig {
    /// Return the resolved data directory path (expanding ~)
    pub fn data_dir(&self) -> PathBuf {
        expand_tilde(&self.proxy.data_dir)
    }

    /// Load config from the default config file path
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: AppConfig = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse config file")?;
        Ok(config)
    }

    /// Save config to the default config file path
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }

    /// Return the default config file path: ~/.fakekey/config.yaml
    pub fn config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".fakekey").join("config.yaml")
    }

    /// Build a mapping from fake_key -> real_key for quick lookup
    pub fn build_key_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for key_config in &self.api_keys {
            map.insert(key_config.fake_key.clone(), key_config.real_key.clone());
        }
        map
    }

    /// Find an API key config by service name
    pub fn find_by_service(&self, service: &str) -> Option<&ApiKeyConfig> {
        self.api_keys.iter().find(|k| k.service == service)
    }

    /// Remove an API key config by service name. Returns true if removed.
    pub fn remove_by_service(&mut self, service: &str) -> bool {
        let before = self.api_keys.len();
        self.api_keys.retain(|k| k.service != service);
        self.api_keys.len() < before
    }
}

/// Generate a fake key from a real key.
/// Strategy: keep the same length, replace trailing chars with `_fk` suffix.
pub fn generate_fake_key(real_key: &str) -> String {
    let suffix = "_fk";
    if real_key.len() <= suffix.len() {
        // Key is too short, just append
        return format!("{}{}", real_key, suffix);
    }

    // Replace the last 3 characters with _fk
    let base = &real_key[..real_key.len() - suffix.len()];
    format!("{}{}", base, suffix)
}

/// Ensure uniqueness of the fake key among existing keys.
/// If there is a collision, append random chars before the suffix.
pub fn generate_unique_fake_key(real_key: &str, existing_fake_keys: &[String]) -> String {
    let mut fake = generate_fake_key(real_key);
    let mut attempts = 0;
    while existing_fake_keys.contains(&fake) && attempts < 100 {
        let mut rng = rand::thread_rng();
        let rand_char: char = rng.gen_range(b'a'..=b'z') as char;
        let suffix = format!("{}_fk", rand_char);
        let base = &real_key[..real_key.len().saturating_sub(suffix.len())];
        fake = format!("{}{}", base, suffix);
        attempts += 1;
    }
    fake
}

/// Expand ~ to the user's home directory
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

/// Initialize the data directory structure
pub fn init_data_dir(data_dir: &Path) -> Result<()> {
    let dirs_to_create = [
        data_dir.to_path_buf(),
        data_dir.join("certs"),
        data_dir.join("certs").join("ca"),
        data_dir.join("certs").join("cache"),
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
        assert!(fake.ends_with("_fk"));
    }

    #[test]
    fn test_generate_fake_key_short() {
        let real = "ab";
        let fake = generate_fake_key(real);
        assert_eq!(fake, "ab_fk");
    }

    #[test]
    fn test_unique_fake_key() {
        let real = "sk-proj-1234567890abcdefghijk";
        let existing = vec![generate_fake_key(real)];
        let fake = generate_unique_fake_key(real, &existing);
        assert!(fake.ends_with("_fk"));
        assert!(!existing.contains(&fake));
    }

    #[test]
    fn test_build_key_map() {
        let config = AppConfig {
            api_keys: vec![ApiKeyConfig {
                service: "openai".to_string(),
                real_key: "sk-real".to_string(),
                fake_key: "sk-fake_fk".to_string(),
                header_name: "Authorization".to_string(),
                scan_locations: vec![],
                created_at: Utc::now(),
            }],
            ..Default::default()
        };
        let map = config.build_key_map();
        assert_eq!(map.get("sk-fake_fk"), Some(&"sk-real".to_string()));
    }
}
