use crate::ui::components::{section::section, ViewKit};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonGroup, ButtonVariant, ButtonVariants as _},
    dropdown::{Dropdown, DropdownState},
    h_flex,
    input::{InputEvent, InputState, TextInput},
    switch::Switch,
    v_flex, ContextModal, Disableable, FocusableCycle, Icon, IconName, Sizable, StyledExt,
};

actions!(
    mcp_provider,
    [
        Tab,
        TabPrev,
        AddMcpProvider,
        SaveMcpProvider,
        DeleteMcpProvider
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
pub struct McpProviderInfo {
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    transport: McpTransport,
    enabled: bool,
    capabilities: Vec<McpCapability>,
    description: String,
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
        }
    }
}

pub struct McpProvider {
    focus_handle: FocusHandle,
    providers: Vec<McpProviderInfo>,
    expanded_providers: Vec<usize>,

    // 编辑表单字段
    editing_provider: Option<usize>,
    name_input: Entity<InputState>,
    command_input: Entity<InputState>,
    args_input: Entity<InputState>,
    description_input: Entity<InputState>,
    transport_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    _subscriptions: Vec<Subscription>,
}

impl ViewKit for McpProvider {
    fn title() -> &'static str {
        "MCP服务"
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
            KeyBinding::new("tab", Tab, Some(CONTEXT)),
            KeyBinding::new("ctrl-n", AddMcpProvider, Some(CONTEXT)),
        ])
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("MCP服务名称"));

        let command_input = cx.new(|cx| InputState::new(window, cx).placeholder("可执行文件路径"));

        let args_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("启动参数 (用空格分隔)"));

        let description_input = cx.new(|cx| InputState::new(window, cx).placeholder("服务描述"));

        let transport_dropdown =
            cx.new(|cx| DropdownState::new(McpTransport::all(), Some(0), window, cx));

        let _subscriptions = vec![
            cx.subscribe_in(&name_input, window, Self::on_input_event),
            cx.subscribe_in(&command_input, window, Self::on_input_event),
            cx.subscribe_in(&args_input, window, Self::on_input_event),
            cx.subscribe_in(&description_input, window, Self::on_input_event),
        ];

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

        Self {
            focus_handle: cx.focus_handle(),
            providers: vec![filesystem_provider, database_provider],
            expanded_providers: vec![0],
            editing_provider: None,
            name_input,
            command_input,
            args_input,
            description_input,
            transport_dropdown,
            _subscriptions,
        }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
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
        self.editing_provider = Some(self.providers.len());
        self.providers.push(McpProviderInfo::default());
        self.clear_form(window, cx);
        cx.notify();
    }

    fn save_mcp_provider(
        &mut self,
        _: &SaveMcpProvider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.editing_provider {
            if let Some(provider) = self.providers.get_mut(index) {
                provider.name = self.name_input.read(cx).value().to_string();
                provider.command = self.command_input.read(cx).value().to_string();
                provider.description = self.description_input.read(cx).value().to_string();

                // 解析启动参数
                let args_text = self.args_input.read(cx).value().to_string();
                provider.args = args_text
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                if let Some(selected) = self.transport_dropdown.read(cx).selected_value() {
                    provider.transport = match selected.as_ref() {
                        "Stdio" => McpTransport::Stdio,
                        "HTTP" => McpTransport::Http,
                        "WebSocket" => McpTransport::WebSocket,
                        _ => McpTransport::Stdio,
                    };
                }
            }
        }

        self.editing_provider = None;
        self.clear_form(window, cx);
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
                self.clear_form(window, cx);
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

    fn edit_mcp_provider(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_provider = Some(index);

        if let Some(provider) = self.providers.get(index) {
            self.name_input.update(cx, |state, cx| {
                *state = InputState::new(window, cx).default_value(&provider.name);
            });

            self.command_input.update(cx, |state, cx| {
                *state = InputState::new(window, cx).default_value(&provider.command);
            });

            self.args_input.update(cx, |state, cx| {
                let args_text = provider.args.join(" ");
                *state = InputState::new(window, cx).default_value(&args_text);
            });

            self.description_input.update(cx, |state, cx| {
                *state = InputState::new(window, cx).default_value(&provider.description);
            });

            let transport_index = match provider.transport {
                McpTransport::Stdio => 0,
                McpTransport::Http => 1,
                McpTransport::WebSocket => 2,
            };

            self.transport_dropdown.update(cx, |state, cx| {
                state.set_selected_index(Some(transport_index), window, cx);
            });
        }

        cx.notify();
    }

    fn clear_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.name_input.update(cx, |state, cx| {
            *state = InputState::new(window, cx).placeholder("MCP服务名称");
        });

        self.command_input.update(cx, |state, cx| {
            *state = InputState::new(window, cx).placeholder("可执行文件路径");
        });

        self.args_input.update(cx, |state, cx| {
            *state = InputState::new(window, cx).placeholder("启动参数 (用空格分隔)");
        });

        self.description_input.update(cx, |state, cx| {
            *state = InputState::new(window, cx).placeholder("服务描述");
        });

        self.transport_dropdown.update(cx, |state, cx| {
            state.set_selected_index(Some(0), window, cx);
        });
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
            InputEvent::PressEnter { .. } => {
                if self.editing_provider.is_some() {
                    self.save_mcp_provider(&SaveMcpProvider, window, cx);
                }
            }
            _ => {}
        };
    }

    fn render_provider_form(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_4()
            .bg(gpui::rgb(0xF9FAFB))
            .rounded_lg()
            .border_1()
            .border_color(gpui::rgb(0xE5E7EB))
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .text_color(gpui::rgb(0x374151))
                    .child(if self.editing_provider.is_some() {
                        "编辑MCP服务"
                    } else {
                        "添加MCP服务"
                    }),
            )
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
                            .child(TextInput::new(&self.name_input).cleanable()),
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
                                Dropdown::new(&self.transport_dropdown)
                                    .placeholder("选择传输方式")
                                    .small(),
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
                            .child("可执行文件路径 *"),
                    )
                    .child(TextInput::new(&self.command_input).cleanable()),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x6B7280))
                            .child("启动参数"),
                    )
                    .child(TextInput::new(&self.args_input).cleanable()),
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
                    .child(TextInput::new(&self.description_input).cleanable()),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-edit")
                            .label("取消")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editing_provider = None;
                                this.clear_form(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("save-provider")
                            .with_variant(ButtonVariant::Primary)
                            .label("保存")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save_mcp_provider(&SaveMcpProvider, window, cx);
                            })),
                    ),
            )
    }
}

