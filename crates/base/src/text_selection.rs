use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use gpui::{
    App, AppContext as _, Bounds, Element, ElementId, Entity, EntityId, Global, GlobalElementId,
    Hitbox, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollWheelEvent, Style, WeakEntity, Window,
};

use crate::{AutoScroll, GlobalState};

/// An opaque selection layer identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SelectionScopeId(u64);

impl SelectionScopeId {
    /// Creates a stable scope identifier for a selection layer.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// The window-coordinate endpoints of a selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionSnapshot {
    pub anchor: Point<Pixels>,
    pub cursor: Point<Pixels>,
    pub is_selecting: bool,
}

/// Per-frame geometry reported by a selectable region.
pub struct SelectionRegionFrame {
    pub hitbox: Hitbox,
    pub bounds: Bounds<Pixels>,
    pub scroll_offset: Point<Pixels>,
    pub scope: SelectionScopeId,
    pub document_order: u64,
    pub text_bounds: Vec<Bounds<Pixels>>,
}

type RegionSelectionCallback = Rc<dyn Fn(Option<SelectionSnapshot>, &mut App)>;
type RegionAutoScrollCallback = Rc<dyn Fn(Option<Pixels>, &mut App)>;
type RegionVoidCallback = Rc<dyn Fn(&mut App)>;
type RegionCopyCallback = Rc<dyn Fn(&App) -> String>;

/// Renderer-owned state associated with a selectable region.
///
/// The base host owns only generic geometry and gesture state. Renderers store
/// their projection, highlights, and optional focus/scroll hooks here.
pub struct TextSelectionRegionState {
    selected_text: String,
    local_selection: bool,
    snapshot: Option<SelectionSnapshot>,
    on_selection: Option<RegionSelectionCallback>,
    on_auto_scroll: Option<RegionAutoScrollCallback>,
    on_focus: Option<RegionVoidCallback>,
    on_clear: Option<RegionVoidCallback>,
    copy: Option<RegionCopyCallback>,
}

impl TextSelectionRegionState {
    fn new(selected_text: impl Into<String>) -> Self {
        Self {
            selected_text: selected_text.into(),
            local_selection: false,
            snapshot: None,
            on_selection: None,
            on_auto_scroll: None,
            on_focus: None,
            on_clear: None,
            copy: None,
        }
    }

    /// The current geometry selection snapshot for this region.
    pub fn snapshot(&self) -> Option<SelectionSnapshot> {
        self.snapshot
    }

    /// Sets the text copied by this region when it participates in selection.
    pub fn set_selected_text(&mut self, text: impl Into<String>) {
        self.selected_text = text.into();
    }

    /// Marks renderer-local selection (for example select-all) as active.
    pub fn set_local_selection(&mut self, active: bool) {
        self.local_selection = active;
    }

    /// Installs the callback which updates renderer highlights from host state.
    pub fn on_selection(
        &mut self,
        callback: impl Fn(Option<SelectionSnapshot>, &mut App) + 'static,
    ) {
        self.on_selection = Some(Rc::new(callback));
    }

    /// Installs the callback which receives drag auto-scroll commands.
    pub fn on_auto_scroll(&mut self, callback: impl Fn(Option<Pixels>, &mut App) + 'static) {
        self.on_auto_scroll = Some(Rc::new(callback));
    }

    /// Installs the callback which focuses the region when a drag begins in it.
    pub fn on_focus(&mut self, callback: impl Fn(&mut App) + 'static) {
        self.on_focus = Some(Rc::new(callback));
    }

    /// Installs renderer-local cleanup invoked by [`TextSelectionHost::clear`].
    pub fn on_clear(&mut self, callback: impl Fn(&mut App) + 'static) {
        self.on_clear = Some(Rc::new(callback));
    }

    /// Installs a renderer-specific copy projection.
    pub fn copy_with(&mut self, callback: impl Fn(&App) -> String + 'static) {
        self.copy = Some(Rc::new(callback));
    }

    fn set_snapshot(&mut self, snapshot: Option<SelectionSnapshot>, cx: &mut App) {
        self.snapshot = snapshot;
        if let Some(callback) = &self.on_selection {
            callback(snapshot, cx);
        }
    }

