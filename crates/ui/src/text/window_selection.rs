use gpui::{
    App, Bounds, Context, Element, ElementId, Entity, EntityId, GlobalElementId, Hitbox,
    InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollWheelEvent, Style, WeakEntity, Window,
};

use crate::{Root, global_state::GlobalState, scroll::AutoScroll, text::TextViewState};

/// Window-level text selection state, owned by [`Root`].
///
/// All text selection (including within a single TextView) is driven by this
/// state. Selection endpoints are content-anchored when they fall inside a
/// TextView, so the selection follows the content when it scrolls.
#[derive(Default)]
pub struct WindowTextSelection {
    pub(crate) anchor: Option<SelectionEndpoint>,
    pub(crate) cursor: Option<SelectionEndpoint>,
    pub(crate) is_selecting: bool,
    pub(crate) did_hit_text: bool,
}

/// A selection endpoint, content-anchored to a TextView.
///
/// `point` is always stored in the view's content coordinates (relative to its
/// `bounds().origin` and `scroll_offset()`), even when the press landed in
/// blank space: in that case the endpoint is proxy-anchored to the nearest view
/// in document flow (see [`Root::text_selection_endpoint`]) and `inside` is
/// false. This keeps the selection following the content when an outer
/// container scrolls — a window-coordinate anchor would drift relative to the
/// content. `view` is only `None` when no view is registered at all.
#[derive(Clone)]
pub(crate) struct SelectionEndpoint {
    /// Some: the endpoint is anchored to this TextView; `point` is in that
    /// view's content coordinates (may fall outside the view when proxy-
    /// anchored from blank space). None: no view is registered; `point` is
    /// window coordinates.
    pub(crate) view: Option<WeakEntity<TextViewState>>,
    pub(crate) point: Point<Pixels>,
    /// True when the press actually hit the view's hitbox; false when the
    /// endpoint is proxy-anchored to the nearest view from blank space (so
    /// the selection follows content when an outer container scrolls).
    pub(crate) inside: bool,
    /// True when the endpoint hit an Inline text run, not just blank space in
    /// the parent TextView bounds.
    pub(crate) inside_text: bool,
}

impl SelectionEndpoint {
    /// Resolve this endpoint to window coordinates.
    ///
    /// Whether the endpoint was a true hit or proxy-anchored from blank space,
    /// `point` is in the view's content coordinates, so resolving uses the
    /// view's current `bounds().origin + scroll_offset()` (refreshed every
    /// frame in prepaint) and the endpoint follows the content as it moves.
    fn resolve(&self, cx: &App) -> Option<Point<Pixels>> {
        match &self.view {
            Some(view) => {
                let state = view.upgrade()?;
                let state = state.read(cx);
                Some(self.point + state.scroll_offset() + state.bounds().origin)
            }
            None => Some(self.point),
        }
    }

    fn view_id(&self) -> Option<EntityId> {
        self.view.as_ref().map(|view| view.entity_id())
    }
}

impl WindowTextSelection {
    /// The (anchor, cursor) points in window coordinates, `None` if the
    /// selection is empty.
    pub(crate) fn resolved_points(&self, cx: &App) -> Option<(Point<Pixels>, Point<Pixels>)> {
        if !self.did_hit_text {
            return None;
        }
        let start = self.anchor.as_ref()?.resolve(cx)?;
        let end = self.cursor.as_ref()?.resolve(cx)?;
        if start == end {
            return None;
        }
        Some((start, end))
    }

    /// If both endpoints are anchored to the same TextView, return its id.
    ///
    /// This is the single-view fast path: when a drag starts and ends anchored
    /// to one TextView, only that view participates, keeping the single-view
    /// behavior identical to before. Proxy-anchored endpoints (from blank
    /// space) count here too: a drag that begins in the blank space just above
    /// view A proxy-anchors its anchor to A, so a drag from there into A stays
    /// single-view — geometrically the selection starts at A's top, which is
    /// correct. When the two endpoints anchor to different views, all
    /// registered views participate and the per-character geometric test (in
    /// `Inline`) decides what is actually selected.
    pub(crate) fn single_view(&self) -> Option<EntityId> {
        let anchor = self.anchor.as_ref()?.view_id()?;
        let cursor = self.cursor.as_ref()?.view_id()?;
        (anchor == cursor).then_some(anchor)
    }

