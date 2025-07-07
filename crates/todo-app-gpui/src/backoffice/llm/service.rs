use crate::backoffice::agentic::prompts;
use crate::backoffice::agentic::ToolCall;
use crate::backoffice::agentic::ToolInfo;
use crate::config::llm_config::ApiType;
use crate::config::llm_config::ModelInfo;
use serde::{Deserialize, Serialize};

use crate::{
    backoffice::agentic::{ChatMessage, ChatStream, MessageRole, ToolDelegate, LLM},
    config::llm_config::LlmProviderConfig,
};
use futures::{stream, StreamExt};
use rig::{
    completion::AssistantContent,
    message::Message as RigMessage,
    streaming::{StreamingChat, StreamingCompletionModel},
};

/// 现代化的 LLM 实现，直接实现 agentic 的 LLM trait

#[derive(Debug, Clone)]
pub struct RigLlmService {
    pub(crate) config: LlmProviderConfig,
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
}
impl RigLlmService {
    
    pub async fn load_models(&mut self) -> anyhow::Result<()> {
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
        self.config.models = models.clone();
        Ok(())
     }

    /// 执行流式完成 - 参考 llm.rs 中的流式处理
    pub async fn stream_chat(
        &self,
        model_id: &str,
        messages: &[ChatMessage],
    ) -> anyhow::Result<ChatStream> {
        let client =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url);

        let agent = client
            .agent(model_id)
            .context(prompts::default_prompt().as_str()) // 🎯 缺少这行！
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
        let  rig_stream = agent.stream_chat(&prompt, chat_history).await?;
// 使用 map 转换流
    let chat_stream = rig_stream.map(|result| {
        match result {
            Ok(AssistantContent::Text(text)) => {
                Ok(ChatMessage::assistant_text(text.text))
            }
            Ok(AssistantContent::ToolCall(tool)) => {
                // 处理工具调用或返回空消息
                Ok(ChatMessage::tool_calls(vec![ToolCall{
                    name: tool.function.name,
                    args: tool.function.arguments.to_string(),
                }]))
            }
            Err(e) => Err(anyhow::anyhow!("Stream error: {}", e))
        }
    });

