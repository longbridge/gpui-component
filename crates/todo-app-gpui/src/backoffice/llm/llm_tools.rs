#[derive(Message)]
#[rtype(result = "anyhow::Result<ChatStream>")]
pub struct LlmChatRequest {
    pub provider_id: String,
    pub model_id: String,
    pub source: String,
    pub messages: Vec<ChatMessage>,
    pub tool_delegate: Option<Box<dyn crate::backoffice::agentic::ToolDelegate<Output = crate::backoffice::mcp::McpCallToolResult, Args = String>>>,
}

impl Handler<LlmChatRequest> for LlmRegistry {
    type Result = ResponseActFuture<Self, anyhow::Result<ChatStream>>;

    fn handle(&mut self, msg: LlmChatRequest, _ctx: &mut Self::Context) -> Self::Result {
        tracing::trace!(
            "Received LLM chat request: provider_id={}, model_id={}, source={}, messages={}, has_tools={}",
            msg.provider_id,
            msg.model_id,
            msg.source,
            msg.messages.len(),
            msg.tool_delegate.is_some()
        );

        if let Some(config) = self.providers.get(&msg.provider_id).cloned() {
            let model_id = msg.model_id.clone();
            let messages = msg.messages;
            let tool_delegate = msg.tool_delegate;
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
                
                if has_tools && tool_delegate.is_some() {
                    // 有工具调用需求，使用工具处理流
                    create_tool_enabled_stream(llm, &model_id, &messages, tool_delegate.unwrap()).await
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
    llm: LlmProvider<'_>,
    model_id: &str,
    messages: &[ChatMessage],
    tool_delegate: Box<dyn crate::backoffice::agentic::ToolDelegate<Output = crate::backoffice::mcp::McpCallToolResult, Args = String>>,
) -> anyhow::Result<ChatStream> {
    use futures::stream;

    #[derive(Debug)]
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
        (llm, model_id.to_string(), tool_delegate, initial_state),
        |(llm, model_id, tool_delegate, state)| async move {
            match state {
                ToolStreamState::Streaming { mut stream, mut chat_history, mut accumulated_response } => {
                    // 如果没有活跃的流，创建新的
                    if stream.is_none() {
                        match llm.stream_chat(&model_id, &chat_history).await {
                            Ok(new_stream) => stream = Some(new_stream),
                            Err(e) => return Some((
                                Err(anyhow::anyhow!("Failed to create stream: {}", e)),
                                (llm, model_id, tool_delegate, ToolStreamState::Finished)
                            )),
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
                                        if let Some(tool_calls) = message.get_tool_calls() {
                                            if let Some(tool_call) = tool_calls.first() {
                                                // 保存当前响应到历史
                                                if !accumulated_response.is_empty() {
                                                    chat_history.push(ChatMessage::assistant_text(accumulated_response.clone()));
                                                }
                                                
                                                let new_state = ToolStreamState::ExecutingTool {
                                                    tool_call: tool_call.clone(),
                                                    chat_history,
                                                };
                                                
                                                return Some((
                                                    Ok(message),
                                                    (llm, model_id, tool_delegate, new_state)
                                                ));
                                            }
                                        }
                                    } else {
                                        // 普通文本消息
                                        accumulated_response.push_str(&message.get_text());
                                        
                                        let new_state = ToolStreamState::Streaming {
                                            stream: Some(current_stream),
                                            chat_history,
                                            accumulated_response,
                                        };
                                        
                                        return Some((
                                            Ok(message),
                                            (llm, model_id, tool_delegate, new_state)
                                        ));
                                    }
                                }
                                Err(e) => {
                                    return Some((
                                        Err(e),
                                        (llm, model_id, tool_delegate, ToolStreamState::Finished)
                                    ));
                                }
                            }
                        }
                        
                        // 流结束
                        return None;
                    }
                    
                    None
                }
                ToolStreamState::ExecutingTool { tool_call, mut chat_history } => {
                    // 执行工具调用
                    tracing::info!("执行工具调用: {:?}", tool_call);
                    
                    let tool_result = match tool_delegate.call(&tool_call.name, tool_call.args.clone()).await {
                        Ok(result) => result.content,
                        Err(e) => format!("工具调用失败: {}", e),
                    };

                    // 添加工具结果到历史
                    chat_history.push(ChatMessage::tool_response(&tool_call.name, &tool_result));

                    // 继续对话
                    let continuation_prompt = ChatMessage::user_text("继续完成任务，基于工具调用的结果。");
                    chat_history.push(continuation_prompt);

                    let new_state = ToolStreamState::Streaming {
                        stream: None,
                        chat_history,
                        accumulated_response: String::new(),
                    };
                    
                    // 返回工具执行结果消息
                    Some((
                        Ok(ChatMessage::tool_response(&tool_call.name, &tool_result)),
                        (llm, model_id, tool_delegate, new_state)
                    ))
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
                tool_delegate: None,
            })
            .await??;
        Ok(result)
    }

    /// 带工具调用的对话流
    pub async fn chat_stream_with_tools(
        provider_id: &str,
        model_id: &str,
        source: &str,
        messages: Vec<ChatMessage>,
        tool_delegate: Box<dyn crate::backoffice::agentic::ToolDelegate<Output = crate::backoffice::mcp::McpCallToolResult, Args = String>>,
    ) -> anyhow::Result<ChatStream> {
        let registry = Self::global();
        let result = registry
            .send(LlmChatRequest {
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                source: source.to_string(),
                messages,
                tool_delegate: Some(tool_delegate),
            })
            .await??;
        Ok(result)
    }
}