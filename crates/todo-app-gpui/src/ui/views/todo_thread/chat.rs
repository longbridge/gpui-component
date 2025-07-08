use super::*;
use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::backoffice::llm::types::ToolDefinition;
use crate::config::llm_config::LlmProviderManager;
use gpui::*;

impl TodoThreadChat {
    pub(crate) fn send_message(
        &mut self,
        _: &SendMessage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let todo = self.todoitem.clone();
        let message_content = self.chat_input.read(cx).value();

        // 检查消息内容是否为空
        if message_content.is_empty() {
            return;
        }

        // 检查是否选择了模型
        let Some(selected_model) = &self.todoitem.selected_model else {
            println!("未选择模型，无法发送消息");
            return;
        };

        println!("使用模型: {:?}", selected_model);

        // 获取模型提供商信息
        let Some(provider_info) = LlmProviderManager::get_enabled_providers()
            .iter()
            .find(|provider| provider.id == selected_model.provider_id)
            .cloned()
        else {
            println!("未找到模型提供商: {}", selected_model.provider_id);
            return;
        };

        // 检查模型ID是否有效
        if selected_model.model_id.is_empty() {
            println!("模型ID为空，无法发送消息");
            return;
        }

        let message_content = message_content.to_string().trim().to_string();

        // 创建用户消息并添加工具定义
        let user_message =
            ChatMessage::user_text_with_source(message_content.clone(), todo.id.clone())
                .with_tool_definitions(
                    self.todoitem
                        .selected_tools
                        .iter()
                        .map(|tool| ToolDefinition {
                            name: ToolDefinition::format_tool_name(
                                &tool.provider_id,
                                &tool.tool_name,
                            ),
                            description: tool.description.clone(),
                            parameters: tool.args_schema.clone().unwrap_or_default(),
                        })
                        .collect::<Vec<_>>(),
                );

        // 添加用户消息到聊天历史
        self.chat_messages.push(user_message);
        let history_message = self.chat_messages.clone();
        // 清空输入框
        self.chat_input
            .update(cx, |input, cx| input.set_value("", window, cx));

        // 设置加载状态
        self.is_loading = true;
        self.scroll_handle.scroll_to_bottom();

        // 准备助手消息占位符
        let assistant_placeholder =
            ChatMessage::assistant_text_with_source("".to_string(), todo.id.clone()).with_model(
                selected_model.model_id.clone(),
                selected_model.model_name.clone(),
            );

        self.chat_messages.push(assistant_placeholder);

        // 准备异步调用参数
        let provider_id = provider_info.id.clone();
        let model_id = selected_model.model_id.clone();
        let source = self.todoitem.id.clone();

        // 发起异步调用
        cx.spawn(async move |this, cx| {
            println!(
                "开始调用 LLM - Provider: {}, Model: {}",
                provider_id, model_id
            );

            match CrossRuntimeBridge::global()
                .llm_chat(provider_id, model_id, source, history_message)
                .await
            {
                Ok(_) => {
                    println!("LLM 调用成功");
                }
                Err(e) => {
                    println!("LLM 调用失败: {:?}", e);
                    // // 可以考虑在这里更新UI显示错误信息
                    // cx.update(|cx| {
                    //     // this.is_loading = false;
                    //     // 移除占位符消息或显示错误消息
                    //     if let Some(last_message) = this.chat_messages.last_mut() {
                    //         if last_message.get_text().is_empty() {
                    //             this.chat_messages.pop();
                    //         }
                    //     }
                    //     cx.notify();
                    // })
                    // .ok();
                }
            }
        })
        .detach();

        cx.notify();
    }
}
