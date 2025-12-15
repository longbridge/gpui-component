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
}

impl<E: Collapsible + IntoElement> SidebarGroup<E> {
    /// Create a new [`SidebarGroup`] with the given label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            base: div().gap_2().flex_col(),
            label: Some(label.into()),
            header: None,
            collapsed: false,
            children: Vec::new(),
        }
    }

    /// Creates a new [`SidebarGroup`] with a fully custom header element.
    pub fn new_with_header(header: impl IntoElement) -> Self {
        Self {
            base: div().gap_2().flex_col(),
            label: None,
            header: Some(header.into_any_element()),
            collapsed: false,
            children: Vec::new(),
        }
    }

    /// Creates a new [`SidebarGroup`] with a horizontal header layout where the left and right elements are spaced apart.
    pub fn new_with_spaced_header(left: impl IntoElement, right: impl IntoElement) -> Self {
        Self::new_with_header(
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

    /// Add a child to the sidebar group, the child should implement [`Collapsible`] + [`IntoElement`].
    pub fn child(mut self, child: E) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children to the sidebar group.
    ///
    /// See also [`SidebarGroup::child`].
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
        v_flex()
            .relative()
            .when(!self.collapsed, |this| {
                this.child(
                    h_flex()
                        .flex_shrink_0()
                        .px_2()
                        .rounded(cx.theme().radius)
                        .text_xs()
                        .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                        .h_8()
                        .child(
                            self.header
                                .or_else(|| self.label.map(|label| label.into_any_element()))
                                .expect("SidebarGroup requires either label or header"),
                        ),
                )
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
