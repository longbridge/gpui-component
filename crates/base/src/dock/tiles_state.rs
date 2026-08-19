//! A tiles canvas's behavior, with no appearance of its own.

use std::{rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, Bounds, Context, Div, Empty, EntityId, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Point, Render, Stateful,
    Styled as _, WeakEntity, Window, div, px,
};

use crate::history::History;

use super::{
    drag::AnyDrag,
    layout::{NodeId, PanelId},
    panel::PanelView,
    tiles_geometry::{
        MINIMUM_SIZE, ResizeDrag, ResizeSide, TileChange, apply_boundary_constraints,
        compute_resized_bounds, magnetic_snap,
    },
};

/// What a tiles canvas cannot carry out on its own.
///
/// The canvas mirrors one `Tiles` node but does not own the tree that node
/// lives in, so — exactly as with [`TabGroupEvent`](super::TabGroupEvent) —
/// every outcome is reported as an intent and applied by the container
/// through `LayoutTree::set_tile_bounds` / `LayoutTree::bring_to_front`.
#[non_exhaustive]
pub enum TilesEvent {
    /// A tile finished moving or resizing at `bounds`.
    BoundsChanged {
        panel: PanelId,
        bounds: Bounds<Pixels>,
    },
    /// A tile was interacted with and should stack above its peers.
    BringToFront { panel: PanelId },
    /// The user asked to close `panel`, dismissing its tile.
    ClosePanel { panel: PanelId },
    /// A host-owned drag landed on the canvas. The canvas has free
    /// coordinates, so the host reads the landing position itself.
    DragDrop { item: AnyDrag },
}

/// One tile, mirrored from a `Tiles` node.
#[derive(Clone)]
struct Tile {
    panel: Arc<dyn PanelView>,
    id: PanelId,
    bounds: Bounds<Pixels>,
    z_index: usize,
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

/// A tiles canvas's behavior, with no appearance of its own.
///
/// It owns the tile list mirrored from the layout tree, the in-flight move and
/// resize state, and the undo stack. Everything visible is produced by the
/// [`TilesRenderer`] the host installs.
pub struct TilesState {
    node: NodeId,
    /// Handed to the callbacks in [`TileContext`], which are built from a
    /// plain `&App` and so cannot ask for it.
    this: WeakEntity<Self>,
    tiles: Vec<Tile>,
    focus_handle: FocusHandle,
    moving: Option<TileMove>,
    resizing: Option<TileResize>,
    history: History<TileChange>,
    renderer: Rc<dyn TilesRenderer>,
}

impl TilesState {
    /// Only a container builds canvases: a canvas is the entity mirror of one
    /// `Tiles` node, created when that node first appears in the tree.
    pub(crate) fn new(node: NodeId, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            node,
            this: cx.weak_entity(),
            tiles: Vec::new(),
            focus_handle: cx.focus_handle(),
            moving: None,
            resizing: None,
            history: History::new().group_interval(std::time::Duration::from_millis(100)),
            renderer: Rc::new(BareTiles),
        }
    }

    pub fn with_renderer(mut self, renderer: Rc<dyn TilesRenderer>) -> Self {
        self.renderer = renderer;
        self
    }

    /// The `Tiles` node this canvas mirrors.
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// Every tile, in stacking order (lowest first).
    pub fn tiles(&self, cx: &App) -> Vec<TileContext> {
        let mut order: Vec<usize> = (0..self.tiles.len()).collect();
        order.sort_by_key(|ix| (self.tiles[*ix].z_index, *ix));
        order
            .into_iter()
            .map(|ix| self.tile_context(ix, cx))
            .collect()
    }

    /// Mirror one `Tiles` node's membership and geometry into this canvas.
    pub(crate) fn sync_from_tree(
        &mut self,
        tiles: Vec<(Arc<dyn PanelView>, Bounds<Pixels>, usize)>,
        cx: &mut Context<Self>,
    ) {
        self.tiles = tiles
            .into_iter()
            .map(|(panel, bounds, z_index)| Tile {
                id: panel.panel_id(cx),
                panel,
                bounds,
                z_index,
            })
            .collect();
        // A tile that left the canvas must not be resurrected by an in-flight
        // gesture that outlived it.
        if self
            .moving
            .is_some_and(|drag| self.index_of(drag.panel).is_none())
        {
            self.moving = None;
        }
        if self
            .resizing
            .is_some_and(|drag| self.index_of(drag.panel).is_none())
        {
            self.resizing = None;
        }
        cx.notify();
    }

