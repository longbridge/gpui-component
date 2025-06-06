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
    tab::{Tab, TabBar},
    v_flex, ContextModal, Disableable, FocusableCycle, Icon, IconName, Sizable, StyledExt,
};

actions!(
    provider,
    [
        Tab1,
        TabPrev,
        AddProvider,
        SaveProvider,
        DeleteProvider,
        CancelEdit,
    ]
);

const CONTEXT: &str = "LlmProvider";

#[derive(Debug, Clone)]
pub enum ApiType {
    OpenAI,
    OpenAIResponse,
    Gemini,
    Anthropic,
    AzureOpenAI,
}

impl ApiType {
    fn as_str(&self) -> &'static str {
        match self {
            ApiType::OpenAI => "OpenAI",
            ApiType::OpenAIResponse => "OpenAI-Response",
            ApiType::Gemini => "Gemini",
            ApiType::Anthropic => "Anthropic",
            ApiType::AzureOpenAI => "Azure-OpenAI",
        }
    }

    fn all() -> Vec<SharedString> {
        vec![
            "OpenAI".into(),
            "OpenAI-Response".into(),
            "Gemini".into(),
            "Anthropic".into(),
            "Azure-OpenAI".into(),
        ]
    }
}

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
            ModelCapability::Tools => IconName::Wrench, // 修正拼写错误
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

#[derive(Debug, Clone)]
pub struct ModelInfo {
    name: String,
    capabilities: Vec<ModelCapability>,
    enabled: bool,
}

#[derive(Debug, Clone)]
pub struct LlmProviderInfo {
    id: String,
    name: String,
    api_url: String,
    api_key: String,
    api_type: ApiType,
    enabled: bool,
    models: Vec<ModelInfo>,
}

impl Default for LlmProviderInfo {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            api_url: String::new(),
            api_key: String::new(),
            api_type: ApiType::OpenAI,
            enabled: true,
            models: vec![
                ModelInfo {
                    name: "gpt-4o".to_string(),
                    capabilities: vec![
                        ModelCapability::Text,
                        ModelCapability::Vision,
                        ModelCapability::Tools,
                    ],
                    enabled: true,
                },
                ModelInfo {
                    name: "gpt-4o-mini".to_string(),
                    capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                    enabled: true,
                },
            ],
        }
    }
}

// 用于存储每个Provider的编辑状态输入框
#[derive(Clone)]
struct ProviderInputs {
    name_input: Entity<InputState>,
    api_url_input: Entity<InputState>,
    api_key_input: Entity<InputState>,
    api_type_dropdown: Entity<DropdownState<Vec<SharedString>>>,
}

pub struct LlmProvider {
    focus_handle: FocusHandle,
    providers: Vec<LlmProviderInfo>,
    expanded_providers: Vec<usize>,
    active_provider_tabs: std::collections::HashMap<usize, usize>,
    editing_provider: Option<usize>,
    // 每个Provider的编辑状态输入框
    provider_inputs: std::collections::HashMap<usize, ProviderInputs>,
    _subscriptions: Vec<Subscription>,
}

