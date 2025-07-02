use std::time::Duration;

use super::*;
use crate::{app::AppState, config::llm_config::LlmProviders};
use futures::channel;
use gpui::*;
use rig::message::*;

impl TodoThreadChat {
    pub(crate) fn send_message(
        &mut self,
        _: &SendMessage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let todo = self.todoitem.clone().clone();
        let message_content = self.chat_input.read(cx).value();
        if message_content.is_empty() {
            return;
        }
        let message_content = message_content.to_string().trim().to_string();

        let user_message = ChatMessage {
            id: format!("user_{}", chrono::Utc::now().timestamp()),
            role: MessageRole::User,
            content: message_content.clone(),
            timestamp: chrono::Utc::now(),
            model: None,
            tools_used: vec![],
        };

        self.chat_messages.push(user_message);

        self.chat_input
            .update(cx, |input, cx| input.set_value("", window, cx));

        self.is_loading = true;
        // self.simulate_ai_response(message_content, cx);
        self.scroll_handle.scroll_to_bottom();
        // 从选择的模型中获取第一个作为当前使用的模型

        let selected_model = self.todoitem.selected_model.clone();
        println!("使用模型: {:?}", selected_model);
        // 获取当前选择的模型提供商信息
        if let Some(selected_model) = selected_model {
            let provider_info = LlmProviders::get_enabled_providers()
                .iter()
                .find(|provider| provider.id == selected_model.provider_id)
                .cloned();
            let model_id = selected_model.model_id.clone();
            self.chat_messages.push(ChatMessage {
                id: format!("assistant_{}", chrono::Utc::now().timestamp()),
                role: MessageRole::Assistant,
                content: "".to_string(),
                timestamp: chrono::Utc::now(),
                model: Some(selected_model.model_name.clone()),
                tools_used: vec![],
            });
            if let Some(provider) = provider_info {
                // let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
                // let _sub = xbus::subscribe(move |msg: &Message| {
                //     tx.try_send(msg.clone()).unwrap_or_else(|e| {
                //         tracing::error!("Failed to send message to channel: {}", e);
                //     });
                // });
                tokio::spawn(async move {
                    if let Err(err) = provider.stream_chat(&model_id, &message_content).await {
                        tracing::error!("Error streaming chat: {}", err);
                    }
                });
            }
        }
        // 从选择的工具中获取所有工具信息
        if !self.todoitem.selected_tools.is_empty() {
            let tool_names: Vec<String> = self
                .todoitem
                .selected_tools
                .iter()
                .map(|tool| format!("{} ({})", tool.tool_name, tool.provider_name))
                .collect();
            println!("使用工具: {}", tool_names.join(", "));
        }
        // cx.notify();
    }
}
