use super::types::*;
use crate::backoffice::agentic::prompts;
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
}
impl LlmProvider {
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
        // 1. 从ChatMessage数组的contents字段里提取最后一条用户消息作为prompt
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

        // 2. 构建聊天历史，过滤掉工具定义消息
        let chat_history: Vec<RigMessage> = messages
            .iter()
            .take(last_user_index) // 排除最后一条用户消息
            .filter(|chat_msg| {
                // 过滤掉包含ToolDefinitions的消息
                !chat_msg.has_tool_definitions()
            })
            .map(|chat_msg| match chat_msg.role {
                MessageRole::User => RigMessage::user(chat_msg.get_text()),
                MessageRole::Assistant => RigMessage::assistant(chat_msg.get_text()),
                MessageRole::System => RigMessage::user(chat_msg.get_text()),
            })
            .collect();

        tracing::trace!("使用系统提示: {}", system_prompt);
        let agent = client
            .agent(model_id)
            .context(system_prompt.as_str())
            .max_tokens(4096)
            .temperature(0.7)
            .build();

        let rig_stream = agent.stream_chat(&prompt, chat_history).await?;

        // 根据是否有工具选择不同的处理方式
        if no_tools {
            // 没有工具，使用简单的流转换
            let chat_stream = rig_stream.map(|result| match result {
                Ok(AssistantContent::Text(text)) => Ok(ChatMessage::assistant_chunk(text.text)),
                Ok(AssistantContent::ToolCall(tool)) => Ok(ChatMessage::tool_call(ToolCall {
                    name: tool.function.name,
                    args: tool.function.arguments.to_string(),
                })),
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            });
            Ok(Box::pin(chat_stream))
        } else {
            let chat_stream = create_streaming_tool_parser(&agent, rig_stream);
            Ok(Box::pin(chat_stream))
        }
    }
}

