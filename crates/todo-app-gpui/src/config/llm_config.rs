use crate::config::provider_config_path;
use gpui::SharedString;
use gpui_component::IconName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ApiType {
    #[default]
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

    /// Infer model capabilities based on the model name
    pub fn infer_model_capabilities(model_name: &str) -> Vec<ModelCapability> {
        let name_lower = model_name.to_lowercase();

        let mut capabilities = vec![ModelCapability::Text];

        if name_lower.contains("vision")
            || name_lower.contains("gpt-4o")
            || name_lower.contains("claude-3")
        {
            capabilities.push(ModelCapability::Vision);
            capabilities.push(ModelCapability::Multimodal);
        }

        if name_lower.contains("whisper") || name_lower.contains("audio") {
            capabilities.push(ModelCapability::Audio);
        }

        if name_lower.contains("gpt-4")
            || name_lower.contains("gpt-3.5-turbo")
            || name_lower.contains("claude")
            || name_lower.contains("gemini")
        {
            capabilities.push(ModelCapability::Tools);
        }

        if name_lower.contains("o1") || name_lower.contains("reasoning") {
            capabilities.push(ModelCapability::Reasoning);
        }

        if name_lower.contains("code")
            || name_lower.contains("codex")
            || name_lower.contains("gpt-4")
            || name_lower.contains("claude")
        {
            capabilities.push(ModelCapability::CodeGeneration);
        }

        if name_lower.contains("embedding")
            || name_lower.contains("ada")
            || name_lower.contains("text-embedding")
        {
            capabilities.clear();
            capabilities.push(ModelCapability::Embedding);
        }

        if name_lower.contains("dall")
            || name_lower.contains("image")
            || name_lower.contains("midjourney")
            || name_lower.contains("stable-diffusion")
        {
            capabilities.clear();
            capabilities.push(ModelCapability::ImageGeneration);
        }

        if name_lower.contains("video") || name_lower.contains("sora") {
            capabilities.clear();
            capabilities.push(ModelCapability::VideoGeneration);
        }

        capabilities
    }

    /// Infer model limits based on the model name
    pub fn infer_model_limits(model_name: &str) -> ModelLimits {
        let name_lower = model_name.to_lowercase();

        if name_lower.contains("gpt-4o") {
            ModelLimits {
                context_length: Some(128000),
                max_output_tokens: Some(4096),
                max_requests_per_minute: Some(500),
                max_requests_per_day: None,
                max_tokens_per_minute: Some(30000),
            }
        } else if name_lower.contains("gpt-4") {
            ModelLimits {
                context_length: Some(8192),
                max_output_tokens: Some(4096),
                max_requests_per_minute: Some(200),
                max_requests_per_day: None,
                max_tokens_per_minute: Some(10000),
            }
        } else if name_lower.contains("gpt-3.5") {
            ModelLimits {
                context_length: Some(16385),
                max_output_tokens: Some(4096),
                max_requests_per_minute: Some(3500),
                max_requests_per_day: None,
                max_tokens_per_minute: Some(90000),
            }
        } else if name_lower.contains("claude-3") {
            ModelLimits {
                context_length: Some(200000),
                max_output_tokens: Some(4096),
                max_requests_per_minute: Some(50),
                max_requests_per_day: None,
                max_tokens_per_minute: Some(40000),
            }
        } else if name_lower.contains("gemini") {
            ModelLimits {
                context_length: Some(32768),
                max_output_tokens: Some(8192),
                max_requests_per_minute: Some(60),
                max_requests_per_day: None,
                max_tokens_per_minute: Some(32000),
            }
        } else {
            ModelLimits::default()
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ModelCapability {
    #[default]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub capabilities: Vec<ModelCapability>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub limits: ModelLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: u64,
    pub max_delay: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_type: ApiType,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub models: Vec<ModelInfo>,
    #[serde(default)]
    pub retry_config: RetryConfig,
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            api_url: String::new(),
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            enabled: true,
            retry_config: RetryConfig::default(),
            models: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmProviderManager;

impl LlmProviderManager {
    /// 从文件加载所有提供商
    fn load_providers() -> Vec<LlmProviderConfig> {
        let config_path = provider_config_path();
        if !config_path.exists() {
            return Vec::new();
        }

        match std::fs::read_to_string(config_path) {
            Ok(content) => match serde_yaml::from_str::<Vec<LlmProviderConfig>>(&content) {
                Ok(providers) => providers,
                Err(e) => {
                    eprintln!("Failed to parse LLM provider config: {}", e);
                    Vec::new()
                }
            },
            Err(e) => {
                eprintln!("Failed to read LLM provider config file: {}", e);
                Vec::new()
            }
        }
    }

    /// 保存所有提供商到文件
    pub fn save_providers(providers: &[LlmProviderConfig]) -> anyhow::Result<()> {
        let config_path = provider_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(providers)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 获取所有提供商列表
    pub fn list_providers() -> Vec<LlmProviderConfig> {
        Self::load_providers()
    }

    /// 根据ID查询提供商
    pub fn get_provider(id: &str) -> Option<LlmProviderConfig> {
        Self::load_providers().into_iter().find(|p| p.id == id)
    }

    /// 根据索引获取提供商
    pub fn get_provider_by_index(index: usize) -> Option<LlmProviderConfig> {
        Self::load_providers().get(index).cloned()
    }

    /// 根据ID查找提供商索引
    pub fn find_provider_index(id: &str) -> Option<usize> {
        Self::load_providers().iter().position(|p| p.id == id)
    }

    /// 添加新的提供商
    pub fn add_provider(provider: LlmProviderConfig) -> anyhow::Result<String> {
        let mut providers = Self::load_providers();

        if providers.iter().any(|p| p.name == provider.name) {
            return Err(anyhow::anyhow!(
                "Provider '{}' already exists",
                provider.name
            ));
        }

        let id = provider.id.clone();
        providers.push(provider);
        Self::save_providers(&providers)?;
        Ok(id)
    }

    /// 更新提供商
    pub fn update_provider(id: &str, provider: LlmProviderConfig) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        let index = providers
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        if let Some(existing) = providers.iter().find(|p| p.name == provider.name) {
            if existing.id != id {
                return Err(anyhow::anyhow!(
                    "Provider name '{}' already exists",
                    provider.name
                ));
            }
        }

        providers[index] = provider;
        Self::save_providers(&providers)?;
        Ok(())
    }

    /// 删除提供商
    pub fn delete_provider(id: &str) -> anyhow::Result<LlmProviderConfig> {
        let mut providers = Self::load_providers();
        let index = providers
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        let removed = providers.remove(index);
        Self::save_providers(&providers)?;
        Ok(removed)
    }

    /// 启用/禁用提供商
    pub fn toggle_provider(id: &str, enabled: bool) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        let provider = providers
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        provider.enabled = enabled;
        Self::save_providers(&providers)?;
        Ok(())
    }

    /// 获取启用的提供商
    pub fn get_enabled_providers() -> Vec<LlmProviderConfig> {
        Self::load_providers()
            .into_iter()
            .filter(|provider| provider.enabled)
            .collect()
    }

    /// 同步提供商的模型列表
    pub fn sync_provider_models(provider_id: &str, models: Vec<ModelInfo>) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        let provider = providers
            .iter_mut()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", provider_id))?;

        // 合并现有模型配置和新获取的模型
        let mut updated_models = Vec::new();

        for new_model in models {
            if let Some(existing_model) = provider.models.iter().find(|m| m.id == new_model.id) {
                // 保留现有配置，但更新其他信息
                updated_models.push(ModelInfo {
                    id: new_model.id,
                    display_name: new_model.display_name,
                    capabilities: new_model.capabilities,
                    enabled: existing_model.enabled, // 保留用户设置
                    limits: new_model.limits,
                });
            } else {
                // 新模型，默认启用
                updated_models.push(new_model);
            }
        }

        provider.models = updated_models;
        Self::save_providers(&providers)?;
        Ok(())
    }

    /// 更新模型的启用状态
    pub fn toggle_model(provider_id: &str, model_id: &str, enabled: bool) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        let provider = providers
            .iter_mut()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", provider_id))?;

        let model = provider
            .models
            .iter_mut()
            .find(|m| m.id == model_id)
            .ok_or_else(|| anyhow::anyhow!("Model with id '{}' not found", model_id))?;

        model.enabled = enabled;
        Self::save_providers(&providers)?;
        Ok(())
    }

    /// 获取提供商的启用模型列表
    pub fn get_enabled_models(provider_id: &str) -> Vec<ModelInfo> {
        Self::get_provider(provider_id)
            .map(|provider| {
                provider
                    .models
                    .into_iter()
                    .filter(|model| model.enabled)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取提供商的所有模型列表
    pub fn get_all_models(provider_id: &str) -> Vec<ModelInfo> {
        Self::get_provider(provider_id)
            .map(|provider| provider.models)
            .unwrap_or_default()
    }
}
