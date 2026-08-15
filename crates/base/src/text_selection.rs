use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
};

use gpui::{
    App, AppContext as _, Bounds, Element, ElementId, Entity, EntityId, Global, GlobalElementId,
    Half, Hitbox, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollWheelEvent, SharedString, Style, TextLayout,
    WeakEntity, Window,
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

/// A selection endpoint anchored to a region's content coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionEndpointSnapshot {
    pub region_id: Option<EntityId>,
    pub point: Point<Pixels>,
    virtual_key: Option<u64>,
}

impl SelectionEndpointSnapshot {
    /// Returns renderer-defined endpoint metadata captured when it hit a region.
    pub fn virtual_key(&self) -> Option<u64> {
        self.virtual_key
    }
}

/// Region-relative selection endpoints with an optional rendering projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionSnapshot {
    pub anchor: SelectionEndpointSnapshot,
    pub cursor: SelectionEndpointSnapshot,
    pub is_selecting: bool,
    resolved_points: Option<(Point<Pixels>, Point<Pixels>)>,
    coverage: SelectionRegionCoverage,
}

/// The portion of a participating region covered by a window selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionRegionCoverage {
    #[default]
    Bounded,
    FromStart,
    ToEnd,
    Full,
}

impl SelectionSnapshot {
    /// Returns the window-coordinate endpoints for renderers that need them.
    pub fn resolved_points(&self) -> Option<(Point<Pixels>, Point<Pixels>)> {
        self.resolved_points
    }

    pub fn region_coverage(&self) -> SelectionRegionCoverage {
        self.coverage
    }
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

/// Per-frame text layout reported by a plain selectable region.
#[derive(Clone)]
pub struct SelectionRunFrame {
    /// Logical order within the containing region.
    pub order: u64,
    /// The exact text used to produce `layout`.
    pub text: SharedString,
    /// Laid-out glyph geometry in window coordinates.
    pub layout: TextLayout,
    /// The run's window-coordinate paint bounds.
    pub bounds: Bounds<Pixels>,
}

/// Selection projection for one [`SelectionRunFrame`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionRunState {
    /// The selected UTF-8 byte range in the run's text.
    pub byte_range: Option<Range<usize>>,
    /// Whether the containing region participates in the current selection.
    pub active: bool,
}

/// Projects a region selection snapshot onto laid-out plain-text runs.
///
/// The returned states retain the input order so callers can pair every state
/// with its frame. The ranges are always character boundaries; `order` is used
/// only when a region caches selected text for copying.
pub fn project_selection_runs(
    snapshot: Option<SelectionSnapshot>,
    runs: &[SelectionRunFrame],
) -> Vec<SelectionRunState> {
    let Some(snapshot) = snapshot else {
        return vec![SelectionRunState::default(); runs.len()];
    };
    let Some((anchor, cursor)) = snapshot.resolved_points() else {
        return runs
            .iter()
            .map(|_| SelectionRunState {
                byte_range: None,
                active: true,
            })
            .collect();
    };

    runs.iter()
        .map(|run| SelectionRunState {
            byte_range: selection_range_for_run(run, anchor, cursor),
            active: true,
        })
        .collect()
}

fn selection_range_for_run(
    run: &SelectionRunFrame,
    selection_start: Point<Pixels>,
    selection_end: Point<Pixels>,
) -> Option<Range<usize>> {
    if run.text.len() != run.layout.len() {
        return None;
    }

    let line_height = run.layout.line_height();
    let mut range = None;
    for (offset, character) in run.text.char_indices() {
        let next_offset = offset + character.len_utf8();
        let Some(position) = run.layout.position_for_index(offset) else {
            continue;
        };

        let char_width = run
            .layout
            .position_for_index(next_offset)
            .filter(|next| next.y == position.y)
            .map_or_else(|| line_height.half(), |next| next.x - position.x);

        if point_in_selection_band(
            position,
            char_width,
            selection_start,
            selection_end,
            line_height,
        ) {
            range.get_or_insert(offset..offset).end = next_offset;
        }
    }
    range
}

fn point_in_selection_band(
    position: Point<Pixels>,
    char_width: Pixels,
    selection_start: Point<Pixels>,
    selection_end: Point<Pixels>,
    line_height: Pixels,
) -> bool {
    let point_in_line =
        |point: Point<Pixels>| point.y >= position.y && point.y < position.y + line_height;
    let top = selection_start.y.min(selection_end.y);
    let bottom = selection_start.y.max(selection_end.y);
    let x = position.x + char_width.half();

    if position.y + line_height <= top || position.y > bottom {
        return false;
    }

    if point_in_line(selection_start) && point_in_line(selection_end) {
        let left = selection_start.x.min(selection_end.x);
        let right = selection_start.x.max(selection_end.x);
        return x >= left && x <= right;
    }

    let (top_point, bottom_point) = if selection_start.y < selection_end.y {
        (selection_start, selection_end)
    } else {
        (selection_end, selection_start)
    };
    if point_in_line(top_point) {
        x >= top_point.x
    } else if point_in_line(bottom_point) {
        x <= bottom_point.x
    } else {
        true
    }
}

type RegionSelectionCallback = Rc<dyn Fn(Option<SelectionSnapshot>, &mut App)>;
type RegionAutoScrollCallback = Rc<dyn Fn(Option<Pixels>, &mut App)>;
type RegionVoidCallback = Rc<dyn Fn(&mut App)>;
type RegionFocusCallback = Rc<dyn Fn(&mut Window, &mut App)>;
type RegionCopyCallback = Rc<dyn Fn(&App) -> String>;
type RegionVirtualKeyCallback = Rc<dyn Fn(Point<Pixels>, &App) -> Option<u64>>;
type RegionClearCallbacks = (Option<RegionVoidCallback>, Option<RegionSelectionCallback>);