    fn involves(&self, view_id: EntityId) -> bool {
        self.anchor.as_ref().and_then(|e| e.view_id()) == Some(view_id)
            || self.cursor.as_ref().and_then(|e| e.view_id()) == Some(view_id)
    }
}

impl Root {
    /// Register a selectable TextView for window-level selection.
    /// Called from TextView's paint on every frame.
    pub(crate) fn register_selectable_text_view(
        state: &Entity<TextViewState>,
        hitbox: &Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(root) = window.root::<Root>().flatten() else {
            return;
        };
        let id = state.entity_id();
        let weak = state.downgrade();
        let hitbox = hitbox.clone();
        root.update(cx, |root, _| {
            // Prune dead views on each registration. This is O(N) per call (O(N²)
            // per frame across N selectable views), acceptable for typical view
            // counts; revisit if a window ever hosts hundreds of selectable views.
            root.selectable_text_views
                .retain(|_, (view, _)| view.upgrade().is_some());
            root.selectable_text_views.insert(id, (weak, hitbox));
            root.selectable_text_inlines.remove(&id);
        });
    }

    /// Register Inline text bounds for a selectable TextView.
    /// Called from Inline's paint on every frame.
    pub(crate) fn register_selectable_text_inline(
        state: &Entity<TextViewState>,
        text_bounds: Vec<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if text_bounds.is_empty() {
            return;
        }
        let Some(root) = window.root::<Root>().flatten() else {
            return;
        };
        let id = state.entity_id();
        root.update(cx, |root, _| {
            root.selectable_text_inlines
                .entry(id)
                .or_default()
                .extend(text_bounds);
        });
    }

    /// Whether there is an active text selection (window-level or view-local).
    pub(crate) fn has_text_selection(&self, cx: &App) -> bool {
        if self.text_selection.resolved_points(cx).is_some() {
            return true;
        }
        self.selectable_text_views.values().any(|(view, _)| {
            view.upgrade()
                .is_some_and(|view| view.read(cx).has_view_selection())
        })
    }

