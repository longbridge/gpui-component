//! Application-owned switch presentation. Edit it to match your design system.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Window, black, div, prelude::FluentBuilder as _, white,
};
use gpui_component_base as base;

type ToggleHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

pub struct Switch {
    id: ElementId,
    label: SharedString,
    checked: bool,
    disabled: bool,
    on_toggle: Option<ToggleHandler>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked: false,
            disabled: false,
            on_toggle: None,
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

    pub fn on_toggle(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        base::Switch::new(self.id)
            .checked(checked)
            .disabled(self.disabled)
            .styles(|styles| styles.disabled(|style| style.opacity(0.5)))
            .accessibility_label(self.label.clone())
            .when_some(self.on_toggle, |this, on_toggle| {
                this.on_toggle(move |checked, event, window, cx| {
                    on_toggle(checked, event, window, cx)
                })
            })
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .w_8()
                    .h_4()
                    .p_px()
                    .rounded_full()
                    .border_1()
                    .border_color(black())
                    .when(checked, |this| this.justify_end().bg(black()))
                    .child(div().size_3().rounded_full().bg(if checked {
                        white()
                    } else {
                        black()
                    })),
            )
            .child(self.label)
    }
}