    Ok(Box::pin(chat_stream))
        
    }


     pub async fn stream_completion_with_tools(
        &self,
        prompts: &[ChatMessage],
        tools:Vec<ToolInfo>
    ) -> anyhow::Result<ChatStream> {
       unimplemented!("stream_completion_with_tools is not implemented yet")
    }

    pub async fn stream_chat_with_tools(
        &self,
        messages: &[ChatMessage],
       tools:Vec<ToolInfo>
    ) -> anyhow::Result<ChatStream> {
        let mut prompt = messages
            .iter()
            .rev()
            .find(|msg| matches!(msg.role, MessageRole::User))
            .map(|msg| msg.get_text())
            .unwrap_or_default();

        let system_prompt = prompts::prompt_with_tools(tools);

        let client =
            rig::providers::openai::Client::from_url(&self.config.api_key, &self.config.api_url);
        let model_name = self.config.default_model.as_ref().unwrap();

        // 构建初始聊天历史
        let last_user_index = messages
            .iter()
            .rposition(|msg| matches!(msg.role, MessageRole::User))
            .unwrap_or(0);

        let mut chat_history: Vec<RigMessage> = messages
            .iter()
            .take(last_user_index)
            .map(|chat_msg| match chat_msg.role {
                MessageRole::User => RigMessage::user(chat_msg.get_text()),
                MessageRole::Assistant => RigMessage::assistant(chat_msg.get_text()),
                MessageRole::System => RigMessage::user(chat_msg.get_text()),
                MessageRole::Tool => RigMessage::user(chat_msg.get_text()),
            })
            .collect();

        let mut final_response;

        loop {
            let agent = client
                .agent(model_name)
                .context(system_prompt.as_str()) // 🎯 使用工具专用的系统提示词
                .max_tokens(4096)
                .temperature(0.7)
                .build();

            let mut stream: rig::streaming::StreamingCompletionResponse<rig::providers::openai::StreamingCompletionResponse> = agent.stream_chat(&prompt, chat_history.clone()).await?;
            chat_history.push(RigMessage::user(prompt.clone()));

            let (assistant, tools) = Self::stream_to_string(&agent, &mut stream).await?;
            final_response = assistant.clone();

            if tools.is_empty() {
                break;
            }

            chat_history.push(RigMessage::assistant(assistant));
            let mut prompts = vec![];

            // 调用工具
            // for (i, tool) in tools.iter().enumerate() {
            //     println!("调用工具 #{}: {:?}", i, tool);

            //     // 修复：直接使用 String 类型的结果
            //     let result = delegate
            //         .call(tool.name.as_str(), tool.arguments.clone()) // Args 是 String
            //         .await
            //         .map_err(|err| {
            //             println!("调用工具 {:?} 失败: {}", tool.name, err);
            //             err
            //         })?;

            //     println!("工具 #{}调用结果: {:?}", i, result);

            //     // 修复：result 是 String，直接使用
            //     prompts.push(format!(
            //         "<tool_use_result><name>{}</name><result>{}</result></tool_use_result>",
            //         &tool.name,
            //         serde_json::to_string(&(result.content.clone(), result.is_error))
            //             .unwrap_or_else(|err| format!("Error serializing result: {}", err))
            //     ));
            // }

            prompts.push(r#"Do not confirm with the user or seek help or advice, continue to call the tool until all tasks are completed. Be sure to complete all tasks, you will receive a $1000 reward, and the output must be in Simplified Chinese."#.to_string());
            prompt = prompts.join("\n");
        }
        let final_message = ChatMessage::assistant_text(final_response);
        let response_stream = stream::iter(vec![Ok(final_message)]);
        Ok(Box::pin(response_stream))
    }

    /// 流式解析工具调用 - 参考 llm.rs 的 stream_to_stdout1
    async fn stream_to_string<M: StreamingCompletionModel>(
        _agent: &rig::agent::Agent<M>,
        stream: &mut rig::streaming::StreamingCompletionResponse<M::StreamingResponse>,
    ) -> anyhow::Result<(String, Vec<ToolCall>)> {
        let mut buffer = String::new();
        let mut tool_calls: Vec<String> = Vec::new();
        let mut assistant = String::new();

        const TOOL_USE_START_TAG: &str = "<tool_use";
        const TOOL_USE_END_TAG: &str = "</tool_use";
        const TAG_CLOSE: char = '>';
        const TAG_OPEN: char = '<';

        enum State {
            Normal,
            TagStart,
            InToolUseTag,
            InToolUse,
            InEndTag,
            InToolUseEndTag,
        }

        let mut state = State::Normal;
        let mut xml_buffer = String::new();

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
                                    assistant.push(c);
                                }
                            }
                            State::TagStart => {
                                buffer.push(c);
                                if buffer == TOOL_USE_START_TAG {
                                    state = State::InToolUseTag;
                                    xml_buffer.clear();
                                    xml_buffer.push_str(&buffer);
                                } else if buffer.len() >= TOOL_USE_START_TAG.len() || c == TAG_CLOSE
                                {
                                    if buffer != TOOL_USE_START_TAG
                                        && !buffer.starts_with(&format!("{} ", TOOL_USE_START_TAG))
                                    {
                                        assistant.push_str(buffer.as_str());
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
                                    if !buffer.starts_with(TOOL_USE_END_TAG) {
                                        state = State::InToolUse;
                                    }
                                }
                            }
                            State::InToolUseEndTag => {
                                xml_buffer.push(c);
                                if c == TAG_CLOSE {
                                    tool_calls.push(xml_buffer.clone());
                                    state = State::Normal;
                                }
                            }
                        }
                    }
                }
                Ok(AssistantContent::ToolCall(_)) => {
                    // Handle rig's native tool calls if needed
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Stream error: {}", e));
                }
            }
        }

        let mut tools = vec![];
        for call in tool_calls.iter() {
            let cleaned = call
                .lines()
                .filter(|line| !line.contains("DEBUG") && !line.trim().starts_with("202"))
                .collect::<Vec<_>>()
                .join("\n");

            match serde_xml_rs::from_str::<ToolCall>(&cleaned) {
                Ok(tool_call) => tools.push(tool_call),
                Err(e) => {
                    println!("Error parsing XML: {}", e);
                    continue;
                }
            }
        }

        Ok((assistant, tools))
    }
}