    fn clear(&mut self, cx: &mut App) {
        self.snapshot = None;
        self.local_selection = false;
        if let Some(callback) = &self.on_clear {
            callback(cx);
        }
        if let Some(callback) = &self.on_selection {
            callback(None, cx);
        }
    }

    fn set_auto_scroll(&self, delta: Option<Pixels>, cx: &mut App) {
        if let Some(callback) = &self.on_auto_scroll {
            callback(delta, cx);
        }
    }

    fn focus(&self, cx: &mut App) {
        if let Some(callback) = &self.on_focus {
            callback(cx);
        }
    }

    fn copied_text(&self, cx: &App) -> String {
        self.copy
            .as_ref()
            .map(|callback| callback(cx))
            .unwrap_or_else(|| self.selected_text.clone())
    }
}

/// A renderer-neutral selectable region backed by GPUI entity state.
#[derive(Clone)]
pub struct TextSelectionRegion(Entity<TextSelectionRegionState>);

impl TextSelectionRegion {
    /// Creates a selectable region with a default copy string.
    pub fn new(selected_text: impl Into<String>, cx: &mut App) -> Self {
        Self(cx.new(|_| TextSelectionRegionState::new(selected_text)))
    }

    /// Returns the region state entity for adapter configuration.
    pub fn state(&self) -> Entity<TextSelectionRegionState> {
        self.0.clone()
    }

    fn entity_id(&self) -> EntityId {
        self.0.entity_id()
    }

    fn downgrade(&self) -> WeakEntity<TextSelectionRegionState> {
        self.0.downgrade()
    }
}

#[derive(Clone)]
struct RegionRegistration {
    region: WeakEntity<TextSelectionRegionState>,
    frame: Rc<SelectionRegionFrame>,
}

#[derive(Clone)]
struct SelectionEndpoint {
    region: Option<WeakEntity<TextSelectionRegionState>>,
    point: Point<Pixels>,
    inside: bool,
    inside_text: bool,
}

impl SelectionEndpoint {
    fn resolve(&self, regions: &HashMap<EntityId, RegionRegistration>) -> Option<Point<Pixels>> {
        let region = self.region.as_ref()?;
        let frame = regions.get(&region.entity_id())?;
        region.upgrade()?;
        Some(self.point + frame.frame.scroll_offset + frame.frame.bounds.origin)
    }

    fn region_id(&self) -> Option<EntityId> {
        self.region.as_ref().map(|region| region.entity_id())
    }
}

/// Window-local generic text-selection state.
pub struct TextSelectionHost {
    regions: HashMap<EntityId, RegionRegistration>,
    active_scope: SelectionScopeId,
    anchor: Option<SelectionEndpoint>,
    cursor: Option<SelectionEndpoint>,
    pending_extension_anchor: Option<SelectionEndpoint>,
    is_selecting: bool,
    did_hit_text: bool,
}

impl Default for TextSelectionHost {
    fn default() -> Self {
        Self {
            regions: HashMap::new(),
            active_scope: SelectionScopeId::default(),
            anchor: None,
            cursor: None,
            pending_extension_anchor: None,
            is_selecting: false,
            did_hit_text: false,
        }
    }
}

impl TextSelectionHost {
    /// Returns (and lazily installs) the host for `window`.
    pub fn for_window(window: &Window, cx: &mut App) -> Entity<Self> {
        if !cx.has_global::<TextSelectionHosts>() {
            cx.set_global(TextSelectionHosts::default());
        }
        let live_windows = cx
            .windows()
            .into_iter()
            .map(|handle| handle.window_id())
            .collect::<HashSet<_>>();
        cx.global_mut::<TextSelectionHosts>()
            .0
            .retain(|window_id, _| live_windows.contains(window_id));
        let window_id = window.window_handle().window_id();
        if let Some(host) = cx.global::<TextSelectionHosts>().0.get(&window_id) {
            return host.clone();
        }
        let host = cx.new(|_| Self::default());
        cx.global_mut::<TextSelectionHosts>()
            .0
            .insert(window_id, host.clone());
        host
    }

    /// Updates the active scope. Regions from other scopes cannot participate.
    pub fn set_active_scope(&mut self, scope: SelectionScopeId) {
        self.active_scope = scope;
    }

