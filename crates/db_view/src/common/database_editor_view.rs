use super::DatabaseFormEvent;
use db::{GlobalDbState, plugin::DatabaseOperationRequest};
use gpui::{
    AnyView, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Subscription, Window, div,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    highlighter::Language,
    input::{Input, InputState},
    v_flex,
};
use one_core::storage::DatabaseType;
use rust_i18n::t;

pub struct DatabaseEditorView {
    focus_handle: FocusHandle,
    form: AnyView,
    sql_preview: Entity<InputState>,
    current_tab: EditorTab,
    is_edit_mode: bool,
    error_message: Entity<Option<String>>,
    database_name: Entity<String>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone, Copy, PartialEq)]
enum EditorTab {
    Form,
    SqlPreview,
}

impl DatabaseEditorView {
    pub fn new<F>(
        form: Entity<F>,
        database_type: DatabaseType,
        is_edit_mode: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self
    where
        F: Render + EventEmitter<DatabaseFormEvent> + 'static,
    {
        let sql_preview = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::from_str("sql"))
                .line_number(false)
                .multi_line(true)
        });
        let focus_handle = cx.focus_handle();
        let error_message = cx.new(|_| None);
        let database_name = cx.new(|_| String::new());

        let is_edit = is_edit_mode;

        let database_name_clone = database_name.clone();
        let form_subscription = cx.subscribe_in(
            &form,
            window,
            move |this, _form, event, window, cx| match event {
                DatabaseFormEvent::FormChanged(request) => {
                    database_name_clone.update(cx, |name, _| {
                        *name = request.database_name.clone();
                    });
                    this.update_sql_preview(request, database_type, is_edit, window, cx);
                }
            },
        );

        Self {
            focus_handle,
            form: form.into(),
            sql_preview,
            current_tab: EditorTab::Form,
            is_edit_mode,
            error_message,
            database_name,
            _subscriptions: vec![form_subscription],
        }
    }

    fn update_sql_preview(
        &mut self,
        request: &DatabaseOperationRequest,
        database_type: DatabaseType,
        is_edit_mode: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let global_db_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_db_state.get_plugin(&database_type) {
            let sql = if is_edit_mode {
                plugin.build_modify_database_sql(request)
            } else {
                plugin.build_create_database_sql(request)
            };
            self.sql_preview.update(cx, |state, cx| {
                state.set_value(sql, window, cx);
            });
        }
    }

    pub fn get_sql(&self, cx: &App) -> String {
        self.sql_preview.read(cx).text().to_string()
    }

    pub fn get_database_name(&self, cx: &App) -> String {
        self.database_name.read(cx).clone()
    }

    pub fn set_save_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error_message.update(cx, |msg, cx| {
            *msg = Some(error);
            cx.notify();
        });
    }

    pub fn clear_error(&mut self, cx: &mut Context<Self>) {
        self.error_message.update(cx, |msg, cx| {
            *msg = None;
            cx.notify();
        });
    }

    pub fn is_edit_mode(&self) -> bool {
        self.is_edit_mode
    }
}

impl Focusable for DatabaseEditorView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DatabaseEditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form_button = if self.current_tab == EditorTab::Form {
            Button::new("tab_form")
                .label(t!("EditorView.form_tab").to_string())
                .primary()
        } else {
            Button::new("tab_form")
                .label(t!("EditorView.form_tab").to_string())
                .ghost()
        };

        let sql_button = if self.current_tab == EditorTab::SqlPreview {
            Button::new("tab_sql")
                .label(t!("EditorView.sql_preview_tab").to_string())
                .primary()
        } else {
            Button::new("tab_sql")
                .label(t!("EditorView.sql_preview_tab").to_string())
                .ghost()
        };

        let main_content = if self.current_tab == EditorTab::Form {
            div().flex_1().w_full().child(self.form.clone())
        } else {
            div()
                .flex_1()
                .w_full()
                .min_h_48()
                .child(Input::new(&self.sql_preview).size_full().disabled(true))
        };

        let error_msg = self.error_message.read(cx).clone();

        let mut container = v_flex()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(gpui::rgb(0xe0e0e0))
                    .child(form_button.on_click(cx.listener(|this, _, _, cx| {
                        this.current_tab = EditorTab::Form;
                        cx.notify();
                    })))
                    .child(sql_button.on_click(cx.listener(|this, _, _, cx| {
                        this.current_tab = EditorTab::SqlPreview;
                        cx.notify();
                    }))),
            )
            .child(main_content);

        if let Some(msg) = error_msg {
            container = container.child(
                div()
                    .mx_4()
                    .mb_4()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(gpui::rgb(0xfee2e2))
                    .text_color(gpui::rgb(0x991b1b))
                    .child(format!("✗ {}", msg)),
            );
        }

        container
    }
}
