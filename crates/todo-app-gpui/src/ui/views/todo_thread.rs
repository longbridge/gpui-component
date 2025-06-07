use std::{cell::Cell, rc::Rc};

use chrono::{Days, Utc};
use gpui::prelude::*;
use gpui::*;

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    dropdown::{Dropdown, DropdownDelegate, DropdownEvent, DropdownItem, DropdownState},
    h_flex,
    input::{InputEvent, InputState, TextInput},
    scroll::{Scrollable, Scrollbar, ScrollbarState},
    v_flex, Disableable, FocusableCycle, Icon, IconName, Sizable, StyledExt,
};

use crate::ui::components::ViewKit;

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

// 层级化的模型选项结构
#[derive(Debug, Clone)]
pub enum ModelOption {
    Provider { name: String, expanded: bool },
    Model { name: String, provider: String },
}

impl DropdownItem for ModelOption {
    type Value = String;

    fn title(&self) -> SharedString {
        match self {
            ModelOption::Provider { name, .. } => name.clone().into(),
            ModelOption::Model { name, .. } => name.clone().into(),
        }
    }

    fn display_title(&self) -> Option<AnyElement> {
        match self {
            ModelOption::Provider { name, .. } => Some(
                h_flex()
                    .items_center()
                    .py_1()
                    .child(
                        div()
                            .font_semibold()
                            .text_color(gpui::rgb(0x374151))
                            .child(name.clone()),
                    )
                    .into_any_element(),
            ),
            ModelOption::Model { name, provider } => Some(
                h_flex()
                    .items_center()
                    .gap_3()
                    .pl_6()
                    .py_1()
                    .child(
                        div()
                            .w_4()
                            .h_4()
                            .border_1()
                            .border_color(gpui::rgb(0xD1D5DB))
                            .bg(gpui::rgb(0xFFFFFF))
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .justify_center(),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(gpui::rgb(0x6B7280))
                            .child(name.clone()),
                    )
                    .into_any_element(),
            ),
        }
    }

    fn value(&self) -> &Self::Value {
        match self {
            ModelOption::Provider { name, .. } => name,
            ModelOption::Model { name, .. } => name,
        }
    }
}

// 层级化的Dropdown委托
pub struct HierarchicalModelDelegate {
    providers: Vec<(String, Vec<String>)>,
    flattened_options: Vec<ModelOption>,
    selected_model: Option<String>,
}

impl HierarchicalModelDelegate {
    pub fn new() -> Self {
        let providers = vec![
            (
                "收钱吧".to_string(),
                vec!["sqb-chat-3.5".to_string(), "sqb-chat-4.0".to_string()],
            ),
            (
                "Anthropic".to_string(),
                vec![
                    "claude-3.5-sonnet".to_string(),
                    "claude-3-haiku".to_string(),
                    "claude-3-opus".to_string(),
                ],
            ),
            (
                "OpenAI".to_string(),
                vec![
                    "gpt-4".to_string(),
                    "gpt-4-turbo".to_string(),
                    "gpt-3.5-turbo".to_string(),
                ],
            ),
        ];

        let mut flattened_options = Vec::new();
        for (provider_name, models) in &providers {
            flattened_options.push(ModelOption::Provider {
                name: provider_name.clone(),
                expanded: true,
            });

            for model_name in models {
                flattened_options.push(ModelOption::Model {
                    name: model_name.clone(),
                    provider: provider_name.clone(),
                });
            }
        }

        Self {
            providers,
            flattened_options,
            selected_model: None,
        }
    }

    pub fn set_selected_model(&mut self, model: Option<String>) {
        self.selected_model = model;
    }

    pub fn get_selected_model(&self) -> Option<&String> {
        self.selected_model.as_ref()
    }
}

impl DropdownDelegate for HierarchicalModelDelegate {
    type Item = ModelOption;

    fn len(&self) -> usize {
        self.flattened_options.len()
    }

    fn get(&self, ix: usize) -> Option<&Self::Item> {
        self.flattened_options.get(ix)
    }

