use chrono::{Days, Utc};
use gpui::prelude::*;
use gpui::*;

use gpui_component::{
    accordion::Accordion,
    badge::Badge,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    dropdown::{Dropdown, DropdownDelegate, DropdownEvent, DropdownItem, DropdownState},
    input::{InputEvent, InputState, TextInput},
    sidebar::{SidebarGroup, SidebarMenu, SidebarMenuItem},
    switch::Switch,
    scroll::{*},
    *,
};

use crate::ui::components::ViewKit;

actions!(todo_thread, [Tab, TabPrev, Save, Cancel, Delete]);

const CONTEXT: &str = "TodoThreadEdit";

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

// 简化的模型数据结构
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub is_selected: bool,
}

// 简化的服务商信息
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub models: Vec<ModelInfo>,
}

// 模型管理器
pub struct ModelManager {
    pub providers: Vec<ProviderInfo>,
}

impl ModelManager {
    pub fn new() -> Self {
        let providers = vec![
            ProviderInfo {
                name: "收钱吧".to_string(),
                models: vec![
                    ModelInfo {
                        name: "sqb-chat-3.5".to_string(),
                        provider: "收钱吧".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "sqb-chat-4.0".to_string(),
                        provider: "收钱吧".to_string(),
                        is_selected: false,
                    },
                ],
            },
            ProviderInfo {
                name: "Anthropic".to_string(),
                models: vec![
                    ModelInfo {
                        name: "claude-3.5-sonnet".to_string(),
                        provider: "Anthropic".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "claude-3-haiku".to_string(),
                        provider: "Anthropic".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "claude-3-opus".to_string(),
                        provider: "Anthropic".to_string(),
                        is_selected: false,
                    },
                ],
            },
            ProviderInfo {
                name: "OpenAI".to_string(),
                models: vec![
                    ModelInfo {
                        name: "gpt-4".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "gpt-4-turbo".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "gpt-3.5-turbo".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "gpt-4o".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                    },
                ],
            },
            ProviderInfo {
                name: "百度智能云".to_string(),
                models: vec![
                    ModelInfo {
                        name: "文心一言-4.0".to_string(),
                        provider: "百度智能云".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "文心一言-3.5".to_string(),
                        provider: "百度智能云".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "ERNIE-Bot-turbo".to_string(),
                        provider: "百度智能云".to_string(),
                        is_selected: false,
                    },
                ],
            },
            ProviderInfo {
                name: "阿里云".to_string(),
                models: vec![
                    ModelInfo {
                        name: "通义千问-Max".to_string(),
                        provider: "阿里云".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "通义千问-Plus".to_string(),
                        provider: "阿里云".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "通义千问-Turbo".to_string(),
                        provider: "阿里云".to_string(),
                        is_selected: false,
                    },
                ],
            },
            ProviderInfo {
                name: "腾讯云".to_string(),
                models: vec![
                    ModelInfo {
                        name: "混元-Pro".to_string(),
                        provider: "腾讯云".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "混元-Standard".to_string(),
                        provider: "腾讯云".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "混元-Lite".to_string(),
                        provider: "腾讯云".to_string(),
                        is_selected: false,
                    },
                ],
            },
            ProviderInfo {
                name: "字节跳动".to_string(),
                models: vec![
                    ModelInfo {
                        name: "豆包-Pro-32K".to_string(),
                        provider: "字节跳动".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "豆包-Pro-4K".to_string(),
                        provider: "字节跳动".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "豆包-Lite-4K".to_string(),
                        provider: "字节跳动".to_string(),
                        is_selected: false,
                    },
                ],
            },
            ProviderInfo {
                name: "智谱AI".to_string(),
                models: vec![
                    ModelInfo {
                        name: "GLM-4".to_string(),
                        provider: "智谱AI".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "GLM-4-Air".to_string(),
                        provider: "智谱AI".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "GLM-3-Turbo".to_string(),
                        provider: "智谱AI".to_string(),
                        is_selected: false,
                    },
                ],
            },
        ];

        Self { providers }
    }

    pub fn toggle_model_selection(&mut self, model_name: &str) {
        for provider in &mut self.providers {
            for model in &mut provider.models {
                if model.name == model_name {
                    model.is_selected = !model.is_selected;
                    return;
                }
            }
        }
    }

