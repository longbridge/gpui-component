use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::agent::types::{Agent, AgentContext, AgentDescriptor, AgentEvent, AgentResult};
use crate::llm::{ChatRequest, Message, Role, extract_stream_text};

static DESCRIPTOR: AgentDescriptor = AgentDescriptor {
    id: "general_chat",
    display_name: "General Chat",
    description: "General-purpose conversational AI assistant. Handles everyday questions, \
                  explanations, writing, brainstorming, and any request that doesn't match a \
                  specialized agent.",
    keywords: &[],
    command_prefix: None,
    examples: &[
        "Tell me a joke",
        "Explain how async/await works in Rust",
        "Help me write an email",
    ],
    required_capabilities: &[],
    priority: 100,
};

/// The default catch-all agent that handles general conversation via streaming LLM.
pub struct GeneralChatAgent;

#[async_trait]
impl Agent for GeneralChatAgent {
    fn descriptor(&self) -> &AgentDescriptor {
        &DESCRIPTOR
    }

    async fn execute(&self, ctx: AgentContext, tx: mpsc::Sender<AgentEvent>) {
        let result = self.run(ctx, &tx).await;
        if let Err(e) = result {
            let _ = tx.send(AgentEvent::Error(e)).await;
        }
    }
}

impl GeneralChatAgent {
    async fn run(&self, ctx: AgentContext, tx: &mpsc::Sender<AgentEvent>) -> Result<(), String> {
        // Build message list from history + current user input.
        let mut messages = ctx.chat_history.clone();
        messages.push(Message::text(Role::User, &ctx.user_input));

        let request = ChatRequest {
            model: ctx.provider_config.model.clone(),
            messages,
            max_tokens: ctx
                .provider_config
                .max_tokens
                .map(|v| v as u32)
                .or(Some(4096)),
            temperature: ctx.provider_config.temperature.or(Some(0.7)),
            stream: Some(true),
            ..Default::default()
        };

        // Obtain the LLM provider via the context's provider state.
        let provider = ctx
            .provider_state
            .manager()
            .get_provider(&ctx.provider_config)
            .await
            .map_err(|e| format!("Failed to get provider: {}", e))?;

        let mut stream = provider
            .chat_stream(&request)
            .await
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        let mut full_content = String::new();
        let mut pending_delta = String::new();
        let mut last_emit = Instant::now();
        let throttle = Duration::from_millis(50);

        loop {
            tokio::select! {
                _ = ctx.cancel_token.cancelled() => {
                    let _ = tx.send(AgentEvent::Cancelled).await;
                    return Ok(());
                }
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(response)) => {
                            if let Some(content) = extract_stream_text(&response) {
                                full_content.push_str(content);
                                pending_delta.push_str(content);

                                if last_emit.elapsed() >= throttle {
                                    let delta = std::mem::take(&mut pending_delta);
                                    let _ = tx.send(AgentEvent::TextDelta(delta)).await;
                                    last_emit = Instant::now();
                                }
                            }

                            let is_done = response.choices.iter().any(|c| {
                                c.finish_reason
                                    .as_ref()
                                    .map(|r| r != "null")
                                    .unwrap_or(false)
                            });

                            if is_done {
                                if !pending_delta.is_empty() {
                                    let _ = tx.send(AgentEvent::TextDelta(pending_delta)).await;
                                }
                                let _ = tx
                                    .send(AgentEvent::Completed(AgentResult {
                                        content: full_content,
                                        ..Default::default()
                                    }))
                                    .await;
                                return Ok(());
                            }
                        }
                        Some(Err(e)) => {
                            return Err(format!("Stream error: {}", e));
                        }
                        None => {
                            // Stream ended without an explicit finish_reason.
                            if !pending_delta.is_empty() {
                                let _ = tx.send(AgentEvent::TextDelta(pending_delta)).await;
                            }
                            let _ = tx
                                .send(AgentEvent::Completed(AgentResult {
                                    content: full_content,
                                    ..Default::default()
                                }))
                                .await;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}
