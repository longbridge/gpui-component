use gpui::{
    Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, Pixels, Window,
};
use gpui_shell::gpui::IntoElement as _;
use gpui_shell::{anyhow, gpui};

pub(super) struct Carrier<T: 'static>(Option<T>);
impl<T: 'static> Carrier<T> {
    pub(super) fn new(value: T) -> Self {
        Self(Some(value))
    }
}
impl<T: 'static> gpui::IntoElement for Carrier<T> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<T: 'static> Element for Carrier<T> {
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
        w: &mut Window,
        c: &mut gpui::App,
    ) -> (LayoutId, gpui::AnyElement) {
        let mut e = gpui::div().into_any_element();
        let id = e.request_layout(w, c);
        (id, e)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        e: &mut gpui::AnyElement,
        w: &mut Window,
        c: &mut gpui::App,
    ) {
        e.prepaint(w, c);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        e: &mut gpui::AnyElement,
        _: &mut (),
        w: &mut Window,
        c: &mut gpui::App,
    ) {
        e.paint(w, c)
    }
}
pub(super) fn take<T: 'static>(e: &mut gpui::AnyElement, name: &str) -> anyhow::Result<T> {
    e.downcast_mut::<Carrier<T>>()
        .ok_or_else(|| anyhow::anyhow!("{name} materialized an incompatible child"))?
        .0
        .take()
        .ok_or_else(|| anyhow::anyhow!("{name} child already consumed"))
}
