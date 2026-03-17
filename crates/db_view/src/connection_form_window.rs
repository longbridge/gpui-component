use gpui::prelude::FluentBuilder;
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable, TitleBar,
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};
use one_core::cloud_sync::TeamOption;
use one_core::connection_notifier::{ConnectionDataEvent, emit_connection_event};
use one_core::storage::{DatabaseType, StoredConnection, Workspace};
use rust_i18n::t;

use crate::common::db_connection_form::{DbConnectionForm, DbConnectionFormEvent};
use crate::database_view_plugin::DatabaseViewPluginRegistry;

/// 连接表单窗口的配置
pub struct ConnectionFormWindowConfig {
    pub db_type: DatabaseType,
    pub editing_connection: Option<StoredConnection>,
    pub workspaces: Vec<Workspace>,
    pub teams: Vec<TeamOption>,
}

/// 连接表单窗口
///
/// 包含 TitleBar、DbConnectionForm 和操作按钮
pub struct ConnectionFormWindow {
    focus_handle: FocusHandle,
    form: Entity<DbConnectionForm>,
    title: SharedString,
}

impl ConnectionFormWindow {
    pub fn new(
        config: ConnectionFormWindowConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let is_editing = config.editing_connection.is_some();
        let db_type = config.db_type;

        let title: SharedString = if is_editing {
            t!("Connection.edit", db_type = db_type.as_str()).to_string()
        } else {
            t!("Connection.new", db_type = db_type.as_str()).to_string()
        }
        .into();

        let plugin_registry = cx.global::<DatabaseViewPluginRegistry>();
        let plugin = plugin_registry
            .get(&db_type)
            .expect("Plugin should exist for db_type");

        let form = plugin.create_connection_form(window, cx);

        form.update(cx, |f, cx| {
            f.set_workspaces(config.workspaces.clone(), window, cx);
            f.set_teams(config.teams.clone(), window, cx);
        });

        if let Some(ref conn) = config.editing_connection {
            form.update(cx, |f, cx| {
                f.load_connection(conn, window, cx);
            });
        }

        let is_edit = is_editing;
        cx.subscribe_in(
            &form,
            window,
            move |_this, _form, event: &DbConnectionFormEvent, window, cx| match event {
                DbConnectionFormEvent::Saved(conn) => {
                    if is_edit {
                        emit_connection_event(
                            ConnectionDataEvent::ConnectionUpdated {
                                connection: conn.clone(),
                            },
                            cx,
                        );
                    } else {
                        emit_connection_event(
                            ConnectionDataEvent::ConnectionCreated {
                                connection: conn.clone(),
                            },
                            cx,
                        );
                    }
                    window.remove_window();
                }
                DbConnectionFormEvent::SaveError(_) => {}
            },
        )
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            form,
            title,
        }
    }

    fn on_test(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.form.update(cx, |form, cx| {
            form.trigger_test_connection(cx);
        });
    }

    fn on_save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.form.update(cx, |form, cx| {
            form.save_connection(cx);
        });
    }

    fn on_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.form.update(cx, |form, cx| {
            form.trigger_cancel(cx);
        });
        window.remove_window();
    }
}

impl Focusable for ConnectionFormWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ConnectionFormWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_testing = self.form.read(cx).is_testing(cx);
        let test_result_msg = self.form.read(cx).test_result_msg(cx);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new().child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_1()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(self.title.clone()),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .p_4()
                    .overflow_y_scrollbar()
                    .child(self.form.clone()),
            )
            .when_some(test_result_msg, |this, msg| {
                let is_success = msg.starts_with("✓");
                this.child(
                    div()
                        .mx_4()
                        .mb_2()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(if is_success {
                            gpui::rgb(0xdcfce7)
                        } else {
                            gpui::rgb(0xfee2e2)
                        })
                        .text_color(if is_success {
                            gpui::rgb(0x166534)
                        } else {
                            gpui::rgb(0x991b1b)
                        })
                        .child(msg),
                )
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .p_4()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("cancel")
                            .small()
                            .label(t!("Common.cancel").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_cancel(window, cx);
                            })),
                    )
                    .child(
                        Button::new("test")
                            .small()
                            .outline()
                            .label(if is_testing {
                                t!("Connection.testing").to_string()
                            } else {
                                t!("Connection.test").to_string()
                            })
                            .disabled(is_testing)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_test(window, cx);
                            })),
                    )
                    .child(
                        Button::new("ok")
                            .small()
                            .primary()
                            .label(t!("Common.ok").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_save(window, cx);
                            })),
                    ),
            )
    }
}
