use std::{cell::Cell, rc::Rc};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion, button::{Button, ButtonVariant, ButtonVariants as _}, checkbox::Checkbox, h_flex, input::{InputEvent, InputState, TextInput}, label::Label, scroll::{ Scrollbar, ScrollbarState}, tooltip::Tooltip, Size, *
};
use crate::{app::AppState, models::{mcp_config::{McpProviderInfo, McpTool}, provider_config::{LlmProviderInfo, ModelInfo}}, ui::components::ViewKit};
use crate::models::todo_item::*;
use crate::ui::AppExt;

actions!(todo_thread, [Tab, TabPrev, SendMessage]);

const CONTEXT: &str = "TodoThread";

// 聊天消息结构
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model: Option<String>,
    pub tools_used: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    fn display_name(&self) -> &'static str {
        match self {
            MessageRole::User => "你",
            MessageRole::Assistant => "AI助手",
            MessageRole::System => "系统",
        }
    }

    fn color(&self) -> gpui::Rgba {
        match self {
            MessageRole::User => gpui::rgb(0x3B82F6),
            MessageRole::Assistant => gpui::rgb(0x10B981),
            MessageRole::System => gpui::rgb(0x6B7280),
        }
    }
}

pub struct TodoThreadChat {
    focus_handle: FocusHandle,

    // 聊天功能
    chat_messages: Vec<ChatMessage>,
    chat_input: Entity<InputState>,
    is_loading: bool,
    scroll_handle: ScrollHandle,
    scroll_size: gpui::Size<Pixels>,
    scroll_state: Rc<Cell<ScrollbarState>>,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    _subscriptions: Vec<Subscription>,
    todoitem:Todo,
}

