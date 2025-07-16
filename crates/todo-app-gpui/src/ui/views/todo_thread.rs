mod action;
mod view;
use crate::app::AppExt;
use crate::app::FoEvent;
use crate::backoffice::cross_runtime::StreamMessage;
use crate::backoffice::llm::types::MessageContent;
use crate::backoffice::llm::types::{ChatMessage, MessageRole};
use crate::config::todo_item::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{input::InputState, scroll::ScrollbarState, *};
use std::time::Duration;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{Receiver, Sender};

// 从 rmcp 导入 MCP 类型
use rmcp::model::Tool as McpTool;

actions!(todo_thread, [Tab, TabPrev, CloseWindow, SendMessage]);

const CONTEXT: &str = "TodoThread";

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
    is_running: bool,
    scroll_handle: ScrollHandle,
    scroll_size: gpui::Size<Pixels>,
    scroll_state: ScrollbarState,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    // 新增：缓存从 McpRegistry 获取的工具数据
    cached_server_tools: std::collections::HashMap<String, Vec<McpTool>>,

    _subscriptions: Vec<Subscription>,
    extend_channel: Sender<StreamMessage>,
    todoitem: Todo,
}

const WIDTH: Pixels = px(700.0);
const HEIGHT: Pixels = px(650.0);
const SIZE: gpui::Size<Pixels> = size(WIDTH, HEIGHT);

const PLACEHOLDER: &'static str = if cfg!(target_os = "macos") {
    "输入消息与AI助手对话...，按Cmd+Enter发送，按ESC清除输入框"
} else {
    "输入消息与AI助手对话...，按Ctrl+Enter发送，按ESC清除输入框"
};

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
        let todo_id = todoitem.id.clone();

        window.on_window_should_close(cx, move |_window, app| {
            app.dispatch_event(FoEvent::TodoChatWindowClosed(todo_id.clone()));
            true
        });
        // 聊天输入框 - 多行支持
        let chat_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(PLACEHOLDER)
                .clean_on_escape()
                .multi_line()
                .auto_grow(3, 8)
        });

        let _subscriptions = vec![cx.subscribe_in(&chat_input, window, Self::on_chat_input_event)];
        let chat_messages = if todoitem.description.is_empty() {
            vec![]
        } else {
            vec![ChatMessage::system()
                .with_text(todoitem.description.clone())
                .with_source(todoitem.id.clone())]
        };
        let extend_channel = Self::start_external_message_handler(todoitem.id.clone(), cx);
        let instance = Self {
            focus_handle: cx.focus_handle(),
            chat_messages,
            chat_input,
            is_loading: false,
            is_running: false,
            scroll_handle: ScrollHandle::new(),
            expanded_providers: Vec::new(),
            expanded_tool_providers: Vec::new(),
            cached_server_tools: std::collections::HashMap::new(),
            _subscriptions,
            scroll_state: ScrollbarState::default(),
            scroll_size: gpui::Size::default(),
            todoitem,
            extend_channel,
        };

        instance.scroll_handle.scroll_to_bottom();
        instance
    }

    /// 启动外部消息处理器
    fn start_external_message_handler(
        todo_id: String,
        cx: &mut Context<Self>,
    ) -> Sender<StreamMessage> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let tx1 = tx.clone();
        cx.spawn(async move |this, app: &mut AsyncApp| {
            Self::handle_external_messages(this, app, (todo_id, tx1, rx)).await;
        })
        .detach();
        tx
    }

    /// 处理外部消息的异步任务
    async fn handle_external_messages(
        this: WeakEntity<Self>,
        app: &mut AsyncApp,
        (todo_id, tx, mut rx): (String, Sender<StreamMessage>, Receiver<StreamMessage>),
    ) {
        let todo_id_clone = todo_id.clone();
        // 订阅外部消息
        let _sub = app.subscribe_event(move |msg: &StreamMessage| match msg {
            StreamMessage::Stream(source, message) => {
                if &todo_id_clone == source {
                    tracing::debug!("接收到消息: {} {:?}", source, message);
                    tx.try_send(msg.clone()).unwrap_or_else(|e| {
                        tracing::error!("Failed to send message to channel: {}", e);
                    });
                }
            }
            StreamMessage::Done(source) => {
                if &todo_id_clone == source {
                    tx.try_send(msg.clone()).unwrap_or_else(|e| {
                        tracing::error!("Failed to send message to channel: {}", e);
                    });
                    tracing::debug!("接收到完成消息: {}", source);
                }
            }
        });
        tracing::info!(
            "开始处理外部消息 todoid is {} subscription: {:?}",
            todo_id,
            _sub
        );
        // 消息处理循环
        'message_loop: loop {
            Timer::after(Duration::from_millis(50)).await;
            let mut buffer = String::new();
            let mut message_count = 0;

            // 批量收集消息
            loop {
                match rx.try_recv() {
                    Err(TryRecvError::Empty) => {
                        break;
                    }
                    Err(TryRecvError::Disconnected) => {
                        break 'message_loop;
                    }
                    Ok(msg) => match msg {
                        StreamMessage::Stream(_, content) => match content {
                            MessageContent::ToolFunction(tool) => {
                                buffer.push_str(&format!(
                                    "工具调用: {}({})\n",
                                    tool.name, tool.arguments
                                ));
                            }
                            MessageContent::TextChunk(text) => {
                                if !text.is_empty() {
                                    buffer.push_str(&text);
                                }
                            }
                            MessageContent::Part(part) => {
                                if let Some(text) = part.get_text() {
                                    if !text.is_empty() {
                                        buffer.push_str(text);
                                    }
                                }
                            }

                            MessageContent::ToolDefinitions(_) => {}
                        },
                        StreamMessage::Done(_) => {
                            this.update(app, |this, cx| {
                                this.is_running = false;
                            })
                            .ok();
                        }
                    },
                }
                this.upgrade().is_none().then(|| {
                    tracing::warn!("组件已被销毁，停止消息处理");
                    return;
                });
            }
            if buffer.is_empty() {
                continue;
            }
            if this
                .update(app, |this, cx| {
                    this.process_received_message(MessageContent::TextChunk(buffer), cx);
                })
                .is_err()
            {
                tracing::warn!("组件已被销毁，停止消息处理");
                break 'message_loop;
            };
            message_count += 1;
            tracing::debug!("处理了 {} 条消息", message_count);
        }
        tracing::info!("外部消息处理器已停止 todoid is {}", todo_id);
    }

    fn last_message(&mut self) -> &mut ChatMessage {
        if self.chat_messages.is_empty() {
            self.chat_messages.push(ChatMessage::assistant());
        }
        self.chat_messages.last_mut().expect("No messages in chat")
    }
    /// 处理接收到的消息
    fn process_received_message(&mut self, msg: MessageContent, cx: &mut Context<Self>) {
        let last_message = self.last_message();
        last_message.add_content(msg);
        self.is_loading = false;
        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }
}
