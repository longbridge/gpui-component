use gpui::{
    Bounds, Context, Edges, Empty, EntityId, IntoElement, ParentElement as _, Pixels, Render,
    SharedString, Styled as _, TextAlign, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::ActiveTheme as _;

#[derive(Debug, Clone)]
pub struct Column {
    pub key: SharedString,
    pub name: SharedString,
    pub align: TextAlign,
    pub sort: Option<ColumnSort>,
    pub paddings: Option<Edges<Pixels>>,
    pub width: Pixels,
    pub fixed: Option<ColumnFixed>,
    pub resizable: bool,
    pub movable: bool,
    pub selectable: bool,
}

impl Default for Column {
    fn default() -> Self {
        Self {
            key: SharedString::new(""),
            name: SharedString::new(""),
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

impl Column {
    pub fn new(key: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn sort(mut self, sort: ColumnSort) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sort = Some(ColumnSort::Default);
        self
    }

    pub fn ascending(mut self) -> Self {
        self.sort = Some(ColumnSort::Ascending);
        self
    }

    pub fn descending(mut self) -> Self {
        self.sort = Some(ColumnSort::Descending);
        self
    }

    pub fn text_right(mut self) -> Self {
        self.align = TextAlign::Right;
        self
    }

    pub fn paddings(mut self, paddings: impl Into<Edges<Pixels>>) -> Self {
        self.paddings = Some(paddings.into());
        self
    }

    pub fn p_0(mut self) -> Self {
        self.paddings = Some(Edges::all(px(0.)));
        self
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = width.into();
        self
    }

    pub fn fixed(mut self, fixed: impl Into<ColumnFixed>) -> Self {
        self.fixed = Some(fixed.into());
        self
    }

    pub fn fixed_left(mut self) -> Self {
        self.fixed = Some(ColumnFixed::Left);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }
}

impl FluentBuilder for Column {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnFixed {
    Left,
}

#[derive(Debug, Clone)]
pub(crate) struct ColGroup {
    pub(crate) column: Column,
    pub(crate) width: Pixels,
    pub(crate) bounds: Bounds<Pixels>,
}

impl ColGroup {
    pub(crate) fn is_resizable(&self) -> bool {
        self.column.resizable
    }
}

#[derive(Clone)]
pub(crate) struct DragColumn {
    pub(crate) entity_id: EntityId,
    pub(crate) name: SharedString,
    pub(crate) width: Pixels,
    pub(crate) col_ix: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ColumnSort {
    #[default]
    Default,
    Ascending,
    Descending,
}

impl Render for DragColumn {
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
pub(crate) struct ResizeColumn(pub (EntityId, usize));
impl Render for ResizeColumn {
    fn render(&mut self, _window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// 拖选单元格的标记结构
#[derive(Clone)]
pub(crate) struct DragSelectCell {
    pub entity_id: EntityId,
}

impl Render for DragSelectCell {
    fn render(&mut self, _window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
