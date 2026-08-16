use std::{cell::Cell, collections::HashMap, ops::Range, rc::Rc};

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
    region_id: Option<EntityId>,
    point: Point<Pixels>,
    virtual_key: Option<u64>,
}

impl SelectionEndpointSnapshot {
    /// Creates an endpoint at a region-relative content point.
    pub const fn new(region_id: Option<EntityId>, point: Point<Pixels>) -> Self {
        Self {
            region_id,
            point,
            virtual_key: None,
        }
    }

    /// Sets renderer-defined endpoint metadata.
    pub const fn with_virtual_key(mut self, virtual_key: u64) -> Self {
        self.virtual_key = Some(virtual_key);
        self
    }

    /// Returns the region which owns this endpoint, when it hit one.
    pub const fn region_id(&self) -> Option<EntityId> {
        self.region_id
    }

    /// Returns the region-relative content point.
    pub const fn point(&self) -> Point<Pixels> {
        self.point
    }

    /// Returns renderer-defined endpoint metadata captured when it hit a region.
    pub const fn virtual_key(&self) -> Option<u64> {
        self.virtual_key
    }
}

/// Region-relative selection endpoints with an optional rendering projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionSnapshot {
    anchor: SelectionEndpointSnapshot,
    cursor: SelectionEndpointSnapshot,
    is_selecting: bool,
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
    /// Creates a snapshot from stable region-relative endpoints.
    pub const fn new(anchor: SelectionEndpointSnapshot, cursor: SelectionEndpointSnapshot) -> Self {
        Self {
            anchor,
            cursor,
            is_selecting: false,
            resolved_points: None,
            coverage: SelectionRegionCoverage::Bounded,
        }
    }

    /// Sets whether the pointer gesture is still active.
    pub const fn with_selecting(mut self, is_selecting: bool) -> Self {
        self.is_selecting = is_selecting;
        self
    }

    /// Sets the current window-coordinate rendering projection.
    pub const fn with_resolved_points(
        mut self,
        resolved_points: Option<(Point<Pixels>, Point<Pixels>)>,
    ) -> Self {
        self.resolved_points = resolved_points;
        self
    }

    /// Sets the portion of the receiving region covered by this selection.
    pub const fn with_region_coverage(mut self, coverage: SelectionRegionCoverage) -> Self {
        self.coverage = coverage;
        self
    }

    /// Returns the stable anchor endpoint.
    pub const fn anchor(&self) -> SelectionEndpointSnapshot {
        self.anchor
    }

    /// Returns the moving cursor endpoint.
    pub const fn cursor(&self) -> SelectionEndpointSnapshot {
        self.cursor
    }

    /// Returns whether the pointer gesture is still active.
    pub const fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    /// Returns the window-coordinate endpoints for renderers that need them.
    pub const fn resolved_points(&self) -> Option<(Point<Pixels>, Point<Pixels>)> {
        self.resolved_points
    }

    /// Returns the portion of the receiving region covered by this selection.
    pub const fn region_coverage(&self) -> SelectionRegionCoverage {
        self.coverage
    }
}

/// Per-frame geometry reported by a [`SelectableText`] participant.
pub struct SelectableTextFrame {
    hitbox: Hitbox,
    bounds: Bounds<Pixels>,
    scroll_offset: Point<Pixels>,
    scope: SelectionScopeId,
    document_order: u64,
    text_bounds: Vec<Bounds<Pixels>>,
}

impl SelectableTextFrame {
    /// Creates a region frame with default scope, order, and scroll offset.
    pub fn new(hitbox: Hitbox, bounds: Bounds<Pixels>) -> Self {
        Self {
            hitbox,
            bounds,
            scroll_offset: Point::default(),
            scope: SelectionScopeId::default(),
            document_order: 0,
            text_bounds: Vec::new(),
        }
    }

    /// Sets the region's content scroll offset.
    pub fn with_scroll_offset(mut self, scroll_offset: Point<Pixels>) -> Self {
        self.scroll_offset = scroll_offset;
        self
    }

    /// Sets the opaque selection scope.
    pub fn with_scope(mut self, scope: SelectionScopeId) -> Self {
        self.scope = scope;
        self
    }

    /// Sets the stable logical document order.
    pub fn with_document_order(mut self, document_order: u64) -> Self {
        self.document_order = document_order;
        self
    }

    /// Sets the glyph-bearing bounds used to reject blank-only gestures.
    pub fn with_text_bounds(mut self, text_bounds: Vec<Bounds<Pixels>>) -> Self {
        self.text_bounds = text_bounds;
        self
    }

    /// Returns the region hitbox.
    pub fn hitbox(&self) -> &Hitbox {
        &self.hitbox
    }

    /// Returns the region's window-coordinate bounds.
    pub const fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Returns the region's content scroll offset.
    pub const fn scroll_offset(&self) -> Point<Pixels> {
        self.scroll_offset
    }

    /// Returns the opaque selection scope.
    pub const fn scope(&self) -> SelectionScopeId {
        self.scope
    }

    /// Returns the stable logical document order.
    pub const fn document_order(&self) -> u64 {
        self.document_order
    }

    /// Returns the glyph-bearing bounds used to reject blank-only gestures.
    pub fn text_bounds(&self) -> &[Bounds<Pixels>] {
        &self.text_bounds
    }
}

/// Per-frame text layout reported by a plain selectable region.
#[derive(Clone)]
pub struct SelectionRunFrame {
    /// Logical order within the containing region.
    order: u64,
    /// The exact text used to produce `layout`.
    text: SharedString,
    /// Laid-out glyph geometry in window coordinates.
    layout: TextLayout,
    /// The run's window-coordinate paint bounds.
    bounds: Bounds<Pixels>,
}

impl SelectionRunFrame {
    /// Creates a laid-out text run frame.
    pub fn new(
        order: u64,
        text: impl Into<SharedString>,
        layout: TextLayout,
        bounds: Bounds<Pixels>,
    ) -> Self {
        Self {
            order,
            text: text.into(),
            layout,
            bounds,
        }
    }

    /// Returns the run's logical order within its region.
    pub const fn order(&self) -> u64 {
        self.order
    }

    /// Returns the exact text used to produce the layout.
    pub fn text(&self) -> &SharedString {
        &self.text
    }

    /// Returns the laid-out glyph geometry.
    pub fn layout(&self) -> &TextLayout {
        &self.layout
    }

    /// Returns the run's window-coordinate paint bounds.
    pub const fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
}

