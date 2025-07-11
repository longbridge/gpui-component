use super::parser::StreamingToolParser;
use super::types::*;
use crate::backoffice::agentic::prompts;
use crate::backoffice::llm::parser;
use crate::config::llm_config::ApiType;
use crate::config::llm_config::LlmProviderConfig;
use crate::config::llm_config::ModelInfo;
use futures::StreamExt;
use rig::agent::Agent;
use rig::streaming::StreamingCompletion;
use rig::{
    completion::AssistantContent,
    message::Message as RigMessage,
    streaming::{StreamingChat, StreamingCompletionModel},
};

#[derive(Debug, Clone)]
pub struct LlmProvider {
    pub(crate) config: LlmProviderConfig,
}

impl LlmProvider {
    pub fn new(config: &LlmProviderConfig) -> anyhow::Result<Self> {
        // 验证配置
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key is required"));
        }
        Ok(Self {
            config: config.clone(),
        })
    }

    pub async fn load_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let client = rig::providers::mira::Client::new_with_base_url(
            &self.config.api_key,
            &self.config.api_url.replace("/v1", ""),
        )?;

        let mut models = client
            .list_models()
            .await?
            .into_iter()
            .map(|id| {
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
            .collect::<Vec<ModelInfo>>();

        models.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(models)
    }

    pub async fn stream_chat(
        &self,
        model_id: &str,
        messages: &[ChatMessage],
    ) -> anyhow::Result<ChatStream> {
        messages.iter().enumerate().for_each(|(idx, msg)| {
            tracing::debug!("收到的消息({}): {:?}", idx, msg);
        });
        let tools: Vec<ToolDefinition> = messages
            .iter()
            .flat_map(|msg| msg.get_tool_definitions())
            .cloned()
            .collect();
        let no_tools = tools.is_empty();

        let system_prompt = build_system_prompt(messages, tools);

        //取最后一条用户消息
        let prompt = messages
            .iter()
            .rev()
            .find(|msg| matches!(msg.role, MessageRole::User))
            .map(|msg| msg.get_text())
            .unwrap_or_else(|| "执行".to_string());
        let last_user_index = messages
            .iter()
            .rposition(|msg| matches!(msg.role, MessageRole::User))
            .unwrap_or(0);

        let chat_history: Vec<RigMessage> = messages
            .iter()
            // .take(last_user_index)
            .filter(|chat_msg| {
                chat_msg.role == MessageRole::User
                    || chat_msg.role == MessageRole::Assistant
                    || chat_msg.role == MessageRole::Tool
            })
            .map(|chat_msg| match chat_msg.role {
                MessageRole::User => RigMessage::user(chat_msg.get_text()),
                MessageRole::Assistant => RigMessage::assistant(chat_msg.get_text()),
                MessageRole::System => RigMessage::user(chat_msg.get_text()),
                MessageRole::Tool => RigMessage::user(chat_msg.get_text()),
            })
            .collect();
        tracing::debug!("使用系统提示: {}", system_prompt);
        tracing::debug!("使用提示({}): {}", prompt, chat_history.len());
        chat_history.iter().enumerate().for_each(|(idx, msg)| {
            tracing::debug!("聊天历史消息({}): {:?}", idx, msg);
        });
        let agent =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url)
                .agent(model_id)
                .context(system_prompt.as_str())
                .max_tokens(4096)
                .temperature(0.7)
                .build();

        let rig_stream = agent.stream_chat("", chat_history).await?;

        if no_tools {
            // 没有工具，简单转换
            let chat_stream = rig_stream.map(|result| match result {
                Ok(AssistantContent::Text(text)) => Ok(ChatMessage::assistant_chunk(text.text)),
                Ok(AssistantContent::ToolCall(tool)) => Ok(ChatMessage::tool_call(ToolCall {
                    name: tool.function.name,
                    arguments: tool.function.arguments.to_string(),
                })),
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            });
            Ok(Box::pin(chat_stream))
        } else {
            let text_stream = rig_stream.map(|result| match result {
                Ok(AssistantContent::Text(text)) => Ok(text.text),
                Ok(AssistantContent::ToolCall(_)) => Ok(String::new()), // 忽略原生工具调用
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            });

            let parsed_stream = parser::create_streaming_tool_parser(text_stream);
            Ok(Box::pin(parsed_stream))
        }
    }
}

fn build_system_prompt(messages: &[ChatMessage], tools: Vec<ToolDefinition>) -> String {
    let user_system_prompt = messages
        .iter()
        .rev()
        .find(|msg| matches!(msg.role, MessageRole::System) && !msg.has_tool_definitions());
    match (user_system_prompt, tools.is_empty()) {
        (Some(user_system_prompt), false) => {
            prompts::with_tools_user_system_prompt(tools, user_system_prompt.get_text())
        }
        (Some(custom_prompt), true) => {
            prompts::default_with_user_system_prompt(custom_prompt.get_text())
        }
        (None, false) => prompts::with_tools(tools),
        (None, true) => prompts::default_prompt(),
    }
}
