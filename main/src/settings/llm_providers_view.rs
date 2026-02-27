use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Context, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, SharedString, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants},
    dialog::DialogButtonProps,
    h_flex, v_flex,
};
use one_core::llm::{storage::ProviderRepository, types::ProviderConfig};
use one_core::llm::types::BUILTIN_ONET_CLI_ID;
use one_core::storage::{GlobalStorageState, StorageManager, traits::Repository};
use rust_i18n::t;

use super::provider_form_dialog::ProviderForm;
use crate::setting_tab::GlobalCurrentUser;

pub struct LlmProvidersView {
    focus_handle: FocusHandle,
    storage_manager: StorageManager,
    providers: Vec<ProviderConfig>,
    loading: bool,
    loaded: bool,
    is_logged_in: bool,
}

impl LlmProvidersView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let storage_state = cx.global::<GlobalStorageState>();
        let storage_manager = storage_state.storage.clone();

        Self {
            focus_handle,
            storage_manager,
            providers: vec![],
            loading: false,
            loaded: false,
            is_logged_in: GlobalCurrentUser::get_user(cx).is_some(),
        }
    }

    fn load_providers(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.loaded = true;
        let is_logged_in = GlobalCurrentUser::get_user(cx).is_some();
        self.is_logged_in = is_logged_in;

        let repo = self
            .storage_manager
            .get::<ProviderRepository>()
            .expect("ProviderRepository not found");

        if is_logged_in {
            if let Err(e) = repo.ensure_onetcli_provider() {
                tracing::error!("Failed to ensure OnetCli provider: {}", e);
            }
        }

        match repo.list() {
            Ok(mut providers) => {
                if !is_logged_in {
                    providers.retain(|p| !p.is_builtin());
                }
                self.providers = providers;
                self.loading = false;
            }
            Err(e) => {
                tracing::error!("Failed to load providers: {}", e);
                self.loading = false;
            }
        }
        cx.notify();
    }

    fn add_provider(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_provider_form(None, cx, window);
    }

    fn edit_provider(&mut self, provider_id: i64, window: &mut Window, cx: &mut Context<Self>) {
        // 查找要编辑的 provider
        let provider = self.providers.iter().find(|p| p.id == provider_id).cloned();
        self.open_provider_form(provider, cx, window);
    }

    fn open_provider_form(
        &mut self,
        provider: Option<ProviderConfig>,
        cx: &mut Context<Self>,
        window: &mut Window,
    ) {
        let is_update = provider.is_some();
        let storage_manager = self.storage_manager.clone();
        let form = cx.new(|cx| ProviderForm::new_with_config(provider, window, cx));
        let form_for_ok = form.clone();
        let storage_manager_for_ok = storage_manager.clone();
        let view = cx.entity().clone();

        window.open_dialog(cx, move |dialog, _, _| {
            let form_clone = form_for_ok.clone();
            let storage_clone = storage_manager_for_ok.clone();
            let view_clone = view.clone();

            dialog
                .title(if is_update {
                    t!("LlmProviders.dialog_edit_title").to_string()
                } else {
                    t!("LlmProviders.dialog_add_title").to_string()
                })
                .child(form.clone())
                .confirm()
                .button_props(DialogButtonProps::default().ok_text(if is_update {
                    t!("Common.save")
                } else {
                    t!("LlmProviders.dialog_add_action")
                }))
                .on_ok(move |_, window, cx| {
                    let config_opt = form_clone.update(cx, |form, cx| form.get_config(cx));

                    let Some(mut config) = config_opt else {
                        window.push_notification(t!("LlmProviders.required_notice").to_string(), cx);
                        return false;
                    };

                    let repo = storage_clone
                        .get::<ProviderRepository>()
                        .expect("ProviderRepository not found");

                    if config.is_default {
                        if let Ok(existing) = repo.list() {
                            for mut item in existing {
                                if item.id != config.id && item.is_default {
                                    item.is_default = false;
                                    if let Err(e) = repo.update(&item) {
                                        tracing::error!("Failed to unset default provider: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    let result = if is_update {
                        repo.update(&config)
                    } else {
                        repo.insert(&mut config).map(|_| ())
                    };

                    match result {
                        Ok(_) => {
                            _ = view_clone.update(cx, |view, cx| {
                                view.load_providers(cx);
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to save provider: {}", e);
                        }
                    }
                    true
                })
        });
    }

    fn delete_provider(&mut self, provider_id: i64, cx: &mut Context<Self>) {
        if provider_id == BUILTIN_ONET_CLI_ID {
            return;
        }
        let repo = self
            .storage_manager
            .get::<ProviderRepository>()
            .expect("ProviderRepository not found");

        match repo.delete(provider_id) {
            Ok(_) => {
                self.load_providers(cx);
            }
            Err(e) => {
                tracing::error!("Failed to delete provider: {}", e);
            }
        }
    }

    fn toggle_provider(&mut self, mut provider: ProviderConfig, cx: &mut Context<Self>) {
        if provider.is_builtin() {
            return;
        }
        provider.enabled = !provider.enabled;

        let repo = self
            .storage_manager
            .get::<ProviderRepository>()
            .expect("ProviderRepository not found");

        match repo.update(&provider) {
            Ok(_) => {
                self.load_providers(cx);
            }
            Err(e) => {
                tracing::error!("Failed to toggle provider: {}", e);
            }
        }
    }
}

impl Render for LlmProvidersView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_logged_in = GlobalCurrentUser::get_user(cx).is_some();
        if is_logged_in != self.is_logged_in {
            self.is_logged_in = is_logged_in;
            self.loaded = false;
        }

        // 第一次渲染时开始加载
        if !self.loaded && !self.loading {
            self.load_providers(cx);
        }

        v_flex()
            .size_full()
            .gap_4()
            .p_6()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(t!("LlmProviders.title").to_string()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("LlmProviders.description").to_string()),
                            ),
                    )
                    .child(
                        Button::new("add-provider")
                            .with_variant(ButtonVariant::Primary)
                            .label(t!("LlmProviders.add_provider"))
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.add_provider(window, cx);
                            })),
                    ),
            )
            .child(if self.loading {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(t!("LlmProviders.loading").to_string())
                    .into_any_element()
            } else if self.providers.is_empty() {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        v_flex()
                            .gap_2()
                            .items_center()
                            .child(t!("LlmProviders.empty_title").to_string())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("LlmProviders.empty_description").to_string()),
                            ),
                    )
                    .into_any_element()
            } else {
                let mut cards = v_flex().gap_3();
                for provider in &self.providers {
                    cards = cards.child(self.render_provider_card(provider.clone(), cx));
                }
                cards.into_any_element()
            })
    }
}

