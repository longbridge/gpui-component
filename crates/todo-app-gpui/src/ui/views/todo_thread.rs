use std::{cell::Cell, rc::Rc};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{InputEvent, InputState, TextInput},
    scroll::{ Scrollbar, ScrollbarState},
    *,
};
use crate::ui::{components::ViewKit};
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

// 模型能力
#[derive(Debug, Clone)]
pub enum ModelCapability {
    Text,
    Vision,
    Audio,
    Tools,
}

impl ModelCapability {
    fn icon(&self) -> IconName {
        match self {
            ModelCapability::Text => IconName::LetterText,
            ModelCapability::Vision => IconName::Eye,
            ModelCapability::Audio => IconName::Mic,
            ModelCapability::Tools => IconName::Wrench,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ModelCapability::Text => "文本",
            ModelCapability::Vision => "视觉",
            ModelCapability::Audio => "音频",
            ModelCapability::Tools => "工具",
        }
    }
}

// 模型信息
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub is_selected: bool,
    pub capabilities: Vec<ModelCapability>,
}

// 提供商信息
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
    fn icon(&self) -> IconName {
        match self {
            ToolCapability::FileOperation => IconName::LetterText,
            ToolCapability::CodeReview => IconName::ChevronDown,
            ToolCapability::WebSearch => IconName::Search,
            ToolCapability::Calculation => IconName::Timer,
            ToolCapability::DataAnalysis => IconName::TimerReset,
            ToolCapability::ImageProcessing => IconName::Image,
        }
    }

    fn label(&self) -> &'static str {
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
                name: "网络工具".to_string(),
                tools: vec![
                    McpToolInfo {
                        name: "网页爬虫".to_string(),
                        provider: "网络工具".to_string(),
                        is_selected: false,
                        capabilities: vec![ToolCapability::WebSearch, ToolCapability::DataAnalysis],
                        description: "自动抓取和分析网页数据".to_string(),
                    },
                    McpToolInfo {
                        name: "SEO分析器".to_string(),
                        provider: "网络工具".to_string(),
                        is_selected: false,
                        capabilities: vec![ToolCapability::WebSearch, ToolCapability::DataAnalysis],
                        description: "网站SEO优化分析".to_string(),
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

pub struct TodoThreadChat {
    focus_handle: FocusHandle,

    // 聊天功能
    chat_messages: Vec<ChatMessage>,
    chat_input: Entity<InputState>,
    is_loading: bool,
    scroll_handle: ScrollHandle,
    scroll_size: gpui::Size<Pixels>,
    scroll_state: Rc<Cell<ScrollbarState>>,

    // AI助手配置 - 改为管理器
    model_manager: ModelManager,
    mcp_tool_manager: McpToolManager,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    _subscriptions: Vec<Subscription>,
}

impl TodoThreadChat {
    pub fn open(todo:Todo,
        cx: &mut App) {
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
            cx.create_normal_window(
                format!("xTodo-{}", todo.title),
                options,
                move |window, cx| Self::view(window, cx),
            );
        }

     fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 聊天输入框 - 多行支持
        let chat_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入消息与AI助手对话...，按Ctrl+Enter发送，按ESC清除输入框")
                .clean_on_escape()
                .multi_line()
                .auto_grow(1, 6)
        });

        // AI助手配置 - 使用管理器
        let model_manager = ModelManager::new();
        let mcp_tool_manager = McpToolManager::new();

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
            chat_messages,
            chat_input,
            is_loading: false,
            scroll_handle: ScrollHandle::new(),
            model_manager,
            mcp_tool_manager,
            expanded_providers: Vec::new(),
            expanded_tool_providers: Vec::new(),
            _subscriptions,
            scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_size: gpui::Size::default(),
        }
    }

    // 获取模型选择显示文本
    fn get_model_display_text(&self, _cx: &App) -> String {
        let selected_models = self.model_manager.get_selected_models();
        let selected_count = selected_models.len();

        if selected_count == 0 {
            "选择模型".to_string()
        } else if selected_count == 1 {
            selected_models[0].clone()
        } else {
            format!("{} 等{}个模型", selected_models[0], selected_count)
        }
    }

    // 获取工具选择显示文本
    fn get_tool_display_text(&self, _cx: &App) -> String {
        let selected_tools = self.mcp_tool_manager.get_selected_tools();
        let selected_count = selected_tools.len();

        if selected_count == 0 {
            "选择工具集".to_string()
        } else if selected_count == 1 {
            selected_tools[0].clone()
        } else {
            format!("{} 等{}个工具", selected_tools[0], selected_count)
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

    fn open_model_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let todo_edit_entity = cx.entity().clone();

        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            let providers = todo_edit_entity.read(drawer_cx).model_manager.providers.clone();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_providers.clone();

            let mut accordion = Accordion::new("chat-model-providers")
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
                
                let has_selected_models = provider_models.iter().any(|model| model.is_selected);
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
                                        let model_name_for_event = model.name.clone();
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
                                                                    .checked(model.is_selected)
                                                                    .label(model.name.clone())
                                                                    .on_click(
                                                                        move |_checked, _window, cx| {
                                                                            let model_name_to_toggle =
                                                                                model_name_for_event.clone();
                                                                            
                                                                            todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                todo_edit.model_manager.toggle_model_selection(&model_name_to_toggle);
                                                                                todo_cx.notify();
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
                            Button::new("clear-all-chat-models")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        for provider in &mut todo_edit.model_manager.providers {
                                            for model in &mut provider.models {
                                                model.is_selected = false;
                                            }
                                        }
                                        todo_cx.notify();
                                    });
                                    println!("清空所有模型选择");
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
        let todo_edit_entity = cx.entity().clone();

        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            let providers = todo_edit_entity.read(drawer_cx).mcp_tool_manager.providers.clone();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_tool_providers.clone();

            let mut accordion = Accordion::new("chat-tool-providers")
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
                
                let has_selected_tools = provider_tools.iter().any(|tool| tool.is_selected);
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
                                                                            .checked(tool.is_selected)
                                                                            .label(tool.name.clone())
                                                                            .on_click(
                                                                                move |_checked, _window, cx| {
                                                                                    let tool_name_to_toggle =
                                                                                        tool_name_for_event.clone();
                                                                                    
                                                                                    todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                        todo_edit.mcp_tool_manager.toggle_tool_selection(&tool_name_to_toggle);
                                                                                        todo_cx.notify();
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
                            Button::new("clear-all-chat-tools")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        for provider in &mut todo_edit.mcp_tool_manager.providers {
                                            for tool in &mut provider.tools {
                                                tool.is_selected = false;
                                            }
                                        }
                                        todo_cx.notify();
                                    });
                                    println!("清空所有工具选择");
                                    window.close_drawer(cx);
                                }),
                        ),
                )
        });
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
        let selected_models = self.model_manager.get_selected_models();
        let selected_tools = self.mcp_tool_manager.get_selected_tools();

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
                selected_model.as_deref().unwrap_or("默认")
            ),
        };

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
        Self::view(window, cx)
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
                // 中间区域：模型和工具选择 - 改为按钮
                h_flex()
                    .items_center()
                    .justify_around()
                    .gap_2()
                    .p_2()
                    .border_t_1()
                    .border_b_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .bg(gpui::rgb(0xF9FAFB))
                    .child(
                        h_flex().justify_start().items_center().gap_2().child(
                            Button::new("show-chat-model-drawer")
                                .label({
                                    let display_text = self.get_model_display_text(cx);
                                    if display_text == "选择模型" {
                                        display_text
                                    } else {
                                        display_text
                                    }
                                })
                                .ghost()
                                .xsmall()
                                .justify_center()
                                .text_color(if self.get_model_display_text(cx) == "选择AI模型" {
                                    gpui::rgb(0x9CA3AF)
                                } else {
                                    gpui::rgb(0x374151)
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_model_drawer_at(Placement::Left, window, cx)
                                })),
                        ),
                    )
                    .child(
                        h_flex().justify_start().items_center().gap_2().child(
                            Button::new("show-chat-tool-drawer")
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
                                .text_color(if self.get_tool_display_text(cx) == "选择工具集" {
                                    gpui::rgb(0x9CA3AF)
                                } else {
                                    gpui::rgb(0x374151)
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_tool_drawer_at(Placement::Left, window, cx)
                                })),
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
