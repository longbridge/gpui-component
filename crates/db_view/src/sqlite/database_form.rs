use std::collections::HashMap;

use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Subscription, Window, prelude::*, px,
};
use gpui_component::form::h_form;
use gpui_component::{
    Sizable, Size,
    form::field,
    input::{Input, InputState},
    v_flex,
};
use rust_i18n::t;

use crate::DatabaseFormEvent;
use db::plugin::DatabaseOperationRequest;

pub struct SqliteDatabaseForm {
    focus_handle: FocusHandle,
    name_input: Entity<InputState>,
    path_input: Entity<InputState>,
    is_edit_mode: bool,
    _subscriptions: Vec<Subscription>,
}

impl SqliteDatabaseForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Database.enter_database_name").to_string())
        });

        let path_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Database.file_path_placeholder").to_string())
        });

        let name_sub = cx.observe(&name_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        let path_sub = cx.observe(&path_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        Self {
            focus_handle,
            name_input,
            path_input,
            is_edit_mode: false,
            _subscriptions: vec![name_sub, path_sub],
        }
    }

    pub fn new_for_edit(database_name: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut form = Self::new(window, cx);
        form.is_edit_mode = true;
        form.name_input.update(cx, |input, cx| {
            input.set_value(database_name.to_string(), window, cx);
        });
        form
    }

    fn build_request(&self, cx: &App) -> DatabaseOperationRequest {
        let mut field_values = HashMap::new();

        let db_name = self.name_input.read(cx).text().to_string();
        let path = self.path_input.read(cx).text().to_string();

        field_values.insert("name".to_string(), db_name.clone());
        field_values.insert("path".to_string(), path);

        DatabaseOperationRequest {
            database_name: db_name,
            field_values,
        }
    }

    fn trigger_form_changed(&mut self, cx: &mut Context<Self>) {
        let request = self.build_request(cx);
        cx.emit(DatabaseFormEvent::FormChanged(request));
    }
}

impl EventEmitter<DatabaseFormEvent> for SqliteDatabaseForm {}

impl Focusable for SqliteDatabaseForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SqliteDatabaseForm {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_4().p_4().size_full().child(
            h_form()
                .with_size(Size::Small)
                .columns(1)
                .label_width(px(100.))
                .child(
                    field()
                        .label(t!("Database.database_name").to_string())
                        .required(true)
                        .items_center()
                        .label_justify_end()
                        .child(
                            Input::new(&self.name_input)
                                .w_full()
                                .disabled(self.is_edit_mode),
                        ),
                )
                .child(
                    field()
                        .label(t!("Database.file_path").to_string())
                        .items_center()
                        .label_justify_end()
                        .child(Input::new(&self.path_input).w_full()),
                ),
        )
    }
}