    fn position<V>(&self, value: &V) -> Option<usize>
    where
        Self::Item: DropdownItem<Value = V>,
        V: PartialEq,
    {
        self.flattened_options
            .iter()
            .position(|item| item.value() == value)
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

    // AI助手配置
    model_dropdown: Entity<DropdownState<HierarchicalModelDelegate>>,
    mcp_tools_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    _subscriptions: Vec<Subscription>,
}

impl TodoThreadChat {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 聊天输入框 - 多行支持
        let chat_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入消息与AI助手对话...，按Ctrl+Enter发送，按ESC清除输入框")
                .clean_on_escape()
                .multi_line()
                .auto_grow(1, 6)
        });

        // AI助手配置
        let model_delegate = HierarchicalModelDelegate::new();
        let model_dropdown = cx.new(|cx| DropdownState::new(model_delegate, None, window, cx));

        let mcp_tools = vec![
            "文件操作".into(),
            "代码审查".into(),
            "网络搜索".into(),
            "计算器".into(),
        ];
        let mcp_tools_dropdown = cx.new(|cx| DropdownState::new(mcp_tools, None, window, cx));

        let _subscriptions = vec![
            cx.subscribe_in(&chat_input, window, Self::on_chat_input_event),
            // 监听模型选择变化
            cx.subscribe(&model_dropdown, |this, _, event, cx| match event {
                DropdownEvent::Confirm(selected_value) => {
                    if let Some(model_name) = selected_value {
                        println!("选择了模型: {}", model_name);
                    }
                    cx.notify();
                }
            }),
        ];

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
            chat_messages,
            chat_input,
            is_loading: false,
            scroll_handle: ScrollHandle::new(),
            model_dropdown,
            mcp_tools_dropdown,
            _subscriptions,
            scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_size: gpui::Size::default(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
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

        // 添加用户消息
        let user_message = ChatMessage {
            id: format!("user_{}", chrono::Utc::now().timestamp()),
            role: MessageRole::User,
            content: message_content.clone(),
            timestamp: chrono::Utc::now(),
            model: None,
            tools_used: vec![],
        };

        self.chat_messages.push(user_message);

        // 清空输入框
        self.chat_input
            .update(cx, |input, cx| input.set_value("", window, cx));

        // 设置加载状态
        self.is_loading = true;

        // 模拟AI响应
        self.simulate_ai_response(message_content, cx);
        self.scroll_handle.scroll_to_bottom();

        cx.notify();
    }

    fn simulate_ai_response(&mut self, user_message: String, cx: &mut Context<Self>) {
        // 获取当前选择的模型和工具
        let selected_model = self
            .model_dropdown
            .read(cx)
            .selected_value()
            .map(|v| v.to_string());

        let selected_tools = self
            .mcp_tools_dropdown
            .read(cx)
            .selected_value()
            .map(|v| vec![v.to_string()])
            .unwrap_or_default();

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
                selected_model.as_deref().unwrap_or("默认")
            ),
        };

        // 添加AI响应消息
        let ai_message = ChatMessage {
            id: format!("ai_{}", chrono::Utc::now().timestamp()),
            role: MessageRole::Assistant,
            content: response_content.to_string(),
            timestamp: chrono::Utc::now(),
            model: selected_model,
            tools_used: selected_tools,
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
                // Ctrl+Enter 发送消息
                self.send_message(&SendMessage, window, cx);
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
        Self::view(window, cx)
    }
}

impl FocusableCycle for TodoThreadChat {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![
            self.chat_input.focus_handle(cx),
            self.model_dropdown.focus_handle(cx),
            self.mcp_tools_dropdown.focus_handle(cx),
        ]
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
                div().w_full().flex_1().min_h_64().child(
                    div().relative().border_1().size_full().child(
                        v_flex().id("test-0")
                            .relative()
                            .size_full()
                            .child(
                                v_flex()
                                    .id("id-todo-thread-chat")
                                    .p_2()
                                    .gap_2()
                                    .relative()
                                    .size_full()
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
                            .child({
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .right_0()
                                    .bottom_0()
                                    .child(
                                        Scrollbar::both(
                                            cx.entity().entity_id(),
                                            self.scroll_state.clone(),
                                            self.scroll_handle.clone(),
                                            self.scroll_size,
                                        )
                                        .axis(gpui_component::scroll::ScrollbarAxis::Vertical),
                                    )
                            }),
                    ),
                ),
            )
            .child(
                // 中间区域：模型和工具选择
                h_flex()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .p_2()
                    .border_t_1()
                    .border_b_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .bg(gpui::rgb(0xF9FAFB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child("模型:"),
                            )
                            .child(
                                div().w_48().child(
                                    Dropdown::new(&self.model_dropdown)
                                        .placeholder("选择模型")
                                        .small(),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child("工具:"),
                            )
                            .child(
                                div().w_48().child(
                                    Dropdown::new(&self.mcp_tools_dropdown)
                                        .placeholder("选择工具")
                                        .small(),
                                ),
                            ),
                    ),
            )
            .child(
                // 聊天输入区域 - 固定在底部
                h_flex()
                    .gap_2()
                    .p_2()
                    .child(
                        // 多行输入框
                        div().w_full().child(TextInput::new(&self.chat_input)),
                    )
                    .child(
                        // 发送按钮区域
                        h_flex().justify_end().child(
                            Button::new("send-message")
                                .with_variant(ButtonVariant::Primary)
                                .icon(IconName::Send)
                                .label("发送")
                                .disabled(self.is_loading)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.send_message(&SendMessage, window, cx)
                                })),
                        ),
                    ),
            )
    }
}
