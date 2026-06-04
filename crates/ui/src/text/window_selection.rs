use gpui::{
    App, Bounds, Context, Element, ElementId, Entity, EntityId, GlobalElementId, Hitbox,
    InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Style, WeakEntity, Window,
};

use crate::{Root, scroll::AutoScroll, text::TextViewState};

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
}

/// A selection endpoint, content-anchored if inside a TextView.
#[derive(Clone)]
pub(crate) struct SelectionEndpoint {
    /// Some: the endpoint is inside this TextView; `point` is in that view's
    /// content coordinates. None: blank space; `point` is window coordinates.
    pub(crate) view: Option<WeakEntity<TextViewState>>,
    pub(crate) point: Point<Pixels>,
}

impl SelectionEndpoint {
    /// Resolve this endpoint to window coordinates.
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
        let start = self.anchor.as_ref()?.resolve(cx)?;
        let end = self.cursor.as_ref()?.resolve(cx)?;
        if start == end {
            return None;
        }
        Some((start, end))
    }

    /// If both endpoints are anchored inside the same TextView, return its id.
    ///
    /// This is the single-view fast path: when a drag starts and ends inside
    /// one TextView, only that view participates, keeping the single-view
    /// behavior identical to before. When either endpoint is in blank space,
    /// all registered views participate and the per-character geometric test
    /// (in `Inline`) decides what is actually selected.
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
        });
    }

    /// Return the merged selected text across all selectable TextViews in this
    /// window, ordered by vertical position (top to bottom), joined with `\n`.
    pub fn selected_text(window: &Window, cx: &App) -> String {
        let Some(root) = window.root::<Root>().flatten() else {
            return String::new();
        };
        root.read(cx).window_selected_text(cx)
    }

    /// Internal: collect selected text using `&self` directly, so it is safe
    /// to call while the Root entity is leased (e.g. inside Root's own action
    /// handler).
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
        self.text_selection.anchor = None;
        self.text_selection.cursor = None;
        self.text_selection.is_selecting = false;
        self.selectable_text_views.retain(|_, (view, _)| {
            let Some(view) = view.upgrade() else {
                return false;
            };
            view.update(cx, |state, cx| {
                state.is_selecting = false;
                state.clear_selection(cx);
            });
            true
        });
    }

    /// Clear the window selection when a view it is anchored to has been
    /// resized (its content coordinates are no longer valid). An active drag
    /// is not interrupted, so streaming (append-only) updates keep working.
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
        if let Some(view) = endpoint.view.as_ref().and_then(|v| v.upgrade()) {
            view.update(cx, |state, cx| {
                state.is_selecting = true;
                state.focus_handle.focus(window, cx);
            });
        }
        self.text_selection.anchor = Some(endpoint.clone());
        self.text_selection.cursor = Some(endpoint);
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
        self.text_selection.cursor = Some(self.text_selection_endpoint(position, window, cx));

        // Auto-scroll the anchor view when dragging near its viewport edges,
        // same semantics as the previous per-view implementation.
        if let Some(view) = self
            .text_selection
            .anchor
            .as_ref()
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

        self.notify_selectable_text_views(cx);
    }

    pub(crate) fn end_text_selection(&mut self, cx: &mut Context<Self>) {
        if !self.text_selection.is_selecting {
            return;
        }
        self.text_selection.is_selecting = false;
        if let Some(view) = self
            .text_selection
            .anchor
            .as_ref()
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

        match best.and_then(|(view, _)| view.upgrade().map(|entity| (view, entity))) {
            Some((view, entity)) => {
                let state = entity.read(cx);
                SelectionEndpoint {
                    point: position - state.bounds().origin - state.scroll_offset(),
                    view: Some(view),
                }
            }
            None => SelectionEndpoint {
                view: None,
                point: position,
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
}

/// A zero-size element that drives window-level text selection.
///
/// Must be the FIRST child of Root's container div: bubble-phase mouse
/// listeners fire in reverse registration order, so registering earliest makes
/// the controller run AFTER interactive components (which may stop
/// propagation or prevent default).
///
/// Note: `window.on_mouse_event` handlers are window-global (not scoped to
/// any hitbox); the phase and `default_prevented` checks are the only guards.
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
                // Any left press clears the previous selection (browser
                // behavior), even when an interactive component consumes the
                // event in the bubble phase.
                Root::update(window, cx, |root, _, cx| root.clear_text_selection(cx));
            } else if event.click_count == 1 && !window.default_prevented() {
                // Reaching bubble phase means no component stopped
                // propagation; default_prevented covers Button-like
                // components.
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
    }
}
