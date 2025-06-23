mod chat;
mod update;
mod view;
use crate::models::todo_item::*;
use crate::{app::AppExt, xbus};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{input::InputState, scroll::ScrollbarState, *};
use rig::message::AssistantContent;
use std::time::Duration;
use std::{cell::Cell, rc::Rc};

actions!(todo_thread, [Tab, TabPrev, SendMessage]);

#[derive(Debug, Clone)]
pub enum TodoEvent {
    TodoChatClosed(String),
    TodoEditClosed(String),
}

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
    scroll_state: Rc<Cell<ScrollbarState>>,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    _subscriptions: Vec<Subscription>,
    todoitem: Todo,
}

impl EventEmitter<TodoEvent> for TodoThreadChat {}

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
            loop {
                Timer::after(Duration::from_millis(5)).await;
                match rx.try_recv() {
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        continue;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        break;
                    }
                    Ok(msg) => {
                        let entity = this.clone();
                        match msg {
                            rig::message::Message::Assistant { content } => match content.first() {
                                AssistantContent::Text(text) => {
                                    entity
                                        .update(app, |this, cx| {
                                            if let Some(last_message) =
                                                this.chat_messages.last_mut()
                                            {
                                                last_message.content.push_str(&text.text);
                                            }
                                            this.is_loading = false;
                                            this.scroll_handle.scroll_to_bottom();
                                            cx.notify();
                                        })
                                        .ok();
                                }
                                AssistantContent::ToolCall(tool_call) => {}
                            },
                            rig::message::Message::User { content } => {}
                        }
                    }
                }
            }
            // println!("AI助手响应完成");
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
            _subscriptions,
            scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_size: gpui::Size::default(),
            todoitem,
        }
    }
}
