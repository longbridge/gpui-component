use crate::ui::components::{section::section, ViewKit};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonGroup, ButtonVariant, ButtonVariants as _},
    dropdown::{Dropdown, DropdownState},
    input::{InputEvent, InputState, TextInput},
    switch::Switch,
    tab::{Tab, TabBar},
    *,
};

actions!(
    mcp_provider,
    [
        Tab1,
        TabPrev,
        AddMcpProvider,
        SaveMcpProvider,
        DeleteMcpProvider,
        CancelEdit,
    ]
);

const CONTEXT: &str = "McpProvider";

#[derive(Debug, Clone)]
pub enum McpTransport {
    Stdio,
    Http,
    WebSocket,
}

impl McpTransport {
    fn as_str(&self) -> &'static str {
        match self {
            McpTransport::Stdio => "Stdio",
            McpTransport::Http => "HTTP",
            McpTransport::WebSocket => "WebSocket",
        }
    }

    fn all() -> Vec<SharedString> {
        vec!["Stdio".into(), "HTTP".into(), "WebSocket".into()]
    }
}

#[derive(Debug, Clone)]
pub enum McpCapability {
    Resources,
    Tools,
    Prompts,
    Logging,
}

impl McpCapability {
    fn icon(&self) -> IconName {
        match self {
            McpCapability::Resources => IconName::Database,
            McpCapability::Tools => IconName::Wrench,
            McpCapability::Prompts => IconName::SquareTerminal,
            McpCapability::Logging => IconName::LetterText,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            McpCapability::Resources => "资源",
            McpCapability::Tools => "工具",
            McpCapability::Prompts => "提示",
            McpCapability::Logging => "日志",
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpResource {
    uri: String,
    name: String,
    description: String,
    mime_type: Option<String>,
    subscribable: bool, // 是否支持订阅
    subscribed: bool,   // 当前是否已订阅
}

impl Default for McpResource {
    fn default() -> Self {
        Self {
            uri: String::new(),
            name: String::new(),
            description: String::new(),
            mime_type: None,
            subscribable: true,
            subscribed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpTool {
    name: String,
    description: String,
    parameters: Vec<McpParameter>,
}

#[derive(Debug, Clone)]
pub struct McpParameter {
    name: String,
    param_type: String,
    description: String,
    required: bool,
}

#[derive(Debug, Clone)]
pub struct McpPrompt {
    name: String,
    description: String,
    arguments: Vec<McpArgument>,
}

#[derive(Debug, Clone)]
pub struct McpArgument {
    name: String,
    description: String,
    required: bool,
}

#[derive(Debug, Clone)]
pub struct McpProviderInfo {
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    transport: McpTransport,
    enabled: bool,
    capabilities: Vec<McpCapability>,
    description: String,
    resources: Vec<McpResource>,
    tools: Vec<McpTool>,
    prompts: Vec<McpPrompt>,
    env_vars: std::collections::HashMap<String, String>,
}

impl Default for McpProviderInfo {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            command: String::new(),
            args: Vec::new(),
            transport: McpTransport::Stdio,
            enabled: true,
            capabilities: vec![McpCapability::Resources, McpCapability::Tools],
            description: String::new(),
            resources: vec![
                McpResource {
                    uri: "file:///home/user/documents".to_string(),
                    name: "文档文件夹".to_string(),
                    description: "用户文档目录访问".to_string(),
                    mime_type: Some("inode/directory".to_string()),
                    subscribable: true,
                    subscribed: false,
                },
                McpResource {
                    uri: "file:///home/user/config.json".to_string(),
                    name: "配置文件".to_string(),
                    description: "应用配置文件".to_string(),
                    mime_type: Some("application/json".to_string()),
                    subscribable: true,
                    subscribed: false,
                },
            ],
            tools: vec![
                McpTool {
                    name: "read_file".to_string(),
                    description: "读取指定文件的内容".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "path".to_string(),
                            param_type: "string".to_string(),
                            description: "要读取的文件路径".to_string(),
                            required: true,
                        },
                        McpParameter {
                            name: "encoding".to_string(),
                            param_type: "string".to_string(),
                            description: "文件编码格式".to_string(),
                            required: false,
                        },
                    ],
                },
                McpTool {
                    name: "write_file".to_string(),
                    description: "写入内容到指定文件".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "path".to_string(),
                            param_type: "string".to_string(),
                            description: "目标文件路径".to_string(),
                            required: true,
                        },
                        McpParameter {
                            name: "content".to_string(),
                            param_type: "string".to_string(),
                            description: "要写入的内容".to_string(),
                            required: true,
                        },
                    ],
                },
            ],
            prompts: vec![
                McpPrompt {
                    name: "code_review".to_string(),
                    description: "对代码进行审查和建议".to_string(),
                    arguments: vec![
                        McpArgument {
                            name: "code".to_string(),
                            description: "要审查的代码内容".to_string(),
                            required: true,
                        },
                        McpArgument {
                            name: "language".to_string(),
                            description: "编程语言类型".to_string(),
                            required: false,
                        },
                    ],
                },
                McpPrompt {
                    name: "explain_concept".to_string(),
                    description: "解释技术概念".to_string(),
                    arguments: vec![McpArgument {
                        name: "concept".to_string(),
                        description: "要解释的概念".to_string(),
                        required: true,
                    }],
                },
            ],
            env_vars: std::collections::HashMap::from([
                (
                    "PATH".to_string(),
                    "/usr/local/bin:/usr/bin:/bin".to_string(),
                ),
                ("NODE_ENV".to_string(), "production".to_string()),
            ]),
        }
    }
}

