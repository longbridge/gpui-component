use crate::scroll::ScrollbarHandle;

use super::{Scrollbar, ScrollbarAxis};
use gpui::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder,
};

pub trait ScrollableElement: ParentElement + Sized {
    /// Adds a scrollbar to the element.
    fn scrollbar<H: ScrollbarHandle + Clone>(
        self,
        scroll_handle: &H,
        axis: impl Into<ScrollbarAxis>,
    ) -> Self {
        self.child(render_scrollbar(scroll_handle, axis))
    }

    /// Adds a vertical scrollbar to the element.
    fn vertical_scrollbar<H: ScrollbarHandle + Clone>(self, scroll_handle: &H) -> Self {
        self.scrollbar(scroll_handle, ScrollbarAxis::Vertical)
    }
    /// Adds a horizontal scrollbar to the element.
    fn horizontal_scrollbar<H: ScrollbarHandle + Clone>(self, scroll_handle: &H) -> Self {
        self.scrollbar(scroll_handle, ScrollbarAxis::Horizontal)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_scroll`], but adds scrollbars.
    fn overflow_scrollbar(self) -> Scrollable<Self> {
        Scrollable {
            element: self,
            axis: ScrollbarAxis::Both,
        }
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_x_scroll`], but adds Horizontal scrollbar.
    fn overflow_x_scrollbar(self) -> Scrollable<Self> {
        Scrollable {
            element: self,
            axis: ScrollbarAxis::Horizontal,
        }
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_y_scroll`], but adds Vertical scrollbar.
    fn overflow_y_scrollbar(self) -> Scrollable<Self> {
        Scrollable {
            element: self,
            axis: ScrollbarAxis::Vertical,
        }
    }
}

/// A scrollable element wrapper that adds scrollbars to an interactive element.
#[derive(IntoElement)]
pub struct Scrollable<E> {
    element: E,
    children: Vec<AnyElement>,
    axis: ScrollbarAxis,
}

impl<E> Styled for Scrollable<E>
where
    E: InteractiveElement + Styled + IntoElement,
{
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.style()
    }
}

impl<E> ParentElement for Scrollable<E>
where
    E: ParentElement + IntoElement,
{
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.children.extend(elements);
    }
}

impl<E> RenderOnce for Scrollable<E>
where
    E: StatefulInteractiveElement + ParentElement + IntoElement + 'static,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = window
            .use_state(cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();

        self.element
            .track_scroll(&scroll_handle)
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.overflow_y_scroll(),
                ScrollbarAxis::Horizontal => this.overflow_x_scroll(),
                ScrollbarAxis::Both => this.overflow_scroll(),
            })
            .children(self.children)
            .child(render_scrollbar(&scroll_handle, self.axis.clone()))
    }
}

impl ScrollableElement for Div {}
impl<E: ParentElement> ScrollableElement for Stateful<E> {}

#[inline]
#[track_caller]
fn render_scrollbar<H: ScrollbarHandle + Clone>(
    scroll_handle: &H,
    axis: impl Into<ScrollbarAxis>,
) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .p_1()
        .child(Scrollbar::new(scroll_handle).axis(axis))
}