impl TodoThreadChat {
    pub fn open(todo:Todo,
        cx: &mut App) {
            cx.activate(true);
            let window_size = size(px(400.0), px(600.0));
            let window_bounds = Bounds::centered(None, window_size, cx);
            let options = WindowOptions {
                app_id: Some("x-todo-app".to_string()),
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(400.),
                    height: px(600.),
                }),
                kind: WindowKind::Normal,
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                ..Default::default()
            };
            
            cx.create_normal_window(
                format!("xTo-Do {}", todo.title),
                options,
                move |window, cx| cx.new(|cx| Self::new(todo,window, cx)),
            );
        }

     fn new(todoitem:Todo,window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 聊天输入框 - 多行支持
        let chat_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入消息与AI助手对话...，按Ctrl+Enter发送，按ESC清除输入框")
                .clean_on_escape()
                .multi_line()
                .auto_grow(2, 6)
        });

        let _subscriptions = vec![cx.subscribe_in(&chat_input, window, Self::on_chat_input_event)];

        // 初始化欢迎消息
        let chat_messages = vec![ChatMessage {
            id: "1".to_string(),
            role: MessageRole::System,
            content: todoitem.description
                .clone(),
            timestamp: chrono::Utc::now(),
            model: None,
            tools_used: vec![],
        }];

        Self {
            focus_handle: cx.focus_handle(),
            chat_messages,
            chat_input,
            is_loading: false,
            scroll_handle: ScrollHandle::new(),
            expanded_providers: Vec::new(),
            expanded_tool_providers: Vec::new(),
            _subscriptions,
            scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_size: gpui::Size::default(),
            todoitem
        }
    }

    // 获取模型选择显示文本
    fn get_model_display_text(&self, _cx: &App) -> String {
        let selected_models = self.todoitem.selected_models.clone();
        let selected_count = selected_models.len();

        if selected_count == 0 {
            "".to_string()
        } else if selected_count == 1 {
            selected_models[0].model_name.clone()
        } else {
            format!("{} 等{}个模型", selected_models[0].model_name, selected_count)
        }
    }

    // 获取工具选择显示文本
    fn get_tool_display_text(&self, _cx: &App) -> String {
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

    fn toggle_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_providers = open_indices.to_vec();
        cx.notify();
    }

    fn toggle_tool_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_tool_providers = open_indices.to_vec();
        cx.notify();
    }

    fn toggle_model_selection(&mut self,checked:bool, model:&ModelInfo,provider:&LlmProviderInfo, cx: &mut Context<Self>) {
        if checked {
            // 如果选中，则添加
            if let Some((id,provider)) = AppState::state(cx).llm_provider.providers.iter().find(|(id,p)| p.models.iter().any(|t| t.id == model.id)) {
                if let Some(model) = provider.models.iter().find(|t| t.id == model.id) {
                    self.todoitem.selected_models.push(crate::models::todo_item::SelectedModel {
                        provider_id: provider.id.clone(),
                        provider_name: provider.name.clone(),
                        model_id: model.id.clone(),
                        model_name: model.display_name.clone(),
                    });
                }
            }
        } else {
            // 如果取消选中，则移除
            if let Some(index) = self.todoitem.selected_models.iter().position(|t| t.model_id == model.id && t.provider_id == provider.id) {
                self.todoitem.selected_models.remove(index);
            }
        }
        // // 检查工具是否已被选中
        // if let Some(index) = self.todoitem.selected_models.iter().position(|t| t.model_id == model.id) {
        //     // 如果已选中，则移除
        //     self.todoitem.selected_models.remove(index);
        // } else {
        //     // 如果未选中，则添加
        //     if let Some((id,provider)) = AppState::state(cx).llm_provider.providers.iter().find(|(id,p)| p.models.iter().any(|t| t.id == model.id)) {
        //         if let Some(model) = provider.models.iter().find(|t| t.id == model.id) {
        //             self.todoitem.selected_models.push(crate::models::todo_item::SelectedModel {
        //                 provider_id: provider.id.clone(),
        //                 provider_name: provider.name.clone(),
        //                 model_id: model.id.clone(),
        //                 model_name: model.display_name.clone(),
        //             });
        //         }
        //     }
        // }
          cx.notify(); // 通知主界面更新
    }

    fn toggle_tool_selection(&mut self,checked:bool, tool:&McpTool,provider:&McpProviderInfo, cx: &mut Context<Self>) {
        if checked {
            // 如果选中，则添加
            if let Some((id,provider)) = AppState::state(cx).mcp_provider.providers.iter().find(|(id,p)| p.tools.iter().any(|t| t.name == tool.name)) {
                if let Some(tool) = provider.tools.iter().find(|t| t.name == tool.name) {
                    self.todoitem.selected_tools.push(crate::models::todo_item::SelectedTool {
                        provider_id: provider.id.clone(),
                        provider_name: provider.name.clone(),
                        description: tool.description.clone(),
                        tool_name: tool.name.clone(),
                    });
                }
            }
        } else {
            // 如果取消选中，则移除
            if let Some(index) = self.todoitem.selected_tools.iter().position(|t| t.tool_name == tool.name && t.provider_id == provider.id) {
                self.todoitem.selected_tools.remove(index);
            }
        }
        // // 检查工具是否已被选中
        // if let Some(index) = self.todoitem.selected_tools.iter().position(|t| t.tool_name == tool.name) {
        //     // 如果已选中，则移除
        //     self.todoitem.selected_tools.remove(index);
        // } else {
        //     // 如果未选中，则添加
        //     if let Some((id,provider)) = AppState::state(cx).mcp_provider.providers.iter().find(|(id,p)| p.tools.iter().any(|t| t.name == tool.name)) {
        //         if let Some(tool) = provider.tools.iter().find(|t| t.name == tool.name) {
        //             self.todoitem.selected_tools.push(crate::models::todo_item::SelectedTool {
        //                 provider_id: provider.id.clone(),
        //                 provider_name: provider.name.clone(),
        //                 description: tool.description.clone(),
        //                 tool_name: tool.name.clone(),
        //             });
        //         }
        //     }
        // }
          cx.notify(); // 通知主界面更新
    }

    fn open_model_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let todo_edit_entity = cx.entity().clone();

        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
           // let providers = todo_edit_entity.read(drawer_cx).model_manager.providers.clone();
            let providers = AppState::state(drawer_cx).llm_provider.providers.clone();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_providers.clone();
            let todoitem = todo_edit_entity.read(drawer_cx).todoitem.clone();
            let mut accordion = Accordion::new("chat-model-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, todo_cx| {
                            todo_edit.toggle_accordion(open_indices, todo_cx);
                        });
                    }
                });

            for (provider_index, (id,provider)) in providers.iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_models = provider.models.clone();
                
                //let has_selected_models = provider_models.iter().any(|model| model.is_selected);
                let has_selected_models = provider_models.iter().any(|model| {
                    todoitem.selected_models.iter().any(|selected| selected.model_id == model.id && selected.provider_id == provider.id)
                });
                let is_expanded = has_selected_models || expanded_providers.contains(&provider_index);

                accordion = accordion.item(|item| {
                    item.open(is_expanded)
                        .icon(IconName::Bot)
                        .title(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .font_medium()
                                                .text_color(gpui::rgb(0x374151))
                                                .child(provider_name.clone()),
                                        )
                                        .when(has_selected_models, |this| {
                                            this.child(
                                                Icon::new(IconName::Check)
                                                    .xsmall()
                                                    .text_color(gpui::rgb(0x10B981)),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(if has_selected_models {
                                            gpui::rgb(0xDCFCE7)
                                        } else {
                                            gpui::rgb(0xEFF6FF)
                                        })
                                        .text_color(if has_selected_models {
                                            gpui::rgb(0x166534)
                                        } else {
                                            gpui::rgb(0x1D4ED8)
                                        })
                                        .rounded_md()
                                        .text_xs()
                                        .child(format!("{} 个模型", provider_models.len())),
                                ),
                        )
                        .content(
                            v_flex()
                                .gap_2()
                                .p_2()
                                .children(provider_models.iter().enumerate().map(
                                    |(model_index, model)| {
                                        let model_name_for_event = model.display_name.clone();
                                        let checkbox_id = SharedString::new(format!(
                                            "chat-model-{}-{}",
                                            provider_index, model_index
                                        ));
                                        let todo_edit_entity_for_event = todo_edit_entity.clone();

                                        div()
                                            .p_1()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .rounded_md()
                                            // .border_1()
                                            // .border_color(gpui::rgb(0xE5E7EB))
                                            .hover(|style| style.bg(gpui::rgb(0xF3F4F6)))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .gap_3()
                                                            .child(
                                                                Checkbox::new(checkbox_id)
                                                                    .checked(todoitem.selected_models.iter().any(|selected| 
                                                                            selected.model_id == model.id && selected.provider_id == provider.id
                                                                        ))
                                                                    .label(model.display_name.clone())
                                                                    .on_click({
                                                                        let model_clone = model.clone();
                                                                                let provider_clone = provider.clone();
                                                                        move |checked, _window, cx| {
                                                                            let model_name_to_toggle =
                                                                                model_name_for_event.clone();
                                                                            // 更新原始数据
                                                                            todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                todo_edit.toggle_model_selection(*checked,&model_clone, &provider_clone, todo_cx);
                                                                            });
                                                                            println!("切换模型选择: {}",model_name_to_toggle);
                                                                        }
                                                                    }
                                                                ),
                                                            )
                                                            .child(
                                                                h_flex().gap_1().items_center().children(
                                                                    model.capabilities.iter().enumerate().map(
                                                                        |(cap_index, cap)| {
                                                                            let capability_unique_id = provider_index * 10000
                                                                                + model_index * 1000
                                                                                + cap_index;

                                                                            div()
                                                                                .id(("chat_capability", capability_unique_id))
                                                                                .p_1()
                                                                                .rounded_md()
                                                                                .bg(gpui::rgb(0xF3F4F6))
                                                                                .child(
                                                                                    Icon::new(cap.icon())
                                                                                        .xsmall()
                                                                                        .text_color(gpui::rgb(0x6B7280)),
                                                                                )
                                                                        },
                                                                    ),
                                                                ),
                                                            ),
                                                    ),
                                            )
                                    },
                                ))
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(280.))
                .title("选择模型")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-chat-models")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        todo_edit.todoitem.selected_models.clear();
                                        todo_cx.notify();
                                    });
                                    // println!("清空所有模型选择");
                                    // window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }

    fn open_tool_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let todo_edit_entity = cx.entity().clone();
        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            let providers = AppState::state(drawer_cx).mcp_provider.providers.clone();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_tool_providers.clone();
            let todoitem = todo_edit_entity.read(drawer_cx).todoitem.clone();
            let mut accordion = Accordion::new("chat-tool-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, todo_cx| {
                            todo_edit.toggle_tool_accordion(open_indices, todo_cx);
                        });
                    }
                });

            for (provider_index, (id,provider)) in providers.iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_tools = provider.tools.clone();
                
                let has_selected_tools = provider_tools.iter().any(|tool|  todoitem.selected_tools.iter().any(|selected| selected.tool_name == tool.name && selected.provider_id == provider.id));
                let is_expanded = has_selected_tools || expanded_providers.contains(&provider_index);

                accordion = accordion.item(|item| {
                    item.open(is_expanded)
                        .icon(IconName::Wrench)
                        .title(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .font_medium()
                                                .text_color(gpui::rgb(0x374151))
                                                .child(provider_name.clone()),
                                        )
                                        .when(has_selected_tools, |this| {
                                            this.child(
                                                Icon::new(IconName::Check)
                                                    .xsmall()
                                                    .text_color(gpui::rgb(0x10B981)),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(if has_selected_tools {
                                            gpui::rgb(0xDCFCE7)
                                        } else {
                                            gpui::rgb(0xFFF7ED)
                                        })
                                        .text_color(if has_selected_tools {
                                            gpui::rgb(0x166534)
                                        } else {
                                            gpui::rgb(0xEA580C)
                                        })
                                        .rounded_md()
                                        .text_xs()
                                        .child(format!("{} 个工具", provider_tools.len())),
                                ),
                        )
                        .content(
                            v_flex()
                                .gap_2()
                                .p_2()
                                .children(provider_tools.iter().enumerate().map(
                                    |(tool_index, tool)| {
                                        let tool_name_for_event = tool.name.clone();
                                        let checkbox_id = SharedString::new(format!(
                                            "chat-tool-{}-{}",
                                            provider_index, tool_index
                                        ));
                                        let todo_edit_entity_for_event = todo_edit_entity.clone();

                                        div()
                                            .p_1()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .rounded_md()
                                            .hover(|style| style.bg(gpui::rgb(0xF3F4F6)))
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .child(
                                                                h_flex()
                                                                    .items_center()
                                                                    .gap_3()
                                                                    .child(
                                                                        Checkbox::new(checkbox_id)
                                                                            .checked(todoitem.selected_tools.iter().any(|selected| 
                                                                            selected.tool_name == tool.name && selected.provider_id == provider.id
                                                                        ))
                                                                            .label(tool.name.clone())
                                                                            .on_click({
                                                                                let tool_clone = tool.clone();
                                                                                let provider_clone = provider.clone();
                                                                                move |checked, _window, cx| {
                                                                                    let tool_name_to_toggle =
                                                                                        tool_name_for_event.clone();
                                                                                    
                                                                                    // 更新原始数据
                                                                                    todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                        todo_edit.toggle_tool_selection(*checked,&tool_clone, &provider_clone, todo_cx);
                                                                                    });
                                                                                    println!(
                                                                                        "切换工具选择: {}",
                                                                                        tool_name_to_toggle
                                                                                    );
                                                                                }
                                                                            }
                                                                            ),
                                                                    )
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .pl_6()
                                                            .text_xs()
                                                            .text_color(gpui::rgb(0x6B7280))
                                                            .child(tool.description.clone()),
                                                    ),
                                            )
                                    },
                                ))
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(280.))
                .title("选择工具集")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-chat-tools")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        todo_edit.todoitem.selected_tools.clear();
                                        todo_cx.notify();
                                    });
                                    // println!("清空所有工具选择");
                                    // window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }

    // pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
    //     cx.new(|cx| Self::new(window, cx))
    // }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn send_message(&mut self, _: &SendMessage, window: &mut Window, cx: &mut Context<Self>) {
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
        self.simulate_ai_response(message_content, cx);
        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }

    fn simulate_ai_response(&mut self, user_message: String, cx: &mut Context<Self>) {
        // 获取当前选择的模型和工具
        let selected_models = self.todoitem.selected_models.clone();
        let selected_tools = self.todoitem.selected_tools.clone();

        let selected_model = selected_models.first().cloned();

        // 模拟AI响应内容
        let response_content = match user_message.to_lowercase().as_str() {
            msg if msg.contains("任务") => {
                "我可以帮您创建、管理和跟踪任务。请告诉我任务的具体要求，我会为您提供专业的建议和解决方案。"
            }
            msg if msg.contains("时间") || msg.contains("日期") => {
                "我可以帮您规划时间和设置提醒。请告诉我您的具体需求，我会为您制定合理的时间安排。"
            }
            msg if msg.contains("优先级") => {
                "我会根据任务的重要性和紧急程度帮您设置优先级。这个任务对您来说有多重要？有具体的截止时间吗？"
            }
            msg if msg.contains("帮助") || msg.contains("功能") => {
                "我是您的AI助手，可以帮助您：\n• 创建和管理任务\n• 设置提醒和截止时间\n• 分析任务优先级\n• 提供工作建议\n• 回答各种问题\n\n有什么具体需要帮助的吗？"
            }
            _ => &format!(
                "我理解您的问题：\"{}\"。我正在使用{}模型为您提供帮助。请告诉我更多详细信息，我会给出更精准的建议。",
                user_message,
                selected_model.clone().map_or("默认".to_string(), |model|model.model_name)
            ),
        };

        let ai_message = ChatMessage {
            id: format!("ai_{}", chrono::Utc::now().timestamp()),
            role: MessageRole::Assistant,
            content: response_content.to_string(),
            timestamp: chrono::Utc::now(),
            model: selected_model.map_or(Some("默认".to_string()), |model|Some(model.model_name)),
            tools_used: selected_tools.iter().map(|tool| tool.tool_name.clone()).collect(),
        };

        self.chat_messages.push(ai_message);
        self.is_loading = false;

        cx.notify();
    }

    fn on_chat_input_event(
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

    fn render_chat_message(&self, message: &ChatMessage) -> impl IntoElement {
        let is_user = matches!(message.role, MessageRole::User);

        h_flex()
            .w_full()
            .py_2()
            .px_3()
            .when(is_user, |this| this.justify_end())
            .when(!is_user, |this| this.justify_start())
            .child(
                div().max_w_full().flex_wrap().child(
                    v_flex()
                        .gap_1()
                        .child(
                            // 消息头部：角色和时间
                            h_flex()
                                .items_center()
                                .gap_2()
                                .when(is_user, |this| this.justify_end())
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(message.role.color())
                                        .font_medium()
                                        .child(message.role.display_name()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(gpui::rgb(0x9CA3AF))
                                        .child(message.timestamp.format("%H:%M").to_string()),
                                )
                                .when_some(message.model.as_ref(), |this, model| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(gpui::rgb(0x6B7280))
                                            .child(format!("({})", model)),
                                    )
                                }),
                        )
                        .child(
                            // 消息内容
                            div()
                                .p_3()
                                .rounded_lg()
                                .text_sm()
                                .when(is_user, |this| {
                                    this.bg(gpui::rgb(0x3B82F6)).text_color(gpui::rgb(0xFFFFFF))
                                })
                                .when(!is_user, |this| {
                                    this.bg(gpui::rgb(0xF3F4F6)).text_color(gpui::rgb(0x374151))
                                })
                                .child(message.content.clone()),
                        )
                        .when(!message.tools_used.is_empty(), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child(format!("使用工具: {}", message.tools_used.join(", "))),
                            )
                        }),
                ),
            )
    }
}

