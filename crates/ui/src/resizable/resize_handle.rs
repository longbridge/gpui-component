use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext, Axis, Div, ElementId,
    InteractiveElement, IntoElement, ParentElement as _, Pixels, RenderOnce, Stateful,
    StatefulInteractiveElement, Styled as _, Window,
};

use crate::{ActiveTheme as _, AxisExt as _};

pub(crate) const HANDLE_PADDING: Pixels = px(4.);
pub(crate) const HANDLE_SIZE: Pixels = px(1.);

#[derive(IntoElement)]
pub(crate) struct ResizeHandle {
    base: Stateful<Div>,
    axis: Axis,
}

impl ResizeHandle {
    fn new(id: impl Into<ElementId>, axis: Axis) -> Self {
        Self {
            base: div().id(id.into()),
            axis,
        }
    }
}

/// Create a resize handle for a resizable panel.
pub(crate) fn resize_handle(id: impl Into<ElementId>, axis: Axis) -> ResizeHandle {
    ResizeHandle::new(id, axis)
}

impl InteractiveElement for ResizeHandle {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for ResizeHandle {}

impl RenderOnce for ResizeHandle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let neg_offset = -HANDLE_PADDING;

        self.base
            .occlude()
            .absolute()
            .flex_shrink_0()
            .when(self.axis.is_horizontal(), |this| {
                this.cursor_col_resize()
                    .top_0()
                    .left(neg_offset)
                    .h_full()
                    .w(HANDLE_SIZE)
                    .px(HANDLE_PADDING)
            })
            .when(self.axis.is_vertical(), |this| {
                this.cursor_row_resize()
                    .top(neg_offset)
                    .left_0()
                    .w_full()
                    .h(HANDLE_SIZE)
                    .py(HANDLE_PADDING)
            })
            .child(
                div()
                    .bg(cx.theme().border)
                    .when(self.axis.is_horizontal(), |this| {
                        this.h_full().w(HANDLE_SIZE)
                    })
                    .when(self.axis.is_vertical(), |this| this.w_full().h(HANDLE_SIZE)),
            )
    }
}
