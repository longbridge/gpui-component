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

use crate::common::{SchemaFormEvent, SchemaOperationRequest};

pub struct MsSqlSchemaForm {
    focus_handle: FocusHandle,
    name_input: Entity<InputState>,
    comment_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl MsSqlSchemaForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Database.enter_schema_name").to_string())
        });

        let comment_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Database.enter_remark").to_string())
                .multi_line(true)
                .rows(3)
        });

        let name_sub = cx.observe(&name_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        let comment_sub = cx.observe(&comment_input, |this, _, cx| {
            this.trigger_form_changed(cx);
        });

        Self {
            focus_handle,
            name_input,
            comment_input,
            _subscriptions: vec![name_sub, comment_sub],
        }
    }

    fn build_request(&self, cx: &App) -> SchemaOperationRequest {
        let schema_name = self
            .name_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_string();
        let comment = self
            .comment_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_string();

        SchemaOperationRequest {
            schema_name,
            comment: if comment.is_empty() {
                None
            } else {
                Some(comment)
            },
        }
    }

    fn trigger_form_changed(&mut self, cx: &mut Context<Self>) {
        let request = self.build_request(cx);
        cx.emit(SchemaFormEvent::FormChanged(request));
    }
}

impl EventEmitter<SchemaFormEvent> for MsSqlSchemaForm {}

impl Focusable for MsSqlSchemaForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MsSqlSchemaForm {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_4().p_4().size_full().child(
            h_form()
                .with_size(Size::Small)
                .columns(1)
                .label_width(px(80.))
                .child(
                    field()
                        .label(t!("Database.schema_name").to_string())
                        .required(true)
                        .items_center()
                        .label_justify_end()
                        .child(Input::new(&self.name_input).w_full()),
                )
                .child(
                    field()
                        .label(t!("Database.remark").to_string())
                        .items_start()
                        .label_justify_end()
                        .child(Input::new(&self.comment_input).w_full()),
                ),
        )
    }
}
