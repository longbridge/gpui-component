use std::sync::Arc;

use crate::{h_flex, ActiveTheme, Selectable, Sizable, Size, StyledExt};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, AnyElement, App, Div, Edges, ElementId, IntoElement, ParentElement, RenderOnce,
    ScrollHandle, StatefulInteractiveElement as _, Styled, Window,
};
use gpui::{px, InteractiveElement};
use smallvec::SmallVec;

use super::{Tab, TabVariant};

#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    base: Div,
    scroll_handle: Option<ScrollHandle>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: SmallVec<[Tab; 2]>,
    last_empty_space: Option<AnyElement>,
    selected_index: Option<usize>,
    variant: TabVariant,
    size: Size,
    on_click: Option<Arc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl TabBar {
    /// Create a new TabBar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div().px(px(-1.)),
            children: SmallVec::new(),
            scroll_handle: None,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            last_empty_space: None,
            selected_index: None,
            on_click: None,
        }
    }

    /// Set the Tab variant, all children will inherit the variant.
    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the Tab variant to Pill, all children will inherit the variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Set the Tab variant to Segmented, all children will inherit the variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Set the Tab variant to Underline, all children will inherit the variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Track the scroll of the TabBar
    pub fn track_scroll(mut self, scroll_handle: ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle);
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

    /// Add children of the TabBar, all children will inherit the variant.
    ///
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Add child of the TabBar, tab will inherit the variant.
    pub fn child(mut self, child: impl Into<Tab>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Set the selected index of the TabBar.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Set the last empty space element of the TabBar
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.last_empty_space = Some(last_empty_space.into_any_element());
        self
    }

    /// Set the on_click callback of the TabBar, the first parameter is the index of the clicked tab.
    ///
    /// When this is set, the children's on_click will be ignored.
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

impl Sizable for TabBar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_gap = match self.size {
            Size::Small | Size::XSmall => px(8.),
            _ => px(12.),
        };
        let (bg, paddings, gap) = match self.variant {
            TabVariant::Tab => {
                let padding = Edges::all(px(0.));
                (cx.theme().tab_bar, padding, px(0.))
            }
            TabVariant::Pill => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent, padding, default_gap)
            }
            TabVariant::Segmented => {
                let padding = Edges::all(px(4.));
                (cx.theme().accent, padding, default_gap / 2.)
            }
            TabVariant::Underline => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent, padding, default_gap / 2.)
            }
        };

        self.base
            .id(self.id)
            .group("tab-bar")
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .bg(bg)
            .text_color(cx.theme().tab_foreground)
            .when(
                self.variant == TabVariant::Underline || self.variant == TabVariant::Tab,
                |this| {
                    this.child(
                        div()
                            .id("border-b")
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .size_full()
                            .border_b_1()
                            .border_color(cx.theme().border),
                    )
                },
            )
            .when(
                self.variant == TabVariant::Pill || self.variant == TabVariant::Segmented,
                |this| this.rounded(cx.theme().radius),
            )
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                h_flex()
                    .id("tabs")
                    .flex_1()
                    .overflow_x_scroll()
                    .when_some(self.scroll_handle, |this, scroll_handle| {
                        this.track_scroll(&scroll_handle)
                    })
                    .gap(gap)
                    .paddings(paddings)
                    .children(
                        self.children
                            .into_iter()
                            .enumerate()
                            .map(move |(ix, child)| {
                                child
                                    .variant(self.variant)
                                    .with_size(self.size)
                                    .when_some(self.selected_index, |this, selected_ix| {
                                        this.selected(selected_ix == ix)
                                    })
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
