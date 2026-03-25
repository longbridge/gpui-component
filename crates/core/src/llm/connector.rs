use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use llm_connector::LlmClient;
use llm_connector::types::{ChatRequest, Message, Role, StreamingResponse};

use super::types::{ProviderConfig, ProviderType};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>;

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const ALIYUN_BASE_URL: &str = "https://dashscope.aliyuncs.com";
const ALIYUN_COMPATIBLE_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const ZHIPU_BASE_URL: &str = "https://open.bigmodel.cn";
const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const VOLCENGINE_BASE_URL: &str = "https://ark.cn-beijing.volces.com/api/v3";
const MOONSHOT_BASE_URL: &str = "https://api.moonshot.cn/v1";
const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
const GOOGLE_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

pub use llm_connector::types::{
    ChatRequest as LlmChatRequest, Message as LlmMessage, Role as LlmRole,
};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: &ChatRequest) -> Result<String>;
    async fn chat_stream(&self, request: &ChatRequest) -> Result<ChatStream>;

    async fn models(&self) -> Result<Vec<String>>;
    fn provider_name(&self) -> &str;
}

pub struct LlmConnector {
    client: LlmClient,
    provider_type: ProviderType,
}

impl LlmConnector {
    pub fn from_config(config: &ProviderConfig) -> Result<Self> {
        let client = match config.provider_type {
            ProviderType::OpenAI => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for OpenAI"))?;
                LlmClient::openai(api_key, provider_base_url(config, OPENAI_BASE_URL))?
            }
            ProviderType::Anthropic => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Anthropic"))?;
                LlmClient::anthropic(api_key, provider_base_url(config, ANTHROPIC_BASE_URL))?
            }
            ProviderType::Aliyun => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Aliyun"))?;
                if aliyun_prefers_compatible_mode(config) {
                    let base_url = aliyun_base_url(config);
                    LlmClient::openai_compatible(api_key, base_url, &config.name)?
                } else if let Some(base_url) = &config.api_base {
                    LlmClient::aliyun_private(api_key, base_url)?
                } else {
                    LlmClient::aliyun(api_key, ALIYUN_BASE_URL)?
                }
            }
            ProviderType::Zhipu => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Zhipu"))?;
                LlmClient::zhipu(api_key, provider_base_url(config, ZHIPU_BASE_URL))?
            }
            ProviderType::Ollama => LlmClient::ollama(provider_base_url(config, OLLAMA_BASE_URL))?,
            ProviderType::Volcengine => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Volcengine"))?;
                LlmClient::volcengine(api_key, provider_base_url(config, VOLCENGINE_BASE_URL))?
            }
            ProviderType::Moonshot => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Moonshot"))?;
                LlmClient::moonshot(api_key, provider_base_url(config, MOONSHOT_BASE_URL))?
            }
            ProviderType::DeepSeek => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for DeepSeek"))?;
                LlmClient::deepseek(api_key, provider_base_url(config, DEEPSEEK_BASE_URL))?
            }
            ProviderType::Google => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Google"))?;
                LlmClient::google(api_key, provider_base_url(config, GOOGLE_BASE_URL))?
            }
            ProviderType::AzureOpenAI => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Azure OpenAI"))?;
                let base_url = config
                    .api_base
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Base URL required for Azure OpenAI"))?;
                let api_version = config
                    .api_version
                    .as_deref()
                    .unwrap_or("2024-02-15-preview");
                LlmClient::azure_openai(api_key, base_url, api_version)?
            }
            ProviderType::OpenAICompatible => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for OpenAI Compatible"))?;
                let base_url = config
                    .api_base
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Base URL required for OpenAI Compatible"))?;
                LlmClient::openai_compatible(api_key, base_url, &config.name)?
            }
            ProviderType::OnetCli => {
                // OnetCli 使用专门的 OnetCliLLMProvider，不通过 LlmConnector 创建
                anyhow::bail!(
                    "OnetCli provider should be created via ProviderManager, not LlmConnector"
                )
            }
        };

        Ok(Self {
            client,
            provider_type: config.provider_type,
        })
    }

    pub fn build_request(&self, config: &ProviderConfig, messages: Vec<Message>) -> ChatRequest {
        let mut request = ChatRequest {
            model: config.model.clone(),
            messages,
            ..Default::default()
        };

        if let Some(max_tokens) = config.max_tokens {
            request.max_tokens = Some(max_tokens as u32);
        }

        if let Some(temperature) = config.temperature {
            request.temperature = Some(temperature);
        }

        request
    }
}

