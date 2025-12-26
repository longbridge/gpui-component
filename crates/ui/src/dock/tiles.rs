use std::{
    any::Any,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use crate::{
    ActiveTheme, ElementExt, Icon, IconName, h_flex,
    history::{History, HistoryItem},
    scroll::{Scrollbar, ScrollbarShow},
    v_flex,
};

use super::{
    DockArea, Panel, PanelEvent, PanelInfo, PanelState, PanelView, StackPanel, TabPanel, TileMeta,
};
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, DismissEvent, Div, DragMoveEvent, Empty,
    EntityId, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollHandle, Size,
    StatefulInteractiveElement, Styled, WeakEntity, Window, actions, div, prelude::FluentBuilder,
    px, size,
};

actions!(tiles, [Undo, Redo]);

const MINIMUM_SIZE: Size<Pixels> = size(px(100.), px(100.));
const DRAG_BAR_HEIGHT: Pixels = px(30.);
const HANDLE_SIZE: Pixels = px(5.0);

#[derive(Clone, PartialEq, Debug)]
struct TileChange {
    tile_id: EntityId,
    old_bounds: Option<Bounds<Pixels>>,
    new_bounds: Option<Bounds<Pixels>>,
    old_order: Option<usize>,
    new_order: Option<usize>,
    version: usize,
}

impl HistoryItem for TileChange {
    fn version(&self) -> usize {
        self.version
    }

    fn set_version(&mut self, version: usize) {
        self.version = version;
    }
}

#[derive(Clone)]
pub struct DragMoving(EntityId);
impl Render for DragMoving {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, PartialEq)]
enum ResizeSide {
    Left,
    Right,
    Top,
    Bottom,
    BottomRight,
}

#[derive(Clone)]
pub struct DragResizing(EntityId);

impl Render for DragResizing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
struct ResizeDrag {
    side: ResizeSide,
    last_position: Point<Pixels>,
    last_bounds: Bounds<Pixels>,
}

/// TileItem is a moveable and resizable panel that can be added to a Tiles view.
#[derive(Clone)]
pub struct TileItem {
    id: EntityId,
    pub(crate) panel: Arc<dyn PanelView>,
    bounds: Bounds<Pixels>,
    z_index: usize,
}

impl Debug for TileItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileItem")
            .field("bounds", &self.bounds)
            .field("z_index", &self.z_index)
            .finish()
    }
}

impl TileItem {
    pub fn new(panel: Arc<dyn PanelView>, bounds: Bounds<Pixels>) -> Self {
        Self {
            id: panel.view().entity_id(),
            panel,
            bounds,
            z_index: 0,
        }
    }

    pub fn z_index(mut self, z_index: usize) -> Self {
        self.z_index = z_index;
        self
    }
}

#[derive(Clone, Debug)]
pub struct AnyDrag {
    pub value: Arc<dyn Any>,
}

impl AnyDrag {
    pub fn new(value: impl Any) -> Self {
        Self {
            value: Arc::new(value),
        }
    }
}

/// Tiles is a canvas that can contain multiple panels, each of which can be dragged and resized.
pub struct Tiles {
    focus_handle: FocusHandle,
    pub(crate) panels: Vec<TileItem>,
    dragging_id: Option<EntityId>,
    dragging_initial_mouse: Point<Pixels>,
    dragging_initial_bounds: Bounds<Pixels>,
    resizing_id: Option<EntityId>,
    resizing_drag_data: Option<ResizeDrag>,
    bounds: Bounds<Pixels>,
    history: History<TileChange>,
    scroll_handle: ScrollHandle,
    scrollbar_show: Option<ScrollbarShow>,
}

impl Panel for Tiles {
    fn panel_name(&self) -> &'static str {
        "Tiles"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Tiles".into_any_element()
    }

    fn dump(&self, cx: &App) -> PanelState {
        let panels = self
            .panels
            .iter()
            .map(|item: &TileItem| item.panel.dump(cx))
            .collect();

        let metas = self
            .panels
            .iter()
            .map(|item: &TileItem| TileMeta {
                bounds: item.bounds,
                z_index: item.z_index,
            })
            .collect();

        let mut state = PanelState::new(self);
        state.panel_name = self.panel_name().to_string();
        state.children = panels;
        state.info = PanelInfo::Tiles { metas };
        state
    }
}

