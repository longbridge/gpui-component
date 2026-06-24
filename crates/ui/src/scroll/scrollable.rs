use std::{panic::Location, rc::Rc};

use crate::StyledExt;

use super::{Scrollbar, ScrollbarAxis, ScrollbarHandle};
use gpui::{
    App, Div, Element, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    ScrollHandle, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder,
};

/// A trait for elements that can be made scrollable with scrollbars. 
//  The wrapped element is the scroll area itself, rather than being inserted as a child of a new scroll area.
pub trait ScrollableElement: InteractiveElement + Styled + ParentElement + Element {
    /// Adds a scrollbar to the element.
    #[track_caller]
    fn scrollbar<H: ScrollbarHandle + Clone>(
        self,
        scroll_handle: &H,
        axis: impl Into<ScrollbarAxis>,
    ) -> Self {
        self.child(ScrollbarLayer {
            id: caller_id(),
            axis: axis.into(),
            scroll_handle: Rc::new(scroll_handle.clone()),
        })
    }

    /// Adds a vertical scrollbar to the element.
    #[track_caller]
    fn vertical_scrollbar<H: ScrollbarHandle + Clone>(self, scroll_handle: &H) -> Self {
        self.scrollbar(scroll_handle, ScrollbarAxis::Vertical)
    }

    /// Adds a horizontal scrollbar to the element.
    #[track_caller]
    fn horizontal_scrollbar<H: ScrollbarHandle + Clone>(self, scroll_handle: &H) -> Self {
        self.scrollbar(scroll_handle, ScrollbarAxis::Horizontal)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_scroll`], but adds scrollbars.
    /// Preserves the source element as the scrollable container.
    #[track_caller]
    fn overflow_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Both)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_x_scroll`], but adds Horizontal scrollbar.
    /// Preserves the source element as the scrollable container.
    #[track_caller]
    fn overflow_x_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Horizontal)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_y_scroll`], but adds Vertical scrollbar.
    /// Preserves the source element as the scrollable container.
    #[track_caller]
    fn overflow_y_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Vertical)
    }
}

/// A scrollable element wrapper that renders the original element as the scroll area and overlays scrollbars.
#[derive(IntoElement)]
pub struct Scrollable<E: InteractiveElement + Styled + ParentElement + Element> {
    id: ElementId,
    element: E,
    axis: ScrollbarAxis,
}

impl<E> Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    #[track_caller]
    fn new(element: E, axis: impl Into<ScrollbarAxis>) -> Self {
        Self {
            id: caller_id(),
            element,
            axis: axis.into(),
        }
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

impl<E> InteractiveElement for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.element.interactivity()
    }
}

impl<E> RenderOnce for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element + 'static,
{
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = scroll_handle_for(&self.id, window, cx);

        // Preserve the caller-requested size on the wrapper, while keeping the
        // caller's element as the actual scroll-tracked layout container.
        let root_style = root_style_from(&mut self.element);

        let root_id = self.id.clone();
        let area_id = (self.id.clone(), "area");
        let content_id = (self.id.clone(), "content");
        let scrollbar_id = (self.id.clone(), "scrollbar");

        let content = self
            .element
            .id(content_id)
            .flex_none()
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.h_auto().min_h_full(),
                ScrollbarAxis::Horizontal => this.w_auto().min_w_full(),
                ScrollbarAxis::Both => this.size_auto().min_size_full(),
            });

        let scroll_area = div()
            .id(area_id)
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .size_full()
            .flex()
            .track_scroll(&scroll_handle)
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.flex_col().overflow_y_scroll(),
                ScrollbarAxis::Horizontal => this.flex_row().overflow_x_scroll(),
                ScrollbarAxis::Both => this.overflow_scroll(),
            })
            .child(content);

        div()
            .id(root_id)
            .size_full()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .refine_style(&root_style)
            .relative()
            .overflow_hidden()
            .child(scroll_area)
            .child(render_scrollbar(
                scrollbar_id,
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

#[derive(IntoElement)]
struct ScrollbarLayer<H: ScrollbarHandle + Clone> {
    id: ElementId,
    axis: ScrollbarAxis,
    scroll_handle: Rc<H>,
}

impl<H> RenderOnce for ScrollbarLayer<H>
where
    H: ScrollbarHandle + Clone + 'static,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_scrollbar(self.id, self.scroll_handle.as_ref(), self.axis, window, cx)
    }
}

#[inline]
#[track_caller]
fn caller_id() -> ElementId {
    ElementId::CodeLocation(*Location::caller())
}

#[inline]
fn scroll_handle_for(id: &ElementId, window: &mut Window, cx: &mut App) -> ScrollHandle {
    window
        .use_keyed_state(id.clone(), cx, |_, _| ScrollHandle::default())
        .read(cx)
        .clone()
}

#[inline]
fn root_style_from<E>(element: &mut E) -> StyleRefinement
where
    E: Styled,
{
    StyleRefinement {
        size: element.style().size.clone(),
        ..Default::default()
    }
}

#[inline]
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, Render, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point,
        px,
    };

    struct SizeFullChildTest;

    impl Render for SizeFullChildTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(100.))
                .h(px(100.))
                .overflow_y_scrollbar()
                .child(
                    div()
                        .size_full()
                        .child(crate::v_flex().children((0..4).map(|ix| {
                            div().h(px(50.)).flex_shrink_0().when(ix == 3, |this| {
                                this.debug_selector(|| "last-row".to_string())
                            })
                        }))),
                )
        }
    }

    struct GapLayoutTest;

    impl Render for GapLayoutTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::v_flex()
                .w(px(100.))
                .h(px(100.))
                .gap(px(10.))
                .overflow_y_scrollbar()
                .child(
                    div()
                        .h(px(20.))
                        .flex_shrink_0()
                        .debug_selector(|| "first-row".to_string()),
                )
                .child(
                    div()
                        .h(px(20.))
                        .flex_shrink_0()
                        .debug_selector(|| "second-row".to_string()),
                )
        }
    }

    #[gpui::test]
    fn vertical_scrollbar_scrolls_past_a_size_full_child(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| SizeFullChildTest);
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        let initial_y = cx.debug_bounds("last-row").unwrap().origin.y;
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-50.))),
            ..Default::default()
        });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        assert!(cx.debug_bounds("last-row").unwrap().origin.y < initial_y);
    }

    #[gpui::test]
    fn vertical_scrollbar_preserves_source_gap(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| GapLayoutTest);
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        let first = cx.debug_bounds("first-row").unwrap();
        let second = cx.debug_bounds("second-row").unwrap();
        assert_eq!(second.top() - first.bottom(), px(10.));
    }
}
