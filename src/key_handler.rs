use std::collections::HashMap;

/// Replace all occurrences of fake keys with real keys in the given text.
/// Returns the modified text and the number of replacements made.
pub fn replace_keys(text: &str, key_map: &HashMap<String, String>) -> (String, usize) {
    let mut result = text.to_string();
    let mut count = 0;

    for (fake_key, real_key) in key_map {
        if result.contains(fake_key) {
            result = result.replace(fake_key, real_key);
            count += 1;
        }
    }

    (result, count)
}

/// Replace fake keys in HTTP header value (e.g., "Bearer sk-xxx_fk")
pub fn replace_in_header_value(value: &str, key_map: &HashMap<String, String>) -> (String, bool) {
    let (result, count) = replace_keys(value, key_map);
    (result, count > 0)
}

/// Replace fake keys in a URL query string
pub fn replace_in_url(url: &str, key_map: &HashMap<String, String>) -> (String, bool) {
    let (result, count) = replace_keys(url, key_map);
    (result, count > 0)
}


/// Mask a key for safe logging (show first 4 and last 4 chars)
pub fn mask_key(key: &str) -> String {
    if key.len() <= 10 {
        return "*".repeat(key.len());
    }
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{}{}{}", prefix, "*".repeat(key.len() - 8), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_keys() {
        let mut map = HashMap::new();
        map.insert("sk-fake_fk".to_string(), "sk-real123".to_string());

        let input = "Bearer sk-fake_fk";
        let (output, count) = replace_keys(input, &map);
        assert_eq!(output, "Bearer sk-real123");
        assert_eq!(count, 1);
    }


    #[test]
    fn test_replace_in_url() {
        let mut map = HashMap::new();
        map.insert("tok_fake_fk".to_string(), "tok_real123".to_string());

        let url = "https://api.example.com/v1?token=tok_fake_fk&foo=bar";
        let (result, replaced) = replace_in_url(url, &map);
        assert!(replaced);
        assert_eq!(result, "https://api.example.com/v1?token=tok_real123&foo=bar");
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("sk-proj-1234567890"), "sk-p**********7890");
        assert_eq!(mask_key("short"), "*****");
    }

    #[test]
    fn test_no_replacement() {
        let map = HashMap::new();
        let input = "Bearer sk-something";
        let (output, count) = replace_keys(input, &map);
        assert_eq!(output, input);
        assert_eq!(count, 0);
    }
}
