use crate::backoffice::llm::provider::LlmChoice;
use crate::backoffice::llm::types::{ChatMessage, ChatStream, MessageContent};
use crate::backoffice::mcp::McpRegistry;
use rmcp::model::RawContent;

pub(crate) async fn chat_stream_with_tools_simple(
    llm: LlmChoice,
    model_id: &str,
    chat_history: Vec<ChatMessage>,
    max_tool_rounds: usize,
) -> anyhow::Result<ChatStream> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let llm_clone = llm.clone();
    let model_id_clone = model_id.to_string();
    let history_clone = chat_history.clone();

    actix::Arbiter::new().spawn(async move {
        let mut current_history = history_clone;
        let mut round = 0;
        while round < max_tool_rounds {
            let stream_result = llm_clone
                .stream_chat(&model_id_clone, &current_history)
                .await;
            let mut stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = sender.send(Err(e));
                    break;
                }
            };

            let mut accumulated_text = String::new();
            let mut tool_calls = Vec::new();
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                tracing::debug!("LLM响应流消息: {:?}", chunk);
                match chunk {
                    Ok(message) => {
                        if message.is_tool_call() {
                            tool_calls.extend(message.get_tool_calls());
                        } else {
                            accumulated_text.push_str(&message.get_text());
                        }
                        if sender.send(Ok(message)).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                        return;
                    }
                }
            }
            if !accumulated_text.trim().is_empty() {
                current_history.push(ChatMessage::assistant().with_text(accumulated_text));
            }
            if tool_calls.is_empty() {
                break;
            }
            let _ = sender.send(Ok(ChatMessage::system().with_text(format!(
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
                let result_message = ChatMessage::system().with_content(MessageContent::ToolResult(
                    tool_call.name.clone(),
                    content,
                ));
                current_history.push(result_message.clone());
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
