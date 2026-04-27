use gpui::{
    AbsoluteLength, AnyElement, App, BorderStyle, Bounds, ContentMask, Corners, Element, Global,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Window, px, quad,
    transparent_black,
};

#[derive(Default)]
pub(crate) struct ForegroundDrawQueue {
    draws: Vec<ForegroundDraw>,
}

pub(crate) struct ForegroundDraw {
    bounds: Bounds<Pixels>,
    corner_radii: Corners<Pixels>,
    border_color: Hsla,
    border_width: Pixels,
    content_mask: ContentMask<Pixels>,
}

impl Global for ForegroundDrawQueue {}

/// Draw a border overlay on the foreground layer — above all content,
/// below all floating UI (popups, popovers, tooltips).
///
/// The `child` element participates in normal layout to determine the border position
/// and size, but is not painted. Instead a quad border is painted directly in the
/// foreground layer, avoiding GPUI arena lifetime issues.
///
/// Use this for focus rings, active selection borders, and similar decorations that
/// must not be clipped by sibling elements.
pub fn deferred_foreground(child: impl IntoElement) -> DeferredForeground {
    DeferredForeground {
        child: Some(child.into_any_element()),
        border_color: transparent_black(),
        border_width: px(1.),
        corner_radii: None,
    }
}

pub struct DeferredForeground {
    child: Option<AnyElement>,
    border_color: Hsla,
    border_width: Pixels,
    /// Stored as AbsoluteLength so we can convert to pixels in prepaint using rem_size.
    corner_radii: Option<Corners<AbsoluteLength>>,
}

impl DeferredForeground {
    pub fn border_color(mut self, color: impl Into<Hsla>) -> Self {
        self.border_color = color.into();
        self
    }

    pub fn border_width(mut self, width: impl Into<Pixels>) -> Self {
        self.border_width = width.into();
        self
    }

    pub fn corner_radii(mut self, radii: Corners<AbsoluteLength>) -> Self {
        self.corner_radii = Some(radii);
        self
    }

    pub fn corner_radius(mut self, radius: impl Into<AbsoluteLength>) -> Self {
        let r = radius.into();
        self.corner_radii = Some(Corners {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        });
        self
    }
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
        // Use the child only to establish layout bounds; it is never painted.
        let layout_id = self.child.as_mut().unwrap().request_layout(window, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        // Drop the arena-backed child; we only needed it for layout bounds.
        self.child = None;

        let rem_size = window.rem_size();
        let corner_radii = self
            .corner_radii
            .unwrap_or_default()
            .to_pixels(rem_size)
            .clamp_radii_for_quad_size(bounds.size);

        let content_mask = window.content_mask();
        cx.default_global::<ForegroundDrawQueue>()
            .draws
            .push(ForegroundDraw {
                bounds,
                corner_radii,
                border_color: self.border_color,
                border_width: self.border_width,
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
        _window: &mut Window,
        cx: &mut App,
    ) -> Vec<ForegroundDraw> {
        std::mem::take(&mut cx.default_global::<ForegroundDrawQueue>().draws)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        prepaint: &mut Vec<ForegroundDraw>,
        window: &mut Window,
        _cx: &mut App,
    ) {
        for draw in prepaint.iter() {
            window.with_content_mask(Some(draw.content_mask.clone()), |window| {
                window.paint_quad(quad(
                    draw.bounds,
                    draw.corner_radii,
                    transparent_black(),
                    draw.border_width,
                    draw.border_color,
                    BorderStyle::default(),
                ));
            });
        }
    }
}
