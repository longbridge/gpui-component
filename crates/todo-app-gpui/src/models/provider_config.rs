use gpui::SharedString;
use gpui_component::IconName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelCapability {
    Text,
    Vision,
    Audio,
    Tools,
    Reasoning,
    CodeGeneration,
    Multimodal,
    Embedding,
    ImageGeneration,
    VideoGeneration,
}

impl ModelCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelCapability::Text => "文本生成",
            ModelCapability::Vision => "视觉理解",
            ModelCapability::Audio => "音频处理",
            ModelCapability::Tools => "工具调用",
            ModelCapability::Reasoning => "深度思考",
            ModelCapability::CodeGeneration => "代码生成",
            ModelCapability::Multimodal => "多模态",
            ModelCapability::Embedding => "向量嵌入",
            ModelCapability::ImageGeneration => "图像生成",
            ModelCapability::VideoGeneration => "视频生成",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            ModelCapability::Text,
            ModelCapability::Vision,
            ModelCapability::Audio,
            ModelCapability::Tools,
            ModelCapability::Reasoning,
            ModelCapability::CodeGeneration,
            ModelCapability::Multimodal,
            ModelCapability::Embedding,
            ModelCapability::ImageGeneration,
            ModelCapability::VideoGeneration,
        ]
    }

    pub fn icon(&self) -> IconName {
        match self {
            ModelCapability::Text => IconName::LetterText,
            ModelCapability::Vision => IconName::Eye,
            ModelCapability::Audio => IconName::Mic,
            ModelCapability::Tools => IconName::Wrench,
            ModelCapability::Reasoning => IconName::Brain,
            ModelCapability::CodeGeneration => IconName::Code,
            ModelCapability::Multimodal => IconName::Layers,
            ModelCapability::Embedding => IconName::Zap,
            ModelCapability::ImageGeneration => IconName::Image,
            ModelCapability::VideoGeneration => IconName::Video,
        }
    }

    pub fn color(&self) -> gpui::Rgba {
        match self {
            ModelCapability::Text => gpui::rgb(0x3B82F6),
            ModelCapability::Vision => gpui::rgb(0x10B981),
            ModelCapability::Audio => gpui::rgb(0xF59E0B),
            ModelCapability::Tools => gpui::rgb(0xEF4444),
            ModelCapability::Reasoning => gpui::rgb(0x8B5CF6),
            ModelCapability::CodeGeneration => gpui::rgb(0x06B6D4),
            ModelCapability::Multimodal => gpui::rgb(0xEC4899),
            ModelCapability::Embedding => gpui::rgb(0x84CC16),
            ModelCapability::ImageGeneration => gpui::rgb(0xF97316),
            ModelCapability::VideoGeneration => gpui::rgb(0xDC2626),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    ApiKey,
    OAuth,
    Bearer,
    Basic,
    Custom,
}

impl AuthMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthMethod::ApiKey => "API Key",
            AuthMethod::OAuth => "OAuth 2.0",
            AuthMethod::Bearer => "Bearer Token",
            AuthMethod::Basic => "Basic Auth",
            AuthMethod::Custom => "自定义",
        }
    }

    pub fn all() -> Vec<SharedString> {
        vec![
            "API Key".into(),
            "OAuth 2.0".into(),
            "Bearer Token".into(),
            "Basic Auth".into(),
            "自定义".into(),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_price_per_1k: Option<f64>,
    pub output_price_per_1k: Option<f64>,
    pub currency: String,
    pub billing_unit: String, // "token", "character", "request"
}

impl Default for ModelPricing {
    fn default() -> Self {
        Self {
            input_price_per_1k: None,
            output_price_per_1k: None,
            currency: "USD".to_string(),
            billing_unit: "token".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimits {
    pub context_length: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub max_requests_per_minute: Option<u32>,
    pub max_requests_per_day: Option<u32>,
    pub max_tokens_per_minute: Option<u32>,
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            context_length: Some(4096),
            max_output_tokens: Some(2048),
            max_requests_per_minute: None,
            max_requests_per_day: None,
            max_tokens_per_minute: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModel {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub capabilities: Vec<ModelCapability>,
    pub limits: ModelLimits,
    pub pricing: ModelPricing,
    pub enabled: bool,
    pub version: String,
    pub release_date: Option<String>,
    pub deprecated: bool,
    pub beta: bool,
    pub tags: Vec<String>,
    pub custom_parameters: HashMap<String, serde_json::Value>,
}

impl Default for LlmModel {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            description: String::new(),
            capabilities: vec![ModelCapability::Text],
            limits: ModelLimits::default(),
            pricing: ModelPricing::default(),
            enabled: true,
            version: "1.0".to_string(),
            release_date: None,
            deprecated: false,
            beta: false,
            tags: vec![],
            custom_parameters: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub name: String,
    pub url: String,
    pub method: String, // GET, POST, etc.
    pub headers: HashMap<String, String>,
    pub timeout: Option<u32>,
}

impl Default for ApiEndpoint {
    fn default() -> Self {
        Self {
            name: "chat".to_string(),
            url: "/v1/chat/completions".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            timeout: Some(60),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderInfo {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub base_url: String,
    pub auth_method: AuthMethod,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub custom_headers: HashMap<String, String>,
    pub endpoints: HashMap<String, ApiEndpoint>,
    pub models: Vec<LlmModel>,
    pub enabled: bool,
    pub region: Option<String>,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub rate_limits: HashMap<String, u32>,
    pub retry_config: RetryConfig,
    pub proxy_url: Option<String>,
    pub verify_ssl: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: u64, // milliseconds
    pub max_delay: u64,     // milliseconds
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: 1000,
            max_delay: 32000,
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for LlmProviderInfo {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            display_name: String::new(),
            description: String::new(),
            version: "1.0.0".to_string(),
            base_url: String::new(),
            auth_method: AuthMethod::ApiKey,
            api_key: None,
            api_secret: None,
            custom_headers: HashMap::new(),
            endpoints: HashMap::from([("chat".to_string(), ApiEndpoint::default())]),
            models: vec![],
            enabled: true,
            region: None,
            organization: None,
            project: None,
            rate_limits: HashMap::new(),
            retry_config: RetryConfig::default(),
            proxy_url: None,
            verify_ssl: true,
            tags: vec![],
        }
    }
}

impl LlmProviderInfo {
    // 预设的常用 LLM 提供商配置
    pub fn openai() -> Self {
        Self {
            id: "openai".to_string(),
            name: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            description: "OpenAI官方API服务".to_string(),
            base_url: "https://api.openai.com".to_string(),
            auth_method: AuthMethod::Bearer,
            endpoints: HashMap::from([
                (
                    "chat".to_string(),
                    ApiEndpoint {
                        name: "chat".to_string(),
                        url: "/v1/chat/completions".to_string(),
                        method: "POST".to_string(),
                        headers: HashMap::from([(
                            "Content-Type".to_string(),
                            "application/json".to_string(),
                        )]),
                        timeout: Some(60),
                    },
                ),
                (
                    "embeddings".to_string(),
                    ApiEndpoint {
                        name: "embeddings".to_string(),
                        url: "/v1/embeddings".to_string(),
                        method: "POST".to_string(),
                        headers: HashMap::from([(
                            "Content-Type".to_string(),
                            "application/json".to_string(),
                        )]),
                        timeout: Some(30),
                    },
                ),
            ]),
            models: vec![
                LlmModel {
                    id: "gpt-4o".to_string(),
                    display_name: "GPT-4o".to_string(),
                    description: "最新的GPT-4 Omni模型，支持多模态".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Tools,
                        ModelCapability::Reasoning,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Multimodal,
                    ],
                    limits: ModelLimits {
                        context_length: Some(128000),
                        max_output_tokens: Some(4096),
                        max_requests_per_minute: Some(5000),
                        max_tokens_per_minute: Some(800000),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(5.0),
                        output_price_per_1k: Some(15.0),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["latest".to_string(), "multimodal".to_string()],
                    ..Default::default()
                },
                LlmModel {
                    id: "gpt-4o-mini".to_string(),
                    display_name: "GPT-4o Mini".to_string(),
                    description: "轻量级的GPT-4o模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Tools,
                        ModelCapability::CodeGeneration,
                    ],
                    limits: ModelLimits {
                        context_length: Some(128000),
                        max_output_tokens: Some(16384),
                        max_requests_per_minute: Some(10000),
                        max_tokens_per_minute: Some(2000000),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(0.15),
                        output_price_per_1k: Some(0.6),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["fast".to_string(), "cheap".to_string()],
                    ..Default::default()
                },
                LlmModel {
                    id: "o1-preview".to_string(),
                    display_name: "o1-preview".to_string(),
                    description: "OpenAI推理模型预览版".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Reasoning,
                        ModelCapability::CodeGeneration,
                    ],
                    limits: ModelLimits {
                        context_length: Some(128000),
                        max_output_tokens: Some(32768),
                        max_requests_per_minute: Some(20),
                        max_requests_per_day: Some(50),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(15.0),
                        output_price_per_1k: Some(60.0),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    beta: true,
                    tags: vec!["reasoning".to_string(), "preview".to_string()],
                    ..Default::default()
                },
            ],
            rate_limits: HashMap::from([
                ("requests_per_minute".to_string(), 5000),
                ("tokens_per_minute".to_string(), 800000),
            ]),
            tags: vec!["official".to_string(), "popular".to_string()],
            ..Default::default()
        }
    }

    pub fn anthropic() -> Self {
        Self {
            id: "anthropic".to_string(),
            name: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            description: "Anthropic Claude系列模型".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            auth_method: AuthMethod::ApiKey,
            custom_headers: HashMap::from([(
                "anthropic-version".to_string(),
                "2023-06-01".to_string(),
            )]),
            endpoints: HashMap::from([(
                "chat".to_string(),
                ApiEndpoint {
                    name: "messages".to_string(),
                    url: "/v1/messages".to_string(),
                    method: "POST".to_string(),
                    headers: HashMap::from([(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    timeout: Some(60),
                },
            )]),
            models: vec![
                LlmModel {
                    id: "claude-3-5-sonnet-20241022".to_string(),
                    display_name: "Claude 3.5 Sonnet".to_string(),
                    description: "Anthropic最新的Claude 3.5 Sonnet模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Tools,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Reasoning,
                    ],
                    limits: ModelLimits {
                        context_length: Some(200000),
                        max_output_tokens: Some(8192),
                        max_requests_per_minute: Some(1000),
                        max_tokens_per_minute: Some(40000),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(3.0),
                        output_price_per_1k: Some(15.0),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["latest".to_string(), "reasoning".to_string()],
                    ..Default::default()
                },
                LlmModel {
                    id: "claude-3-haiku-20240307".to_string(),
                    display_name: "Claude 3 Haiku".to_string(),
                    description: "快速且经济的Claude 3 Haiku模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Tools,
                    ],
                    limits: ModelLimits {
                        context_length: Some(200000),
                        max_output_tokens: Some(4096),
                        max_requests_per_minute: Some(2000),
                        max_tokens_per_minute: Some(100000),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(0.25),
                        output_price_per_1k: Some(1.25),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["fast".to_string(), "cheap".to_string()],
                    ..Default::default()
                },
            ],
            tags: vec!["safe".to_string(), "reliable".to_string()],
            ..Default::default()
        }
    }

    pub fn google() -> Self {
        Self {
            id: "google".to_string(),
            name: "google".to_string(),
            display_name: "Google".to_string(),
            description: "Google Gemini系列模型".to_string(),
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            auth_method: AuthMethod::ApiKey,
            endpoints: HashMap::from([(
                "chat".to_string(),
                ApiEndpoint {
                    name: "generateContent".to_string(),
                    url: "/v1beta/models/{model}:generateContent".to_string(),
                    method: "POST".to_string(),
                    headers: HashMap::from([(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    timeout: Some(60),
                },
            )]),
            models: vec![
                LlmModel {
                    id: "gemini-1.5-pro".to_string(),
                    display_name: "Gemini 1.5 Pro".to_string(),
                    description: "Google最先进的Gemini模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Audio,
                        ModelCapability::Tools,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Multimodal,
                    ],
                    limits: ModelLimits {
                        context_length: Some(2097152), // 2M tokens
                        max_output_tokens: Some(8192),
                        max_requests_per_minute: Some(360),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(1.25),
                        output_price_per_1k: Some(5.0),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["long-context".to_string(), "multimodal".to_string()],
                    ..Default::default()
                },
                LlmModel {
                    id: "gemini-1.5-flash".to_string(),
                    display_name: "Gemini 1.5 Flash".to_string(),
                    description: "快速的Gemini模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Audio,
                        ModelCapability::Tools,
                        ModelCapability::Multimodal,
                    ],
                    limits: ModelLimits {
                        context_length: Some(1048576), // 1M tokens
                        max_output_tokens: Some(8192),
                        max_requests_per_minute: Some(1000),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(0.075),
                        output_price_per_1k: Some(0.3),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["fast".to_string(), "cheap".to_string()],
                    ..Default::default()
                },
            ],
            tags: vec!["multimodal".to_string(), "long-context".to_string()],
            ..Default::default()
        }
    }

    pub fn deepseek() -> Self {
        Self {
            id: "deepseek".to_string(),
            name: "deepseek".to_string(),
            display_name: "DeepSeek".to_string(),
            description: "DeepSeek AI模型服务".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            auth_method: AuthMethod::Bearer,
            endpoints: HashMap::from([(
                "chat".to_string(),
                ApiEndpoint {
                    name: "chat".to_string(),
                    url: "/v1/chat/completions".to_string(),
                    method: "POST".to_string(),
                    headers: HashMap::from([(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    timeout: Some(60),
                },
            )]),
            models: vec![
                LlmModel {
                    id: "deepseek-chat".to_string(),
                    display_name: "DeepSeek Chat".to_string(),
                    description: "DeepSeek对话模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Tools,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Reasoning,
                    ],
                    limits: ModelLimits {
                        context_length: Some(32768),
                        max_output_tokens: Some(4096),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(0.14),
                        output_price_per_1k: Some(0.28),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["chinese".to_string(), "cheap".to_string()],
                    ..Default::default()
                },
                LlmModel {
                    id: "deepseek-coder".to_string(),
                    display_name: "DeepSeek Coder".to_string(),
                    description: "专门用于代码生成的模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Tools,
                    ],
                    limits: ModelLimits {
                        context_length: Some(16384),
                        max_output_tokens: Some(4096),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(0.14),
                        output_price_per_1k: Some(0.28),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["coding".to_string(), "chinese".to_string()],
                    ..Default::default()
                },
            ],
            tags: vec!["chinese".to_string(), "affordable".to_string()],
            ..Default::default()
        }
    }

    pub fn ollama() -> Self {
        Self {
            id: "ollama".to_string(),
            name: "ollama".to_string(),
            display_name: "Ollama".to_string(),
            description: "本地运行的大语言模型".to_string(),
            base_url: "http://localhost:11434".to_string(),
            auth_method: AuthMethod::Bearer, // Ollama通常不需要认证
            endpoints: HashMap::from([
                (
                    "chat".to_string(),
                    ApiEndpoint {
                        name: "chat".to_string(),
                        url: "/api/chat".to_string(),
                        method: "POST".to_string(),
                        headers: HashMap::from([(
                            "Content-Type".to_string(),
                            "application/json".to_string(),
                        )]),
                        timeout: Some(120), // 本地模型可能需要更长时间
                    },
                ),
                (
                    "generate".to_string(),
                    ApiEndpoint {
                        name: "generate".to_string(),
                        url: "/api/generate".to_string(),
                        method: "POST".to_string(),
                        headers: HashMap::from([(
                            "Content-Type".to_string(),
                            "application/json".to_string(),
                        )]),
                        timeout: Some(120),
                    },
                ),
            ]),
            models: vec![
                LlmModel {
                    id: "llama3.2".to_string(),
                    display_name: "Llama 3.2".to_string(),
                    description: "Meta Llama 3.2模型".to_string(),
                    capabilities: vec![ModelCapability::Text, ModelCapability::CodeGeneration],
                    limits: ModelLimits {
                        context_length: Some(128000),
                        max_output_tokens: Some(4096),
                        max_requests_per_minute: None, // 本地无限制
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(0.0), // 本地免费
                        output_price_per_1k: Some(0.0),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec!["local".to_string(), "free".to_string()],
                    ..Default::default()
                },
                LlmModel {
                    id: "qwen2.5".to_string(),
                    display_name: "Qwen 2.5".to_string(),
                    description: "阿里巴巴通义千问2.5模型".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Reasoning,
                    ],
                    limits: ModelLimits {
                        context_length: Some(32768),
                        max_output_tokens: Some(8192),
                        ..Default::default()
                    },
                    pricing: ModelPricing {
                        input_price_per_1k: Some(0.0),
                        output_price_per_1k: Some(0.0),
                        currency: "USD".to_string(),
                        billing_unit: "token".to_string(),
                    },
                    tags: vec![
                        "local".to_string(),
                        "chinese".to_string(),
                        "free".to_string(),
                    ],
                    ..Default::default()
                },
            ],
            verify_ssl: false, // 本地服务可能使用HTTP
            tags: vec![
                "local".to_string(),
                "free".to_string(),
                "privacy".to_string(),
            ],
            ..Default::default()
        }
    }

    // 保存为YAML文件
    pub fn save_to_yaml(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    // 从YAML文件加载
    pub fn load_from_yaml(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    // 批量保存多个配置
    pub fn save_configs_to_yaml(
        configs: &[Self],
        dir_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(dir_path)?;

        for config in configs {
            let file_path = format!("{}/{}.yaml", dir_path, config.name);
            config.save_to_yaml(&file_path)?;
        }
        Ok(())
    }

    // 从目录加载所有配置
    pub fn load_configs_from_dir(dir_path: &str) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut configs = Vec::new();

        if !std::path::Path::new(dir_path).exists() {
            return Ok(configs);
        }

        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                if let Ok(config) = Self::load_from_yaml(path.to_str().unwrap()) {
                    configs.push(config);
                }
            }
        }

        Ok(configs)
    }

    // 验证配置
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("提供商名称不能为空".to_string());
        }

        if self.display_name.is_empty() {
            errors.push("显示名称不能为空".to_string());
        }

        if self.base_url.is_empty() {
            errors.push("API基础URL不能为空".to_string());
        }

        // 验证URL格式
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            errors.push("API基础URL必须以http://或https://开头".to_string());
        }

        // 验证认证配置
        match self.auth_method {
            AuthMethod::ApiKey | AuthMethod::Bearer => {
                if self.api_key.is_none() || self.api_key.as_ref().unwrap().is_empty() {
                    errors.push("API Key不能为空".to_string());
                }
            }
            AuthMethod::Basic => {
                if self.api_key.is_none() || self.api_secret.is_none() {
                    errors.push("Basic认证需要用户名和密码".to_string());
                }
            }
            _ => {}
        }

        // 验证模型配置
        if self.models.is_empty() {
            errors.push("至少需要配置一个模型".to_string());
        }

        for model in &self.models {
            if model.id.is_empty() {
                errors.push("模型ID不能为空".to_string());
            }
            if model.display_name.is_empty() {
                errors.push(format!("模型 '{}' 的显示名称不能为空", model.id));
            }
            if model.capabilities.is_empty() {
                errors.push(format!("模型 '{}' 至少需要一个能力", model.id));
            }
        }

        // 验证端点配置
        if self.endpoints.is_empty() {
            errors.push("至少需要配置一个API端点".to_string());
        }

        for (name, endpoint) in &self.endpoints {
            if endpoint.url.is_empty() {
                errors.push(format!("端点 '{}' 的URL不能为空", name));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // 获取所有预设配置
    pub fn get_presets() -> Vec<Self> {
        vec![
            Self::openai(),
            Self::anthropic(),
            Self::google(),
            Self::deepseek(),
            Self::ollama(),
        ]
    }

    // 获取启用的模型
    pub fn get_enabled_models(&self) -> Vec<&LlmModel> {
        self.models.iter().filter(|m| m.enabled).collect()
    }

    // 按能力筛选模型
    pub fn get_models_by_capability(&self, capability: &ModelCapability) -> Vec<&LlmModel> {
        self.models
            .iter()
            .filter(|m| m.enabled && m.capabilities.contains(capability))
            .collect()
    }

    // 估算成本
    pub fn estimate_cost(
        &self,
        model_id: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Option<f64> {
        let model = self.models.iter().find(|m| m.id == model_id)?;
        let input_cost = model.pricing.input_price_per_1k? * (input_tokens as f64 / 1000.0);
        let output_cost = model.pricing.output_price_per_1k? * (output_tokens as f64 / 1000.0);
        Some(input_cost + output_cost)
    }
}

// 配置管理器
#[derive(Debug)]
pub struct LlmProviderConfigManager {
    config_dir: String,
    configs: Vec<LlmProviderInfo>,
}

impl LlmProviderConfigManager {
    pub fn new(config_dir: &str) -> Self {
        Self {
            config_dir: config_dir.to_string(),
            configs: Vec::new(),
        }
    }

    pub fn load_configs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.configs = LlmProviderInfo::load_configs_from_dir(&self.config_dir)?;
        Ok(())
    }

    pub fn save_configs(&self) -> Result<(), Box<dyn std::error::Error>> {
        LlmProviderInfo::save_configs_to_yaml(&self.configs, &self.config_dir)
    }

    pub fn add_config(&mut self, config: LlmProviderInfo) -> Result<(), Vec<String>> {
        config.validate()?;

        // 检查重复名称
        if self.configs.iter().any(|c| c.name == config.name) {
            return Err(vec![format!("提供商名称 '{}' 已存在", config.name)]);
        }

        self.configs.push(config);
        Ok(())
    }

    pub fn update_config(
        &mut self,
        index: usize,
        config: LlmProviderInfo,
    ) -> Result<(), Vec<String>> {
        config.validate()?;

        if index >= self.configs.len() {
            return Err(vec!["配置索引超出范围".to_string()]);
        }

        // 检查重复名称（排除自己）
        if self
            .configs
            .iter()
            .enumerate()
            .any(|(i, c)| i != index && c.name == config.name)
        {
            return Err(vec![format!("提供商名称 '{}' 已存在", config.name)]);
        }

        self.configs[index] = config;
        Ok(())
    }

    pub fn remove_config(&mut self, index: usize) -> Result<(), String> {
        if index >= self.configs.len() {
            return Err("配置索引超出范围".to_string());
        }

        self.configs.remove(index);
        Ok(())
    }

    pub fn get_configs(&self) -> &[LlmProviderInfo] {
        &self.configs
    }

    pub fn get_config(&self, index: usize) -> Option<&LlmProviderInfo> {
        self.configs.get(index)
    }

    pub fn get_config_mut(&mut self, index: usize) -> Option<&mut LlmProviderInfo> {
        self.configs.get_mut(index)
    }

    pub fn get_config_by_name(&self, name: &str) -> Option<&LlmProviderInfo> {
        self.configs.iter().find(|c| c.name == name)
    }

    pub fn get_enabled_configs(&self) -> Vec<&LlmProviderInfo> {
        self.configs.iter().filter(|c| c.enabled).collect()
    }

    pub fn get_all_models(&self) -> Vec<(&LlmProviderInfo, &LlmModel)> {
        self.configs
            .iter()
            .filter(|p| p.enabled)
            .flat_map(|p| p.models.iter().filter(|m| m.enabled).map(move |m| (p, m)))
            .collect()
    }

    pub fn get_models_by_capability(
        &self,
        capability: &ModelCapability,
    ) -> Vec<(&LlmProviderInfo, &LlmModel)> {
        self.get_all_models()
            .into_iter()
            .filter(|(_, m)| m.capabilities.contains(capability))
            .collect()
    }

    pub fn init_with_presets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.configs.is_empty() {
            self.configs = LlmProviderInfo::get_presets();
            self.save_configs()?;
        }
        Ok(())
    }

    // 搜索模型
    pub fn search_models(&self, query: &str) -> Vec<(&LlmProviderInfo, &LlmModel)> {
        let query = query.to_lowercase();
        self.get_all_models()
            .into_iter()
            .filter(|(p, m)| {
                p.display_name.to_lowercase().contains(&query)
                    || m.display_name.to_lowercase().contains(&query)
                    || m.description.to_lowercase().contains(&query)
                    || m.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_serialization() {
        let config = LlmProviderInfo::openai();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: LlmProviderInfo = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.display_name, deserialized.display_name);
        assert_eq!(config.models.len(), deserialized.models.len());
    }

    #[test]
    fn test_config_validation() {
        let mut config = LlmProviderInfo::default();
        config.name = "test".to_string();
        config.display_name = "Test".to_string();
        config.base_url = "https://api.example.com".to_string();
        config.api_key = Some("test-key".to_string());
        config.models.push(LlmModel {
            id: "test-model".to_string(),
            display_name: "Test Model".to_string(),
            ..Default::default()
        });

        assert!(config.validate().is_ok());

        config.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_str().unwrap();

        let mut manager = LlmProviderConfigManager::new(config_dir);
        let config = LlmProviderInfo::openai();

        assert!(manager.add_config(config.clone()).is_ok());
        assert!(manager.save_configs().is_ok());

        let mut new_manager = LlmProviderConfigManager::new(config_dir);
        assert!(new_manager.load_configs().is_ok());
        assert_eq!(new_manager.get_configs().len(), 1);
        assert_eq!(new_manager.get_configs()[0].name, config.name);
    }

    #[test]
    fn test_cost_estimation() {
        let config = LlmProviderInfo::openai();
        let cost = config.estimate_cost("gpt-4o", 1000, 500);
        assert!(cost.is_some());
        assert_eq!(cost.unwrap(), 12.5); // 5.0 + 7.5
    }

    #[test]
    fn test_capability_filtering() {
        let config = LlmProviderInfo::openai();
        let vision_models = config.get_models_by_capability(&ModelCapability::Vision);
        assert!(!vision_models.is_empty());

        let reasoning_models = config.get_models_by_capability(&ModelCapability::Reasoning);
        assert!(!reasoning_models.is_empty());
    }
}
