use chrono::{Days, Utc};
use gpui::prelude::*;
use gpui::*;

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    dropdown::{Dropdown, DropdownState},
    h_flex,
    input::{InputEvent, InputState, TextInput},
    switch::Switch,
    v_flex, FocusableCycle, Icon, IconName, Sizable, StyledExt,
};

use crate::ui::components::ViewKit;

actions!(todo_thread, [Tab, TabPrev, Save, Cancel, Delete]);

const CONTEXT: &str = "TodoThread";

#[derive(Debug, Clone)]
pub enum TodoPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TodoPriority {
    fn as_str(&self) -> &'static str {
        match self {
            TodoPriority::Low => "低",
            TodoPriority::Medium => "中",
            TodoPriority::High => "高",
            TodoPriority::Urgent => "紧急",
        }
    }

    fn all() -> Vec<SharedString> {
        vec!["低".into(), "中".into(), "高".into(), "紧急".into()]
    }

    fn icon(&self) -> IconName {
        match self {
            TodoPriority::Low => IconName::ArrowDown,
            TodoPriority::Medium => IconName::Minus,
            TodoPriority::High => IconName::ArrowUp,
            TodoPriority::Urgent => IconName::TriangleAlert,
        }
    }

    fn color(&self) -> gpui::Rgba {
        match self {
            TodoPriority::Low => gpui::rgb(0x6B7280),
            TodoPriority::Medium => gpui::rgb(0x3B82F6),
            TodoPriority::High => gpui::rgb(0xF59E0B),
            TodoPriority::Urgent => gpui::rgb(0xEF4444),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TodoStatus {
    Todo,
    InProgress,
    Done,
    Cancelled,
}

impl TodoStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Todo => "待办",
            TodoStatus::InProgress => "进行中",
            TodoStatus::Done => "已完成",
            TodoStatus::Cancelled => "已取消",
        }
    }

    fn all() -> Vec<SharedString> {
        vec![
            "待办".into(),
            "进行中".into(),
            "已完成".into(),
            "已取消".into(),
        ]
    }
}

pub struct TodoThreadView {
    focus_handle: FocusHandle,

    // 基本信息
    title_input: Entity<InputState>,
    description_input: Entity<InputState>,

    // 状态和优先级
    status_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    priority_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // AI助手配置
    llm_provider_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    model_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    mcp_tools_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // 时间设置
    due_date_picker: Entity<DatePickerState>,
    reminder_date_picker: Entity<DatePickerState>,
    recurring_enabled: bool,
    recurring_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // 其他设置
    auto_execute: bool,
    enable_notifications: bool,

    _subscriptions: Vec<Subscription>,
}