/// Renderer-owned state associated with a selectable region.
///
/// The window layer owns only generic geometry and gesture state. Renderers store
/// their projection, highlights, and optional focus/scroll hooks here.
pub struct TextSelectionRegionState {
    selected_text: String,
    projected_selected_text: Option<String>,
    local_selection: bool,
    snapshot: Option<SelectionSnapshot>,
    on_selection: Option<RegionSelectionCallback>,
    on_auto_scroll: Option<RegionAutoScrollCallback>,
    on_focus: Option<RegionFocusCallback>,
    on_clear: Option<RegionVoidCallback>,
    copy: Option<RegionCopyCallback>,
    virtual_key: Option<RegionVirtualKeyCallback>,
    callback_epoch: Rc<Cell<u64>>,
}

impl TextSelectionRegionState {
    fn new(selected_text: impl Into<String>) -> Self {
        Self {
            selected_text: selected_text.into(),
            projected_selected_text: None,
            local_selection: false,
            snapshot: None,
            on_selection: None,
            on_auto_scroll: None,
            on_focus: None,
            on_clear: None,
            copy: None,
            virtual_key: None,
            callback_epoch: Rc::new(Cell::new(0)),
        }
    }

    /// The current geometry selection snapshot for this region.
    pub fn snapshot(&self) -> Option<SelectionSnapshot> {
        self.snapshot
    }

    /// Sets the text copied by this region when it participates in selection.
    pub fn set_selected_text(&mut self, text: impl Into<String>) {
        self.selected_text = text.into();
        self.projected_selected_text = None;
    }

    /// Marks renderer-local selection (for example select-all) as active.
    pub fn set_local_selection(&mut self, active: bool) {
        self.local_selection = active;
    }

    /// Projects this region's current snapshot onto plain-text runs and caches
    /// their selected substrings for the window selection query.
    ///
    /// Call this once per painted run frame. A snapshot change or
    /// Clearing window selection invalidates the cache immediately, so copy
    /// never returns text from a previous projection while waiting to repaint.
    pub fn project_selection_runs(&mut self, runs: &[SelectionRunFrame]) -> Vec<SelectionRunState> {
        let states = project_selection_runs(self.snapshot, runs);
        let mut selected_runs = runs
            .iter()
            .zip(&states)
            .enumerate()
            .filter_map(|(index, (run, state))| {
                state.byte_range.as_ref().map(|range| {
                    debug_assert!(run.text.is_char_boundary(range.start));
                    debug_assert!(run.text.is_char_boundary(range.end));
                    (run.order, index, run.text[range.clone()].to_string())
                })
            })
            .collect::<Vec<_>>();
        selected_runs.sort_by_key(|(order, index, _)| (*order, *index));
        self.projected_selected_text =
            Some(selected_runs.into_iter().map(|(_, _, text)| text).collect());
        states
    }

    /// Installs the callback which updates renderer highlights from window state.
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
    pub fn on_focus(&mut self, callback: impl Fn(&mut Window, &mut App) + 'static) {
        self.on_focus = Some(Rc::new(callback));
    }

    /// Installs renderer-local cleanup invoked when window selection is cleared.
    pub fn on_clear(&mut self, callback: impl Fn(&mut App) + 'static) {
        self.on_clear = Some(Rc::new(callback));
    }

    /// Installs a renderer-specific copy projection.
    pub fn copy_with(&mut self, callback: impl Fn(&App) -> String + 'static) {
        self.copy = Some(Rc::new(callback));
    }

    /// Installs a renderer-specific lookup for stable virtualized content keys.
    pub fn on_virtual_key(
        &mut self,
        callback: impl Fn(Point<Pixels>, &App) -> Option<u64> + 'static,
    ) {
        self.virtual_key = Some(Rc::new(callback));
    }

    fn set_snapshot(&mut self, snapshot: Option<SelectionSnapshot>, cx: &mut App) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        self.projected_selected_text = None;
        let epoch = self.callback_epoch.get().wrapping_add(1);
        self.callback_epoch.set(epoch);
        if let Some(callback) = self.on_selection.clone() {
            let callback_epoch = self.callback_epoch.clone();
            cx.defer(move |cx| {
                if callback_epoch.get() == epoch {
                    callback(snapshot, cx);
                }
            });
        }
    }

    fn clear_state(&mut self) -> RegionClearCallbacks {
        self.callback_epoch
            .set(self.callback_epoch.get().wrapping_add(1));
        self.snapshot = None;
        self.projected_selected_text = None;
        self.local_selection = false;
        (self.on_clear.clone(), self.on_selection.clone())
    }

    fn dispatch_clear(callbacks: RegionClearCallbacks, cx: &mut App) {
        if let Some(callback) = callbacks.0 {
            callback(cx);
        }
        if let Some(callback) = callbacks.1 {
            callback(None, cx);
        }
    }

    fn set_auto_scroll(&self, delta: Option<Pixels>, cx: &mut App) {
        if let Some(callback) = self.on_auto_scroll.clone() {
            cx.defer(move |cx| callback(delta, cx));
        }
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        if let Some(callback) = self.on_focus.clone() {
            window.defer(cx, move |window, cx| callback(window, cx));
        }
    }

    fn copied_text(&self, cx: &App) -> String {
        self.copy
            .as_ref()
            .map(|callback| callback(cx))
            .unwrap_or_else(|| {
                self.projected_selected_text
                    .clone()
                    .unwrap_or_else(|| self.selected_text.clone())
            })
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
    generation: u64,
}

#[derive(Clone)]
struct SelectionEndpoint {
    region: Option<WeakEntity<TextSelectionRegionState>>,
    point: Point<Pixels>,
    inside: bool,
    inside_text: bool,
    virtual_key: Option<u64>,
    virtual_resolver: Option<(RegionVirtualKeyCallback, Point<Pixels>)>,
}

