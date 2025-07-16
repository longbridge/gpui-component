use crate::app::FoEvent;
use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::config::llm_config::{ApiType, LlmProviderConfig, LlmProviderManager, ModelInfo};
use crate::ui::components::ViewKit;
use gpui::prelude::*;
use gpui::*;
use gpui_component::tooltip::Tooltip;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
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
    providers: Vec<LlmProviderConfig>,
    expanded_providers: Vec<usize>,
    active_provider_tabs: std::collections::HashMap<usize, usize>,
    editing_provider: Option<usize>,
    // 每个Provider的编辑状态输入框
    provider_inputs: std::collections::HashMap<usize, ProviderInputs>,
    _subscriptions: Vec<Subscription>,
    // llm_provider_manager: LlmProviderManager,
}

impl ViewKit for LlmProvider {
    fn title() -> &'static str {
        "服务提供商"
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

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        // let llm_provider_manager = LlmProviderManager::load();
        Self {
            focus_handle: cx.focus_handle(),
            providers: LlmProviderManager::list_providers(),
            expanded_providers: vec![],
            active_provider_tabs: std::collections::HashMap::new(),
            editing_provider: None,
            provider_inputs: std::collections::HashMap::new(),
            _subscriptions: vec![],
            //llm_provider_manager,
        }
    }

    // 刷新提供商列表
    fn refresh_providers(&mut self, cx: &mut Context<Self>) {
        self.providers = LlmProviderManager::list_providers();
        cx.notify();
    }

    // 保存配置到文件
    fn save_config(&mut self, cx: &mut Context<Self>) {
        println!("正在保存配置...");
        if let Err(e) = LlmProviderManager::save_providers(&self.providers[..]) {
            eprintln!("保存配置失败: {}", e);
        }
        CrossRuntimeBridge::global().emit(&FoEvent::LlmConfigUpdated);
    }

    fn tab(&mut self, _: &Tab1, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn add_provider(&mut self, _: &AddProvider, window: &mut Window, cx: &mut Context<Self>) {
        let new_provider = LlmProviderConfig::default();
        let new_index = self.providers.len();
        self.providers.push(new_provider);
        self.expanded_providers.push(new_index);
        self.start_editing(new_index, window, cx);
        self.save_config(cx);
        cx.notify();
    }

    fn cancel_edit(&mut self, _: &CancelEdit, _: &mut Window, cx: &mut Context<Self>) {
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
                // 更新提供商信息
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

                // 检查是否为新创建的提供商
                let is_new_provider = provider.id.is_empty()
                    || LlmProviderManager::get_provider(&provider.id).is_none();

                if is_new_provider {
                    // 新建提供商 - 使用 add_provider
                    provider.id = uuid::Uuid::new_v4().to_string();
                    match LlmProviderManager::add_provider(provider.clone()) {
                        Ok(id) => {
                            provider.id = id;
                            window.push_notification(
                                format!("成功添加服务提供商 \"{}\"", provider.name),
                                cx,
                            );
                            self.save_config(cx);
                            self.refresh_providers(cx);
                        }
                        Err(e) => {
                            window.push_notification(format!("添加服务提供商失败: {}", e), cx);
                            return;
                        }
                    }
                } else {
                    // 更新现有提供商 - 使用 update_provider
                    match LlmProviderManager::update_provider(&provider.id, provider.clone()) {
                        Ok(_) => {
                            window.push_notification(
                                format!("成功更新服务提供商 \"{}\"", provider.name),
                                cx,
                            );
                            self.save_config(cx);
                            self.refresh_providers(cx);
                        }
                        Err(e) => {
                            window.push_notification(format!("更新服务提供商失败: {}", e), cx);
                            return;
                        }
                    }
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
            let provider = &self.providers[index];
            let provider_name = provider.name.clone();
            let provider_id = provider.id.clone();

            // 使用 LlmProviderManager 删除提供商
            match LlmProviderManager::delete_provider(&provider_id) {
                Ok(_) => {
                    // 从本地列表中删除
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
                    // 保存配置并刷新
                    self.save_config(cx);
                    window.push_notification(
                        format!("已成功删除服务提供商 \"{}\"", provider_name),
                        cx,
                    );
                    cx.notify();
                }
                Err(e) => {
                    window.push_notification(format!("删除服务提供商失败: {}", e), cx);
                }
            }
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
            let provider_id = provider.id.clone();

            // 使用 LlmProviderManager 切换启用状态
            match LlmProviderManager::toggle_provider(&provider_id, enabled) {
                Ok(_) => {
                    provider.enabled = enabled;

                    // 如果禁用提供商，自动关闭其 accordion
                    if !enabled {
                        self.expanded_providers.retain(|&i| i != index);
                    }

                    // 保存配置
                    self.save_config(cx);
                    cx.notify();
                }
                Err(e) => {
                    eprintln!("切换服务提供商状态失败: {}", e);
                }
            }
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
            // 克隆provider在修改之前
            let provider_id = provider.id.clone();
            let mut provider_clone = provider.clone();

            if let Some(model) = provider.models.get_mut(model_index) {
                // 更新本地模型状态
                model.enabled = enabled;

                // 同时更新clone中的模型状态
                if let Some(clone_model) = provider_clone.models.get_mut(model_index) {
                    clone_model.enabled = enabled;
                }

                // 同步到 LlmProviderManager 并保存
                match LlmProviderManager::update_provider(&provider_id, provider_clone) {
                    Ok(_) => {
                        self.save_config(cx);
                        cx.notify();
                    }
                    Err(e) => {
                        // 如果保存失败，回滚本地状态
                        model.enabled = !enabled;
                        eprintln!("切换模型状态失败: {}", e);
                    }
                }
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
        // let provider = self.providers.get(provider_index).cloned();
        // provider.map(|provider| {
        //     if tab_index == 1 {
        //         cx.spawn(async move |this, cx| {
        //             let timeout_duration = std::time::Duration::from_secs(5); // 30秒超时
        //             let models_future = provider.models
        //             let this = this.clone();
        //             match tokio::time::timeout(timeout_duration, models_future).await {
        //                 Ok(models_result) => {
        //                     this.update(cx, |this, cx| match models_result {
        //                         Ok(models) => {
        //                             if let Some(provider) = this.providers.get_mut(provider_index) {
        //                                 provider.models = models;
        //                                 LlmProviderManager::update_provider(
        //                                     &provider.id,
        //                                     provider.clone(),
        //                                 )
        //                                 .unwrap_or_else(
        //                                     |e| {
        //                                         eprintln!("更新模型列表失败: {}", e);
        //                                     },
        //                                 );
        //                             }
        //                             cx.notify();
        //                         }
        //                         Err(e) => {
        //                             eprintln!("加载模型列表失败: {}", e);
        //                         }
        //                     })
        //                     .ok();
        //                 }
        //                 Err(_) => {
        //                     eprintln!("加载模型列表超时（5秒）");
        //                 }
        //             }
        //         })
        //         .detach();
        //     }
        // });
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

    fn render_config_content_static(provider: &LlmProviderConfig) -> impl IntoElement {
        v_flex().gap_4().child(
            v_flex()
                .gap_2()
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
                let model_id = model.id.clone();
                let model_name = model.display_name.clone();
                let model_enabled = model.enabled;
                let model_capabilities = model.capabilities.clone();
                let model_limits = model.limits.clone();
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
                                    .id(SharedString::new(format!(
                                        "model-{}-{}",
                                        provider_index, model_index
                                    )))
                                    .font_medium()
                                    .text_color(if model_enabled {
                                        gpui::rgb(0x111827)
                                    } else {
                                        gpui::rgb(0xD1D5DB)
                                    })
                                    .child(model_name.clone())
                                    .tooltip(move |window, cx| {
                                        Tooltip::new(model_id.clone()).build(window, cx)
                                    }),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .text_xs()
                                    .font_medium()
                                    .when_some(
                                        model_limits.context_length,
                                        |this, context_length| {
                                            this.child(
                                                h_flex()
                                                    .items_center()
                                                    .px_1()
                                                    .text_xs()
                                                    // .bg(gpui::rgb(0x7C3AED))
                                                    .rounded_md()
                                                    .text_color(gpui::rgb(0x374151))
                                                    .child(context_length.to_string()),
                                            )
                                        },
                                    )
                                    .when_some(
                                        model_limits.max_output_tokens,
                                        |this, max_output_tokens| {
                                            this.child(
                                                h_flex()
                                                    .items_center()
                                                    .px_1()
                                                    .text_xs()
                                                    //  .bg(gpui::rgb(0x7C3AED))
                                                    .rounded_md()
                                                    .text_color(gpui::rgb(0x374151))
                                                    .child(max_output_tokens.to_string()),
                                            )
                                        },
                                    ),
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
                // .child(
                //     h_flex().items_center().gap_2().child(
                //         // 修复：使用二元组格式，参考mcp_provider.rs的实现
                //         Switch::new(("model-enabled", provider_index * 1000 + model_index))
                //             .checked(model_enabled)
                //             .on_click(cx.listener(move |this, checked, window, cx| {
                //                 this.toggle_model_enabled(
                //                     provider_index,
                //                     model_index,
                //                     *checked,
                //                     window,
                //                     cx,
                //                 );
                //             })),
                //     ),
                // )
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
                        .label("添加服务商")
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