#[derive(Clone, Debug)]
pub struct DragDrop(pub AnyDrag);

impl EventEmitter<DragDrop> for Tiles {}

impl Tiles {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            panels: vec![],
            dragging_id: None,
            dragging_initial_mouse: Point::default(),
            dragging_initial_bounds: Bounds::default(),
            resizing_id: None,
            scrollbar_show: None,
            resizing_drag_data: None,
            bounds: Bounds::default(),
            history: History::new().group_interval(std::time::Duration::from_millis(100)),
            scroll_handle: ScrollHandle::default(),
        }
    }

    /// Set the scrollbar show mode [`ScrollbarShow`], if not set use the `cx.theme().scrollbar_show`.
    pub fn set_scrollbar_show(
        &mut self,
        scrollbar_show: Option<ScrollbarShow>,
        cx: &mut Context<Self>,
    ) {
        self.scrollbar_show = scrollbar_show;
        cx.notify();
    }

    pub fn panels(&self) -> &[TileItem] {
        &self.panels
    }

    fn sorted_panels(&self) -> Vec<TileItem> {
        let mut items: Vec<(usize, TileItem)> = self.panels.iter().cloned().enumerate().collect();
        items.sort_by(|a, b| a.1.z_index.cmp(&b.1.z_index).then_with(|| a.0.cmp(&b.0)));
        items.into_iter().map(|(_, item)| item).collect()
    }

    /// Return the index of the panel.
    #[inline]
    pub(crate) fn index_of(&self, id: &EntityId) -> Option<usize> {
        self.panels.iter().position(|p| &p.id == id)
    }

    #[inline]
    pub(crate) fn panel(&self, id: &EntityId) -> Option<&TileItem> {
        self.panels.iter().find(|p| &p.id == id)
    }

    /// Remove panel from the children.
    pub fn remove(&mut self, panel: Arc<dyn PanelView>, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.index_of(&panel.panel_id(cx)) {
            self.panels.remove(ix);

            cx.emit(PanelEvent::LayoutChanged);
        }
    }

    /// Calculate magnetic snap position for the dragging panel
    fn calculate_magnetic_snap(
        &self,
        dragging_bounds: Bounds<Pixels>,
        item_ix: usize,
        snap_threshold: Pixels,
    ) -> (Option<Pixels>, Option<Pixels>) {
        // Only check nearby panels
        let search_bounds = Bounds {
            origin: Point {
                x: dragging_bounds.left() - snap_threshold,
                y: dragging_bounds.top() - snap_threshold,
            },
            size: Size {
                width: dragging_bounds.size.width + snap_threshold * 2.0,
                height: dragging_bounds.size.height + snap_threshold * 2.0,
            },
        };

        let mut snap_x: Option<Pixels> = None;
        let mut snap_y: Option<Pixels> = None;
        let mut min_x_dist = snap_threshold;
        let mut min_y_dist = snap_threshold;

        // Pre-calculate dragging bounds edges to avoid repeated method calls
        let drag_left = dragging_bounds.left();
        let drag_right = dragging_bounds.right();
        let drag_top = dragging_bounds.top();
        let drag_bottom = dragging_bounds.bottom();
        let drag_width = dragging_bounds.size.width;
        let drag_height = dragging_bounds.size.height;

        // Check for edge snapping first (top and left boundaries)
        let edge_snap_pos = px(0.);

        // Snap to top edge
        let top_dist = drag_top.abs();
        if top_dist < snap_threshold {
            snap_y = Some(edge_snap_pos);
            min_y_dist = top_dist;
        }

        // Snap to left edge
        let left_dist = drag_left.abs();
        if left_dist < snap_threshold {
            snap_x = Some(edge_snap_pos);
            min_x_dist = left_dist;
        }

        // If both edges are snapped, return early
        if snap_x.is_some() && snap_y.is_some() {
            return (snap_x, snap_y);
        }

        for (ix, other) in self.panels.iter().enumerate() {
            if ix == item_ix {
                continue;
            }

            // Pre-calculate other bounds edges
            let other_left = other.bounds.left();
            let other_right = other.bounds.right();
            let other_top = other.bounds.top();
            let other_bottom = other.bounds.bottom();

            // Skip panels that are far away
            if other_right < search_bounds.left()
                || other_left > search_bounds.right()
                || other_bottom < search_bounds.top()
                || other_top > search_bounds.bottom()
            {
                continue;
            }

            // Horizontal snapping (X axis) - find closest snap point
            if snap_x.is_none() {
                let candidates = [
                    ((drag_left - other_left).abs(), other_left),
                    ((drag_left - other_right).abs(), other_right),
                    ((drag_right - other_left).abs(), other_left - drag_width),
                    ((drag_right - other_right).abs(), other_right - drag_width),
                ];

                for (dist, snap_pos) in candidates {
                    if dist < min_x_dist {
                        min_x_dist = dist;
                        snap_x = Some(snap_pos);
                    }
                }
            }

            // Vertical snapping (Y axis) - find closest snap point
            if snap_y.is_none() {
                let candidates = [
                    ((drag_top - other_top).abs(), other_top),
                    ((drag_top - other_bottom).abs(), other_bottom),
                    ((drag_bottom - other_top).abs(), other_top - drag_height),
                    (
                        (drag_bottom - other_bottom).abs(),
                        other_bottom - drag_height,
                    ),
                ];

                for (dist, snap_pos) in candidates {
                    if dist < min_y_dist {
                        min_y_dist = dist;
                        snap_y = Some(snap_pos);
                    }
                }
            }

            // Early exit if both axes are snapped
            if snap_x.is_some() && snap_y.is_some() {
                break;
            }
        }

        (snap_x, snap_y)
    }

    /// Apply boundary constraints to the panel origin
    fn apply_boundary_constraints(&self, mut origin: Point<Pixels>) -> Point<Pixels> {
        // Top boundary
        if origin.y < px(0.) {
            origin.y = px(0.);
        }

        // Left boundary (allow partial off-screen but keep 64px visible)
        let min_left = -self.dragging_initial_bounds.size.width + px(64.);
        if origin.x < min_left {
            origin.x = min_left;
        }

        origin
    }

    fn update_position(&mut self, mouse_position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(dragging_id) = self.dragging_id else {
            return;
        };

        let Some(item_ix) = self.panels.iter().position(|p| p.id == dragging_id) else {
            return;
        };

        let previous_bounds = self.panels[item_ix].bounds;
        let adjusted_position = mouse_position - self.bounds.origin;
        let delta = adjusted_position - self.dragging_initial_mouse;
        let mut new_origin = self.dragging_initial_bounds.origin + delta;

        // Apply magnetic snap before boundary checks
        let snap_threshold = cx.theme().tile_grid_size;
        let dragging_bounds = Bounds {
            origin: new_origin,
            size: self.dragging_initial_bounds.size,
        };

        let (snap_x, snap_y) =
            self.calculate_magnetic_snap(dragging_bounds, item_ix, snap_threshold);

        // Apply snapping
        if let Some(x) = snap_x {
            new_origin.x = x;
        }
        if let Some(y) = snap_y {
            new_origin.y = y;
        }

        // Apply boundary constraints after snapping
        new_origin = self.apply_boundary_constraints(new_origin);

        // Update position without grid rounding (smooth dragging)
        if new_origin != previous_bounds.origin {
            self.panels[item_ix].bounds.origin = new_origin;
            let item = &self.panels[item_ix];
            let bounds = item.bounds;
            let entity_id = item.panel.view().entity_id();

            if !self.history.ignore {
                self.history.push(TileChange {
                    tile_id: entity_id,
                    old_bounds: Some(previous_bounds),
                    new_bounds: Some(bounds),
                    old_order: None,
                    new_order: None,
                    version: 0,
                });
            }
            cx.notify();
        }
    }

    fn resize(
        &mut self,
        new_x: Option<Pixels>,
        new_y: Option<Pixels>,
        new_width: Option<Pixels>,
        new_height: Option<Pixels>,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(resizing_id) = self.resizing_id else {
            return;
        };
        let Some(item) = self.panels.iter_mut().find(|item| item.id == resizing_id) else {
            return;
        };

        let previous_bounds = item.bounds;
        let final_x = if let Some(x) = new_x {
            round_to_nearest_ten(x, cx)
        } else {
            previous_bounds.origin.x
        };
        let final_y = if let Some(y) = new_y {
            round_to_nearest_ten(y, cx)
        } else {
            previous_bounds.origin.y
        };
        let final_width = if let Some(width) = new_width {
            round_to_nearest_ten(width, cx)
        } else {
            previous_bounds.size.width
        };

        let final_height = if let Some(height) = new_height {
            round_to_nearest_ten(height, cx)
        } else {
            previous_bounds.size.height
        };

        // Only push to history if size has changed
        if final_width != item.bounds.size.width
            || final_height != item.bounds.size.height
            || final_x != item.bounds.origin.x
            || final_y != item.bounds.origin.y
        {
            item.bounds.origin.x = final_x;
            item.bounds.origin.y = final_y;
            item.bounds.size.width = final_width;
            item.bounds.size.height = final_height;

            // Only push if not during history operations
            if !self.history.ignore {
                self.history.push(TileChange {
                    tile_id: item.panel.view().entity_id(),
                    old_bounds: Some(previous_bounds),
                    new_bounds: Some(item.bounds),
                    old_order: None,
                    new_order: None,
                    version: 0,
                });
            }
        }

        cx.notify();
    }

    pub fn add_item(
        &mut self,
        item: TileItem,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(tab_panel) = item.panel.view().downcast::<TabPanel>() else {
            panic!("only allows to add TabPanel type")
        };

        tab_panel.update(cx, |tab_panel, _| {
            tab_panel.set_in_tiles(true);
        });

        self.panels.push(item.clone());
        window.defer(cx, {
            let panel = item.panel.clone();
            let dock_area = dock_area.clone();

            move |window, cx| {
                // Subscribe to the panel's layout change event.
                _ = dock_area.update(cx, |this, cx| {
                    if let Ok(tab_panel) = panel.view().downcast::<TabPanel>() {
                        this.subscribe_panel(&tab_panel, window, cx);
                    }
                });
            }
        });

        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    #[inline]
    fn reset_current_index(&mut self) {
        self.dragging_id = None;
        self.resizing_id = None;
    }

    /// Bring the panel of target_index to front, returns (old_index, new_index) if successful
    fn bring_to_front(
        &mut self,
        target_id: Option<EntityId>,
        cx: &mut Context<Self>,
    ) -> Option<EntityId> {
        let Some(old_id) = target_id else {
            return None;
        };

        let old_ix = self.panels.iter().position(|item| item.id == old_id)?;
        if old_ix < self.panels.len() {
            let item = self.panels.remove(old_ix);
            self.panels.push(item);
            let new_ix = self.panels.len() - 1;
            let new_id = self.panels[new_ix].id;
            self.history.push(TileChange {
                tile_id: new_id,
                old_bounds: None,
                new_bounds: None,
                old_order: Some(old_ix),
                new_order: Some(new_ix),
                version: 0,
            });
            cx.notify();
            return Some(new_id);
        }
        None
    }

    /// Handle the undo action
    pub fn undo(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.history.ignore = true;

        if let Some(changes) = self.history.undo() {
            for change in changes {
                if let Some(index) = self
                    .panels
                    .iter()
                    .position(|item| item.panel.view().entity_id() == change.tile_id)
                {
                    if let Some(old_bounds) = change.old_bounds {
                        self.panels[index].bounds = old_bounds;
                    }
                    if let Some(old_order) = change.old_order {
                        let item = self.panels.remove(index);
                        self.panels.insert(old_order, item);
                    }
                }
            }
            cx.emit(PanelEvent::LayoutChanged);
        }

        self.history.ignore = false;
        cx.notify();
    }

    /// Handle the redo action
    pub fn redo(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.history.ignore = true;

        if let Some(changes) = self.history.redo() {
            for change in changes {
                if let Some(index) = self
                    .panels
                    .iter()
                    .position(|item| item.panel.view().entity_id() == change.tile_id)
                {
                    if let Some(new_bounds) = change.new_bounds {
                        self.panels[index].bounds = new_bounds;
                    }
                    if let Some(new_order) = change.new_order {
                        let item = self.panels.remove(index);
                        self.panels.insert(new_order, item);
                    }
                }
            }
            cx.emit(PanelEvent::LayoutChanged);
        }

        self.history.ignore = false;
        cx.notify();
    }

    /// Returns the active panel, if any.
    pub fn active_panel(&self, cx: &App) -> Option<Arc<dyn PanelView>> {
        self.panels.last().and_then(|item| {
            if let Ok(tab_panel) = item.panel.view().downcast::<TabPanel>() {
                tab_panel.read(cx).active_panel(cx)
            } else if let Ok(_) = item.panel.view().downcast::<StackPanel>() {
                None
            } else {
                Some(item.panel.clone())
            }
        })
    }

    /// Produce a vector of AnyElement representing the three possible resize handles
    fn render_resize_handles(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
        entity_id: EntityId,
        item: &TileItem,
    ) -> Vec<AnyElement> {
        let item_id = item.id;
        let item_bounds = item.bounds;
        let handle_offset = -HANDLE_SIZE + px(1.);

        let mut elements = Vec::new();

        // Left resize handle
        elements.push(
            div()
                .id("left-resize-handle")
                .cursor_ew_resize()
                .absolute()
                .top_0()
                .left(handle_offset)
                .w(HANDLE_SIZE)
                .h(item_bounds.size.height)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Left,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };
                            if drag_data.side != ResizeSide::Left {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = drag_data.last_position.x - pos.x;
                            let new_x = (drag_data.last_bounds.origin.x - delta).max(px(0.0));
                            let size_delta = drag_data.last_bounds.origin.x - new_x;
                            let new_width = (drag_data.last_bounds.size.width + size_delta)
                                .max(MINIMUM_SIZE.width);
                            this.resize(Some(new_x), None, Some(new_width), None, window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Right resize handle
        elements.push(
            div()
                .id("right-resize-handle")
                .cursor_ew_resize()
                .absolute()
                .top_0()
                .right(handle_offset)
                .w(HANDLE_SIZE)
                .h(item_bounds.size.height)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Right,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };

                            if drag_data.side != ResizeSide::Right {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = pos.x - drag_data.last_position.x;
                            let new_width =
                                (drag_data.last_bounds.size.width + delta).max(MINIMUM_SIZE.width);
                            this.resize(None, None, Some(new_width), None, window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Top resize handle
        elements.push(
            div()
                .id("top-resize-handle")
                .cursor_ns_resize()
                .absolute()
                .left(px(0.0))
                .top(handle_offset)
                .w(item_bounds.size.width)
                .h(HANDLE_SIZE)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Top,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };
                            if drag_data.side != ResizeSide::Top {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = drag_data.last_position.y - pos.y;
                            let new_y = (drag_data.last_bounds.origin.y - delta).max(px(0.));
                            let size_delta = drag_data.last_position.y - new_y;
                            let new_height = (drag_data.last_bounds.size.height + size_delta)
                                .max(MINIMUM_SIZE.width);
                            this.resize(None, Some(new_y), None, Some(new_height), window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Bottom resize handle
        elements.push(
            div()
                .id("bottom-resize-handle")
                .cursor_ns_resize()
                .absolute()
                .left(px(0.0))
                .bottom(handle_offset)
                .w(item_bounds.size.width)
                .h(HANDLE_SIZE)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            this.on_resize_handle_mouse_down(
                                ResizeSide::Bottom,
                                item_id,
                                item_bounds,
                                event,
                                window,
                                cx,
                            );
                        }
                    }),
                )
                .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragResizing>, window, cx| match e.drag(cx) {
                        DragResizing(id) => {
                            if *id != entity_id {
                                return;
                            }

                            let Some(ref drag_data) = this.resizing_drag_data else {
                                return;
                            };

                            if drag_data.side != ResizeSide::Bottom {
                                return;
                            }

                            let pos = e.event.position;
                            let delta = pos.y - drag_data.last_position.y;
                            let new_height =
                                (drag_data.last_bounds.size.height + delta).max(MINIMUM_SIZE.width);
                            this.resize(None, None, None, Some(new_height), window, cx);
                        }
                    },
                ))
                .into_any_element(),
        );

        // Corner resize handle
        elements.push(
            div()
                .child(
                    Icon::new(IconName::ResizeCorner)
                        .size_3()
                        .absolute()
                        .right(px(1.))
                        .bottom(px(1.))
                        .text_color(cx.theme().muted_foreground.opacity(0.5)),
                )
                .child(
                    div()
                        .id("corner-resize-handle")
                        .cursor_nwse_resize()
                        .absolute()
                        .right(handle_offset)
                        .bottom(handle_offset)
                        .size_3()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener({
                                move |this, event: &MouseDownEvent, window, cx| {
                                    this.on_resize_handle_mouse_down(
                                        ResizeSide::BottomRight,
                                        item_id,
                                        item_bounds,
                                        event,
                                        window,
                                        cx,
                                    );
                                }
                            }),
                        )
                        .on_drag(DragResizing(entity_id), |drag, _, _, cx| {
                            cx.stop_propagation();
                            cx.new(|_| drag.clone())
                        })
                        .on_drag_move(cx.listener(
                            move |this, e: &DragMoveEvent<DragResizing>, window, cx| {
                                match e.drag(cx) {
                                    DragResizing(id) => {
                                        if *id != entity_id {
                                            return;
                                        }

                                        let Some(ref drag_data) = this.resizing_drag_data else {
                                            return;
                                        };

                                        if drag_data.side != ResizeSide::BottomRight {
                                            return;
                                        }

                                        let pos = e.event.position;
                                        let delta_x = pos.x - drag_data.last_position.x;
                                        let delta_y = pos.y - drag_data.last_position.y;
                                        let new_width = (drag_data.last_bounds.size.width
                                            + delta_x)
                                            .max(MINIMUM_SIZE.width);
                                        let new_height = (drag_data.last_bounds.size.height
                                            + delta_y)
                                            .max(MINIMUM_SIZE.height);
                                        this.resize(
                                            None,
                                            None,
                                            Some(new_width),
                                            Some(new_height),
                                            window,
                                            cx,
                                        );
                                    }
                                }
                            },
                        )),
                )
                .into_any_element(),
        );

        elements
    }

    fn on_resize_handle_mouse_down(
        &mut self,
        side: ResizeSide,
        item_id: EntityId,
        item_bounds: Bounds<Pixels>,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let last_position = event.position;
        self.resizing_id = Some(item_id);
        self.resizing_drag_data = Some(ResizeDrag {
            side,
            last_position,
            last_bounds: item_bounds,
        });

        if let Some(new_id) = self.bring_to_front(self.resizing_id, cx) {
            self.resizing_id = Some(new_id);
        }
        cx.stop_propagation();
    }

    /// Produce the drag-bar element for the given panel item
    fn render_drag_bar(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
        entity_id: EntityId,
        item: &TileItem,
    ) -> AnyElement {
        let item_id = item.id;
        let item_bounds = item.bounds;

        h_flex()
            .id("drag-bar")
            .absolute()
            .w_full()
            .h(DRAG_BAR_HEIGHT)
            .bg(cx.theme().transparent)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    let inner_pos = event.position - this.bounds.origin;
                    this.dragging_id = Some(item_id);
                    this.dragging_initial_mouse = inner_pos;
                    this.dragging_initial_bounds = item_bounds;

                    if let Some(new_id) = this.bring_to_front(Some(item_id), cx) {
                        this.dragging_id = Some(new_id);
                    }
                }),
            )
            .on_drag(DragMoving(entity_id), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move(
                cx.listener(
                    move |this, e: &DragMoveEvent<DragMoving>, _, cx| match e.drag(cx) {
                        DragMoving(id) => {
                            if *id != entity_id {
                                return;
                            }
                            this.update_position(e.event.position, cx);
                        }
                    },
                ),
            )
            .into_any_element()
    }

    fn render_panel(
        &mut self,
        item: &TileItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let entity_id = cx.entity_id();
        let item_id = item.id;
        let panel_view = item.panel.view();

        v_flex()
            .occlude()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .absolute()
            .left(item.bounds.origin.x)
            .top(item.bounds.origin.y)
            // More 1px to account for the border width when 2 panels are too close
            .w(item.bounds.size.width + px(1.))
            .h(item.bounds.size.height + px(1.))
            .rounded(cx.theme().tile_radius)
            .child(h_flex().overflow_hidden().size_full().child(panel_view))
            .children(self.render_resize_handles(window, cx, entity_id, &item))
            .child(self.render_drag_bar(window, cx, entity_id, &item))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, _| {
                    this.dragging_id = Some(item_id);
                }),
            )
            // Here must be mouse up for avoid conflict with Drag event
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if this.dragging_id == Some(item_id) {
                        this.dragging_id = None;
                        this.bring_to_front(Some(item_id), cx);
                    }
                }),
            )
    }

    /// Handle the mouse up event to finalize drag or resize operations
    fn on_mouse_up(&mut self, _: &mut Window, cx: &mut Context<'_, Tiles>) {
        // Check if a drag or resize was active
        if self.dragging_id.is_some()
            || self.resizing_id.is_some()
            || self.resizing_drag_data.is_some()
        {
            let mut changes_to_push = vec![];

            // Handle dragging
            if let Some(dragging_id) = self.dragging_id {
                if let Some(idx) = self.panels.iter().position(|p| p.id == dragging_id) {
                    let initial_bounds = self.dragging_initial_bounds;
                    let current_bounds = self.panels[idx].bounds;

                    // Apply grid alignment to final position
                    let aligned_origin = round_point_to_nearest_ten(current_bounds.origin, cx);

                    if initial_bounds.origin != aligned_origin
                        || initial_bounds.size != current_bounds.size
                    {
                        self.panels[idx].bounds.origin = aligned_origin;

                        changes_to_push.push(TileChange {
                            tile_id: self.panels[idx].panel.view().entity_id(),
                            old_bounds: Some(initial_bounds),
                            new_bounds: Some(self.panels[idx].bounds),
                            old_order: None,
                            new_order: None,
                            version: 0,
                        });
                    }
                }
            }

            // Handle resizing
            if let Some(resizing_id) = self.resizing_id {
                if let Some(drag_data) = &self.resizing_drag_data {
                    if let Some(item) = self.panel(&resizing_id) {
                        let initial_bounds = drag_data.last_bounds;
                        let current_bounds = item.bounds;
                        if initial_bounds.size != current_bounds.size {
                            changes_to_push.push(TileChange {
                                tile_id: item.panel.view().entity_id(),
                                old_bounds: Some(initial_bounds),
                                new_bounds: Some(current_bounds),
                                old_order: None,
                                new_order: None,
                                version: 0,
                            });
                        }
                    }
                }
            }

            // Push changes to history if any
            if !changes_to_push.is_empty() {
                for change in changes_to_push {
                    self.history.push(change);
                }
            }

            // Reset drag and resize state
            self.reset_current_index();
            self.resizing_drag_data = None;
            cx.emit(PanelEvent::LayoutChanged);
            cx.notify();
        }
    }
}