    /// Registers this frame's geometry for a region.
    pub fn register_region(
        &mut self,
        region: TextSelectionRegion,
        frame: SelectionRegionFrame,
        cx: &mut App,
    ) {
        self.prune_dead_regions();
        self.regions.insert(
            region.entity_id(),
            RegionRegistration {
                region: region.downgrade(),
                frame: Rc::new(frame),
            },
        );
        self.publish_snapshots(cx);
    }

    /// Starts a selection gesture using bounds hit testing (useful to adapters/tests).
    pub fn begin(&mut self, position: Point<Pixels>, extend: bool, cx: &mut App) {
        self.begin_impl(position, extend, None, cx);
    }

    /// Updates the current gesture using bounds hit testing.
    pub fn update(&mut self, position: Point<Pixels>, cx: &mut App) {
        self.update_impl(position, None, cx);
    }

    /// Ends the current gesture and keeps its selection visible.
    pub fn end(&mut self, cx: &mut App) {
        self.pending_extension_anchor = None;
        if !self.is_selecting {
            return;
        }
        self.is_selecting = false;
        if !self.did_hit_text {
            self.anchor = None;
            self.cursor = None;
        }
        self.stop_anchor_auto_scroll(cx);
        self.publish_snapshots(cx);
    }

    /// Clears both host selection and every region's local selection.
    pub fn clear(&mut self, cx: &mut App) {
        self.anchor = None;
        self.cursor = None;
        self.pending_extension_anchor = None;
        self.is_selecting = false;
        self.did_hit_text = false;
        self.prune_dead_regions();
        for registration in self.regions.values() {
            if let Some(region) = registration.region.upgrade() {
                region.update(cx, |state, cx| state.clear(cx));
            }
        }
    }

