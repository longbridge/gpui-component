//! A canvas of freely placed panels, itself a dockable panel.
//!
//! [`Tiles`] holds other panels at stored bounds. Each tile is dragged by its
//! title bar, resized from its edges and its corner, snapped to its
//! neighbours and to the theme's grid, raised when pressed, zoomed to fill
//! the dock, and closed from its menu. The canvas is a [`Panel`], so it sits
//! in a tab group like any other panel and `gpui_base::dock` never has to
//! know what it draws: a canvas is a shape one product wants, not a container
//! every dock needs.
//!
//! A zoomed tile is drawn by the canvas, whose own tab group is zoomed along
//! with it: the group fills the area, the canvas fills the group, and the one
//! tile fills the canvas — wearing the chrome that zooms it back out.
//!
//! Persistence goes through [`gpui_base::dock::Panel::dump`]: the canvas
//! writes its children and their placement under `PanelInfo::Tiles`, and the
//! builder `gpui_component::init` registers under `"Tiles"` rebuilds both on
//! the next load, through the same [`PanelRegistry`] every other panel uses.

mod geometry;

use std::sync::Arc;

use gpui::{
    AnyElement, App, AppContext as _, Bounds, Context, Div, DragMoveEvent, Empty, EntityId,
    EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement, MouseButton,
    MouseDownEvent, ParentElement as _, Pixels, Point, Render, ScrollHandle, Stateful,
    StatefulInteractiveElement as _, Styled as _, WeakEntity, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_base::{
    UndoHistory,
    dock::{
        AnyDrag, DockArea, DockEvent, PanelBuildContext, PanelEvent, PanelId, PanelInfo,
        PanelRegistry, PanelState, TabGroup, TileMeta,
    },
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    dock::{BasePanelView, Panel, PanelHandle, invalid_panel::InvalidPanel, panel_handle},
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::{Scrollbar, ScrollbarMode},
    v_flex,
};

use self::geometry::{
    DRAG_BAR_HEIGHT, HANDLE_SIZE, MINIMUM_SIZE, ResizeDrag, ResizeSide, TileChange,
    apply_boundary_constraints, compute_resized_bounds, content_size, magnetic_snap,
};

/// The name a canvas is filed under in a persisted layout. Contract, not a
/// type name: it must keep its value even if the type is renamed.
pub(crate) const TILES_PANEL_NAME: &str = "Tiles";
/// What the old dock wrapped every tile in, and what a canvas written by it
/// still holds as children.
const TAB_PANEL_NAME: &str = "TabPanel";

/// How far a resize handle sticks out past the tile's edge.
const HANDLE_OFFSET: Pixels = px(-4.);

/// What a canvas reports outward.
#[non_exhaustive]
pub enum TilesEvent {
    /// A host-owned drag landed on the canvas. The canvas has free
    /// coordinates, so the host reads the landing position itself and places
    /// whatever it adds with [`Tiles::add_panel`].
    DragDrop { item: AnyDrag },
}

/// One panel on the canvas, at its stored bounds.
#[derive(Clone)]
pub struct Tile {
    panel: Arc<dyn BasePanelView>,
    id: PanelId,
    bounds: Bounds<Pixels>,
    z_index: usize,
}

impl Tile {
    pub fn panel(&self) -> &Arc<dyn BasePanelView> {
        &self.panel
    }

    pub fn panel_id(&self) -> PanelId {
        self.id
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Higher is nearer the viewer.
    pub fn z_index(&self) -> usize {
        self.z_index
    }
}

/// The payload a tile drag carries, so one canvas ignores another's drags.
#[derive(Clone)]
struct DragMoving(EntityId);

impl Render for DragMoving {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// The payload a tile resize carries, for the same reason.
#[derive(Clone)]
struct DragResizing(EntityId);

impl Render for DragResizing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// An in-flight move: which tile, and where the pointer and the tile were
/// when it started.
#[derive(Clone, Copy)]
struct TileMove {
    panel: PanelId,
    initial_pointer: Point<Pixels>,
    initial_bounds: Bounds<Pixels>,
}

/// An in-flight resize: which tile, and the geometry module's own drag record.
#[derive(Clone, Copy)]
struct TileResize {
    panel: PanelId,
    initial_bounds: Bounds<Pixels>,
    drag: ResizeDrag,
}

/// A canvas of panels at free positions, each dragged and resized on its own.
///
/// Build one, add panels to it, and hand it to the dock like any other panel:
///
/// ```ignore
/// let tiles = cx.new(|cx| Tiles::new(dock_area.downgrade(), window, cx));
/// tiles.update(cx, |tiles, cx| {
///     tiles.add_panel(chart, Bounds::new(point(px(20.), px(20.)), size(px(380.), px(280.))), window, cx);
/// });
/// area.set_center(DockLayout::tabs().panel_view(panel_handle(tiles), cx), window, cx);
/// ```
///
/// The canvas draws a title bar on every tile, so the tab group holding it
/// draws none of its own while the canvas is the group's only panel.
pub struct Tiles {
    focus_handle: FocusHandle,
    /// Where a layout change is reported, so a host persisting the area hears
    /// about a moved tile the way it hears about a moved tab.
    dock_area: WeakEntity<DockArea>,
    /// The group this canvas sits in, once it has joined one. A zoomed tile
    /// zooms the group along with it — that is how one tile comes to fill
    /// the dock.
    group: Option<WeakEntity<TabGroup>>,
    tiles: Vec<Tile>,
    /// The tile filling the whole dock, if one is.
    zoomed: Option<PanelId>,
    moving: Option<TileMove>,
    resizing: Option<TileResize>,
    history: UndoHistory<TileChange>,
    scroll_handle: ScrollHandle,
    scrollbar_mode: Option<ScrollbarMode>,
}

impl Tiles {
    /// An empty canvas. `dock_area` is the area the canvas will be docked in,
    /// which is told about every tile move so a layout subscriber can save.
    pub fn new(
        dock_area: WeakEntity<DockArea>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            dock_area,
            group: None,
            tiles: Vec::new(),
            zoomed: None,
            moving: None,
            resizing: None,
            history: UndoHistory::new().group_interval(std::time::Duration::from_millis(100)),
            scroll_handle: ScrollHandle::default(),
            scrollbar_mode: None,
        }
    }

    /// Rebuild a canvas from what [`gpui_base::dock::Panel::dump`] wrote,
    /// building each child through the [`PanelRegistry`]. A child no builder
    /// answers for becomes a placeholder that carries its state forward, so
    /// the next save does not erase it.
    ///
    /// This is what the builder registered under `"Tiles"` calls; it is
    /// public for a host that wraps a canvas in a panel of its own.
    pub fn from_state(
        state: &PanelState,
        dock_area: WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self::new(dock_area.clone(), window, cx);
        let metas: &[TileMeta] = match &state.info {
            PanelInfo::Tiles { metas } => metas,
            _ => &[],
        };
        for (ix, child) in state.children.iter().enumerate() {
            // Keyed by the *child* index, not by the output index: a child
            // that expands to several tiles must not shift the metas of the
            // children after it.
            let meta = metas.get(ix).copied().unwrap_or_default();
            for panel in restored_panels(child, &dock_area, window, cx) {
                this.push(panel, meta.bounds, meta.z_index, cx);
            }
        }
        this
    }

    /// Every tile, in the order it was added. Stacking order is
    /// [`Tile::z_index`].
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    pub fn tile(&self, panel: PanelId) -> Option<&Tile> {
        self.index_of(panel).map(|ix| &self.tiles[ix])
    }

    /// When the canvas shows its scrollbar. `None` follows the theme.
    pub fn scrollbar_mode(&self) -> Option<ScrollbarMode> {
        self.scrollbar_mode
    }

    pub fn set_scrollbar_mode(&mut self, mode: Option<ScrollbarMode>, cx: &mut Context<Self>) {
        self.scrollbar_mode = mode;
        cx.notify();
    }

    /// Place a panel on the canvas at `bounds`, on top of the tiles already
    /// there.
    pub fn add_panel<P: Panel>(
        &mut self,
        panel: gpui::Entity<P>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_panel_view(panel_handle(panel), bounds, window, cx);
    }

    /// [`Self::add_panel`] for an already-wrapped handle.
    ///
    /// A panel already on the canvas is moved to `bounds` and raised rather
    /// than added twice.
    pub fn add_panel_view(
        &mut self,
        panel: Arc<dyn BasePanelView>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = panel.panel_id(cx);
        if self.index_of(id).is_some() {
            self.set_tile_bounds(id, bounds, cx);
            self.bring_to_front(id, cx);
            return;
        }
        let top = self.top_z_index();
        self.push(panel, bounds, top + 1, cx);
        self.layout_changed(cx);
    }

    /// Take a panel off the canvas, telling it that it was removed.
    pub fn remove_panel(&mut self, panel: PanelId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        if self.zoomed == Some(panel) {
            self.set_zoomed_tile(None, window, cx);
        }
        if self.moving.is_some_and(|drag| drag.panel == panel) {
            self.moving = None;
        }
        if self.resizing.is_some_and(|drag| drag.panel == panel) {
            self.resizing = None;
        }
        let tile = self.tiles.remove(ix);
        tile.panel.on_removed(window, cx);
        self.layout_changed(cx);
    }

    /// Move or resize one tile outright, with no snapping and no undo record.
    pub fn set_tile_bounds(
        &mut self,
        panel: PanelId,
        bounds: Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.apply_bounds(panel, bounds, cx);
    }

    /// Stack one tile above its peers.
    pub fn bring_to_front(&mut self, panel: PanelId, cx: &mut Context<Self>) {
        let top = self.top_z_index();
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        // Already on top is a no-op rather than another increment, so
        // repeatedly grabbing the front tile neither churns the layout nor
        // lets z-indices climb forever.
        if self.tiles[ix].z_index >= top
            && self.tiles.iter().filter(|tile| tile.z_index == top).count() == 1
        {
            return;
        }
        self.tiles[ix].z_index = top + 1;
        self.layout_changed(cx);
    }

    /// The tile filling the whole dock, if one is.
    pub fn zoomed_tile(&self) -> Option<PanelId> {
        self.zoomed
    }

    /// Flip one tile between filling the whole dock and sitting at its stored
    /// bounds.
    ///
    /// Zooming *in* is refused for a tile that is not on this canvas or whose
    /// panel is not zoomable. Zooming *out* is never refused: a tile that
    /// became unzoomable while zoomed still has to be able to give the dock
    /// back.
    pub fn toggle_zoom(&mut self, panel: PanelId, window: &mut Window, cx: &mut Context<Self>) {
        let zoomed = (self.zoomed != Some(panel)).then_some(panel);
        self.set_zoomed_tile(zoomed, window, cx);
    }

    /// Undo the most recent group of tile moves and resizes.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.history.undo() else {
            return;
        };
        for change in changes {
            self.apply_bounds(change.panel(), change.old_bounds(), cx);
        }
    }

    /// Redo the most recently undone group of tile moves and resizes.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.history.redo() else {
            return;
        };
        for change in changes {
            self.apply_bounds(change.panel(), change.new_bounds(), cx);
        }
    }
}

