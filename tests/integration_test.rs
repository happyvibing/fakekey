use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_key_replacement() {
    let mut key_map = HashMap::new();
    key_map.insert("sk-fake_fk".to_string(), "sk-real123".to_string());
    key_map.insert("ghp_fake_fk".to_string(), "ghp_real456".to_string());

    // Test header replacement
    let header = "Bearer sk-fake_fk";
    let (result, replaced) = fakekey::key_handler::replace_in_header_value(header, &key_map);
    assert!(replaced);
    assert_eq!(result, "Bearer sk-real123");

    // Test URL replacement
    let url = "https://api.example.com/v1?token=ghp_fake_fk&foo=bar";
    let (result, replaced) = fakekey::key_handler::replace_in_url(url, &key_map);
    assert!(replaced);
    assert_eq!(result, "https://api.example.com/v1?token=ghp_real456&foo=bar");

    // Test body replacement
    let body = br#"{"api_key": "sk-fake_fk"}"#;
    let (result, replaced) = fakekey::key_handler::replace_in_body(body, &key_map);
    assert!(replaced);
    assert_eq!(
        String::from_utf8(result).unwrap(),
        r#"{"api_key": "sk-real123"}"#
    );
}

#[test]
fn test_config_management() {
    use fakekey::config::{ApiKeyConfig, AppConfig, ProxyConfig, ScanLocation, SecurityConfig};
    
    let temp_dir = TempDir::new().unwrap();

    // Create a test config
    let mut config = AppConfig {
        proxy: ProxyConfig {
            port: 1157,
            log_level: "info".to_string(),
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            allowed_hosts: vec!["api.openai.com".to_string()],
        },
        api_keys: vec![ApiKeyConfig {
            service: "openai".to_string(),
            real_key: "sk-real123".to_string(),
            fake_key: "sk-fake_fk".to_string(),
            header_name: "Authorization".to_string(),
            scan_locations: vec![ScanLocation::Header("Authorization".to_string())],
            created_at: chrono::Utc::now(),
        }],
        security: SecurityConfig {
            encrypt_config: false,
        },
    };

    // Test key map building
    let key_map = config.build_key_map();
    assert_eq!(key_map.len(), 1);
    assert_eq!(key_map.get("sk-fake_fk"), Some(&"sk-real123".to_string()));

    // Test find by service
    assert!(config.find_by_service("openai").is_some());
    assert!(config.find_by_service("github").is_none());

    // Test remove by service
    assert!(config.remove_by_service("openai"));
    assert_eq!(config.api_keys.len(), 0);
}

#[test]
fn test_fake_key_generation() {
    use fakekey::config::generate_fake_key;

    let real_key = "sk-proj-1234567890abcdefghijk";
    let fake_key = generate_fake_key(real_key);

    assert_eq!(fake_key.len(), real_key.len());
    assert!(fake_key.ends_with("_fk"));
    assert_ne!(fake_key, real_key);
}

#[test]
fn test_key_masking() {
    use fakekey::key_handler::mask_key;

    let key = "sk-proj-1234567890abcdefghijk";
    let masked = mask_key(key);

    assert!(masked.starts_with("sk-p"));
    assert!(masked.ends_with("hijk"));
    assert!(masked.contains("****"));
    assert_ne!(masked, key);
}

#[test]
fn test_encryption() {
    use fakekey::security::{decrypt_data, encrypt_data};

    let data = b"sensitive config data";
    let password = "test_password_123";

    let encrypted = encrypt_data(data, password).unwrap();
    assert_ne!(encrypted.as_slice(), data);

    let decrypted = decrypt_data(&encrypted, password).unwrap();
    assert_eq!(decrypted, data);

    // Wrong password should fail
    let result = decrypt_data(&encrypted, "wrong_password");
    assert!(result.is_err());
}

#[test]
fn test_sensitive_masking() {
    use fakekey::security::mask_sensitive;

    let text = r#"{"api_key": "sk-proj-1234567890abcdef"}"#;
    let masked = mask_sensitive(text, &["api_key\": \""]);
    assert!(masked.contains("****"));
    assert!(!masked.contains("1234567890abcdef"));
}
