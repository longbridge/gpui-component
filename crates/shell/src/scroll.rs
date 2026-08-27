//! A scroll area that paints scrollbars over the element it was called on.
//!
//! This lives here rather than being imported because the shell is built on
//! `gpui-base` alone: everything it draws has to be expressible in the base
//! layer, and a dependency on `gpui-component` would put the product component
//! library underneath a runtime that is supposed to sit beside it. The scrollbar
//! itself — [`gpui_base::Scrollbar`] — is a base type; only this wrapper had to
//! come along. `gpui-component` keeps its own copy behind a
//! `ScrollableElement` trait; the shell needs one call site, so it names
//! [`Scrollable`] directly and skips the trait.

use std::panic::Location;

use gpui::{
    App, Div, Element, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    ScrollHandle, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder,
};
use gpui_base::{InteractiveElementExt as _, Scrollbar, ScrollbarAxis, StyledExt as _};

/// A scrollable element wrapper that renders the original element as the scroll area and overlays scrollbars.
#[derive(IntoElement)]
pub(crate) struct Scrollable<E: InteractiveElement + Styled + ParentElement + Element> {
    id: ElementId,
    element: E,
    axis: ScrollbarAxis,
}

impl<E> Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    #[track_caller]
    pub(crate) fn new(element: E, axis: impl Into<ScrollbarAxis>) -> Self {
        Self {
            id: caller_id(),
            element,
            axis: axis.into(),
        }
    }

    /// Set a specific element id, default is the [`std::panic::Location::caller`].
    ///
    /// Only needed when one call site creates several scrollables, which would
    /// otherwise share a single scroll position.
    pub(crate) fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
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

        // Keep the scroll area in the normal flow: its content size must
        // propagate to auto-sized ancestors (e.g. a Dialog that grows with
        // its content). An absolutely positioned scroll area would collapse
        // such ancestors to zero height.
        let scroll_area = div()
            .id(area_id)
            .size_full()
            .flex()
            .track_scroll(&scroll_handle)
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.flex_col().overflow_y_scroll(),
                ScrollbarAxis::Horizontal => this.flex_row().overflow_x_scroll(),
                ScrollbarAxis::Both => this.overflow_scroll(),
            })
            // On a single-axis area gpui otherwise remaps the other axis' delta
            // onto ours, so a purely horizontal swipe scrolls this vertically.
            .lock_scroll_axis()
            .child(content);

        div()
            .id(root_id)
            .size_full()
            .refine_style(&root_style)
            .relative()
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

/// Copies the outer layout styles from the element, so the wrapper can
/// participate in the parent's layout the same way the source element would.
#[inline]
fn root_style_from<E>(element: &mut E) -> StyleRefinement
where
    E: Styled,
{
    let style = element.style();
    StyleRefinement {
        size: style.size.clone(),
        min_size: style.min_size.clone(),
        max_size: style.max_size.clone(),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis,
        align_self: style.align_self,
        ..Default::default()
    }
}

#[inline]
fn render_scrollbar(
    id: impl Into<ElementId>,
    scroll_handle: &ScrollHandle,
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
        .inset_0()
        .debug_selector(|| "scrollbar-overlay".to_string())
        .child(
            Scrollbar::new(scroll_handle)
                .id(id)
                .axis(axis)
                .viewport_from_layout(),
        )
}
