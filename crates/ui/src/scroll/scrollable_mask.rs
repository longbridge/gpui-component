use gpui::{
    App, Axis, BorderStyle, Bounds, ContentMask, Edges, Element, ElementId, GlobalElementId,
    Hitbox, Hsla, InteractiveElement as _, IntoElement, IsZero as _, LayoutId, PaintQuad,
    ParentElement as _, Point, Position, ScrollHandle, ScrollWheelEvent,
    StatefulInteractiveElement as _, Style, StyleRefinement, Styled as _, Window, div, px,
    relative,
};
use gpui::{Corners, Pixels};

use crate::{AxisExt, StyledExt as _};

/// A horizontal scroll viewport that only consumes horizontal wheel deltas.
///
/// GPUI's native `overflow_x_scroll` maps vertical wheel input onto horizontal
/// scrolling when there is no vertical overflow. This wrapper keeps the visual
/// clipping and scroll offset, while delegating wheel input to [`ScrollableMask`]
/// so vertical wheel events can continue bubbling to the parent scroller.
pub(crate) fn horizontal_scroll_area(
    id: impl Into<ElementId>,
    scroll_handle: &ScrollHandle,
    style: &StyleRefinement,
    child: impl IntoElement,
) -> impl IntoElement {
    // The mask must be a sibling of the scrolled element (like in Table), not
    // a child of it: children are prepainted with the scroll offset applied,
    // which would slide the mask away from the viewport as the content
    // scrolls, leaving the uncovered part to the parent scroller.
    div()
        .w_full()
        .relative()
        .child(
            div()
                .id(id)
                .w_full()
                .refine_style(style)
                .overflow_hidden()
                .track_scroll(scroll_handle)
                .child(child),
        )
        .child(ScrollableMask::new(Axis::Horizontal, scroll_handle))
}

/// Make a scrollable mask element to cover the parent view with the mouse wheel event listening.
///
/// When the mouse wheel is scrolled, will move the `scroll_handle` scrolling with the `axis` direction.
/// You can use this `scroll_handle` to control what you want to scroll.
/// This is only can handle once axis scrolling.
///
/// Axis-dominant wheel events are consumed in the capture phase, so the mask
/// wins over ancestor scrollers (e.g. `gpui::list`) that register their
/// listeners after their children; events dominated by the other axis keep
/// propagating. The mask stays inert while occluded.
pub struct ScrollableMask {
    axis: Axis,
    scroll_handle: ScrollHandle,
    debug: Option<Hsla>,
}

impl ScrollableMask {
    /// Create a new scrollable mask element.
    pub fn new(axis: Axis, scroll_handle: &ScrollHandle) -> Self {
        Self {
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

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
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
        _: Option<&gpui::InspectorElementId>,
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

        window.insert_hitbox(cover_bounds, gpui::HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let is_horizontal = self.axis.is_horizontal();
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
                let view_id = window.current_view();
                let scroll_handle = self.scroll_handle.clone();
                let hitbox_id = hitbox.id;

                move |event: &ScrollWheelEvent, phase, window, cx| {
                    // Handle in the capture phase: ancestor scrollers such as
                    // `gpui::list` register their wheel listeners after their
                    // children paint, so in the bubble phase (reverse
                    // registration order) they run first and would consume the
                    // vertical component of a trackpad swipe before this mask
                    // could stop the propagation.
                    //
                    // `should_handle_scroll` (instead of a raw bounds check)
                    // keeps the mask inert when it is occluded, e.g. below an
                    // open dialog or context menu.
                    if !(phase.capture() && hitbox_id.should_handle_scroll(window)) {
                        return;
                    }

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

                    // NOTE: `set_offset` does not clamp (clamping happens in
                    // the div's prepaint), so any non-zero axis-dominant delta
                    // passes this guard — even at the scroll edge the event is
                    // consumed rather than turned into a parent scroll.
                    if offset != scroll_handle.offset() {
                        scroll_handle.set_offset(offset);
                        cx.notify(view_id);
                        cx.stop_propagation();
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, IntoElement, ListAlignment, ListState, Render, ScrollDelta, ScrollWheelEvent,
        TestAppContext, VisualTestContext, Window, div, list, point, px,
    };

    struct HorizontalScrollAreaTest {
        scroll_handle: ScrollHandle,
    }

    impl Render for HorizontalScrollAreaTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(100.)).h(px(40.)).child(horizontal_scroll_area(
                "horizontal-scroll-area",
                &self.scroll_handle,
                &Default::default(),
                div().w(px(300.)).h(px(40.)),
            ))
        }
    }

    #[gpui::test]
    fn horizontal_scroll_area_ignores_vertical_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            move |_, _| HorizontalScrollAreaTest {
                scroll_handle: scroll_handle.clone(),
            }
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        assert_eq!(scroll_handle.offset().x, px(0.));
    }