fn build_system_prompt(messages: &[ChatMessage], tools: Vec<ToolDefinition>) -> String {
    let user_system_prompt = messages
        .iter()
        .rev() // 从最新的消息开始查找
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

/// 创建流式工具解析器，边解析边返回
fn create_streaming_tool_parser<M: StreamingCompletionModel + 'static>(
    _: &Agent<M>,
    rig_stream: rig::streaming::StreamingCompletionResponse<M::StreamingResponse>,
) -> impl futures::Stream<Item = anyhow::Result<ChatMessage>> {
    tracing::trace!("开始创建流式工具解析器");
    use futures::stream;

    // 解析状态
    struct ParserState {
        buffer: String,
        xml_buffer: String,
        state: ParseState,
        current_text: String, // 当前正在积累的非工具文本
    }

    #[derive(Clone)]
    enum ParseState {
        Normal,
        TagStart,
        InToolUseTag,
        InToolUse,
        InEndTag,
        InToolUseEndTag,
    }

    let initial_state = ParserState {
        buffer: String::new(),
        xml_buffer: String::new(),
        state: ParseState::Normal,
        current_text: String::new(),
    };

    stream::unfold(
        (rig_stream, initial_state),
        |(mut stream, mut parser_state)| async move {
            const TOOL_USE_START_TAG: &str = "<tool_use";
            const TOOL_USE_END_TAG: &str = "</tool_use";
            const TAG_CLOSE: char = '>';
            const TAG_OPEN: char = '<';

            while let Some(chunk) = stream.next().await {
                tracing::trace!("接收到流数据块 {:?}", chunk);
                match chunk {
                    Ok(AssistantContent::Text(text)) => {
                        for c in text.text.chars() {
                            match parser_state.state {
                                ParseState::Normal => {
                                    if c == TAG_OPEN {
                                        // 检测到可能的工具调用开始
                                        // 如果有积累的文本，立即返回作为文本碎片
                                        if !parser_state.current_text.is_empty() {
                                            let text_to_send = parser_state.current_text.clone();
                                            parser_state.current_text.clear();

                                            // 保存当前状态，准备下次继续解析
                                            parser_state.state = ParseState::TagStart;
                                            parser_state.buffer.clear();
                                            parser_state.buffer.push(c);

                                            let message =
                                                ChatMessage::assistant_chunk(text_to_send);
                                            return Some((Ok(message), (stream, parser_state)));
                                        } else {
                                            parser_state.state = ParseState::TagStart;
                                            parser_state.buffer.clear();
                                            parser_state.buffer.push(c);
                                        }
                                    } else {
                                        // 普通文本字符，添加到当前文本缓冲区
                                        parser_state.current_text.push(c);
                                    }
                                }
                                ParseState::TagStart => {
                                    parser_state.buffer.push(c);
                                    if parser_state.buffer == TOOL_USE_START_TAG {
                                        // 确认是工具调用标签
                                        parser_state.state = ParseState::InToolUseTag;
                                        parser_state.xml_buffer.clear();
                                        parser_state.xml_buffer.push_str(&parser_state.buffer);
                                    } else if parser_state.buffer.len() >= TOOL_USE_START_TAG.len()
                                        || c == TAG_CLOSE
                                    {
                                        if parser_state.buffer != TOOL_USE_START_TAG
                                            && !parser_state
                                                .buffer
                                                .starts_with(&format!("{} ", TOOL_USE_START_TAG))
                                        {
                                            // 不是工具调用标签，恢复为普通文本
                                            parser_state
                                                .current_text
                                                .push_str(&parser_state.buffer);
                                            parser_state.state = ParseState::Normal;
                                        }
                                    }
                                }
                                ParseState::InToolUseTag => {
                                    // 在工具调用标签内部，不输出文本碎片
                                    parser_state.buffer.push(c);
                                    parser_state.xml_buffer.push(c);
                                    if c == TAG_CLOSE {
                                        parser_state.state = ParseState::InToolUse;
                                    }
                                }
                                ParseState::InToolUse => {
                                    // 在工具调用内容内部，不输出文本碎片
                                    parser_state.xml_buffer.push(c);
                                    if c == TAG_OPEN {
                                        parser_state.state = ParseState::InEndTag;
                                        parser_state.buffer.clear();
                                        parser_state.buffer.push(c);
                                    }
                                }
                                ParseState::InEndTag => {
                                    // 在结束标签内部，不输出文本碎片
                                    parser_state.buffer.push(c);
                                    parser_state.xml_buffer.push(c);
                                    if parser_state.buffer == TOOL_USE_END_TAG {
                                        parser_state.state = ParseState::InToolUseEndTag;
                                    } else if parser_state.buffer.len() >= TOOL_USE_END_TAG.len()
                                        || c == TAG_CLOSE
                                    {
                                        if !parser_state.buffer.starts_with(TOOL_USE_END_TAG) {
                                            parser_state.state = ParseState::InToolUse;
                                        }
                                    }
                                }
                                ParseState::InToolUseEndTag => {
                                    parser_state.xml_buffer.push(c);
                                    if c == TAG_CLOSE {
                                        // 完整的工具调用XML已解析完成，立即返回
                                        let xml = parser_state.xml_buffer.clone();
                                        let cleaned = xml
                                            .lines()
                                            .filter(|line| {
                                                !line.contains("DEBUG")
                                                    && !line.trim().starts_with("202")
                                            })
                                            .collect::<Vec<_>>()
                                            .join("\n");

                                        // 重置状态，继续解析后续内容
                                        parser_state.state = ParseState::Normal;
                                        parser_state.xml_buffer.clear();
                                        tracing::trace!("解析到完整的工具调用 XML: {}", cleaned);
                                        if let Ok(tool_call) =
                                            serde_xml_rs::from_str::<ToolCall>(&cleaned)
                                        {
                                            let message = ChatMessage::tool_call(tool_call);

                                            return Some((Ok(message), (stream, parser_state)));
                                        }
                                        // 如果解析失败，继续处理下一个字符
                                    }
                                }
                            }
                        }

                        // 如果当前文本缓冲区有内容且不在工具调用状态中，发送文本碎片
                        if !parser_state.current_text.is_empty()
                            && matches!(parser_state.state, ParseState::Normal)
                        {
                            let text_to_send = parser_state.current_text.clone();
                            parser_state.current_text.clear();
                            let message = ChatMessage::assistant_chunk(text_to_send);
                            return Some((Ok(message), (stream, parser_state)));
                        }
                    }
                    Ok(AssistantContent::ToolCall(tool)) => {
                        // 处理 Rig 原生工具调用
                        let tool_call = ToolCall {
                            name: tool.function.name,
                            args: tool.function.arguments.to_string(),
                        };
                        let message = ChatMessage::tool_call(tool_call);
                        return Some((Ok(message), (stream, parser_state)));
                    }
                    Err(e) => {
                        return Some((
                            Err(anyhow::anyhow!("Stream error: {}", e)),
                            (stream, parser_state),
                        ));
                    }
                }
            }

            // 流结束，如果还有剩余的非工具文本，发送最后的文本碎片
            if !parser_state.current_text.is_empty() {
                let message = ChatMessage::assistant_chunk(parser_state.current_text.clone());
                parser_state.current_text.clear();
                Some((Ok(message), (stream, parser_state)))
            } else {
                None
            }
        },
    )
}
