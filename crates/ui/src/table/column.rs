use gpui::{
    div, px, Bounds, Context, Edges, Empty, EntityId, IntoElement, ParentElement as _, Pixels,
    Render, SharedString, Styled as _, TextAlign, Window,
};

use crate::ActiveTheme as _;

/// Represents a column in a table, used for initializing table columns.
#[derive(Debug, Clone)]
pub struct TableCol {
    pub key: SharedString,
    pub name: SharedString,
    pub col_span: usize,
    pub align: TextAlign,
    pub sort: Option<ColSort>,
    pub paddings: Option<Edges<Pixels>>,
    pub width: Pixels,
    pub fixed: Option<ColFixed>,
    pub resizable: bool,
    pub movable: bool,
    pub selectable: bool,
}

impl Default for TableCol {
    fn default() -> Self {
        Self {
            key: SharedString::new(""),
            name: SharedString::new(""),
            col_span: 1,
            align: TextAlign::Left,
            sort: None,
            paddings: None,
            width: px(100.),
            fixed: None,
            resizable: true,
            movable: true,
            selectable: true,
        }
    }
}

impl TableCol {
    /// Create a new column with the given key and name.
    pub fn new(key: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the col span for the Table Head, default is 1.
    pub fn col_span(mut self, col_span: usize) -> Self {
        self.col_span = col_span;
        self
    }

    /// Set the column to be sortable with custom sort function, default is None (not sortable).
    pub fn sort(mut self, sort: ColSort) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Set the alignment of the column text, default is left.
    ///
    /// Only `text_left`, `text_right` is supported.
    pub fn text_right(mut self) -> Self {
        self.align = TextAlign::Right;
        self
    }

    /// Set the padding of the column, default is None.
    pub fn p(mut self, paddings: impl Into<Edges<Pixels>>) -> Self {
        self.paddings = Some(paddings.into());
        self
    }

    pub fn p_0(mut self) -> Self {
        self.paddings = Some(Edges::all(px(0.)));
        self
    }

    /// Set the width of the column, default is 100px.
    pub fn w(mut self, width: impl Into<Pixels>) -> Self {
        self.width = width.into();
        self
    }

    /// Set whether the column is fixed, default is false.
    pub fn fixed(mut self, fixed: impl Into<ColFixed>) -> Self {
        self.fixed = Some(fixed.into());
        self
    }

    /// Set whether the column is resizable, default is true.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set whether the column is movable, default is true.
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// Set whether the column is selectable, default is true.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Set whether the column is sortable, default is true.
    pub fn sortable(mut self) -> Self {
        self.sort = Some(ColSort::Default);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColFixed {
    Left,
}

/// Used to sort the column runtime info in Table internal.
#[derive(Debug, Clone)]
pub(crate) struct ColGroup {
    pub(crate) column: TableCol,
    /// This is the runtime width of the column, we may update it when the column is resized.
    pub(crate) width: Pixels,
    /// The bounds of the column in the table after it renders.
    pub(crate) bounds: Bounds<Pixels>,
}

#[derive(Clone)]
pub(crate) struct DragCol {
    pub(crate) entity_id: EntityId,
    pub(crate) name: SharedString,
    pub(crate) width: Pixels,
    pub(crate) col_ix: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ColSort {
    /// No sorting.
    #[default]
    Default,
    /// Sort in ascending order.
    Ascending,
    /// Sort in descending order.
    Descending,
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
