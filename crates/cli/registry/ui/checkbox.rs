//! Application-owned checkbox presentation. Edit it to match your design system.

use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    black, div, prelude::FluentBuilder as _, white,
};
use gpui_component_base::{self as base, CheckboxState};

type ChangeHandler = Rc<dyn Fn(CheckboxState, &mut Window, &mut App)>;

pub struct Checkbox {
    id: ElementId,
    label: SharedString,
    state: CheckboxState,
    disabled: bool,
    on_change: Option<ChangeHandler>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            state: CheckboxState::Unchecked,
            disabled: false,
            on_change: None,
        }
    }

    pub fn state(mut self, state: CheckboxState) -> Self {
        self.state = state;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.state = if checked {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        self
    }

    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        if indeterminate {
            self.state = CheckboxState::Indeterminate;
        }
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(CheckboxState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let state = self.state;
        base::Checkbox::new(self.id)
            .state(state)
            .disabled(self.disabled)
            .accessibility_label(self.label.clone())
            .when_some(self.on_change, |this, on_change| {
                this.on_change(move |state, window, cx| on_change(state, window, cx))
            })
            .flex()
            .items_center()
            .gap_2()
            .when(self.disabled, |this| this.opacity(0.5))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_4()
                    .border_1()
                    .border_color(black())
                    .rounded_sm()
                    .when(state != CheckboxState::Unchecked, |this| {
                        this.bg(black()).text_color(white())
                    })
                    .child(match state {
                        CheckboxState::Unchecked => "",
                        CheckboxState::Checked => "✓",
                        CheckboxState::Indeterminate => "−",
                    }),
            )
            .child(self.label)
    }
}
