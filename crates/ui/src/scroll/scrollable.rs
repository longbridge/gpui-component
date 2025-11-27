use std::{panic::Location, rc::Rc};

use crate::{StyledExt, scroll::ScrollbarHandle};

use super::{Scrollbar, ScrollbarAxis};
use gpui::{
    App, Div, Element, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    ScrollHandle, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder,
};

/// A trait for elements that can be made scrollable with scrollbars.
pub trait ScrollableElement: InteractiveElement + Styled + ParentElement + Element {
    /// Adds a scrollbar to the element.
    #[track_caller]
    fn scrollbar<H: ScrollbarHandle + Clone>(
        self,
        scroll_handle: &H,
        axis: impl Into<ScrollbarAxis>,
    ) -> Scrollable<Self> {
        Scrollable::new(self, axis).scroll_handle(scroll_handle)
    }

    /// Adds a vertical scrollbar to the element.
    #[track_caller]
    fn vertical_scrollbar<H: ScrollbarHandle + Clone>(self, scroll_handle: &H) -> Scrollable<Self> {
        self.scrollbar(scroll_handle, ScrollbarAxis::Vertical)
    }
    /// Adds a horizontal scrollbar to the element.
    #[track_caller]
    fn horizontal_scrollbar<H: ScrollbarHandle + Clone>(
        self,
        scroll_handle: &H,
    ) -> Scrollable<Self> {
        self.scrollbar(scroll_handle, ScrollbarAxis::Horizontal)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_scroll`], but adds scrollbars.
    #[track_caller]
    fn overflow_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Both)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_x_scroll`], but adds Horizontal scrollbar.
    #[track_caller]
    fn overflow_x_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Horizontal)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_y_scroll`], but adds Vertical scrollbar.
    #[track_caller]
    fn overflow_y_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Vertical)
    }
}

/// A scrollable element wrapper that adds scrollbars to an interactive element.
#[derive(IntoElement)]
pub struct Scrollable<E: InteractiveElement + Styled + ParentElement + Element> {
    id: ElementId,
    element: E,
    axis: ScrollbarAxis,
    scroll_handle: Option<Rc<dyn ScrollbarHandle>>,
}

impl<E> Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    #[track_caller]
    fn new(element: E, axis: impl Into<ScrollbarAxis>) -> Self {
        let caller = Location::caller();
        Self {
            id: ElementId::CodeLocation(*caller),
            element,
            axis: axis.into(),
            scroll_handle: None,
        }
    }

    fn scroll_handle<H: ScrollbarHandle + Clone + 'static>(mut self, handle: &H) -> Self {
        self.scroll_handle = Some(Rc::new(handle.clone()));
        self
    }
}

impl<E> Styled for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.style()
    }
}

impl<E> ParentElement for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.element.extend(elements)
    }
}

impl InteractiveElement for Scrollable<Div> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.element.interactivity()
    }
}

impl InteractiveElement for Scrollable<Stateful<Div>> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.element.interactivity()
    }
}

impl<E> RenderOnce for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element + 'static,
{
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();

        let style = self.element.style().clone();
        *self.element.style() = StyleRefinement::default();

        div()
            .id(self.id)
            .size_full()
            .refine_style(&style)
            .relative()
            .child(
                div()
                    .id("scroll-area")
                    .flex()
                    .size_full()
                    .track_scroll(&scroll_handle)
                    .map(|this| match self.axis {
                        ScrollbarAxis::Vertical => this.flex_col().overflow_y_scroll(),
                        ScrollbarAxis::Horizontal => this.flex_row().overflow_x_scroll(),
                        ScrollbarAxis::Both => this.overflow_scroll(),
                    })
                    .child(self.element.flex_1().min_h_full().min_w_full()),
            )
            .child(render_scrollbar(
                "scrollbar",
                &scroll_handle,
                self.axis,
                window,
                cx,
            ))
    }
}

impl ScrollableElement for Div {}
impl<E> ScrollableElement for Stateful<E>
where
    E: ParentElement + Styled + Element,
    Self: InteractiveElement,
{
}

#[inline]
#[track_caller]
fn render_scrollbar<H: ScrollbarHandle + Clone>(
    id: impl Into<ElementId>,
    scroll_handle: &H,
    axis: ScrollbarAxis,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    // Do not render scrollbar when inspector is picking elements,
    // to allow us to pick the background elements.
    let is_inspector_picking = window.is_inspector_picking(cx);
    if is_inspector_picking {
        return div();
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(Scrollbar::new(scroll_handle).id(id).axis(axis))
}
