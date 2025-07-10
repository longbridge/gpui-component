mod provider;
pub mod types;

use crate::backoffice::llm::provider::LlmProvider;
use crate::backoffice::llm::types::{ChatMessage, ChatStream, MediaContent, ToolCall};
use crate::backoffice::mcp::McpRegistry;
use crate::{
    backoffice::YamlFile,
    config::{llm_config::*, provider_config_path, todo_item::SelectedTool},
};
use actix::prelude::*;
use rmcp::model::RawContent;
use std::{collections::HashMap, time::Duration};

#[derive(Message)]
#[rtype(result = "anyhow::Result<ChatStream>")]
pub struct LlmChatRequest {
    pub provider_id: String,
    pub model_id: String,
    pub source: String,
    pub messages: Vec<ChatMessage>,
    // pub tool_delegate: Option<Box<dyn crate::backoffice::agentic::ToolDelegate<Output = crate::backoffice::mcp::McpCallToolResult, Args = String>>>,
}

// Registry 保持不变，但使用新的命名
pub struct LlmRegistry {
    providers: HashMap<String, LlmProviderConfig>,
    file: YamlFile,
    handle: Option<SpawnHandle>,
}

impl LlmRegistry {
    /// 获取全局注册表实例
    pub fn global() -> Addr<Self> {
        LlmRegistry::from_registry()
    }

    fn check_and_update(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
        if self.file.modified()? {
            let configs = LlmProviderManager::list_providers();
            let enabled_ids: Vec<_> = configs
                .iter()
                .filter(|config| config.enabled)
                .map(|config| config.id.as_str())
                .collect();

            // 移除不再启用的提供商
            self.providers
                .retain(|id, _| enabled_ids.contains(&id.as_str()));

            // 添加新启用的提供商
            for config in configs.iter().filter(|c| c.enabled) {
                if !self.providers.contains_key(&config.id) {
                    let config_clone = config.clone();
                    let mut config = config_clone.clone();
                    async move {
                        let llm = LlmProvider::new(&config_clone)?;
                        llm.load_models().await
                    }
                    .into_actor(self)
                    .then(move |models, act, ctx| match models {
                        Ok(models) => {
                            tracing::trace!("Loaded models for {}: {:?}", config.id, models);
                            config.models = models;
                            act.providers.insert(config.id.clone(), config);
                            fut::ready(())
                        }
                        Err(err) => {
                            tracing::trace!("Failed to load models for {}: {}", config.id, err);
                            fut::ready(())
                        }
                    })
                    .spawn(ctx);
                }
            }
            self.file.open()?;
        }
        Ok(())
    }
}

impl Default for LlmRegistry {
    fn default() -> Self {
        let file = YamlFile::new(provider_config_path());
        Self {
            providers: HashMap::new(),
            file,
            handle: None,
        }
    }
}

impl Supervised for LlmRegistry {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        log::info!("LlmRegistry is restarting");
    }
}

impl SystemService for LlmRegistry {}

impl LlmRegistry {
    fn tick(&mut self, ctx: &mut Context<Self>) {
        if let Ok(false) = &self.file.exist() {
            self.providers.clear();
            return;
        }
        if let Err(err) = self.check_and_update(ctx) {
            tracing::error!("{} {err}", self.file.path.display());
        }
    }
}

impl Actor for LlmRegistry {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let handle = ctx.run_interval(Duration::from_secs(1), Self::tick);
        self.handle = Some(handle);
        tracing::trace!("LlmRegistry started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::trace!("LlmRegistry stopped");
    }
}

impl Handler<LlmChatRequest> for LlmRegistry {
    type Result = ResponseActFuture<Self, anyhow::Result<ChatStream>>;

    fn handle(&mut self, msg: LlmChatRequest, _ctx: &mut Self::Context) -> Self::Result {
        tracing::trace!(
            "Received LLM chat request: provider_id={}, model_id={}, source={}, messages={}",
            msg.provider_id,
            msg.model_id,
            msg.source,
            msg.messages.len(),
        );

        if let Some(config) = self.providers.get(&msg.provider_id).cloned() {
            let model_id = msg.model_id.clone();
            let messages = msg.messages;
            let source = msg.source;

            async move {
                tracing::trace!(
                    "Starting LLM chat with provider: {}, model: {}, source: {}",
                    msg.provider_id,
                    model_id,
                    source
                );

                let llm = LlmProvider::new(&config)?;

                // 检查是否需要工具调用
                let has_tools = messages.iter().any(|msg| msg.has_tool_definitions());

                if has_tools {
                    // 有工具调用需求，使用工具处理流
                    create_tool_enabled_stream(llm, &model_id, &messages).await
                } else {
                    // 普通对话，直接使用provider的流
                    llm.stream_chat(&model_id, &messages).await
                }
            }
            .into_actor(self)
            .map(|res, _act, _ctx| res)
            .boxed_local()
        } else {
            let provider_id = msg.provider_id.clone();
            async move {
                Err(anyhow::anyhow!(
                    "Provider '{}' not found or not enabled",
                    provider_id
                ))
            }
            .into_actor(self)
            .boxed_local()
        }
    }
}

