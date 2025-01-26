use gpui::{Window, ModelContext, Axis, };

mod panel;
mod resize_handle;
pub use panel::*;
pub(crate) use resize_handle::*;

pub fn h_resizable(cx: &mut ViewContext<ResizablePanelGroup>) -> ResizablePanelGroup {
    ResizablePanelGroup::new(cx).axis(Axis::Horizontal)
}

pub fn v_resizable(cx: &mut ViewContext<ResizablePanelGroup>) -> ResizablePanelGroup {
    ResizablePanelGroup::new(cx).axis(Axis::Vertical)
}

pub fn resizable_panel() -> ResizablePanel {
    ResizablePanel::new()
}