    /// Internal: collect selected text using `&self` directly, so it is safe
    /// to call while the Root entity is leased (e.g. inside Root's own action
    /// handler).
    ///
    /// Note: per-view selected text is collected from `InlineState`, which is
    /// populated during paint. The result reflects the last painted frame; a
    /// copy action racing ahead of a pending repaint may observe the previous
    /// selection state.
    pub(crate) fn window_selected_text(&self, cx: &App) -> String {
        let resolved = self.text_selection.resolved_points(cx);
        let single_view = self.text_selection.single_view();

        let mut items: Vec<(Point<Pixels>, String)> = Vec::new();
        for (id, (view, _)) in self.selectable_text_views.iter() {
            let Some(view) = view.upgrade() else { continue };
            let state = view.read(cx);
            let in_window_selection = resolved.is_some()
                && state.is_selectable()
                && single_view.map_or(true, |v| v == *id);
            if !state.has_view_selection() && !in_window_selection {
                continue;
            }
            let text = state.selected_text();
            if text.trim().is_empty() {
                continue;
            }
            items.push((state.bounds().origin, text));
        }

        items.sort_by(|a, b| {
            a.0.y
                .partial_cmp(&b.0.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.0.x
                        .partial_cmp(&b.0.x)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        items
            .into_iter()
            .map(|(_, text)| text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clear the window selection and all view-local selections.
    pub fn clear_text_selection(&mut self, cx: &mut Context<Self>) {
        let had_window_selection = self.text_selection.anchor.is_some();
        self.text_selection.anchor = None;
        self.text_selection.cursor = None;
        self.text_selection.is_selecting = false;
        self.text_selection.did_hit_text = false;
        self.selectable_text_views.retain(|_, (view, _)| {
            let Some(view) = view.upgrade() else {
                return false;
            };
            // Skip views with nothing to clear: without a window selection nor
            // a view-local selection, their inline selection state is already
            // empty, and notifying would re-render every selectable view on
            // every click.
            //
            // When `had_window_selection` is true this still clears every view,
            // even though the selection may have covered only some of them: the
            // set of views that painted a highlight is not cheaply tracked, so
            // clearing all of them is the conservative, correctness-first
            // choice.
            if had_window_selection || view.read(cx).has_view_selection() {
                view.update(cx, |state, cx| {
                    state.is_selecting = false;
                    state.clear_selection(cx);
                });
            }
            true
        });
        self.selectable_text_inlines
            .retain(|id, _| self.selectable_text_views.contains_key(id));
    }

    /// Clear the window selection when a view it is anchored to has been
    /// resized (its content coordinates are no longer valid). An active drag
    /// is not interrupted, so streaming (append-only) updates keep working.
    ///
    /// `involves` also matches a proxy-anchored endpoint (blank space anchored
    /// to this view): once the view resizes, the content the blank endpoint was
    /// pinned relative to has moved, so clearing the selection is the
    /// conservative, correctness-first choice there too.
    pub(crate) fn clear_text_selection_for_resized_view(
        &mut self,
        view_id: EntityId,
        cx: &mut Context<Self>,
    ) {
        if self.text_selection.is_selecting {
            return;
        }
        if self.text_selection.involves(view_id) {
            self.clear_text_selection(cx);
        }
    }

    pub(crate) fn start_text_selection(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let endpoint = self.text_selection_endpoint(position, window, cx);
        // Components that own their own mouse-down interaction (Input, Button,
        // etc.) set `GlobalState::suppress_text_selection` in their bubble-phase
        // handler; the controller checks that flag before calling this, so a
        // press starts a selection from any point that is not consumed by such a
        // component — including blank space inside a focusable container, which
        // GPUI's focus-on-mouse-down would otherwise mark default-prevented.
        // Only focus the view when the press actually hit it. A proxy-anchored
        // endpoint (blank space) must not steal focus from wherever it was.
        if endpoint.inside {
            if let Some(view) = endpoint.view.as_ref().and_then(|v| v.upgrade()) {
                view.update(cx, |state, cx| {
                    state.is_selecting = true;
                    state.focus_handle.focus(window, cx);
                });
            }
        }
        self.text_selection.anchor = Some(endpoint.clone());
        self.text_selection.cursor = Some(endpoint);
        self.text_selection.did_hit_text = self
            .text_selection
            .anchor
            .as_ref()
            .is_some_and(|endpoint| endpoint.inside_text);
        self.text_selection.is_selecting = true;
    }

    pub(crate) fn update_text_selection(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.text_selection.is_selecting {
            return;
        }
        // Do not update the selection while a GPUI drag-and-drop is active
        // (e.g. dragging a dock tab or a resize handle across TextViews).
        if cx.has_active_drag() {
            return;
        }

        // Compute the selection band before and after moving the cursor so the
        // notify can be limited to the views that actually changed. Order
        // matters: read the old points first, then update the cursor, then read
        // the new points.
        let old_points = self.text_selection.resolved_points(cx);
        let endpoint = self.text_selection_endpoint(position, window, cx);
        self.text_selection.did_hit_text |= endpoint.inside_text;
        self.text_selection.cursor = Some(endpoint);
        let new_points = self.text_selection.resolved_points(cx);

        // Auto-scroll the anchor view when dragging near its viewport edges,
        // same semantics as the previous per-view implementation. Only a true
        // hit anchor (inside == true) auto-scrolls; a proxy-anchored view was
        // never pressed and must not scroll.
        if let Some(view) = self
            .text_selection
            .anchor
            .as_ref()
            .filter(|e| e.inside)
            .and_then(|e| e.view.as_ref())
            .and_then(|v| v.upgrade())
        {
            view.update(cx, |state, cx| {
                if state.scrollable {
                    let delta = AutoScroll::compute_delta(position.y, state.bounds());
                    state.set_auto_scroll(delta, cx);
                }
            });
        }

        self.notify_selection_band(old_points, new_points, cx);
    }

    pub(crate) fn end_text_selection(&mut self, cx: &mut Context<Self>) {
        if !self.text_selection.is_selecting {
            return;
        }
        self.text_selection.is_selecting = false;
        if !self.text_selection.did_hit_text {
            self.text_selection.anchor = None;
            self.text_selection.cursor = None;
            return;
        }
        // Only a true hit anchor (inside == true) had `is_selecting` and
        // auto-scroll set in `start_text_selection`; a proxy-anchored view
        // has nothing to tear down.
        if let Some(view) = self
            .text_selection
            .anchor
            .as_ref()
            .filter(|e| e.inside)
            .and_then(|e| e.view.as_ref())
            .and_then(|v| v.upgrade())
        {
            view.update(cx, |state, cx| {
                state.is_selecting = false;
                state.stop_auto_scroll();
                cx.notify();
            });
        }
        self.notify_selectable_text_views(cx);
    }

    /// Resolve a window position to a selection endpoint. Uses hitbox hover
    /// testing so clipped or occluded TextViews are correctly excluded.
    ///
    /// When the position falls inside a view's hitbox, the endpoint is a true
    /// hit (`inside == true`), anchored to that view's content coordinates.
    /// When it lands in blank space, the endpoint is proxy-anchored to the
    /// nearest view in document flow (`inside == false`), so the selection
    /// still follows the content when an outer container scrolls. Only when no
    /// view is registered does it fall back to a window-coordinate endpoint.
    fn text_selection_endpoint(
        &self,
        position: Point<Pixels>,
        window: &Window,
        cx: &App,
    ) -> SelectionEndpoint {
        let mut best: Option<(WeakEntity<TextViewState>, f32)> = None;
        // `is_hovered` reflects the hitbox state as of the last prepaint frame —
        // a one-frame lag that is negligible for mouse-driven selection.
        // Smallest-area wins as a proxy for the innermost (topmost) view when
        // TextViews overlap.
        for (view, hitbox) in self.selectable_text_views.values() {
            if view.upgrade().is_none() {
                continue;
            }
            if !hitbox.is_hovered(window) {
                continue;
            }
            let area = f32::from(hitbox.bounds.size.width) * f32::from(hitbox.bounds.size.height);
            if best.as_ref().map_or(true, |(_, a)| area < *a) {
                best = Some((view.clone(), area));
            }
        }

        if let Some((view, entity)) =
            best.and_then(|(view, _)| view.upgrade().map(|entity| (view, entity)))
        {
            let state = entity.read(cx);
            let inside_text = self
                .selectable_text_inlines
                .get(&state.entity_id)
                .is_some_and(|bounds| bounds.iter().any(|bounds| bounds.contains(&position)));
            return SelectionEndpoint {
                point: position - state.bounds().origin - state.scroll_offset(),
                view: Some(view),
                inside: true,
                inside_text,
            };
        }

        // Blank space: proxy-anchor to the nearest view in document flow so the
        // endpoint moves with the content (a window-coordinate anchor would
        // drift when an outer container scrolls). Prefer the view whose top is
        // the largest value still at or above `position.y` (the nearest
        // predecessor in the flow); if the position is above every view, fall
        // back to the first view (smallest top). `point` is computed with the
        // same formula as a true hit and may fall outside the view's bounds —
        // it is a pure relative offset.
        let mut predecessor: Option<(WeakEntity<TextViewState>, Pixels)> = None;
        let mut first: Option<(WeakEntity<TextViewState>, Pixels)> = None;
        for (view, _) in self.selectable_text_views.values() {
            let Some(entity) = view.upgrade() else {
                continue;
            };
            let top = entity.read(cx).bounds().top();
            if top <= position.y {
                if predecessor.as_ref().map_or(true, |(_, t)| top > *t) {
                    predecessor = Some((view.clone(), top));
                }
            }
            if first.as_ref().map_or(true, |(_, t)| top < *t) {
                first = Some((view.clone(), top));
            }
        }

        match predecessor.or(first) {
            Some((view, _)) => {
                let entity = view.upgrade();
                // `view.upgrade()` succeeded above when the candidate was
                // chosen; if it raced to None, fall back to a window endpoint.
                match entity {
                    Some(entity) => {
                        let state = entity.read(cx);
                        SelectionEndpoint {
                            point: position - state.bounds().origin - state.scroll_offset(),
                            view: Some(view),
                            inside: false,
                            inside_text: false,
                        }
                    }
                    None => SelectionEndpoint {
                        view: None,
                        point: position,
                        inside: false,
                        inside_text: false,
                    },
                }
            }
            None => SelectionEndpoint {
                view: None,
                point: position,
                inside: false,
                inside_text: false,
            },
        }
    }

    fn notify_selectable_text_views(&mut self, cx: &mut Context<Self>) {
        self.selectable_text_views.retain(|_, (view, _)| {
            let Some(view) = view.upgrade() else {
                return false;
            };
            view.update(cx, |_, cx| cx.notify());
            true
        });
    }

    /// Notify the views affected by the current selection update. For a
    /// single-view selection only the anchor view re-renders; for a
    /// cross-view selection only views whose bounds intersect the vertical
    /// band covered by the old and new selection participate, plus everything
    /// that may need to clear a previously painted highlight.
    fn notify_selection_band(
        &mut self,
        old_points: Option<(Point<Pixels>, Point<Pixels>)>,
        new_points: Option<(Point<Pixels>, Point<Pixels>)>,
        cx: &mut Context<Self>,
    ) {
        // Single-view fast path: when the selection lives entirely in the
        // anchor view, only it can paint a highlight, so only it needs to
        // re-render.
        //
        // This is only safe when there is no *previous* band that may have
        // painted a highlight on some other view: a drag that crossed into a
        // second view and then came back inside the anchor view leaves the new
        // band single-view, but the old band still covers the view that must
        // clear its now-stale highlight. In that case fall through to the
        // general band path (band = old ∪ new), which always covers the anchor
        // view too.
        if old_points.is_none() {
            if let Some(id) = self.text_selection.single_view() {
                if let Some((view, _)) = self.selectable_text_views.get(&id) {
                    if let Some(view) = view.upgrade() {
                        view.update(cx, |_, cx| cx.notify());
                    }
                }
                return;
            }
        }

        // Merge the old and new selection bands. The old band covers views that
        // may need to clear a previously painted highlight; the new band covers
        // views that may need to paint one. If both are empty there is nothing
        // to update.
        let band = |points: Option<(Point<Pixels>, Point<Pixels>)>| {
            points.map(|(a, b)| {
                let (lo, hi) = if a.y <= b.y { (a.y, b.y) } else { (b.y, a.y) };
                (lo, hi)
            })
        };
        let (band_min, band_max) = match (band(old_points), band(new_points)) {
            (Some((lo_a, hi_a)), Some((lo_b, hi_b))) => (lo_a.min(lo_b), hi_a.max(hi_b)),
            (Some(b), None) | (None, Some(b)) => b,
            (None, None) => return,
        };

        self.selectable_text_views.retain(|_, (view, _)| {
            let Some(view) = view.upgrade() else {
                return false;
            };
            let bounds = view.read(cx).bounds();
            if bounds.top() <= band_max && bounds.bottom() >= band_min {
                view.update(cx, |_, cx| cx.notify());
            }
            true
        });
    }
}

/// A zero-size element that drives window-level text selection.
///
/// Must be the FIRST child of Root's container div: bubble-phase mouse
/// listeners fire in reverse registration order, so registering earliest makes
/// the controller run AFTER interactive components (which may stop
/// propagation or prevent default).
///
/// Note: `window.on_mouse_event` handlers are window-global (not scoped to
/// any hitbox); the phase check and the `GlobalState::suppress_text_selection`
/// flag are the only guards. The flag is reset in the capture phase of every
/// left mouse down and set in the bubble phase by components that own their own
/// press/drag interaction (Button, Input, etc.). Because bubble-phase listeners
/// fire in reverse registration order and this controller registers earliest,
/// it observes the flag after those components have set it, so presses consumed
/// by them are excluded while presses on blank space (even inside a focusable
/// container) still start a selection.
pub(crate) struct TextSelectionController;

impl IntoElement for TextSelectionController {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextSelectionController {
    type RequestLayoutState = ();
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
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if event.button != MouseButton::Left {
                return;
            }
            if phase.capture() {
                // Reset the suppression flag at the start of every press, then
                // clear the previous selection (browser behavior), even when an
                // interactive component consumes the event in the bubble phase.
                GlobalState::global_mut(cx).suppress_text_selection = false;
                Root::update(window, cx, |root, _, cx| root.clear_text_selection(cx));
            } else if event.click_count == 1 {
                // Reaching bubble phase means no component stopped propagation.
                // Components that own their own press (Button, Input, etc.) set
                // `suppress_text_selection` in their bubble handler; if set, the
                // press is theirs and must not start a window selection.
                if GlobalState::global(cx).suppress_text_selection {
                    return;
                }
                Root::update(window, cx, |root, window, cx| {
                    root.start_text_selection(event.position, window, cx);
                });
            }
        });

        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            Root::update(window, cx, |root, window, cx| {
                root.update_text_selection(event.position, window, cx);
            });
        });

        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            Root::update(window, cx, |root, _, cx| root.end_text_selection(cx));
        });

