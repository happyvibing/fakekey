/// Pre-defined API service templates
pub struct ServiceTemplate {
    pub name: &'static str,
    pub header_name: &'static str,
    pub description: &'static str,
}

static OPENAI_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "openai",
    header_name: "Authorization",
    description: "OpenAI API (api.openai.com)",
};

static ANTHROPIC_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "anthropic",
    header_name: "x-api-key",
    description: "Anthropic Claude API (api.anthropic.com)",
};

static GITHUB_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "github",
    header_name: "Authorization",
    description: "GitHub Personal Access Token (api.github.com)",
};

static GOOGLE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "google",
    header_name: "Authorization",
    description: "Google Cloud API (googleapis.com)",
};

static HUGGINGFACE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "huggingface",
    header_name: "Authorization",
    description: "Hugging Face API (huggingface.co)",
};

static DEEPSEEK_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "deepseek",
    header_name: "Authorization",
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
}