/// 创建支持工具调用的流
async fn create_tool_enabled_stream(
    llm: LlmProvider,
    model_id: &str,
    messages: &[ChatMessage],
) -> anyhow::Result<ChatStream> {
    use futures::stream;

    enum ToolStreamState {
        Streaming {
            stream: Option<ChatStream>,
            chat_history: Vec<ChatMessage>,
            accumulated_response: String,
        },
        ExecutingTool {
            tool_call: ToolCall,
            chat_history: Vec<ChatMessage>,
        },
        Finished,
    }

    let initial_state = ToolStreamState::Streaming {
        stream: None,
        chat_history: messages.to_vec(),
        accumulated_response: String::new(),
    };

    let tool_stream = stream::unfold(
        (llm, model_id.to_string(), initial_state),
        |(llm, model_id, state)| async move {
            match state {
                ToolStreamState::Streaming {
                    mut stream,
                    mut chat_history,
                    mut accumulated_response,
                } => {
                    // 如果没有活跃的流，创建新的
                    if stream.is_none() {
                        match llm.stream_chat(&model_id, &chat_history).await {
                            Ok(new_stream) => stream = Some(new_stream),
                            Err(e) => {
                                return Some((
                                    Err(anyhow::anyhow!("Failed to create stream: {}", e)),
                                    (llm, model_id, ToolStreamState::Finished),
                                ))
                            }
                        }
                    }

                    // 处理流数据
                    if let Some(mut current_stream) = stream {
                        use futures::StreamExt;

                        while let Some(chunk) = current_stream.next().await {
                            match chunk {
                                Ok(message) => {
                                    if message.is_tool_call() {
                                        // 检测到工具调用
                                        let tool_calls = message.get_tool_calls();
                                        if let Some(tool_call) = tool_calls.first().cloned() {
                                            // 保存当前响应到历史
                                            if !accumulated_response.is_empty() {
                                                chat_history.push(ChatMessage::assistant_text(
                                                    accumulated_response.clone(),
                                                ));
                                            }

                                            let new_state = ToolStreamState::ExecutingTool {
                                                tool_call: tool_call.clone(),
                                                chat_history,
                                            };

                                            return Some((Ok(message), (llm, model_id, new_state)));
                                        }
                                    } else {
                                        // 普通文本消息
                                        accumulated_response.push_str(&message.get_text());

                                        let new_state = ToolStreamState::Streaming {
                                            stream: Some(current_stream),
                                            chat_history,
                                            accumulated_response,
                                        };

                                        return Some((Ok(message), (llm, model_id, new_state)));
                                    }
                                }
                                Err(e) => {
                                    return Some((
                                        Err(e),
                                        (llm, model_id, ToolStreamState::Finished),
                                    ));
                                }
                            }
                        }

                        // 流结束
                        return None;
                    }

                    None
                }
                ToolStreamState::ExecutingTool {
                    tool_call,
                    mut chat_history,
                } => {
                    // 执行工具调用
                    tracing::info!("执行工具调用: {:?}", tool_call);

                    let tool_result = match McpRegistry::call_tool(
                        tool_call.id(),
                        tool_call.tool_name(),
                        &tool_call.args,
                    )
                    .await
                    {
                        Ok(result) => {
                            // 添加工具结果到历史
                            let mut chat_message = ChatMessage::user_text("工具调用结果: ");
                            result.content.iter().for_each(|content| match content.raw {
                                RawContent::Text(ref text) => {
                                    chat_message.add_text(text.text.clone());
                                }
                                RawContent::Image(ref image) => {
                                    unimplemented!("处理图片内容: {:?}", image);
                                }
                                RawContent::Resource(ref resource) => {
                                    unimplemented!("处理资源内容: {:?}", resource);
                                }
                                RawContent::Audio(ref audio) => {
                                    unimplemented!("处理音频内容: {:?}", audio);
                                }
                            });
                            chat_message
                        }
                        Err(e) => ChatMessage::user_text(format!("工具调用失败: {}", e)),
                    };

                    // 添加工具结果到历史
                    chat_history.push(tool_result.clone());

                    // 继续对话
                    let continuation_prompt =
                        ChatMessage::user_text("继续完成任务，基于工具调用的结果。");
                    chat_history.push(continuation_prompt);

                    let new_state = ToolStreamState::Streaming {
                        stream: None,
                        chat_history,
                        accumulated_response: String::new(),
                    };

                    // 返回工具执行结果消息
                    Some((Ok(tool_result.clone()), (llm, model_id, new_state)))
                }
                ToolStreamState::Finished => None,
            }
        },
    );

    Ok(Box::pin(tool_stream))
}

impl LlmRegistry {
    /// 普通对话流
    pub async fn chat_stream(
        provider_id: &str,
        model_id: &str,
        source: &str,
        messages: Vec<ChatMessage>,
    ) -> anyhow::Result<ChatStream> {
        let registry = Self::global();
        let result = registry
            .send(LlmChatRequest {
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                source: source.to_string(),
                messages,
            })
            .await??;
        Ok(result)
    }

    // /// 带工具调用的对话流
    // pub async fn chat_stream_with_tools(
    //     provider_id: &str,
    //     model_id: &str,
    //     source: &str,
    //     messages: Vec<ChatMessage>,
    // ) -> anyhow::Result<ChatStream> {
    //     let registry = Self::global();
    //     let result = registry
    //         .send(LlmChatRequest {
    //             provider_id: provider_id.to_string(),
    //             model_id: model_id.to_string(),
    //             source: source.to_string(),
    //             messages,
    //         })
    //         .await??;
    //     Ok(result)
    // }
}
