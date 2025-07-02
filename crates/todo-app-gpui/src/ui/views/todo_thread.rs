mod chat;
mod view;
use crate::config::todo_item::*;
use crate::{app::AppExt, xbus};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    scroll::ScrollbarState,
    *,
};
use rig::message::AssistantContent;
use std::time::Duration;

// 从 rmcp 导入 MCP 类型
use rmcp::model::Tool as McpTool;

actions!(todo_thread, [Tab, TabPrev, SendMessage]);

const CONTEXT: &str = "TodoThread";

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
    scroll_state: ScrollbarState,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    // 新增：缓存从 McpRegistry 获取的工具数据
    cached_server_tools: std::collections::HashMap<String, Vec<McpTool>>,

    _subscriptions: Vec<Subscription>,
    todoitem: Todo,
}

const WIDTH: Pixels = px(500.0);
const HEIGHT: Pixels = px(650.0);
const SIZE: gpui::Size<Pixels> = size(WIDTH, HEIGHT);

impl TodoThreadChat {
    pub fn open(todo: Todo, cx: &mut App) -> WindowHandle<Root> {
        cx.activate(true);
        let window_bounds = Bounds::centered(None, SIZE, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(SIZE),
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
            move |window, cx| cx.new(|cx| Self::new(todo, window, cx)),
        )
    }

    fn new(todoitem: Todo, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 聊天输入框 - 多行支持
        let chat_input = cx.new(|cx| {
            let placeholder = if cfg!(target_os = "macos") {
                "输入消息与AI助手对话...，按Cmd+Enter发送，按ESC清除输入框"
            } else {
                "输入消息与AI助手对话...，按Ctrl+Enter发送，按ESC清除输入框"
            };
            InputState::new(window, cx)
                .placeholder(placeholder)
                .clean_on_escape()
                .multi_line()
                .auto_grow(1, 6)
        });

        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
        let _sub = xbus::subscribe(move |msg: &rig::message::Message| {
            tx.try_send(msg.clone()).unwrap_or_else(|e| {
                tracing::error!("Failed to send message to channel: {}", e);
            });
        });
        cx.spawn(async move |this, app| {
            //println!("开始接收AI助手响应");
            let _sub = _sub;
            'a: loop {
                Timer::after(Duration::from_millis(10)).await;
                let mut buffer = String::new();
                loop {
                    match rx.try_recv() {
                        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                            break;
                        }
                        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                            break 'a;
                        }
                        Ok(msg) => match msg {
                            rig::message::Message::Assistant { content } => match content.first() {
                                AssistantContent::Text(text) => {
                                    buffer.push_str(&text.text);
                                }
                                AssistantContent::ToolCall(tool_call) => {}
                            },
                            rig::message::Message::User { content } => {}
                        },
                    }
                }
                if buffer.is_empty() {
                    continue;
                }
                let entity = this.clone();
                entity
                    .update(app, |this, cx| {
                        if let Some(last_message) = this.chat_messages.last_mut() {
                            last_message.content.push_str(&buffer);
                        }
                        this.is_loading = false;
                        this.scroll_handle.scroll_to_bottom();
                        cx.notify();
                    })
                    .ok();
            }
        })
        .detach();

        let _subscriptions = vec![cx.subscribe_in(&chat_input, window, Self::on_chat_input_event)];
        // 初始化欢迎消息
        let chat_messages = vec![ChatMessage {
            id: "user_prompt".to_string(),
            role: MessageRole::System,
            content: todoitem.description.clone(),
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
            cached_server_tools: std::collections::HashMap::new(), // 新增初始化
            _subscriptions,
            scroll_state: ScrollbarState::default(),
            scroll_size: gpui::Size::default(),
            todoitem,
        }
    }

    // 新增：获取缓存的工具数据
    fn get_server_tools(&self, server_id: &str) -> Vec<McpTool> {
        self.cached_server_tools
            .get(server_id)
            .cloned()
            .unwrap_or_default()
    }

    // 新增：获取模型选择显示文本
    fn get_model_display_text(&self, _cx: &App) -> String {
        if let Some(selected_model) = &self.todoitem.selected_model {
            selected_model.model_name.clone()
        } else {
            "选择模型".to_string()
        }
    }

    // 新增：获取工具选择显示文本
    fn get_tool_display_text(&self, _cx: &App) -> String {
        let selected_count = self.todoitem.selected_tools.len();

        if selected_count == 0 {
            "选择工具".to_string()
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
    fn toggle_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_providers = open_indices.to_vec();
        cx.notify();
    }

    fn toggle_tool_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_tool_providers = open_indices.to_vec();
        cx.notify();
    }

    // 新增：切换模型选择
    fn toggle_model_selection(
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
    fn toggle_tool_selection(
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
                });
        } else {
            self.todoitem
                .selected_tools
                .retain(|t| t.tool_name != tool.name || t.provider_id != server.id);
        }
        cx.notify();
    }

    // 新增：保存方法（用于在选择模型/工具后保存状态）
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 这里可以保存 todoitem 的状态
        // 根据需要实现具体的保存逻辑
        match crate::config::todo_item::TodoManager::update_todo(self.todoitem.clone()) {
            Ok(_) => {
                // 保存成功，可以显示通知
                log::info!("Todo item saved successfully");
            }
            Err(err) => {
                // 保存失败，显示错误通知
                log::error!("Failed to save todo item: {}", err);
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