impl LlmProvidersView {
    fn render_provider_card(
        &self,
        provider: ProviderConfig,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let provider_id = provider.id;
        let provider_id_edit = provider.id;
        let provider_clone = provider.clone();
        let default_value = t!("LlmProviders.default_value").to_string();
        let api_base_display = provider
            .api_base
            .as_deref()
            .unwrap_or(default_value.as_str())
            .to_string();
        let api_version_display = provider
            .api_version
            .as_deref()
            .unwrap_or(default_value.as_str())
            .to_string();

        div()
            .flex()
            .p_4()
            .gap_4()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                v_flex()
                    .flex_1()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(provider.name.clone()),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py(px(2.0))
                                    .rounded_md()
                                    .bg(cx.theme().secondary)
                                    .text_xs()
                                    .child(provider.provider_type.display_name()),
                            )
                            .when(provider.is_default, |this| {
                                this.child(
                                    div()
                                        .px_2()
                                        .py(px(2.0))
                                        .rounded_md()
                                        .bg(cx.theme().primary)
                                        .text_xs()
                                        .text_color(cx.theme().primary_foreground)
                                        .child(t!("LlmProviders.default_label").to_string()),
                                )
                            })
                            .when(!provider.enabled, |this| {
                                this.child(
                                    div()
                                        .px_2()
                                        .py(px(2.0))
                                        .rounded_md()
                                        .bg(cx.theme().muted)
                                        .text_xs()
                                        .child(t!("LlmProviders.disabled_label").to_string()),
                                )
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(
                                        t!(
                                            "LlmProviders.default_model",
                                            model = provider.model.as_str()
                                        )
                                            .to_string(),
                                    ),
                            )
                            .when(!provider.models.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(
                                            t!(
                                                "LlmProviders.optional_models_count",
                                                count = provider.models.len()
                                            )
                                            .to_string(),
                                        ),
                                )
                            })
                            .when(provider.api_base.is_some(), |this| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(
                                            t!(
                                                "LlmProviders.api_base",
                                                value = api_base_display.as_str()
                                            )
                                            .to_string(),
                                        ),
                                )
                            }),
                    )
                    .when(provider.api_version.is_some(), |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(
                                    t!(
                                        "LlmProviders.api_version",
                                        value = api_version_display.as_str()
                                    )
                                    .to_string(),
                                ),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .when(!provider.is_builtin(), |this| {
                        this.child(
                            Button::new(SharedString::from(format!("toggle-{}", provider_id)))
                                .with_variant(if provider.enabled {
                                    ButtonVariant::Secondary
                                } else {
                                    ButtonVariant::Primary
                                })
                                .label(if provider.enabled {
                                    t!("LlmProviders.action_disable")
                                } else {
                                    t!("LlmProviders.action_enable")
                                })
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.toggle_provider(provider_clone.clone(), cx);
                                })),
                        )
                    })
                    .child(
                        Button::new(SharedString::from(format!("edit-{}", provider_id_edit)))
                            .with_variant(ButtonVariant::Secondary)
                            .label(t!("LlmProviders.action_edit"))
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.edit_provider(provider_id_edit, window, cx);
                            })),
                    )
                    .when(!provider.is_builtin(), |this| {
                        this.child(
                            Button::new(SharedString::from(format!("delete-{}", provider_id)))
                                .with_variant(ButtonVariant::Secondary)
                                .label(t!("LlmProviders.action_delete"))
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.delete_provider(provider_id, cx);
                                })),
                        )
                    }),
            )
    }
}

impl Focusable for LlmProvidersView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<()> for LlmProvidersView {}
