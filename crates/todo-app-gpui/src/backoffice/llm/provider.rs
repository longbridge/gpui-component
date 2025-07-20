use super::types::*;
use crate::backoffice::agentic::prompts;
use crate::backoffice::llm::parser;
use crate::config::llm_config::ApiType;
use crate::config::llm_config::LlmProviderConfig;
use crate::config::llm_config::ModelInfo;
use futures::StreamExt;
use rig::providers::openai::Client as OpenAiClient;
use rig::{completion::AssistantContent, message::Message as RigMessage, streaming::StreamingChat};
#[derive(Debug, Clone)]
pub struct LlmChoice {
    pub(crate) config: LlmProviderConfig,
}

impl LlmChoice {
    pub fn new(config: &LlmProviderConfig) -> anyhow::Result<Self> {
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
        tracing::debug!(
            "Loading models for provider {} from {}",
            self.config.id,
            self.config.api_url
        );
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
        prompt: &str,
        history: &[ChatMessage],
    ) -> anyhow::Result<ChatStream> {
        history.iter().enumerate().for_each(|(idx, msg)| {
            tracing::debug!("收到的消息({}): {:?}", idx, msg);
        });
        let tools: Vec<&ToolDefinition> = history
            .iter()
            .flat_map(|msg| msg.get_tool_definitions())
            .collect();
        let no_tools = tools.is_empty();

        let system_prompt = build_system_prompt(history, tools);
        let chat_history = build_history_message(history);
        tracing::debug!("使用系统提示: {}", system_prompt);
        tracing::debug!("使用提示({}): {}", chat_history.len(), prompt);
        chat_history.iter().enumerate().for_each(|(idx, msg)| {
            tracing::debug!("历史消息({}): {:?}", idx, msg);
        });

        let agent = OpenAiClient::from_url(&self.config.api_key, &self.config.api_url)
            .agent(model_id)
            .preamble(system_prompt.as_str())
            .max_tokens(4096)
            .temperature(0.7)
            .build();

        let rig_stream = agent.stream_chat(prompt, chat_history).await?;
        if no_tools {
            let chat_stream = rig_stream.map(|result| match result {
                Ok(AssistantContent::Text(text)) => Ok(MessageContent::TextChunk(text.text)),
                Ok(AssistantContent::ToolCall(tool)) => Ok(MessageContent::ToolFunction(
                    ToolFunction::new(tool.function.name, tool.function.arguments.to_string()),
                )),
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            });
            Ok(Box::pin(chat_stream))
        } else {
            let text_stream = rig_stream.map(|result| match result {
                Ok(AssistantContent::Text(text)) => Ok(text.text),
                Ok(AssistantContent::ToolCall(_)) => Ok(String::new()),
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            });

            let parsed_stream = parser::create_streaming_tool_parser(text_stream);
            Ok(Box::pin(parsed_stream))
        }
    }
}

fn build_history_message(messages: &[ChatMessage]) -> Vec<RigMessage> {
    messages
        .iter()
        .filter_map(|msg| {
            match msg.role {
                MessageRole::User => {
                    // 用户消息：过滤掉包含工具定义的消息
                    if msg.has_tool_definitions() {
                        None
                    } else {
                        Some(RigMessage::user(msg.get_text()))
                    }
                }
                MessageRole::Assistant => {
                    // 助手消息：只包含文本内容
                    let text = msg.get_text();
                    if !text.trim().is_empty() {
                        Some(RigMessage::assistant(text))
                    } else {
                        None
                    }
                }
                MessageRole::System => {
                    // 过滤掉系统消息
                    None
                }
            }
        })
        .collect()
}

fn build_system_prompt(messages: &[ChatMessage], tools: Vec<&ToolDefinition>) -> String {
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
