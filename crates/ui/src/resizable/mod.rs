use gpui::Axis;

mod panel;
mod resize_handle;
pub use panel::*;
pub(crate) use resize_handle::*;

pub fn h_resizable() -> ResizablePanelGroup {
    ResizablePanelGroup::new().axis(Axis::Horizontal)
}

pub fn v_resizable() -> ResizablePanelGroup {
    ResizablePanelGroup::new().axis(Axis::Vertical)
}

pub fn resizable_panel() -> ResizablePanel {
    ResizablePanel::new()
}