impl SelectionEndpoint {
    fn snapshot(&self) -> SelectionEndpointSnapshot {
        SelectionEndpointSnapshot {
            region_id: self.region_id(),
            point: self.point,
            virtual_key: self.virtual_key,
        }
    }

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
struct TextSelectionState {
    regions: HashMap<EntityId, RegionRegistration>,
    active_scope: SelectionScopeId,
    anchor: Option<SelectionEndpoint>,
    cursor: Option<SelectionEndpoint>,
    pending_extension_anchor: Option<SelectionEndpoint>,
    is_selecting: bool,
    did_hit_text: bool,
    frame_generation: u64,
    finish_frame_scheduled: bool,
    mouse_down_prepared: bool,
}

impl Default for TextSelectionState {
    fn default() -> Self {
        Self {
            regions: HashMap::new(),
            active_scope: SelectionScopeId::default(),
            anchor: None,
            cursor: None,
            pending_extension_anchor: None,
            is_selecting: false,
            did_hit_text: false,
            frame_generation: 0,
            finish_frame_scheduled: false,
            mouse_down_prepared: false,
        }
    }
}

impl TextSelectionState {
    fn resolve_virtual_keys(state: &Entity<Self>, cx: &mut App) {
        let pending = state.update(cx, |state, _| {
            [
                state
                    .anchor
                    .as_ref()
                    .and_then(|endpoint| endpoint.virtual_resolver.clone()),
                state
                    .cursor
                    .as_ref()
                    .and_then(|endpoint| endpoint.virtual_resolver.clone()),
            ]
        });
        let resolved =
            pending.map(|pending| pending.and_then(|(callback, point)| callback(point, cx)));
        state.update(cx, |state, cx| {
            if let (Some(endpoint), Some(key)) = (state.anchor.as_mut(), resolved[0]) {
                endpoint.virtual_key = Some(key);
                endpoint.virtual_resolver = None;
            }
            if let (Some(endpoint), Some(key)) = (state.cursor.as_mut(), resolved[1]) {
                endpoint.virtual_key = Some(key);
                endpoint.virtual_resolver = None;
            }
            state.publish_snapshots(cx);
        });
    }
    /// Creates the private state used by the [`TextSelection`] element.
    fn ensure(window: &Window, cx: &mut App) -> Entity<Self> {
        if !cx.has_global::<WindowTextSelections>() {
            cx.set_global(WindowTextSelections::default());
        }
        let live_windows = cx
            .windows()
            .into_iter()
            .map(|handle| handle.window_id())
            .collect::<HashSet<_>>();
        cx.global_mut::<WindowTextSelections>()
            .0
            .retain(|window_id, _| live_windows.contains(window_id));
        let window_id = window.window_handle().window_id();
        if let Some(state) = cx.global::<WindowTextSelections>().0.get(&window_id) {
            return state.clone();
        }
        let state = cx.new(|_| Self::default());
        cx.global_mut::<WindowTextSelections>()
            .0
            .insert(window_id, state.clone());
        state
    }

    fn existing(window: &Window, cx: &App) -> Option<Entity<Self>> {
        if !cx.has_global::<WindowTextSelections>() {
            return None;
        }
        cx.global::<WindowTextSelections>()
            .0
            .get(&window.window_handle().window_id())
            .cloned()
    }

    /// Updates the active scope. Regions from other scopes cannot participate.
    #[cfg(test)]
    fn set_active_scope(&mut self, scope: SelectionScopeId, cx: &mut App) {
        let callbacks = self.set_active_scope_state(scope, cx);
        for callbacks in callbacks {
            TextSelectionRegionState::dispatch_clear(callbacks, cx);
        }
    }

    fn set_active_scope_state(
        &mut self,
        scope: SelectionScopeId,
        cx: &mut App,
    ) -> Vec<RegionClearCallbacks> {
        if self.active_scope == scope {
            return Vec::new();
        }
        let callbacks = self.clear_state(cx);
        self.active_scope = scope;
        self.publish_snapshots(cx);
        callbacks
    }

    /// Sweeps regions after a rendered frame has completed.
    ///
    /// Registrations are stamped with the current generation while any sibling
    /// is painting. Sweeping only after paint makes registration independent of
    /// whether a region or the controller paints first.
    pub fn finish_frame(&mut self, cx: &mut App) {
        self.finish_frame_scheduled = false;
        let stale = self
            .regions
            .iter()
            .filter_map(|(id, registration)| {
                (registration.generation != self.frame_generation)
                    .then(|| (*id, registration.region.clone()))
            })
            .collect::<Vec<_>>();
        for (id, region) in stale {
            self.regions.remove(&id);
            if let Some(region) = region.upgrade() {
                region.update(cx, |state, cx| state.set_snapshot(None, cx));
            }
        }
        self.publish_snapshots(cx);
        self.frame_generation = self.frame_generation.wrapping_add(1);
    }