    pub fn get_selected_models(&self) -> Vec<String> {
        let mut selected = Vec::new();
        for provider in &self.providers {
            for model in &provider.models {
                if model.is_selected {
                    selected.push(model.name.clone());
                }
            }
        }
        selected
    }

    pub fn get_selected_count(&self) -> usize {
        self.get_selected_models().len()
    }
}

pub struct TodoThreadEdit {
    focus_handle: FocusHandle,

    // 基本信息
    title_input: Entity<InputState>,
    description_input: Entity<InputState>,

    // 状态和优先级
    status_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    priority_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // AI助手配置 - 简化为模型管理器
    model_manager: ModelManager,
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

impl TodoThreadEdit {
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

        // 简化的模型管理器
        let model_manager = ModelManager::new();

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
            cx.subscribe(&due_date_picker, |_, _, ev, cx| match ev {
                DatePickerEvent::Change(_) => {
                    cx.notify();
                }
            }),
            cx.subscribe(&reminder_date_picker, |_, _, ev, cx| match ev {
                DatePickerEvent::Change(_) => {
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
            model_manager,
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
        let selected_models = self.model_manager.get_selected_models();

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
            selected_models,
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => {
                self.save(&Save, window, cx);
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

    // 获取模型选择显示文本
    fn get_model_display_text(&self, _cx: &App) -> String {
        let selected_models = self.model_manager.get_selected_models();
        let selected_count = selected_models.len();

        if selected_count == 0 {
            "选择AI模型".to_string()
        } else if selected_count <= 2 {
            selected_models.join(", ")
        } else {
            let first_two = selected_models
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} 等{}个模型", first_two, selected_count)
        }
    }

    fn open_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 使用 Entity 来共享状态
        let todo_edit_entity = cx.entity().clone();
        
        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            // 从 entity 中读取当前的模型数据
            let providers = todo_edit_entity.read(drawer_cx).model_manager.providers.clone();

            // 创建手风琴组件
            let mut accordion = Accordion::new("model-providers").multiple(true);

            for (provider_index, provider) in providers.iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_models = provider.models.clone();

                accordion = accordion.item(|item| {
                    item.open(true) // 默认展开所有服务商
                        .icon(IconName::Bot)
                        .title(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .font_medium()
                                        .text_color(gpui::rgb(0x374151))
                                        .child(provider_name.clone()),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(gpui::rgb(0xEFF6FF))
                                        .text_color(gpui::rgb(0x1D4ED8))
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
                                        let model_name_for_event = model.name.clone();
                                        let checkbox_id = SharedString::new(format!(
                                            "model-{}-{}",
                                            provider_index, model_index
                                        ));
                                        let todo_edit_entity_for_event = todo_edit_entity.clone();

                                        div()
                                            .p_2()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .rounded_md()
                                            .border_1()
                                            .border_color(gpui::rgb(0xE5E7EB))
                                            .hover(|style| style.bg(gpui::rgb(0xF3F4F6)))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        Checkbox::new(checkbox_id)
                                                            .checked(model.is_selected)
                                                            .label(model.name.clone())
                                                            .on_click(
                                                                move |_checked, _window, cx| {
                                                                    let model_name_to_toggle =
                                                                        model_name_for_event.clone();
                                                                    
                                                                    // 更新原始数据
                                                                    todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                        todo_edit.model_manager.toggle_model_selection(&model_name_to_toggle);
                                                                        todo_cx.notify(); // 通知主界面更新
                                                                    });

                                                                    println!(
                                                                        "切换模型选择: {}",
                                                                        model_name_to_toggle
                                                                    );
                                                                },
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .px_2()
                                                            .py_1()
                                                            .bg(gpui::rgb(0xF3F4F6))
                                                            .text_color(gpui::rgb(0x6B7280))
                                                            .rounded_md()
                                                            .text_xs()
                                                            .child(model.provider.clone()),
                                                    ),
                                            )
                                    },
                                ))
                                .when(provider_models.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .p_4()
                                            .text_center()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x9CA3AF))
                                            .child("该服务商暂无可用模型"),
                                    )
                                }),
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(380.)) // 稍微增加宽度以适应手风琴
                .title("选择模型")
                .child(accordion) // 直接放置手风琴，不需要额外的滚动容器
                .footer(
                    h_flex()
                        .justify_end() // 只保留右对齐的按钮
                        .items_center()
                        .p_3()
                        .border_t_1() // 添加顶部边框分隔
                        .border_color(gpui::rgb(0xE5E7EB))
                        .bg(gpui::rgb(0xFAFAFA)) // 底部背景色
                        .child(
                            Button::new("clear-all-models")
                                .label("清空选择")
                                .on_click(move |_, _window, cx| {
                                    // 清空所有模型选择
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        for provider in &mut todo_edit.model_manager.providers {
                                            for model in &mut provider.models {
                                                model.is_selected = false;
                                            }
                                        }
                                        todo_cx.notify(); // 通知主界面更新
                                    });
                                    println!("清空所有模型选择");
                                }),
                        ),
                )
        });
    }
}

