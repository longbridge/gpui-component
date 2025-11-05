use super::{Scrollbar, ScrollbarAxis, ScrollbarState};
use gpui::{
    div, relative, AnyElement, App, Bounds, Div, Element, ElementId, GlobalElementId,
    InspectorElementId, InteractiveElement, Interactivity, IntoElement, LayoutId, ParentElement,
    Pixels, Position, ScrollHandle, SharedString, Stateful, StatefulInteractiveElement, Style,
    StyleRefinement, Styled, Window,
};

/// A scroll view is a container that allows the user to scroll through a large amount of content.
pub struct Scrollable<E> {
    id: ElementId,
    element: Option<E>,
    axis: ScrollbarAxis,
    /// This is a fake element to handle Styled, InteractiveElement, not used.
    _element: Stateful<Div>,
}

impl<E> Scrollable<E>
where
    E: Element,
{
    pub(crate) fn new(axis: impl Into<ScrollbarAxis>, element: E) -> Self {
        let id = ElementId::Name(SharedString::from(
            format!("scrollable-{:?}", element.id(),),
        ));

        Self {
            element: Some(element),
            _element: div().id("fake"),
            id,
            axis: axis.into(),
        }
    }

    /// Set only a vertical scrollbar.
    pub fn vertical(self) -> Self {
        self.axis(ScrollbarAxis::Vertical)
    }

    /// Set only a horizontal scrollbar.
    /// In current implementation, this is not supported yet.
    pub fn horizontal(self) -> Self {
        self.axis(ScrollbarAxis::Horizontal)
    }

    /// Set the axis of the scroll view.
    pub fn axis(mut self, axis: impl Into<ScrollbarAxis>) -> Self {
        self.axis = axis.into();
        self
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

#[doc(hidden)]
pub struct ScrollViewState {
    state: ScrollbarState,
    handle: ScrollHandle,
}

impl Default for ScrollViewState {
    fn default() -> Self {
        Self {
            handle: ScrollHandle::new(),
            state: ScrollbarState::default(),
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
    fn interactivity(&mut self) -> &mut Interactivity {
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

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.flex_grow = 1.0;
        style.position = Position::Relative;
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();

        let axis = self.axis;
        let scroll_id = self.id.clone();
        let content = self.element.take().map(|c| c.into_any_element());

        self.with_element_state(id.unwrap(), window, cx, |_, element_state, window, cx| {
            let mut element = div()
                .relative()
                .size_full()
                .overflow_hidden()
                .child(
                    div()
                        .id(scroll_id)
                        .track_scroll(&element_state.handle)
                        .overflow_scroll()
                        .relative()
                        .size_full()
                        .child(div().children(content)),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(
                            Scrollbar::both(&element_state.state, &element_state.handle).axis(axis),
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
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
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
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx)
    }
}
