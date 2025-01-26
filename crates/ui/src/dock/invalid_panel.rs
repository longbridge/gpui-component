use gpui::{Window, 
    AppContext, EventEmitter, FocusHandle, FocusableView, ParentElement as _, Render, SharedString,
    Styled as _, 
};

use crate::ActiveTheme as _;

use super::{Panel, PanelEvent, PanelState};

pub(crate) struct InvalidPanel {
    name: SharedString,
    focus_handle: FocusHandle,
    old_state: PanelState,
}

impl InvalidPanel {
    pub(crate) fn new(name: &str, state: PanelState, cx: &mut WindowContext) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            name: SharedString::from(name.to_owned()),
            old_state: state,
        }
    }
}
impl Panel for InvalidPanel {
    fn panel_name(&self) -> &'static str {
        "InvalidPanel"
    }

    fn dump(&self, _cx: &AppContext) -> super::PanelState {
        self.old_state.clone()
    }
}
impl EventEmitter<PanelEvent> for InvalidPanel {}
impl FocusableView for InvalidPanel {
    fn focus_handle(&self, _: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for InvalidPanel {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        gpui::div()
            .size_full()
            .my_6()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child(format!(
                "The `{}` panel type is not registered in PanelRegistry.",
                self.name.clone()
            ))
    }
}