/// Lookups and the gestures.
impl Tiles {
    fn index_of(&self, panel: PanelId) -> Option<usize> {
        self.tiles.iter().position(|tile| tile.id == panel)
    }

    fn bounds_of(&self, panel: PanelId) -> Option<Bounds<Pixels>> {
        self.index_of(panel).map(|ix| self.tiles[ix].bounds)
    }

    fn panel_view(&self, panel: PanelId) -> Option<Arc<dyn BasePanelView>> {
        self.index_of(panel).map(|ix| self.tiles[ix].panel.clone())
    }

    fn top_z_index(&self) -> usize {
        self.tiles
            .iter()
            .map(|tile| tile.z_index)
            .max()
            .unwrap_or(0)
    }

    /// Every other tile's bounds, which is what the snapping arithmetic
    /// measures against.
    fn other_bounds(&self, panel: PanelId) -> Vec<Bounds<Pixels>> {
        self.tiles
            .iter()
            .filter(|tile| tile.id != panel)
            .map(|tile| tile.bounds)
            .collect()
    }

    /// The tiles in stacking order, lowest first.
    fn stacked(&self) -> Vec<Tile> {
        let mut order: Vec<usize> = (0..self.tiles.len()).collect();
        order.sort_by_key(|ix| (self.tiles[*ix].z_index, *ix));
        order.into_iter().map(|ix| self.tiles[ix].clone()).collect()
    }

