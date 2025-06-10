use chrono::{ Utc};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    dropdown::{Dropdown,  DropdownState},
    input::{InputEvent, InputState, TextInput},
    label::Label,
    switch::Switch,
    tooltip::Tooltip,
    *,
};
use crate::ui::{AppExt,components::ViewKit, views::todolist::Todo};

actions!(todo_thread, [Tab, TabPrev, Save, Cancel, Delete]);

const CONTEXT: &str = "TodoThreadEdit";

// 添加文件信息结构体
#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum TodoPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TodoPriority {
   const fn as_str(&self) -> &'static str {
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
   const fn as_str(&self) -> &'static str {
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

// 模型能力
#[derive(Debug, Clone)]
pub enum ModelCapability {
    Text,
    Vision,
    Audio,
    Tools,
}

impl ModelCapability {
   const fn icon(&self) -> IconName {
        match self {
            ModelCapability::Text => IconName::LetterText,
            ModelCapability::Vision => IconName::Eye,
            ModelCapability::Audio => IconName::Mic,
            ModelCapability::Tools => IconName::Wrench,
        }
    }

   const fn label(&self) -> &'static str {
        match self {
            ModelCapability::Text => "文本",
            ModelCapability::Vision => "视觉",
            ModelCapability::Audio => "音频",
            ModelCapability::Tools => "工具",
        }
    }
}

// 工具能力
#[derive(Debug, Clone)]
pub enum ToolCapability {
    FileOperation,
    CodeReview,
    WebSearch,
    Calculation,
    DataAnalysis,
    ImageProcessing,
}

impl ToolCapability {
   const fn icon(&self) -> IconName {
        match self {
            ToolCapability::FileOperation => IconName::LetterText,
            ToolCapability::CodeReview => IconName::ChevronDown,
            ToolCapability::WebSearch => IconName::Search,
            ToolCapability::Calculation => IconName::Timer,
            ToolCapability::DataAnalysis => IconName::TimerReset,
            ToolCapability::ImageProcessing => IconName::Image,
        }
    }

   const fn label(&self) -> &'static str {
        match self {
            ToolCapability::FileOperation => "文件",
            ToolCapability::CodeReview => "代码",
            ToolCapability::WebSearch => "搜索",
            ToolCapability::Calculation => "计算",
            ToolCapability::DataAnalysis => "分析",
            ToolCapability::ImageProcessing => "图像",
        }
    }
}

// 简化的模型数据结构
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub is_selected: bool,
    pub capabilities: Vec<ModelCapability>, // 添加能力字段
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
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "sqb-chat-4.0".to_string(),
                        provider: "收钱吧".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
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
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "claude-3-haiku".to_string(),
                        provider: "Anthropic".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "claude-3-opus".to_string(),
                        provider: "Anthropic".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
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
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "gpt-4-turbo".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "gpt-3.5-turbo".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "gpt-4o".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Audio,
                            ModelCapability::Tools,
                        ],
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
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "文心一言-3.5".to_string(),
                        provider: "百度智能云".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "ERNIE-Bot-turbo".to_string(),
                        provider: "百度智能云".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
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
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "通义千问-Plus".to_string(),
                        provider: "阿里云".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "通义千问-Turbo".to_string(),
                        provider: "阿里云".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
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
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "混元-Standard".to_string(),
                        provider: "腾讯云".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "混元-Lite".to_string(),
                        provider: "腾讯云".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text],
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
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "豆包-Pro-4K".to_string(),
                        provider: "字节跳动".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "豆包-Lite-4K".to_string(),
                        provider: "字节跳动".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text],
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
                        capabilities: vec![
                            ModelCapability::Text,
                            ModelCapability::Vision,
                            ModelCapability::Tools,
                        ],
                    },
                    ModelInfo {
                        name: "GLM-4-Air".to_string(),
                        provider: "智谱AI".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    },
                    ModelInfo {
                        name: "GLM-3-Turbo".to_string(),
                        provider: "智谱AI".to_string(),
                        is_selected: false,
                        capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
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

// MCP工具信息
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub provider: String,
    pub is_selected: bool,
    pub capabilities: Vec<ToolCapability>,
    pub description: String,
}

// MCP工具提供商信息
#[derive(Debug, Clone)]
pub struct McpProviderInfo {
    pub name: String,
    pub tools: Vec<McpToolInfo>,
}

// MCP工具管理器
pub struct McpToolManager {
    pub providers: Vec<McpProviderInfo>,
}