    fn schedule_finish_frame(&mut self) -> bool {
        if self.finish_frame_scheduled {
            return false;
        }
        self.finish_frame_scheduled = true;
        true
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
                generation: self.frame_generation,
            },
        );
        self.publish_snapshots(cx);
    }

    /// Starts a selection gesture using bounds hit testing (useful to adapters/tests).
    #[cfg(test)]
    fn begin(&mut self, position: Point<Pixels>, extend: bool, cx: &mut App) {
        self.begin_impl(position, extend, None, cx);
    }

    /// Updates the current gesture using bounds hit testing.
    #[cfg(test)]
    fn update(&mut self, position: Point<Pixels>, cx: &mut App) {
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

    /// Clears both window selection and every region's local selection.
    pub fn clear(&mut self, cx: &mut App) {
        let callbacks = self.clear_state(cx);
        for callbacks in callbacks {
            TextSelectionRegionState::dispatch_clear(callbacks, cx);
        }
    }

    fn clear_state(&mut self, cx: &mut App) -> Vec<RegionClearCallbacks> {
        self.stop_anchor_auto_scroll(cx);
        self.anchor = None;
        self.cursor = None;
        self.pending_extension_anchor = None;
        self.is_selecting = false;
        self.did_hit_text = false;
        self.prune_dead_regions();
        self.regions
            .values()
            .filter_map(|registration| registration.region.upgrade())
            .map(|region| region.update(cx, |state, _| state.clear_state()))
            .collect()
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
        if !self.did_hit_text {
            return None;
        }
        let anchor = self.anchor.as_ref()?.resolve(&self.regions)?;
        let cursor = self.cursor.as_ref()?.resolve(&self.regions)?;
        (anchor != cursor).then_some(SelectionSnapshot {
            anchor: self.anchor.as_ref()?.snapshot(),
            cursor: self.cursor.as_ref()?.snapshot(),
            is_selecting: self.is_selecting,
            coverage: SelectionRegionCoverage::Bounded,
            resolved_points: Some((anchor, cursor)),
        })
    }

    /// Returns whether a drag is currently in progress.
    #[cfg(test)]
    fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    fn prepare_for_mouse_down(&mut self, extend: bool, cx: &mut App) -> Vec<RegionClearCallbacks> {
        let pending_extension_anchor = extend.then(|| self.anchor.clone()).flatten();
        self.stop_anchor_auto_scroll(cx);
        self.anchor = None;
        self.cursor = None;
        self.pending_extension_anchor = None;
        self.is_selecting = false;
        self.did_hit_text = false;
        self.prune_dead_regions();
        let callbacks = self
            .regions
            .values()
            .filter_map(|registration| registration.region.upgrade())
            .map(|region| region.update(cx, |state, _| state.clear_state()))
            .collect();
        self.pending_extension_anchor = pending_extension_anchor;
        callbacks
    }

    fn begin_in_window(
        &mut self,
        position: Point<Pixels>,
        extend: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.begin_impl(position, extend, Some(window), cx);
    }

    fn update_in_window(&mut self, position: Point<Pixels>, window: &Window, cx: &mut App) {
        self.update_in_window_with_active_drag(position, cx.has_active_drag(), window, cx);
    }

    fn update_in_window_with_active_drag(
        &mut self,
        position: Point<Pixels>,
        active_drag: bool,
        window: &Window,
        cx: &mut App,
    ) {
        if !active_drag {
            self.update_impl(position, Some(window), cx);
        }
    }

    fn begin_impl(
        &mut self,
        position: Point<Pixels>,
        extend: bool,
        mut window: Option<&mut Window>,
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
            .flatten()
            .filter(|anchor| anchor.resolve(&self.regions).is_some());
        if !extend {
            self.clear(cx);
        }
        let endpoint = self.endpoint(position, window.as_deref(), cx);
        let focus_region = endpoint.inside.then(|| endpoint.region.clone()).flatten();
        let anchor = previous_anchor.unwrap_or_else(|| endpoint.clone());
        self.anchor = Some(anchor.clone());
        self.cursor = Some(endpoint.clone());
        self.did_hit_text = anchor.inside_text || endpoint.inside_text;
        self.is_selecting = true;
        if let Some(region) = focus_region.and_then(|region| region.upgrade()) {
            if let Some(window) = window.as_deref_mut() {
                region.update(cx, |state, cx| state.focus(window, cx));
            }
        }
        self.publish_snapshots(cx);
    }

    fn update_impl(&mut self, position: Point<Pixels>, window: Option<&Window>, cx: &mut App) {
        if !self.is_selecting {
            return;
        }
        let endpoint = self.endpoint(position, window, cx);
        self.did_hit_text |= endpoint.inside_text;
        self.cursor = Some(endpoint);
        self.update_anchor_auto_scroll(position, cx);
        self.publish_snapshots(cx);
    }

    fn endpoint(
        &mut self,
        position: Point<Pixels>,
        window: Option<&Window>,
        cx: &App,
    ) -> SelectionEndpoint {
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
                && predecessor.as_ref().is_none_or(|(_, best)| {
                    frame.bounds.top() > best.bounds.top()
                        || (frame.bounds.top() == best.bounds.top()
                            && frame.document_order < best.document_order)
                })
            {
                predecessor = Some((registration.region.clone(), frame.clone()));
            }
            if first.as_ref().is_none_or(|(_, best)| {
                frame.bounds.top() < best.bounds.top()
                    || (frame.bounds.top() == best.bounds.top()
                        && frame.document_order < best.document_order)
            }) {
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
            Some((region, frame, inside)) => {
                let point = position - frame.bounds.origin - frame.scroll_offset;
                let virtual_resolver = region.upgrade().and_then(|region| {
                    region
                        .read(cx)
                        .virtual_key
                        .clone()
                        .map(|callback| (callback, point))
                });
                SelectionEndpoint {
                    point,
                    region: Some(region),
                    inside,
                    inside_text: inside
                        && frame
                            .text_bounds
                            .iter()
                            .any(|bounds| bounds.contains(&position)),
                    virtual_key: None,
                    virtual_resolver,
                }
            }
            None => SelectionEndpoint {
                region: None,
                point: position,
                inside: false,
                inside_text: false,
                virtual_key: None,
                virtual_resolver: None,
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
                && self.participates(*id, registration)
                && single_region.is_none_or(|single| single == *id))
            .then_some(snapshot)
            .flatten()
            .map(|mut snapshot| {
                snapshot.coverage = self.coverage_for(*id);
                snapshot
            });
            region.update(cx, |state, cx| state.set_snapshot(region_snapshot, cx));
        }
    }

    fn coverage_for(&self, id: EntityId) -> SelectionRegionCoverage {
        let Some(anchor) = self.anchor.as_ref().and_then(SelectionEndpoint::region_id) else {
            return SelectionRegionCoverage::Bounded;
        };
        let Some(cursor) = self.cursor.as_ref().and_then(SelectionEndpoint::region_id) else {
            return SelectionRegionCoverage::Bounded;
        };
        if anchor == cursor {
            return SelectionRegionCoverage::Bounded;
        }
        let anchor_order = self.regions[&anchor].frame.document_order;
        let cursor_order = self.regions[&cursor].frame.document_order;
        if id != anchor && id != cursor {
            SelectionRegionCoverage::Full
        } else if (id == anchor) == (anchor_order < cursor_order) {
            SelectionRegionCoverage::ToEnd
        } else {
            SelectionRegionCoverage::FromStart
        }
    }

    fn single_region(&self) -> Option<EntityId> {
        let anchor = self.anchor.as_ref()?.region_id()?;
        let cursor = self.cursor.as_ref()?.region_id()?;
        (anchor == cursor).then_some(anchor)
    }

    fn participates(&self, id: EntityId, registration: &RegionRegistration) -> bool {
        let Some(anchor) = self.anchor.as_ref().and_then(SelectionEndpoint::region_id) else {
            return false;
        };
        let Some(cursor) = self.cursor.as_ref().and_then(SelectionEndpoint::region_id) else {
            return false;
        };
        let Some(anchor_frame) = self.regions.get(&anchor) else {
            return false;
        };
        let Some(cursor_frame) = self.regions.get(&cursor) else {
            return false;
        };
        let start = anchor_frame
            .frame
            .document_order
            .min(cursor_frame.frame.document_order);
        let end = anchor_frame
            .frame
            .document_order
            .max(cursor_frame.frame.document_order);
        (start..=end).contains(&registration.frame.document_order) || id == anchor || id == cursor
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
struct WindowTextSelections(HashMap<gpui::WindowId, Entity<TextSelectionState>>);

impl Global for WindowTextSelections {}

/// A zero-sized element which enables text selection for a window root.
pub struct TextSelection;

impl IntoElement for TextSelection {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextSelection {
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
        let state = TextSelectionState::ensure(window, cx);
        if state.update(cx, |state, _| state.schedule_finish_frame()) {
            let state = state.clone();
            window.on_next_frame(move |_, cx| {
                state.update(cx, |state, cx| state.finish_frame(cx));
            });
        }
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if event.button != MouseButton::Left {
                return;
            }
            let state = TextSelectionState::ensure(window, cx);
            if phase.capture() {
                GlobalState::init(cx);
                GlobalState::reset_text_selection_suppression(cx);
                let callbacks = state.update(cx, |state, cx| {
                    if state.mouse_down_prepared {
                        return Vec::new();
                    }
                    state.mouse_down_prepared = true;
                    state
                        .prepare_for_mouse_down(event.click_count == 1 && event.modifiers.shift, cx)
                });
                for callbacks in callbacks {
                    TextSelectionRegionState::dispatch_clear(callbacks, cx);
                }
            } else if event.click_count == 1 {
                if GlobalState::is_text_selection_suppressed(cx) {
                    state.update(cx, |state, _| state.pending_extension_anchor = None);
                    return;
                }
                state.update(cx, |state, cx| {
                    if !state.is_selecting {
                        state.begin_in_window(event.position, event.modifiers.shift, window, cx)
                    }
                });
                TextSelectionState::resolve_virtual_keys(&state, cx);
            }
        });
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase.bubble() {
                let state = TextSelectionState::ensure(window, cx);
                state.update(cx, |state, cx| {
                    state.update_in_window(event.position, window, cx)
                });
                TextSelectionState::resolve_virtual_keys(&state, cx);
            }
        });
        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
            if phase.bubble() {
                let state = TextSelectionState::ensure(window, cx);
                state.update(cx, |state, cx| {
                    state.mouse_down_prepared = false;
                    state.end(cx)
                });
            }
        });
        window.on_mouse_event(move |_: &ScrollWheelEvent, phase, window, cx| {
            if phase.bubble() {
                let position = window.mouse_position();
                let state = TextSelectionState::ensure(window, cx);
                state.update(cx, |state, cx| state.update_in_window(position, window, cx));
                TextSelectionState::resolve_virtual_keys(&state, cx);
            }
        });
    }
}