    /// Reproduces the markdown table case: the scroll area lives inside a
    /// `gpui::list` item. The list registers its wheel listener after its
    /// items paint, so in the bubble phase (reverse registration order) the
    /// list runs first and consumes `delta.y` of every trackpad swipe.
    struct ListWithHorizontalAreaTest {
        scroll_handle: ScrollHandle,
        list_state: ListState,
        occluded: bool,
    }

    impl Render for ListWithHorizontalAreaTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let scroll_handle = self.scroll_handle.clone();
            let mut root = div().w(px(100.)).h(px(100.)).child(
                list(self.list_state.clone(), move |ix, _, _| {
                    if ix == 0 {
                        horizontal_scroll_area(
                            "horizontal-scroll-area",
                            &scroll_handle,
                            &Default::default(),
                            div().w(px(300.)).h(px(40.)),
                        )
                        .into_any_element()
                    } else {
                        div().w(px(100.)).h(px(40.)).into_any_element()
                    }
                })
                .w_full()
                .h_full(),
            );
            if self.occluded {
                // An overlay above the list, like an open dialog or menu.
                root = root.child(div().absolute().top_0().left_0().size_full().occlude());
            }
            root
        }
    }

    fn setup_list_test<'a>(
        cx: &'a mut TestAppContext,
        scroll_handle: &ScrollHandle,
        list_state: &ListState,
        occluded: bool,
    ) -> &'a mut VisualTestContext {
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            let list_state = list_state.clone();
            move |_, _| ListWithHorizontalAreaTest {
                scroll_handle: scroll_handle.clone(),
                list_state: list_state.clone(),
                occluded,
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        cx
    }

    #[gpui::test]
    fn horizontal_scroll_area_in_list_keeps_horizontal_dominant_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, false);

        // A trackpad swipe is rarely axis-pure: horizontal dominant with a
        // small vertical component.
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });

        // The area consumes the horizontal delta...
        assert_eq!(scroll_handle.offset().x, px(-40.));
        // ...and the outer list must not scroll vertically.
        let scroll_top = list_state.logical_scroll_top();
        assert_eq!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }

    #[gpui::test]
    fn horizontal_scroll_area_in_list_bubbles_vertical_dominant_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, false);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-10.), px(-40.))),
            ..Default::default()
        });

        // Vertical dominant: the list scrolls, the area does not.
        assert_eq!(scroll_handle.offset().x, px(0.));
        let scroll_top = list_state.logical_scroll_top();
        assert_ne!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }

    #[gpui::test]
    fn horizontal_scroll_area_covers_viewport_after_scrolled(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, false);

        // Scroll the area to the middle, then repaint.
        scroll_handle.set_offset(point(px(-150.), px(0.)));
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Swipe over the right side of the viewport. The mask must still
        // cover it — it must not slide away with the scrolled content.
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(90.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });

        assert_eq!(scroll_handle.offset().x, px(-190.));
        let scroll_top = list_state.logical_scroll_top();
        assert_eq!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }

    #[gpui::test]
    fn horizontal_scroll_area_ignores_wheel_when_occluded(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, true);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });

        // An overlay (dialog, context menu) occludes the area: the area
        // must not scroll underneath it.
        assert_eq!(scroll_handle.offset().x, px(0.));
    }

    #[gpui::test]
    fn horizontal_scroll_area_uses_horizontal_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            move |_, _| HorizontalScrollAreaTest {
                scroll_handle: scroll_handle.clone(),
            }
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(0.))),
            ..Default::default()
        });

        assert_eq!(scroll_handle.offset().x, px(-40.));
    }
}
