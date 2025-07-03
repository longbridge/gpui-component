use super::*;
use crate::backoffice::agentic::llm::LlmRegistry;
use crate::config::llm_config::LlmProviderManager;
use gpui::*;

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
            source: todo.id.clone(),
        };

        self.chat_messages.push(user_message);
        let source = todo.id.clone();
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
            let provider_info = LlmProviderManager::get_enabled_providers()
                .iter()
                .find(|provider| provider.id == selected_model.provider_id)
                .cloned();

            if let Some(provider) = provider_info {
                let model_id = selected_model.model_id.clone();
                let history_message = self.chat_messages.clone();
                self.chat_messages.push(ChatMessage {
                    id: format!("assistant_{}", chrono::Utc::now().timestamp()),
                    role: MessageRole::Assistant,
                    content: "".to_string(),
                    timestamp: chrono::Utc::now(),
                    model: Some(selected_model.model_name.clone()),
                    tools_used: vec![],
                    source: todo.id.clone(),
                });
                    let provider_id = provider.id.clone();
                    let model_id = model_id.clone();
                    let prompt = message_content.clone();
                    let source =self.todoitem.id.clone();
                     let chat_history = self.chat_messages.clone();
                if !self.todoitem.selected_tools.is_empty() {
                    let tools = self.todoitem.selected_tools.clone();
                    cx.spawn(async move |this,cx|{
                        LlmRegistry::chat_with_tools_static(&provider_id, &model_id, &source, &prompt, tools, chat_history).await.ok();
                    }).detach();
                    // tokio::spawn(async move {
                    //     if let Err(err) = provider
                    //         .stream_chat_with_tools(
                    //             &source,
                    //             &model_id,
                    //             &message_content,
                    //             tools,
                    //             history_message,
                    //         )
                    //         .await
                    //     {
                    //         tracing::error!("Error streaming chat: {}", err);
                    //     }
                    // });
                } else {
                      cx.spawn(async move |this,cx|{
                        LlmRegistry::chat_static(&provider_id, &model_id, &source, &prompt, chat_history).await.ok();
                    }).detach();
                    // tokio::spawn(async move {
                    //     if let Err(err) = provider
                    //         .stream_chat(&source, &model_id, &message_content, history_message)
                    //         .await
                    //     {
                    //         tracing::error!("Error streaming chat: {}", err);
                    //     }
                    // });
                }
            }
        }
        // 从选择的工具中获取所有工具信息

        // cx.notify();
    }
}
