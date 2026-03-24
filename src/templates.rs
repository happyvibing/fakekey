use crate::config::{ApiKeyConfig, ScanLocation};
use chrono::Utc;

/// Pre-defined API service templates
pub struct ServiceTemplate {
    pub name: &'static str,
    pub header_name: &'static str,
    pub scan_locations: Vec<ScanLocation>,
    pub key_pattern: &'static str,
    pub description: &'static str,
}

impl ServiceTemplate {
    pub fn to_api_key_config(&self, real_key: String, fake_key: String) -> ApiKeyConfig {
        ApiKeyConfig {
            service: self.name.to_string(),
            real_key,
            fake_key,
            header_name: self.header_name.to_string(),
            scan_locations: self.scan_locations.clone(),
            created_at: Utc::now(),
        }
    }
}

static OPENAI_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "openai",
    header_name: "Authorization",
    scan_locations: vec![],
    key_pattern: "sk-",
    description: "OpenAI API (api.openai.com)",
};

static ANTHROPIC_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "anthropic",
    header_name: "x-api-key",
    scan_locations: vec![],
    key_pattern: "sk-ant-",
    description: "Anthropic Claude API (api.anthropic.com)",
};

static GITHUB_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "github",
    header_name: "Authorization",
    scan_locations: vec![],
    key_pattern: "ghp_",
    description: "GitHub Personal Access Token (api.github.com)",
};

static GOOGLE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "google",
    header_name: "Authorization",
    scan_locations: vec![],
    key_pattern: "AIza",
    description: "Google Cloud API (googleapis.com)",
};

static HUGGINGFACE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "huggingface",
    header_name: "Authorization",
    scan_locations: vec![],
    key_pattern: "hf_",
    description: "Hugging Face API (huggingface.co)",
};

static DEEPSEEK_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "deepseek",
    header_name: "Authorization",
    scan_locations: vec![],
    key_pattern: "sk-",
    description: "DeepSeek API (api.deepseek.com)",
};

pub fn get_template(name: &str) -> Option<&'static ServiceTemplate> {
    match name.to_lowercase().as_str() {
        "openai" => Some(&OPENAI_TEMPLATE),
        "anthropic" | "claude" => Some(&ANTHROPIC_TEMPLATE),
        "github" => Some(&GITHUB_TEMPLATE),
        "google" => Some(&GOOGLE_TEMPLATE),
        "huggingface" | "hf" => Some(&HUGGINGFACE_TEMPLATE),
        "deepseek" => Some(&DEEPSEEK_TEMPLATE),
        _ => None,
    }
}

pub fn list_templates() -> Vec<&'static ServiceTemplate> {
    vec![
        &OPENAI_TEMPLATE,
        &ANTHROPIC_TEMPLATE,
        &GITHUB_TEMPLATE,
        &GOOGLE_TEMPLATE,
        &HUGGINGFACE_TEMPLATE,
        &DEEPSEEK_TEMPLATE,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_template() {
        assert!(get_template("openai").is_some());
        assert!(get_template("github").is_some());
        assert!(get_template("unknown").is_none());
    }

    #[test]
    fn test_template_to_config() {
        let template = get_template("openai").unwrap();
        let config = template.to_api_key_config(
            "sk-real123".to_string(),
            "sk-fake_fk".to_string(),
        );
        assert_eq!(config.service, "openai");
        assert_eq!(config.header_name, "Authorization");
    }
}