impl ViewKit for LlmProvider {
    fn title() -> &'static str {
        "LLM服务提供商"
    }

    fn description() -> &'static str {
        "配置和管理LLM服务提供商"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl LlmProvider {
    pub fn init(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
            KeyBinding::new("tab", Tab1, Some(CONTEXT)),
            KeyBinding::new("ctrl-n", AddProvider, Some(CONTEXT)),
            KeyBinding::new("escape", CancelEdit, Some(CONTEXT)),
        ])
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 初始化示例数据保持不变
        let mut default_provider = LlmProviderInfo::default();
        default_provider.name = "收钱吧".to_string();
        default_provider.api_url = "https://hcb.aliyunddos1117.com/v1".to_string();

        let mut anthropic_provider = LlmProviderInfo::default();
        anthropic_provider.name = "Anthropic".to_string();
        anthropic_provider.api_url = "https://api.anthropic.com".to_string();
        anthropic_provider.api_type = ApiType::Anthropic;
        anthropic_provider.models = vec![
            ModelInfo {
                name: "claude-3.5-sonnet".to_string(),
                capabilities: vec![
                    ModelCapability::Text,
                    ModelCapability::Vision,
                    ModelCapability::Tools,
                ],
                enabled: true,
            },
            ModelInfo {
                name: "claude-3-haiku".to_string(),
                capabilities: vec![ModelCapability::Text, ModelCapability::Tools],
                enabled: true,
            },
        ];

        Self {
            focus_handle: cx.focus_handle(),
            providers: vec![default_provider, anthropic_provider],
            expanded_providers: vec![],
            active_provider_tabs: std::collections::HashMap::new(),
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

    fn add_provider(&mut self, _: &AddProvider, window: &mut Window, cx: &mut Context<Self>) {
        let new_index = self.providers.len();
        self.providers.push(LlmProviderInfo::default());
        self.expanded_providers.push(new_index);
        self.start_editing(new_index, window, cx);
        cx.notify();
    }

    fn cancel_edit(&mut self, _: &CancelEdit, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editing_index) = self.editing_provider {
            // 如果是新添加的空Provider，删除它
            if let Some(provider) = self.providers.get(editing_index) {
                if provider.name.is_empty() && provider.api_url.is_empty() {
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

    fn save_provider(&mut self, _: &SaveProvider, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.editing_provider {
            if let (Some(provider), Some(inputs)) = (
                self.providers.get_mut(index),
                self.provider_inputs.get(&index),
            ) {
                provider.name = inputs.name_input.read(cx).value().to_string();
                provider.api_url = inputs.api_url_input.read(cx).value().to_string();
                provider.api_key = inputs.api_key_input.read(cx).value().to_string();

                if let Some(selected) = inputs.api_type_dropdown.read(cx).selected_value() {
                    provider.api_type = match selected.as_ref() {
                        "OpenAI" => ApiType::OpenAI,
                        "OpenAI-Response" => ApiType::OpenAIResponse,
                        "Gemini" => ApiType::Gemini,
                        "Anthropic" => ApiType::Anthropic,
                        "Azure-OpenAI" => ApiType::AzureOpenAI,
                        _ => ApiType::OpenAI,
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

    fn delete_provider(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
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
                    "确定要删除服务提供商 \"{}\" 吗？\n\n此操作无法撤销。",
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
                            this.confirm_delete_provider(index, window, cx);
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

    fn confirm_delete_provider(
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

            window.push_notification(format!("已成功删除服务提供商 \"{}\"", provider_name), cx);
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

    fn toggle_model_enabled(
        &mut self,
        provider_index: usize,
        model_index: usize,
        enabled: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(provider) = self.providers.get_mut(provider_index) {
            if let Some(model) = provider.models.get_mut(model_index) {
                model.enabled = enabled;
                cx.notify();
            }
        }
    }

    fn start_editing(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_provider = Some(index);

        let provider = &self.providers[index];

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("服务提供商名称")
                .default_value(&provider.name)
        });

        let api_url_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("API 地址")
                .default_value(&provider.api_url)
        });

        let api_key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("API 密钥")
                .masked(true)
                .default_value(&provider.api_key)
        });

        let type_index = match provider.api_type {
            ApiType::OpenAI => 0,
            ApiType::OpenAIResponse => 1,
            ApiType::Gemini => 2,
            ApiType::Anthropic => 3,
            ApiType::AzureOpenAI => 4,
        };

        let api_type_dropdown =
            cx.new(|cx| DropdownState::new(ApiType::all(), Some(type_index), window, cx));

        self.provider_inputs.insert(
            index,
            ProviderInputs {
                name_input,
                api_url_input,
                api_key_input,
                api_type_dropdown,
            },
        );

        cx.notify();
    }

    fn toggle_accordion(&mut self, open_ixs: &[usize], _: &mut Window, cx: &mut Context<Self>) {
        self.expanded_providers = open_ixs.to_vec();
        cx.notify();
    }

    fn set_active_provider_tab(
        &mut self,
        provider_index: usize,
        tab_index: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_provider_tabs.insert(provider_index, tab_index);
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
            //         self.save_provider(&SaveProvider, window, cx);
            //     }
            // }
            _ => {}
        };
    }

    // 静态方法渲染Tab内容
    fn render_provider_content_static(
        provider: &LlmProviderInfo,
        tab_index: usize,
    ) -> impl IntoElement {
        div().child(match tab_index {
            0 => div().child(Self::render_config_content_static(provider)),
            1 => div().child(Self::render_models_content_static(&provider.models)),
            _ => div().child("未知Tab"),
        })
    }

    fn render_config_content_static(provider: &LlmProviderInfo) -> impl IntoElement {
        v_flex().gap_4().child(
            v_flex()
                .gap_2()
                // .child(
                //     h_flex()
                //         .gap_4()
                //         // .child(
                //         //     v_flex()
                //         //         .gap_1()
                //         //         .flex_1()
                //         //         .child(
                //         //             div()
                //         //                 .text_sm()
                //         //                 .font_medium()
                //         //                 .text_color(gpui::rgb(0x374151))
                //         //                 .child("API 类型"),
                //         //         )
                //         //         .child(
                //         //             div()
                //         //                 .px_2()
                //         //                 .bg(gpui::rgb(0xDDD6FE))
                //         //                 .text_color(gpui::rgb(0x7C3AED))
                //         //                 .rounded_md()
                //         //                 .text_sm()
                //         //                 .child(provider.api_type.as_str()),
                //         //         ),
                //         // )
                //         .child(
                //             v_flex()
                //                 .gap_1()
                //                 .flex_1()
                //                 .child(
                //                     div()
                //                         .text_sm()
                //                         .font_medium()
                //                         .text_color(gpui::rgb(0x374151))
                //                         .child("服务状态"),
                //                 )
                //                 .child(
                //                     h_flex()
                //                         .items_center()
                //                         .gap_2()
                //                         .child(
                //                             Icon::new(if provider.enabled {
                //                                 IconName::CircleCheck
                //                             } else {
                //                                 IconName::CircleX
                //                             })
                //                             .small()
                //                             .text_color(if provider.enabled {
                //                                 gpui::rgb(0x059669)
                //                             } else {
                //                                 gpui::rgb(0xDC2626)
                //                             }),
                //                         )
                //                         .child(
                //                             div()
                //                                 .text_sm()
                //                                 .text_color(if provider.enabled {
                //                                     gpui::rgb(0x059669)
                //                                 } else {
                //                                     gpui::rgb(0xDC2626)
                //                                 })
                //                                 .child(if provider.enabled {
                //                                     "已启用"
                //                                 } else {
                //                                     "已禁用"
                //                                 }),
                //                         ),
                //                 ),
                //         ),
                // )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .text_color(gpui::rgb(0x374151))
                                .child("API 地址"),
                        )
                        .child(
                            div()
                                .p_2()
                                .bg(gpui::rgb(0xF9FAFB))
                                .rounded_md()
                                .border_1()
                                .border_color(gpui::rgb(0xE5E7EB))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui::rgb(0x374151))
                                        .font_light()
                                        .child(if provider.api_url.is_empty() {
                                            "未设置API地址".to_string()
                                        } else {
                                            provider.api_url.clone()
                                        }),
                                ),
                        ),
                )
                .when(!provider.api_url.is_empty(), |divv| {
                    divv.child(v_flex().gap_1().child(
                        div().text_xs().text_color(gpui::rgb(0x9CA3AF)).child({
                            let full_url = if provider.api_url.is_empty() {
                                "".to_string()
                            } else {
                                format!(
                                    "{}/chat/completions",
                                    provider.api_url.trim_end_matches('/')
                                )
                            };
                            full_url
                        }),
                    ))
                })
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .text_color(gpui::rgb(0x374151))
                                .child("API 密钥"),
                        )
                        .child(div().text_sm().text_color(gpui::rgb(0x6B7280)).child(
                            if provider.api_key.is_empty() {
                                "未配置"
                            } else {
                                "••••••••"
                            },
                        )),
                ),
        )
    }

    fn render_models_content_static(models: &[ModelInfo]) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(models.iter().enumerate().map(|(model_index, model)| {
                let model_name = model.name.clone();
                let model_enabled = model.enabled;
                let model_capabilities = model.capabilities.clone();

                h_flex()
                    .items_center()
                    .justify_between()
                    .p_1()
                    .bg(gpui::rgb(0xFAFAFA))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(if model_enabled {
                                        gpui::rgb(0x111827)
                                    } else {
                                        gpui::rgb(0xD1D5DB)
                                    })
                                    .child(model_name.clone()),
                            )
                            .child(
                                h_flex().gap_1().items_center().children(
                                    model_capabilities.iter().enumerate().map(
                                        |(cap_index, cap)| {
                                            let capability_unique_id =
                                                model_index * 1000 + cap_index;

                                            div()
                                                .id(("capability", capability_unique_id))
                                                .p_1()
                                                .rounded_md()
                                                .bg(if model_enabled {
                                                    gpui::rgb(0xF3F4F6)
                                                } else {
                                                    gpui::rgb(0xFAFAFA)
                                                })
                                                .child(
                                                    Icon::new(cap.icon())
                                                        .xsmall()
                                                        .when(!model_enabled, |icon| {
                                                            icon.text_color(gpui::rgb(0xD1D5DB))
                                                        }),
                                                )
                                        },
                                    ),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(if model_enabled {
                                gpui::rgb(0xDEF7EC)
                            } else {
                                gpui::rgb(0xFEF2F2)
                            })
                            .text_color(if model_enabled {
                                gpui::rgb(0x047857)
                            } else {
                                gpui::rgb(0xDC2626)
                            })
                            .rounded_md()
                            .text_xs()
                            .child(if model_enabled {
                                "已启用"
                            } else {
                                "已禁用"
                            }),
                    )
            }))
            .when(models.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用模型"),
                )
            })
    }

    // 添加非静态方法来渲染带Switch的模型列表
    fn render_models_content_with_switch(
        &self,
        models: &[ModelInfo],
        provider_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(models.iter().enumerate().map(|(model_index, model)| {
                let model_name = model.name.clone();
                let model_enabled = model.enabled;
                let model_capabilities = model.capabilities.clone();

                h_flex()
                    .items_center()
                    .justify_between()
                    .p_1()
                    .bg(gpui::rgb(0xFAFAFA))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(if model_enabled {
                                        gpui::rgb(0x111827)
                                    } else {
                                        gpui::rgb(0xD1D5DB)
                                    })
                                    .child(model_name.clone()),
                            )
                            .child(
                                h_flex().gap_1().items_center().children(
                                    model_capabilities.iter().enumerate().map(
                                        |(cap_index, cap)| {
                                            let capability_unique_id = provider_index * 10000
                                                + model_index * 1000
                                                + cap_index;

                                            div()
                                                .id(("capability", capability_unique_id))
                                                .p_1()
                                                .rounded_md()
                                                .bg(if model_enabled {
                                                    gpui::rgb(0xF3F4F6)
                                                } else {
                                                    gpui::rgb(0xFAFAFA)
                                                })
                                                .child(
                                                    Icon::new(cap.icon())
                                                        .xsmall()
                                                        .when(!model_enabled, |icon| {
                                                            icon.text_color(gpui::rgb(0xD1D5DB))
                                                        }),
                                                )
                                        },
                                    ),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            // .child(
                            //     div()
                            //         .px_2()
                            //         .py_1()
                            //         .bg(if model_enabled {
                            //             gpui::rgb(0xDEF7EC)
                            //         } else {
                            //             gpui::rgb(0xFEF2F2)
                            //         })
                            //         .text_color(if model_enabled {
                            //             gpui::rgb(0x047857)
                            //         } else {
                            //             gpui::rgb(0xDC2626)
                            //         })
                            //         .rounded_md()
                            //         .text_xs()
                            //         .child(if model_enabled {
                            //             "已启用"
                            //         } else {
                            //             "已禁用"
                            //         }),
                            // )
                            .child(
                                // 修复：使用二元组格式，参考mcp_provider.rs的实现
                                Switch::new(("model-enabled", provider_index * 1000 + model_index))
                                    .checked(model_enabled)
                                    .on_click(cx.listener(move |this, checked, window, cx| {
                                        this.toggle_model_enabled(
                                            provider_index,
                                            model_index,
                                            *checked,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    )
            }))
            .when(models.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x9CA3AF))
                        .child("暂无可用模型"),
                )
            })
    }
}