fn provider_base_url<'a>(config: &'a ProviderConfig, default_base_url: &'static str) -> &'a str {
    config.api_base.as_deref().unwrap_or(default_base_url)
}

fn aliyun_base_url(config: &ProviderConfig) -> &str {
    config
        .api_base
        .as_deref()
        .unwrap_or(ALIYUN_COMPATIBLE_BASE_URL)
}

fn aliyun_prefers_compatible_mode(config: &ProviderConfig) -> bool {
    config.model.starts_with("qwen3.5-")
        || config
            .api_base
            .as_deref()
            .map(|base_url| base_url.contains("/compatible-mode/"))
            .unwrap_or(false)
}

#[async_trait]
impl LlmProvider for LlmConnector {
    async fn chat(&self, request: &ChatRequest) -> Result<String> {
        let response = self.client.chat(request).await?;
        Ok(response.content)
    }

    async fn chat_stream(&self, request: &ChatRequest) -> Result<ChatStream> {
        let stream = self.client.chat_stream(request).await?;
        Ok(Box::pin(futures::stream::StreamExt::map(
            stream,
            |result| result.map_err(|e| anyhow::anyhow!("{}", e)),
        )))
    }

    async fn models(&self) -> Result<Vec<String>> {
        let models = self.client.models().await?;
        Ok(models)
    }

    fn provider_name(&self) -> &str {
        self.provider_type.as_str()
    }
}

pub fn create_message(role: Role, content: impl Into<String>) -> Message {
    Message::text(role, content)
}

pub fn user_message(content: impl Into<String>) -> Message {
    create_message(Role::User, content)
}

pub fn assistant_message(content: impl Into<String>) -> Message {
    create_message(Role::Assistant, content)
}

pub fn system_message(content: impl Into<String>) -> Message {
    create_message(Role::System, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_base_url_prefers_configured_value() {
        let config = ProviderConfig {
            api_base: Some("https://custom.example.com".to_string()),
            ..Default::default()
        };

        assert_eq!(
            provider_base_url(&config, OPENAI_BASE_URL),
            "https://custom.example.com"
        );
    }

    #[test]
    fn provider_base_url_uses_default_when_config_missing() {
        let config = ProviderConfig::default();

        assert_eq!(provider_base_url(&config, OLLAMA_BASE_URL), OLLAMA_BASE_URL);
    }

    #[test]
    fn aliyun_prefers_compatible_mode_for_qwen35_models() {
        let config = ProviderConfig {
            provider_type: ProviderType::Aliyun,
            model: "qwen3.5-plus".to_string(),
            ..Default::default()
        };

        assert!(aliyun_prefers_compatible_mode(&config));
        assert_eq!(aliyun_base_url(&config), ALIYUN_COMPATIBLE_BASE_URL);
    }

    #[test]
    fn aliyun_prefers_compatible_mode_for_explicit_compatible_base_url() {
        let config = ProviderConfig {
            provider_type: ProviderType::Aliyun,
            api_base: Some("https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()),
            model: "qwen-plus".to_string(),
            ..Default::default()
        };

        assert!(aliyun_prefers_compatible_mode(&config));
        assert_eq!(
            aliyun_base_url(&config),
            "https://dashscope.aliyuncs.com/compatible-mode/v1"
        );
    }

    #[test]
    fn aliyun_keeps_private_protocol_for_non_compatible_models() {
        let config = ProviderConfig {
            provider_type: ProviderType::Aliyun,
            api_base: Some("https://dashscope.aliyuncs.com".to_string()),
            model: "qwen-plus".to_string(),
            ..Default::default()
        };

        assert!(!aliyun_prefers_compatible_mode(&config));
    }
}
