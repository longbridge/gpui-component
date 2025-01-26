use crate::{
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollbarAxis,
    v_flex, ActiveTheme, Collapsible, Icon, IconName, Side, Sizable, StyledExt,
};
use gpui::{Window, AppContext, Model, 
    div, prelude::FluentBuilder, px, AnyElement, ClickEvent, Entity, EntityId,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, Render, RenderOnce, Styled, 
    
};
use std::rc::Rc;

mod footer;
mod group;
mod header;
mod menu;
pub use footer::*;
pub use group::*;
pub use header::*;
pub use menu::*;

const DEFAULT_WIDTH: Pixels = px(255.);
const COLLAPSED_WIDTH: Pixels = px(48.);

/// A sidebar
#[derive(IntoElement)]
pub struct Sidebar<E: Collapsible + IntoElement + 'static> {
    /// The parent view id
    view_id: EntityId,
    content: Vec<E>,
    /// header view
    header: Option<AnyElement>,
    /// footer view
    footer: Option<AnyElement>,
    /// The side of the sidebar
    side: Side,
    collapsible: bool,
    width: Pixels,
    is_collapsed: bool,
}

impl<E: Collapsible + IntoElement> Sidebar<E> {
    fn new(view_id: EntityId, side: Side) -> Self {
        Self {
            view_id,
            content: vec![],
            header: None,
            footer: None,
            side,
            collapsible: true,
            width: DEFAULT_WIDTH,
            is_collapsed: false,
        }
    }

    pub fn left<V: Render + 'static>(view: &View<V>) -> Self {
        Self::new(view.entity_id(), Side::Left)
    }

    pub fn right<V: Render + 'static>(view: &View<V>) -> Self {
        Self::new(view.entity_id(), Side::Right)
    }

    /// Set the width of the sidebar
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Set the sidebar to be collapsible, default is true
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Set the sidebar to be collapsed
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.is_collapsed = collapsed;
        self
    }

    /// Set the header of the sidebar.
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Set the footer of the sidebar.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Add a child element to the sidebar, the child must implement `Collapsible`
    pub fn child(mut self, child: E) -> Self {
        self.content.push(child);
        self
    }

    /// Add multiple children to the sidebar, the children must implement `Collapsible`
    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.content.extend(children);
        self
    }
}

/// Sidebar collapse button with Icon.
#[derive(IntoElement)]
pub struct SidebarToggleButton {
    btn: Button,
    is_collapsed: bool,
    side: Side,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut WindowContext)>>,
}

impl SidebarToggleButton {
    fn new(side: Side) -> Self {
        Self {
            btn: Button::new("sidebar-collapse").ghost().small(),
            is_collapsed: false,
            side,
            on_click: None,
        }
    }

    pub fn left() -> Self {
        Self::new(Side::Left)
    }

    pub fn right() -> Self {
        Self::new(Side::Right)
    }

    pub fn collapsed(mut self, is_collapsed: bool) -> Self {
        self.is_collapsed = is_collapsed;
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }
}

impl RenderOnce for SidebarToggleButton {
    fn render(self, _: &mut WindowContext) -> impl IntoElement {
        let is_collapsed = self.is_collapsed;
        let on_click = self.on_click.clone();

        let icon = if is_collapsed {
            if self.side.is_left() {
                IconName::PanelLeftOpen
            } else {
                IconName::PanelRightOpen
            }
        } else {
            if self.side.is_left() {
                IconName::PanelLeftClose
            } else {
                IconName::PanelRightClose
            }
        };

        self.btn
            .when_some(on_click, |this, on_click| {
                this.on_click(move |ev, cx| {
                    on_click(ev, cx);
                })
            })
            .icon(Icon::new(icon).size_4())
    }
}

impl<E: Collapsible + IntoElement> RenderOnce for Sidebar<E> {
    fn render(mut self, cx: &mut WindowContext) -> impl IntoElement {
        let is_collapsed = self.is_collapsed;
        v_flex()
            .id("sidebar")
            .w(self.width)
            .when(self.is_collapsed, |this| this.w(COLLAPSED_WIDTH))
            .flex_shrink_0()
            .h_full()
            .overflow_hidden()
            .relative()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .border_color(cx.theme().sidebar_border)
            .map(|this| match self.side {
                Side::Left => this.border_r_1(),
                Side::Right => this.text_2xl(),
            })
            .when_some(self.header.take(), |this, header| {
                this.child(h_flex().id("header").p_2().gap_2().child(header))
            })
            .child(
                v_flex().id("content").flex_1().min_h_0().child(
                    div()
                        .children(self.content.into_iter().map(|c| c.collapsed(is_collapsed)))
                        .gap_2()
                        .scrollable(self.view_id, ScrollbarAxis::Vertical),
                ),
            )
            .when_some(self.footer.take(), |this, footer| {
                this.child(h_flex().id("footer").gap_2().p_2().child(footer))
            })
    }
}
