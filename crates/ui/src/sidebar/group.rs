use crate::{v_flex, ActiveTheme, Collapsible};
use gpui::{Window, AppContext, 
    div, prelude::FluentBuilder as _, Div, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled as _, 
};

/// A sidebar group
#[derive(IntoElement)]
pub struct SidebarGroup<E: Collapsible + IntoElement + 'static> {
    base: Div,
    label: SharedString,
    is_collapsed: bool,
    children: Vec<E>,
}

impl<E: Collapsible + IntoElement> SidebarGroup<E> {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            base: div().gap_2().flex_col(),
            label: label.into(),
            is_collapsed: false,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: E) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.children.extend(children);
        self
    }
}
impl<E: Collapsible + IntoElement> Collapsible for SidebarGroup<E> {
    fn is_collapsed(&self) -> bool {
        self.is_collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.is_collapsed = collapsed;
        self
    }
}
impl<E: Collapsible + IntoElement> RenderOnce for SidebarGroup<E> {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        v_flex()
            .relative()
            .p_2()
            .when(!self.is_collapsed, |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .px_2()
                        .rounded_md()
                        .text_xs()
                        .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                        .h_8()
                        .child(self.label),
                )
            })
            .child(
                self.base.children(
                    self.children
                        .into_iter()
                        .map(|child| child.collapsed(self.is_collapsed)),
                ),
            )
    }
}
