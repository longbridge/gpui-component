use gpui::{
    AnyElement, App, DefiniteLength, IntoElement, ParentElement, RenderOnce, StyleRefinement,
    Styled, TextAlign, Window, div, prelude::FluentBuilder as _,
};

use crate::{ActiveTheme as _, Sizable, Size, StyledExt as _};

/// A basic table component for directly rendering tabular data.
///
/// Unlike [`DataTable`], this is a simple, stateless, composable table
/// without virtual scrolling or column management.
///
/// # Example
///
/// ```rust,ignore
/// Table::new()
///     .child(TableHeader::new().child(
///         TableRow::new()
///             .child(TableHead::new().child("Name"))
///             .child(TableHead::new().child("Email"))
///     ))
///     .child(TableBody::new()
///         .child(TableRow::new()
///             .child(TableCell::new().child("John"))
///             .child(TableCell::new().child("john@example.com")))
///     )
///     .child(TableCaption::new().child("A list of recent invoices."))
/// ```
#[derive(IntoElement)]
pub struct Table {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    size: Size,
}

impl Table {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new(), size: Size::default() }
    }
}

impl ParentElement for Table {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Table {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Table {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Table {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .text_sm()
            .overflow_hidden()
            .bg(cx.theme().table)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// The header section of a [`Table`], wrapping header rows.
#[derive(IntoElement)]
pub struct TableHeader {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TableHeader {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new() }
    }
}

impl ParentElement for TableHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TableHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .bg(cx.theme().table_head)
            .text_color(cx.theme().table_head_foreground)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// The body section of a [`Table`], wrapping data rows.
#[derive(IntoElement)]
pub struct TableBody {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TableBody {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new() }
    }
}

impl ParentElement for TableBody {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TableBody {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableBody {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().w_full().refine_style(&self.style).children(self.children)
    }
}

/// The footer section of a [`Table`], wrapping footer rows.
#[derive(IntoElement)]
pub struct TableFooter {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TableFooter {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new() }
    }
}

impl ParentElement for TableFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TableFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .bg(cx.theme().table_foot)
            .text_color(cx.theme().table_foot_foreground)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A row in a [`Table`].
#[derive(IntoElement)]
pub struct TableRow {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TableRow {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new() }
    }
}

impl ParentElement for TableRow {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TableRow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_row()
            .border_b_1()
            .border_color(cx.theme().table_row_border)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A header cell in a [`TableRow`].
#[derive(IntoElement)]
pub struct TableHead {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    width: Option<DefiniteLength>,
    align: TextAlign,
}

impl TableHead {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
            width: None,
            align: TextAlign::Left,
        }
    }

    /// Set the width of this column.
    pub fn w(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set text alignment to center.
    pub fn text_center(mut self) -> Self {
        self.align = TextAlign::Center;
        self
    }

    /// Set text alignment to right.
    pub fn text_right(mut self) -> Self {
        self.align = TextAlign::Right;
        self
    }
}

impl ParentElement for TableHead {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TableHead {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableHead {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let paddings = Size::default().table_cell_padding();

        div()
            .flex()
            .items_center()
            .flex_1()
            .truncate()
            .h(Size::default().table_row_height())
            .px(paddings.left)
            .py(paddings.top)
            .when(self.align == TextAlign::Center, |this| this.justify_center())
            .when(self.align == TextAlign::Right, |this| this.justify_end())
            .when_some(self.width, |this, width| this.flex_none().w(width))
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A data cell in a [`TableRow`].
#[derive(IntoElement)]
pub struct TableCell {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    width: Option<DefiniteLength>,
    align: TextAlign,
}

impl TableCell {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
            width: None,
            align: TextAlign::Left,
        }
    }

    /// Set the width of this cell.
    pub fn w(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set text alignment to center.
    pub fn text_center(mut self) -> Self {
        self.align = TextAlign::Center;
        self
    }

    /// Set text alignment to right.
    pub fn text_right(mut self) -> Self {
        self.align = TextAlign::Right;
        self
    }
}

impl ParentElement for TableCell {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TableCell {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableCell {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let paddings = Size::default().table_cell_padding();

        div()
            .flex()
            .items_center()
            .flex_1()
            .truncate()
            .h(Size::default().table_row_height())
            .px(paddings.left)
            .py(paddings.top)
            .when(self.align == TextAlign::Center, |this| this.justify_center())
            .when(self.align == TextAlign::Right, |this| this.justify_end())
            .when_some(self.width, |this, width| this.flex_none().w(width))
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A caption displayed below the [`Table`].
#[derive(IntoElement)]
pub struct TableCaption {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TableCaption {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new() }
    }
}

impl ParentElement for TableCaption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TableCaption {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableCaption {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let paddings = Size::default().table_cell_padding();

        div()
            .w_full()
            .px(paddings.left)
            .py(paddings.top)
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .children(self.children)
    }
}
