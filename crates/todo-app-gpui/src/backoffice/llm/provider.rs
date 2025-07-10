use super::parser::StreamingToolParser;
use super::types::*;
use crate::backoffice::agentic::prompts;
use crate::backoffice::llm::parser;
use crate::config::llm_config::ApiType;
use crate::config::llm_config::LlmProviderConfig;
use crate::config::llm_config::ModelInfo;
use futures::StreamExt;
use rig::agent::Agent;
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
        let client =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url);

        let tools: Vec<ToolDefinition> = messages
            .iter()
            .flat_map(|msg| msg.get_tool_definitions())
            .cloned()
            .collect();
        let no_tools = tools.is_empty();
        let system_prompt = build_system_prompt(messages, tools);
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
            .take(last_user_index)
            .filter(|chat_msg| {
                !chat_msg.has_tool_definitions() && chat_msg.role != MessageRole::System
            })
            .map(|chat_msg| match chat_msg.role {
                MessageRole::User => RigMessage::user(chat_msg.get_text()),
                MessageRole::Assistant => RigMessage::assistant(chat_msg.get_text()),
                MessageRole::System => RigMessage::user(chat_msg.get_text()),
                MessageRole::Tool => RigMessage::user(chat_msg.get_text()),
            })
            .collect();
        tracing::debug!("使用模型: {}", model_id);
        tracing::debug!("使用系统提示: {}", system_prompt);
        tracing::debug!("使用提示: {}", prompt);
        chat_history.iter().enumerate().for_each(|(idx, msg)| {
            tracing::debug!("聊天历史消息({}): {:?}", idx, msg);
        });
        let agent = client
            .agent(model_id)
            .context(system_prompt.as_str())
            .max_tokens(4096)
            .temperature(0.7)
            .build();

        let rig_stream = agent.stream_chat(&prompt, chat_history).await?;

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
            // 有工具，使用专门的解析器
            let text_stream = rig_stream.map(|result| match result {
                Ok(AssistantContent::Text(text)) => Ok(text.text),
                Ok(AssistantContent::ToolCall(_)) => Ok(String::new()), // 忽略原生工具调用
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            });

            // **修改点：调用 parser 模块中的新函数**
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

// /// 简化的流式工具解析器
// fn create_streaming_tool_parser_simple<S>(
//     input_stream: S,
// ) -> impl futures::Stream<Item = anyhow::Result<ChatMessage>>
// where
//     S: futures::Stream<Item = anyhow::Result<String>> + Send + Unpin + 'static,
// {
//     use futures::stream;
//     use futures::StreamExt;

//     stream::unfold(
//         (input_stream, StreamingToolParser::new(), Vec::<ChatMessage>::new()),
//         |(mut stream, mut parser, mut message_queue)| async move {
//             // 如果队列中有消息，先发送队列中的消息
//             if !message_queue.is_empty() {
//                 let message = message_queue.remove(0);
//                 return Some((Ok(message), (stream, parser, message_queue)));
//             }

//             match stream.next().await {
//                 Some(Ok(text)) => {
//                     tracing::debug!("处理文本chunk: {}", text);
//                     let mut messages = parser.process_chunk(&text);

//                     if !messages.is_empty() {
//                         // 取出第一个消息发送，其余放入队列
//                         let first_message = messages.remove(0);
//                         message_queue.extend(messages);
//                         Some((Ok(first_message), (stream, parser, message_queue)))
//                     } else {
//                         // 没有消息，返回空的chunk以保持流活跃
//                         Some((Ok(ChatMessage::assistant_chunk("")), (stream, parser, message_queue)))
//                     }
//                 }
//                 Some(Err(e)) => Some((Err(e), (stream, parser, message_queue))),
//                 None => {
//                     // 流结束，处理剩余内容
//                     let final_messages = parser.finish();
//                     if !final_messages.is_empty() {
//                         // 取出第一个消息发送，其余放入队列
//                         let mut final_iter = final_messages.into_iter();
//                         let first_message = final_iter.next().unwrap();
//                         message_queue.extend(final_iter);
//                         Some((Ok(first_message), (stream, parser, message_queue)))
//                     } else {
//                         None
//                     }
//                 }
//             }
//         },
//     )
// }
