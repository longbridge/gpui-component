use super::{SchemaFormEvent, SchemaOperationRequest};
use db::GlobalDbState;
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

pub struct SchemaEditorView {
    focus_handle: FocusHandle,
    form: AnyView,
    sql_preview: Entity<InputState>,
    current_tab: EditorTab,
    error_message: Entity<Option<String>>,
    database_type: DatabaseType,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone, Copy, PartialEq)]
enum EditorTab {
    Form,
    SqlPreview,
}

impl SchemaEditorView {
    pub fn new<F>(
        form: Entity<F>,
        database_type: DatabaseType,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self
    where
        F: Render + EventEmitter<SchemaFormEvent> + 'static,
    {
        let sql_preview = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::from_str("sql"))
                .line_number(false)
                .multi_line(true)
        });
        let focus_handle = cx.focus_handle();
        let error_message = cx.new(|_| None);

        let db_type = database_type;

        let form_subscription = cx.subscribe_in(
            &form,
            window,
            move |this, _form, event, window, cx| match event {
                SchemaFormEvent::FormChanged(request) => {
                    this.update_sql_preview(request, db_type, window, cx);
                }
            },
        );

        Self {
            focus_handle,
            form: form.into(),
            sql_preview,
            current_tab: EditorTab::Form,
            error_message,
            database_type,
            _subscriptions: vec![form_subscription],
        }
    }

    fn update_sql_preview(
        &mut self,
        request: &SchemaOperationRequest,
        database_type: DatabaseType,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let global_db_state = cx.global::<GlobalDbState>();
        if let Ok(plugin) = global_db_state.get_plugin(&database_type) {
            let mut sql = plugin.build_create_schema_sql(&request.schema_name);
            if let Some(comment) = &request.comment {
                if !comment.is_empty() {
                    if let Some(comment_sql) =
                        plugin.build_comment_schema_sql(&request.schema_name, comment)
                    {
                        sql = format!("{}\n{}", sql, comment_sql);
                    }
                }
            }
            self.sql_preview.update(cx, |state, cx| {
                state.set_value(sql, window, cx);
            });
        }
    }

    pub fn get_sql(&self, cx: &App) -> String {
        self.sql_preview.read(cx).text().to_string()
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

    pub fn database_type(&self) -> DatabaseType {
        self.database_type
    }
}

impl Focusable for SchemaEditorView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SchemaEditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form_button = if self.current_tab == EditorTab::Form {
            Button::new("tab_form").label("表单").primary()
        } else {
            Button::new("tab_form").label("表单").ghost()
        };

        let sql_button = if self.current_tab == EditorTab::SqlPreview {
            Button::new("tab_sql").label("SQL 预览").primary()
        } else {
            Button::new("tab_sql").label("SQL 预览").ghost()
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