/// Window text-selection operations. Add one [`TextSelection`] element to a
/// custom window root; before it paints these operations are safe no-ops.
pub trait WindowTextSelection {
    fn selected_text(&mut self, cx: &mut App) -> String;
    fn has_text_selection(&mut self, cx: &mut App) -> bool;
    fn clear_text_selection(&mut self, cx: &mut App);
    fn end_text_selection(&mut self, cx: &mut App);
    #[doc(hidden)]
    fn set_text_selection_scope(&mut self, scope: SelectionScopeId, cx: &mut App);
    #[doc(hidden)]
    fn register_text_selection_region(
        &mut self,
        region: TextSelectionRegion,
        frame: SelectionRegionFrame,
        cx: &mut App,
    );
}

impl WindowTextSelection for Window {
    fn selected_text(&mut self, cx: &mut App) -> String {
        TextSelectionState::existing(self, cx)
            .map(|state| state.read(cx).selected_text(cx))
            .unwrap_or_default()
    }

    fn has_text_selection(&mut self, cx: &mut App) -> bool {
        TextSelectionState::existing(self, cx)
            .is_some_and(|state| state.read(cx).has_text_selection(cx))
    }

    fn clear_text_selection(&mut self, cx: &mut App) {
        if let Some(state) = TextSelectionState::existing(self, cx) {
            let callbacks = state.update(cx, |state, cx| state.clear_state(cx));
            for callbacks in callbacks {
                TextSelectionRegionState::dispatch_clear(callbacks, cx);
            }
        }
    }

    fn end_text_selection(&mut self, cx: &mut App) {
        if let Some(state) = TextSelectionState::existing(self, cx) {
            state.update(cx, |state, cx| state.end(cx));
        }
    }

    fn set_text_selection_scope(&mut self, scope: SelectionScopeId, cx: &mut App) {
        let state = TextSelectionState::ensure(self, cx);
        let callbacks = state.update(cx, |state, cx| state.set_active_scope_state(scope, cx));
        for callbacks in callbacks {
            TextSelectionRegionState::dispatch_clear(callbacks, cx);
        }
    }

