use crate::app::{AppExt, FoEvent};
use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::backoffice::llm::types::{ChatMessage, MessageContent, ToolDefinition};
use crate::config::llm_config::LlmProviderManager;

use super::TodoThreadChat;
use super::{SendMessage, Tab};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    *,
};
// 从 rmcp 导入 MCP 类型
use rmcp::model::Tool as McpTool;

impl EventEmitter<FoEvent> for TodoThreadChat {}

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
            tracing::trace!("未选择模型，无法发送消息");
            return;
        };

        tracing::trace!("使用模型: {:?}", selected_model);

        // 获取模型提供商信息
        let Some(provider_info) = LlmProviderManager::get_enabled_providers()
            .iter()
            .find(|provider| provider.id == selected_model.provider_id)
            .cloned()
        else {
            tracing::trace!("未找到模型提供商: {}", selected_model.provider_id);
            return;
        };

        // 检查模型ID是否有效
        if selected_model.model_id.is_empty() {
            tracing::trace!("模型ID为空，无法发送消息");
            return;
        }
        let mut history_message = self.chat_messages.clone();
        if !self.todoitem.selected_tools.is_empty() {
            history_message.push(
                ChatMessage::system().with_content(MessageContent::ToolDefinitions(
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
                )),
            );
        }
        let message_content = message_content.to_string().trim().to_string();

        // 创建用户消息
        let user_message = ChatMessage::user()
            .with_text(message_content.clone())
            .with_source(todo.id.clone())
            .with_metadata(
                selected_model.model_id.clone(),
                selected_model.model_name.clone(),
            );
        // 添加用户消息到聊天历史
        self.chat_messages.push(user_message);

        // 准备助手消息占位符
        self.chat_messages
            .push(ChatMessage::assistant().with_source(todo.id.clone()));
        // 清空输入框
        self.chat_input
            .update(cx, |input, cx| input.set_value("", window, cx));

        // 设置加载状态
        self.is_loading = true;
        self.scroll_handle.scroll_to_bottom();

        // 准备异步调用参数
        let provider_id = provider_info.id.clone();
        let model_id = selected_model.model_id.clone();
        let source = self.todoitem.id.clone();
        // 发起异步调用
        cx.spawn(async move |this, cx| {
            tracing::trace!(
                "开始调用 LLM - Provider: {}, Model: {}",
                provider_id,
                model_id
            );
            match CrossRuntimeBridge::global()
                .llm_chat(
                    provider_id,
                    model_id,
                    source,
                    message_content,
                    history_message,
                )
                .await
            {
                Ok(_) => {
                    tracing::trace!("LLM 调用成功");
                }
                Err(e) => {
                    tracing::error!("LLM 调用失败: {:?}", e);
                    this.update(cx, |this, cx| {
                        // 移除占位符消息或显示错误消息
                        if let Some(last_message) = this.chat_messages.last_mut() {
                            if last_message.get_text().is_empty() {
                                this.chat_messages.pop();
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();

        cx.notify();
    }

    // 新增：获取缓存的工具数据
    pub(crate) fn get_server_tools(&self, server_id: &str) -> Vec<McpTool> {
        self.cached_server_tools
            .get(server_id)
            .cloned()
            .unwrap_or_default()
    }

    // 新增：获取模型选择显示文本
    pub(crate) fn get_model_display_text(&self, _cx: &App) -> String {
        if let Some(selected_model) = &self.todoitem.selected_model {
            selected_model.model_name.clone()
        } else {
            "".to_string()
        }
    }

    // 新增：获取工具选择显示文本
    pub(crate) fn get_tool_display_text(&self, _cx: &App) -> String {
        let selected_count = self.todoitem.selected_tools.len();

        if selected_count == 0 {
            "".to_string()
        } else if selected_count <= 2 {
            self.todoitem
                .selected_tools
                .iter()
                .map(|item| item.tool_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            let first_two = self
                .todoitem
                .selected_tools
                .iter()
                .take(2)
                .map(|item| item.tool_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} 等{}个工具", first_two, selected_count)
        }
    }

    // 新增：切换手风琴状态
    pub(crate) fn toggle_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_providers = open_indices.to_vec();
        cx.notify();
    }

    pub(crate) fn toggle_tool_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_tool_providers = open_indices.to_vec();
        cx.notify();
    }

    // 新增：切换模型选择
    pub(crate) fn toggle_model_selection(
        &mut self,
        checked: bool,
        model: &crate::config::llm_config::ModelInfo,
        provider: &crate::config::llm_config::LlmProviderConfig,
        cx: &mut Context<Self>,
    ) {
        if checked {
            self.todoitem.selected_model = Some(crate::config::todo_item::SelectedModel {
                provider_id: provider.id.clone(),
                provider_name: provider.name.clone(),
                model_id: model.id.clone(),
                model_name: model.display_name.clone(),
            });
        } else {
            self.todoitem.selected_model = None;
        }
        cx.notify();
    }

    // 新增：切换工具选择
    pub(crate) fn toggle_tool_selection(
        &mut self,
        checked: bool,
        tool: &McpTool,
        server: &crate::config::mcp_config::McpServerConfig,
        cx: &mut Context<Self>,
    ) {
        if checked {
            self.todoitem
                .selected_tools
                .push(crate::config::todo_item::SelectedTool {
                    provider_id: server.id.clone(),
                    provider_name: server.name.clone(),
                    description: tool
                        .description
                        .as_ref()
                        .map(|desc| desc.to_string())
                        .unwrap_or_default(),
                    tool_name: tool.name.to_string(),
                    args_schema: Some(
                        serde_json::Value::Object(tool.input_schema.as_ref().clone()).to_string(),
                    ),
                });
        } else {
            self.todoitem
                .selected_tools
                .retain(|t| t.tool_name != tool.name || t.provider_id != server.id);
        }
        cx.notify();
    }

    // 新增：保存方法（用于在选择模型/工具后保存状态）
    pub(crate) fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 这里可以保存 todoitem 的状态
        // 根据需要实现具体的保存逻辑
        match crate::config::todo_item::TodoManager::update_todo(self.todoitem.clone()) {
            Ok(_) => {
                // 保存成功，可以显示通知
                tracing::info!("Todo item saved successfully");
            }
            Err(err) => {
                // 保存失败，显示错误通知
                tracing::error!("Failed to save todo item: {}", err);
                window.push_notification(
                    (
                        gpui_component::notification::NotificationType::Error,
                        SharedString::new(format!("保存失败: {}", err)),
                    ),
                    cx,
                );
            }
        }
        cx.notify();
    }

    pub(crate) fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    pub(crate) fn on_chat_input_event(
        &mut self,
        _entity: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { secondary, .. } if *secondary => {
                window.dispatch_action(Box::new(SendMessage), cx);
            }
            InputEvent::PressEnter { .. } => {
                // 普通Enter只是换行，不做任何处理
            }
            _ => {}
        }
    }
}
