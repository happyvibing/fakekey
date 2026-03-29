/// Pre-defined API service templates
pub struct ServiceTemplate {
    pub name: &'static str,
    pub header_name: &'static str,
    pub description: &'static str,
    pub default_endpoints: &'static [&'static str], // 默认端点列表
}

static OPENAI_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "openai",
    header_name: "Authorization",
    description: "OpenAI API (api.openai.com)",
    default_endpoints: &["api.openai.com"],
};

static ANTHROPIC_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "anthropic",
    header_name: "x-api-key",
    description: "Anthropic Claude API (api.anthropic.com)",
    default_endpoints: &["api.anthropic.com"],
};

static GITHUB_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "github",
    header_name: "Authorization",
    description: "GitHub Personal Access Token (api.github.com)",
    default_endpoints: &["api.github.com"],
};

static GOOGLE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "google",
    header_name: "Authorization",
    description: "Google Cloud API (googleapis.com)",
    default_endpoints: &["googleapis.com", "ai.googleapis.com", "generativelanguage.googleapis.com"],
};

static HUGGINGFACE_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "huggingface",
    header_name: "Authorization",
    description: "Hugging Face API (huggingface.co)",
    default_endpoints: &["api-inference.huggingface.co", "huggingface.co"],
};

static DEEPSEEK_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "deepseek",
    header_name: "Authorization",
    description: "DeepSeek API (api.deepseek.com)",
    default_endpoints: &["api.deepseek.com"],
};

static ZHIPU_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "zhipu",
    header_name: "Authorization",
    description: "Zhipu AI API (open.bigmodel.cn)",
    default_endpoints: &["open.bigmodel.cn"],
};

static ZAI_TEMPLATE: ServiceTemplate = ServiceTemplate {
    name: "zai",
    header_name: "Authorization",
    description: "ZAI API (api.z.ai)",
    default_endpoints: &["api.z.ai"],
};

pub fn get_template(name: &str) -> Option<&'static ServiceTemplate> {
    match name.to_lowercase().as_str() {
        "openai" => Some(&OPENAI_TEMPLATE),
        "anthropic" | "claude" => Some(&ANTHROPIC_TEMPLATE),
        "github" => Some(&GITHUB_TEMPLATE),
        "google" => Some(&GOOGLE_TEMPLATE),
        "huggingface" | "hf" => Some(&HUGGINGFACE_TEMPLATE),
        "deepseek" => Some(&DEEPSEEK_TEMPLATE),
        "zhipu" => Some(&ZHIPU_TEMPLATE),
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
        &ZHIPU_TEMPLATE,
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
