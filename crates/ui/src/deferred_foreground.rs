use gpui::{
    AnyElement, App, Bounds, ContentMask, Element, Global, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Point, Window,
};

#[derive(Default)]
pub(crate) struct ForegroundDrawQueue {
    draws: Vec<ForegroundDraw>,
}

pub(crate) struct ForegroundDraw {
    child: AnyElement,
    offset: Point<Pixels>,
    content_mask: ContentMask<Pixels>,
}

impl Global for ForegroundDrawQueue {}

/// Draw a child on the foreground layer — above all content, below all floating UI
/// (popups, popovers, tooltips). The child participates in normal layout but its
/// painting is deferred until after all sibling content has been painted.
///
/// The current scroll clip is preserved, so the border is correctly clipped when
/// inside a scroll container.
///
/// Use this for focus rings, active selection borders, and similar widget decorations
/// that must not be clipped by sibling elements.
pub fn deferred_foreground(child: impl IntoElement) -> DeferredForeground {
    DeferredForeground {
        child: Some(child.into_any_element()),
    }
}

pub struct DeferredForeground {
    child: Option<AnyElement>,
}

impl IntoElement for DeferredForeground {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DeferredForeground {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout_id = self.child.as_mut().unwrap().request_layout(window, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = self.child.take().unwrap();
        let offset = window.element_offset();
        let content_mask = window.content_mask();
        cx.default_global::<ForegroundDrawQueue>()
            .draws
            .push(ForegroundDraw {
                child,
                offset,
                content_mask,
            });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
}

/// An element that flushes the foreground draw queue. Must be placed in the Root
/// element tree after the main content view and before any deferred floating UI.
pub(crate) struct ForegroundLayer;

impl IntoElement for ForegroundLayer {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ForegroundLayer {
    type RequestLayoutState = ();
    type PrepaintState = Vec<ForegroundDraw>;

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout_id = window.request_layout(gpui::Style::default(), [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<ForegroundDraw> {
        let draws = std::mem::take(&mut cx.default_global::<ForegroundDrawQueue>().draws);
        let mut prepainted = Vec::new();
        for mut draw in draws {
            window.with_content_mask(Some(draw.content_mask.clone()), |window| {
                window.with_absolute_element_offset(draw.offset, |window| {
                    draw.child.prepaint(window, cx);
                });
            });
            prepainted.push(draw);
        }
        prepainted
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        prepaint: &mut Vec<ForegroundDraw>,
        window: &mut Window,
        cx: &mut App,
    ) {
        for draw in prepaint.iter_mut() {
            window.with_content_mask(Some(draw.content_mask.clone()), |window| {
                draw.child.paint(window, cx);
            });
        }
    }
}
