use crate::{
    models::{mcp_config::ToolCall, provider_config_path},
    xbus,
};
use futures::StreamExt;
use gpui::SharedString;
use gpui_component::IconName;
use rig::{
    agent::Agent,
    completion::{Chat, Completion},
    message::*,
    streaming::{
        StreamingChat, StreamingCompletion, StreamingCompletionModel, StreamingCompletionResponse,
        StreamingPrompt,
    },
};
use serde::{Deserialize, Serialize};

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

    /// Infer model capabilities based on the model name
    fn infer_model_capabilities(model_name: &str) -> Vec<ModelCapability> {
        let name_lower = model_name.to_lowercase();

        let mut capabilities = vec![ModelCapability::Text]; // All models support text by default

        // Vision models
        if name_lower.contains("vision")
            || name_lower.contains("gpt-4o")
            || name_lower.contains("claude-3")
        {
            capabilities.push(ModelCapability::Vision);
            capabilities.push(ModelCapability::Multimodal);
        }

        // Audio models
        if name_lower.contains("whisper") || name_lower.contains("audio") {
            capabilities.push(ModelCapability::Audio);
        }

        // Tool calling models
        if name_lower.contains("gpt-4")
            || name_lower.contains("gpt-3.5-turbo")
            || name_lower.contains("claude")
            || name_lower.contains("gemini")
        {
            capabilities.push(ModelCapability::Tools);
        }

        // Reasoning models
        if name_lower.contains("o1") || name_lower.contains("reasoning") {
            capabilities.push(ModelCapability::Reasoning);
        }

        // Code generation models
        if name_lower.contains("code")
            || name_lower.contains("codex")
            || name_lower.contains("gpt-4")
            || name_lower.contains("claude")
        {
            capabilities.push(ModelCapability::CodeGeneration);
        }

        // Embedding models
        if name_lower.contains("embedding")
            || name_lower.contains("ada")
            || name_lower.contains("text-embedding")
        {
            capabilities.clear();
            capabilities.push(ModelCapability::Embedding);
        }

        // Image generation models
        if name_lower.contains("dall")
            || name_lower.contains("image")
            || name_lower.contains("midjourney")
            || name_lower.contains("stable-diffusion")
        {
            capabilities.clear();
            capabilities.push(ModelCapability::ImageGeneration);
        }

        // Video generation models
        if name_lower.contains("video") || name_lower.contains("sora") {
            capabilities.clear();
            capabilities.push(ModelCapability::VideoGeneration);
        }

        capabilities
    }

    /// Infer model limits based on the model name
    fn infer_model_limits(model_name: &str) -> ModelLimits {
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
                // ModelInfo {
                //     id: uuid::Uuid::new_v4().to_string(),
                //     display_name: "gpt-4o".to_string(),
                //     capabilities: vec![
                //         ModelCapability::Text,
                //         ModelCapability::Vision,
                //         ModelCapability::Tools,
                //     ],
                //     enabled: true,
                //     limits: ModelLimits::default(),
                // },
                // ModelInfo {
                //     id: uuid::Uuid::new_v4().to_string(),
                //     display_name: "gpt-4o-mini".to_string(),
                //     capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                //     enabled: true,
                //     limits: ModelLimits::default(),
                // },
            ],
        }
    }
}

