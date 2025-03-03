use std::sync::Arc;

use crate::{h_flex, ActiveTheme, Selectable};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, AnyElement, App, Div, ElementId, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    StatefulInteractiveElement as _, Styled, Window,
};
use gpui::{px, InteractiveElement};
use smallvec::SmallVec;

use super::{Tab, TabVariant};

#[derive(IntoElement)]
pub struct TabBar {
    base: Div,
    id: ElementId,
    scroll_handle: ScrollHandle,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: SmallVec<[Tab; 2]>,
    last_empty_space: Option<AnyElement>,
    selected_index: usize,
    variant: TabVariant,
    on_click: Option<Arc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
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
            variant: TabVariant::default(),
            last_empty_space: None,
            selected_index: 0,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
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

    pub fn children(mut self, children: impl IntoIterator<Item = Tab>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn child(mut self, child: Tab) -> Self {
        self.children.push(child);
        self
    }

    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// Set the last empty space element of the TabBar
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.last_empty_space = Some(last_empty_space.into_any_element());
        self
    }

    /// Set the on_click callback of the TabBar, the first parameter is the index of the clicked tab.
    pub fn on_click(mut self, on_click: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Arc::new(on_click));
        self
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
                    .children(
                        self.children
                            .into_iter()
                            .enumerate()
                            .map(move |(ix, child)| {
                                child
                                    .variant(self.variant)
                                    .selected(ix == self.selected_index)
                                    .when_some(self.on_click.clone(), move |this, on_click| {
                                        this.on_click(move |_, window, cx| {
                                            on_click(&ix, window, cx)
                                        })
                                    })
                            }),
                    )
                    .children(self.last_empty_space),
            )
            .when_some(self.suffix, |this, suffix| this.child(suffix))
    }
}