#[derive(Debug)]
struct TodoData {
    title: String,
    description: String,
    status: String,
    priority: String,
    selected_models: Vec<String>, // 多选模型列表
    mcp_tools: String,
    recurring_enabled: bool,
    auto_execute: bool,
    enable_notifications: bool,
}

impl ViewKit for TodoThreadEdit {
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

impl FocusableCycle for TodoThreadEdit {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![
            self.title_input.focus_handle(cx),
            self.description_input.focus_handle(cx),
            self.status_dropdown.focus_handle(cx),
            self.priority_dropdown.focus_handle(cx),
            self.mcp_tools_dropdown.focus_handle(cx),
            self.due_date_picker.focus_handle(cx),
            self.reminder_date_picker.focus_handle(cx),
            self.recurring_dropdown.focus_handle(cx),
        ]
    }
}

impl Focusable for TodoThreadEdit {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoThreadEdit {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .p_2()
            .gap_2()
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        v_flex()
                            .gap_3()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .items_center()
                                            .child(Self::section_title("任务描述"))
                                            .child(
                                                Checkbox::new("push-feishu-button")
                                                    .label("推送到飞书")
                                                    .checked(true)
                                                    .on_click(cx.listener(|view, _, _, cx| {
                                                        // view.disabled = !view.disabled;
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                    .child(TextInput::new(&self.description_input).cleanable()),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(
                                div()
                                    .id("file-drop-zone")
                                    .h_24()
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
                                                Icon::new(IconName::Upload)
                                                    .size_6()
                                                    .text_color(gpui::rgb(0x6B7280)),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(gpui::rgb(0x9CA3AF))
                                                    .child("拖拽文件到此处上传或点击选择文件"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(gpui::rgb(0xB91C1C))
                                                    .child("支持 PDF、DOC、TXT、图片等格式"),
                                            ),
                                    )
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        println!("点击上传文件");
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(Self::section_title("助手配置"))
                            .child(Self::form_row(
                                "MCP工具",
                                Dropdown::new(&self.mcp_tools_dropdown)
                                    .placeholder("选择工具集")
                                    .small(),
                            ))
                            .child(
                                h_flex()
                                    .gap_4()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x6B7280))
                                            .min_w_24()
                                            .child("模型选择"),
                                    )
                                    .child(
                                        div().flex_1().max_w_80().child(
                                            Button::new("show-drawer-left")
                                                .label({
                                                    let display_text =
                                                        self.get_model_display_text(cx);
                                                    if display_text == "选择AI模型" {
                                                        display_text
                                                    } else {
                                                        display_text
                                                    }
                                                })
                                                .w_full()
                                                .justify_start()
                                                .text_color(
                                                    if self.get_model_display_text(cx)
                                                        == "选择AI模型"
                                                    {
                                                        gpui::rgb(0x9CA3AF)
                                                    } else {
                                                        gpui::rgb(0x374151)
                                                    },
                                                )
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.open_drawer_at(Placement::Left, window, cx)
                                                })),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(Self::section_title("时间安排"))
                            .child(Self::form_row(
                                "截止日期",
                                DatePicker::new(&self.due_date_picker)
                                    .placeholder("选择截止日期")
                                    .cleanable()
                                    .presets(due_date_presets.clone())
                                    .small(),
                            ))
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
                                            .on_click(cx.listener(
                                                move |this, checked, window, cx| {
                                                    this.toggle_recurring(*checked, window, cx);
                                                },
                                            )),
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
                    ),
            )
            .child(
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
