use crate::backoffice::llm::provider::LlmChoice;
use crate::backoffice::llm::types::{ChatMessage, ChatStream, MessageContent, ToolFunction};
use crate::backoffice::mcp::McpRegistry;
use futures::StreamExt;
use rmcp::model::RawContent;

pub(crate) async fn chat_stream_with_tools_simple(
    llm: LlmChoice,
    model_id: &str,
    prompt: &str,
    chat_history: Vec<ChatMessage>,
    max_tool_rounds: usize,
) -> anyhow::Result<ChatStream> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let llm_clone = llm.clone();
    let model_id_clone = model_id.to_string();
    let prompt = prompt.to_string();
    let chat_history = chat_history.clone();

    actix::Arbiter::new().spawn(async move {
        let mut round = 0;
        let mut prompt = prompt.to_string();
        let mut current_history = chat_history;

        while round < max_tool_rounds {
            let mut tool_calls = Vec::new();
            let stream_result = llm_clone
                .stream_chat(&model_id_clone, &prompt, &current_history)
                .await;
            let mut stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = sender.send(Err(e));
                    break;
                }
            };
            if round == 0 {
                current_history.push(ChatMessage::user().with_text(prompt.clone()));
            }

            let mut accumulated_text = String::new();
            while let Some(chunk) = stream.next().await {
                tracing::debug!("LLM响应流消息({}): {:?}", round, chunk);
                match chunk {
                    Ok(message) => {
                        if message.is_tool_function() {
                            tool_calls.extend(message.get_tool_function().cloned());
                        } else {
                            accumulated_text.push_str(&message.get_text());
                            sender.send(Ok(message)).ok();
                        }
                    }
                    Err(e) => {
                        sender.send(Err(e)).ok();
                        return;
                    }
                }
            }
            if tool_calls.is_empty() {
                break;
            }
            if !accumulated_text.trim().is_empty() {
                current_history.push(ChatMessage::assistant().with_text(accumulated_text));
            }
            sender
                .send(Ok(MessageContent::TextChunk(format!(
                    "执行 {} 个工具...\n",
                    tool_calls.len()
                ))))
                .ok();
            let mut tool_results = vec![];
            for tool_call in &tool_calls {
                let result_content = match McpRegistry::call_tool(
                    tool_call.tool_id(),
                    tool_call.tool_name(),
                    &tool_call.arguments,
                )
                .await
                {
                    Ok(result) => result.content.iter().fold(String::new(), |mut acc, item| {
                        if let RawContent::Text(text) = &item.raw {
                            acc.push_str(&text.text);
                        }
                        acc
                    }),
                    Err(e) => format!("工具调用失败: {}", e),
                };
                tracing::debug!(
                    "工具调用结果: {}({}) -> {}",
                    tool_call.tool_name(),
                    tool_call.arguments,
                    result_content
                );
                let message = MessageContent::ToolFunction(
                    ToolFunction::new(tool_call.name.clone(), tool_call.arguments.clone())
                        .with_result(result_content),
                );
                tool_results.push(message.clone());
                sender.send(Ok(message)).ok();
            }
            prompt = tool_results
                .iter()
                .map(|result| result.get_text())
                .collect::<Vec<_>>()
                .join("\n");
            round += 1;
        }
    });

    // 将接收器包装成一个流并返回
    let result_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Box::pin(result_stream))
}
