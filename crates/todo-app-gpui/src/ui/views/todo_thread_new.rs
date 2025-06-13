use crate::app::AppState;
use crate::ui::{components::ViewKit, AppExt};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    input::{InputEvent, InputState, TextInput},
    scroll::{Scrollbar, ScrollbarState},
    *,
};
use std::{cell::Cell, rc::Rc};

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
    todo_id: u32,

    // 聊天功能
    chat_messages: Vec<ChatMessage>,
    chat_input: Entity<InputState>,
    is_loading: bool,
    scroll_handle: ScrollHandle,
    scroll_size: gpui::Size<Pixels>,
    scroll_state: Rc<Cell<ScrollbarState>>,

    _subscriptions: Vec<Subscription>,
}

impl TodoThreadChat {
    pub fn open(todo_id: u32, cx: &mut App) {
        cx.activate(true);
        let window_size = size(px(600.0), px(800.0));
        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(gpui::Size {
                width: px(600.),
                height: px(800.),
            }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        // 获取Todo标题作为窗口名称
        let window_title = {
            let app_state = AppState::global(cx);
            if let Ok(todo) = app_state.todo_service.get_todo_by_id(todo_id) {
                format!("Todo聊天 - {}", todo.title)
            } else {
                "Todo聊天".to_string()
            }
        };

        cx.create_normal_window(window_title, options, move |window, cx| {
            Self::view(todo_id, window, cx)
        });
    }

    fn new(todo_id: u32, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 聊天输入框 - 多行支持
        let chat_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入消息与AI助手对话...，按Ctrl+Enter发送，按ESC清除输入框")
                .clean_on_escape()
                .multi_line()
                .auto_grow(1, 6)
        });

        let _subscriptions = vec![cx.subscribe_in(&chat_input, window, Self::on_chat_input_event)];

        // 初始化欢迎消息
        let chat_messages = vec![ChatMessage {
            id: "1".to_string(),
            role: MessageRole::System,
            content: "AI助手已准备就绪，我可以帮助您管理任务、回答问题和提供建议。请随时与我对话！"
                .to_string(),
            timestamp: chrono::Utc::now(),
            model: None,
            tools_used: vec![],
        }];

        Self {
            focus_handle: cx.focus_handle(),
            todo_id,
            chat_messages,
            chat_input,
            is_loading: false,
            scroll_handle: ScrollHandle::new(),
            _subscriptions,
            scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_size: gpui::Size::default(),
        }
    }

    pub fn view(todo_id: u32, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(todo_id, window, cx))
    }

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
                "我理解您的问题：\"{}\"。请告诉我更多详细信息，我会给出更精准的建议。",
                user_message
            ),
        };

        let ai_message = ChatMessage {
            id: format!("ai_{}", chrono::Utc::now().timestamp()),
            role: MessageRole::Assistant,
            content: response_content.to_string(),
            timestamp: chrono::Utc::now(),
            model: Some("GPT-4".to_string()),
            tools_used: vec![],
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
                div().max_w_96().child(
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

impl ViewKit for TodoThreadChat {
    fn title() -> &'static str {
        "Todo对话"
    }

    fn description() -> &'static str {
        "与AI助手对话，管理您的任务和计划"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(1, window, cx)
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
        v_flex()
            .key_context(CONTEXT)
            .id("todo-thread-view")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::send_message))
            .size_full()
            .p_2()
            .child(
                // 聊天消息区域
                div().size_full().min_h_32().child(
                    div().relative().size_full().child(
                        v_flex()
                            .border_1()
                            .border_color(gpui::rgb(0xE5E7EB))
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
                                                    .text_sm()
                                                    .text_color(gpui::rgb(0x6B7280))
                                                    .child("AI正在思考中..."),
                                            ),
                                        )
                                    }),
                            )
                            .child(
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
                ),
            )
            .child(
                // 聊天输入区域 - 固定在底部
                h_flex()
                    .gap_2()
                    .p_2()
                    .border_t_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        // 多行输入框
                        div().w_full().child(TextInput::new(&self.chat_input)),
                    )
                    .child(
                        h_flex().justify_end().child(
                            Button::new("send-message")
                                .with_variant(ButtonVariant::Primary)
                                .icon(IconName::Send)
                                .label("发送")
                                .disabled(self.is_loading)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.dispatch_action(Box::new(SendMessage), cx);
                                })),
                        ),
                    ),
            )
    }
}