    /// Returns the currently selected text in logical document order.
    pub fn selected_text(&self, cx: &App) -> String {
        let mut items = self
            .regions
            .values()
            .filter_map(|registration| {
                let region = registration.region.upgrade()?;
                let state = region.read(cx);
                (state.snapshot.is_some() || state.local_selection)
                    .then(|| (registration.frame.document_order, state.copied_text(cx)))
            })
            .filter(|(_, text)| !text.trim().is_empty())
            .collect::<Vec<_>>();
        items.sort_by_key(|(order, _)| *order);
        items
            .into_iter()
            .map(|(_, text)| text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns whether a drag or a renderer-local selection is active.
    pub fn has_text_selection(&self, cx: &App) -> bool {
        self.snapshot().is_some()
            || self.regions.values().any(|registration| {
                registration
                    .region
                    .upgrade()
                    .is_some_and(|region| region.read(cx).local_selection)
            })
    }

    /// Returns the current resolved selection endpoints.
    pub fn snapshot(&self) -> Option<SelectionSnapshot> {
        let anchor = self.anchor.as_ref()?.resolve(&self.regions)?;
        let cursor = self.cursor.as_ref()?.resolve(&self.regions)?;
        (anchor != cursor).then_some(SelectionSnapshot {
            anchor,
            cursor,
            is_selecting: self.is_selecting,
        })
    }

    /// Returns whether a drag is currently in progress.
    pub fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    fn prepare_for_mouse_down(&mut self, extend: bool, cx: &mut App) {
        self.pending_extension_anchor = extend.then(|| self.anchor.clone()).flatten();
        self.clear(cx);
    }

    fn begin_in_window(
        &mut self,
        position: Point<Pixels>,
        extend: bool,
        window: &Window,
        cx: &mut App,
    ) {
        self.begin_impl(position, extend, Some(window), cx);
    }

    fn update_in_window(&mut self, position: Point<Pixels>, window: &Window, cx: &mut App) {
        self.update_impl(position, Some(window), cx);
    }

    fn begin_impl(
        &mut self,
        position: Point<Pixels>,
        extend: bool,
        window: Option<&Window>,
        cx: &mut App,
    ) {
        GlobalState::init(cx);
        if GlobalState::is_text_selection_suppressed(cx) {
            self.pending_extension_anchor = None;
            return;
        }
        let previous_anchor = extend
            .then(|| {
                self.pending_extension_anchor
                    .take()
                    .or_else(|| self.anchor.clone())
            })
            .flatten();
        if !extend {
            self.clear(cx);
        }
        let endpoint = self.endpoint(position, window);
        let anchor = previous_anchor.unwrap_or_else(|| endpoint.clone());
        self.anchor = Some(anchor.clone());
        self.cursor = Some(endpoint.clone());
        self.did_hit_text = anchor.inside_text || endpoint.inside_text;
        self.is_selecting = true;
        if anchor.inside {
            if let Some(region) = anchor.region.and_then(|region| region.upgrade()) {
                region.update(cx, |state, cx| state.focus(cx));
            }
        }
        self.publish_snapshots(cx);
    }

    fn update_impl(&mut self, position: Point<Pixels>, window: Option<&Window>, cx: &mut App) {
        if !self.is_selecting {
            return;
        }
        let endpoint = self.endpoint(position, window);
        self.did_hit_text |= endpoint.inside_text;
        self.cursor = Some(endpoint);
        self.update_anchor_auto_scroll(position, cx);
        self.publish_snapshots(cx);
    }

    fn endpoint(&mut self, position: Point<Pixels>, window: Option<&Window>) -> SelectionEndpoint {
        self.prune_dead_regions();
        let mut hit: Option<(
            WeakEntity<TextSelectionRegionState>,
            Rc<SelectionRegionFrame>,
            f32,
        )> = None;
        let mut predecessor: Option<(
            WeakEntity<TextSelectionRegionState>,
            Rc<SelectionRegionFrame>,
        )> = None;
        let mut first: Option<(
            WeakEntity<TextSelectionRegionState>,
            Rc<SelectionRegionFrame>,
        )> = None;

        for registration in self.regions.values() {
            if registration.frame.scope != self.active_scope
                || registration.region.upgrade().is_none()
            {
                continue;
            }
            let frame = &registration.frame;
            let hovered = window.map_or_else(
                || frame.bounds.contains(&position),
                |window| frame.hitbox.is_hovered(window),
            );
            if hovered {
                let area = f32::from(frame.bounds.size.width) * f32::from(frame.bounds.size.height);
                if hit
                    .as_ref()
                    .is_none_or(|(_, _, best_area)| area < *best_area)
                {
                    hit = Some((registration.region.clone(), frame.clone(), area));
                }
            }
            if frame.bounds.top() <= position.y
                && predecessor
                    .as_ref()
                    .is_none_or(|(_, best)| frame.bounds.top() > best.bounds.top())
            {
                predecessor = Some((registration.region.clone(), frame.clone()));
            }
            if first
                .as_ref()
                .is_none_or(|(_, best)| frame.bounds.top() < best.bounds.top())
            {
                first = Some((registration.region.clone(), frame.clone()));
            }
        }

        let selection = hit
            .map(|(region, frame, _)| (region, frame, true))
            .or_else(|| {
                predecessor
                    .or(first)
                    .map(|(region, frame)| (region, frame, false))
            });
        match selection {
            Some((region, frame, inside)) => SelectionEndpoint {
                point: position - frame.bounds.origin - frame.scroll_offset,
                region: Some(region),
                inside,
                inside_text: inside
                    && frame
                        .text_bounds
                        .iter()
                        .any(|bounds| bounds.contains(&position)),
            },
            None => SelectionEndpoint {
                region: None,
                point: position,
                inside: false,
                inside_text: false,
            },
        }
    }

    fn publish_snapshots(&mut self, cx: &mut App) {
        self.prune_dead_regions();
        let snapshot = self.snapshot();
        let single_region = self.single_region();
        for (id, registration) in &self.regions {
            let Some(region) = registration.region.upgrade() else {
                continue;
            };
            let region_snapshot = (registration.frame.scope == self.active_scope
                && single_region.is_none_or(|single| single == *id))
            .then_some(snapshot)
            .flatten();
            region.update(cx, |state, cx| state.set_snapshot(region_snapshot, cx));
        }
    }

    fn single_region(&self) -> Option<EntityId> {
        let anchor = self.anchor.as_ref()?.region_id()?;
        let cursor = self.cursor.as_ref()?.region_id()?;
        (anchor == cursor).then_some(anchor)
    }

    fn update_anchor_auto_scroll(&self, position: Point<Pixels>, cx: &mut App) {
        let Some(anchor) = self.anchor.as_ref().filter(|anchor| anchor.inside) else {
            return;
        };
        let Some(region) = anchor.region.as_ref().and_then(WeakEntity::upgrade) else {
            return;
        };
        let Some(frame) = self.regions.get(&region.entity_id()) else {
            return;
        };
        let delta = AutoScroll::compute_delta(position.y, frame.frame.bounds);
        region.update(cx, |state, cx| state.set_auto_scroll(delta, cx));
    }

    fn stop_anchor_auto_scroll(&self, cx: &mut App) {
        let Some(region) = self
            .anchor
            .as_ref()
            .filter(|anchor| anchor.inside)
            .and_then(|anchor| anchor.region.as_ref())
            .and_then(WeakEntity::upgrade)
        else {
            return;
        };
        region.update(cx, |state, cx| state.set_auto_scroll(None, cx));
    }

    fn prune_dead_regions(&mut self) {
        self.regions
            .retain(|_, registration| registration.region.upgrade().is_some());
    }
}

#[derive(Default)]
struct TextSelectionHosts(HashMap<gpui::WindowId, Entity<TextSelectionHost>>);

impl Global for TextSelectionHosts {}

/// A zero-sized controller which translates window mouse events into host gestures.
pub struct TextSelectionController;

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
        cx: &mut App,
    ) {
        let _ = TextSelectionHost::for_window(window, cx);
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if event.button != MouseButton::Left {
                return;
            }
            let host = TextSelectionHost::for_window(window, cx);
            if phase.capture() {
                GlobalState::reset_text_selection_suppression(cx);
                host.update(cx, |host, cx| {
                    host.prepare_for_mouse_down(event.click_count == 1 && event.modifiers.shift, cx)
                });
            } else if event.click_count == 1 {
                if GlobalState::is_text_selection_suppressed(cx) {
                    host.update(cx, |host, _| host.pending_extension_anchor = None);
                    return;
                }
                host.update(cx, |host, cx| {
                    host.begin_in_window(event.position, event.modifiers.shift, window, cx)
                });
            }
        });
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase.bubble() {
                let host = TextSelectionHost::for_window(window, cx);
                host.update(cx, |host, cx| {
                    host.update_in_window(event.position, window, cx)
                });
            }
        });
        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
            if phase.bubble() {
                let host = TextSelectionHost::for_window(window, cx);
                host.update(cx, |host, cx| host.end(cx));
            }
        });
        window.on_mouse_event(move |_: &ScrollWheelEvent, phase, window, cx| {
            if phase.bubble() {
                let position = window.mouse_position();
                let host = TextSelectionHost::for_window(window, cx);
                host.update(cx, |host, cx| host.update_in_window(position, window, cx));
            }
        });
    }
}

