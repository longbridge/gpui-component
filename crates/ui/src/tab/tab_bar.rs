use crate::{h_flex, ActiveTheme};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, AnyElement, App, Div, ElementId, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui::{px, InteractiveElement};
use smallvec::SmallVec;

#[derive(IntoElement)]
pub struct TabBar {
    base: Div,
    id: ElementId,
    scroll_handle: ScrollHandle,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
}

impl TabBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().px(px(-1.)),
            id: id.into(),
            children: SmallVec::new(),
            scroll_handle: ScrollHandle::new(),
            prefix: None,
            suffix: None,
        }
    }

    /// Track the scroll of the TabBar
    pub fn track_scroll(mut self, scroll_handle: ScrollHandle) -> Self {
        self.scroll_handle = scroll_handle;
        self
    }

    /// Set the prefix element of the TabBar
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the suffix element of the TabBar
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }
}

impl ParentElement for TabBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl Styled for TabBar {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for TabBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .id(self.id)
            .group("tab-bar")
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .bg(cx.theme().tab_bar)
            .text_color(cx.theme().tab_foreground)
            .child(
                div()
                    .id("border-b")
                    .absolute()
                    .bottom_0()
                    .size_full()
                    .border_b_1()
                    .border_color(cx.theme().border),
            )
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                h_flex()
                    .id("tabs")
                    .flex_grow()
                    .overflow_x_scroll()
                    .track_scroll(&self.scroll_handle)
                    .children(self.children),
            )
            .when_some(self.suffix, |this, suffix| this.child(suffix))
    }
}
