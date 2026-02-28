use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use llm_connector::LlmClient;
use llm_connector::types::{ChatRequest, Message, Role, StreamingResponse};

use super::types::{ProviderConfig, ProviderType};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>;

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
                if let Some(base_url) = &config.api_base {
                    LlmClient::openai_with_base_url(api_key, base_url)?
                } else {
                    LlmClient::openai(api_key)?
                }
            }
            ProviderType::Anthropic => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Anthropic"))?;
                LlmClient::anthropic(api_key)?
            }
            ProviderType::Aliyun => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Aliyun"))?;
                if let Some(base_url) = &config.api_base {
                    LlmClient::aliyun_private(api_key, base_url)?
                } else {
                    LlmClient::aliyun(api_key)?
                }
            }
            ProviderType::Zhipu => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Zhipu"))?;
                LlmClient::zhipu(api_key)?
            }
            ProviderType::Ollama => {
                if let Some(base_url) = &config.api_base {
                    LlmClient::ollama_with_base_url(base_url)?
                } else {
                    LlmClient::ollama()?
                }
            }
            ProviderType::Volcengine => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Volcengine"))?;
                LlmClient::volcengine(api_key)?
            }
            ProviderType::Moonshot => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Moonshot"))?;
                LlmClient::moonshot(api_key)?
            }
            ProviderType::DeepSeek => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for DeepSeek"))?;
                LlmClient::deepseek(api_key)?
            }
            ProviderType::Google => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("API key required for Google"))?;
                LlmClient::google(api_key)?
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
