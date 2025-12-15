use crate::{ActiveTheme, Collapsible, h_flex, v_flex};
use gpui::{
    AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, SharedString, Styled as _,
    Window, div, prelude::FluentBuilder as _,
};

/// A group of items in the [`super::Sidebar`].
#[derive(IntoElement)]
pub struct SidebarGroup<E: Collapsible + IntoElement + 'static> {
    base: Div,
    label: Option<SharedString>,
    header: Option<AnyElement>,
    collapsed: bool,
    children: Vec<E>,
    header_style: Option<Box<dyn Fn(Div) -> Div>>,
}

impl<E: Collapsible + IntoElement> SidebarGroup<E> {
    /// Create a new [`SidebarGroup`] with a text label.
    ///
    /// This label will be displayed in the header unless a custom header is defined via
    /// [`header`] or [`spaced_header`].
    ///
    /// # Example
    /// ```
    /// let group = SidebarGroup::new("Settings");
    /// ```
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            base: div().gap_2().flex_col(),
            label: Some(label.into()),
            header: None,
            collapsed: false,
            children: Vec::new(),
            header_style: None,
        }
    }

    /// Sets a fully custom header element.
    ///
    /// Accepts any type implementing [`IntoElement`]. The provided element will completely
    /// replace the label from [`new`].
    ///
    /// # Example
    /// ```
    /// let group = SidebarGroup::new("Ignored Label")
    ///     .header(div().child("Custom Header"));
    /// ```
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Sets a horizontally spaced header with left and right elements.
    ///
    /// Convenience method for aligning content at the left and right of the header.
    ///
    /// # Example
    /// ```
    /// let group = SidebarGroup::new("Ignored Label")
    ///     .spaced_header("Title", "+");
    /// ```
    ///
    /// **Warning:** This replaces the label from [`new`].
    pub fn spaced_header(self, left: impl IntoElement, right: impl IntoElement) -> Self {
        self.header(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .flex_1()
                .child(left)
                .child(right),
        )
    }

    /// Set a closure to override the default header styling.
    ///
    /// The closure receives the default [`Div`] used for the header and should return a
    /// customized [`Div`]. This allows changing padding, color, rounding, height, etc.
    ///
    /// # Example
    /// ```
    /// let group = SidebarGroup::new("Title")
    ///     .with_header_style(|this| this.p(px(4.0)));
    /// ```
    pub fn with_header_style<F>(mut self, f: F) -> Self
    where
        F: Fn(Div) -> Div + 'static,
    {
        self.header_style = Some(Box::new(f));
        self
    }

    /// Add a single child to the sidebar group.
    ///
    /// The child must implement [`Collapsible`] and [`IntoElement`].
    pub fn child(mut self, child: E) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children to the sidebar group.
    ///
    /// See also [`SidebarGroup::child`] for adding a single child.
    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.children.extend(children);
        self
    }
}

impl<E: Collapsible + IntoElement> Collapsible for SidebarGroup<E> {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl<E: Collapsible + IntoElement> RenderOnce for SidebarGroup<E> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut header_div = h_flex()
            .flex_shrink_0()
            .px_2()
            .rounded(cx.theme().radius)
            .text_xs()
            .text_color(cx.theme().sidebar_foreground.opacity(0.7))
            .h_8();

        if let Some(f) = self.header_style {
            header_div = f(header_div);
        }

        let header_element = self
            .header
            .or_else(|| self.label.map(|label| label.into_any_element()))
            .expect("SidebarGroup requires either label or header");

        v_flex()
            .relative()
            .when(!self.collapsed, |this| {
                this.child(header_div.child(header_element))
            })
            .child(
                self.base.children(
                    self.children
                        .into_iter()
                        .map(|child| child.collapsed(self.collapsed)),
                ),
            )
    }
}
