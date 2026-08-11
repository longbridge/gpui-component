//! Application-owned button presentation.
//!
//! Edit this file freely to match your design system.

use gpui::{ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled as _, Window};
use gpui_component_base as base;

pub struct Button {
    id: ElementId,
    label: SharedString,
    disabled: bool,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut gpui::App) -> impl IntoElement {
        base::Button::new(self.id)
            .disabled(self.disabled)
            .px_3()
            .py_2()
            .rounded_md()
            .child(self.label)
    }
}