    /// Undo the most recent group of tile changes.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.history.undo() else {
            return;
        };
        for change in changes {
            if let (Some(panel), Some(bounds)) =
                (self.panel_of(change.tile_id()), change.old_bounds())
            {
                cx.emit(TilesEvent::BoundsChanged { panel, bounds });
            }
        }
        cx.notify();
    }

    /// Redo the most recently undone group of tile changes.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.history.redo() else {
            return;
        };
        for change in changes {
            if let (Some(panel), Some(bounds)) =
                (self.panel_of(change.tile_id()), change.new_bounds())
            {
                cx.emit(TilesEvent::BoundsChanged { panel, bounds });
            }
        }
        cx.notify();
    }
}

impl TilesState {
    fn index_of(&self, panel: PanelId) -> Option<usize> {
        self.tiles.iter().position(|tile| tile.id == panel)
    }

    fn bounds_of(&self, panel: PanelId) -> Option<Bounds<Pixels>> {
        self.index_of(panel).map(|ix| self.tiles[ix].bounds)
    }

    /// The panel behind a history record's `EntityId`.
    fn panel_of(&self, entity: EntityId) -> Option<PanelId> {
        self.tiles
            .iter()
            .find(|tile| tile.panel.view().entity_id() == entity)
            .map(|tile| tile.id)
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

    fn grid_size(&self, cx: &App) -> Pixels {
        self.renderer.grid_size(cx)
    }

    fn begin_move(&mut self, panel: PanelId, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(initial_bounds) = self.bounds_of(panel) else {
            return;
        };
        self.moving = Some(TileMove {
            panel,
            initial_pointer: pointer,
            initial_bounds,
        });
        cx.emit(TilesEvent::BringToFront { panel });
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
            self.grid_size(cx),
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
        let Some(initial_bounds) = self.bounds_of(panel) else {
            return;
        };
        self.resizing = Some(TileResize {
            panel,
            initial_bounds,
            drag: ResizeDrag::new(side, pointer, initial_bounds),
        });
        cx.emit(TilesEvent::BringToFront { panel });
        cx.notify();
    }

    fn resize_to(&mut self, pointer: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(resize) = self.resizing else {
            return;
        };
        let previous = resize.drag.last_bounds();
        let (new_x, new_y, new_width, new_height) = match resize.drag.side() {
            ResizeSide::Left => (Some(pointer.x), None, None, None),
            ResizeSide::Right => (
                None,
                None,
                Some((pointer.x - previous.origin.x).max(MINIMUM_SIZE.width)),
                None,
            ),
            ResizeSide::Top => (None, Some(pointer.y), None, None),
            ResizeSide::Bottom => (
                None,
                None,
                None,
                Some((pointer.y - previous.origin.y).max(MINIMUM_SIZE.height)),
            ),
            ResizeSide::BottomRight => (
                None,
                None,
                Some((pointer.x - previous.origin.x).max(MINIMUM_SIZE.width)),
                Some((pointer.y - previous.origin.y).max(MINIMUM_SIZE.height)),
            ),
        };

        let bounds = compute_resized_bounds(
            previous,
            new_x,
            new_y,
            new_width,
            new_height,
            &self.other_bounds(resize.panel),
            self.grid_size(cx),
        );

        self.resizing = Some(TileResize {
            drag: resize
                .drag
                .with_last_position(pointer)
                .with_last_bounds(bounds),
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

    /// Show the new geometry immediately and report it, so the container's
    /// tree and this mirror never disagree for a frame.
    fn apply_bounds(&mut self, panel: PanelId, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        if self.tiles[ix].bounds == bounds {
            return;
        }
        self.tiles[ix].bounds = bounds;
        cx.emit(TilesEvent::BoundsChanged { panel, bounds });
        cx.notify();
    }

    /// Push one completed gesture onto the undo stack.
    fn record(&mut self, panel: PanelId, old_bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let Some(ix) = self.index_of(panel) else {
            return;
        };
        let tile = &self.tiles[ix];
        if tile.bounds == old_bounds {
            return;
        }
        self.history.push(TileChange::bounds_change(
            tile.panel.view().entity_id(),
            old_bounds,
            tile.bounds,
        ));
        cx.notify();
    }

    /// Ask the container to close `panel`. Nothing happens for a tile that
    /// is not on this canvas, or for a panel that refuses to close.
    fn close_tile(&mut self, panel: PanelId, cx: &mut Context<Self>) {
        let closable = self
            .tiles
            .iter()
            .any(|tile| tile.id == panel && tile.panel.closable(cx));
        if !closable {
            return;
        }
        cx.emit(TilesEvent::ClosePanel { panel });
        cx.notify();
    }

    fn tile_context(&self, ix: usize, cx: &App) -> TileContext {
        let tile = &self.tiles[ix];
        let panel = tile.id;
        let canvas = self.this.clone();

        TileContext {
            node: self.node,
            panel: tile.panel.clone(),
            id: panel,
            bounds: tile.bounds,
            z_index: tile.z_index,
            moving: self.moving.is_some_and(|drag| drag.panel == panel),
            resizing: self.resizing.is_some_and(|drag| drag.panel == panel),
            closable: tile.panel.closable(cx),
            on_begin_move: {
                let canvas = canvas.clone();
                Rc::new(move |pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.begin_move(panel, pointer, cx));
                })
            },
            on_move_to: {
                let canvas = canvas.clone();
                Rc::new(move |pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.move_to(pointer, cx));
                })
            },
            on_end_move: {
                let canvas = canvas.clone();
                Rc::new(move |_, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.end_move(cx));
                })
            },
            on_begin_resize: {
                let canvas = canvas.clone();
                Rc::new(move |side, pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| {
                        canvas.begin_resize(panel, side, pointer, cx)
                    });
                })
            },
            on_resize_to: {
                let canvas = canvas.clone();
                Rc::new(move |pointer, _, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.resize_to(pointer, cx));
                })
            },
            on_end_resize: {
                let canvas = canvas.clone();
                Rc::new(move |_, cx| {
                    _ = canvas.update(cx, |canvas, cx| canvas.end_resize(cx));
                })
            },
            on_bring_to_front: {
                let canvas = canvas.clone();
                Rc::new(move |_, cx| {
                    _ = canvas.update(cx, |_, cx| {
                        cx.emit(TilesEvent::BringToFront { panel });
                    });
                })
            },
            on_close: Rc::new(move |_, cx| {
                _ = canvas.update(cx, |canvas, cx| canvas.close_tile(panel, cx));
            }),
        }
    }
}