impl FocusableCycle for LlmProvider {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        if let Some(editing_index) = self.editing_provider {
            if let Some(inputs) = self.provider_inputs.get(&editing_index) {
                return vec![
                    inputs.name_input.focus_handle(cx),
                    inputs.api_url_input.focus_handle(cx),
                    inputs.api_key_input.focus_handle(cx),
                    inputs.api_type_dropdown.focus_handle(cx),
                ];
            }
        }
        vec![self.focus_handle.clone()]
    }
}

impl Focusable for LlmProvider {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LlmProvider {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("llm-provider")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::add_provider))
            .on_action(cx.listener(Self::save_provider))
            .on_action(cx.listener(Self::cancel_edit))
            .size_full()
            .gap_4()
            .child(
                h_flex().justify_start().child(
                    Button::new("add-provider")
                        .with_variant(ButtonVariant::Primary)
                        .label("添加提供商")
                        .icon(IconName::Plus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_provider(&AddProvider, window, cx);
                        })),
                ),
            )
            .child(div().w_full().child({
                let mut accordion = Accordion::new("providers").multiple(true);

                let expanded_providers = self.expanded_providers.clone();
                let editing_provider = self.editing_provider;
                let active_provider_tabs = self.active_provider_tabs.clone();

                let edit_inputs = if let Some(editing_index) = editing_provider {
                    self.provider_inputs.get(&editing_index).map(|inputs| {
                        (
                            inputs.name_input.clone(),
                            inputs.api_url_input.clone(),
                            inputs.api_key_input.clone(),
                            inputs.api_type_dropdown.clone(),
                        )
                    })
                } else {
                    None
                };

                for (index, provider) in self.providers.iter().enumerate() {
                    let provider_name = provider.name.clone();
                    let provider_api_type = provider.api_type.as_str().to_string();
                    let provider_enabled = provider.enabled;
                    let provider_clone = provider.clone();
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
                                                "新建服务提供商".to_string()
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
                                                        gpui::rgb(0xDDD6FE)
                                                    } else {
                                                        gpui::rgb(0xF3F4F6)
                                                    })
                                                    .text_color(if provider_enabled {
                                                        gpui::rgb(0x7C3AED)
                                                    } else {
                                                        gpui::rgb(0xD1D5DB)
                                                    })
                                                    .rounded_md()
                                                    .text_xs()
                                                    .whitespace_nowrap()
                                                    .child(provider_api_type.clone()),
                                            )
                                            .child(
                                                Switch::new(("provider-enabled", index))
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
                                                Button::new(("edit-provider", index))
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
                                                Button::new(("delete-provider", index))
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
                                                            this.delete_provider(index, window, cx);
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            .content(v_flex().gap_4().child(if is_editing {
                                // 内联编辑表单
                                if let Some((
                                    name_input,
                                    api_url_input,
                                    api_key_input,
                                    api_type_dropdown,
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
                                                                    .child("名称 *"),
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
                                                                    .child("接口类型"),
                                                            )
                                                            .child(
                                                                Dropdown::new(api_type_dropdown)
                                                                    .placeholder("选择接口类型")
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
                                                            .child("API 地址 *"),
                                                    )
                                                    .child(
                                                        TextInput::new(api_url_input).cleanable(),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(gpui::rgb(0x6B7280))
                                                            .child("API 密钥 *"),
                                                    )
                                                    .child(
                                                        TextInput::new(api_key_input)
                                                            .cleanable()
                                                            .mask_toggle(),
                                                    ),
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
                                                                    this.save_provider(
                                                                        &SaveProvider,
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
                                // Tab页显示配置信息和模型列表
                                div().child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            TabBar::new(("provider-tabs", index))
                                                .w_full()
                                                .pill()
                                                .small()
                                                .selected_index(
                                                    active_provider_tabs
                                                        .get(&index)
                                                        .copied()
                                                        .unwrap_or(0),
                                                )
                                                .child(Tab::new("配置信息"))
                                                .child(Tab::new("模型列表"))
                                                .on_click(cx.listener(
                                                    move |this, tab_ix: &usize, window, cx| {
                                                        this.set_active_provider_tab(
                                                            index, *tab_ix, window, cx,
                                                        );
                                                    },
                                                )),
                                        )
                                        .child(
                                            div().mt_2().child(
                                                match active_provider_tabs
                                                    .get(&index)
                                                    .copied()
                                                    .unwrap_or(0)
                                                {
                                                    0 => div().child(
                                                        Self::render_config_content_static(
                                                            &provider_clone,
                                                        ),
                                                    ),
                                                    1 => div().child(
                                                        self.render_models_content_with_switch(
                                                            &provider_clone.models,
                                                            index,
                                                            cx,
                                                        ),
                                                    ),
                                                    _ => div().child("未知Tab"),
                                                },
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