impl TodoThreadView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 基本信息输入框
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入任务标题..."));

        let description_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("详细描述任务内容和要求...")
                .auto_grow(5, 10)
        });

        // 状态和优先级下拉框
        let status_dropdown =
            cx.new(|cx| DropdownState::new(TodoStatus::all(), Some(0), window, cx));

        let priority_dropdown =
            cx.new(|cx| DropdownState::new(TodoPriority::all(), Some(1), window, cx));

        // AI助手配置下拉框
        let llm_providers = vec!["收钱吧".into(), "Anthropic".into(), "OpenAI".into()];
        let llm_provider_dropdown =
            cx.new(|cx| DropdownState::new(llm_providers, Some(0), window, cx));

        let models = vec![
            "claude-3.5-sonnet".into(),
            "gpt-4".into(),
            "gpt-3.5-turbo".into(),
        ];
        let model_dropdown = cx.new(|cx| DropdownState::new(models, Some(0), window, cx));

        let mcp_tools = vec![
            "文件操作".into(),
            "代码审查".into(),
            "网络搜索".into(),
            "计算器".into(),
        ];
        let mcp_tools_dropdown = cx.new(|cx| DropdownState::new(mcp_tools, None, window, cx));

        // 时间选择器
        let due_date_picker = cx.new(|cx| DatePickerState::new(window, cx));

        let reminder_date_picker = cx.new(|cx| DatePickerState::new(window, cx));

        let recurring_options = vec!["每日".into(), "每周".into(), "每月".into(), "每年".into()];
        let recurring_dropdown =
            cx.new(|cx| DropdownState::new(recurring_options, Some(1), window, cx));

        let _subscriptions = vec![
            cx.subscribe_in(&title_input, window, Self::on_input_event),
            cx.subscribe_in(&description_input, window, Self::on_input_event),
            cx.subscribe(&due_date_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(_) => {
                    println!("截止日期已更改");
                    cx.notify();
                }
            }),
            cx.subscribe(&reminder_date_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(_) => {
                    println!("提醒日期已更改");
                    cx.notify();
                }
            }),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            title_input,
            description_input,
            status_dropdown,
            priority_dropdown,
            llm_provider_dropdown,
            model_dropdown,
            mcp_tools_dropdown,
            due_date_picker,
            reminder_date_picker,
            recurring_enabled: false,
            recurring_dropdown,
            auto_execute: false,
            enable_notifications: true,
            _subscriptions,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn save(&mut self, _: &Save, _window: &mut Window, cx: &mut Context<Self>) {
        let todo_data = TodoData {
            title: self.title_input.read(cx).value().to_string(),
            description: self.description_input.read(cx).value().to_string(),
            status: self
                .status_dropdown
                .read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            priority: self
                .priority_dropdown
                .read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            llm_provider: self
                .llm_provider_dropdown
                .read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            model: self
                .model_dropdown
                .read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            mcp_tools: self
                .mcp_tools_dropdown
                .read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            recurring_enabled: self.recurring_enabled,
            auto_execute: self.auto_execute,
            enable_notifications: self.enable_notifications,
        };

        println!("保存Todo: {:?}", todo_data);
        cx.notify();
    }

    fn cancel(&mut self, _: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
        println!("取消编辑");
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        println!("删除Todo");
        cx.notify();
    }

    fn toggle_recurring(&mut self, enabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.recurring_enabled = enabled;
        cx.notify();
    }

    fn toggle_auto_execute(&mut self, enabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.auto_execute = enabled;
        cx.notify();
    }

    fn toggle_notifications(&mut self, enabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.enable_notifications = enabled;
        cx.notify();
    }

    fn on_input_event(
        &mut self,
        _entity: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => {
                // 按回车键保存
                self.save(&Save, _window, cx);
            }
            _ => {}
        }
    }

    fn section_title(title: &'static str) -> impl IntoElement {
        div()
            .text_lg()
            .font_semibold()
            .text_color(gpui::rgb(0x374151))
            .pb_2()
            .child(title)
    }

    fn form_row(label: &'static str, content: impl IntoElement) -> impl IntoElement {
        h_flex()
            .gap_4()
            .items_center()
            .child(
                div()
                    .text_sm()
                    .text_color(gpui::rgb(0x6B7280))
                    .min_w_24()
                    .child(label),
            )
            .child(div().flex_1().max_w_80().child(content))
    }
}

#[derive(Debug)]
struct TodoData {
    title: String,
    description: String,
    status: String,
    priority: String,
    llm_provider: String,
    model: String,
    mcp_tools: String,
    recurring_enabled: bool,
    auto_execute: bool,
    enable_notifications: bool,
}

impl ViewKit for TodoThreadView {
    fn title() -> &'static str {
        "任务编辑"
    }

    fn description() -> &'static str {
        "创建和编辑任务，配置AI助手和时间安排"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl FocusableCycle for TodoThreadView {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![
            self.title_input.focus_handle(cx),
            self.description_input.focus_handle(cx),
            self.status_dropdown.focus_handle(cx),
            self.priority_dropdown.focus_handle(cx),
            self.llm_provider_dropdown.focus_handle(cx),
            self.model_dropdown.focus_handle(cx),
            self.mcp_tools_dropdown.focus_handle(cx),
            self.due_date_picker.focus_handle(cx),
            self.reminder_date_picker.focus_handle(cx),
            self.recurring_dropdown.focus_handle(cx),
        ]
    }
}

