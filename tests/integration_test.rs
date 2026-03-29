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

}

#[test]
fn test_config_management() {
    use fakekey::config::{ApiKeyConfig, AppConfig, ProxyConfig};

    // Create a test config
    let mut config = AppConfig {
        proxy: ProxyConfig {
            port: 1155,
            log_level: "info".to_string(),
        },
        api_keys: vec![ApiKeyConfig {
            name: "openai".to_string(),
            encrypted_key: "sk-real123".to_string(),
            fake_key: "sk-fake_fk".to_string(),
            endpoints: vec!["api.openai.com".to_string()],
            created_at: chrono::Utc::now(),
        }],
    };

    // Test key map building
    let key_map = config.build_key_map();
    assert_eq!(key_map.len(), 1);
    assert_eq!(key_map.get("sk-fake_fk"), Some(&"sk-real123".to_string()));

    // Test find by name
    assert!(config.find_by_name("openai").is_some());
    assert!(config.find_by_name("github").is_none());

    // Test remove by name
    assert!(config.remove_by_name("openai"));
    assert_eq!(config.api_keys.len(), 0);
}

#[test]
fn test_fake_key_generation() {
    use fakekey::config::generate_fake_key;

    let real_key = "sk-proj-1234567890abcdefghijk";
    let fake_key = generate_fake_key(real_key);

    assert_eq!(fake_key.len(), real_key.len());
    assert!(fake_key.contains("_fk_")); // _fk_ pattern in the middle for long keys
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
fn test_sensitive_data_masking() {
    use fakekey::security::mask_sensitive;

    let text = r#"Authorization: Bearer sk-proj-1234567890abcdef other"#;
    let masked = mask_sensitive(text, &["Bearer "]);
    assert!(masked.contains("****"));
    assert!(!masked.contains("1234567890abcdef"));
}

#[test]
fn test_sensitive_masking() {
    use fakekey::security::mask_sensitive;

    let text = r#"{"api_key": "sk-proj-1234567890abcdef"}"#;
    let masked = mask_sensitive(text, &["api_key\": \""]);
    assert!(masked.contains("****"));
    assert!(!masked.contains("1234567890abcdef"));
}