impl McpToolManager {
    pub fn new() -> Self {
        let providers = vec![
            McpProviderInfo {
                name: "开发工具".to_string(),
                tools: vec![
                    McpToolInfo {
                        name: "代码审查助手".to_string(),
                        provider: "开发工具".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ToolCapability::CodeReview,
                            ToolCapability::FileOperation,
                        ],
                        description: "自动审查代码质量和安全性".to_string(),
                    },
                    McpToolInfo {
                        name: "Git操作工具".to_string(),
                        provider: "开发工具".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ToolCapability::FileOperation,
                            ToolCapability::CodeReview,
                        ],
                        description: "管理Git仓库和版本控制".to_string(),
                    },
                ],
            },
            McpProviderInfo {
                name: "数据处理".to_string(),
                tools: vec![
                    McpToolInfo {
                        name: "Excel处理器".to_string(),
                        provider: "数据处理".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ToolCapability::FileOperation,
                            ToolCapability::DataAnalysis,
                        ],
                        description: "处理和分析Excel文件".to_string(),
                    },
                    McpToolInfo {
                        name: "数据可视化".to_string(),
                        provider: "数据处理".to_string(),
                        is_selected: false,
                        capabilities: vec![
                            ToolCapability::DataAnalysis,
                            ToolCapability::ImageProcessing,
                        ],
                        description: "生成图表和数据可视化".to_string(),
                    },
                ],
            },
            McpProviderInfo {
                name: "办公工具".to_string(),
                tools: vec![
                    McpToolInfo {
                        name: "文档生成器".to_string(),
                        provider: "办公工具".to_string(),
                        is_selected: false,
                        capabilities: vec![ToolCapability::FileOperation],
                        description: "自动生成各类文档".to_string(),
                    },
                    McpToolInfo {
                        name: "邮件助手".to_string(),
                        provider: "办公工具".to_string(),
                        is_selected: false,
                        capabilities: vec![ToolCapability::WebSearch],
                        description: "智能邮件管理和回复".to_string(),
                    },
                ],
            },
        ];

        Self { providers }
    }

    pub fn toggle_tool_selection(&mut self, tool_name: &str) {
        for provider in &mut self.providers {
            for tool in &mut provider.tools {
                if tool.name == tool_name {
                    tool.is_selected = !tool.is_selected;
                    return;
                }
            }
        }
    }

    pub fn get_selected_tools(&self) -> Vec<String> {
        let mut selected = Vec::new();
        for provider in &self.providers {
            for tool in &provider.tools {
                if tool.is_selected {
                    selected.push(tool.name.clone());
                }
            }
        }
        selected
    }

    pub fn get_selected_count(&self) -> usize {
        self.get_selected_tools().len()
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

    // AI助手配置
    model_manager: ModelManager,
    mcp_tool_manager: McpToolManager,

    // 时间设置
    due_date_picker: Entity<DatePickerState>,
    reminder_date_picker: Entity<DatePickerState>,
    recurring_enabled: bool,
    recurring_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // 其他设置
    auto_execute: bool,
    enable_notifications: bool,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    // 添加上传文件列表
    uploaded_files: Vec<UploadedFile>,

    _subscriptions: Vec<Subscription>,
}

impl TodoThreadEdit {