impl Focusable for TodoThreadView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoThreadView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let due_date_presets = vec![
            DateRangePreset::single("今天", Utc::now().naive_local().date()),
            DateRangePreset::single(
                "明天",
                (Utc::now() + chrono::Duration::days(1))
                    .naive_local()
                    .date(),
            ),
            DateRangePreset::single(
                "下周",
                (Utc::now() + chrono::Duration::weeks(1))
                    .naive_local()
                    .date(),
            ),
            DateRangePreset::single(
                "下个月",
                (Utc::now() + chrono::Duration::days(30))
                    .naive_local()
                    .date(),
            ),
        ];

        v_flex()
            .key_context(CONTEXT)
            .id("todo-thread-view")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::delete))
            .size_full()
            .gap_2()
            .p_2()
            .child(
                // 基本信息
                v_flex()
                    .gap_3()
                    .p_2()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Self::section_title("任务描述"))
                            .text_sm()
                            .child(
                                TextInput::new(&self.description_input).cleanable(), // .prefix(Icon::new(IconName::LetterText).small().ml_3()),
                            ),
                    ),
            )
            .child(
                // 附件拖拽上传区域 - 添加section结构保持一致
                v_flex()
                    .gap_3()
                    .p_2()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    // .child(Self::section_title("附件上传"))
                    .child(
                        div()
                            .id("file-drop-zone")
                            .h_32()
                            .w_full()
                            .border_2()
                            .border_color(gpui::rgb(0xD1D5DB))
                            .border_dashed()
                            .rounded_lg()
                            .bg(gpui::rgb(0xFAFAFA))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| {
                                style
                                    .border_color(gpui::rgb(0x3B82F6))
                                    .bg(gpui::rgb(0xF0F9FF))
                            })
                            .active(|style| {
                                style
                                    .border_color(gpui::rgb(0x1D4ED8))
                                    .bg(gpui::rgb(0xE0F2FE))
                            })
                            .child(
                                v_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        // 上传图标
                                        Icon::new(IconName::Upload)
                                            .size_6()
                                            .text_color(gpui::rgb(0x6B7280)),
                                    )
                                    .child(
                                        // 副标题提示
                                        div()
                                            .text_xs()
                                            .text_color(gpui::rgb(0x9CA3AF))
                                            .child("拖拽文件到此处上传或点击选择文件"),
                                    )
                                    .child(
                                        // 支持的文件类型提示
                                        div()
                                            .text_xs()
                                            .text_color(gpui::rgb(0xB91C1C))
                                            .child("支持 PDF、DOC、TXT、图片等格式"),
                                    ),
                            )
                            .on_click(cx.listener(|this, _, _window, cx| {
                                // TODO: 处理点击上传逻辑
                                println!("点击上传文件");
                                cx.notify();
                            })), // TODO: 添加拖拽事件处理
                                 // .on_drag_enter(...)
                                 // .on_drag_over(...)
                                 // .on_drop(...)
                    ),
            )
            .child(
                // AI助手配置
                v_flex()
                    .gap_3()
                    .p_2()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    .child(Self::section_title("AI助手配置"))
                    .child(Self::form_row(
                        "服务提供商",
                        Dropdown::new(&self.llm_provider_dropdown)
                            .placeholder("选择LLM服务")
                            .small(),
                    ))
                    .child(Self::form_row(
                        "模型",
                        Dropdown::new(&self.model_dropdown)
                            .placeholder("选择模型")
                            .small(),
                    ))
                    .child(Self::form_row(
                        "MCP工具",
                        Dropdown::new(&self.mcp_tools_dropdown)
                            .placeholder("选择工具集")
                            .small(),
                    )),
            )
            .child(
                // 时间安排
                v_flex()
                    .gap_3()
                    .p_2()
                    .bg(gpui::rgb(0xF9FAFB))
                    .rounded_lg()
                    .child(Self::section_title("时间安排"))
                    .child(
                        // 使用 form_row 保持一致的对齐
                        Self::form_row(
                            "截止日期",
                            DatePicker::new(&self.due_date_picker)
                                .placeholder("选择截止日期")
                                .cleanable()
                                .presets(due_date_presets.clone())
                                .small(),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .min_w_24()
                                    .child("周期重复"),
                            )
                            .child(
                                Switch::new("recurring")
                                    .checked(self.recurring_enabled)
                                    .on_click(cx.listener(move |this, checked, window, cx| {
                                        this.toggle_recurring(*checked, window, cx);
                                    })),
                            )
                            .when(self.recurring_enabled, |this| {
                                this.child(
                                    div().ml_4().child(
                                        Dropdown::new(&self.recurring_dropdown)
                                            .placeholder("选择周期")
                                            .small(),
                                    ),
                                )
                            }),
                    ),
            )
            .child(
                // 操作按钮
                h_flex().items_center().justify_center().pt_2().child(
                    h_flex().gap_3().child(
                        Button::new("save-btn")
                            .with_variant(ButtonVariant::Primary)
                            .label("保存任务")
                            .icon(IconName::Check)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.save(&Save, window, cx)),
                            ),
                    ),
                ),
            )
    }
}
