use std::{cell::Cell, rc::Rc};

use super::{Scrollbar, ScrollbarAxis, ScrollbarState};
use gpui::{
    canvas, div, relative, AnyElement, App, Div, Element, ElementId, EntityId, GlobalElementId,
    InteractiveElement, IntoElement, ParentElement, Pixels, Position, ScrollHandle, SharedString,
    Size, Stateful, StatefulInteractiveElement, Style, StyleRefinement, Styled, Window,
};

/// A scroll view is a container that allows the user to scroll through a large amount of content.
pub struct Scrollable<E> {
    id: ElementId,
    element: Option<E>,
    view_id: EntityId,
    axis: ScrollbarAxis,
    /// This is a fake element to handle Styled, InteractiveElement, not used.
    _element: Stateful<Div>,
}

impl<E> Scrollable<E>
where
    E: Element,
{
    pub(crate) fn new(view_id: EntityId, element: E, axis: ScrollbarAxis) -> Self {
        let id = ElementId::Name(SharedString::from(format!(
            "ScrollView:{}-{:?}",
            view_id,
            element.id(),
        )));

        Self {
            element: Some(element),
            _element: div().id("fake"),
            id,
            view_id,
            axis,
        }
    }

    /// Set only a vertical scrollbar.
    pub fn vertical(mut self) -> Self {
        self.set_axis(ScrollbarAxis::Vertical);
        self
    }

    /// Set only a horizontal scrollbar.
    /// In current implementation, this is not supported yet.
    pub fn horizontal(mut self) -> Self {
        self.set_axis(ScrollbarAxis::Horizontal);
        self
    }

    /// Set the axis of the scroll view.
    pub fn set_axis(&mut self, axis: ScrollbarAxis) {
        self.axis = axis;
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut ScrollViewState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<ScrollViewState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

pub struct ScrollViewState {
    scroll_size: Rc<Cell<Size<Pixels>>>,
    state: Rc<Cell<ScrollbarState>>,
    handle: ScrollHandle,
}

impl Default for ScrollViewState {
    fn default() -> Self {
        Self {
            handle: ScrollHandle::new(),
            scroll_size: Rc::new(Cell::new(Size::default())),
            state: Rc::new(Cell::new(ScrollbarState::default())),
        }
    }
}

impl<E> ParentElement for Scrollable<E>
where
    E: Element + ParentElement,
{
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E> Styled for Scrollable<E>
where
    E: Element + Styled,
{
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            self._element.style()
        }
    }
}

impl<E> InteractiveElement for Scrollable<E>
where
    E: Element + InteractiveElement,
{
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        if let Some(element) = &mut self.element {
            element.interactivity()
        } else {
            self._element.interactivity()
        }
    }
}
impl<E> StatefulInteractiveElement for Scrollable<E> where E: Element + StatefulInteractiveElement {}

impl<E> IntoElement for Scrollable<E>
where
    E: Element,
{
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E> Element for Scrollable<E>
where
    E: Element,
{
    type RequestLayoutState = AnyElement;
    type PrepaintState = ScrollViewState;

    fn id(&self) -> Option<gpui::ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.flex_grow = 1.0;
        style.position = Position::Relative;
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();

        let axis = self.axis;
        let view_id = self.view_id;

        let scroll_id = self.id.clone();
        let content = self.element.take().map(|c| c.into_any_element());

        self.with_element_state(id.unwrap(), window, cx, |_, element_state, window, cx| {
            let handle = element_state.handle.clone();
            let state = element_state.state.clone();
            let scroll_size = element_state.scroll_size.clone();

            let mut element = div()
                .relative()
                .size_full()
                .overflow_hidden()
                .child(
                    div()
                        .id(scroll_id)
                        .track_scroll(&handle)
                        .overflow_scroll()
                        .relative()
                        .size_full()
                        .child(div().children(content).child({
                            let scroll_size = element_state.scroll_size.clone();
                            canvas(move |b, _, _| scroll_size.set(b.size), |_, _, _, _| {})
                                .absolute()
                                .size_full()
                        })),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(
                            Scrollbar::both(view_id, state, handle.clone(), scroll_size.get())
                                .axis(axis),
                        ),
                )
                .into_any_element();
            let element_id = element.request_layout(window, cx);

            let layout_id = window.request_layout(style, vec![element_id], cx);

            (layout_id, element)
        })
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        element.prepaint(window, cx);
        // do nothing
        ScrollViewState::default()
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx)
    }
}