    fn register_text_selection_region(
        &mut self,
        region: TextSelectionRegion,
        frame: SelectionRegionFrame,
        cx: &mut App,
    ) {
        TextSelectionState::ensure(self, cx)
            .update(cx, |state, cx| state.register_region(region, frame, cx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Bounds, ContentMask, Context, Hitbox, HitboxBehavior, HitboxId, InteractiveElement as _,
        IntoElement, ParentElement as _, Render, SharedString, Styled as _, StyledText,
        TestAppContext, TextLayout, Window, div, point, px, size,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    struct FakeRegion {
        region: TextSelectionRegion,
    }

    struct WindowRegionView {
        region: TextSelectionRegion,
    }

    struct ControllerOnlyView;

    struct DoubleControllerView {
        region: TextSelectionRegion,
    }

    struct PlainRunLayoutView {
        texts: Vec<SharedString>,
        layouts: Vec<TextLayout>,
    }

    impl Render for WindowRegionView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    impl Render for ControllerOnlyView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(TextSelection)
                .child(
                    div()
                        .size_full()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            GlobalState::suppress_text_selection(cx);
                        }),
                )
        }
    }

    impl Render for DoubleControllerView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(TextSelection).child(TextSelection)
        }
    }

    impl Render for PlainRunLayoutView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.layouts.clear();
            let children = self
                .texts
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    let text = StyledText::new(text.clone());
                    self.layouts.push(text.layout().clone());
                    div().absolute().top(px(index as f32 * 40.)).child(text)
                })
                .collect::<Vec<_>>();
            div().size_full().children(children)
        }
    }

    impl FakeRegion {
        fn new(text: &str, cx: &mut gpui::App) -> Self {
            let region = TextSelectionRegion::new(text, cx);
            Self { region }
        }

        fn register(
            &self,
            host: &mut TextSelectionState,
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

    fn laid_out_runs(texts: &[&str], cx: &mut TestAppContext) -> Vec<(SharedString, TextLayout)> {
        let texts = texts
            .iter()
            .map(|text| SharedString::from(*text))
            .collect::<Vec<_>>();
        let view = cx.add_window({
            let texts = texts.clone();
            move |_, _| PlainRunLayoutView {
                texts,
                layouts: Vec::new(),
            }
        });
        cx.update_window(*view, |_, window, cx| {
            let _ = window.draw(cx);
        })
        .unwrap();
        let layouts = cx.update(|cx| view.read(cx).unwrap().layouts.clone());
        texts.into_iter().zip(layouts).collect()
    }

    fn plain_snapshot(anchor: Point<Pixels>, cursor: Point<Pixels>) -> SelectionSnapshot {
        SelectionSnapshot {
            anchor: SelectionEndpointSnapshot {
                region_id: None,
                point: anchor,
                virtual_key: None,
            },
            cursor: SelectionEndpointSnapshot {
                region_id: None,
                point: cursor,
                virtual_key: None,
            },
            is_selecting: false,
            coverage: SelectionRegionCoverage::Bounded,
            resolved_points: Some((anchor, cursor)),
        }
    }

    #[gpui::test]
    fn selection_callback_can_reenter_its_host(cx: &mut TestAppContext) {
        let called = Rc::new(Cell::new(false));
        let called_from_callback = called.clone();
        cx.update(|cx| {
            let host = cx.new(|_| TextSelectionState::default());
            let host_for_callback = host.clone();
            let region = FakeRegion::new("region", cx);
            region.region.state().update(cx, |state, _| {
                state.on_selection(move |snapshot, cx| {
                    if snapshot.is_some() {
                        host_for_callback.update(cx, |_, _| called_from_callback.set(true));
                    }
                });
            });
            host.update(cx, |host, cx| {
                region.register(host, 0., SelectionScopeId::default(), 0, cx);
                host.begin(point(px(1.), px(1.)), false, cx);
                host.update(point(px(20.), px(1.)), cx);
            });
        });
        cx.run_until_parked();
        assert!(called.get());
    }

    #[gpui::test]
    fn deferred_snapshot_cannot_overtake_a_synchronous_clear(cx: &mut TestAppContext) {
        let observed = Rc::new(RefCell::new(Vec::new()));
        let observed_for_callback = observed.clone();
        cx.update(|cx| {
            let region = TextSelectionRegion::new("region", cx);
            region.state().update(cx, |state, cx| {
                state.on_selection(move |snapshot, _| {
                    observed_for_callback.borrow_mut().push(snapshot.is_some());
                });
                state.set_snapshot(
                    Some(plain_snapshot(point(px(1.), px(1.)), point(px(8.), px(1.)))),
                    cx,
                );
                let callbacks = state.clear_state();
                TextSelectionRegionState::dispatch_clear(callbacks, cx);
            });
        });
        cx.run_until_parked();
        assert_eq!(&*observed.borrow(), &[false]);
    }

    fn run_frame(order: u64, text: SharedString, layout: TextLayout) -> SelectionRunFrame {
        SelectionRunFrame {
            order,
            text,
            bounds: layout.bounds(),
            layout,
        }
    }

    #[gpui::test]
    fn plain_projection_preserves_forward_reversed_and_unicode_ranges(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["aé🙂z"], cx).pop().unwrap();
        let run = run_frame(0, text, layout.clone());
        let start = layout.position_for_index(1).unwrap();
        let end = layout.position_for_index(7).unwrap();

        let forward = project_selection_runs(Some(plain_snapshot(start, end)), &[run.clone()]);
        let reversed = project_selection_runs(Some(plain_snapshot(end, start)), &[run]);

        assert_eq!(forward[0].byte_range, Some(1..7));
        assert_eq!(reversed[0].byte_range, Some(1..7));
        assert!(forward[0].active);
        assert!(reversed[0].active);
    }

    #[gpui::test]
    fn plain_projection_spans_multiple_runs_and_leaves_empty_gutters_unselected(
        cx: &mut TestAppContext,
    ) {
        let mut runs = laid_out_runs(&["first", "", "second"], cx);
        let (first_text, first_layout) = runs.remove(0);
        let (gutter_text, gutter_layout) = runs.remove(0);
        let (second_text, second_layout) = runs.remove(0);
        let start = first_layout.position_for_index(2).unwrap();
        let end = second_layout.position_for_index(3).unwrap();
        let states = project_selection_runs(
            Some(plain_snapshot(start, end)),
            &[
                run_frame(2, second_text, second_layout),
                run_frame(1, gutter_text, gutter_layout),
                run_frame(0, first_text, first_layout),
            ],
        );

        assert_eq!(states[0].byte_range, Some(0..3));
        assert_eq!(states[1].byte_range, None);
        assert_eq!(states[2].byte_range, Some(2..5));
        assert!(states.iter().all(|state| state.active));
    }

    #[gpui::test]
    fn plain_projection_caches_multiple_region_copies_in_document_order(cx: &mut TestAppContext) {
        let mut runs = laid_out_runs(&["one", "two"], cx);
        let (first_text, first_layout) = runs.remove(0);
        let (second_text, second_layout) = runs.remove(0);
        let snapshot = plain_snapshot(
            first_layout.position_for_index(1).unwrap(),
            second_layout.position_for_index(2).unwrap(),
        );
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let first = FakeRegion::new("", cx);
            let second = FakeRegion::new("", cx);
            first.register(&mut host, 0., SelectionScopeId::default(), 1, cx);
            second.register(&mut host, 20., SelectionScopeId::default(), 0, cx);

            first.region.state().update(cx, |state, cx| {
                state.set_snapshot(Some(snapshot), cx);
                assert_eq!(
                    state.project_selection_runs(&[run_frame(0, first_text, first_layout)]),
                    vec![SelectionRunState {
                        byte_range: Some(1..3),
                        active: true,
                    }]
                );
            });
            second.region.state().update(cx, |state, cx| {
                state.set_snapshot(Some(snapshot), cx);
                assert_eq!(
                    state.project_selection_runs(&[run_frame(0, second_text, second_layout)]),
                    vec![SelectionRunState {
                        byte_range: Some(0..2),
                        active: true,
                    }]
                );
            });

            assert_eq!(host.selected_text(cx), "tw\nne");
        });
    }

    #[gpui::test]
    fn plain_projection_invalidates_cached_copy_when_the_snapshot_changes(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["first"], cx).pop().unwrap();
        let first_snapshot = plain_snapshot(
            layout.position_for_index(1).unwrap(),
            layout.position_for_index(3).unwrap(),
        );
        let changed_snapshot = plain_snapshot(
            layout.position_for_index(3).unwrap(),
            layout.position_for_index(5).unwrap(),
        );
        let run = run_frame(0, text, layout);
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("", cx);
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            region.region.state().update(cx, |state, cx| {
                state.set_snapshot(Some(first_snapshot), cx);
                state.project_selection_runs(&[run.clone()]);
            });
            assert_eq!(host.selected_text(cx), "ir");

            region.region.state().update(cx, |state, cx| {
                state.set_snapshot(Some(changed_snapshot), cx);
            });
            assert_eq!(host.selected_text(cx), "");

            region.region.state().update(cx, |state, _| {
                state.project_selection_runs(&[run]);
            });
            assert_eq!(host.selected_text(cx), "st");
            host.clear(cx);
            region
                .region
                .state()
                .update(cx, |state, _| state.set_local_selection(true));
            assert_eq!(host.selected_text(cx), "");
        });
    }

    #[gpui::test]
    fn plain_projection_orders_cached_runs_by_frame_order_not_input_order(cx: &mut TestAppContext) {
        let mut runs = laid_out_runs(&["one", "two"], cx);
        let (first_text, first_layout) = runs.remove(0);
        let (second_text, second_layout) = runs.remove(0);
        let snapshot = plain_snapshot(
            first_layout.position_for_index(1).unwrap(),
            second_layout.position_for_index(2).unwrap(),
        );
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("", cx);
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            region.region.state().update(cx, |state, cx| {
                state.set_snapshot(Some(snapshot), cx);
                state.project_selection_runs(&[
                    run_frame(1, first_text, first_layout),
                    run_frame(0, second_text, second_layout),
                ]);
            });

            assert_eq!(host.selected_text(cx), "twne");
        });
    }

    #[gpui::test]
    fn plain_projection_safely_rejects_a_text_layout_length_mismatch(cx: &mut TestAppContext) {
        let (_, layout) = laid_out_runs(&["short"], cx).pop().unwrap();
        let start = layout.position_for_index(0).unwrap();
        let end = layout.position_for_index(5).unwrap();
        let states = project_selection_runs(
            Some(plain_snapshot(start, end)),
            &[run_frame(0, SharedString::from("longer"), layout)],
        );

        assert_eq!(
            states,
            vec![SelectionRunState {
                byte_range: None,
                active: true,
            }]
        );
    }

    #[gpui::test]
    fn begin_update_and_end_publish_a_cross_region_selection(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
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
            let mut host = TextSelectionState::default();
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
            assert!(reversed.cursor.point.x < reversed.anchor.point.x);
        });
    }

    #[gpui::test]
    fn virtual_key_callback_runs_outside_the_window_state_lease(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = cx.new(|_| TextSelectionState::default());
            let region = FakeRegion::new("virtual", cx);
            let state_for_callback = state.clone();
            region.region.state().update(cx, |region, _| {
                region.on_virtual_key(move |_, cx| {
                    let _ = state_for_callback.read(cx).snapshot();
                    Some(7)
                });
            });
            state.update(cx, |state, cx| {
                region.register(state, 0., SelectionScopeId::default(), 0, cx);
                state.begin(point(px(1.), px(1.)), false, cx);
                state.update(point(px(8.), px(1.)), cx);
            });

            TextSelectionState::resolve_virtual_keys(&state, cx);

            assert_eq!(
                state.read(cx).snapshot().unwrap().cursor.virtual_key(),
                Some(7)
            );
        });
    }

    #[gpui::test]
    fn active_dnd_does_not_move_a_text_selection_cursor(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| WindowRegionView {
            region: TextSelectionRegion::new("unused", cx),
        });
        window
            .update(cx, |_, window, cx| {
                let mut state = TextSelectionState::default();
                let region = FakeRegion::new("region", cx);
                region.register(&mut state, 0., SelectionScopeId::default(), 0, cx);
                state.begin(point(px(1.), px(1.)), false, cx);
                let before = state.cursor.as_ref().unwrap().point;
                state.update_in_window_with_active_drag(point(px(80.), px(1.)), true, window, cx);
                assert_eq!(state.cursor.as_ref().unwrap().point, before);
            })
            .unwrap();
    }

    #[gpui::test]
    fn shift_extension_falls_back_when_the_anchor_region_was_swept(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let first = FakeRegion::new("first", cx);
            let second = FakeRegion::new("second", cx);
            first.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(8.), px(1.)), cx);
            host.end(cx);

            host.finish_frame(cx);
            host.finish_frame(cx);
            second.register(&mut host, 20., SelectionScopeId::default(), 1, cx);
            host.begin(point(px(1.), px(21.)), true, cx);
            host.update(point(px(8.), px(21.)), cx);
            host.end(cx);

            assert_eq!(host.selected_text(cx), "second");
        });
    }

    #[gpui::test]
    fn scope_and_suppression_prevent_unrelated_regions_from_participating(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let base = FakeRegion::new("base", cx);
            let modal = FakeRegion::new("modal", cx);
            base.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            modal.register(&mut host, 20., SelectionScopeId(1), 1, cx);

            host.set_active_scope(SelectionScopeId(1), cx);
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
            let host = cx.new(|_| TextSelectionState::default());
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
            let host = TextSelectionState::ensure(window, cx);
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

    #[gpui::test]
    fn cross_region_selection_excludes_regions_outside_its_document_interval(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let first = FakeRegion::new("first", cx);
            let second = FakeRegion::new("second", cx);
            let third = FakeRegion::new("third", cx);
            first.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            second.register(&mut host, 20., SelectionScopeId::default(), 1, cx);
            third.register(&mut host, 40., SelectionScopeId::default(), 2, cx);

            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(1.), px(25.)), cx);
            host.end(cx);

            assert_eq!(host.selected_text(cx), "first\nsecond");
            assert!(third.region.state().read(cx).snapshot().is_none());
        });
    }

    #[gpui::test]
    fn changing_scope_clears_the_previous_scope_selection(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let base = FakeRegion::new("base", cx);
            let modal = FakeRegion::new("modal", cx);
            base.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            modal.register(&mut host, 20., SelectionScopeId::new(1), 1, cx);

            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(8.), px(1.)), cx);
            host.end(cx);
            host.set_active_scope(SelectionScopeId::new(1), cx);

            assert!(!host.has_text_selection(cx));
            assert!(base.region.state().read(cx).snapshot().is_none());
        });
    }

    #[gpui::test]
    fn blank_only_drag_never_publishes_or_copies_selection(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("region", cx);
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);

            host.begin(point(px(200.), px(1.)), false, cx);
            host.update(point(px(200.), px(8.)), cx);
            host.end(cx);

            assert!(!host.has_text_selection(cx));
            assert_eq!(host.selected_text(cx), "");
            assert!(region.region.state().read(cx).snapshot().is_none());
        });
    }

    #[gpui::test]
    fn stale_live_regions_are_removed_when_the_next_frame_begins(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("stale", cx);
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(8.), px(1.)), cx);
            host.end(cx);

            host.finish_frame(cx);
            host.finish_frame(cx);
            assert_eq!(host.selected_text(cx), "");
            assert!(region.region.state().read(cx).snapshot().is_none());
        });
    }

    #[gpui::test]
    fn clear_stops_anchor_auto_scroll_before_discarding_the_anchor(cx: &mut TestAppContext) {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let observed = commands.clone();
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("scroll", cx);
            region.region.state().update(cx, |state, _| {
                state.on_auto_scroll(move |delta, _| observed.borrow_mut().push(delta));
            });
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);

            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(1.), px(25.)), cx);
            host.clear(cx);
        });
        cx.run_until_parked();
        assert!(commands.borrow().iter().any(Option::is_some));
        assert_eq!(commands.borrow().last(), Some(&None));
    }

    #[gpui::test]
    fn proxy_endpoints_break_equal_position_ties_by_document_order(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let later = FakeRegion::new("later", cx);
            let earlier = FakeRegion::new("earlier", cx);
            later.register(&mut host, 0., SelectionScopeId::default(), 2, cx);
            earlier.register(&mut host, 0., SelectionScopeId::default(), 1, cx);

            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(200.), px(25.)), cx);
            let endpoint = host.snapshot().unwrap().cursor;

            assert_eq!(endpoint.region_id, Some(earlier.region.state().entity_id()));
        });
    }

    #[gpui::test]
    fn window_extension_is_a_safe_no_op_until_a_host_is_explicitly_installed(
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = cx.add_window_view(|_, cx| WindowRegionView {
            region: TextSelectionRegion::new("not installed", cx),
        });
        cx.update(|window, cx| {
            assert!(!window.has_text_selection(cx));
            assert_eq!(window.selected_text(cx), "");
            window.clear_text_selection(cx);
            window.end_text_selection(cx);
            assert!(!window.has_text_selection(cx));
        });
    }

    #[gpui::test]
    fn controller_initializes_suppression_before_capture_and_respects_bubble_suppression(
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = cx.add_window_view(|_, _| ControllerOnlyView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_mouse_down(
            point(px(1.), px(1.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(1.), px(1.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.update(|window, cx| {
            assert!(GlobalState::is_text_selection_suppressed(cx));
            assert!(!window.has_text_selection(cx));
        });
    }

    #[gpui::test]
    fn frame_sweep_keeps_a_region_registered_before_the_controller_paints(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("painted first", cx);
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(8.), px(1.)), cx);
            host.end(cx);

            host.finish_frame(cx);

            assert_eq!(host.selected_text(cx), "painted first");
            assert!(region.region.state().read(cx).snapshot().is_some());
        });
    }

    #[gpui::test]
    fn two_controllers_schedule_only_one_post_frame_sweep(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| DoubleControllerView {
            region: TextSelectionRegion::new("once", cx),
        });
        cx.update(|window, cx| {
            let host = TextSelectionState::ensure(window, cx);
            let region = view.read(cx).region.clone();
            host.update(cx, |host, cx| {
                FakeRegion { region }.register(host, 0., SelectionScopeId::default(), 0, cx);
                host.begin(point(px(1.), px(1.)), false, cx);
                host.update(point(px(8.), px(1.)), cx);
                host.end(cx);
            });

            let _ = window.draw(cx);
            window.simulate_next_frame(cx);

            assert_eq!(host.read(cx).selected_text(cx), "once");
        });
    }
}
