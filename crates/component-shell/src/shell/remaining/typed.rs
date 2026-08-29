use gpui::{
    Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, Pixels, Window,
};
use gpui_shell::gpui::IntoElement as _;
use gpui_shell::{anyhow, gpui};

pub(super) struct OpaqueChildElement<T: 'static> {
    value: Option<T>,
}

impl<T: 'static> OpaqueChildElement<T> {
    pub(super) fn new(value: T) -> Self {
        Self { value: Some(value) }
    }

    fn take(&mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T: 'static> gpui::IntoElement for OpaqueChildElement<T> {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl<T: 'static> Element for OpaqueChildElement<T> {
    type RequestLayoutState = gpui::AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut element = gpui::div().into_any_element();
        let layout = element.request_layout(window, cx);
        (layout, element)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut (),
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        element.paint(window, cx);
    }
}

pub(super) fn take_opaque<T: 'static>(
    element: &mut gpui::AnyElement,
    name: &str,
) -> anyhow::Result<T> {
    element
        .downcast_mut::<OpaqueChildElement<T>>()
        .ok_or_else(|| anyhow::anyhow!("registered {name} materialized an incompatible element"))?
        .take()
        .ok_or_else(|| anyhow::anyhow!("registered {name} child was already consumed"))
}
