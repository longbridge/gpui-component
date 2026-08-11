//! Application-owned radio presentation. Edit it to match your design system.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Window, black, div, prelude::FluentBuilder as _,
};
use gpui_component_base as base;

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

pub struct Radio {
    id: ElementId,
    label: SharedString,
    checked: bool,
    disabled: bool,
    on_change: Option<ChangeHandler>,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked: false,
            disabled: false,
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Radio {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        base::Radio::new(self.id)
            .checked(checked)
            .disabled(self.disabled)
            .styles(|styles| styles.disabled(|style| style.opacity(0.5)))
            .accessibility_label(self.label.clone())
            .when_some(self.on_change, |this, on_change| {
                this.on_change(move |checked, event, window, cx| {
                    on_change(checked, event, window, cx)
                })
            })
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_4()
                    .rounded_full()
                    .border_1()
                    .border_color(black())
                    .when(checked, |this| {
                        this.child(div().size_2().rounded_full().bg(black()))
                    }),
            )
            .child(self.label)
    }
}
