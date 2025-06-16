use gpui::SharedString;
use gpui_component::IconName;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ApiType {
    #[default] // 默认值为 OpenAI
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ModelCapability {
    #[default] // 默认值为 Text
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderInfo {
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

const CONFIG_FILE: &str = "config/llm_providers.yml";
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmProviderManager {
    #[serde(flatten, default)]
    pub providers: std::collections::HashMap<String, LlmProviderInfo>,
}

impl LlmProviderManager {
    /// 从文件加载配置
    pub fn load() -> Self {
        let content =
            std::fs::read_to_string(CONFIG_FILE).map_or("".to_string(), |content| content);
        // println!("Loading LLM provider config from: {}", content);
        // let manager: LlmProviderManager = serde_yaml::from_str(&content).unwrap();
        let manager: LlmProviderManager =
            serde_yaml::from_str(&content).map_or(LlmProviderManager::default(), |map| map);
        manager
    }

    /// 保存配置到文件
    pub fn save(&self) -> anyhow::Result<()> {
        let content = serde_yaml::to_string(self)?;

        if let Some(parent) = Path::new(CONFIG_FILE).parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(CONFIG_FILE, content)?;
        Ok(())
    }

    /// 获取所有提供商列表
    pub fn list_providers(&self) -> Vec<LlmProviderInfo> {
        self.providers.values().cloned().collect()
    }

    /// 根据ID查询提供商
    pub fn get_provider(&self, id: &str) -> Option<&LlmProviderInfo> {
        self.providers.get(id)
    }

    /// 根据名称查询提供商
    pub fn get_provider_by_name(&self, name: &str) -> Option<&LlmProviderInfo> {
        self.providers.values().find(|p| p.name == name)
    }

    /// 添加新的提供商
    pub fn add_provider(&mut self, provider: LlmProviderInfo) -> anyhow::Result<String> {
        if self.get_provider_by_name(&provider.name).is_some() {
            return Err(anyhow::anyhow!(
                "Provider '{}' already exists",
                provider.name
            ));
        }

        let id = provider.id.clone();
        self.providers.insert(id.clone(), provider);
        self.save()?;
        Ok(id)
    }

    /// 更新提供商
    pub fn update_provider(&mut self, id: &str, provider: LlmProviderInfo) -> anyhow::Result<()> {
        if !self.providers.contains_key(id) {
            return Err(anyhow::anyhow!("Provider with id '{}' not found", id));
        }

        // 检查名称冲突
        if let Some(existing) = self.get_provider_by_name(&provider.name) {
            if existing.id != id {
                return Err(anyhow::anyhow!(
                    "Provider name '{}' already exists",
                    provider.name
                ));
            }
        }

        self.providers.insert(id.to_string(), provider);
        self.save()?;
        Ok(())
    }

    /// 删除提供商
    pub fn delete_provider(&mut self, id: &str) -> anyhow::Result<LlmProviderInfo> {
        let provider = self
            .providers
            .remove(id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;
        self.save()?;
        Ok(provider)
    }

    /// 启用/禁用提供商
    pub fn toggle_provider(&mut self, id: &str, enabled: bool) -> anyhow::Result<()> {
        let provider = self
            .providers
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        provider.enabled = enabled;
        self.save()?;
        Ok(())
    }

    // /// 搜索提供商
    // pub fn search_providers(&self, query: &str) -> Vec<&LlmProvider> {
    //     let query_lower = query.to_lowercase();
    //     self.providers
    //         .values()
    //         .filter(|provider| {
    //             provider.name.to_lowercase().contains(&query_lower)
    //                 || provider.model.to_lowercase().contains(&query_lower)
    //                 || provider.base_url.to_lowercase().contains(&query_lower)
    //         })
    //         .collect()
    // }

    /// 获取提供商数量
    pub fn count(&self) -> usize {
        self.providers.len()
    }

    /// 获取启用的提供商
    pub fn get_enabled_providers(&self) -> Vec<&LlmProviderInfo> {
        self.providers
            .values()
            .filter(|provider| provider.enabled)
            .collect()
    }

    /// 批量删除提供商
    pub fn batch_delete(&mut self, ids: &[String]) -> anyhow::Result<Vec<LlmProviderInfo>> {
        let mut deleted = Vec::new();
        for id in ids {
            if let Some(provider) = self.providers.remove(id) {
                deleted.push(provider);
            }
        }
        self.save()?;
        Ok(deleted)
    }

    /// 清空所有提供商
    pub fn clear(&mut self) -> anyhow::Result<()> {
        self.providers.clear();
        self.save()?;
        Ok(())
    }
}