#[inline]
fn round_to_nearest_ten(value: Pixels, cx: &App) -> Pixels {
    (value / cx.theme().tile_grid_size).round() * cx.theme().tile_grid_size
}

#[inline]
fn round_point_to_nearest_ten(point: Point<Pixels>, cx: &App) -> Point<Pixels> {
    Point::new(
        round_to_nearest_ten(point.x, cx),
        round_to_nearest_ten(point.y, cx),
    )
}

impl Focusable for Tiles {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<PanelEvent> for Tiles {}
impl EventEmitter<DismissEvent> for Tiles {}
impl Render for Tiles {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let panels = self.sorted_panels();
        let scroll_bounds =
            self.panels
                .iter()
                .fold(Bounds::default(), |acc: Bounds<Pixels>, item| Bounds {
                    origin: Point {
                        x: acc.origin.x.min(item.bounds.origin.x),
                        y: acc.origin.y.min(item.bounds.origin.y),
                    },
                    size: Size {
                        width: acc.size.width.max(item.bounds.right()),
                        height: acc.size.height.max(item.bounds.bottom()),
                    },
                });
        let scroll_size = scroll_bounds.size - size(scroll_bounds.origin.x, scroll_bounds.origin.y);

        div()
            .relative()
            .bg(cx.theme().tiles)
            .child(
                div()
                    .id("tiles")
                    .track_scroll(&self.scroll_handle)
                    .size_full()
                    .top(-px(1.))
                    .left(-px(1.))
                    .overflow_scroll()
                    .children(
                        panels
                            .into_iter()
                            .map(|item| self.render_panel(&item, window, cx)),
                    )
                    .on_prepaint(move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds))
                    .on_drop(cx.listener(move |_, item: &AnyDrag, _, cx| {
                        cx.emit(DragDrop(item.clone()));
                    })),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _event: &MouseUpEvent, window, cx| {
                    this.on_mouse_up(window, cx);
                }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .child(
                        Scrollbar::new(&self.scroll_handle)
                            .scroll_size(scroll_size)
                            .when_some(self.scrollbar_show, |this, scrollbar_show| {
                                this.scrollbar_show(scrollbar_show)
                            }),
                    ),
            )
            .size_full()
    }
}