impl FocusableCycle for TodoThreadChat {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![self.chat_input.focus_handle(cx)]
    }
}

impl Focusable for TodoThreadChat {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoThreadChat {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_model=self.get_model_display_text(cx);
        let selected_tool=self.get_tool_display_text(cx);
        let has_tools = !selected_tool.is_empty();
        let has_models = !selected_model.is_empty();
        v_flex()
            .key_context(CONTEXT)
            .id("todo-thread-view")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::send_message))
            .size_full()
            .p_2().gap_2()
            .child(
                div().size_full().min_h_32().child(
                    div().relative().size_full().child(
                        v_flex()
                            // .border_1()
                            // .border_color(gpui::rgb(0xE5E7EB))
                            .relative()
                            .size_full()
                            .child(
                                v_flex()
                                    .id("id-todo-thread-chat")
                                    .p_1()
                                    .gap_1()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.scroll_handle)
                                    .children(
                                        self.chat_messages
                                            .iter()
                                            .map(|msg| self.render_chat_message(msg)),
                                    )
                                    .when(self.is_loading, |this| {
                                        this.child(
                                            h_flex().justify_start().py_2().child(
                                                div()
                                                    .p_3()
                                                    .bg(gpui::rgb(0xF3F4F6))
                                                    .rounded_lg()
                                                    .text_color(gpui::rgb(0x6B7280))
                                                    .child("AI正在思考中..."),
                                            ),
                                        )
                                    }),
                            )
                            ,
                    ).child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .right_0()
                                    .bottom_0()
                                    .child(Scrollbar::vertical(
                                        cx.entity().entity_id(),
                                        self.scroll_state.clone(),
                                        self.scroll_handle.clone(),
                                        self.scroll_size,
                                    )),
                            ),
                ),
            )
            .child(
                // 聊天输入区域 - 固定在底部
                v_flex().p_1().gap_0()
                .border_1()
                    .border_1()
                    .rounded_lg()
                    .border_color(gpui::rgb(0xE5E7EB)).when(has_models||has_tools, |this|{
                        this.child(
                            h_flex().items_center().gap_2().bg(gpui::rgb(0xF9FAFB)).child(
                                div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child(selected_model),
                            ).child( div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child(selected_tool),),
                        )
                    })
            .child(
                h_flex()
                    .items_center()
                    .justify_start()
                    .gap_1()
                    // .p_1()
                    .bg(gpui::rgb(0xF9FAFB))
                    .child(
                        h_flex().justify_start().items_center().gap_2().child(
                            Button::new("show-chat-model-drawer")
                            .icon(Icon::new(IconName::Database)
                                    .xsmall().when(has_models, |this|this.text_color(green_500()))
                                    )
                                .ghost()
                                .small()
                                .justify_center()
                                .tooltip("选择模型")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_model_drawer_at(Placement::Left, window, cx)
                                })),
                        ),
                    )
                    .child(
                        h_flex().justify_start().items_center().gap_2().child(
                            Button::new("show-chat-tool-drawer")
                            .icon(Icon::new(IconName::Wrench)
                                    .xsmall().when(has_tools, |this|this.text_color(green_500()))
                                    )
                                .ghost()
                                .small()
                                .justify_center()
                                .tooltip("选择工具")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_tool_drawer_at(Placement::Left, window, cx)
                                })),
                        ),
                    ),
            ).child( h_flex()
                    .gap_1()
                    // .p_1()
                    .child(
                        // 多行输入框
                        div().w_full().text_sm().child(TextInput::new(&self.chat_input).bordered(false)),
                    )
                    .child(
                        h_flex().justify_end().child(
                            Button::new("send-message")
                                .with_variant(ButtonVariant::Primary)
                                .icon(IconName::Send)
                                // .label("发送")
                                .disabled(self.is_loading)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.dispatch_action(Box::new(SendMessage), cx);
                                })),
                        ),
                    ),)
            )
    }
}