        window.on_mouse_event(move |_: &ScrollWheelEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            // While drag-selecting, a wheel scroll moves content under the
            // stationary cursor; re-resolve the cursor endpoint at the current
            // mouse position so the selection keeps extending to the pointer
            // (browser behavior). `update_text_selection` is a no-op unless a
            // selection drag is active, so the idle cost is negligible.
            //
            // Bounds are refreshed in the next frame's prepaint, so a single
            // wheel event may resolve one frame stale; continuous scrolling
            // converges, so this is left unhandled.
            let position = window.mouse_position();
            Root::update(window, cx, |root, window, cx| {
                root.update_text_selection(position, window, cx);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::global_state::GlobalState;
    use crate::{
        Root,
        text::{TextView, TextViewState},
    };
    use gpui::{
        AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
        Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement as _, Render,
        Styled as _, TestAppContext, VisualTestContext, Window, div, point, px,
    };
    use std::cell::Cell;
    use std::rc::Rc;

    struct ChatTestView {
        focus_handle: FocusHandle,
        first: Entity<TextViewState>,
        second: Entity<TextViewState>,
        second_selectable: bool,
        /// Top padding above the views. Bumping it shifts the whole content
        /// down, which is the layout-level equivalent of an outer container
        /// scrolling (see `selection_follows_content_when_layout_shifts`).
        top_offset: gpui::Pixels,
        /// Blank gap between the two views, used to anchor a selection in blank
        /// space (the proxy-anchored endpoint path).
        mid_gap: gpui::Pixels,
    }

    impl ChatTestView {
        fn new(second_selectable: bool, cx: &mut Context<Self>) -> Self {
            Self {
                focus_handle: cx.focus_handle(),
                first: cx.new(|cx| TextViewState::markdown("Hello world", cx)),
                second: cx.new(|cx| TextViewState::markdown("Second message", cx)),
                second_selectable,
                top_offset: px(10.),
                mid_gap: px(0.),
            }
        }
    }

    impl Render for ChatTestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            // `track_focus` makes the root a focusable container, so GPUI's
            // focus-on-mouse-down marks every press inside it default-prevented.
            // Selection must still start from blank space here (regression
            // guard for `drag_from_blank_space_selects_views_below`), which the
            // `suppress_text_selection` mechanism guarantees because blank-space
            // presses never set that flag.
            div()
                .track_focus(&self.focus_handle)
                .size_full()
                .pt(self.top_offset)
                .child(
                    div()
                        .h(px(40.))
                        .child(TextView::new(&self.first).selectable(true)),
                )
                // A blank gap between the two views. It is not over any
                // TextView hitbox, so a press here exercises the blank-space
                // (proxy-anchored) endpoint path.
                .child(div().h(self.mid_gap))
                .child(
                    div()
                        .h(px(40.))
                        .child(TextView::new(&self.second).selectable(self.second_selectable)),
                )
                // A 20px region below the views that owns its press the way
                // Input/Button do: its bubble-phase handler sets the suppress
                // flag, so a press starting here must not start a selection.
                .child(
                    div()
                        .h(px(20.))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            GlobalState::suppress_text_selection(cx);
                        }),
                )
        }
    }

    fn setup(
        second_selectable: bool,
        cx: &mut TestAppContext,
    ) -> (Entity<ChatTestView>, &mut VisualTestContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let chat = cx.new(|cx| ChatTestView::new(second_selectable, cx));
            Root::new(chat, window, cx)
        });
        let chat = root.read_with(cx, |root, _| {
            root.view().clone().downcast::<ChatTestView>().unwrap()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        (chat, cx)
    }

    fn drag(
        cx: &mut VisualTestContext,
        from: gpui::Point<gpui::Pixels>,
        to: gpui::Point<gpui::Pixels>,
    ) {
        drag_through(cx, &[from, to]);
    }

    fn drag_through(cx: &mut VisualTestContext, points: &[gpui::Point<gpui::Pixels>]) {
        assert!(points.len() >= 2);
        let from = points[0];
        let to = *points.last().unwrap();

        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        for point in &points[1..] {
            cx.simulate_mouse_move(*point, Some(MouseButton::Left), Modifiers::default());
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
        }

        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    fn window_selected_text(cx: &mut VisualTestContext) -> String {
        use crate::WindowExt as _;
        cx.update(|window, cx| window.selected_text(cx))
    }

    #[gpui::test]
    fn cross_view_drag_merges_text_top_to_bottom(cx: &mut TestAppContext) {
        let (_, cx) = setup(true, cx);

        // From the very start of the first view down into the second view.
        drag(cx, point(px(0.), px(15.)), point(px(300.), px(70.)));

        let text = window_selected_text(cx);
        let first = text.find("Hello world").expect("first view text missing");
        let second = text
            .find("Second message")
            .expect("second view text missing");
        assert!(first < second, "wrong order: {text:?}");
        assert!(text.contains('\n'), "expected newline separator: {text:?}");
    }

    #[gpui::test]
    fn drag_from_blank_space_selects_views_below(cx: &mut TestAppContext) {
        let (_, cx) = setup(true, cx);

        // Start in the blank padding above the first view, enter the second
        // view's rendered text, then drag past its end.
        drag_through(
            cx,
            &[
                point(px(5.), px(2.)),
                point(px(20.), px(70.)),
                point(px(300.), px(70.)),
            ],
        );

        let text = window_selected_text(cx);
        assert!(text.contains("Hello world"), "got: {text:?}");
        assert!(text.contains("Second message"), "got: {text:?}");
    }

    #[gpui::test]
    fn drag_entirely_in_blank_gap_selects_nothing(cx: &mut TestAppContext) {
        let (chat, cx) = setup(true, cx);

        // Layout: first [10,50], gap [50,110], second [110,150].
        chat.update(cx, |chat, cx| {
            chat.mid_gap = px(60.);
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        // Drag only inside the gap. The selection never enters either TextView.
        drag(cx, point(px(5.), px(70.)), point(px(300.), px(90.)));

        let text = window_selected_text(cx);
        assert_eq!(text, "", "blank-only drag selected text: {text:?}");
    }

    #[gpui::test]
    fn drag_entirely_in_right_gutter_selects_nothing(cx: &mut TestAppContext) {
        let (_, cx) = setup(true, cx);

        // x=300 is far to the right of the rendered text. Dragging vertically
        // through only that blank gutter must not select nearby TextViews.
        drag(cx, point(px(300.), px(2.)), point(px(300.), px(70.)));

        let text = window_selected_text(cx);
        assert_eq!(text, "", "right-gutter drag selected text: {text:?}");
    }

    #[gpui::test]
    fn selection_follows_content_when_layout_shifts(cx: &mut TestAppContext) {
        let (chat, cx) = setup(true, cx);

        // Open a blank gap between the two views so we can anchor a selection
        // in blank space that sits *below* the first view's text and *above*
        // the second. Layout: first [10,50], gap [50,110], second [110,150].
        chat.update(cx, |chat, cx| {
            chat.mid_gap = px(60.);
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        // Anchor in the gap (blank space) and drag down-right into the second
        // view, ending past the end of its text so the whole line is selected.
        // The anchor sits below "Hello world", so only the second view is
        // selected.
        drag_through(
            cx,
            &[
                point(px(0.), px(80.)),
                point(px(20.), px(120.)),
                point(px(300.), px(120.)),
            ],
        );
        let before = window_selected_text(cx);
        assert!(
            before.contains("Second message") && !before.contains("Hello world"),
            "expected only the second view selected, got: {before:?}"
        );

        // Shift the whole content down by 80px — the equivalent of an outer
        // container scrolling. A window-anchored blank endpoint stays at window
        // y=80, which the first view now covers (first moves to ~[90,130]), so
        // the selection drifts to also grab "Hello world". A proxy-anchored
        // endpoint moves with the content and the selection stays stable.
        chat.update(cx, |chat, cx| {
            chat.top_offset = px(90.);
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let after = window_selected_text(cx);
        assert_eq!(before, after, "selection drifted after layout shift");
    }

    #[gpui::test]
    fn suppressed_mouse_down_does_not_start_selection(cx: &mut TestAppContext) {
        let (_, cx) = setup(true, cx);

        // The suppress region sits below the two views (root pt=10, two 40px
        // view rows -> y in [90, 110)). Pressing inside it makes its bubble
        // handler set the suppress flag, so dragging up across both views must
        // not produce any window selection.
        drag(cx, point(px(20.), px(100.)), point(px(20.), px(15.)));

        let text = window_selected_text(cx);
        assert!(text.is_empty(), "expected no selection, got: {text:?}");
    }

    #[gpui::test]
    fn non_selectable_view_is_excluded(cx: &mut TestAppContext) {
        let (_, cx) = setup(false, cx);

        drag_through(
            cx,
            &[
                point(px(5.), px(2.)),
                point(px(20.), px(15.)),
                point(px(300.), px(15.)),
            ],
        );

        let text = window_selected_text(cx);
        assert!(text.contains("Hello world"), "got: {text:?}");
        assert!(!text.contains("Second message"), "got: {text:?}");
    }

    #[gpui::test]
    fn drag_within_single_view_excludes_others(cx: &mut TestAppContext) {
        let (_, cx) = setup(true, cx);

        // Entirely inside the first view.
        drag(cx, point(px(5.), px(15.)), point(px(60.), px(15.)));

        let text = window_selected_text(cx);
        assert!(!text.contains("Second message"), "got: {text:?}");
        assert!(!text.trim().is_empty(), "expected some selection");
    }

    #[gpui::test]
    fn mouse_down_clears_previous_selection(cx: &mut TestAppContext) {
        let (_, cx) = setup(true, cx);

        drag(cx, point(px(5.), px(15.)), point(px(300.), px(70.)));
        assert!(!window_selected_text(cx).is_empty());

        // A plain click clears the selection.
        cx.simulate_click(point(px(300.), px(100.)), Modifiers::default());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        assert_eq!(window_selected_text(cx), "");
    }

    #[gpui::test]
    fn double_click_selects_word_under_root(cx: &mut TestAppContext) {
        let (_, cx) = setup(true, cx);

        // Double-click inside the first view: must trigger the per-view word
        // selection (Inline), not a window-level drag selection.
        let position = point(px(10.), px(15.));
        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let text = window_selected_text(cx);
        assert_eq!(text.trim(), "Hello", "expected word selection: {text:?}");
        assert!(!text.contains("Second message"), "got: {text:?}");
    }

    #[gpui::test]
    fn drag_back_into_anchor_view_clears_other_views(cx: &mut TestAppContext) {
        let (chat, cx) = setup(true, cx);
        let second = chat.read_with(cx, |chat, _| chat.second.clone());

        // Drag from view A down into view B: this is a cross-view selection, so
        // B paints a highlight and `selected_text` reports it.
        cx.simulate_mouse_down(
            point(px(0.), px(15.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_mouse_move(
            point(px(300.), px(70.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let text = second.read_with(cx, |state, _| state.selected_text());
        assert!(
            text.contains("Second message"),
            "precondition: B should be selected, got {text:?}"
        );

        // Observe B's re-render requests. A view only drops a stale highlight
        // when it is notified and repaints; this asserts the controller does
        // notify B, independently of whether the test harness happens to
        // repaint B for unrelated reasons.
        let b_notified = Rc::new(Cell::new(false));
        let _subscription = cx.update({
            let b_notified = b_notified.clone();
            let second = second.clone();
            move |_, cx| cx.observe(&second, move |_, _| b_notified.set(true))
        });
        b_notified.set(false);

        // Drag back up inside view A. The drag now lives entirely in A, so
        // `single_view` is Some(A) and the fast path runs. It must still notify
        // B (whose old band crossed B) so B can clear its now-stale highlight.
        //
        // We check this on the in-drag frame, not after mouse-up:
        // `end_text_selection` notifies every selectable view, which would
        // notify B for an unrelated reason and mask the bug.
        cx.simulate_mouse_move(
            point(px(60.), px(15.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.run_until_parked();

        assert!(
            b_notified.get(),
            "view B was not notified when the drag returned to the anchor view, \
             so its stale highlight would never be repainted away",
        );
    }
}
