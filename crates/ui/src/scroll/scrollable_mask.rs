use gpui::{
    px, relative, App, Axis, BorderStyle, Bounds, ContentMask, Corners, Edges, Element, ElementId,
    EntityId, GlobalElementId, Hitbox, Hsla, IntoElement, IsZero as _, LayoutId, PaintQuad, Pixels,
    Point, Position, ScrollHandle, ScrollWheelEvent, Style, Window,
};

use crate::AxisExt;

/// Make a scrollable mask element to cover the parent view with the mouse wheel event listening.
///
/// When the mouse wheel is scrolled, will move the `scroll_handle` scrolling with the `axis` direction.
/// You can use this `scroll_handle` to control what you want to scroll.
/// This is only can handle once axis scrolling.
pub struct ScrollableMask {
    view_id: EntityId,
    axis: Axis,
    scroll_handle: ScrollHandle,
    debug: Option<Hsla>,
}

impl ScrollableMask {
    /// Create a new scrollable mask element.
    pub fn new(view_id: EntityId, axis: Axis, scroll_handle: &ScrollHandle) -> Self {
        Self {
            view_id,
            scroll_handle: scroll_handle.clone(),
            axis,
            debug: None,
        }
    }

    /// Enable the debug border, to show the mask bounds.
    #[allow(dead_code)]
    pub fn debug(mut self) -> Self {
        self.debug = Some(gpui::yellow());
        self
    }
}

impl IntoElement for ScrollableMask {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ScrollableMask {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        // Set the layout style relative to the table view to get same size.
        style.position = Position::Absolute;
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        // Move y to bounds height to cover the parent view.
        let cover_bounds = Bounds {
            origin: Point {
                x: bounds.origin.x,
                y: bounds.origin.y - bounds.size.height,
            },
            size: bounds.size,
        };

        window.insert_hitbox(cover_bounds, false)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let line_height = window.line_height();
        let bounds = hitbox.bounds;

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(color) = self.debug {
                window.paint_quad(PaintQuad {
                    bounds,
                    border_widths: Edges::all(px(1.0)),
                    border_color: color,
                    background: gpui::transparent_white().into(),
                    corner_radii: Corners::all(px(0.)),
                    border_style: BorderStyle::default(),
                });
            }

            window.on_mouse_event({
                let view_id = self.view_id;
                let is_horizontal = self.axis.is_horizontal();
                let scroll_handle = self.scroll_handle.clone();
                let hitbox = hitbox.clone();
                let mouse_position = window.mouse_position();
                let last_offset = scroll_handle.offset();

                move |event: &ScrollWheelEvent, phase, window, cx| {
                    if bounds.contains(&mouse_position)
                        && phase.bubble()
                        && hitbox.is_hovered(window)
                    {
                        let mut offset = scroll_handle.offset();
                        let mut delta = event.delta.pixel_delta(line_height);

                        // Limit for only one way scrolling at same time.
                        // When use MacBook touchpad we may get both x and y delta,
                        // only allows the one that more to scroll.
                        if !delta.x.is_zero() && !delta.y.is_zero() {
                            if delta.x.abs() > delta.y.abs() {
                                delta.y = px(0.);
                            } else {
                                delta.x = px(0.);
                            }
                        }

                        if is_horizontal {
                            offset.x += delta.x;
                        } else {
                            offset.y += delta.y;
                        }

                        if last_offset != offset {
                            scroll_handle.set_offset(offset);
                            cx.notify(view_id);
                            cx.stop_propagation();
                        }
                    }
                }
            });
        });
    }
}