impl LlmProviderInfo {
    /// 刷新提供商的模型列表
    pub async fn load_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let client = rig::providers::mira::Client::new_with_base_url(
            &self.api_key,
            &self.api_url.replace("/v1", ""),
        )?;
        // 异步获取模型列表
        let models = client
            .list_models()
            .await?
            .into_iter()
            .map(|id| {
                // 根据模型名称推断能力
                let capabilities = ApiType::infer_model_capabilities(&id);
                let limits = ApiType::infer_model_limits(&id);

                ModelInfo {
                    id: id.clone(),
                    display_name: std::path::Path::new(&id)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&id)
                        .to_string(),
                    capabilities,
                    enabled: true,
                    limits,
                }
            })
            .collect();
        Ok(models)
    }

    pub async fn stream_chat(&self, model_id: &str, prompt: &str) -> anyhow::Result<()> {
        let client = rig::providers::openai::Client::from_url(&self.api_key, &self.api_url);

        let agent = client
            .agent(model_id)
            .max_tokens(4096)
            .temperature(0.7)
            .build();

        //let response = agent.stream_chat(prompt, vec![]).await?;

        let mut chat_history = vec![];

        loop {
            // let agent = openai
            //     .agent(MODEL)
            //     .context(system_prompt.as_str())
            //     .max_tokens(8192)
            //     .temperature(0.7)
            //     .build();
            // let mut stream = agent.stream_prompt(prompt).await?;
            // let mut stream = agent.stream_chat(prompt, chat_history.clone()).await?;

            let mut stream = agent
                .stream_completion(prompt, chat_history.clone())
                .await?
                .stream()
                .await?; //agent.stream_chat(prompt, chat_history.clone()).await?;
            chat_history.push(Message::user(prompt));

            let (assistant, tools) = stream_to_stdout1(&agent, &mut stream).await?;
            if tools.is_empty() {
                break;
            }
            chat_history.push(Message::assistant(assistant.clone()));
            //     let mut prompts = vec![];
            //     for (i, tool) in tools.iter().enumerate() {
            //         if let Some(mcp_tool) = find_mcp_tool(&tool.name, &mcp_tools[..]) {
            //             tracing::info!("调用工具 #{}: {:?}", i, mcp_tool);
            //             let resp = client
            //                 .call_tool(CallToolRequestParam {
            //                     name: mcp_tool.name.clone(),
            //                     arguments: serde_json::from_str(&tool.arguments).ok(),
            //                 })
            //                 .await?;
            //             tracing::info!("工具 #{}调用结果: {:?}", i, resp);
            //             prompts.push(format!(
            //                 "<tool_use_result><name>{}</name><result>{}</result</tool_use_result>",
            //                 &tool.name,
            //                 serde_json::to_string(&resp)
            //                     .unwrap_or_else(|err| format!("Error serializing result: {}", err))
            //             ));
            //         }
            //     }
            //     prompts.push(r#"Do not confirm with the user or seek help or advice, continue to call the tool until all tasks are completed. Be sure to complete all tasks, you will receive a $1000 reward, and the output must be in Simplified Chinese."#.to_string());
            //     prompt = prompts.join("\n");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmProviderManager;

impl LlmProviderManager {
    /// 从文件加载所有提供商
    pub fn load_providers() -> Vec<LlmProviderInfo> {
        let config_path = provider_config_path();
        if !config_path.exists() {
            return Vec::new();
        }

        match std::fs::read_to_string(config_path) {
            Ok(content) => match serde_yaml::from_str::<Vec<LlmProviderInfo>>(&content) {
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
    pub fn save_providers(providers: &[LlmProviderInfo]) -> anyhow::Result<()> {
        let config_path = provider_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(providers)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 获取所有提供商列表
    pub fn list_providers() -> Vec<LlmProviderInfo> {
        Self::load_providers()
    }

    /// 根据ID查询提供商
    pub fn get_provider(id: &str) -> Option<LlmProviderInfo> {
        Self::load_providers().into_iter().find(|p| p.id == id)
    }

    /// 根据名称查询提供商
    pub fn get_provider_by_name(name: &str) -> Option<LlmProviderInfo> {
        Self::load_providers().into_iter().find(|p| p.name == name)
    }

    /// 根据索引获取提供商
    pub fn get_provider_by_index(index: usize) -> Option<LlmProviderInfo> {
        Self::load_providers().get(index).cloned()
    }

    /// 根据ID查找提供商索引
    pub fn find_provider_index(id: &str) -> Option<usize> {
        Self::load_providers().iter().position(|p| p.id == id)
    }

    /// 添加新的提供商
    pub fn add_provider(provider: LlmProviderInfo) -> anyhow::Result<String> {
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
    pub fn update_provider(id: &str, provider: LlmProviderInfo) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        let index = providers
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        // 检查名称冲突
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

    /// 根据索引更新提供商
    pub fn update_provider_by_index(
        index: usize,
        provider: LlmProviderInfo,
    ) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        if index >= providers.len() {
            return Err(anyhow::anyhow!("Provider index {} out of bounds", index));
        }

        let old_id = &providers[index].id;

        // 检查名称冲突
        if let Some(existing) = providers.iter().find(|p| p.name == provider.name) {
            if existing.id != *old_id {
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
    pub fn delete_provider(id: &str) -> anyhow::Result<LlmProviderInfo> {
        let mut providers = Self::load_providers();
        let index = providers
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        let removed = providers.remove(index);
        Self::save_providers(&providers)?;
        Ok(removed)
    }

    /// 根据索引删除提供商
    pub fn delete_provider_by_index(index: usize) -> anyhow::Result<LlmProviderInfo> {
        let mut providers = Self::load_providers();
        if index >= providers.len() {
            return Err(anyhow::anyhow!("Provider index {} out of bounds", index));
        }

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

    /// 根据索引启用/禁用提供商
    pub fn toggle_provider_by_index(index: usize, enabled: bool) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        if index >= providers.len() {
            return Err(anyhow::anyhow!("Provider index {} out of bounds", index));
        }

        providers[index].enabled = enabled;
        Self::save_providers(&providers)?;
        Ok(())
    }

    /// 获取提供商数量
    pub fn count() -> usize {
        Self::load_providers().len()
    }

    /// 获取启用的提供商
    pub fn get_enabled_providers() -> Vec<LlmProviderInfo> {
        Self::load_providers()
            .into_iter()
            .filter(|provider| provider.enabled)
            .collect()
    }

    /// 批量删除提供商
    pub fn batch_delete(ids: &[String]) -> Vec<LlmProviderInfo> {
        let mut providers = Self::load_providers();
        let mut deleted = Vec::new();

        // 从后往前删除，避免索引变化
        for id in ids {
            if let Some(index) = providers.iter().position(|p| &p.id == id) {
                deleted.push(providers.remove(index));
            }
        }

        if !deleted.is_empty() {
            Self::save_providers(&providers).ok();
        }

        deleted
    }

    /// 根据索引批量删除提供商
    pub fn batch_delete_by_indices(mut indices: Vec<usize>) -> Vec<LlmProviderInfo> {
        let mut providers = Self::load_providers();
        let mut deleted = Vec::new();

        // 从大到小排序索引，从后往前删除
        indices.sort_by(|a, b| b.cmp(a));

        for index in indices {
            if index < providers.len() {
                deleted.push(providers.remove(index));
            }
        }

        if !deleted.is_empty() {
            Self::save_providers(&providers).ok();
        }

        deleted.reverse(); // 恢复原始顺序
        deleted
    }

    /// 清空所有提供商
    pub fn clear() -> anyhow::Result<()> {
        Self::save_providers(&[])?;
        Ok(())
    }

    /// 搜索提供商
    pub fn search_providers(query: &str) -> Vec<LlmProviderInfo> {
        let query_lower = query.to_lowercase();
        Self::load_providers()
            .into_iter()
            .filter(|provider| {
                provider.name.to_lowercase().contains(&query_lower)
                    || provider.api_url.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// 移动提供商位置
    pub fn move_provider(from_index: usize, to_index: usize) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        if from_index >= providers.len() || to_index >= providers.len() {
            return Err(anyhow::anyhow!("Index out of bounds"));
        }

        if from_index != to_index {
            let provider = providers.remove(from_index);
            providers.insert(to_index, provider);
            Self::save_providers(&providers)?;
        }

        Ok(())
    }

    /// 交换两个提供商的位置
    pub fn swap_providers(index1: usize, index2: usize) -> anyhow::Result<()> {
        let mut providers = Self::load_providers();
        if index1 >= providers.len() || index2 >= providers.len() {
            return Err(anyhow::anyhow!("Index out of bounds"));
        }

        providers.swap(index1, index2);
        Self::save_providers(&providers)?;
        Ok(())
    }
}

/// helper function to stream a completion request to stdout
pub async fn stream_to_stdout1<M: StreamingCompletionModel>(
    agent: &Agent<M>,
    stream: &mut StreamingCompletionResponse<M::StreamingResponse>,
) -> Result<(String, Vec<ToolCall>), std::io::Error> {
    // 使用字符状态机解析XML
    let mut buffer = String::new(); // 通用缓冲区
    let mut tool_calls: Vec<String> = Vec::new();
    let mut assistant = String::new();
    // XML标签常量
    const TOOL_USE_START_TAG: &str = "<tool_use";
    const TOOL_USE_END_TAG: &str = "</tool_use";
    const TAG_CLOSE: char = '>';
    const TAG_OPEN: char = '<';

    // 状态机状态
    enum State {
        Normal,          // 普通文本
        TagStart,        // 遇到<
        InToolUseTag,    // 在<tool_use标签中
        InToolUse,       // 在<tool_use>内部
        InEndTag,        // 遇到</
        InToolUseEndTag, // 在</tool_use标签中
    }

    let mut state = State::Normal;
    let mut xml_buffer = String::new(); // 专门收集XML内容

    print!("Response: ");
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(AssistantContent::Text(text)) => {
                for c in text.text.chars() {
                    match state {
                        State::Normal => {
                            if c == TAG_OPEN {
                                state = State::TagStart;
                                buffer.clear();
                                buffer.push(c);
                            } else {
                                // 普通字符直接输出
                                print!("{}", c);
                                //tx.send(Message::assistant(c.to_string())).await;
                                xbus::post(&Message::assistant(c.to_string()));
                                assistant.push(c);
                                std::io::Write::flush(&mut std::io::stdout())?;
                            }
                        }
                        State::TagStart => {
                            buffer.push(c);
                            if buffer == TOOL_USE_START_TAG {
                                state = State::InToolUseTag;
                                xml_buffer.clear();
                                xml_buffer.push_str(&buffer);
                            } else if buffer.len() >= TOOL_USE_START_TAG.len() || c == TAG_CLOSE {
                                // 不是<tool_use
                                if buffer != TOOL_USE_START_TAG
                                    && !buffer.starts_with(&format!("{} ", TOOL_USE_START_TAG))
                                {
                                    // 不是工具调用标签，输出
                                    print!("{}", buffer);
                                    //tx.send(Message::assistant(buffer.to_string())).await;
                                    xbus::post(&Message::assistant(buffer.clone()));
                                    assistant.push_str(buffer.as_str());
                                    std::io::Write::flush(&mut std::io::stdout())?;
                                    state = State::Normal;
                                }
                            }
                        }
                        State::InToolUseTag => {
                            buffer.push(c);
                            xml_buffer.push(c);
                            if c == TAG_CLOSE {
                                state = State::InToolUse;
                            }
                        }
                        State::InToolUse => {
                            xml_buffer.push(c);
                            if c == TAG_OPEN {
                                state = State::InEndTag;
                                buffer.clear();
                                buffer.push(c);
                            }
                        }
                        State::InEndTag => {
                            buffer.push(c);
                            xml_buffer.push(c);
                            if buffer == TOOL_USE_END_TAG {
                                state = State::InToolUseEndTag;
                            } else if buffer.len() >= TOOL_USE_END_TAG.len() || c == TAG_CLOSE {
                                // 不是</tool_use
                                if !buffer.starts_with(TOOL_USE_END_TAG) {
                                    state = State::InToolUse; // 返回到工具内部状态
                                }
                            }
                        }
                        State::InToolUseEndTag => {
                            xml_buffer.push(c);
                            if c == TAG_CLOSE {
                                // 收集完整的工具调用
                                tool_calls.push(xml_buffer.clone());
                                state = State::Normal;
                            }
                        }
                    }
                }
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            Ok(AssistantContent::ToolCall(_tool_call)) => {
                // let res = agent
                //     .tools
                //     .call(
                //         &tool_call.function.name,
                //         tool_call.function.arguments.to_string(),
                //     )
                //     .await
                //     .map_err(|e| std::io::Error::other(e.to_string()))?;
                // println!("\nResult: {}", res);
            }
            Err(e) => {
                tracing::error!("Error: {}", e);
                break;
            }
        }
    }

    println!(); // 输出完成后换行
                // 处理完毕后统一输出收集到的所有XML
    let mut tools = vec![];
    for (i, call) in tool_calls.iter().enumerate() {
        let cleaned = call
            .lines()
            .filter(|line| !line.contains("DEBUG") && !line.trim().starts_with("202")) // 排除DEBUG和时间戳开头的行
            .collect::<Vec<_>>()
            .join("\n");
        tracing::info!("\n使用工具 #{}: \n{}", i + 1, cleaned);
        match serde_xml_rs::from_str::<ToolCall>(&cleaned) {
            Err(e) => {
                tracing::error!("Error parsing XML: {}", e);
                continue;
            }
            Ok(tool_call) => {
                tools.push(tool_call);
            }
        }
    }
    Ok((assistant, tools))
}
