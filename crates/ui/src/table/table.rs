use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, TextAlign,
    Window, div, prelude::FluentBuilder as _, relative,
};

use gpui::Hsla;

use crate::{ActiveTheme as _, Sizable, Size, StyledExt as _, element_ext::AnySizableElement};

/// Render children with `border_b` on all except the last one.
fn render_row_children(
    children: Vec<AnySizableElement>,
    size: Size,
    border_color: Hsla,
) -> Vec<AnyElement> {
    let len = children.len();
    children
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let el = c.into_any(size);
            if i < len - 1 {
                div().w_full().border_b_1().border_color(border_color).child(el).into_any_element()
            } else {
                el
            }
        })
        .collect()
}

/// A basic table component for directly rendering tabular data.
///
/// Unlike [`DataTable`], this is a simple, stateless, composable table
/// without virtual scrolling or column management.
///
/// Size set via [`Sizable`] is automatically propagated to all children.
///
/// # Example
///
/// ```rust,ignore
/// Table::new()
///     .small()
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
    children: Vec<AnySizableElement>,
    size: Size,
}

impl Table {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new(), size: Size::default() }
    }

    pub fn child(mut self, child: impl IntoElement + Sizable + 'static) -> Self {
        self.children.push(AnySizableElement::new(child));
        self
    }

    pub fn children<E: IntoElement + Sizable + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(AnySizableElement::new));
        self
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
        let children: Vec<AnyElement> =
            self.children.into_iter().map(|c| c.into_any(self.size)).collect();

        div()
            .w_full()
            .text_sm()
            .overflow_hidden()
            .bg(cx.theme().table)
            .refine_style(&self.style)
            .children(children)
    }
}

/// The header section of a [`Table`], wrapping header rows.
#[derive(IntoElement)]
pub struct TableHeader {
    style: StyleRefinement,
    children: Vec<AnySizableElement>,
    size: Size,
}

impl TableHeader {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new(), size: Size::default() }
    }

    pub fn child(mut self, child: impl IntoElement + Sizable + 'static) -> Self {
        self.children.push(AnySizableElement::new(child));
        self
    }

    pub fn children<E: IntoElement + Sizable + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(AnySizableElement::new));
        self
    }
}

impl Styled for TableHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TableHeader {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TableHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border_color = cx.theme().table_row_border;
        let children = render_row_children(self.children, self.size, border_color);

        div()
            .w_full()
            .bg(cx.theme().table_head)
            .text_color(cx.theme().table_head_foreground)
            .border_b_1()
            .border_color(cx.theme().table_row_border)
            .refine_style(&self.style)
            .children(children)
    }
}

/// The body section of a [`Table`], wrapping data rows.
#[derive(IntoElement)]
pub struct TableBody {
    style: StyleRefinement,
    children: Vec<AnySizableElement>,
    size: Size,
}

impl TableBody {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new(), size: Size::default() }
    }

    pub fn child(mut self, child: impl IntoElement + Sizable + 'static) -> Self {
        self.children.push(AnySizableElement::new(child));
        self
    }

    pub fn children<E: IntoElement + Sizable + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(AnySizableElement::new));
        self
    }
}

impl Styled for TableBody {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TableBody {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TableBody {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border_color = cx.theme().table_row_border;
        let children = render_row_children(self.children, self.size, border_color);

        div().w_full().refine_style(&self.style).children(children)
    }
}

/// The footer section of a [`Table`], wrapping footer rows.
#[derive(IntoElement)]
pub struct TableFooter {
    style: StyleRefinement,
    children: Vec<AnySizableElement>,
    size: Size,
}

impl TableFooter {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new(), size: Size::default() }
    }

    pub fn child(mut self, child: impl IntoElement + Sizable + 'static) -> Self {
        self.children.push(AnySizableElement::new(child));
        self
    }

    pub fn children<E: IntoElement + Sizable + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(AnySizableElement::new));
        self
    }
}

impl Styled for TableFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TableFooter {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TableFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border_color = cx.theme().table_row_border;
        let children = render_row_children(self.children, self.size, border_color);

        div()
            .w_full()
            .bg(cx.theme().table_foot)
            .text_color(cx.theme().table_foot_foreground)
            .border_t_1()
            .border_color(cx.theme().table_row_border)
            .refine_style(&self.style)
            .children(children)
    }
}

/// A row in a [`Table`].
#[derive(IntoElement)]
pub struct TableRow {
    style: StyleRefinement,
    children: Vec<AnySizableElement>,
    size: Size,
}

impl TableRow {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new(), size: Size::default() }
    }

    pub fn child(mut self, child: impl IntoElement + Sizable + 'static) -> Self {
        self.children.push(AnySizableElement::new(child));
        self
    }

    pub fn children<E: IntoElement + Sizable + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(AnySizableElement::new));
        self
    }
}

impl Styled for TableRow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TableRow {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TableRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let children: Vec<AnyElement> =
            self.children.into_iter().map(|c| c.into_any(self.size)).collect();

        div().w_full().flex().flex_row().refine_style(&self.style).children(children)
    }
}

/// A header cell in a [`TableRow`].
#[derive(IntoElement)]
pub struct TableHead {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    col_span: usize,
    align: TextAlign,
    size: Size,
}

impl TableHead {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
            col_span: 1,
            align: TextAlign::Left,
            size: Size::default(),
        }
    }

    /// Set the column span of this header cell.
    pub fn col_span(mut self, span: usize) -> Self {
        self.col_span = span.max(1);
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

impl Sizable for TableHead {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for TableHead {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableHead {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let paddings = self.size.table_cell_padding();

        div()
            .flex()
            .items_center()
            .flex_shrink()
            .flex_basis(relative(self.col_span as f32))
            .px(paddings.left)
            .py(paddings.top)
            .when(self.align == TextAlign::Center, |this| this.justify_center())
            .when(self.align == TextAlign::Right, |this| this.justify_end())
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A data cell in a [`TableRow`].
#[derive(IntoElement)]
pub struct TableCell {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    col_span: usize,
    align: TextAlign,
    size: Size,
}

impl TableCell {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
            col_span: 1,
            align: TextAlign::Left,
            size: Size::default(),
        }
    }

    /// Set the column span of this cell.
    pub fn col_span(mut self, span: usize) -> Self {
        self.col_span = span.max(1);
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

impl Sizable for TableCell {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for TableCell {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableCell {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let paddings = self.size.table_cell_padding();

        div()
            .flex()
            .items_center()
            .flex_shrink()
            .flex_basis(relative(self.col_span as f32))
            .px(paddings.left)
            .py(paddings.top)
            .when(self.align == TextAlign::Center, |this| this.justify_center())
            .when(self.align == TextAlign::Right, |this| this.justify_end())
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A caption displayed below the [`Table`].
#[derive(IntoElement)]
pub struct TableCaption {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    size: Size,
}

impl TableCaption {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new(), size: Size::default() }
    }
}

impl ParentElement for TableCaption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for TableCaption {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for TableCaption {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TableCaption {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let paddings = self.size.table_cell_padding();

        div()
            .w_full()
            .px(paddings.left)
            .py(paddings.top)
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .text_center()
            .refine_style(&self.style)
            .children(self.children)
    }
}
