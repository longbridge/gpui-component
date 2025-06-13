use crate::app::AppState;
use crate::models::todo_item::{TodoItem as TodoItemModel, TodoPriority as ModelTodoPriority};
use crate::ui::{components::ViewKit, AppExt};
use chrono::Utc;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    dropdown::{Dropdown, DropdownState},
    input::{InputEvent, InputState, TextInput},
    label::Label,
    switch::Switch,
    tooltip::Tooltip,
    *,
};

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

    fn to_model_priority(&self) -> ModelTodoPriority {
        match self {
            TodoPriority::Low => ModelTodoPriority::Low,
            TodoPriority::Medium => ModelTodoPriority::Medium,
            TodoPriority::High => ModelTodoPriority::High,
            TodoPriority::Urgent => ModelTodoPriority::Urgent,
        }
    }

    fn from_model_priority(priority: &ModelTodoPriority) -> Self {
        match priority {
            ModelTodoPriority::Low => TodoPriority::Low,
            ModelTodoPriority::Medium => TodoPriority::Medium,
            ModelTodoPriority::High => TodoPriority::High,
            ModelTodoPriority::Urgent => TodoPriority::Urgent,
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

// 简化的模型结构
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub is_selected: bool,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub models: Vec<ModelInfo>,
}

pub struct ModelManager {
    pub providers: Vec<ProviderInfo>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            providers: vec![ProviderInfo {
                name: "OpenAI".to_string(),
                models: vec![
                    ModelInfo {
                        name: "gpt-3.5-turbo".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                    },
                    ModelInfo {
                        name: "gpt-4".to_string(),
                        provider: "OpenAI".to_string(),
                        is_selected: false,
                    },
                ],
            }],
        }
    }

    pub fn get_selected_models(&self) -> Vec<String> {
        self.providers
            .iter()
            .flat_map(|provider| &provider.models)
            .filter(|model| model.is_selected)
            .map(|model| model.name.clone())
            .collect()
    }
}

// 简化的工具结构
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub is_selected: bool,
}

#[derive(Debug, Clone)]
pub struct ToolProviderInfo {
    pub name: String,
    pub tools: Vec<ToolInfo>,
}

pub struct McpToolManager {
    pub providers: Vec<ToolProviderInfo>,
}

impl McpToolManager {
    pub fn new() -> Self {
        Self {
            providers: vec![ToolProviderInfo {
                name: "文件工具".to_string(),
                tools: vec![ToolInfo {
                    name: "文件读取".to_string(),
                    description: "读取和分析文件内容".to_string(),
                    is_selected: false,
                }],
            }],
        }
    }

    pub fn get_selected_tools(&self) -> Vec<String> {
        self.providers
            .iter()
            .flat_map(|provider| &provider.tools)
            .filter(|tool| tool.is_selected)
            .map(|tool| tool.name.clone())
            .collect()
    }
}

pub struct TodoThreadEdit {
    focus_handle: FocusHandle,

    // 编辑状态
    edit_todo_id: Option<u32>,

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
    pub fn edit(todo_id: u32, cx: &mut App) {
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
        cx.create_normal_window("xTodo-Edit", options, move |window, cx| {
            let view = Self::view(window, cx);
            // 设置编辑模式并加载Todo数据
            view.update(cx, |edit_view, inner_cx| {
                edit_view.load_todo_for_edit(todo_id, inner_cx);
            });
            view
        });
    }