/// Window helpers backed by the base-owned selection host.
pub trait WindowTextSelectionExt {
    fn selected_text(&mut self, cx: &mut App) -> String;
    fn has_text_selection(&mut self, cx: &mut App) -> bool;
    fn clear_text_selection(&mut self, cx: &mut App);
    fn end_text_selection(&mut self, cx: &mut App);
}

impl WindowTextSelectionExt for Window {
    fn selected_text(&mut self, cx: &mut App) -> String {
        TextSelectionHost::for_window(self, cx)
            .read(cx)
            .selected_text(cx)
    }

    fn has_text_selection(&mut self, cx: &mut App) -> bool {
        TextSelectionHost::for_window(self, cx)
            .read(cx)
            .has_text_selection(cx)
    }

    fn clear_text_selection(&mut self, cx: &mut App) {
        TextSelectionHost::for_window(self, cx).update(cx, |host, cx| host.clear(cx));
    }

    fn end_text_selection(&mut self, cx: &mut App) {
        TextSelectionHost::for_window(self, cx).update(cx, |host, cx| host.end(cx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Bounds, ContentMask, Context, Hitbox, HitboxBehavior, HitboxId, IntoElement, Render,
        TestAppContext, Window, div, point, px, size,
    };

    struct FakeRegion {
        region: TextSelectionRegion,
    }

    struct WindowRegionView {
        region: TextSelectionRegion,
    }

    impl Render for WindowRegionView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    impl FakeRegion {
        fn new(text: &str, cx: &mut gpui::App) -> Self {
            let region = TextSelectionRegion::new(text, cx);
            Self { region }
        }

        fn register(
            &self,
            host: &mut TextSelectionHost,
            y: f32,
            scope: SelectionScopeId,
            document_order: u64,
            cx: &mut gpui::App,
        ) {
            let bounds = Bounds::new(point(px(0.), px(y)), size(px(100.), px(10.)));
            host.register_region(
                self.region.clone(),
                SelectionRegionFrame {
                    hitbox: Hitbox {
                        id: HitboxId::placeholder(),
                        bounds,
                        content_mask: ContentMask { bounds },
                        behavior: HitboxBehavior::Normal,
                    },
                    bounds,
                    scroll_offset: point(px(0.), px(0.)),
                    scope,
                    document_order,
                    text_bounds: vec![bounds],
                },
                cx,
            );
        }
    }

    #[gpui::test]
    fn begin_update_and_end_publish_a_cross_region_selection(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionHost::default();
            let first = FakeRegion::new("first", cx);
            let second = FakeRegion::new("second", cx);
            first.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            second.register(&mut host, 20., SelectionScopeId::default(), 1, cx);

            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(1.), px(25.)), cx);
            assert!(host.has_text_selection(cx));
            assert_eq!(host.selected_text(cx), "first\nsecond");

            host.end(cx);
            assert!(!host.is_selecting());
        });
    }

    #[gpui::test]
    fn shift_extension_keeps_its_original_anchor_when_reversed(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionHost::default();
            let region = FakeRegion::new("region", cx);
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);

            host.begin(point(px(2.), px(2.)), false, cx);
            host.end(cx);
            host.begin(point(px(8.), px(2.)), true, cx);
            host.end(cx);
            let first_anchor = host.snapshot().unwrap().anchor;

            host.begin(point(px(0.), px(2.)), true, cx);
            host.end(cx);
            let reversed = host.snapshot().unwrap();
            assert_eq!(reversed.anchor, first_anchor);
            assert!(reversed.cursor.x < reversed.anchor.x);
        });
    }

    #[gpui::test]
    fn scope_and_suppression_prevent_unrelated_regions_from_participating(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionHost::default();
            let base = FakeRegion::new("base", cx);
            let modal = FakeRegion::new("modal", cx);
            base.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            modal.register(&mut host, 20., SelectionScopeId(1), 1, cx);

            host.set_active_scope(SelectionScopeId(1));
            host.begin(point(px(1.), px(21.)), false, cx);
            host.update(point(px(8.), px(21.)), cx);
            host.end(cx);
            assert_eq!(host.selected_text(cx), "modal");

            host.clear(cx);
            GlobalState::init(cx);
            GlobalState::suppress_text_selection(cx);
            host.begin(point(px(1.), px(21.)), false, cx);
            host.update(point(px(8.), px(21.)), cx);
            assert!(!host.has_text_selection(cx));
        });
    }

    #[gpui::test]
    fn dead_regions_are_pruned_and_empty_selection_falls_back_safely(cx: &mut TestAppContext) {
        let host = cx.update(|cx| {
            let host = cx.new(|_| TextSelectionHost::default());
            let region = FakeRegion::new("gone", cx);
            host.update(cx, |host, cx| {
                region.register(host, 0., SelectionScopeId::default(), 0, cx)
            });
            host
        });
        cx.update(|cx| {
            host.update(cx, |host, cx| {
                host.begin(point(px(1.), px(1.)), false, cx);
                host.update(point(px(8.), px(1.)), cx);
                host.end(cx);

                assert_eq!(host.selected_text(cx), "");
                assert!(!host.has_text_selection(cx));
            });
        });
    }

    #[gpui::test]
    fn window_extension_reports_copies_ends_and_clears_host_selection(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| WindowRegionView {
            region: TextSelectionRegion::new("copied", cx),
        });
        cx.update(|window, cx| {
            let region = view.read(cx).region.clone();
            let host = TextSelectionHost::for_window(window, cx);
            host.update(cx, |host, cx| {
                FakeRegion { region }.register(host, 0., SelectionScopeId::default(), 0, cx);
                host.begin(point(px(1.), px(1.)), false, cx);
                host.update(point(px(8.), px(1.)), cx);
            });

            assert!(window.has_text_selection(cx));
            assert_eq!(window.selected_text(cx), "copied");
            window.end_text_selection(cx);
            assert!(window.has_text_selection(cx));
            window.clear_text_selection(cx);
            assert!(!window.has_text_selection(cx));
            assert_eq!(window.selected_text(cx), "");
        });
    }
}