// 用于存储每个Provider的编辑状态输入框
#[derive(Clone)]
struct ProviderInputs {
    name_input: Entity<InputState>,
    command_input: Entity<InputState>,
    args_input: Entity<InputState>,
    description_input: Entity<InputState>,
    env_input: Entity<InputState>,
    transport_dropdown: Entity<DropdownState<Vec<SharedString>>>,
}

pub struct McpProvider {
    focus_handle: FocusHandle,
    providers: Vec<McpProviderInfo>,
    expanded_providers: Vec<usize>,
    active_capability_tabs: std::collections::HashMap<usize, usize>,
    editing_provider: Option<usize>,
    // 每个Provider的编辑状态输入框
    provider_inputs: std::collections::HashMap<usize, ProviderInputs>,
    _subscriptions: Vec<Subscription>,
}

impl ViewKit for McpProvider {
    fn title() -> &'static str {
        "工具集"
    }

    fn description() -> &'static str {
        "配置和管理MCP服务组件"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl McpProvider {
    pub fn init(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
            KeyBinding::new("tab", Tab1, Some(CONTEXT)),
            KeyBinding::new("ctrl-n", AddMcpProvider, Some(CONTEXT)),
            KeyBinding::new("escape", CancelEdit, Some(CONTEXT)),
        ])
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 初始化一些示例数据
        let mut filesystem_provider = McpProviderInfo::default();
        filesystem_provider.name = "文件系统".to_string();
        filesystem_provider.command = "node".to_string();
        filesystem_provider.args = vec!["filesystem-server.js".to_string()];
        filesystem_provider.description = "提供文件系统访问功能".to_string();
        filesystem_provider.capabilities = vec![McpCapability::Resources, McpCapability::Tools];

        let mut database_provider = McpProviderInfo::default();
        database_provider.name = "数据库连接".to_string();
        database_provider.command = "python".to_string();
        database_provider.args = vec!["-m", "mcp_database", "--host", "localhost"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        database_provider.description = "提供数据库查询和操作功能".to_string();
        database_provider.capabilities = vec![
            McpCapability::Resources,
            McpCapability::Tools,
            McpCapability::Logging,
        ];
        database_provider.tools = vec![McpTool {
            name: "execute_query".to_string(),
            description: "执行SQL查询语句".to_string(),
            parameters: vec![
                McpParameter {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    description: "SQL查询语句".to_string(),
                    required: true,
                },
                McpParameter {
                    name: "database".to_string(),
                    param_type: "string".to_string(),
                    description: "目标数据库名称".to_string(),
                    required: false,
                },
            ],
        }];
        database_provider.resources = vec![McpResource {
            uri: "db://localhost:5432/main".to_string(),
            name: "主数据库".to_string(),
            description: "主要的业务数据库连接".to_string(),
            mime_type: Some("application/sql".to_string()),
            subscribable: true,
            subscribed: false,
        }];

        Self {
            focus_handle: cx.focus_handle(),
            providers: vec![filesystem_provider, database_provider],
            expanded_providers: vec![],
            active_capability_tabs: std::collections::HashMap::new(),
            editing_provider: None,
            provider_inputs: std::collections::HashMap::new(),
            _subscriptions: vec![],
        }
    }

    fn tab(&mut self, _: &Tab1, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn add_mcp_provider(
        &mut self,
        _: &AddMcpProvider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let new_index = self.providers.len();
        self.providers.push(McpProviderInfo::default());
        self.expanded_providers.push(new_index);
        self.start_editing(new_index, window, cx);
        cx.notify();
    }

    fn cancel_edit(&mut self, _: &CancelEdit, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editing_index) = self.editing_provider {
            // 如果是新添加的空Provider，删除它
            if let Some(provider) = self.providers.get(editing_index) {
                if provider.name.is_empty() && provider.command.is_empty() {
                    self.providers.remove(editing_index);
                    self.expanded_providers.retain(|&i| i != editing_index);
                    self.expanded_providers = self
                        .expanded_providers
                        .iter()
                        .map(|&i| if i > editing_index { i - 1 } else { i })
                        .collect();
                }
            }
            // 清理输入框
            self.provider_inputs.remove(&editing_index);
        }
        self.editing_provider = None;
        cx.notify();
    }

    fn save_mcp_provider(
        &mut self,
        _: &SaveMcpProvider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.editing_provider {
            if let (Some(provider), Some(inputs)) = (
                self.providers.get_mut(index),
                self.provider_inputs.get(&index),
            ) {
                provider.name = inputs.name_input.read(cx).value().to_string();
                provider.command = inputs.command_input.read(cx).value().to_string();
                provider.description = inputs.description_input.read(cx).value().to_string();

                // 解析启动参数
                let args_text = inputs.args_input.read(cx).value().to_string();
                provider.args = args_text
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                // 解析环境变量
                let env_text = inputs.env_input.read(cx).value().to_string();
                provider.env_vars.clear();
                for line in env_text.lines() {
                    let line = line.trim();
                    if !line.is_empty() && line.contains('=') {
                        if let Some((key, value)) = line.split_once('=') {
                            provider
                                .env_vars
                                .insert(key.trim().to_string(), value.trim().to_string());
                        }
                    }
                }

                if let Some(selected) = inputs.transport_dropdown.read(cx).selected_value() {
                    provider.transport = match selected.as_ref() {
                        "Stdio" => McpTransport::Stdio,
                        "HTTP" => McpTransport::Http,
                        "WebSocket" => McpTransport::WebSocket,
                        _ => McpTransport::Stdio,
                    };
                }
            }
        }

        // 清理编辑状态
        if let Some(index) = self.editing_provider {
            self.provider_inputs.remove(&index);
        }
        self.editing_provider = None;
        cx.notify();
    }

    fn delete_mcp_provider(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let provider_name = if let Some(provider) = self.providers.get(index) {
            provider.name.clone()
        } else {
            return;
        };

        let entity = cx.entity().downgrade();

        window.open_modal(cx, move |modal, _, _| {
            let entity = entity.clone();
            modal
                .confirm()
                .child(format!(
                    "确定要删除MCP服务 \"{}\" 吗？\n\n此操作无法撤销。",
                    provider_name
                ))
                .button_props(
                    gpui_component::modal::ModalButtonProps::default()
                        .cancel_text("取消")
                        .cancel_variant(ButtonVariant::Secondary)
                        .ok_text("删除")
                        .ok_variant(ButtonVariant::Danger),
                )
                .on_ok(move |_, window, cx| {
                    if let Some(entity) = entity.upgrade() {
                        entity.update(cx, |this, cx| {
                            this.confirm_delete_mcp_provider(index, window, cx);
                        });
                    }
                    true
                })
                .on_cancel(|_, window, cx| {
                    window.push_notification("已取消删除操作", cx);
                    true
                })
        });
    }

    fn confirm_delete_mcp_provider(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index < self.providers.len() {
            let provider_name = self.providers[index].name.clone();
            self.providers.remove(index);
            self.provider_inputs.remove(&index);

            // 更新展开状态
            self.expanded_providers.retain(|&i| i != index);
            self.expanded_providers = self
                .expanded_providers
                .iter()
                .map(|&i| if i > index { i - 1 } else { i })
                .collect();

            // 如果正在编辑被删除的提供商，清除编辑状态
            if self.editing_provider == Some(index) {
                self.editing_provider = None;
            } else if let Some(editing) = self.editing_provider {
                if editing > index {
                    self.editing_provider = Some(editing - 1);
                }
            }

            window.push_notification(format!("已成功删除MCP服务 \"{}\"", provider_name), cx);
            cx.notify();
        }
    }

    fn toggle_provider_enabled(
        &mut self,
        index: usize,
        enabled: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(provider) = self.providers.get_mut(index) {
            provider.enabled = enabled;

            // 如果禁用提供商，自动关闭其 accordion
            if !enabled {
                self.expanded_providers.retain(|&i| i != index);
            }

            cx.notify();
        }
    }

    fn start_editing(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_provider = Some(index);

        // 创建输入框实例
        let provider = &self.providers[index];

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("MCP服务名称")
                .default_value(&provider.name)
        });

        let command_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("可执行文件路径")
                .default_value(&provider.command)
        });

        let args_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("启动参数 (用空格分隔)")
                .default_value(&provider.args.join(" "))
        });

        let description_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("服务描述")
                .default_value(&provider.description)
        });

        let env_input = cx.new(|cx| {
            let env_text = provider
                .env_vars
                .iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<_>>()
                .join("\n");
            InputState::new(window, cx)
                .placeholder("环境变量 (每行一个 KEY=value)")
                .auto_grow(3, 6)
                .default_value(&env_text)
        });

        let transport_index = match provider.transport {
            McpTransport::Stdio => 0,
            McpTransport::Http => 1,
            McpTransport::WebSocket => 2,
        };

        let transport_dropdown =
            cx.new(|cx| DropdownState::new(McpTransport::all(), Some(transport_index), window, cx));

        self.provider_inputs.insert(
            index,
            ProviderInputs {
                name_input,
                command_input,
                args_input,
                description_input,
                env_input,
                transport_dropdown,
            },
        );

        cx.notify();
    }

    fn toggle_accordion(&mut self, open_ixs: &[usize], _: &mut Window, cx: &mut Context<Self>) {
        self.expanded_providers = open_ixs.to_vec();
        cx.notify();
    }

    fn on_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            // InputEvent::PressEnter { .. } => {
            //     if self.editing_provider.is_some() {
            //         self.save_mcp_provider(&SaveMcpProvider, window, cx);
            //     }
            // }
            _ => {}
        };
    }

    fn render_edit_form(&mut self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(inputs) = self.provider_inputs.get(&index) {
            v_flex()
                .gap_3()
                .p_4()
                .bg(gpui::rgb(0xF0F9FF))
                .rounded_lg()
                .border_1()
                .border_color(gpui::rgb(0x0EA5E9))
                .child(
                    h_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui::rgb(0x6B7280))
                                        .child("服务名称 *"),
                                )
                                .child(TextInput::new(&inputs.name_input).cleanable()),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui::rgb(0x6B7280))
                                        .child("传输方式"),
                                )
                                .child(
                                    Dropdown::new(&inputs.transport_dropdown)
                                        .placeholder("选择传输方式")
                                        .small(),
                                ),
                        ),
                )
                .child(
                    h_flex().gap_3().child(
                        v_flex()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child("启动命令&参数"),
                            )
                            .child(TextInput::new(&inputs.args_input).cleanable()),
                    ),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(gpui::rgb(0x6B7280))
                                .child("服务描述"),
                        )
                        .child(TextInput::new(&inputs.description_input).cleanable()),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(gpui::rgb(0x6B7280))
                                .child("环境变量"),
                        )
                        .child(TextInput::new(&inputs.env_input).cleanable()),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new(("cancel-edit", index)).label("取消").on_click(
                            cx.listener(|this, _, window, cx| {
                                this.cancel_edit(&CancelEdit, window, cx);
                            }),
                        ))
                        .child(
                            Button::new(("save-provider", index))
                                .with_variant(ButtonVariant::Primary)
                                .label("保存")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.save_mcp_provider(&SaveMcpProvider, window, cx);
                                })),
                        ),
                )
        } else {
            div().child("加载中...")
        }
    }

    fn set_active_capability_tab(
        &mut self,
        provider_index: usize,
        tab_index: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_capability_tabs
            .insert(provider_index, tab_index);
        cx.notify();
    }

    fn render_capability_content(
        &self,
        provider: &McpProviderInfo,
        tab_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().child(match tab_index {
            0 => div().child(self.render_resources_content(&provider.resources, cx)),
            1 => div().child(self.render_tools_content(&provider.tools, cx)),
            2 => div().child(self.render_prompts_content(&provider.prompts, cx)),
            3 => div().child(self.render_logging_content(cx)),
            _ => div().child("未知能力"),
        })
    }

    fn render_resources_content(
        &self,
        resources: &[McpResource],
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(resources.iter().map(|resource| {
                v_flex()
                    .gap_2()
                    .p_3()
                    .bg(gpui::rgb(0xFAFAFA))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Database)
                                    .small()
                                    .text_color(gpui::rgb(0x059669)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child(resource.name.clone()),
                            )
                            .when(resource.mime_type.is_some(), |this| {
                                this.child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(gpui::rgb(0xE0F2FE))
                                        .text_color(gpui::rgb(0x0369A1))
                                        .rounded_md()
                                        .text_xs()
                                        .child(resource.mime_type.as_ref().unwrap().clone()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child(resource.description.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(gpui::rgb(0x9CA3AF))
                            .child(resource.uri.clone()),
                    )
            }))
            .when(resources.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用资源"),
                )
            })
    }

    fn render_tools_content(&self, tools: &[McpTool], _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .children(tools.iter().map(|tool| {
                v_flex()
                    .gap_2()
                    .p_3()
                    .bg(gpui::rgb(0xFAFAFA))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Wrench)
                                    .small()
                                    .text_color(gpui::rgb(0xDC2626)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child(tool.name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child(tool.description.clone()),
                    )
                    .when(!tool.parameters.is_empty(), |this| {
                        this.child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_medium()
                                        .text_color(gpui::rgb(0x374151))
                                        .child("参数:"),
                                )
                                .children(tool.parameters.iter().map(|param| {
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .pl_2()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(if param.required {
                                                    gpui::rgb(0xDC2626)
                                                } else {
                                                    gpui::rgb(0x059669)
                                                })
                                                .child(format!(
                                                    "{}{}",
                                                    param.name,
                                                    if param.required { "*" } else { "" }
                                                )),
                                        )
                                        .child(
                                            div()
                                                .px_1()
                                                .bg(gpui::rgb(0xF3F4F6))
                                                .rounded_sm()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x6B7280))
                                                .child(param.param_type.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x9CA3AF))
                                                .child(param.description.clone()),
                                        )
                                })),
                        )
                    })
            }))
            .when(tools.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用工具"),
                )
            })
    }

    fn render_prompts_content(
        &self,
        prompts: &[McpPrompt],
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_3()
            .children(prompts.iter().map(|prompt| {
                v_flex()
                    .gap_2()
                    .p_3()
                    .bg(gpui::rgb(0xFAFAFA))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::SquareTerminal)
                                    .small()
                                    .text_color(gpui::rgb(0x7C3AED)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child(prompt.name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child(prompt.description.clone()),
                    )
                    .when(!prompt.arguments.is_empty(), |this| {
                        this.child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_medium()
                                        .text_color(gpui::rgb(0x374151))
                                        .child("参数:"),
                                )
                                .children(prompt.arguments.iter().map(|arg| {
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .pl_2()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(if arg.required {
                                                    gpui::rgb(0xDC2626)
                                                } else {
                                                    gpui::rgb(0x059669)
                                                })
                                                .child(format!(
                                                    "{}{}",
                                                    arg.name,
                                                    if arg.required { "*" } else { "" }
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x9CA3AF))
                                                .child(arg.description.clone()),
                                        )
                                })),
                        )
                    })
            }))
            .when(prompts.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用提示"),
                )
            })
    }

    fn render_logging_content(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_3()
            .bg(gpui::rgb(0xFAFAFA))
            .rounded_md()
            .border_1()
            .border_color(gpui::rgb(0xE5E7EB))
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::LetterText)
                                    .small()
                                    .text_color(gpui::rgb(0xF59E0B)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child("日志记录"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child("此服务支持日志记录功能，可以输出调试和运行状态信息。"),
                    ),
            )
    }
}