impl EventEmitter<TilesEvent> for TilesState {}

impl Focusable for TilesState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TilesState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let renderer = self.renderer.clone();
        let focus_handle = self.focus_handle.clone();
        let tiles = self.tiles(cx);

        renderer
            .frame(window, cx)
            .track_focus(&focus_handle)
            .on_drop(cx.listener(|_, item: &AnyDrag, _, cx| {
                cx.emit(TilesEvent::DragDrop { item: item.clone() });
            }))
            .children(
                tiles
                    .into_iter()
                    .map(|tile| {
                        // The only positioning base installs anywhere in the
                        // dock. A tiles canvas *is* "panels at stored
                        // coordinates": drawing one somewhere other than its
                        // own bounds would not be a different skin, it would
                        // be a different data structure.
                        renderer
                            .tile_frame(&tile, window, cx)
                            .absolute()
                            .left(tile.bounds.origin.x)
                            .top(tile.bounds.origin.y)
                            .w(tile.bounds.size.width)
                            .h(tile.bounds.size.height)
                            .child(renderer.render_drag_bar(&tile, window, cx))
                            .child(tile.panel.view())
                            .child(renderer.render_resize_handles(&tile, window, cx))
                    })
                    .collect::<Vec<_>>(),
            )
    }
}

type MovePointerHandler = Rc<dyn Fn(Point<Pixels>, &mut Window, &mut App)>;
type ResizeStartHandler = Rc<dyn Fn(ResizeSide, Point<Pixels>, &mut Window, &mut App)>;
type GestureEndHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// What a skin needs to draw one tile, and the callbacks it invokes rather
/// than reimplementing the snapping and resize arithmetic.
#[derive(Clone)]
pub struct TileContext {
    node: NodeId,
    panel: Arc<dyn PanelView>,
    id: PanelId,
    bounds: Bounds<Pixels>,
    z_index: usize,
    moving: bool,
    resizing: bool,
    closable: bool,
    on_begin_move: MovePointerHandler,
    on_move_to: MovePointerHandler,
    on_end_move: GestureEndHandler,
    on_begin_resize: ResizeStartHandler,
    on_resize_to: MovePointerHandler,
    on_end_resize: GestureEndHandler,
    on_bring_to_front: GestureEndHandler,
    on_close: GestureEndHandler,
}

