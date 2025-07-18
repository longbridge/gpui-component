use gpui::{
    div, px, Bounds, Context, Edges, Empty, EntityId, IntoElement, ParentElement as _, Pixels,
    Render, SharedString, Styled as _, Window,
};

use crate::ActiveTheme as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColFixed {
    Left,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ColGroup {
    pub(crate) width: Pixels,
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) sort: Option<ColSort>,
    pub(crate) fixed: Option<ColFixed>,
    pub(crate) paddings: Option<Edges<Pixels>>,
}

#[derive(Clone)]
pub(crate) struct DragCol {
    pub(crate) entity_id: EntityId,
    pub(crate) name: SharedString,
    pub(crate) width: Pixels,
    pub(crate) col_ix: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColSort {
    /// No sorting.
    Default,
    /// Sort in ascending order.
    Ascending,
    /// Sort in descending order.
    Descending,
}

#[derive(Clone, Copy, Default)]
pub(super) struct FixedCols {
    pub(super) left: usize,
}

impl Render for DragCol {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_4()
            .py_1()
            .bg(cx.theme().table_head)
            .text_color(cx.theme().muted_foreground)
            .opacity(0.9)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .w(self.width)
            .min_w(px(100.))
            .max_w(px(450.))
            .child(self.name.clone())
    }
}

#[derive(Clone)]
pub struct ResizeCol(pub (EntityId, usize));
impl Render for ResizeCol {
    fn render(&mut self, _window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