impl FocusableCycle for McpProvider {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        if let Some(editing_index) = self.editing_provider {
            if let Some(inputs) = self.provider_inputs.get(&editing_index) {
                return vec![
                    inputs.name_input.focus_handle(cx),
                    inputs.command_input.focus_handle(cx),
                    inputs.args_input.focus_handle(cx),
                    inputs.description_input.focus_handle(cx),
                    inputs.env_input.focus_handle(cx),
                    inputs.transport_dropdown.focus_handle(cx),
                ];
            }
        }
        vec![self.focus_handle.clone()]
    }
}

impl Focusable for McpProvider {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for McpProvider {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("mcp-provider")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::add_mcp_provider))
            .on_action(cx.listener(Self::save_mcp_provider))
            .on_action(cx.listener(Self::cancel_edit))
            .size_full()
            .gap_4()
            .child(
                h_flex().justify_start().child(
                    Button::new("add-mcp-provider")
                        .with_variant(ButtonVariant::Primary)
                        .label("添加工具集")
                        .icon(IconName::Plus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_mcp_provider(&AddMcpProvider, window, cx);
                        })),
                ),
            )
            .child(div().w_full().child({
                let mut accordion = Accordion::new("mcp-providers").multiple(true);

                // 预先收集所有需要的数据，避免在闭包中借用self
                let expanded_providers = self.expanded_providers.clone();
                let editing_provider = self.editing_provider;
                let active_capability_tabs = self.active_capability_tabs.clone();

                // 预先收集编辑表单的输入框数据
                let edit_inputs = if let Some(editing_index) = editing_provider {
                    self.provider_inputs.get(&editing_index).map(|inputs| {
                        (
                            inputs.name_input.clone(),
                            inputs.command_input.clone(),
                            inputs.args_input.clone(),
                            inputs.description_input.clone(),
                            inputs.env_input.clone(),
                            inputs.transport_dropdown.clone(),
                        )
                    })
                } else {
                    None
                };

                for (index, provider) in self.providers.iter().enumerate() {
                    let provider_name = provider.name.clone();
                    let provider_command = provider.command.clone();
                    let provider_args = provider.args.join(" ");
                    let provider_transport = provider.transport.as_str().to_string();
                    let provider_enabled = provider.enabled;
                    let provider_clone = provider.clone(); // 为Tab内容克隆整个provider
                    let is_editing = editing_provider == Some(index);

                    accordion = accordion.item(|item| {
                        item.open(expanded_providers.contains(&index) && provider_enabled)
                            .disabled(!provider_enabled)
                            .icon(if provider_enabled {
                                IconName::CircleCheck
                            } else {
                                IconName::CircleX
                            })
                            .title(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_medium()
                                            .flex_1()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .text_color(if provider_enabled {
                                                gpui::rgb(0x111827)
                                            } else {
                                                gpui::rgb(0xD1D5DB)
                                            })
                                            .child(if provider_name.is_empty() {
                                                "新建MCP服务".to_string()
                                            } else {
                                                provider_name.clone()
                                            }),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .flex_shrink_0()
                                            .child(
                                                div()
                                                    .px_2()
                                                    .bg(if provider_enabled {
                                                        gpui::rgb(0xDEF7EC)
                                                    } else {
                                                        gpui::rgb(0xF3F4F6)
                                                    })
                                                    .text_color(if provider_enabled {
                                                        gpui::rgb(0x047857)
                                                    } else {
                                                        gpui::rgb(0xD1D5DB)
                                                    })
                                                    .rounded_md()
                                                    .text_xs()
                                                    .whitespace_nowrap()
                                                    .child(provider_transport.clone()),
                                            )
                                            .child(
                                                Switch::new(("mcp-provider-enabled", index))
                                                    .checked(provider_enabled)
                                                    .on_click(cx.listener(
                                                        move |this, checked, window, cx| {
                                                            this.toggle_provider_enabled(
                                                                index, *checked, window, cx,
                                                            );
                                                        },
                                                    )),
                                            )
                                            .child(
                                                Button::new(("edit-mcp-provider", index))
                                                    .icon(if provider_enabled {
                                                        Icon::new(IconName::SquarePen)
                                                    } else {
                                                        Icon::new(IconName::SquarePen)
                                                            .text_color(gpui::rgb(0xD1D5DB))
                                                    })
                                                    .small()
                                                    .ghost()
                                                    .tooltip("编辑")
                                                    .disabled(!provider_enabled || is_editing)
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.start_editing(index, window, cx);
                                                        },
                                                    )),
                                            )
                                            .child(
                                                Button::new(("delete-mcp-provider", index))
                                                    .icon(if provider_enabled {
                                                        Icon::new(IconName::Trash2)
                                                            .text_color(gpui::rgb(0xEF4444))
                                                    } else {
                                                        Icon::new(IconName::Trash2)
                                                            .text_color(gpui::rgb(0xD1D5DB))
                                                    })
                                                    .small()
                                                    .ghost()
                                                    .tooltip("删除")
                                                    .disabled(!provider_enabled || is_editing)
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.delete_mcp_provider(
                                                                index, window, cx,
                                                            );
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            .content(v_flex().gap_4().child(if is_editing {
                                // 编辑表单保持不变
                                if let Some((
                                    name_input,
                                    command_input,
                                    args_input,
                                    description_input,
                                    env_input,
                                    transport_dropdown,
                                )) = &edit_inputs
                                {
                                    div().child(
                                        v_flex()
                                            .gap_3()
                                            .p_4()
                                            .bg(gpui::rgb(0xF0F9FF))
                                            .rounded_lg()
                                            .border_1()
                                            .border_color(gpui::rgb(0x0EA5E9))
                                            .child(
                                                h_flex()
                                                    .gap_3()
                                                    .child(
                                                        v_flex()
                                                            .gap_1()
                                                            .flex_1()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .text_color(gpui::rgb(0x6B7280))
                                                                    .child("服务名称 *"),
                                                            )
                                                            .child(
                                                                TextInput::new(name_input)
                                                                    .cleanable(),
                                                            ),
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_1()
                                                            .flex_1()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .text_color(gpui::rgb(0x6B7280))
                                                                    .child("传输方式"),
                                                            )
                                                            .child(
                                                                Dropdown::new(transport_dropdown)
                                                                    .placeholder("选择传输方式")
                                                                    .small(),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                h_flex().gap_3().child(
                                                    v_flex()
                                                        .gap_1()
                                                        .flex_1()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(gpui::rgb(0x6B7280))
                                                                .child("启动命令&参数"),
                                                        )
                                                        .child(
                                                            TextInput::new(args_input).cleanable(),
                                                        ),
                                                ),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(gpui::rgb(0x6B7280))
                                                            .child("服务描述"),
                                                    )
                                                    .child(
                                                        TextInput::new(description_input)
                                                            .cleanable(),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(gpui::rgb(0x6B7280))
                                                            .child("环境变量"),
                                                    )
                                                    .child(TextInput::new(env_input).cleanable()),
                                            )
                                            .child(
                                                h_flex()
                                                    .justify_end()
                                                    .gap_2()
                                                    .child(
                                                        Button::new(("cancel-edit", index))
                                                            .label("取消")
                                                            .on_click(cx.listener(
                                                                |this, _, window, cx| {
                                                                    this.cancel_edit(
                                                                        &CancelEdit,
                                                                        window,
                                                                        cx,
                                                                    );
                                                                },
                                                            )),
                                                    )
                                                    .child(
                                                        Button::new(("save-provider", index))
                                                            .with_variant(ButtonVariant::Primary)
                                                            .label("保存")
                                                            .on_click(cx.listener(
                                                                |this, _, window, cx| {
                                                                    this.save_mcp_provider(
                                                                        &SaveMcpProvider,
                                                                        window,
                                                                        cx,
                                                                    );
                                                                },
                                                            )),
                                                    ),
                                            ),
                                    )
                                } else {
                                    div().child("加载中...")
                                }
                            } else {
                                // 使用Tab页显示服务信息
                                div().child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            TabBar::new(("mcp-capabilities", index))
                                                .w_full()
                                                .pill()
                                                .small()
                                                .selected_index(
                                                    active_capability_tabs
                                                        .get(&index)
                                                        .copied()
                                                        .unwrap_or(0),
                                                )
                                                .child(Tab::new("配置信息"))
                                                .child(Tab::new("资源"))
                                                .child(Tab::new("工具"))
                                                .child(Tab::new("提示"))
                                                .child(Tab::new("日志"))
                                                .on_click(cx.listener(
                                                    move |this, tab_ix: &usize, window, cx| {
                                                        this.set_active_capability_tab(
                                                            index, *tab_ix, window, cx,
                                                        );
                                                    },
                                                )),
                                        )
                                        .child(
                                            div().mt_2().child(
                                                Self::render_capability_content_static(
                                                    &provider_clone,
                                                    active_capability_tabs
                                                        .get(&index)
                                                        .copied()
                                                        .unwrap_or(0),
                                                    cx,
                                                ),
                                            ),
                                        ),
                                )
                            }))
                    });
                }
                accordion.on_toggle_click(cx.listener(Self::toggle_accordion))
            }))
    }
}