impl TileContext {
    /// The `Tiles` node this tile belongs to, for a skin that needs to name
    /// the canvas in a drag payload or a drop target.
    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn panel(&self) -> &Arc<dyn PanelView> {
        &self.panel
    }

    pub fn panel_id(&self) -> PanelId {
        self.id
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    pub fn z_index(&self) -> usize {
        self.z_index
    }

    pub fn is_moving(&self) -> bool {
        self.moving
    }

    pub fn is_resizing(&self) -> bool {
        self.resizing
    }

    pub fn can_close(&self) -> bool {
        self.closable
    }

    /// Pointer positions are in window coordinates: every gesture is resolved
    /// against the position the gesture started at, so the skin never has to
    /// convert into canvas space.
    pub fn begin_move(&self, pointer: Point<Pixels>, window: &mut Window, cx: &mut App) {
        (self.on_begin_move)(pointer, window, cx);
    }

    pub fn move_to(&self, pointer: Point<Pixels>, window: &mut Window, cx: &mut App) {
        (self.on_move_to)(pointer, window, cx);
    }

    pub fn end_move(&self, window: &mut Window, cx: &mut App) {
        (self.on_end_move)(window, cx);
    }

    pub fn begin_resize(
        &self,
        side: ResizeSide,
        pointer: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        (self.on_begin_resize)(side, pointer, window, cx);
    }

    pub fn resize_to(&self, pointer: Point<Pixels>, window: &mut Window, cx: &mut App) {
        (self.on_resize_to)(pointer, window, cx);
    }

    pub fn end_resize(&self, window: &mut Window, cx: &mut App) {
        (self.on_end_resize)(window, cx);
    }

    pub fn bring_to_front(&self, window: &mut Window, cx: &mut App) {
        (self.on_bring_to_front)(window, cx);
    }

    /// Dismiss this tile. Refused when [`Self::can_close`] is false, so a
    /// skin that offers a Close control should gate it on that.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        (self.on_close)(window, cx);
    }
}

/// Appearance for a tiles canvas. Base draws none of it.
///
/// Like [`TabGroupRenderer`](super::TabGroupRenderer), the frame hooks return
/// the element itself rather than wrapping one: base attaches focus and drop
/// handling to the canvas frame and the stored bounds to the tile frame, so a
/// wrapper would put the hit area and the painted area on different elements.
/// That is also why there is no `wrap_canvas` hook — it would be exactly the
/// wrapper the `TabGroupRenderer` review ruled out.
#[allow(unused_variables)]
pub trait TilesRenderer: 'static {
    /// The canvas frame, which base tracks focus and drop handling on.
    fn frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        div().id("tiles")
    }

    /// One tile's frame, which base positions at the tile's stored bounds.
    fn tile_frame(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        div().id("tile")
    }

    /// The strip the tile is dragged by. Its height is
    /// [`DRAG_BAR_HEIGHT`](super::DRAG_BAR_HEIGHT), which base's snapping
    /// arithmetic and the skin must agree on.
    fn render_drag_bar(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> AnyElement;

    /// The tile's resize affordances. Their hit size is
    /// [`HANDLE_SIZE`](super::HANDLE_SIZE).
    fn render_resize_handles(
        &self,
        tile: &TileContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        Empty.into_any_element()
    }

    /// The grid a tile snaps to when no neighbouring edge is close enough.
    ///
    /// The old canvas read this off the theme, which base cannot see; the
    /// default is the ten-pixel grid the original rounded to.
    fn grid_size(&self, cx: &App) -> Pixels {
        px(10.)
    }
}

/// The renderer a canvas starts with: the tiles and nothing else.
pub(crate) struct BareTiles;

impl TilesRenderer for BareTiles {
    fn render_drag_bar(&self, _: &TileContext, _: &mut Window, _: &mut App) -> AnyElement {
        Empty.into_any_element()
    }
}