impl FocusableCycle for McpProvider {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![
            self.name_input.focus_handle(cx),
            self.command_input.focus_handle(cx),
            self.args_input.focus_handle(cx),
            self.description_input.focus_handle(cx),
            self.transport_dropdown.focus_handle(cx),
        ]
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
            .size_full()
            .gap_4()
            .child(
                h_flex().justify_start().child(
                    Button::new("add-mcp-provider")
                        .with_variant(ButtonVariant::Primary)
                        .label("添加MCP服务")
                        .icon(IconName::Plus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_mcp_provider(&AddMcpProvider, window, cx);
                        })),
                ),
            )
            .child(div().when(self.editing_provider.is_some(), |this| {
                this.child(self.render_provider_form(cx))
            }))
            .child(div().w_full().child({
                let mut accordion = Accordion::new("mcp-providers").multiple(true);

                for (index, provider) in self.providers.iter().enumerate() {
                    let provider_name = provider.name.clone();
                    let provider_command = provider.command.clone();
                    let provider_args = provider.args.join(" ");
                    let provider_transport = provider.transport.as_str().to_string();
                    let provider_enabled = provider.enabled;
                    let provider_capabilities = provider.capabilities.clone();
                    let provider_description = provider.description.clone();

                    accordion = accordion.item(|item| {
                        item.open(self.expanded_providers.contains(&index) && provider_enabled)
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
                                            .child(provider_name.clone()),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .flex_shrink_0()
                                            .child(
                                                div()
                                                    .px_2()
                                                    // .py_1()
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
                                                    .disabled(!provider_enabled)
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.edit_mcp_provider(
                                                                index, window, cx,
                                                            );
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
                                                    .disabled(!provider_enabled)
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
                            .content(
                                v_flex()
                                    .gap_4()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .child(
                                                h_flex()
                                                    .gap_4()
                                                    .child(
                                                        v_flex()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_medium()
                                                                    .text_color(gpui::rgb(0x374151))
                                                                    .child("可执行文件"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .text_color(gpui::rgb(0x6B7280))
                                                                    .child(
                                                                        provider_command.clone(),
                                                                    ),
                                                            ),
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_medium()
                                                                    .text_color(gpui::rgb(0x374151))
                                                                    .child("启动参数"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .text_color(gpui::rgb(0x6B7280))
                                                                    .child({
                                                                        let provider_args_clone =
                                                                            provider_args.clone();
                                                                        if provider_args_clone
                                                                            .is_empty()
                                                                        {
                                                                            "无".to_string()
                                                                        } else {
                                                                            provider_args_clone
                                                                                .clone()
                                                                        }
                                                                    }),
                                                            ),
                                                    ),
                                            )
                                            .when(!provider_description.is_empty(), |this| {
                                                this.child(
                                                    v_flex()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .font_medium()
                                                                .text_color(gpui::rgb(0x374151))
                                                                .child("服务描述"),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(gpui::rgb(0x6B7280))
                                                                .child(
                                                                    provider_description.clone(),
                                                                ),
                                                        ),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(gpui::rgb(0x9CA3AF))
                                                    .child({
                                                        let provider_args_clone =
                                                            provider_args.clone();
                                                        if provider_args_clone.is_empty() {
                                                            provider_command.clone()
                                                        } else {
                                                            format!(
                                                                "{} {}",
                                                                provider_command.clone(),
                                                                provider_args_clone
                                                            )
                                                        }
                                                    }),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .text_color(gpui::rgb(0x374151))
                                                    .child("服务能力"),
                                            )
                                            .child(h_flex().gap_2().items_center().children(
                                                provider_capabilities.iter().enumerate().map(
                                                    |(cap_index, cap)| {
                                                        let capability_unique_id =
                                                            index * 1000 + cap_index;

                                                        div()
                                                            .id((
                                                                "mcp-capability",
                                                                capability_unique_id,
                                                            ))
                                                            .p_1()
                                                            .rounded_md()
                                                            .bg(gpui::rgb(0xF3F4F6))
                                                            .child(Icon::new(cap.icon()).xsmall())
                                                    },
                                                ),
                                            )),
                                    ),
                            )
                    });
                }

                accordion.on_toggle_click(cx.listener(Self::toggle_accordion))
            }))
    }
}
