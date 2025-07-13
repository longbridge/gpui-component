use crate::backoffice::llm::provider::LlmProvider;
use crate::backoffice::llm::types::{ChatMessage, ChatStream, MessageContent};
use crate::backoffice::mcp::McpRegistry;
use rmcp::model::RawContent;

pub(crate) async fn chat_stream_with_tools_simple(
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
            let stream_result = llm_clone
                .stream_chat(&model_id_clone, &current_history)
                .await;
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
                tracing::debug!("LLM响应流消息: {:?}", chunk);
                match chunk {
                    Ok(message) => {
                        if message.is_tool_call() {
                            tool_calls.extend(message.get_tool_calls());
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
                current_history.push(ChatMessage::assistant().with_text(accumulated_text));
            }

            // 4. 如果没有工具调用，说明流程结束，退出循环
            if tool_calls.is_empty() {
                break;
            }

            // // 5. 执行所有收集到的工具
            let _ = sender.send(Ok(ChatMessage::assistant().with_text(format!(
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
                let content = format!(
                    "<tool_use_result>
                        <name>{}</name>
                        <result>{}</result>
                    </tool_use_result>",
                    tool_call.name, result_content
                );
                // 创建工具结果消息，并添加到历史记录
                let result_message = ChatMessage::system().with_content(MessageContent::ToolResult(
                    tool_call.name.clone(),
                    result_content,
                ));
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
