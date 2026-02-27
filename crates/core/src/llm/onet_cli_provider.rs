//! Onet CLI LLM Provider
//!
//! 使用 Supabase 边缘函数作为 AI 代理的 LLM Provider。
//! 委托给 CloudApiClient 实现，支持 OpenAI 兼容的 /chat/completions 接口。

use anyhow::Result;
use async_trait::async_trait;
use llm_connector::types::ChatRequest;
use std::sync::Arc;

use super::connector::{ChatStream, LlmProvider};
use crate::cloud_sync::client::CloudApiClient;

/// Onet CLI LLM Provider
///
/// 使用 Supabase 边缘函数作为 AI 代理，委托给 CloudApiClient 实现。
pub struct OnetCliLLMProvider {
    cloud_client: Arc<dyn CloudApiClient>,
}

impl OnetCliLLMProvider {
    /// 创建新的 Onet CLI LLM Provider
    pub fn new(cloud_client: Arc<dyn CloudApiClient>) -> Self {
        Self { cloud_client }
    }
}

#[async_trait]
impl LlmProvider for OnetCliLLMProvider {
    async fn chat(&self, request: &ChatRequest) -> Result<String> {
        self.cloud_client
            .chat(request)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn chat_stream(&self, request: &ChatRequest) -> Result<ChatStream> {
        self.cloud_client
            .chat_stream(request)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn models(&self) -> Result<Vec<String>> {
        self.cloud_client
            .list_models()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn provider_name(&self) -> &str {
        "onet_cli"
    }
}
