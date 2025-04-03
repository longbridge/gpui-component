use crate::{
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollbarAxis,
    v_flex, ActiveTheme, Collapsible, Icon, IconName, Side, Sizable, StyledExt,
};
use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, ClickEvent, InteractiveElement as _,
    IntoElement, ParentElement, Pixels, RenderOnce, Styled, Window,
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
    content: Vec<E>,
    /// header view
    header: Option<AnyElement>,
    /// footer view
    footer: Option<AnyElement>,
    /// The side of the sidebar
    side: Side,
    collapsible: bool,
    width: Pixels,
    collapsed: bool,
}

impl<E: Collapsible + IntoElement> Sidebar<E> {
    pub fn new(side: Side) -> Self {
        Self {
            content: vec![],
            header: None,
            footer: None,
            side,
            collapsible: true,
            width: DEFAULT_WIDTH,
            collapsed: false,
        }
    }

    pub fn left() -> Self {
        Self::new(Side::Left)
    }

    pub fn right() -> Self {
        Self::new(Side::Right)
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
        self.collapsed = collapsed;
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
    collapsed: bool,
    side: Side,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl SidebarToggleButton {
    fn new(side: Side) -> Self {
        Self {
            btn: Button::new("collapse").ghost().small(),
            collapsed: false,
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

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }
}

impl RenderOnce for SidebarToggleButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let collapsed = self.collapsed;
        let on_click = self.on_click.clone();

        let icon = if collapsed {
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
                this.on_click(move |ev, window, cx| {
                    on_click(ev, window, cx);
                })
            })
            .icon(Icon::new(icon).size_4())
    }
}

impl<E: Collapsible + IntoElement> RenderOnce for Sidebar<E> {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view_id = window.current_view();
        v_flex()
            .id("sidebar")
            .w(self.width)
            .when(self.collapsed, |this| this.w(COLLAPSED_WIDTH))
            .flex_shrink_0()
            .h_full()
            .overflow_hidden()
            .relative()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .border_color(cx.theme().sidebar_border)
            .map(|this| match self.side {
                Side::Left => this.border_r_1(),
                Side::Right => this.border_l_1(),
            })
            .when_some(self.header.take(), |this, header| {
                this.child(h_flex().id("header").p_2().gap_2().child(header))
            })
            .child(
                v_flex().id("content").flex_1().min_h_0().child(
                    div()
                        .children(
                            self.content
                                .into_iter()
                                .enumerate()
                                .map(|(ix, c)| div().id(ix).child(c.collapsed(self.collapsed))),
                        )
                        .gap_2()
                        .scrollable(view_id, ScrollbarAxis::Vertical),
                ),
            )
            .when_some(self.footer.take(), |this, footer| {
                this.child(h_flex().id("footer").gap_2().p_2().child(footer))
            })
    }
}
