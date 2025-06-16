use gpui::SharedString;
use gpui_component::IconName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ApiType {
    OpenAI,
    OpenAIResponse,
    Gemini,
    Anthropic,
    AzureOpenAI,
}

impl ApiType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiType::OpenAI => "OpenAI",
            ApiType::OpenAIResponse => "OpenAI-Response",
            ApiType::Gemini => "Gemini",
            ApiType::Anthropic => "Anthropic",
            ApiType::AzureOpenAI => "Azure-OpenAI",
        }
    }

    pub fn all() -> Vec<SharedString> {
        vec![
            "OpenAI".into(),
            "OpenAI-Response".into(),
            "Gemini".into(),
            "Anthropic".into(),
            "Azure-OpenAI".into(),
        ]
    }
}

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

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub capabilities: Vec<ModelCapability>,
    pub enabled: bool,
    pub limits: ModelLimits,
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

#[derive(Debug, Clone)]
pub struct LlmProviderInfo {
    pub id: String,
    pub name: String,
    pub api_url: String,
    pub api_key: String,
    pub api_type: ApiType,
    pub enabled: bool,
    pub models: Vec<ModelInfo>,
    pub retry_config: RetryConfig,
}

impl Default for LlmProviderInfo {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            api_url: String::new(),
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            enabled: true,
            retry_config: RetryConfig::default(),
            models: vec![
                ModelInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    display_name: "gpt-4o".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Tools,
                    ],
                    enabled: true,
                    limits: ModelLimits::default(),
                },
                ModelInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    display_name: "gpt-4o-mini".to_string(),
                    capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    enabled: true,
                    limits: ModelLimits::default(),
                },
            ],
        }
    }
}