    pub fn add(cx: &mut App) {
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
        cx.create_normal_window("xTodo-Add", options, move |window, cx| {
            Self::view(window, cx)
        });
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
            edit_todo_id: None,
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
            uploaded_files: Vec::new(),
            _subscriptions,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    // 加载Todo数据用于编辑
    fn load_todo_for_edit(&mut self, todo_id: u32, cx: &mut Context<Self>) {
        self.edit_todo_id = Some(todo_id);

        let app_state = AppState::global(cx);
        if let Ok(todo) = app_state.todo_service.get_todo_by_id(todo_id) {
            // 设置标题和描述
            self.title_input.update(cx, |input, cx| {
                input.set_value(&todo.title, cx);
            });

            if let Some(description) = &todo.description {
                self.description_input.update(cx, |input, cx| {
                    input.set_value(description, cx);
                });
            }

            // 设置优先级下拉框
            let priority_index = match todo.priority {
                ModelTodoPriority::Low => 0,
                ModelTodoPriority::Medium => 1,
                ModelTodoPriority::High => 2,
                ModelTodoPriority::Urgent => 3,
            };
            self.priority_dropdown.update(cx, |dropdown, cx| {
                dropdown.set_selected_index(Some(priority_index), cx);
            });

            // 设置状态下拉框（根据完成状态）
            let status_index = if todo.completed { 2 } else { 0 }; // 已完成或待办
            self.status_dropdown.update(cx, |dropdown, cx| {
                dropdown.set_selected_index(Some(status_index), cx);
            });
        }

        cx.notify();
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn save(&mut self, _: &Save, _window: &mut Window, cx: &mut Context<Self>) {
        let title = self.title_input.read(cx).value().to_string();
        let description = self.description_input.read(cx).value().to_string();
        let description = if description.trim().is_empty() {
            None
        } else {
            Some(description)
        };

        // 获取优先级
        let priority_index = self
            .priority_dropdown
            .read(cx)
            .selected_index()
            .unwrap_or(1);
        let priority = match priority_index {
            0 => ModelTodoPriority::Low,
            1 => ModelTodoPriority::Medium,
            2 => ModelTodoPriority::High,
            3 => ModelTodoPriority::Urgent,
            _ => ModelTodoPriority::Medium,
        };

        let app_state = AppState::global(cx);

        let result = if let Some(todo_id) = self.edit_todo_id {
            // 编辑模式：更新现有Todo
            let update_result = app_state
                .todo_service
                .update_todo_title(todo_id, title)
                .and_then(|_| {
                    app_state
                        .todo_service
                        .update_todo_description(todo_id, description)
                })
                .and_then(|_| {
                    app_state
                        .todo_service
                        .update_todo_priority(todo_id, priority)
                });

            match update_result {
                Ok(_) => {
                    println!("Todo更新成功: ID {}", todo_id);
                    // 关闭窗口
                    cx.emit(WindowEvent::CloseWindow);
                }
                Err(e) => {
                    eprintln!("Todo更新失败: {}", e);
                }
            }
        } else {
            // 新建模式：创建新Todo
            match app_state
                .todo_service
                .create_todo_with_details(title, description, priority)
            {
                Ok(todo) => {
                    println!("Todo创建成功: {:?}", todo);
                    // 关闭窗口
                    cx.emit(WindowEvent::CloseWindow);
                }
                Err(e) => {
                    eprintln!("Todo创建失败: {}", e);
                }
            }
        };

        cx.notify();
    }

    fn cancel(&mut self, _: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
        println!("取消编辑");
        cx.emit(WindowEvent::CloseWindow);
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo_id) = self.edit_todo_id {
            let app_state = AppState::global(cx);
            match app_state.todo_service.delete_todo(todo_id) {
                Ok(_) => {
                    println!("Todo删除成功: ID {}", todo_id);
                    cx.emit(WindowEvent::CloseWindow);
                }
                Err(e) => {
                    eprintln!("Todo删除失败: {}", e);
                }
            }
        }
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
}

impl ViewKit for TodoThreadEdit {
    fn title() -> &'static str {
        "任务编辑"
    }

    fn description() -> &'static str {
        "创建和编辑任务"
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_edit_mode = self.edit_todo_id.is_some();
        let window_title = if is_edit_mode {
            "编辑任务"
        } else {
            "新建任务"
        };

        div()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::delete))
            .size_full()
            .p_4()
            .child(
                v_flex()
                    .gap_4()
                    .child(
                        // 标题栏
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(div().text_xl().font_semibold().child(window_title))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .when(is_edit_mode, |this| {
                                        this.child(
                                            Button::new("delete-button")
                                                .label("删除")
                                                .variant(ButtonVariant::Destructive)
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.delete(&Delete, window, cx);
                                                })),
                                        )
                                    })
                                    .child(
                                        Button::new("cancel-button")
                                            .label("取消")
                                            .variant(ButtonVariant::Ghost)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.cancel(&Cancel, window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("save-button")
                                            .label(if is_edit_mode { "保存" } else { "创建" })
                                            .variant(ButtonVariant::Default)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.save(&Save, window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        // 主要表单内容
                        v_flex()
                            .gap_4()
                            .child(
                                // 基本信息
                                v_flex()
                                    .gap_3()
                                    .child(Label::new("任务标题").text_sm().font_medium())
                                    .child(
                                        TextInput::new("title-input")
                                            .state(self.title_input.clone())
                                            .placeholder("输入任务标题..."),
                                    )
                                    .child(Label::new("任务描述").text_sm().font_medium())
                                    .child(
                                        TextInput::new("description-input")
                                            .state(self.description_input.clone())
                                            .placeholder("详细描述任务内容...")
                                            .auto_grow(10, 10),
                                    ),
                            )
                            .child(
                                // 优先级设置
                                h_flex()
                                    .gap_4()
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .gap_2()
                                            .child(Label::new("状态").text_sm().font_medium())
                                            .child(
                                                Dropdown::new("status-dropdown")
                                                    .state(self.status_dropdown.clone())
                                                    .placeholder("选择状态"),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .gap_2()
                                            .child(Label::new("优先级").text_sm().font_medium())
                                            .child(
                                                Dropdown::new("priority-dropdown")
                                                    .state(self.priority_dropdown.clone())
                                                    .placeholder("选择优先级"),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}
