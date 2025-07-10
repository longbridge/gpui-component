mod parser;
mod provider;
pub mod types;

use crate::backoffice::llm::provider::LlmProvider;
use crate::backoffice::llm::types::{ChatMessage, ChatStream, ToolCall};
use crate::backoffice::mcp::McpRegistry;
use crate::{
    backoffice::YamlFile,
    config::{llm_config::*, provider_config_path},
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
}

pub struct LlmRegistry {
    providers: HashMap<String, LlmProviderConfig>,
    file: YamlFile,
    handle: Option<SpawnHandle>,
}

impl LlmRegistry {
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
                    .then(move |models, act, _ctx| match models {
                        Ok(models) => {
                            tracing::trace!("Loaded models for {}: {:?}", config.id, models);
                            config.models = models;
                            act.providers.insert(config.id.clone(), config);
                            fut::ready(())
                        }
                        Err(err) => {
                            tracing::error!("Failed to load models for {}: {}", config.id, err);
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
                // llm.stream_chat(&model_id, &messages).await
                create_tool_enabled_stream(llm, &model_id, &messages).await
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

async fn create_tool_enabled_stream(
    llm: LlmProvider,
    model_id: &str,
    messages: &[ChatMessage],
) -> anyhow::Result<ChatStream> {
    use futures::stream;
    use futures::StreamExt;

    enum ToolStreamState {
        Streaming {
            stream: Option<ChatStream>,
            chat_history: Vec<ChatMessage>,
            accumulated_response: String,
            pending_tool_calls: Vec<ToolCall>,
            tool_rounds_completed: usize,
            max_tool_rounds: usize,
        },
        ExecutingTools {
            tool_calls: Vec<ToolCall>,
            chat_history: Vec<ChatMessage>,
            accumulated_response: String,
            tool_rounds_completed: usize,
            max_tool_rounds: usize,
        },
        Finished,
    }

    let initial_state = ToolStreamState::Streaming {
        stream: None,
        chat_history: messages.to_vec(),
        accumulated_response: String::new(),
        pending_tool_calls: Vec::new(),
        tool_rounds_completed: 0,
        max_tool_rounds: 128,
    };

    let tool_stream = stream::unfold(
        (llm, model_id.to_string(), initial_state),
        |(llm, model_id, state)| async move {
            match state {
                ToolStreamState::Streaming {
                    mut stream,
                    mut chat_history,
                    mut accumulated_response,
                    mut pending_tool_calls,
                    tool_rounds_completed,
                    max_tool_rounds,
                } => {
                    // 如果流为空，创建新流
                    if stream.is_none() {
                        // 超过最大轮数则移除工具定义，防止死循环
                        let filtered_history = if tool_rounds_completed >= max_tool_rounds {
                            chat_history
                                .into_iter()
                                .map(|mut msg| {
                                    msg.contents
                                        .retain(|content| !content.is_tool_definitions());
                                    msg
                                })
                                .filter(|msg| !msg.has_tool_definitions())
                                .collect()
                        } else {
                            chat_history.clone()
                        };

                        match llm.stream_chat(&model_id, &filtered_history).await {
                            Ok(new_stream) => {
                                stream = Some(new_stream);
                                chat_history = filtered_history;
                            }
                            Err(e) => {
                                return Some((
                                    Err(anyhow::anyhow!("Failed to create stream: {}", e)),
                                    (llm, model_id, ToolStreamState::Finished),
                                ));
                            }
                        }
                    }

                    if let Some(mut current_stream) = stream {
                        loop {
                            match current_stream.next().await {
                                Some(Ok(message)) => {
                                    if message.is_tool_call()
                                        && tool_rounds_completed < max_tool_rounds
                                    {
                                        // 收集工具调用，不把ToolCall消息放入chat_history
                                        let tool_calls = message.get_tool_calls();
                                        pending_tool_calls.extend(tool_calls.into_iter().cloned());

                                        let new_state = ToolStreamState::Streaming {
                                            stream: Some(current_stream),
                                            chat_history,
                                            accumulated_response,
                                            pending_tool_calls,
                                            tool_rounds_completed,
                                            max_tool_rounds,
                                        };
                                        return Some((Ok(message), (llm, model_id, new_state)));
                                    } else if message.is_tool_call()
                                        && tool_rounds_completed >= max_tool_rounds
                                    {
                                        // 超过最大轮数，将工具调用转换为普通文本
                                        let tool_calls = message.get_tool_calls();
                                        let tool_text = tool_calls
                                            .iter()
                                            .map(|tc| format!("想要调用工具: {}", tc.name))
                                            .collect::<Vec<_>>()
                                            .join(", ");

                                        let text_message = ChatMessage::assistant_text(format!(
                                            "{}（已达到最大工具调用轮数，无法执行）",
                                            tool_text
                                        ));

                                        let new_state = ToolStreamState::Streaming {
                                            stream: Some(current_stream),
                                            chat_history,
                                            accumulated_response,
                                            pending_tool_calls,
                                            tool_rounds_completed,
                                            max_tool_rounds,
                                        };
                                        return Some((
                                            Ok(text_message),
                                            (llm, model_id, new_state),
                                        ));
                                    } else if !message.get_text().trim().is_empty() {
                                        // 累积普通文本响应
                                        accumulated_response.push_str(&message.get_text());

                                        let new_state = ToolStreamState::Streaming {
                                            stream: Some(current_stream),
                                            chat_history,
                                            accumulated_response,
                                            pending_tool_calls,
                                            tool_rounds_completed,
                                            max_tool_rounds,
                                        };
                                        return Some((Ok(message), (llm, model_id, new_state)));
                                    } else {
                                        // 空消息，继续循环
                                        continue;
                                    }
                                }
                                Some(Err(e)) => {
                                    return Some((
                                        Err(e),
                                        (llm, model_id, ToolStreamState::Finished),
                                    ));
                                }
                                None => {
                                    // 流结束，处理收集到的工具调用
                                    if !pending_tool_calls.is_empty()
                                        && tool_rounds_completed < max_tool_rounds
                                    {
                                        if !accumulated_response.trim().is_empty() {
                                            chat_history.push(ChatMessage::assistant_text(
                                                accumulated_response.clone(),
                                            ));
                                        }

                                        let new_state = ToolStreamState::ExecutingTools {
                                            tool_calls: pending_tool_calls.clone(),
                                            chat_history,
                                            accumulated_response,
                                            tool_rounds_completed,
                                            max_tool_rounds,
                                        };

                                        let executing_message =
                                            ChatMessage::assistant_text(format!(
                                                "正在执行 {} 个工具... (第 {}/{} 轮)",
                                                pending_tool_calls.len(),
                                                tool_rounds_completed + 1,
                                                max_tool_rounds
                                            ));
                                        return Some((
                                            Ok(executing_message),
                                            (llm, model_id, new_state),
                                        ));
                                    } else {
                                        // 没有工具调用或超过最大轮数，流正常结束
                                        if !accumulated_response.trim().is_empty() {
                                            let final_message = if tool_rounds_completed
                                                >= max_tool_rounds
                                            {
                                                ChatMessage::assistant_text(format!(
                                                    "{}\n\n(已达到最大工具调用轮数 {})",
                                                    accumulated_response, max_tool_rounds
                                                ))
                                            } else {
                                                ChatMessage::assistant_text(accumulated_response)
                                            };
                                            return Some((
                                                Ok(final_message),
                                                (llm, model_id, ToolStreamState::Finished),
                                            ));
                                        }
                                        return None;
                                    }
                                }
                            }
                        }
                    }

                    None
                }

                ToolStreamState::ExecutingTools {
                    tool_calls,
                    mut chat_history,
                    accumulated_response: _,
                    tool_rounds_completed,
                    max_tool_rounds,
                } => {
                    tracing::debug!(
                        "开始执行 {} 个工具调用 (第 {}/{} 轮)",
                        tool_calls.len(),
                        tool_rounds_completed + 1,
                        max_tool_rounds
                    );

                    // 去重工具调用
                    let mut unique_calls = std::collections::HashSet::new();
                    let filtered_tool_calls: Vec<_> = tool_calls
                        .into_iter()
                        .filter(|tc| {
                            let key = format!("{}:{}", tc.name, tc.arguments);
                            if unique_calls.contains(&key) {
                                tracing::warn!("跳过重复的工具调用: {}", tc.name);
                                false
                            } else {
                                unique_calls.insert(key);
                                true
                            }
                        })
                        .collect();

                    // 执行所有工具调用，并把结果消息追加到 chat_history
                    for tool_call in &filtered_tool_calls {
                        tracing::debug!("执行工具调用: {:?}", tool_call);

                        let tool_result_message = match McpRegistry::call_tool(
                            tool_call.id(),
                            tool_call.tool_name(),
                            &tool_call.arguments,
                        )
                        .await
                        {
                            Ok(result) => {
                                tracing::debug!("工具调用成功: {:?}", result);

                                // 创建工具结果消息，使用 Tool 角色
                                let mut tool_result_content = String::new();
                                for content in &result.content {
                                    match &content.raw {
                                        RawContent::Text(text) => {
                                            tool_result_content.push_str(&text.text);
                                        }
                                        RawContent::Image(image) => {
                                            tool_result_content
                                                .push_str(&format!("图片内容: {:?}", image));
                                        }
                                        RawContent::Resource(resource) => {
                                            tool_result_content
                                                .push_str(&format!("资源内容: {:?}", resource));
                                        }
                                        RawContent::Audio(audio) => {
                                            tool_result_content
                                                .push_str(&format!("音频内容: {:?}", audio));
                                        }
                                    }
                                }

                                ChatMessage::tool_result_text(
                                    &tool_call.name,
                                    tool_result_content,
                                    true,
                                )
                            }
                            Err(e) => {
                                tracing::error!("工具调用失败: {}", e);
                                ChatMessage::tool_result_text(
                                    &tool_call.name,
                                    format!("错误: {}", e),
                                    false,
                                )
                            }
                        };

                        chat_history.push(tool_result_message); // 只加工具结果，不加调用
                    }

                    // 工具执行完成，增加轮数计数器
                    let new_tool_rounds_completed = tool_rounds_completed + 1;

                    // 返回工具执行完成的消息
                    let completion_message = ChatMessage::assistant_text(format!(
                        "已完成第 {} 轮工具执行（共 {} 个工具），正在生成基于结果的响应...",
                        new_tool_rounds_completed,
                        filtered_tool_calls.len()
                    ));

                    // 重新进入流式对话状态，让 LLM 基于工具结果生成响应
                    let new_state = ToolStreamState::Streaming {
                        stream: None,
                        chat_history,
                        accumulated_response: String::new(),
                        pending_tool_calls: Vec::new(),
                        tool_rounds_completed: new_tool_rounds_completed,
                        max_tool_rounds,
                    };

                    Some((Ok(completion_message), (llm, model_id, new_state)))
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

    /// 带工具的对话流
    pub async fn chat_stream_with_tools(
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
}

async fn chat_stream_with_tools_simple(
    llm: LlmProvider,
    model_id: &str,
    chat_history: Vec<ChatMessage>,
    max_tool_rounds: usize,
) -> anyhow::Result<ChatStream> {
    // 创建一个消息队列，用于将后台任务的消息实时发送到前端
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

    // 在后台任务中处理整个工具调用循环，避免阻塞
    let llm_clone = llm.clone();
    let model_id_clone = model_id.to_string();
    let history_clone = chat_history.clone();
    
    actix::Arbiter::new().spawn(async move {
        let mut current_history = history_clone;
        let mut round = 0;

        // 循环执行，直到达到最大轮次或没有工具可调用
        while round < max_tool_rounds {
            // 1. 调用LLM获取流式响应
            let stream_result = llm_clone.stream_chat(&model_id_clone, &current_history).await;
            let mut stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = sender.send(Err(e));
                    break; // 如果创建流失败，则退出
                }
            };

            let mut accumulated_text = String::new();
            let mut tool_calls = Vec::new();

            // 2. 处理并转发LLM的流式响应
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(message) => {
                        if message.is_tool_call() {
                            // 如果是工具调用，收集起来
                            if let Some(calls) = message.get_tool_calls() {
                                tool_calls.extend(calls.iter().cloned());
                            }
                        } else {
                            // 如果是文本，累加起来
                            accumulated_text.push_str(&message.get_text());
                        }
                        // 实时将原始消息转发给调用者
                        if sender.send(Ok(message)).is_err() {
                            // 如果接收端关闭，则任务没有意义，退出
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                        return; // 出现错误，终止任务
                    }
                }
            }

            // 3. 将LLM的文本响应添加到历史记录
            if !accumulated_text.trim().is_empty() {
                current_history.push(ChatMessage::assistant_text(accumulated_text));
            }

            // 4. 如果没有工具调用，说明流程结束，退出循环
            if tool_calls.is_empty() {
                break;
            }

            // 5. 执行所有收集到的工具
            let _ = sender.send(Ok(ChatMessage::assistant_text(format!(
                "执行 {} 个工具...",
                tool_calls.len()
            ))));

            for tool_call in &tool_calls {
                let result_content = match McpRegistry::call_tool(
                    tool_call.id(),
                    tool_call.tool_name(),
                    &tool_call.arguments,
                )
                .await
                {
                    Ok(result) => {
                        // 将工具结果转换为纯文本
                        result.content.iter().fold(String::new(), |mut acc, item| {
                            if let RawContent::Text(text) = &item.raw {
                                acc.push_str(&text.text);
                            }
                            acc
                        })
                    }
                    Err(e) => format!("工具调用失败: {}", e),
                };

                // 创建工具结果消息，并添加到历史记录
                let result_message =
                    ChatMessage::tool_result_text(&tool_call.name, result_content, true);
                current_history.push(result_message.clone());

                // 将工具结果也实时转发给调用者
                let _ = sender.send(Ok(result_message));
            }

            // 轮次加一，准备下一次循环
            round += 1;
        }
    });

    // 将接收器包装成一个流并返回
    let result_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Box::pin(result_stream))
}
