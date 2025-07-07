use crate::{
    backoffice::agentic::{ChatMessage, ChatStream, MessageRole, ToolDelegate, LLM},
    config::llm_config::LlmProviderConfig,
};
use futures::{stream, StreamExt};
use rig::{completion::AssistantContent, message::Message as RigMessage, streaming::StreamingChat};

/// 现代化的 LLM 实现，直接实现 agentic 的 LLM trait
pub struct RigLlmService {
    config: LlmProviderConfig,
}

impl RigLlmService {
    pub fn new(config: LlmProviderConfig) -> anyhow::Result<Self> {
        // 验证配置
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key is required"));
        }

        if config.default_model.is_none() {
            return Err(anyhow::anyhow!("Default model must be specified"));
        }
        Ok(Self { config })
    }
    /// 内部方法：执行实际的 LLM 调用 - 参考 llm.rs 中的 stream_chat
    async fn execute_completion(&self, messages: &[ChatMessage]) -> anyhow::Result<String> {
        let client =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url);

        // 获取默认模型
        let model_name = self
            .config
            .default_model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No default model configured"))?;

        // 创建 agent - 完全参考 llm.rs 的方式
        let agent = client
            .agent(model_name)
            .max_tokens(4096)
            .temperature(0.7)
            .build();

        // 找到最后一条用户消息的索引
        let last_user_index = messages
            .iter()
            .rposition(|msg| matches!(msg.role, MessageRole::User))
            .unwrap_or(0);

        // 提取最后一条用户消息作为 prompt
        let prompt = if last_user_index < messages.len() {
            messages[last_user_index].get_text()
        } else {
            "执行".to_string()
        };

        // 转换除最后一条用户消息外的所有消息为上下文
        let chat_history: Vec<RigMessage> = messages
            .iter()
            .take(last_user_index) // 只取最后一条用户消息之前的消息
            .map(|chat_msg| match chat_msg.role {
                MessageRole::User => RigMessage::user(chat_msg.get_text()),
                MessageRole::Assistant => RigMessage::assistant(chat_msg.get_text()),
                MessageRole::System => RigMessage::user(chat_msg.get_text()),
                MessageRole::Tool => RigMessage::user(chat_msg.get_text()),
            })
            .collect();

        // 使用流式聊天并收集完整响应 - 参考 llm.rs 的 stream_chat
        let mut stream = agent.stream_chat(&prompt, chat_history).await?;
        let mut response_text = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(content) => {
                    match content {
                        AssistantContent::Text(text) => {
                            response_text.push_str(&text.text);
                        }
                        AssistantContent::ToolCall(_) => {
                            // 暂时忽略工具调用
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Stream error: {}", e));
                }
            }
        }

        Ok(response_text)
    }

    /// 执行流式完成 - 参考 llm.rs 中的流式处理
    async fn execute_completion_stream(
        &self,
        messages: &[ChatMessage],
    ) -> anyhow::Result<ChatStream> {
        let client =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url);

        let model_name = self
            .config
            .default_model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No default model configured"))?;

        let agent = client
            .agent(model_name)
            .max_tokens(4096)
            .temperature(0.7)
            .build();

        // 找到最后一条用户消息的索引
        let last_user_index = messages
            .iter()
            .rposition(|msg| matches!(msg.role, MessageRole::User))
            .unwrap_or(0);

        // 提取最后一条用户消息作为 prompt
        let prompt = if last_user_index < messages.len() {
            messages[last_user_index].get_text()
        } else {
            "执行".to_string()
        };

        // 转换除最后一条用户消息外的所有消息为上下文
        let chat_history: Vec<RigMessage> = messages
            .iter()
            .take(last_user_index) // 只取最后一条用户消息之前的消息
            .map(|chat_msg| match chat_msg.role {
                MessageRole::User => RigMessage::user(chat_msg.get_text()),
                MessageRole::Assistant => RigMessage::assistant(chat_msg.get_text()),
                MessageRole::System => RigMessage::user(chat_msg.get_text()),
                MessageRole::Tool => RigMessage::user(chat_msg.get_text()),
            })
            .collect();

        // 使用流式聊天 - 参考 llm.rs 中的实现
        let mut rig_stream = agent.stream_chat(&prompt, chat_history).await?;

        // 手动收集流并转换为我们的格式
        let mut collected_chunks = Vec::new();

        while let Some(chunk) = rig_stream.next().await {
            match chunk {
                Ok(content) => {
                    match content {
                        AssistantContent::Text(text) => {
                            // 为每个文本块创建一个消息
                            collected_chunks.push(ChatMessage::assistant_text(text.text));
                        }
                        AssistantContent::ToolCall(_) => {
                            // 暂时忽略工具调用
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Stream error: {}", e));
                }
            }
        }

        // 如果没有收集到任何内容，返回空响应
        if collected_chunks.is_empty() {
            collected_chunks.push(ChatMessage::assistant_text(""));
        }

        // 创建简单的流
        let response_stream = stream::iter(collected_chunks.into_iter().map(Ok));
        Ok(Box::pin(response_stream))
    }
}

impl LLM for RigLlmService {
    async fn completion_stream(&self, prompts: &[ChatMessage]) -> anyhow::Result<ChatStream> {
        self.execute_completion_stream(prompts).await
    }

    async fn chat_stream(&self, messages: &[ChatMessage]) -> anyhow::Result<ChatStream> {
        self.execute_completion_stream(messages).await
    }

    async fn completion_with_tools_stream<T: ToolDelegate>(
        &self,
        prompts: &[ChatMessage],
        tools: &T,
    ) -> anyhow::Result<ChatStream> {
        unimplemented!()
    }

    async fn chat_with_tools_stream<T: ToolDelegate>(
        &self,
        messages: &[ChatMessage],
        tools: &T,
    ) -> anyhow::Result<ChatStream> {
        let tool_info = tools.available_tools();

        let mut enhanced_messages = messages.to_vec();
        if !tool_info.is_empty() {
            let tool_description = tool_info
                .iter()
                .map(|t| format!("- {}: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n");

            enhanced_messages.insert(
                0,
                ChatMessage::system_text(format!("你可以使用以下工具：\n{}", tool_description)),
            );
        }

        self.execute_completion_stream(&enhanced_messages).await
    }
}
