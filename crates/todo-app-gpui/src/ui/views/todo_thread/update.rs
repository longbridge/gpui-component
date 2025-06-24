use super::*;
use crate::app::AppExt;
use crate::{
    app::AppState,
    models::{
        mcp_config::{McpProviderInfo, McpTool},
        provider_config::{LlmProviderInfo, ModelInfo},
    },
    ui::views::todo_thread_edit::Save,
};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    notification::NotificationType,
    *,
};

impl TodoThreadChat {
    // 获取模型选择显示文本
    pub(crate) fn get_model_display_text(&self, _cx: &App) -> String {
        if let Some(selected_model) = &self.todoitem.selected_model {
            selected_model.model_name.clone()
        } else {
            "".to_string()
        }
    }

    // 获取工具选择显示文本
    pub(crate) fn get_tool_display_text(&self, _cx: &App) -> String {
        let selected_tools = self.todoitem.selected_tools.clone();
        let selected_count = selected_tools.len();

        if selected_count == 0 {
            "".to_string()
        } else if selected_count == 1 {
            selected_tools[0].tool_name.clone()
        } else {
            format!("{} 等{}个工具", selected_tools[0].tool_name, selected_count)
        }
    }

    pub(crate) fn toggle_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_providers = open_indices.to_vec();
        cx.notify();
    }

    pub(crate) fn toggle_tool_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_tool_providers = open_indices.to_vec();
        cx.notify();
    }

    pub(crate) fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        match AppState::state_mut(cx)
            .todo_manager
            .update_todo(self.todoitem.clone())
            .save()
        {
            Ok(_) => {
                // TODO: 处理保存成功的情况
                //_window.push_notification((NotificationType::Success, "Todo保存成功"), cx);
                println!("todo保存成功");
                cx.dispatch_global_action(Box::new(Save));
            }
            Err(err) => {
                // TODO: 处理保存失败的情况
                _window.push_notification(
                    (
                        NotificationType::Error,
                        SharedString::new(format!("Todo保存失败-{}", err)),
                    ),
                    cx,
                );
            }
        }
        cx.notify();
    }

    pub(crate) fn toggle_model_selection(
        &mut self,
        checked: bool,
        model: &ModelInfo,
        provider: &LlmProviderInfo,
        cx: &mut Context<Self>,
    ) {
        if checked {
            // 如果选中，则添加
            self.todoitem.selected_model = Some(crate::models::todo_item::SelectedModel {
                provider_id: provider.id.clone(),
                provider_name: provider.name.clone(),
                model_id: model.id.clone(),
                model_name: model.display_name.clone(),
            });
        } else {
            // 如果取消选中，则移除
            self.todoitem.selected_model = None;
        }
        cx.notify(); // 通知主界面更新
    }

    pub(crate) fn toggle_tool_selection(
        &mut self,
        checked: bool,
        tool: &McpTool,
        provider: &McpProviderInfo,
        cx: &mut Context<Self>,
    ) {
        if checked {
            // 如果选中，则添加
            self.todoitem
                .selected_tools
                .push(crate::models::todo_item::SelectedTool {
                    provider_id: provider.id.clone(),
                    provider_name: provider.name.clone(),
                    description: tool.description.clone().unwrap_or_default().to_string(),
                    tool_name: tool.name.clone().to_string(),
                });
        } else {
            // 如果取消选中，则移除
            if let Some(index) = self
                .todoitem
                .selected_tools
                .iter()
                .position(|t| t.tool_name == tool.name && t.provider_id == provider.id)
            {
                self.todoitem.selected_tools.remove(index);
            }
        }
        cx.notify(); // 通知主界面更新
    }

    // pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
    //     cx.new(|cx| Self::new(window, cx))
    // }

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