    pub fn edit(todo:Todo,
        cx: &mut App,
    )  {
      cx.activate(true);
            let window_size = size(px(600.0), px(650.0));
            let window_bounds = Bounds::centered(None, window_size, cx);
            let options = WindowOptions {
                app_id: Some("x-todo-app".to_string()),
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(600.),
                    height: px(650.),
                }),
                kind: WindowKind::PopUp,
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                ..Default::default()
            };
            cx.create_normal_window(
                format!("xTodo-{}", todo.title),
                options,
                move |window, cx| Self::view(window, cx),
            );
    }

    pub fn add(
        cx: &mut App,
    )  {
      cx.activate(true);
            let window_size = size(px(600.0), px(650.0));
            let window_bounds = Bounds::centered(None, window_size, cx);
            let options = WindowOptions {
                app_id: Some("x-todo-app".to_string()),
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(600.),
                    height: px(650.),
                }),
                kind: WindowKind::PopUp,
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                ..Default::default()
            };
            cx.create_normal_window(
                "xTodo-Add",
                options,
                move |window, cx| Self::view(window, cx),
            );
    }
     fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 基本信息输入框
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入任务标题..."));

        let description_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("详细描述任务内容和要求...")
                .auto_grow(10, 10)
        });

        // 状态和优先级下拉框
        let status_dropdown =
            cx.new(|cx| DropdownState::new(TodoStatus::all(), Some(0), window, cx));

        let priority_dropdown =
            cx.new(|cx| DropdownState::new(TodoPriority::all(), Some(1), window, cx));

        // 模型管理器和工具管理器
        let model_manager = ModelManager::new();
        let mcp_tool_manager = McpToolManager::new();

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
            mcp_tool_manager,
            due_date_picker,
            reminder_date_picker,
            recurring_enabled: false,
            recurring_dropdown,
            auto_execute: false,
            enable_notifications: true,
            expanded_providers: Vec::new(),
            expanded_tool_providers: Vec::new(),
            uploaded_files: Vec::new(), // 初始化空文件列表
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
        let selected_tools = self.mcp_tool_manager.get_selected_tools(); // 改为获取选中的工具

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
            selected_tools, // 改为工具列表
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
            "选择模型".to_string()
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

    // 获取工具选择显示文本
    fn get_tool_display_text(&self, _cx: &App) -> String {
        let selected_tools = self.mcp_tool_manager.get_selected_tools();
        let selected_count = selected_tools.len();

        if selected_count == 0 {
            "选择工具集".to_string()
        } else if selected_count <= 2 {
            selected_tools.join(", ")
        } else {
            let first_two = selected_tools
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} 等{}个工具", first_two, selected_count)
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
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_providers.clone();

            // 创建手风琴组件并添加切换监听器
            let mut accordion = Accordion::new("model-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, todo_cx| {
                            todo_edit.toggle_accordion(open_indices, todo_cx);
                        });
                    }
                });

            for (provider_index, provider) in providers.iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_models = provider.models.clone();
                
                // 检查该供应商是否有被选中的模型
                let has_selected_models = provider_models.iter().any(|model| model.is_selected);
                
                // 检查当前供应商是否应该展开
                let is_expanded = has_selected_models || expanded_providers.contains(&provider_index);

                accordion = accordion.item(|item| {
                    item.open(is_expanded) // 根据选中状态和展开状态决定是否展开
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
                                            gpui::rgb(0xDCFCE7) // 有选中模型时使用绿色背景
                                        } else {
                                            gpui::rgb(0xEFF6FF) // 无选中模型时使用蓝色背景
                                        })
                                        .text_color(if has_selected_models {
                                            gpui::rgb(0x166534) // 绿色文字
                                        } else {
                                            gpui::rgb(0x1D4ED8) // 蓝色文字
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
                                        let model_name_for_event = model.name.clone();
                                        let checkbox_id = SharedString::new(format!(
                                            "model-{}-{}",
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
                                                                h_flex().gap_1().items_center().children(
                                                                    model.capabilities.iter().enumerate().map(
                                                                        |(cap_index, cap)| {
                                                                            let capability_unique_id = provider_index * 10000
                                                                                + model_index * 1000
                                                                                + cap_index;

                                                                            div()
                                                                                .id(("capability", capability_unique_id))
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
                .size(px(380.))
                .title("选择模型")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-models")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
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
                                    // 关闭抽屉
                                    window.close_drawer(cx);
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
        // 使用 Entity 来共享状态
        let todo_edit_entity = cx.entity().clone();

        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            // 从 entity 中读取当前的工具数据
            let providers = todo_edit_entity.read(drawer_cx).mcp_tool_manager.providers.clone();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_tool_providers.clone();

            // 创建手风琴组件并添加切换监听器
            let mut accordion = Accordion::new("tool-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, todo_cx| {
                            todo_edit.toggle_tool_accordion(open_indices, todo_cx);
                        });
                    }
                });

            for (provider_index, provider) in providers.iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_tools = provider.tools.clone();
                
                // 检查该供应商是否有被选中的工具
                let has_selected_tools = provider_tools.iter().any(|tool| tool.is_selected);
                
                // 检查当前供应商是否应该展开
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
                                            gpui::rgb(0xDCFCE7) // 有选中工具时使用绿色背景
                                        } else {
                                            gpui::rgb(0xFFF7ED) // 无选中工具时使用橙色背景
                                        })
                                        .text_color(if has_selected_tools {
                                            gpui::rgb(0x166534) // 绿色文字
                                        } else {
                                            gpui::rgb(0xEA580C) // 橙色文字
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
                                            "tool-{}-{}",
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
                                                                            .checked(tool.is_selected)
                                                                            .label(tool.name.clone())
                                                                            .on_click(
                                                                                move |_checked, _window, cx| {
                                                                                    let tool_name_to_toggle =
                                                                                        tool_name_for_event.clone();
                                                                                    
                                                                                    // 更新原始数据
                                                                                    todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                        todo_edit.mcp_tool_manager.toggle_tool_selection(&tool_name_to_toggle);
                                                                                        todo_cx.notify(); // 通知主界面更新
                                                                                    });

                                                                                    println!(
                                                                                        "切换工具选择: {}",
                                                                                        tool_name_to_toggle
                                                                                    );
                                                                                },
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
                                .when(provider_tools.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .p_4()
                                            .text_center()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x9CA3AF))
                                            .child("该类别暂无可用工具"),
                                    )
                                }),
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(380.))
                .title("选择工具集")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-tools")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    // 清空所有工具选择
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        for provider in &mut todo_edit.mcp_tool_manager.providers {
                                            for tool in &mut provider.tools {
                                                tool.is_selected = false;
                                            }
                                        }
                                        todo_cx.notify(); // 通知主界面更新
                                    });
                                    println!("清空所有工具选择");
                                    // 关闭抽屉
                                    window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }

    // 添加移除文件的方法
    fn remove_file(&mut self, file_path: &str, _window: &mut Window, cx: &mut Context<Self>) {
        self.uploaded_files.retain(|file| file.path != file_path);
        cx.notify();
    }

    // 添加处理文件拖拽的方法
    fn handle_file_drop(
        &mut self,
        external_paths: &ExternalPaths,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for path in external_paths.paths() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                let path_str = path.to_string_lossy().to_string();

                // 检查文件是否已经存在
                if !self.uploaded_files.iter().any(|f| f.path == path_str) {
                    // 获取文件大小（可选）
                    let file_size = std::fs::metadata(&path).ok().map(|metadata| metadata.len());

                    let uploaded_file = UploadedFile {
                        name: file_name.to_string(),
                        path: path_str,
                        size: file_size,
                    };

                    self.uploaded_files.push(uploaded_file);
                }
            }
        }
        cx.notify();
    }

    // 格式化文件大小的辅助方法
    fn format_file_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size_f = size as f64;
        let mut unit_index = 0;

        while size_f >= 1024.0 && unit_index < UNITS.len() - 1 {
            size_f /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size_f, UNITS[unit_index])
        }
    }
}

