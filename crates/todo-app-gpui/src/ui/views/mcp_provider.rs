use crate::app::FoEvent;
use crate::backoffice::mcp::server::{ResourceDefinition, ResourceTemplateDefinition};
use crate::backoffice::mcp::McpRegistry; // 新增导入
use crate::config::mcp_config::{
    McpConfigManager,
    McpServerConfig,
    McpTransport, // 移除 McpPrompt, McpTool - 这些现在从 rmcp 导入
};
use crate::ui::components::ViewKit;
use crate::xbus;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dropdown::{Dropdown, DropdownState},
    input::{InputEvent, InputState, TextInput},
    switch::Switch,
    tab::{Tab, TabBar},
    *,
};
// 从 rmcp 导入需要的类型
use rmcp::model::{Prompt as McpPrompt, Tool as McpTool};

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

// 用于存储每个Provider的编辑状态输入框
#[derive(Clone)]
struct ProviderInputs {
    name_input: Entity<InputState>,
    //command_input: Entity<InputState>,
    args_input: Entity<InputState>,
    description_input: Entity<InputState>,
    env_input: Entity<InputState>,
    transport_dropdown: Entity<DropdownState<Vec<SharedString>>>,
}

pub struct McpProvider {
    focus_handle: FocusHandle,
    providers: Vec<McpServerConfig>,
    expanded_providers: Vec<usize>,
    active_capability_tabs: std::collections::HashMap<usize, usize>,
    editing_provider: Option<usize>,
    provider_inputs: std::collections::HashMap<usize, ProviderInputs>,
    // 缓存从 Registry 获取的能力数据
    cached_capabilities: std::collections::HashMap<
        String,
        (
            Vec<McpTool>,
            Vec<McpPrompt>,
            Vec<ResourceDefinition>,
            Vec<ResourceTemplateDefinition>,
        ),
    >,
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

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            providers: McpConfigManager::load_servers().unwrap_or_default(),
            expanded_providers: vec![],
            active_capability_tabs: std::collections::HashMap::new(),
            editing_provider: None,
            provider_inputs: std::collections::HashMap::new(),
            cached_capabilities: std::collections::HashMap::new(),
            _subscriptions: vec![],
        }
    }

    // 保存配置到文件
    fn save_config(&self, cx: &mut Context<Self>) {
        println!("保存MCP配置到文件...");
        if let Err(e) = McpConfigManager::save_servers(&self.providers[..]) {
            eprintln!("保存MCP配置失败: {}", e);
        }
        xbus::post(&FoEvent::McpConfigUpdated);
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
        self.providers.push(McpServerConfig::default());
        self.expanded_providers.push(new_index);
        self.start_editing(new_index, window, cx);
        cx.notify();
        self.save_config(cx);
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
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.editing_provider {
            if let (Some(provider), Some(inputs)) = (
                self.providers.get_mut(index),
                self.provider_inputs.get(&index),
            ) {
                provider.name = inputs.name_input.read(cx).value().to_string();
                provider.description = inputs.description_input.read(cx).value().to_string();

                // 解析启动参数
                let args_text = inputs.args_input.read(cx).value().to_string();
                provider.command = args_text;

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
                        "Sse" => McpTransport::Sse,
                        "Streamable" => McpTransport::Streamable,
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
        self.save_config(cx);
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
        self.save_config(cx);
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
            self.save_config(cx);
        }
    }

    // 异步获取提供商能力
    fn refresh_provider_capabilities(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            // 通过 McpRegistry 获取实例
            println!("正在获取MCP服务的能力信息...");
            if let Ok(instances) = McpRegistry::get_all_instances_static().await {
                for instance in instances {
                    // 获取能力信息
                    let mut tools = instance.tools.clone();
                    tools.sort_by(|a, b| a.name.cmp(&b.name));
                    let mut prompts = instance.prompts.clone();
                    prompts.sort_by(|a, b| a.name.cmp(&b.name));
                    let mut resources = instance.resources.clone();
                    resources.sort_by(|a, b| a.resource.uri.cmp(&b.resource.uri));
                    let mut resource_templates = instance.resource_templates.clone();
                    resource_templates.sort_by(|a, b| {
                        a.resource_template
                            .uri_template
                            .cmp(&b.resource_template.uri_template)
                    });
                    println!(
                        "获取到 {} 个工具, {} 个提示, {} 个资源",
                        tools.len(),
                        prompts.len(),
                        resources.len()
                    );
                    // 更新缓存和UI
                    this.update(cx, |this, cx| {
                        this.cached_capabilities.insert(
                            instance.config.id.clone(),
                            (tools, prompts, resources, resource_templates),
                        );
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn toggle_provider_enabled(
        &mut self,
        index: usize,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(provider) = self.providers.get_mut(index) {
            provider.enabled = enabled;

            let provider_id = provider.id.clone();
            let provider_name = provider.name.clone();

            self.save_config(cx);

            if enabled {
                // 异步刷新能力信息
                self.refresh_provider_capabilities(cx);
                window.push_notification(
                    format!("MCP服务 '{}' 已启用，正在启动...", provider_name),
                    cx,
                );
            } else {
                // 清除缓存的能力数据
                self.cached_capabilities.remove(&provider_id);
                window.push_notification(format!("MCP服务 '{}' 已禁用", provider_name), cx);
            }
            cx.notify();
        }
    }

    // 获取服务器实例的状态信息（用于显示）
    fn get_provider_status(&self, provider: &McpServerConfig) -> (String, bool) {
        // 这里可以通过 McpRegistry 获取实际状态，暂时返回默认值
        if provider.enabled {
            ("运行中".to_string(), true)
        } else {
            ("已停用".to_string(), false)
        }
    }

    // 从 McpRegistry 获取实例数据
    fn get_provider_capabilities(
        &self,
        provider_id: &str,
    ) -> (
        Vec<McpTool>,
        Vec<McpPrompt>,
        Vec<ResourceDefinition>,
        Vec<ResourceTemplateDefinition>,
    ) {
        // 从缓存中获取数据
        self.cached_capabilities
            .get(provider_id)
            .cloned()
            .unwrap_or_default()
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

        let args_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("启动命令&参数 (用空格分隔)")
                .default_value(&provider.command)
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
            McpTransport::Sse => 1,
            McpTransport::Streamable => 2,
        };

        let transport_dropdown =
            cx.new(|cx| DropdownState::new(McpTransport::all(), Some(transport_index), window, cx));

        self.provider_inputs.insert(
            index,
            ProviderInputs {
                name_input,
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
        self.refresh_provider_capabilities(cx);
        cx.notify();
    }

    fn on_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
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
}

impl FocusableCycle for McpProvider {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        if let Some(editing_index) = self.editing_provider {
            if let Some(inputs) = self.provider_inputs.get(&editing_index) {
                return vec![
                    inputs.name_input.focus_handle(cx),
                    // inputs.command_input.focus_handle(cx),
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
                let providers = self.providers.clone();
                // 预先收集编辑表单的输入框数据
                let edit_inputs = if let Some(editing_index) = editing_provider {
                    self.provider_inputs.get(&editing_index).map(|inputs| {
                        (
                            inputs.name_input.clone(),
                            // inputs.command_input.clone(),
                            inputs.args_input.clone(),
                            inputs.description_input.clone(),
                            inputs.env_input.clone(),
                            inputs.transport_dropdown.clone(),
                        )
                    })
                } else {
                    None
                };

                for (index, provider) in providers.iter().enumerate() {
                    let provider_name = provider.name.clone();
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
                                                .child(Tab::new("资源模板"))
                                                .child(Tab::new("工具"))
                                                .child(Tab::new("提示"))
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
                                                self.render_capability_content_interactive(
                                                    &provider_clone,
                                                    index,
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
    // 只保留从缓存获取数据的静态渲染方法
    fn render_capability_content_interactive(
        &mut self,
        provider: &McpServerConfig,
        _provider_index: usize,
        tab_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (tools, prompts, resources, resource_templates) =
            self.get_provider_capabilities(&provider.id);
        div().child(match tab_index {
            0 => div().child(Self::render_config_content_static(provider)),
            1 => {
                // 从缓存获取资源信息 - 使用静态方法
                div().child(Self::render_resources_content_static(&resources))
            }
            2 => {
                // 资源模板 - 暂时显示空内容
                div().child(Self::render_resource_templates_content_static(
                    &resource_templates,
                ))
            }
            3 => {
                // 从缓存获取工具信息
                div().child(Self::render_tools_content_static(&tools))
            }
            4 => {
                // 从缓存获取提示信息
                div().child(Self::render_prompts_content_static(&prompts))
            }
            _ => div().child("未知能力"),
        })
    }

    // 保留这些静态渲染方法
    fn render_config_content_static(provider: &McpServerConfig) -> impl IntoElement {
        v_flex().gap_3().child(
            div()
                .p_3()
                .bg(gpui::rgb(0xFAFAFA))
                .rounded_md()
                .border_1()
                .border_color(gpui::rgb(0xE5E7EB))
                .child(
                    v_flex().gap_3().child(
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
                                        div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x111827))
                                            .child(provider.command.clone()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_4()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x6B7280))
                                            .min_w_20()
                                            .child("传输方式:"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x111827))
                                            .child(provider.transport.as_str()),
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
                                        .children(provider.env_vars.iter().map(|(key, value)| {
                                            div()
                                                .pl_4()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x9CA3AF))
                                                .child(format!("{}={}", key, value))
                                        })),
                                )
                            }),
                    ),
                ),
        )
    }

    // 渲染资源的静态方法
    fn render_resources_content_static(resources: &[ResourceDefinition]) -> impl IntoElement {
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
                                    .child(SharedString::new(resource.resource.name.clone())),
                            )
                            .when_some(resource.resource.mime_type.clone(), |this, mime_type| {
                                this.child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(gpui::rgb(0xE0F2FE))
                                        .text_color(gpui::rgb(0x0369A1))
                                        .rounded_md()
                                        .text_xs()
                                        .child(mime_type),
                                )
                            }),
                    )
                    .when_some(
                        resource.resource.description.clone(),
                        |this, description| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child(description),
                            )
                        },
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(gpui::rgb(0x9CA3AF))
                            .child(resource.resource.uri.clone()),
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

    // 渲染资源的静态方法
    fn render_resource_templates_content_static(
        resources: &[ResourceTemplateDefinition],
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
                                div().font_medium().text_color(gpui::rgb(0x111827)).child(
                                    SharedString::new(resource.resource_template.name.clone()),
                                ),
                            )
                            .when_some(
                                resource.resource_template.mime_type.clone(),
                                |this, mime_type| {
                                    this.child(
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(gpui::rgb(0xE0F2FE))
                                            .text_color(gpui::rgb(0x0369A1))
                                            .rounded_md()
                                            .text_xs()
                                            .child(mime_type),
                                    )
                                },
                            ),
                    )
                    .when_some(
                        resource.resource_template.description.clone(),
                        |this, description| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child(description),
                            )
                        },
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(gpui::rgb(0x9CA3AF))
                            .child(resource.resource_template.uri_template.clone()),
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

    // 渲染工具的静态方法
    fn render_tools_content_static(tools: &[McpTool]) -> impl IntoElement {
        v_flex()
            .gap_2()
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
                                    .text_color(gpui::rgb(0x0369A1)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child(SharedString::new(tool.name.clone())),
                            ),
                    )
                    .when_some(tool.description.clone(), |this, description| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(gpui::rgb(0x6B7280))
                                .child(SharedString::new(description)),
                        )
                    })
                    .when(!tool.input_schema.is_empty(), |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(gpui::rgb(0x9CA3AF))
                                .child("输入参数已定义"),
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
            .gap_2()
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
                                Icon::new(IconName::SquarePen)
                                    .small()
                                    .text_color(gpui::rgb(0x9333EA)),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(gpui::rgb(0x111827))
                                    .child(prompt.name.clone()),
                            ),
                    )
                    .when_some(prompt.description.clone(), |this, description| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(gpui::rgb(0x6B7280))
                                .child(description),
                        )
                    })
                    .when_some(prompt.arguments.as_ref(), |this, arguments| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(gpui::rgb(0x9CA3AF))
                                .child(format!("参数: {}", arguments.len())),
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

    // 删除事件监听方法，因为还没有实现相应的事件系统
    // 删除 subscribe_to_mcp_events 方法
}

// 删除不需要的实现，保留核心的 ViewKit, Focusable, Render 等