impl McpProvider {
    // 切换资源订阅状态的方法
    fn toggle_resource_subscription(
        &mut self,
        resource_name: String,
        subscribed: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 遍历所有提供商，找到匹配的资源并更新订阅状态
        for provider in &mut self.providers {
            for resource in &mut provider.resources {
                if resource.name == resource_name && resource.subscribable {
                    resource.subscribed = subscribed;

                    // 打印日志，方便调试
                    println!(
                        "资源 '{}' 订阅状态已{}",
                        resource_name,
                        if subscribed { "开启" } else { "关闭" }
                    );

                    cx.notify();
                    return;
                }
            }
        }

        // 如果没找到匹配的资源，打印警告
        eprintln!("警告: 未找到名为 '{}' 的可订阅资源", resource_name);
    }

    // 如果你需要更精确的控制，可以使用这个带提供商索引的版本
    fn toggle_resource_subscription_by_index(
        &mut self,
        provider_index: usize,
        resource_index: usize,
        subscribed: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(provider) = self.providers.get_mut(provider_index) {
            if let Some(resource) = provider.resources.get_mut(resource_index) {
                if resource.subscribable {
                    let old_status = resource.subscribed;
                    resource.subscribed = subscribed;

                    println!(
                        "提供商 '{}' 的资源 '{}' 订阅状态从 {} 变更为 {}",
                        provider.name,
                        resource.name,
                        if old_status { "已订阅" } else { "未订阅" },
                        if subscribed { "已订阅" } else { "未订阅" }
                    );

                    cx.notify();
                } else {
                    eprintln!("警告: 资源 '{}' 不支持订阅功能", resource.name);
                }
            } else {
                eprintln!(
                    "警告: 提供商索引 {} 中不存在资源索引 {}",
                    provider_index, resource_index
                );
            }
        } else {
            eprintln!("警告: 不存在提供商索引 {}", provider_index);
        }
    }