#[derive(Debug)]
struct TodoData {
    title: String,
    description: String,
    status: String,
    priority: String,
    selected_models: Vec<String>,
    selected_tools: Vec<String>, // 改为工具列表
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
                                v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .id("file-drop-zone")
                                            .min_h_24() // 改为最小高度，而不是固定高度
                                            .w_full()
                                            .border_2()
                                            .border_color(gpui::rgb(0xD1D5DB))
                                            .border_dashed()
                                            .rounded_lg()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .flex()
                                            .flex_col() // 改为垂直布局
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
                                                // 拖拽提示区域
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .p_4()
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
                                                    ),
                                            )
                                            // 文件列表区域（集成到拖拽区域内）
                                            .when(!self.uploaded_files.is_empty(), |this| {
                                                this.child(
                                                    div()
                                                        .border_t_1()
                                                        .border_color(gpui::rgb(0xE5E7EB))
                                                        .p_3()
                                                        .bg(gpui::rgb(0xF8F9FA))
                                                        .child(
                                                            v_flex()
                                                                .gap_2()
                                                                
                                                                .child(
                                                                    h_flex()
                                                                        .gap_2()
                                                                        .flex_wrap()
                                                                        .children(
                                                                            self.uploaded_files.iter().enumerate().map(|(index, file)| {
                                                                                let file_path_for_remove = file.path.clone();
                                                                                let file_name = file.name.clone();
                                                                                
                                                                                // 截断文件名，最大显示15个字符
                                                                                let display_name = if file.name.len() > 15 {
                                                                                    format!("{}...", &file.name[..12])
                                                                                } else {
                                                                                    file.name.clone()
                                                                                };
                                                                                
                                                                                div()
                                                                                    .id(("uploaded-file", index))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .gap_1()
                                                                                    .px_2()
                                                                                    .py_1()
                                                                                    .max_w_32() // 限制最大宽度
                                                                                    .bg(gpui::rgb(0xF3F4F6))
                                                                                    .border_1()
                                                                                    .border_color(gpui::rgb(0xE5E7EB))
                                                                                    .rounded_md()
                                                                                    .hover(|style| style.bg(gpui::rgb(0xE5E7EB)))
                                                                                    .tooltip({
                                                                                        let file_name = file_name.clone();
                                                                                        move |window, cx| {
                                                                                            let file_name = file_name.clone();
                                                                                            Tooltip::element( move|_, _| {
                                                                                                Label::new(file_name.clone())
                                                                                            })
                                                                                            .build(window, cx)
                                                                                        }
                                                                                    })
                                                                                    .child(
                                                                                        
                                                                                        div()
                                                                                            .text_xs()
                                                                                            .font_medium()
                                                                                            .text_color(gpui::rgb(0x374151))
                                                                                            .flex_1()
                                                                                            .overflow_hidden()
                                                                                            .child(display_name),
                                                                                    )
                                                                                    .child(
                                                                                        Button::new(SharedString::new(format!("remove-file-{}", index)))
                                                                                            .ghost()
                                                                                            .xsmall()
                                                                                            .icon(IconName::X)
                                                                                            .text_color(gpui::rgb(0x9CA3AF))
                                                                                           
                                                                                            .p_0()
                                                                                            .min_w_4()
                                                                                            .h_4()
                                                                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                                                                this.remove_file(&file_path_for_remove, window, cx);
                                                                                            })),
                                                                                    )
                                                                            })
                                                                        ),
                                                                ),
                                                        )
                                                )
                                            })
                                            .drag_over(|style, _path: &ExternalPaths, _window, _cx| {
                                                style
                                                    .border_color(gpui::rgb(0x3B82F6))
                                                    .bg(gpui::rgb(0xF0F9FF))
                                            })
                                            .on_drop(cx.listener(|this, external_paths: &ExternalPaths, window, cx| {
                                                this.handle_file_drop(external_paths, window, cx);
                                                cx.stop_propagation();
                                            }))
                                            .on_click(cx.listener(|_, _, _, cx| {
                                                println!("点击上传文件");
                                                cx.notify();
                                            })),
                                    )
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(h_flex()
                                            .justify_between()
                                            .items_center()
                                            .child(Self::section_title("助手配置"))
                                            .child(
                                                Checkbox::new("push-feishu-button")
                                                    .label("推送到飞书")
                                                    .checked(true)
                                                    .on_click(cx.listener(|view, _, _, cx| {
                                                        // view.disabled = !view.disabled;
                                                        cx.notify();
                                                    })),
                                            ),)
                            .child(
                                h_flex()
                                    .gap_2().justify_start()
                                    .items_center()
                                    // .child(
                                    //     div()
                                    //         .text_sm()
                                    //         .text_color(gpui::rgb(0x6B7280))
                                    //         .min_w_24()
                                    //         .child("模型选择"),
                                    // )
                                    .child(
                                        div().justify_start().child(
                                            Button::new("show-drawer-left")
                                                .label({
                                                    let display_text =
                                                        self.get_model_display_text(cx);
                                                    if display_text == "选择模型" {
                                                        display_text
                                                    } else {
                                                        display_text
                                                    }
                                                }).ghost().xsmall()
                                                .justify_center()
                                                .text_color(
                                                    if self.get_model_display_text(cx)
                                                        == "选择模型"
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
                                    ).child(
                               h_flex().max_w_32().child( DatePicker::new(&self.due_date_picker)
                                    .placeholder("截止日期")
                                    .cleanable()
                                    .presets(due_date_presets.clone())
                                    .small())
                            ),
                            ).child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    // .child(
                                    //     div()
                                    //         .text_sm()
                                    //         .text_color(gpui::rgb(0x6B7280))
                                    //         .min_w_24()
                                    //         .child("工具集"),
                                    // )
                                    .child(
                                        div().justify_start().child(
                                            Button::new("show-tool-drawer-left")
                                                .label({
                                                    let display_text = self.get_tool_display_text(cx);
                                                    if display_text == "选择工具集" {
                                                        display_text
                                                    } else {
                                                        display_text
                                                    }
                                                })
                                                .ghost()
                                                .xsmall()
                                                .justify_center()
                                                .text_color(
                                                    if self.get_tool_display_text(cx) == "选择工具集" {
                                                        gpui::rgb(0x9CA3AF)
                                                    } else {
                                                        gpui::rgb(0x374151)
                                                    },
                                                )
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.open_tool_drawer_at(Placement::Left, window, cx)
                                                })),
                                        ),
                                    ).child(
                                h_flex()
                                    .gap_2()
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
                            )
                            
                    )
                    
            )
            .child(
                h_flex().items_center().justify_center().pt_2().child(
                    h_flex().gap_1().child(
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
