use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Aliyun,
    Zhipu,
    Ollama,
    Volcengine,
    Moonshot,
    DeepSeek,
    Google,
    AzureOpenAI,
    OpenAICompatible,
    OnetCli,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "openai",
            ProviderType::Anthropic => "anthropic",
            ProviderType::Aliyun => "aliyun",
            ProviderType::Zhipu => "zhipu",
            ProviderType::Ollama => "ollama",
            ProviderType::Volcengine => "volcengine",
            ProviderType::Moonshot => "moonshot",
            ProviderType::DeepSeek => "deepseek",
            ProviderType::Google => "google",
            ProviderType::AzureOpenAI => "azure_openai",
            ProviderType::OpenAICompatible => "openai_compatible",
            ProviderType::OnetCli => "onet_cli",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(ProviderType::OpenAI),
            "anthropic" => Some(ProviderType::Anthropic),
            "aliyun" => Some(ProviderType::Aliyun),
            "zhipu" => Some(ProviderType::Zhipu),
            "ollama" => Some(ProviderType::Ollama),
            "volcengine" => Some(ProviderType::Volcengine),
            "moonshot" => Some(ProviderType::Moonshot),
            "deepseek" => Some(ProviderType::DeepSeek),
            "google" => Some(ProviderType::Google),
            "azure_openai" => Some(ProviderType::AzureOpenAI),
            "openai_compatible" => Some(ProviderType::OpenAICompatible),
            "onet_cli" => Some(ProviderType::OnetCli),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Anthropic => "Anthropic",
            ProviderType::Aliyun => "Aliyun (DashScope)",
            ProviderType::Zhipu => "Zhipu (GLM)",
            ProviderType::Ollama => "Ollama",
            ProviderType::Volcengine => "Volcengine",
            ProviderType::Moonshot => "Moonshot",
            ProviderType::DeepSeek => "DeepSeek",
            ProviderType::Google => "Google (Gemini)",
            ProviderType::AzureOpenAI => "Azure OpenAI",
            ProviderType::OpenAICompatible => "OpenAI Compatible",
            ProviderType::OnetCli => "Onet CLI",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Aliyun,
            ProviderType::Zhipu,
            ProviderType::Ollama,
            ProviderType::Volcengine,
            ProviderType::Moonshot,
            ProviderType::DeepSeek,
            ProviderType::Google,
            ProviderType::AzureOpenAI,
            ProviderType::OpenAICompatible,
            ProviderType::OnetCli,
        ]
    }

    pub fn requires_api_key(&self) -> bool {
        !matches!(self, ProviderType::Ollama | ProviderType::OnetCli)
    }

    /// 是否为内置 provider（不需要用户配置）
    pub fn is_builtin(&self) -> bool {
        matches!(self, ProviderType::OnetCli)
    }

    /// 返回用户可配置的 provider 类型列表（不包含内置类型）
    pub fn user_configurable() -> Vec<Self> {
        Self::all()
            .into_iter()
            .filter(|p| !p.is_builtin())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: i64,
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub api_version: Option<String>,
    pub model: String,
    pub models: Vec<String>,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub enabled: bool,
    pub is_default: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            provider_type: ProviderType::OpenAI,
            api_key: None,
            api_base: None,
            api_version: None,
            model: String::new(),
            models: Vec::new(),
            max_tokens: None,
            temperature: None,
            enabled: true,
            is_default: false,
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// 内置 provider 的特殊 ID
pub const BUILTIN_ONET_CLI_ID: i64 = -1;

impl ProviderConfig {
    /// 创建内置的 OnetCli provider 配置
    pub fn builtin_onet_cli() -> Self {
        Self {
            id: BUILTIN_ONET_CLI_ID,
            name: "ONetCli AI".to_string(),
            provider_type: ProviderType::OnetCli,
            api_key: None,
            api_base: None,
            api_version: None,
            model: "qwen-plus".to_string(),
            models: Vec::new(),
            max_tokens: None,
            temperature: None,
            enabled: true,
            is_default: true,
            created_at: 0,
            updated_at: 0,
        }
    }

    /// 是否为内置 provider
    pub fn is_builtin(&self) -> bool {
        self.provider_type.is_builtin()
    }
}