    fn push(
        &mut self,
        panel: Arc<dyn BasePanelView>,
        bounds: Bounds<Pixels>,
        z_index: usize,
        cx: &mut Context<Self>,
    ) {
        self.tiles.push(Tile {
            id: panel.panel_id(cx),
            panel,
            bounds,
            z_index,
        });
        cx.notify();
    }

    /// The tiles are the canvas's persisted state, so a change to them is a
    /// change to the dock's layout, reported where a host saving the area is
    /// already listening.
    ///
    /// Deferred, because the host often edits the canvas from inside an
    /// update of the very area it belongs to — adding a widget to the center
    /// is one `dock_area.update` — and the area cannot be updated again from
    /// within its own update.
    fn layout_changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(PanelEvent::LayoutChanged);
        let dock_area = self.dock_area.clone();
        cx.defer(move |cx| {
            _ = dock_area.update(cx, |_, cx| cx.emit(DockEvent::LayoutChanged));
        });
        cx.notify();
    }

    /// Zoom one tile in, or zoom out with `None`. The tile's panel is told,
    /// and the canvas's own group is zoomed to match, which is what makes the
    /// tile fill the dock rather than just the canvas.
    fn set_zoomed_tile(
        &mut self,
        zoomed: Option<PanelId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.zoomed == zoomed {
            return;
        }
        // The outgoing tile hears about it as well as the incoming one, so a
        // zoom moved straight from one tile to another leaves neither panel
        // believing it still fills the dock.
        let outgoing = self.zoomed.and_then(|panel| self.panel_view(panel));
        let incoming = zoomed.and_then(|panel| self.panel_view(panel));
        if zoomed.is_some() && !incoming.as_ref().is_some_and(|panel| panel.zoomable(cx)) {
            return;
        }

        self.zoomed = zoomed;
        cx.emit(match zoomed {
            Some(_) => PanelEvent::ZoomIn,
            None => PanelEvent::ZoomOut,
        });

        // Delivered outside this update: the group reads this canvas — it is
        // the group's displayed panel — before it zooms, and a `set_zoomed`
        // handler may call back into the canvas.
        let group = self.group.as_ref().and_then(WeakEntity::upgrade);
        cx.spawn_in(window, async move |_, cx| {
            _ = cx.update(|window, cx| {
                if let Some(group) = group {
                    group.update(cx, |group, cx| {
                        if group.is_zoomed() != zoomed.is_some() {
                            group.toggle_zoom(window, cx);
                        }
                    });
                }
                if let Some(panel) = outgoing {
                    panel.set_zoomed(false, window, cx);
                }
                if let Some(panel) = incoming {
                    panel.set_zoomed(true, window, cx);
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn begin_move(&mut self, panel: PanelId, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        // A zoomed tile fills the dock rather than sitting at its stored
        // bounds, so there is nothing for a move to mean.
        if self.zoomed.is_some() {
            return;
        }
        let Some(initial_bounds) = self.bounds_of(panel) else {
            return;
        };
        self.moving = Some(TileMove {
            panel,
            initial_pointer: pointer,
            initial_bounds,
        });
        self.bring_to_front(panel, cx);
        cx.notify();
    }

    fn move_to(&mut self, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.moving else {
            return;
        };
        let delta = pointer - drag.initial_pointer;
        let candidate = Bounds {
            origin: apply_boundary_constraints(
                drag.initial_bounds.origin + delta,
                drag.initial_bounds.size.width,
            ),
            size: drag.initial_bounds.size,
        };
        let origin = magnetic_snap(
            candidate,
            &self.other_bounds(drag.panel),
            cx.theme().tile_grid_size,
        );

        self.apply_bounds(
            drag.panel,
            Bounds {
                origin,
                size: drag.initial_bounds.size,
            },
            cx,
        );
    }

    fn end_move(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.moving.take() else {
            return;
        };
        self.record(drag.panel, drag.initial_bounds, cx);
    }

    fn begin_resize(
        &mut self,
        panel: PanelId,
        side: ResizeSide,
        pointer: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.zoomed.is_some() {
            return;
        }
        let Some(initial_bounds) = self.bounds_of(panel) else {
            return;
        };
        self.resizing = Some(TileResize {
            panel,
            initial_bounds,
            drag: ResizeDrag::new(side, pointer, initial_bounds),
        });
        self.bring_to_front(panel, cx);
        cx.notify();
    }

    fn resize_to(&mut self, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(resize) = self.resizing else {
            return;
        };
        let previous = resize.drag.last_bounds();
        // The pointer is in window coordinates and the bounds in canvas
        // coordinates, so the moving edge is derived from how far the pointer
        // has travelled since the drag began, applied to the bounds it began
        // with — never from the pointer's position itself.
        let initial = resize.initial_bounds;
        let delta = pointer - resize.drag.start_position();
        let (new_x, new_y, new_width, new_height) = match resize.drag.side() {
            ResizeSide::Left => (Some(initial.origin.x + delta.x), None, None, None),
            ResizeSide::Right => (
                None,
                None,
                Some((initial.size.width + delta.x).max(MINIMUM_SIZE.width)),
                None,
            ),
            ResizeSide::Top => (None, Some(initial.origin.y + delta.y), None, None),
            ResizeSide::Bottom => (
                None,
                None,
                None,
                Some((initial.size.height + delta.y).max(MINIMUM_SIZE.height)),
            ),
            ResizeSide::BottomRight => (
                None,
                None,
                Some((initial.size.width + delta.x).max(MINIMUM_SIZE.width)),
                Some((initial.size.height + delta.y).max(MINIMUM_SIZE.height)),
            ),
        };

        let bounds = compute_resized_bounds(
            previous,
            new_x,
            new_y,
            new_width,
            new_height,
            &self.other_bounds(resize.panel),
            cx.theme().tile_grid_size,
        );

        self.resizing = Some(TileResize {
            drag: resize.drag.with_last_bounds(bounds),
            ..resize
        });
        self.apply_bounds(resize.panel, bounds, cx);
    }

    fn end_resize(&mut self, cx: &mut Context<Self>) {
        let Some(resize) = self.resizing.take() else {
            return;
        };
        self.record(resize.panel, resize.initial_bounds, cx);
    }

    fn apply_bounds(&mut self, panel: PanelId, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        if self.tiles[ix].bounds == bounds {
            return;
        }
        self.tiles[ix].bounds = bounds;
        self.layout_changed(cx);
    }

    /// Push one completed gesture onto the undo stack.
    fn record(&mut self, panel: PanelId, old_bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        let bounds = self.tiles[ix].bounds;
        if bounds == old_bounds {
            return;
        }
        self.history
            .push(TileChange::new(panel, old_bounds, bounds));
        cx.notify();
    }

    /// Dismiss a tile. Nothing happens for a panel that refuses to close.
    fn close_tile(&mut self, panel: PanelId, window: &mut Window, cx: &mut Context<Self>) {
        let closable = self.panel_view(panel).is_some_and(|view| view.closable(cx));
        if !closable {
            return;
        }
        self.remove_panel(panel, window, cx);
    }
}

/// The panels one persisted child stands for.
///
/// Every tile the old dock wrote is `TabPanel`-shaped: it wrapped each child
/// in a tab group. A tile *is* a panel here, so the group is unwrapped —
/// building the `"TabPanel"` leaf directly would miss the registry, fall to
/// a placeholder, and never build the user's real panels at all. A group
/// holding several panels expands to one tile per panel, sharing the group's
/// placement; the legacy empty-group form (a `TabPanel` name carrying leaf
/// info) stands for no panel and contributes no tile.
fn restored_panels(
    child: &PanelState,
    dock_area: &WeakEntity<DockArea>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<Arc<dyn BasePanelView>> {
    match &child.info {
        PanelInfo::Tabs { .. } => child
            .children
            .iter()
            .flat_map(|grandchild| restored_panels(grandchild, dock_area, window, cx))
            .collect(),
        PanelInfo::Panel(_) if child.panel_name == TAB_PANEL_NAME => Vec::new(),
        info => {
            let context = PanelBuildContext::new(dock_area.clone(), child, info);
            let panel = PanelRegistry::build_panel(&child.panel_name, context, window, cx)
                .unwrap_or_else(|| {
                    let state = child.clone();
                    panel_handle(
                        cx.new(|cx| InvalidPanel::new(state.panel_name.clone(), state, cx)),
                    )
                });
            vec![panel]
        }
    }
}

impl gpui_base::dock::Panel for Tiles {
    fn panel_name(&self) -> &'static str {
        TILES_PANEL_NAME
    }

    /// The group zooming out — from its own control, or from
    /// [`DockArea::set_zoomed_out`] — takes the zoomed tile with it, so the
    /// canvas cannot be left drawing one tile as if it still filled the dock.
    fn set_zoomed(&mut self, zoomed: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !zoomed {
            self.set_zoomed_tile(None, window, cx);
        }
    }

    fn on_added_to(
        &mut self,
        group: WeakEntity<TabGroup>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.group = Some(group);
    }

    /// The canvas leaving the dock takes its tiles with it, so each is told.
    fn on_removed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.group = None;
        for tile in &self.tiles {
            tile.panel.on_removed(window, cx);
        }
    }

    fn dump(&self, cx: &App) -> PanelState {
        let mut state = PanelState::new(self.panel_name());
        state.children = self.tiles.iter().map(|tile| tile.panel.dump(cx)).collect();
        state.info = PanelInfo::tiles(
            self.tiles
                .iter()
                .map(|tile| TileMeta {
                    bounds: tile.bounds,
                    z_index: tile.z_index,
                })
                .collect(),
        );
        state
    }
}

impl Panel for Tiles {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Tiles"
    }

    /// Every tile carries its own title bar, so the group draws none over
    /// the canvas.
    fn title_bar_visible(&self, _: &App) -> bool {
        false
    }

    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for Tiles {}
impl EventEmitter<TilesEvent> for Tiles {}

impl Focusable for Tiles {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Appearance.
impl Tiles {
    /// One edge or corner handle.
    fn resize_handle(
        &self,
        panel: PanelId,
        id: &'static str,
        side: ResizeSide,
        build: impl FnOnce(Stateful<Div>) -> Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let canvas = cx.entity_id();

        build(div().id(id).absolute())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.begin_resize(panel, side, event.position, cx);
                    cx.stop_propagation();
                }),
            )
            .on_drag(DragResizing(canvas), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<DragResizing>, _, cx| {
                    if event.drag(cx).0 != canvas {
                        return;
                    }
                    this.resize_to(event.event.position, cx);
                }),
            )
    }

    fn render_resize_handles(&self, tile: &Tile, cx: &mut Context<Self>) -> AnyElement {
        let bounds = tile.bounds;
        let panel = tile.id;

        // A passive full-tile box so each handle is positioned against the
        // tile rather than against whatever the flow put it next to. It
        // registers no interaction of its own, so it does not shadow the panel
        // underneath.
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(self.resize_handle(
                panel,
                "left-resize-handle",
                ResizeSide::Left,
                |this| {
                    this.cursor_ew_resize()
                        .top_0()
                        .left(HANDLE_OFFSET)
                        .w(HANDLE_SIZE)
                        .h(bounds.size.height)
                },
                cx,
            ))
            .child(self.resize_handle(
                panel,
                "right-resize-handle",
                ResizeSide::Right,
                |this| {
                    this.cursor_ew_resize()
                        .top_0()
                        .right(HANDLE_OFFSET)
                        .w(HANDLE_SIZE)
                        .h(bounds.size.height)
                },
                cx,
            ))
            .child(self.resize_handle(
                panel,
                "top-resize-handle",
                ResizeSide::Top,
                |this| {
                    this.cursor_ns_resize()
                        .left_0()
                        .top(HANDLE_OFFSET)
                        .w(bounds.size.width)
                        .h(HANDLE_SIZE)
                },
                cx,
            ))
            .child(self.resize_handle(
                panel,
                "bottom-resize-handle",
                ResizeSide::Bottom,
                |this| {
                    this.cursor_ns_resize()
                        .left_0()
                        .bottom(HANDLE_OFFSET)
                        .w(bounds.size.width)
                        .h(HANDLE_SIZE)
                },
                cx,
            ))
            .child(
                Icon::new(IconName::ResizeCorner)
                    .size_3()
                    .absolute()
                    .right(px(1.))
                    .bottom(px(1.))
                    .text_color(cx.theme().muted_foreground.opacity(0.5)),
            )
            .child(self.resize_handle(
                panel,
                "corner-resize-handle",
                ResizeSide::BottomRight,
                |this| {
                    this.cursor_nwse_resize()
                        .right(HANDLE_OFFSET)
                        .bottom(HANDLE_OFFSET)
                        .size_3()
                },
                cx,
            ))
            .into_any_element()
    }

    /// The trailing controls of a tile's title bar.
    ///
    /// A tile has no tab bar to hang a toolbar off, so this is where its zoom,
    /// close and ellipsis menu live. The entries use click handlers rather
    /// than the [`ToggleZoom`](super::ToggleZoom) and
    /// [`ClosePanel`](super::ClosePanel) actions: those are dispatched to a
    /// focused tab group, and a tile is not one.
    fn render_tile_controls(
        &self,
        tile: &Tile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = tile.id;
        let handle = PanelHandle::of(&tile.panel);
        let control = handle.and_then(|handle| handle.zoom_control(cx));
        let zoomed = self.zoomed == Some(id);
        let zoomable = tile.panel.zoomable(cx);
        let toolbar_zoom = zoomable && control.is_some_and(|control| control.toolbar_visible());
        let menu_zoom = zoomable && control.is_some_and(|control| control.menu_visible());
        let closable = tile.panel.closable(cx);
        let buttons = handle.and_then(|handle| handle.toolbar_buttons(window, cx));
        let panel = handle.map(|handle| handle.panel());
        let this = cx.weak_entity();

        h_flex()
            .gap_1()
            .flex_shrink_0()
            .occlude()
            .when_some(buttons, |this, buttons| {
                this.children(
                    buttons
                        .into_iter()
                        .map(|button| button.xsmall().ghost().tab_stop(false)),
                )
            })
            .when_some(
                match (zoomed, toolbar_zoom) {
                    (true, _) => Some(("zoom-out", IconName::Minimize, t!("Dock.Zoom Out"))),
                    (false, true) => Some(("zoom-in", IconName::Maximize, t!("Dock.Zoom In"))),
                    (false, false) => None,
                },
                |this, (button_id, icon, tooltip)| {
                    this.child(
                        Button::new(button_id)
                            .icon(icon)
                            .xsmall()
                            .ghost()
                            .tab_stop(false)
                            .tooltip(tooltip)
                            .selected(zoomed)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.toggle_zoom(id, window, cx)
                            })),
                    )
                },
            )
            .child(
                Button::new("menu")
                    .icon(IconName::Ellipsis)
                    .xsmall()
                    .ghost()
                    .tab_stop(false)
                    .dropdown_menu(move |menu, window, cx| {
                        menu.when_some(panel.clone(), |menu, panel| {
                            panel.dropdown_menu(menu, window, cx)
                        })
                        .separator()
                        .item(
                            PopupMenuItem::new(match zoomed {
                                true => t!("Dock.Zoom Out"),
                                false => t!("Dock.Zoom In"),
                            })
                            .disabled(!menu_zoom && !zoomed)
                            .on_click({
                                let this = this.clone();
                                move |_, window, cx| {
                                    _ = this
                                        .update(cx, |tiles, cx| tiles.toggle_zoom(id, window, cx));
                                }
                            }),
                        )
                        .when(closable, |menu| {
                            menu.separator()
                                .item(PopupMenuItem::new(t!("Dock.Close")).on_click({
                                    let this = this.clone();
                                    move |_, window, cx| {
                                        _ = this.update(cx, |tiles, cx| {
                                            tiles.close_tile(id, window, cx)
                                        });
                                    }
                                }))
                        })
                    })
                    .anchor(gpui::Anchor::TopRight),
            )
    }

    /// The strip the tile is dragged by, which is also its title bar.
    fn render_drag_bar(
        &self,
        tile: &Tile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = tile.id;
        let canvas = cx.entity_id();
        let zoomed = self.zoomed == Some(id);
        let handle = PanelHandle::of(&tile.panel);
        let title_style = handle.and_then(|handle| handle.title_style(cx));

        h_flex()
            .id("drag-bar")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(DRAG_BAR_HEIGHT)
            .items_center()
            .gap_1()
            .pl_3()
            .pr_2()
            .when_some(title_style, |this, style| {
                // The tile frame does not clip its children, so a painted
                // title bar rounds its own top corners to stay inside the
                // frame's.
                this.bg(style.background)
                    .text_color(style.foreground)
                    .rounded_t(cx.theme().tile_radius)
            })
            .child(
                div()
                    .flex_1()
                    .min_w_16()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(super::tab_panel::panel_title(&tile.panel, window, cx)),
            )
            .children(handle.and_then(|handle| handle.title_suffix(window, cx)))
            .child(self.render_tile_controls(tile, window, cx))
            // A zoomed tile is not at its stored bounds, so there is nothing
            // for a move to mean; the canvas refuses the gesture too.
            .when(!zoomed, |this| {
                this.cursor_grab()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            this.begin_move(id, event.position, cx);
                        }),
                    )
                    .on_drag(DragMoving(canvas), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(cx.listener(
                        move |this, event: &DragMoveEvent<DragMoving>, _, cx| {
                            if event.drag(cx).0 != canvas {
                                return;
                            }
                            this.move_to(event.event.position, cx);
                        },
                    ))
            })
            .into_any_element()
    }

    fn render_tile(&self, tile: &Tile, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = tile.id;
        let zoomed = self.zoomed == Some(id);
        let bounds = tile.bounds;

        v_flex()
            .id(("tile", id.as_u64()))
            .occlude()
            // No `overflow_hidden` here: the resize handles hang past the
            // tile's edge, and a content mask would cut their hit areas down
            // to the sliver inside it. The panel is clipped by its own frame.
            .bg(cx.theme().tokens.background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().tile_radius)
            // Room for the title bar, which is positioned over the padding so
            // the panel below it is never covered.
            .pt(DRAG_BAR_HEIGHT)
            // A zoomed tile fills the canvas, which fills the group, which
            // fills the dock. Anything else sits at its stored bounds — a
            // canvas *is* "panels at stored coordinates". One extra pixel of
            // minimum size past the stored bounds, so a snapped neighbour's
            // border overlaps this tile's instead of stacking beside it into
            // a double-width line.
            .map(|this| match zoomed {
                true => this.size_full(),
                false => this
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .min_w(bounds.size.width + px(1.))
                    .min_h(bounds.size.height + px(1.)),
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.bring_to_front(id, cx)),
            )
            // A gesture can end with the pointer anywhere, so both halves are
            // needed; each is a no-op unless this tile is the one moving.
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.end_move(cx);
                    this.end_resize(cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.end_move(cx);
                    this.end_resize(cx);
                }),
            )
            .child(self.render_drag_bar(tile, window, cx))
            .child(
                // A panel that does not size itself would otherwise have no
                // size at all.
                h_flex()
                    .id(("tile-panel", id.as_u64()))
                    .overflow_hidden()
                    .size_full()
                    .child(tile.panel.view()),
            )
            // Nothing to resize against while zoomed, and the canvas refuses
            // the gesture anyway.
            .when(!zoomed, |this| {
                this.child(self.render_resize_handles(tile, cx))
            })
            .into_any_element()
    }
}

