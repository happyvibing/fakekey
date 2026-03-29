/// Pre-defined API service templates
pub struct ServiceTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub default_endpoints: &'static [&'static str], // 默认端点列表
}

static OPENAI_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "openai",
    description: "OpenAI API (api.openai.com)",
    default_endpoints: &["api.openai.com"],
};

static ANTHROPIC_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "anthropic",
    description: "Anthropic Claude API (api.anthropic.com)",
    default_endpoints: &["api.anthropic.com"],
};

static GITHUB_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "github",
    description: "GitHub Personal Access Token (api.github.com)",
    default_endpoints: &["api.github.com"],
};

static GOOGLE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "google",
    description: "Google Cloud API (googleapis.com)",
    default_endpoints: &["googleapis.com", "ai.googleapis.com", "generativelanguage.googleapis.com"],
};

static HUGGINGFACE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "huggingface",
    description: "Hugging Face API (huggingface.co)",
    default_endpoints: &["api-inference.huggingface.co", "huggingface.co"],
};

static DEEPSEEK_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "deepseek",
    description: "DeepSeek API (api.deepseek.com)",
    default_endpoints: &["api.deepseek.com"],
};

static ZAI_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "zai",
    description: "ZAI API (api.z.ai | open.bigmodel.cn)",
    default_endpoints: &["api.z.ai", "open.bigmodel.cn"],
};

pub fn get_template(name: &str) -> Option<&'static ServiceTemplate> {
    match name.to_lowercase().as_str() {
        "openai" => Some(&OPENAI_TEMPLATE),
        "anthropic" | "claude" => Some(&ANTHROPIC_TEMPLATE),
        "github" => Some(&GITHUB_TEMPLATE),
        "google" => Some(&GOOGLE_TEMPLATE),
        "huggingface" | "hf" => Some(&HUGGINGFACE_TEMPLATE),
        "deepseek" => Some(&DEEPSEEK_TEMPLATE),
        "zai" => Some(&ZAI_TEMPLATE),
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
        &ZAI_TEMPLATE,
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