/// Selection projection for one [`SelectionRunFrame`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionRunState {
    /// The selected UTF-8 byte range in the run's text.
    byte_range: Option<Range<usize>>,
    /// Whether the containing region participates in the current selection.
    active: bool,
}

impl SelectionRunState {
    /// Creates an inactive run projection with no selected range.
    pub const fn new() -> Self {
        Self {
            byte_range: None,
            active: false,
        }
    }

    /// Sets the selected UTF-8 byte range.
    pub fn with_byte_range(mut self, byte_range: Option<Range<usize>>) -> Self {
        self.byte_range = byte_range;
        self
    }

    /// Sets whether the containing region participates in the selection.
    pub const fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Returns the selected UTF-8 byte range.
    pub fn byte_range(&self) -> Option<&Range<usize>> {
        self.byte_range.as_ref()
    }

    /// Returns whether the containing region participates in the selection.
    pub const fn is_active(&self) -> bool {
        self.active
    }
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
            .map(|_| SelectionRunState::new().with_active(true))
            .collect();
    };

    runs.iter()
        .map(|run| {
            SelectionRunState::new()
                .with_byte_range(selection_range_for_run(run, anchor, cursor))
                .with_active(true)
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
type RegionCopyCallback = Rc<dyn Fn(&mut App) -> String>;
type RegionVirtualKeyCallback = Rc<dyn Fn(Point<Pixels>, &App) -> Option<u64>>;
type RegionClearCallbacks = (Option<RegionVoidCallback>, Option<RegionSelectionCallback>);

struct RegionCopyItem {
    document_order: u64,
    callback: Option<RegionCopyCallback>,
    fallback: String,
}

fn resolve_copy_items(mut items: Vec<RegionCopyItem>, cx: &mut App) -> String {
    items.sort_by_key(|item| item.document_order);
    items
        .into_iter()
        .map(|item| {
            item.callback
                .map(|callback| callback(cx))
                .unwrap_or(item.fallback)
        })
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn dispatch_clear_callbacks(callbacks: Vec<RegionClearCallbacks>, cx: &mut App) {
    for callbacks in callbacks {
        SelectableTextState::dispatch_clear(callbacks, cx);
    }
}

struct SelectableTextState {
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

impl SelectableTextState {
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
    pub fn copy_with(&mut self, callback: impl Fn(&mut App) -> String + 'static) {
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

    fn copy_item(&self, document_order: u64) -> Option<RegionCopyItem> {
        (self.snapshot.is_some() || self.local_selection).then(|| RegionCopyItem {
            document_order,
            callback: self.copy.clone(),
            fallback: self
                .projected_selected_text
                .clone()
                .unwrap_or_else(|| self.selected_text.clone()),
        })
    }
}

/// A stable, renderer-neutral handle for text that participates in window selection.
#[derive(Clone)]
pub struct SelectableText(Entity<SelectableTextState>);

impl SelectableText {
    /// Creates selectable text with a default copy string.
    pub fn new(selected_text: impl Into<String>, cx: &mut App) -> Self {
        Self(cx.new(|_| SelectableTextState::new(selected_text)))
    }

    /// Returns this region's stable identity.
    pub fn entity_id(&self) -> EntityId {
        self.0.entity_id()
    }

    /// Returns the current geometry selection snapshot for this region.
    pub fn snapshot(&self, cx: &App) -> Option<SelectionSnapshot> {
        self.0.read(cx).snapshot()
    }

    /// Sets the fallback text copied while this region participates.
    pub fn set_selected_text(&self, text: impl Into<String>, cx: &mut App) {
        self.0.update(cx, |state, _| state.set_selected_text(text));
    }

    /// Marks renderer-local selection, such as select-all, as active.
    pub fn set_local_selection(&self, active: bool, cx: &mut App) {
        self.0
            .update(cx, |state, _| state.set_local_selection(active));
    }

    /// Returns whether renderer-local selection is active.
    pub fn has_local_selection(&self, cx: &App) -> bool {
        self.0.read(cx).local_selection
    }

    /// Projects the current snapshot onto plain-text runs and caches their copy text.
    pub fn project_selection_runs(
        &self,
        runs: &[SelectionRunFrame],
        cx: &mut App,
    ) -> Vec<SelectionRunState> {
        self.0
            .update(cx, |state, _| state.project_selection_runs(runs))
    }

    /// Sets the callback which updates renderer highlights from window state.
    pub fn on_selection(
        &self,
        callback: impl Fn(Option<SelectionSnapshot>, &mut App) + 'static,
        cx: &mut App,
    ) {
        self.0.update(cx, |state, _| state.on_selection(callback));
    }

    /// Sets the callback which receives drag auto-scroll commands.
    pub fn on_auto_scroll(
        &self,
        callback: impl Fn(Option<Pixels>, &mut App) + 'static,
        cx: &mut App,
    ) {
        self.0.update(cx, |state, _| state.on_auto_scroll(callback));
    }

    /// Sets the callback which focuses the region when a drag begins in it.
    pub fn on_focus(&self, callback: impl Fn(&mut Window, &mut App) + 'static, cx: &mut App) {
        self.0.update(cx, |state, _| state.on_focus(callback));
    }

    /// Sets renderer-local cleanup invoked when window selection is cleared.
    pub fn on_clear(&self, callback: impl Fn(&mut App) + 'static, cx: &mut App) {
        self.0.update(cx, |state, _| state.on_clear(callback));
    }

    /// Sets a renderer-specific copy projection.
    pub fn copy_with(&self, callback: impl Fn(&mut App) -> String + 'static, cx: &mut App) {
        self.0.update(cx, |state, _| state.copy_with(callback));
    }

    /// Sets a renderer-specific lookup for stable virtualized content keys.
    pub fn on_virtual_key(
        &self,
        callback: impl Fn(Point<Pixels>, &App) -> Option<u64> + 'static,
        cx: &mut App,
    ) {
        self.0.update(cx, |state, _| state.on_virtual_key(callback));
    }

    fn downgrade(&self) -> WeakEntity<SelectableTextState> {
        self.0.downgrade()
    }
}

#[derive(Clone)]
struct RegionRegistration {
    region: WeakEntity<SelectableTextState>,
    frame: Rc<SelectableTextFrame>,
    generation: u64,
}

#[derive(Clone)]
struct SelectionEndpoint {
    region: Option<WeakEntity<SelectableTextState>>,
    point: Point<Pixels>,
    inside: bool,
    inside_text: bool,
    virtual_key: Option<u64>,
    virtual_resolver: Option<(RegionVirtualKeyCallback, Point<Pixels>)>,
}

impl SelectionEndpoint {
    fn snapshot(&self) -> SelectionEndpointSnapshot {
        let snapshot = SelectionEndpointSnapshot::new(self.region_id(), self.point);
        if let Some(virtual_key) = self.virtual_key {
            snapshot.with_virtual_key(virtual_key)
        } else {
            snapshot
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
#[derive(Default)]
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
    fn acquire(window_id: gpui::WindowId, cx: &mut App) -> Entity<Self> {
        if !cx.has_global::<WindowTextSelections>() {
            cx.set_global(WindowTextSelections::default());
        }
        if let Some(state) = cx
            .global::<WindowTextSelections>()
            .0
            .get(&window_id)
            .and_then(WeakEntity::upgrade)
        {
            return state;
        }

        let state = cx.new(|cx| {
            let entity_id = cx.entity_id();
            cx.on_release(move |state: &mut TextSelectionState, cx| {
                let callbacks = state.clear_state(cx);
                if cx.has_global::<WindowTextSelections>() {
                    let registry = &mut cx.global_mut::<WindowTextSelections>().0;
                    if registry
                        .get(&window_id)
                        .is_some_and(|state| state.entity_id() == entity_id)
                    {
                        registry.remove(&window_id);
                    }
                }
                if !callbacks.is_empty() {
                    cx.defer(move |cx| dispatch_clear_callbacks(callbacks, cx));
                }
            })
            .detach();
            Self::default()
        });
        cx.global_mut::<WindowTextSelections>()
            .0
            .insert(window_id, state.downgrade());
        state
    }

    #[cfg(test)]
    fn ensure(window: &Window, cx: &mut App) -> Entity<Self> {
        Self::acquire(window.window_handle().window_id(), cx)
    }

    fn existing(window: &Window, cx: &App) -> Option<Entity<Self>> {
        if !cx.has_global::<WindowTextSelections>() {
            return None;
        }
        cx.global::<WindowTextSelections>()
            .0
            .get(&window.window_handle().window_id())
            .and_then(WeakEntity::upgrade)
    }

    /// Updates the active scope. Regions from other scopes cannot participate.
    #[cfg(test)]
    fn set_active_scope(&mut self, scope: SelectionScopeId, cx: &mut App) {
        let callbacks = self.set_active_scope_state(scope, cx);
        dispatch_clear_callbacks(callbacks, cx);
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
    /// whether a region or the lifecycle element paints first.
    pub fn finish_frame(&mut self, cx: &mut App) -> Vec<RegionClearCallbacks> {
        self.finish_frame_scheduled = false;
        let stale = self
            .regions
            .iter()
            .filter_map(|(id, registration)| {
                (registration.generation != self.frame_generation)
                    .then(|| (*id, registration.region.clone()))
            })
            .collect::<Vec<_>>();
        let mut callbacks = Vec::new();
        for (id, region) in stale {
            self.regions.remove(&id);
            if let Some(region) = region.upgrade() {
                callbacks.push(region.update(cx, |state, _| state.clear_state()));
            }
        }
        self.publish_snapshots(cx);
        self.frame_generation = self.frame_generation.wrapping_add(1);
        callbacks
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
        region: SelectableText,
        frame: SelectableTextFrame,
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
        self.begin_impl(position, extend, false, None, cx);
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
        dispatch_clear_callbacks(callbacks, cx);
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

    fn copy_items(&self, cx: &App) -> Vec<RegionCopyItem> {
        self.regions
            .values()
            .filter_map(|registration| {
                let region = registration.region.upgrade()?;
                region.read(cx).copy_item(registration.frame.document_order)
            })
            .collect()
    }

    #[cfg(test)]
    fn selected_text(&self, cx: &mut App) -> String {
        resolve_copy_items(self.copy_items(cx), cx)
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
        let anchor_endpoint = self.anchor.as_ref()?;
        let cursor_endpoint = self.cursor.as_ref()?;
        let anchor = anchor_endpoint.resolve(&self.regions)?;
        let cursor = cursor_endpoint.resolve(&self.regions)?;
        (anchor != cursor).then(|| {
            SelectionSnapshot::new(anchor_endpoint.snapshot(), cursor_endpoint.snapshot())
                .with_selecting(self.is_selecting)
                .with_resolved_points(Some((anchor, cursor)))
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
        self.begin_impl(position, extend, true, Some(window), cx);
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
        already_prepared: bool,
        window: Option<&mut Window>,
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
        if !extend && !already_prepared {
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
            if let Some(window) = window {
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
            WeakEntity<SelectableTextState>,
            Rc<SelectableTextFrame>,
            f32,
        )> = None;
        let mut predecessor: Option<(WeakEntity<SelectableTextState>, Rc<SelectableTextFrame>)> =
            None;
        let mut first: Option<(WeakEntity<SelectableTextState>, Rc<SelectableTextFrame>)> = None;

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
                if hit.as_ref().is_none_or(|(_, best, best_area)| {
                    area < *best_area
                        || (area == *best_area && frame.document_order < best.document_order)
                }) {
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
/// Non-owning window locator; retained [`TextSelection`] element state owns
/// each live selection entity.
struct WindowTextSelections(HashMap<gpui::WindowId, WeakEntity<TextSelectionState>>);

impl Global for WindowTextSelections {}

#[derive(Default)]
struct TextSelectionScopeStacks(HashMap<gpui::WindowId, Vec<SelectionScopeId>>);

impl Global for TextSelectionScopeStacks {}

fn push_text_selection_scope(window_id: gpui::WindowId, scope: SelectionScopeId, cx: &mut App) {
    if !cx.has_global::<TextSelectionScopeStacks>() {
        cx.set_global(TextSelectionScopeStacks::default());
    }
    cx.global_mut::<TextSelectionScopeStacks>()
        .0
        .entry(window_id)
        .or_default()
        .push(scope);
}

fn pop_text_selection_scope(window_id: gpui::WindowId, cx: &mut App) {
    let stacks = &mut cx.global_mut::<TextSelectionScopeStacks>().0;
    let remove_stack = stacks.get_mut(&window_id).is_some_and(|stack| {
        stack.pop();
        stack.is_empty()
    });
    if remove_stack {
        stacks.remove(&window_id);
    }
}

fn current_text_selection_scope(window_id: gpui::WindowId, cx: &App) -> Option<SelectionScopeId> {
    cx.has_global::<TextSelectionScopeStacks>()
        .then(|| {
            cx.global::<TextSelectionScopeStacks>()
                .0
                .get(&window_id)
                .and_then(|stack| stack.last().copied())
        })
        .flatten()
}

fn with_text_selection_scope<T>(
    window_id: gpui::WindowId,
    scope: SelectionScopeId,
    cx: &mut App,
    callback: impl FnOnce(&mut App) -> T,
) -> T {
    push_text_selection_scope(window_id, scope, cx);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(cx)));
    pop_text_selection_scope(window_id, cx);
    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// A zero-sized element which enables text selection for a window root.
///
/// Mount one as the root's first child. Its stable `"window-text-selection"`
/// element identity retains the window-local selection entity across frames;
/// additional instances share that identity and are unnecessary.
pub struct TextSelection;

impl TextSelection {
    /// Constructs the root element and binds its window state immediately.
    ///
    /// Use this form when a custom root calls [`WindowTextSelection`] methods
    /// during the same render. Insert the returned element as the first child.
    /// A root which only registers regions during child prepaint or paint may
    /// use the unit `TextSelection` value instead.
    pub fn new(window: &Window, cx: &mut App) -> impl IntoElement + use<> {
        PreboundTextSelection {
            state: TextSelectionState::acquire(window.window_handle().window_id(), cx),
        }
    }

    /// Marks an element subtree as belonging to `scope`.
    pub fn scope(scope: SelectionScopeId, element: impl IntoElement) -> impl IntoElement {
        TextSelectionScopeMarker {
            scope,
            element: element.into_element(),
        }
    }
}

struct TextSelectionScopeMarker<E> {
    scope: SelectionScopeId,
    element: E,
}

impl<E: Element> IntoElement for TextSelectionScopeMarker<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: Element> Element for TextSelectionScopeMarker<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let window_id = window.window_handle().window_id();
        with_text_selection_scope(window_id, self.scope, cx, |cx| {
            self.element.request_layout(id, inspector_id, window, cx)
        })
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let window_id = window.window_handle().window_id();
        with_text_selection_scope(window_id, self.scope, cx, |cx| {
            self.element
                .prepaint(id, inspector_id, bounds, request_layout, window, cx)
        })
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let window_id = window.window_handle().window_id();
        with_text_selection_scope(window_id, self.scope, cx, |cx| {
            self.element.paint(
                id,
                inspector_id,
                bounds,
                request_layout,
                prepaint,
                window,
                cx,
            );
        });
    }
}

struct PreboundTextSelection {
    state: Entity<TextSelectionState>,
}

#[doc(hidden)]
pub struct TextSelectionPrepaintState(Entity<TextSelectionState>);

impl IntoElement for PreboundTextSelection {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl IntoElement for TextSelection {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextSelection {
    type RequestLayoutState = ();
    type PrepaintState = TextSelectionPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some("window-text-selection".into())
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
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        TextSelectionPrepaintState(retain_text_selection_state(global_id, None, window, cx))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        paint_text_selection(&state.0, window, cx);
    }
}

impl Element for PreboundTextSelection {
    type RequestLayoutState = ();
    type PrepaintState = TextSelectionPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some("window-text-selection".into())
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
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        TextSelectionPrepaintState(retain_text_selection_state(
            global_id,
            Some(&self.state),
            window,
            cx,
        ))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        paint_text_selection(&state.0, window, cx);
    }
}

fn retain_text_selection_state(
    global_id: Option<&GlobalElementId>,
    prebound: Option<&Entity<TextSelectionState>>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<TextSelectionState> {
    let window_id = window.window_handle().window_id();
    let prebound = prebound.cloned();
    let state = window.with_element_state::<Entity<TextSelectionState>, _>(
        global_id.expect("TextSelection has a stable element id"),
        |retained, _| {
            let state = retained
                .or(prebound)
                .unwrap_or_else(|| TextSelectionState::acquire(window_id, cx));
            (state.clone(), state)
        },
    );
    if !cx.has_global::<WindowTextSelections>() {
        cx.set_global(WindowTextSelections::default());
    }
    cx.global_mut::<WindowTextSelections>()
        .0
        .insert(window_id, state.downgrade());
    state
}

fn paint_text_selection(state: &Entity<TextSelectionState>, window: &mut Window, cx: &mut App) {
    if state.update(cx, |state, _| state.schedule_finish_frame()) {
        let state = state.downgrade();
        window.on_next_frame(move |_, cx| {
            let Some(state) = state.upgrade() else {
                return;
            };
            let callbacks = state.update(cx, |state, cx| state.finish_frame(cx));
            dispatch_clear_callbacks(callbacks, cx);
        });
    }

    let mouse_down_state = state.downgrade();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if event.button != MouseButton::Left {
            return;
        }
        let Some(state) = mouse_down_state.upgrade() else {
            return;
        };
        if phase.capture() {
            GlobalState::init(cx);
            GlobalState::reset_text_selection_suppression(cx);
            let callbacks = state.update(cx, |state, cx| {
                if state.mouse_down_prepared {
                    return Vec::new();
                }
                state.mouse_down_prepared = true;
                state.prepare_for_mouse_down(event.click_count == 1 && event.modifiers.shift, cx)
            });
            dispatch_clear_callbacks(callbacks, cx);
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

    let mouse_move_state = state.downgrade();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase.bubble()
            && let Some(state) = mouse_move_state.upgrade()
        {
            state.update(cx, |state, cx| {
                state.update_in_window(event.position, window, cx)
            });
            TextSelectionState::resolve_virtual_keys(&state, cx);
        }
    });

    let mouse_up_state = state.downgrade();
    window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
        if phase.bubble()
            && let Some(state) = mouse_up_state.upgrade()
        {
            state.update(cx, |state, cx| {
                state.mouse_down_prepared = false;
                state.end(cx)
            });
        }
    });

    let scroll_state = state.downgrade();
    window.on_mouse_event(move |_: &ScrollWheelEvent, phase, window, cx| {
        if phase.bubble()
            && let Some(state) = scroll_state.upgrade()
        {
            let position = window.mouse_position();
            state.update(cx, |state, cx| state.update_in_window(position, window, cx));
            TextSelectionState::resolve_virtual_keys(&state, cx);
        }
    });
}

/// Window text-selection operations.
///
/// Add one [`TextSelection`] as the first child of a custom window root. Use
/// [`TextSelection::new`] before same-render scope changes; the unit value is
/// sufficient when registration happens later during child prepaint or paint.
/// Calls made before either form binds the state are safe no-ops.
pub trait WindowTextSelection {
    /// Returns the currently selected text in logical document order.
    fn selected_text(&mut self, cx: &mut App) -> String;

    /// Returns whether a drag or renderer-local selection is active.
    fn has_text_selection(&mut self, cx: &mut App) -> bool;

    /// Clears window selection and every participating region's local selection.
    fn clear_text_selection(&mut self, cx: &mut App);

    /// Ends the current drag while leaving its selection visible.
    fn end_text_selection(&mut self, cx: &mut App);

    /// Sets the active opaque selection scope for this window.
    fn set_text_selection_scope(&mut self, scope: SelectionScopeId, cx: &mut App);

    /// Registers selectable text and its geometry for the current frame.
    fn register_selectable_text(
        &mut self,
        region: SelectableText,
        frame: SelectableTextFrame,
        cx: &mut App,
    );
}

impl WindowTextSelection for Window {
    fn selected_text(&mut self, cx: &mut App) -> String {
        let Some(state) = live_text_selection_state(self, cx) else {
            return String::new();
        };
        let items = state.read(cx).copy_items(cx);
        resolve_copy_items(items, cx)
    }

    fn has_text_selection(&mut self, cx: &mut App) -> bool {
        live_text_selection_state(self, cx)
            .is_some_and(|state| state.read(cx).has_text_selection(cx))
    }

    fn clear_text_selection(&mut self, cx: &mut App) {
        if let Some(state) = live_text_selection_state(self, cx) {
            let callbacks = state.update(cx, |state, cx| state.clear_state(cx));
            dispatch_clear_callbacks(callbacks, cx);
        }
    }

    fn end_text_selection(&mut self, cx: &mut App) {
        if let Some(state) = live_text_selection_state(self, cx) {
            state.update(cx, |state, cx| state.end(cx));
        }
    }

    fn set_text_selection_scope(&mut self, scope: SelectionScopeId, cx: &mut App) {
        let Some(state) = TextSelectionState::existing(self, cx) else {
            return;
        };
        let callbacks = state.update(cx, |state, cx| state.set_active_scope_state(scope, cx));
        dispatch_clear_callbacks(callbacks, cx);
    }

    fn register_selectable_text(
        &mut self,
        region: SelectableText,
        mut frame: SelectableTextFrame,
        cx: &mut App,
    ) {
        if let Some(scope) = current_text_selection_scope(self.window_handle().window_id(), cx) {
            frame.scope = scope;
        }
        let Some(state) = TextSelectionState::existing(self, cx) else {
            return;
        };
        state.update(cx, |state, cx| state.register_region(region, frame, cx));
    }
}

fn live_text_selection_state(window: &Window, cx: &mut App) -> Option<Entity<TextSelectionState>> {
    TextSelectionState::existing(window, cx)
}

pub(crate) fn clear_window_text_selection(window_id: gpui::WindowId, cx: &mut App) {
    if !cx.has_global::<WindowTextSelections>() {
        return;
    }
    let Some(state) = cx
        .global::<WindowTextSelections>()
        .0
        .get(&window_id)
        .and_then(WeakEntity::upgrade)
    else {
        return;
    };
    let callbacks = state.update(cx, |state, cx| state.clear_state(cx));
    dispatch_clear_callbacks(callbacks, cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementExt as _;
    use gpui::{
        Bounds, ContentMask, Context, Hitbox, HitboxBehavior, HitboxId, InteractiveElement as _,
        IntoElement, ParentElement as _, Render, SharedString, Styled as _, StyledText,
        TestAppContext, TextLayout, Window, div, point, prelude::FluentBuilder as _, px, size,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    struct FakeRegion {
        region: SelectableText,
    }

    struct WindowRegionView {
        region: SelectableText,
    }

    struct SelectionElementOnlyView;
    struct ToggleSelectionElementView {
        enabled: bool,
        region: SelectableText,
    }

    struct DoubleSelectionElementView {
        region: SelectableText,
    }

    struct WindowOwnedSelectionView {
        region: SelectableText,
    }

    struct FirstFrameScopedSelectionView {
        region: SelectableText,
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

    impl Render for SelectionElementOnlyView {
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

    impl Render for ToggleSelectionElementView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let region = self.region.clone();
            div().when(self.enabled, |this| {
                this.child(TextSelection)
                    .child(div().size_full().on_prepaint(move |bounds, window, cx| {
                        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                        window.register_selectable_text(
                            region,
                            SelectableTextFrame::new(hitbox, bounds).with_text_bounds(vec![bounds]),
                            cx,
                        );
                    }))
            })
        }
    }

    impl Render for DoubleSelectionElementView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let region = self.region.clone();
            div()
                .size_full()
                .child(TextSelection)
                .child(TextSelection)
                .on_prepaint(move |bounds, window, cx| {
                    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                    window.register_selectable_text(
                        region,
                        SelectableTextFrame::new(hitbox, bounds).with_text_bounds(vec![bounds]),
                        cx,
                    );
                })
        }
    }

    impl Render for WindowOwnedSelectionView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let region = self.region.clone();
            div()
                .size_full()
                .child(TextSelection)
                .child(div().size_full().on_prepaint(move |bounds, window, cx| {
                    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                    window.register_selectable_text(
                        region,
                        SelectableTextFrame::new(hitbox, bounds).with_text_bounds(vec![bounds]),
                        cx,
                    );
                }))
        }
    }

    impl Render for FirstFrameScopedSelectionView {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let selection = TextSelection::new(window, cx);
            let scope = SelectionScopeId::new(23);
            window.set_text_selection_scope(scope, cx);
            let region = self.region.clone();

            div().child(selection).child(TextSelection::scope(
                scope,
                div().size_full().on_prepaint(move |bounds, window, cx| {
                    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                    window.register_selectable_text(
                        region,
                        SelectableTextFrame::new(hitbox, bounds).with_text_bounds(vec![bounds]),
                        cx,
                    );
                }),
            ))
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
            let region = SelectableText::new(text, cx);
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
                SelectableTextFrame::new(
                    Hitbox {
                        id: HitboxId::placeholder(),
                        bounds,
                        content_mask: ContentMask { bounds },
                        behavior: HitboxBehavior::Normal,
                    },
                    bounds,
                )
                .with_scope(scope)
                .with_document_order(document_order)
                .with_text_bounds(vec![bounds]),
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
        SelectionSnapshot::new(
            SelectionEndpointSnapshot::new(None, anchor),
            SelectionEndpointSnapshot::new(None, cursor),
        )
        .with_resolved_points(Some((anchor, cursor)))
    }

    #[gpui::test]
    fn scope_stack_is_cleaned_after_panicking_subtree(cx: &mut TestAppContext) {
        let window_id = {
            let (_, window_cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
            window_cx.update(|window, _| window.window_handle().window_id())
        };
        let scope = SelectionScopeId::new(41);

        cx.update(|cx| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_text_selection_scope(window_id, scope, cx, |_| panic!("subtree failed"));
            }));

            assert!(result.is_err());
            assert_eq!(current_text_selection_scope(window_id, cx), None);
        });
    }

    #[gpui::test]
    fn reentrant_scope_from_one_window_does_not_pollute_another(cx: &mut TestAppContext) {
        let first_window_id = {
            let (_, window_cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
            window_cx.update(|window, _| window.window_handle().window_id())
        };
        let second_window_id = {
            let (_, window_cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
            window_cx.update(|window, _| window.window_handle().window_id())
        };
        let scope = SelectionScopeId::new(42);

        cx.update(|cx| {
            with_text_selection_scope(first_window_id, scope, cx, |cx| {
                assert_eq!(current_text_selection_scope(second_window_id, cx), None);
                assert_eq!(
                    current_text_selection_scope(first_window_id, cx),
                    Some(scope)
                );
            });
        });
    }

    #[gpui::test]
    fn selection_callback_can_reenter_its_host(cx: &mut TestAppContext) {
        let called = Rc::new(Cell::new(false));
        let called_from_callback = called.clone();
        cx.update(|cx| {
            let host = cx.new(|_| TextSelectionState::default());
            let host_for_callback = host.clone();
            let region = FakeRegion::new("region", cx);
            region.region.on_selection(
                move |snapshot, cx| {
                    if snapshot.is_some() {
                        host_for_callback.update(cx, |_, _| called_from_callback.set(true));
                    }
                },
                cx,
            );
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
            let region = SelectableText::new("region", cx);
            region.on_selection(
                move |snapshot, _| {
                    observed_for_callback.borrow_mut().push(snapshot.is_some());
                },
                cx,
            );
            region.0.update(cx, |state, cx| {
                state.set_snapshot(
                    Some(plain_snapshot(point(px(1.), px(1.)), point(px(8.), px(1.)))),
                    cx,
                );
                let callbacks = state.clear_state();
                SelectableTextState::dispatch_clear(callbacks, cx);
            });
        });
        cx.run_until_parked();
        assert_eq!(&*observed.borrow(), &[false]);
    }

    fn run_frame(order: u64, text: SharedString, layout: TextLayout) -> SelectionRunFrame {
        let bounds = layout.bounds();
        SelectionRunFrame::new(order, text, layout, bounds)
    }

    #[gpui::test]
    fn public_selection_data_uses_builders_and_readers(cx: &mut TestAppContext) {
        let bounds = Bounds::new(point(px(1.), px(2.)), size(px(30.), px(10.)));
        let hitbox = Hitbox {
            id: HitboxId::placeholder(),
            bounds,
            content_mask: ContentMask { bounds },
            behavior: HitboxBehavior::Normal,
        };
        let scope = SelectionScopeId::new(7);
        let endpoint = SelectionEndpointSnapshot::new(None, bounds.origin).with_virtual_key(11);
        let snapshot = SelectionSnapshot::new(endpoint, endpoint)
            .with_selecting(true)
            .with_resolved_points(Some((bounds.origin, bounds.bottom_right())))
            .with_region_coverage(SelectionRegionCoverage::Full);
        let region_frame = SelectableTextFrame::new(hitbox, bounds)
            .with_scroll_offset(point(px(3.), px(4.)))
            .with_scope(scope)
            .with_document_order(9)
            .with_text_bounds(vec![bounds]);

        assert_eq!(endpoint.region_id(), None);
        assert_eq!(endpoint.point(), bounds.origin);
        assert_eq!(endpoint.virtual_key(), Some(11));
        assert_eq!(snapshot.anchor(), endpoint);
        assert_eq!(snapshot.cursor(), endpoint);
        assert!(snapshot.is_selecting());
        assert_eq!(snapshot.region_coverage(), SelectionRegionCoverage::Full);
        assert_eq!(
            snapshot.resolved_points(),
            Some((bounds.origin, bounds.bottom_right()))
        );
        assert_eq!(region_frame.bounds(), bounds);
        assert_eq!(region_frame.scroll_offset(), point(px(3.), px(4.)));
        assert_eq!(region_frame.scope(), scope);
        assert_eq!(region_frame.document_order(), 9);
        assert_eq!(region_frame.text_bounds(), &[bounds]);

        let (text, layout) = laid_out_runs(&["aé"], cx).pop().unwrap();
        let run_frame = SelectionRunFrame::new(3, text.clone(), layout.clone(), layout.bounds());
        assert_eq!(run_frame.order(), 3);
        assert_eq!(run_frame.text(), &text);
        assert_eq!(run_frame.layout().len(), layout.len());
        assert_eq!(run_frame.bounds(), layout.bounds());

        let run_state = SelectionRunState::new()
            .with_byte_range(Some(1..3))
            .with_active(true);
        assert_eq!(run_state.byte_range(), Some(&(1..3)));
        assert!(run_state.is_active());
    }

    #[gpui::test]
    fn region_handle_is_the_public_adapter_seam(cx: &mut TestAppContext) {
        let selected = Rc::new(Cell::new(false));
        let selected_from_callback = selected.clone();
        cx.update(|cx| {
            let region = SelectableText::new("initial", cx);
            let region_id = region.entity_id();
            region.set_selected_text("updated", cx);
            region.set_local_selection(true, cx);
            region.on_selection(
                move |snapshot, _| selected_from_callback.set(snapshot.is_some()),
                cx,
            );
            region.on_auto_scroll(|_, _| {}, cx);
            region.on_focus(|_, _| {}, cx);
            region.on_clear(|_| {}, cx);
            region.copy_with(|_| "copied".to_string(), cx);
            region.on_virtual_key(|_, _| Some(3), cx);

            assert_eq!(region.entity_id(), region_id);
            assert_eq!(region.snapshot(cx), None);
            assert_eq!(
                region.project_selection_runs(&[], cx),
                Vec::<SelectionRunState>::new()
            );
        });
        assert!(!selected.get());
    }

    #[gpui::test]
    fn plain_projection_preserves_forward_reversed_and_unicode_ranges(cx: &mut TestAppContext) {
        let (text, layout) = laid_out_runs(&["aé🙂z"], cx).pop().unwrap();
        let run = run_frame(0, text, layout.clone());
        let start = layout.position_for_index(1).unwrap();
        let end = layout.position_for_index(7).unwrap();

        let forward =
            project_selection_runs(Some(plain_snapshot(start, end)), std::slice::from_ref(&run));
        let reversed = project_selection_runs(Some(plain_snapshot(end, start)), &[run]);

        assert_eq!(forward[0].byte_range(), Some(&(1..7)));
        assert_eq!(reversed[0].byte_range(), Some(&(1..7)));
        assert!(forward[0].is_active());
        assert!(reversed[0].is_active());
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

        assert_eq!(states[0].byte_range(), Some(&(0..3)));
        assert_eq!(states[1].byte_range(), None);
        assert_eq!(states[2].byte_range(), Some(&(2..5)));
        assert!(states.iter().all(SelectionRunState::is_active));
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

            first
                .region
                .0
                .update(cx, |state, cx| state.set_snapshot(Some(snapshot), cx));
            assert_eq!(
                first
                    .region
                    .project_selection_runs(&[run_frame(0, first_text, first_layout)], cx),
                vec![
                    SelectionRunState::new()
                        .with_byte_range(Some(1..3))
                        .with_active(true)
                ]
            );
            second
                .region
                .0
                .update(cx, |state, cx| state.set_snapshot(Some(snapshot), cx));
            assert_eq!(
                second
                    .region
                    .project_selection_runs(&[run_frame(0, second_text, second_layout)], cx),
                vec![
                    SelectionRunState::new()
                        .with_byte_range(Some(0..2))
                        .with_active(true)
                ]
            );

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
            region.region.0.update(cx, |state, cx| {
                state.set_snapshot(Some(first_snapshot), cx);
                state.project_selection_runs(std::slice::from_ref(&run));
            });
            assert_eq!(host.selected_text(cx), "ir");

            region.region.0.update(cx, |state, cx| {
                state.set_snapshot(Some(changed_snapshot), cx);
            });
            assert_eq!(host.selected_text(cx), "");

            region.region.project_selection_runs(&[run], cx);
            assert_eq!(host.selected_text(cx), "st");
            host.clear(cx);
            region.region.set_local_selection(true, cx);
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
            region.region.0.update(cx, |state, cx| {
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

        assert_eq!(states, vec![SelectionRunState::new().with_active(true)]);
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
            let first_anchor = host.snapshot().unwrap().anchor();

            host.begin(point(px(0.), px(2.)), true, cx);
            host.end(cx);
            let reversed = host.snapshot().unwrap();
            assert_eq!(reversed.anchor(), first_anchor);
            assert!(reversed.cursor().point().x < reversed.anchor().point().x);
        });
    }

    #[gpui::test]
    fn virtual_key_callback_runs_outside_the_window_state_lease(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = cx.new(|_| TextSelectionState::default());
            let region = FakeRegion::new("virtual", cx);
            let state_for_callback = state.clone();
            region.region.on_virtual_key(
                move |_, cx| {
                    let _ = state_for_callback.read(cx).snapshot();
                    Some(7)
                },
                cx,
            );
            state.update(cx, |state, cx| {
                region.register(state, 0., SelectionScopeId::default(), 0, cx);
                state.begin(point(px(1.), px(1.)), false, cx);
                state.update(point(px(8.), px(1.)), cx);
            });

            TextSelectionState::resolve_virtual_keys(&state, cx);

            assert_eq!(
                state.read(cx).snapshot().unwrap().cursor().virtual_key(),
                Some(7)
            );
        });
    }

    #[gpui::test]
    fn active_dnd_does_not_move_a_text_selection_cursor(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| WindowRegionView {
            region: SelectableText::new("unused", cx),
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
            region: SelectableText::new("copied", cx),
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
    fn two_windows_isolate_selection_copy_clear_and_release_ownership(cx: &mut TestAppContext) {
        let first = cx.add_window(|_, cx| WindowOwnedSelectionView {
            region: SelectableText::new("first", cx),
        });
        let second = cx.add_window(|_, cx| WindowOwnedSelectionView {
            region: SelectableText::new("second", cx),
        });
        let first_region = cx.update(|cx| first.read(cx).unwrap().region.clone());
        let second_region = cx.update(|cx| second.read(cx).unwrap().region.clone());

        let first_state = cx
            .update_window(*first, |_, window, cx| {
                let _ = window.draw(cx);
                first_region.set_local_selection(true, cx);
                assert_eq!(window.selected_text(cx), "first");
                TextSelectionState::existing(window, cx)
                    .unwrap()
                    .downgrade()
            })
            .unwrap();
        cx.update_window(*second, |_, window, cx| {
            let _ = window.draw(cx);
            second_region.set_local_selection(true, cx);
            assert_eq!(window.selected_text(cx), "second");
        })
        .unwrap();

        cx.update_window(*first, |_, window, cx| {
            window.clear_text_selection(cx);
            assert_eq!(window.selected_text(cx), "");
        })
        .unwrap();
        cx.update_window(*second, |_, window, cx| {
            assert_eq!(window.selected_text(cx), "second");
        })
        .unwrap();

        cx.update_window(*first, |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();

        assert!(first_state.upgrade().is_none());
        cx.update_window(*second, |_, window, cx| {
            assert_eq!(window.selected_text(cx), "second");
        })
        .unwrap();
        cx.update(|cx| {
            assert_eq!(cx.global::<WindowTextSelections>().0.len(), 1);
        });
    }

    #[gpui::test]
    fn copy_callback_can_reenter_window_and_region_selection(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            let state = TextSelectionState::existing(window, cx).unwrap();
            let region = SelectableText::new("fallback", cx);
            let state_for_copy = state.clone();
            let region_for_copy = region.clone();
            region.copy_with(
                move |cx: &mut App| {
                    state_for_copy.update(cx, |state, _| {
                        assert!(state.snapshot().is_some());
                    });
                    assert!(region_for_copy.snapshot(cx).is_some());
                    region_for_copy.set_selected_text("reentered", cx);
                    "reentrant copy".to_string()
                },
                cx,
            );
            state.update(cx, |state, cx| {
                FakeRegion {
                    region: region.clone(),
                }
                .register(state, 0., SelectionScopeId::default(), 0, cx);
                state.begin(point(px(1.), px(1.)), false, cx);
                state.update(point(px(8.), px(1.)), cx);
                state.end(cx);
            });

            assert_eq!(window.selected_text(cx), "reentrant copy");
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
            assert!(third.region.snapshot(cx).is_none());
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
            assert!(base.region.snapshot(cx).is_none());
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
            assert!(region.region.snapshot(cx).is_none());
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
            assert!(region.region.snapshot(cx).is_none());
        });
    }

    #[gpui::test]
    fn clear_stops_anchor_auto_scroll_before_discarding_the_anchor(cx: &mut TestAppContext) {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let observed = commands.clone();
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("scroll", cx);
            region
                .region
                .on_auto_scroll(move |delta, _| observed.borrow_mut().push(delta), cx);
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
            let endpoint = host.snapshot().unwrap().cursor();

            assert_eq!(endpoint.region_id(), Some(earlier.region.entity_id()));
        });
    }

    #[gpui::test]
    fn equal_area_hovered_regions_break_ties_by_document_order(cx: &mut TestAppContext) {
        cx.update(|cx| {
            for _ in 0..64 {
                let mut host = TextSelectionState::default();
                let later = FakeRegion::new("later", cx);
                let earliest = FakeRegion::new("earliest", cx);
                let middle = FakeRegion::new("middle", cx);
                later.register(&mut host, 0., SelectionScopeId::default(), 30, cx);
                earliest.register(&mut host, 0., SelectionScopeId::default(), 10, cx);
                middle.register(&mut host, 0., SelectionScopeId::default(), 20, cx);

                host.begin(point(px(1.), px(1.)), false, cx);
                host.update(point(px(8.), px(1.)), cx);

                assert_eq!(
                    host.snapshot().unwrap().anchor().region_id(),
                    Some(earliest.region.entity_id())
                );
            }
        });
    }

    #[gpui::test]
    fn window_extension_is_a_safe_no_op_until_the_element_is_rendered(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, cx| WindowRegionView {
            region: SelectableText::new("not enabled", cx),
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
    fn constructed_selection_element_supports_scope_and_registration_on_the_first_frame(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| FirstFrameScopedSelectionView {
            region: SelectableText::new("first frame", cx),
        });
        let region = cx.update(|_, cx| view.read(cx).region.clone());

        cx.update(|window, cx| {
            let _ = window.draw(cx);
            let state = TextSelectionState::existing(window, cx).unwrap();
            assert_eq!(state.read(cx).active_scope, SelectionScopeId::new(23));
            assert!(state.read(cx).regions.contains_key(&region.entity_id()));
        });
    }

    #[gpui::test]
    fn lazy_registration_does_not_enable_queries_without_the_element(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, cx| WindowRegionView {
            region: SelectableText::new("registered", cx),
        });
        cx.update(|window, cx| {
            let region = SelectableText::new("registered", cx);
            region.set_local_selection(true, cx);
            let bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(20.)));
            let hitbox = Hitbox {
                id: HitboxId::placeholder(),
                bounds,
                content_mask: ContentMask { bounds },
                behavior: HitboxBehavior::Normal,
            };
            window.register_selectable_text(
                region,
                SelectableTextFrame::new(hitbox, bounds).with_text_bounds(vec![bounds]),
                cx,
            );
            assert_eq!(window.selected_text(cx), "");
            assert!(!window.has_text_selection(cx));
            window.clear_text_selection(cx);
            assert_eq!(window.selected_text(cx), "");
        });
    }

    #[gpui::test]
    fn retained_selection_state_releases_and_does_not_resurrect_selection(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| ToggleSelectionElementView {
            enabled: true,
            region: SelectableText::new("local", cx),
        });
        let region = cx.update(|_, cx| view.read(cx).region.clone());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            region.set_local_selection(true, cx);
            assert!(window.has_text_selection(cx));

            window.simulate_next_frame(cx);
            assert!(window.has_text_selection(cx));
            let _ = window.draw(cx);
            assert!(window.has_text_selection(cx));
        });
        view.update(cx, |view, cx| {
            view.enabled = false;
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
        });
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!window.has_text_selection(cx));
            assert_eq!(window.selected_text(cx), "");
            assert!(!region.has_local_selection(cx));
            window.clear_text_selection(cx);
        });

        view.update(cx, |view, cx| {
            view.enabled = true;
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            assert!(!window.has_text_selection(cx));
            assert_eq!(window.selected_text(cx), "");
        });
    }

    #[gpui::test]
    fn mounted_selection_element_does_not_keep_an_idle_frame_queue_alive(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            assert!(window.simulate_next_frame(cx) > 0);
            assert_eq!(window.simulate_next_frame(cx), 0);
            assert_eq!(window.simulate_next_frame(cx), 0);
            assert!(live_text_selection_state(window, cx).is_some());
        });
    }

    #[gpui::test]
    fn selection_element_initializes_suppression_and_respects_bubble_suppression(
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = cx.add_window_view(|_, _| SelectionElementOnlyView);
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
    fn frame_sweep_keeps_a_region_registered_before_the_selection_element_paints(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            let mut host = TextSelectionState::default();
            let region = FakeRegion::new("painted first", cx);
            region.register(&mut host, 0., SelectionScopeId::default(), 0, cx);
            host.begin(point(px(1.), px(1.)), false, cx);
            host.update(point(px(8.), px(1.)), cx);
            host.end(cx);

            host.finish_frame(cx);

            assert_eq!(host.selected_text(cx), "painted first");
            assert!(region.region.snapshot(cx).is_some());
        });
    }

    #[gpui::test]
    fn two_selection_elements_schedule_only_one_post_frame_sweep(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| DoubleSelectionElementView {
            region: SelectableText::new("once", cx),
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

            let items = host.read(cx).copy_items(cx);
            assert_eq!(resolve_copy_items(items, cx), "once");
        });
    }

    #[gpui::test]
    fn duplicate_selection_elements_gate_real_pointer_gestures_and_reentrant_clear(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| DoubleSelectionElementView {
            region: SelectableText::new("once", cx),
        });
        let clear_count = Rc::new(Cell::new(0));
        cx.update(|window, cx| {
            let state = TextSelectionState::ensure(window, cx);
            let state_for_clear = state.clone();
            let count = clear_count.clone();
            let region = view.read(cx).region.clone();
            region.on_clear(
                move |cx| {
                    count.set(count.get() + 1);
                    let _ = state_for_clear.read(cx).snapshot();
                },
                cx,
            );
            let _ = window.draw(cx);
        });

        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_down(
            point(px(70.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        cx.simulate_mouse_up(
            point(px(70.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.update(|window, cx| assert!(window.has_text_selection(cx)));

        cx.simulate_mouse_down(
            point(px(15.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(85.), px(10.)),
            Some(MouseButton::Left),
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(85.), px(10.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.update(|window, cx| assert!(window.has_text_selection(cx)));
        assert_eq!(clear_count.get(), 3);
    }
}