impl Render for Tiles {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // A zoomed tile fills the dock on its own, so the tiles beside it are
        // not drawn — just as the rest of the dock is not drawn behind it. A
        // zoom naming a tile that has since left the canvas draws the canvas
        // whole rather than nothing at all.
        let zoomed = self.zoomed.filter(|panel| self.index_of(*panel).is_some());
        let tiles: Vec<Tile> = self
            .stacked()
            .into_iter()
            .filter(|tile| zoomed.is_none_or(|panel| tile.id == panel))
            .collect();
        // Every tile, not the drawn subset: an overlay scrollbar measures the
        // whole canvas.
        let content = content_size(
            &self
                .tiles
                .iter()
                .map(|tile| tile.bounds)
                .collect::<Vec<_>>(),
        );

        div()
            .id("tiles")
            .relative()
            .size_full()
            .bg(cx.theme().tokens.tiles)
            .track_scroll(&self.scroll_handle)
            .overflow_scroll()
            .track_focus(&self.focus_handle)
            .on_drop(cx.listener(|_, item: &AnyDrag, _, cx| {
                // The canvas is the drop target, not the group around it.
                cx.stop_propagation();
                cx.emit(TilesEvent::DragDrop { item: item.clone() });
            }))
            .children(
                tiles
                    .iter()
                    .map(|tile| self.render_tile(tile, window, cx))
                    .collect::<Vec<_>>(),
            )
            // Last, so it paints and hit-tests above every tile: the frame is
            // the scroll container, so a scrollbar placed among the tiles
            // would sit beneath them. A zoomed canvas is one tile filling the
            // dock, with no canvas for a scrollbar to sit over.
            .when(zoomed.is_none(), |this| {
                this.child(
                    Scrollbar::new(&self.scroll_handle)
                        .scroll_size(content)
                        .when_some(self.scrollbar_mode, |this, mode| this.mode(mode)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, TestAppContext,
        VisualTestContext, point, size,
    };
    use gpui_base::dock::{DockAreaState, DockLayout, DockPlacement, register_panel};

    use super::*;
    use crate::dock::DockSkin;

    /// A panel that answers to whatever name it was built with, so a fixture
    /// can be restored through the registry without one panel type per name.
    struct Probe {
        name: &'static str,
        focus_handle: FocusHandle,
    }

    impl Probe {
        fn new(name: &'static str, cx: &mut App) -> Entity<Self> {
            cx.new(|cx| Self {
                name,
                focus_handle: cx.focus_handle(),
            })
        }
    }

    impl gpui_base::dock::Panel for Probe {
        fn panel_name(&self) -> &'static str {
            self.name
        }
    }

    impl Panel for Probe {}
    impl EventEmitter<PanelEvent> for Probe {}

    impl Focusable for Probe {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            gpui::Empty
        }
    }

    fn setup(cx: &mut TestAppContext) -> (Entity<DockArea>, &mut VisualTestContext) {
        cx.update(|cx| crate::init(cx));
        cx.add_window_view(|window, cx| {
            let skin = DockSkin::new(cx);
            DockArea::new("tiles", None, window, cx).with_renderer(skin)
        })
    }

    fn register_probe(name: &'static str, cx: &mut App) {
        register_panel(cx, name, move |_, _, cx| panel_handle(Probe::new(name, cx)));
    }

    /// The canvas a load rebuilt, found the way a host has to find it: by
    /// walking the area's panels, since the area hands out no `Tiles`.
    fn restored_canvas(area: &Entity<DockArea>, cx: &mut VisualTestContext) -> Entity<Tiles> {
        cx.read(|cx| {
            let area = area.read(cx);
            area.layout(DockPlacement::Center)
                .expect("the center exists")
                .panels()
                .filter_map(|id| area.panel(id))
                .find_map(|panel| PanelHandle::of(panel)?.view().downcast::<Tiles>().ok())
                .expect("the fixture holds one canvas")
        })
    }

    fn names(canvas: &Entity<Tiles>, cx: &mut VisualTestContext) -> Vec<&'static str> {
        cx.read(|cx| {
            canvas
                .read(cx)
                .tiles()
                .iter()
                .map(|tile| tile.panel().panel_name(cx))
                .collect()
        })
    }

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        }
    }

    /// A canvas is one leaf to the tree and rebuilds its own panels, so a
    /// load has to come back with the tiles where they were saved — and a
    /// dump has to write the same shape out again.
    #[gpui::test]
    fn a_persisted_canvas_restores_its_panels_with_their_placement(cx: &mut TestAppContext) {
        let (area, cx) = setup(cx);
        cx.update(|_, cx| {
            register_probe("Alpha", cx);
            register_probe("Beta", cx);
        });
        let json = include_str!("../fixtures/tiles.json");
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        cx.update(|window, cx| {
            area.update(cx, |area, cx| area.load(state, window, cx).unwrap());
        });
        cx.run_until_parked();

        let canvas = restored_canvas(&area, cx);
        assert_eq!(names(&canvas, cx), vec!["Alpha", "Beta"]);
        let beta = cx.read(|cx| canvas.read(cx).tiles()[1].clone());
        assert_eq!(beta.bounds(), bounds(220., 20., 200., 150.));
        assert_eq!(beta.z_index(), 1);

        let dumped = cx.read(|cx| area.read(cx).dump(cx));
        let tiles = &dumped.center.children[0].children[0];
        assert_eq!(tiles.panel_name, "Tiles");
        assert_eq!(
            tiles
                .children
                .iter()
                .map(|child| child.panel_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Beta"]
        );
        let PanelInfo::Tiles { metas } = &tiles.info else {
            panic!("the placement is written back under the canvas");
        };
        assert_eq!(metas[1].bounds, beta.bounds());
        assert_eq!(metas[1].z_index, 1);
    }

    /// Every canvas the old dock wrote wraps each tile in a `TabPanel`. Those
    /// wrappers are unwrapped to the panels inside — reading the `"TabPanel"`
    /// leaf literally would build a placeholder and lose the user's panels —
    /// and a wrapper holding two panels becomes two tiles sharing its
    /// placement, without shifting the metas of the children after it.
    #[gpui::test]
    fn a_legacy_canvas_with_tab_panel_children_is_unwrapped(cx: &mut TestAppContext) {
        let (area, cx) = setup(cx);
        cx.update(|_, cx| {
            register_probe("Alpha", cx);
            register_probe("Beta", cx);
            register_probe("Gamma", cx);
        });
        let json = include_str!("../fixtures/tiles_tab_panel_children.json");
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        cx.update(|window, cx| {
            area.update(cx, |area, cx| area.load(state, window, cx).unwrap());
        });
        cx.run_until_parked();

        let canvas = restored_canvas(&area, cx);
        assert_eq!(names(&canvas, cx), vec!["Alpha", "Beta", "Gamma"]);
        let tiles = cx.read(|cx| canvas.read(cx).tiles().to_vec());
        assert_eq!(
            tiles[0].bounds().origin.x,
            px(10.),
            "Alpha keeps child 0's meta"
        );
        assert_eq!(
            tiles[1].bounds().origin.x,
            px(10.),
            "Beta shares child 0's meta"
        );
        assert_eq!(
            tiles[2].bounds().origin.x,
            px(400.),
            "Gamma gets child 1's meta"
        );
        assert_eq!(tiles[2].z_index(), 3);
    }

    /// A tile whose panel this build cannot construct comes back as a
    /// placeholder that still dumps the original state, so a save after a
    /// load does not erase it.
    #[gpui::test]
    fn an_unregistered_tile_survives_a_load_and_save(cx: &mut TestAppContext) {
        let (area, cx) = setup(cx);
        cx.update(|_, cx| register_probe("Alpha", cx));
        let json = include_str!("../fixtures/tiles.json");
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        cx.update(|window, cx| {
            area.update(cx, |area, cx| area.load(state, window, cx).unwrap());
        });
        cx.run_until_parked();

        let dumped = cx.read(|cx| area.read(cx).dump(cx));
        let tiles = &dumped.center.children[0].children[0];
        assert_eq!(tiles.children[1].panel_name, "Beta");
    }

    /// A zoomed tile fills the dock, not just the canvas: the canvas zooms its
    /// own group along with the tile, and the group zooming out from outside
    /// takes the tile's zoom with it.
    #[gpui::test]
    fn zooming_a_tile_zooms_the_canvas_group_and_back(cx: &mut TestAppContext) {
        let (area, cx) = setup(cx);
        let (canvas, alpha) = cx.update(|window, cx| {
            let canvas = cx.new(|cx| Tiles::new(area.downgrade(), window, cx));
            let alpha = Probe::new("Alpha", cx);
            canvas.update(cx, |canvas, cx| {
                canvas.add_panel(alpha.clone(), bounds(20., 20., 200., 150.), window, cx);
                canvas.add_panel(
                    Probe::new("Beta", cx),
                    bounds(240., 20., 200., 150.),
                    window,
                    cx,
                );
            });
            let layout = DockLayout::tabs().panel_view(panel_handle(canvas.clone()), cx);
            area.update(cx, |area, cx| area.set_center(layout, window, cx));
            (canvas, PanelId::from(alpha.entity_id()))
        });
        cx.run_until_parked();
        assert!(!cx.read(|cx| area.read(cx).is_zoomed()));

        cx.update(|window, cx| {
            canvas.update(cx, |canvas, cx| canvas.toggle_zoom(alpha, window, cx))
        });
        cx.run_until_parked();
        assert_eq!(cx.read(|cx| canvas.read(cx).zoomed_tile()), Some(alpha));
        assert!(
            cx.read(|cx| area.read(cx).zoomed_group()).is_some(),
            "the canvas's group fills the area while a tile is zoomed"
        );

        cx.update(|window, cx| area.update(cx, |area, cx| area.set_zoomed_out(window, cx)));
        cx.run_until_parked();
        assert_eq!(
            cx.read(|cx| canvas.read(cx).zoomed_tile()),
            None,
            "the group zooming out takes the tile's zoom with it"
        );
        assert!(!cx.read(|cx| area.read(cx).is_zoomed()));
    }

    /// A move snaps to a neighbour's edge, is reported to the area as a
    /// layout change — which is what a host persisting the dock listens to —
    /// and can be undone.
    #[gpui::test]
    fn a_tile_move_snaps_reports_a_layout_change_and_undoes(cx: &mut TestAppContext) {
        let (area, cx) = setup(cx);
        let changes = Rc::new(Cell::new(0));
        let (canvas, alpha) = cx.update(|window, cx| {
            let canvas = cx.new(|cx| Tiles::new(area.downgrade(), window, cx));
            let alpha = Probe::new("Alpha", cx);
            canvas.update(cx, |canvas, cx| {
                canvas.add_panel(alpha.clone(), bounds(20., 20., 200., 150.), window, cx);
                canvas.add_panel(
                    Probe::new("Beta", cx),
                    bounds(240., 20., 200., 150.),
                    window,
                    cx,
                );
            });
            let layout = DockLayout::tabs().panel_view(panel_handle(canvas.clone()), cx);
            area.update(cx, |area, cx| area.set_center(layout, window, cx));
            let changes = changes.clone();
            cx.subscribe(&area, move |_, event: &DockEvent, _| {
                if matches!(event, DockEvent::LayoutChanged) {
                    changes.set(changes.get() + 1);
                }
            })
            .detach();
            (canvas, PanelId::from(alpha.entity_id()))
        });
        cx.run_until_parked();
        changes.set(0);

        // Pointer positions are window coordinates; only the travel counts.
        // 17px of travel puts Alpha's right edge at 237, within the theme's
        // eight-pixel snap distance of Beta's left edge at 240.
        let start = point(px(500.), px(300.));
        cx.update(|_, cx| {
            canvas.update(cx, |canvas, cx| {
                canvas.begin_move(alpha, start, cx);
                canvas.move_to(start + point(px(17.), px(0.)), cx);
                canvas.end_move(cx);
            })
        });
        let moved = cx.read(|cx| canvas.read(cx).tile(alpha).unwrap().bounds());
        assert_eq!(
            moved.origin.x,
            px(40.),
            "the tile snaps flush against its neighbour rather than stopping at 37"
        );
        assert!(changes.get() > 0, "the area heard about the move");

        cx.update(|_, cx| canvas.update(cx, |canvas, cx| canvas.undo(cx)));
        let undone = cx.read(|cx| canvas.read(cx).tile(alpha).unwrap().bounds());
        assert_eq!(undone.origin.x, px(20.));
    }
}