    // 获取所有已订阅的资源列表
    fn get_subscribed_resources(&self) -> Vec<(String, String)> {
        let mut subscribed = Vec::new();

        for provider in &self.providers {
            for resource in &provider.resources {
                if resource.subscribed {
                    subscribed.push((provider.name.clone(), resource.name.clone()));
                }
            }
        }

        subscribed
    }

    // 批量设置资源订阅状态
    fn set_all_resources_subscription(
        &mut self,
        provider_name: &str,
        subscribed: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(provider) = self.providers.iter_mut().find(|p| p.name == provider_name) {
            let mut updated_count = 0;

            for resource in &mut provider.resources {
                if resource.subscribable && resource.subscribed != subscribed {
                    resource.subscribed = subscribed;
                    updated_count += 1;
                }
            }

            if updated_count > 0 {
                println!(
                    "提供商 '{}' 的 {} 个资源订阅状态已{}",
                    provider_name,
                    updated_count,
                    if subscribed { "开启" } else { "关闭" }
                );

                cx.notify();
            }
        } else {
            eprintln!("警告: 未找到名为 '{}' 的提供商", provider_name);
        }
    }

    // 修改静态方法，移除cx参数，因为静态方法中无法使用listener
    fn render_resources_content_static(resources: &[McpResource]) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(
                resources
                    .iter()
                    .enumerate()
                    .map(|(resource_index, resource)| {
                        v_flex()
                            .gap_2()
                            .p_3()
                            .bg(gpui::rgb(0xFAFAFA))
                            .rounded_md()
                            .border_1()
                            .border_color(gpui::rgb(0xE5E7EB))
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                Icon::new(IconName::Database)
                                                    .small()
                                                    .text_color(gpui::rgb(0x059669)),
                                            )
                                            .child(
                                                div()
                                                    .font_medium()
                                                    .text_color(gpui::rgb(0x111827))
                                                    .child(resource.name.clone()),
                                            )
                                            .when(resource.subscribable, |this| {
                                                this.child(
                                                    div()
                                                        .px_2()
                                                        .py_1()
                                                        .bg(if resource.subscribed {
                                                            gpui::rgb(0xDCFCE7)
                                                        } else {
                                                            gpui::rgb(0xF3F4F6)
                                                        })
                                                        .text_color(if resource.subscribed {
                                                            gpui::rgb(0x166534)
                                                        } else {
                                                            gpui::rgb(0x6B7280)
                                                        })
                                                        .rounded_md()
                                                        .text_xs()
                                                        .child(if resource.subscribed {
                                                            "已订阅"
                                                        } else {
                                                            "可订阅"
                                                        }),
                                                )
                                            })
                                            .when(resource.mime_type.is_some(), |this| {
                                                this.child(
                                                    div()
                                                        .px_2()
                                                        .py_1()
                                                        .bg(gpui::rgb(0xE0F2FE))
                                                        .text_color(gpui::rgb(0x0369A1))
                                                        .rounded_md()
                                                        .text_xs()
                                                        .child(
                                                            resource
                                                                .mime_type
                                                                .as_ref()
                                                                .unwrap()
                                                                .clone(),
                                                        ),
                                                )
                                            }),
                                    )
                                    // 注意：在静态方法中无法使用cx.listener，所以这里暂时显示静态状态
                                    // 如果需要交互功能，应该使用非静态的render方法
                                    .when(resource.subscribable, |this| {
                                        this.child(
                                            Switch::new(("resource-subscription", resource_index))
                                                .checked(resource.subscribed)
                                                .on_click(|checked, _window, _cx| {
                                                    // this.toggle_resource_subscription(
                                                    //     resource.name.clone(),
                                                    //     *checked,
                                                    //     _window,
                                                    //     _cx,
                                                    // );
                                                }),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child(resource.description.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x9CA3AF))
                                    .child(resource.uri.clone()),
                            )
                    }),
            )
            .when(resources.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用资源"),
                )
            })
    }

    // 修改调用位置，移除cx参数
    fn render_capability_content_static(
        provider: &McpProviderInfo,
        tab_index: usize,
        _cx: &mut Context<Self>, // 这个参数现在用不到，可以加下划线
    ) -> impl IntoElement {
        div().child(match tab_index {
            0 => div().child(Self::render_config_content_static(provider)),
            1 => div().child(Self::render_resources_content_static(&provider.resources)), // 移除cx参数
            2 => div().child(Self::render_tools_content_static(&provider.tools)),
            3 => div().child(Self::render_prompts_content_static(&provider.prompts)),
            4 => div().child(Self::render_logging_content_static()),
            _ => div().child("未知能力"),
        })
    }

    // 渲染配置信息的静态方法
    fn render_config_content_static(provider: &McpProviderInfo) -> impl IntoElement {
        v_flex().gap_3().child(
            div()
                .p_3()
                .bg(gpui::rgb(0xFAFAFA))
                .rounded_md()
                .border_1()
                .border_color(gpui::rgb(0xE5E7EB))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_2()
                                .when(!provider.description.is_empty(), |this| {
                                    this.child(
                                        h_flex()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(gpui::rgb(0x6B7280))
                                                    .min_w_20()
                                                    .child("描述:"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(gpui::rgb(0x111827))
                                                    .child(provider.description.clone()),
                                            ),
                                    )
                                })
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(gpui::rgb(0x6B7280))
                                                .min_w_20()
                                                .child("启动命令:"),
                                        )
                                        .child(
                                            div().text_sm().text_color(gpui::rgb(0x111827)).child(
                                                format!(
                                                    "{} {}",
                                                    provider.command,
                                                    provider.args.join(" ")
                                                ),
                                            ),
                                        ),
                                )
                                
                                .when(!provider.env_vars.is_empty(), |this| {
                                    this.child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(gpui::rgb(0x6B7280))
                                                    .child("环境变量:"),
                                            )
                                            .children(provider.env_vars.iter().map(
                                                |(key, value)| {
                                                    div()
                                                        .pl_4()
                                                        .text_xs()
                                                        .text_color(gpui::rgb(0x9CA3AF))
                                                        .child(format!("{}={}", key, value))
                                                },
                                            )),
                                    )
                                }),
                        ),
                ),
        )
    }

    // 渲染工具的静态方法
    fn render_tools_content_static(tools: &[McpTool]) -> impl IntoElement {
        v_flex()
            .gap_3()
            .children(tools.iter().map(|tool| {
                v_flex()
                    .gap_2()
                    .p_3()
                    .bg(gpui::rgb(0xFAFAFA))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Wrench)
                                    .small()
                                    .text_color(gpui::rgb(0xDC2626)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child(tool.name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child(tool.description.clone()),
                    )
                    .when(!tool.parameters.is_empty(), |this| {
                        this.child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_medium()
                                        .text_color(gpui::rgb(0x374151))
                                        .child("参数:"),
                                )
                                .children(tool.parameters.iter().map(|param| {
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .pl_2()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(if param.required {
                                                    gpui::rgb(0xDC2626)
                                                } else {
                                                    gpui::rgb(0x059669)
                                                })
                                                .child(format!(
                                                    "{}{}",
                                                    param.name,
                                                    if param.required { "*" } else { "" }
                                                )),
                                        )
                                        .child(
                                            div()
                                                .px_1()
                                                .bg(gpui::rgb(0xF3F4F6))
                                                .rounded_sm()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x6B7280))
                                                .child(param.param_type.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x9CA3AF))
                                                .child(param.description.clone()),
                                        )
                                })),
                        )
                    })
            }))
            .when(tools.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用工具"),
                )
            })
    }

    // 渲染提示的静态方法
    fn render_prompts_content_static(prompts: &[McpPrompt]) -> impl IntoElement {
        v_flex()
            .gap_3()
            .children(prompts.iter().map(|prompt| {
                v_flex()
                    .gap_2()
                    .p_3()
                    .bg(gpui::rgb(0xFAFAFA))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::SquareTerminal)
                                    .small()
                                    .text_color(gpui::rgb(0x7C3AED)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child(prompt.name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child(prompt.description.clone()),
                    )
                    .when(!prompt.arguments.is_empty(), |this| {
                        this.child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_medium()
                                        .text_color(gpui::rgb(0x374151))
                                        .child("参数:"),
                                )
                                .children(prompt.arguments.iter().map(|arg| {
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .pl_2()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(if arg.required {
                                                    gpui::rgb(0xDC2626)
                                                } else {
                                                    gpui::rgb(0x059669)
                                                })
                                                .child(format!(
                                                    "{}{}",
                                                    arg.name,
                                                    if arg.required { "*" } else { "" }
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x9CA3AF))
                                                .child(arg.description.clone()),
                                        )
                                })),
                        )
                    })
            }))
            .when(prompts.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用提示"),
                )
            })
    }

    // 渲染日志的静态方法
    fn render_logging_content_static() -> impl IntoElement {
        div()
            .p_3()
            .bg(gpui::rgb(0xFAFAFA))
            .rounded_md()
            .border_1()
            .border_color(gpui::rgb(0xE5E7EB))
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::LetterText)
                                    .small()
                                    .text_color(gpui::rgb(0xF59E0B)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child("日志记录"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child("此服务支持日志记录功能，可以输出调试和运行状态信息。"),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x374151))
                                    .child("日志级别:"),
                            )
                            .child(
                                h_flex().gap_2().pl_2().children(
                                    [
                                        ("DEBUG", gpui::rgb(0x6B7280)),
                                        ("INFO", gpui::rgb(0x3B82F6)),
                                        ("WARN", gpui::rgb(0xF59E0B)),
                                        ("ERROR", gpui::rgb(0xEF4444)),
                                    ]
                                    .iter()
                                    .map(|(level, color)| {
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(gpui::rgb(0xF3F4F6))
                                            .text_color(*color)
                                            .rounded_sm()
                                            .text_xs()
                                            .child(*level)
                                    }),
                                ),
                            ),
                    ),
            )
    }
}
