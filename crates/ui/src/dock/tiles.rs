use std::{
    any::Any,
    cell::Cell,
    fmt::{Debug, Formatter},
    rc::Rc,
    sync::Arc,
};

use crate::{
    h_flex,
    history::{History, HistoryItem},
    scroll::{Scrollbar, ScrollbarState},
    v_flex, ActiveTheme, Icon, IconName,
};

use super::{
    DockArea, Panel, PanelEvent, PanelInfo, PanelState, PanelView, StackPanel, TabPanel, TileMeta,
};
use gpui::{
    actions, canvas, div, point, px, size, AnyElement, App, AppContext, Bounds, Context,
    DismissEvent, DragMoveEvent, Empty, EntityId, EventEmitter, FocusHandle, Focusable, Half,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement,
    Pixels, Point, Render, ScrollHandle, Size, StatefulInteractiveElement, Styled, WeakEntity,
    Window,
};

actions!(tiles, [Undo, Redo,]);

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

#[derive(Clone)]
pub struct DragResizing(EntityId);
impl Render for DragResizing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
struct ResizeDrag {
    axis: ResizeAxis,
    last_position: Point<Pixels>,
    last_bounds: Bounds<Pixels>,
}

#[derive(Clone, PartialEq)]
enum ResizeAxis {
    Horizontal,
    Vertical,
    Both,
}

/// TileItem is a moveable and resizable panel that can be added to a Tiles view.
#[derive(Clone)]
pub struct TileItem {
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
    dragging_index: Option<usize>,
    dragging_initial_mouse: Point<Pixels>,
    dragging_initial_bounds: Bounds<Pixels>,
    resizing_index: Option<usize>,
    resizing_drag_data: Option<ResizeDrag>,
    bounds: Bounds<Pixels>,
    history: History<TileChange>,
    scroll_state: Rc<Cell<ScrollbarState>>,
    scroll_handle: ScrollHandle,
}

impl Panel for Tiles {
    fn panel_name(&self) -> &'static str {
        "Tiles"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
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
            dragging_index: None,
            dragging_initial_mouse: Point::default(),
            dragging_initial_bounds: Bounds::default(),
            resizing_index: None,
            resizing_drag_data: None,
            bounds: Bounds::default(),
            history: History::new().group_interval(std::time::Duration::from_millis(100)),
            scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_handle: ScrollHandle::default(),
        }
    }

    fn sorted_panels(&self) -> Vec<TileItem> {
        let mut items: Vec<(usize, TileItem)> = self.panels.iter().cloned().enumerate().collect();
        items.sort_by(|a, b| a.1.z_index.cmp(&b.1.z_index).then_with(|| a.0.cmp(&b.0)));
        items.into_iter().map(|(_, item)| item).collect()
    }

    /// Return the index of the panel.
    #[inline]
    pub(crate) fn index_of(&self, panel: Arc<dyn PanelView>) -> Option<usize> {
        self.panels.iter().position(|p| &p.panel == &panel)
    }

    /// Remove panel from the children.
    pub fn remove(&mut self, panel: Arc<dyn PanelView>, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.index_of(panel.clone()) {
            self.panels.remove(ix);

            cx.emit(PanelEvent::LayoutChanged);
        }
    }

    fn update_initial_position(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((index, item)) = self.find_at_position(position) else {
            return;
        };

        let inner_pos = position - self.bounds.origin;
        let bounds = item.bounds;
        self.dragging_index = Some(index);
        self.dragging_initial_mouse = inner_pos;
        self.dragging_initial_bounds = bounds;
        cx.notify();
    }

    fn update_position(
        &mut self,
        mouse_position: Point<Pixels>,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(index) = self.dragging_index else {
            return;
        };

        let Some(item) = self.panels.get_mut(index) else {
            return;
        };

        let previous_bounds = item.bounds;
        let adjusted_position = mouse_position - self.bounds.origin;
        let delta = adjusted_position - self.dragging_initial_mouse;
        let mut new_origin = self.dragging_initial_bounds.origin + delta;

        // Avoid out of bounds
        if new_origin.y < px(0.) {
            new_origin.y = px(0.);
        }
        let min_left = -self.dragging_initial_bounds.size.width + px(64.);
        if new_origin.x < min_left {
            new_origin.x = min_left;
        }

        let final_origin = round_point_to_nearest_ten(new_origin, cx);
        // Only push to history if bounds have changed
        if final_origin != previous_bounds.origin {
            item.bounds.origin = final_origin;

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

    fn update_resizing_drag(
        &mut self,
        drag_data: ResizeDrag,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some((index, _item)) = self.find_at_position(drag_data.last_position) {
            self.resizing_index = Some(index);
            self.resizing_drag_data = Some(drag_data);
            cx.notify();
        }
    }

    fn resize_width(&mut self, new_width: Pixels, _: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(index) = self.resizing_index {
            if let Some(item) = self.panels.get_mut(index) {
                let previous_bounds = item.bounds;
                let final_width = round_to_nearest_ten(new_width, cx);

                // Only push to history if width has changed
                if final_width != item.bounds.size.width {
                    item.bounds.size.width = final_width;

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
        }
    }

    fn resize_height(&mut self, new_height: Pixels, _: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(index) = self.resizing_index {
            if let Some(item) = self.panels.get_mut(index) {
                let previous_bounds = item.bounds;
                let final_height = round_to_nearest_ten(new_height, cx);

                // Only push to history if height has changed
                if final_height != item.bounds.size.height {
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
        }
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

    /// Find the panel at a given position, considering z-index
    fn find_at_position(&self, position: Point<Pixels>) -> Option<(usize, &TileItem)> {
        let inner_pos = position - self.bounds.origin;
        let mut panels_with_indices: Vec<(usize, &TileItem)> =
            self.panels.iter().enumerate().collect();

        panels_with_indices
            .sort_by(|a, b| b.1.z_index.cmp(&a.1.z_index).then_with(|| b.0.cmp(&a.0)));

        for (index, item) in panels_with_indices {
            let extended_bounds = Bounds::new(
                item.bounds.origin,
                item.bounds.size + size(HANDLE_SIZE, HANDLE_SIZE) / 2.0,
            );
            if extended_bounds.contains(&inner_pos) {
                return Some((index, item));
            }
        }

        None
    }

    #[inline]
    fn reset_current_index(&mut self) {
        self.dragging_index = None;
        self.resizing_index = None;
    }

    /// Bring the panel of target_index to front, returns (old_index, new_index) if successful
    fn bring_to_front(
        &mut self,
        target_index: Option<usize>,
        cx: &mut Context<Self>,
    ) -> Option<(usize, usize)> {
        if let Some(old_index) = target_index {
            if old_index < self.panels.len() {
                let item = self.panels.remove(old_index);
                self.panels.push(item);
                let new_index = self.panels.len() - 1;
                self.history.push(TileChange {
                    tile_id: self.panels[new_index].panel.view().entity_id(),
                    old_bounds: None,
                    new_bounds: None,
                    old_order: Some(old_index),
                    new_order: Some(new_index),
                    version: 0,
                });
                cx.notify();
                return Some((old_index, new_index));
            }
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
        is_occluded: impl Fn(&Bounds<Pixels>) -> bool,
    ) -> Vec<AnyElement> {
        let panel_bounds = item.bounds;
        let right_handle_bounds = Bounds::new(
            panel_bounds.origin + point(panel_bounds.size.width - HANDLE_SIZE, px(0.0)),
            size(HANDLE_SIZE, panel_bounds.size.height),
        );

        let bottom_handle_bounds = Bounds::new(
            panel_bounds.origin + point(px(0.0), panel_bounds.size.height - HANDLE_SIZE.half()),
            size(panel_bounds.size.width, HANDLE_SIZE.half()),
        );

        let corner_handle_bounds = Bounds::new(
            panel_bounds.origin
                + point(
                    panel_bounds.size.width - HANDLE_SIZE.half(),
                    panel_bounds.size.height - HANDLE_SIZE.half(),
                ),
            size(HANDLE_SIZE.half(), HANDLE_SIZE.half()),
        );
        let handle_offset = -HANDLE_SIZE + px(1.);

        let mut elements = Vec::new();

        // Right resize handle
        elements.push(if !is_occluded(&right_handle_bounds) {
            div()
                .id("right-resize-handle")
                .cursor_ew_resize()
                .absolute()
                .top_0()
                .right(handle_offset)
                .w(HANDLE_SIZE)
                .h(panel_bounds.size.height)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            let last_position = event.position;
                            let drag_data = ResizeDrag {
                                axis: ResizeAxis::Horizontal,
                                last_position,
                                last_bounds: panel_bounds,
                            };
                            this.update_resizing_drag(drag_data, window, cx);
                            if let Some((_, new_ix)) = this.bring_to_front(this.resizing_index, cx)
                            {
                                this.resizing_index = Some(new_ix);
                            }
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

                            if let Some(ref drag_data) = this.resizing_drag_data {
                                if drag_data.axis != ResizeAxis::Horizontal {
                                    return;
                                }
                                let pos = e.event.position;
                                let delta = pos.x - drag_data.last_position.x;
                                let new_width = (drag_data.last_bounds.size.width + delta)
                                    .max(MINIMUM_SIZE.width);
                                this.resize_width(new_width, window, cx);
                            }
                        }
                    },
                ))
                .into_any_element()
        } else {
            div().into_any_element()
        });

        // Bottom resize handle
        elements.push(if !is_occluded(&bottom_handle_bounds) {
            div()
                .id("bottom-resize-handle")
                .cursor_ns_resize()
                .absolute()
                .left(px(0.0))
                .bottom(handle_offset)
                .w(panel_bounds.size.width)
                .h(HANDLE_SIZE)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        move |this, event: &MouseDownEvent, window, cx| {
                            let last_position = event.position;
                            let drag_data = ResizeDrag {
                                axis: ResizeAxis::Vertical,
                                last_position,
                                last_bounds: panel_bounds,
                            };
                            this.update_resizing_drag(drag_data, window, cx);
                            if let Some((_, new_ix)) = this.bring_to_front(this.resizing_index, cx)
                            {
                                this.resizing_index = Some(new_ix);
                            }
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

                            if let Some(ref drag_data) = this.resizing_drag_data {
                                let pos = e.event.position;
                                let delta = pos.y - drag_data.last_position.y;
                                let new_height = (drag_data.last_bounds.size.height + delta)
                                    .max(MINIMUM_SIZE.width);
                                this.resize_height(new_height, window, cx);
                            }
                        }
                    },
                ))
                .into_any_element()
        } else {
            div().into_any_element()
        });

        // Corner resize handle
        elements.push(if !is_occluded(&corner_handle_bounds) {
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
                                    let last_position = event.position;
                                    let drag_data = ResizeDrag {
                                        axis: ResizeAxis::Both,
                                        last_position,
                                        last_bounds: panel_bounds,
                                    };
                                    this.update_resizing_drag(drag_data, window, cx);
                                    if let Some((_, new_ix)) =
                                        this.bring_to_front(this.resizing_index, cx)
                                    {
                                        this.resizing_index = Some(new_ix);
                                    }
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

                                        if let Some(ref drag_data) = this.resizing_drag_data {
                                            if drag_data.axis != ResizeAxis::Both {
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
                                            this.resize_height(new_height, window, cx);
                                            this.resize_width(new_width, window, cx);
                                        }
                                    }
                                }
                            },
                        )),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        });

        elements
    }

    /// Produce the drag-bar element for the given panel item
    fn render_drag_bar(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
        entity_id: EntityId,
        item: &TileItem,
        is_occluded: &impl Fn(&Bounds<Pixels>) -> bool,
    ) -> AnyElement {
        let drag_bar_bounds = Bounds::new(
            item.bounds.origin,
            Size {
                width: item.bounds.size.width,
                height: DRAG_BAR_HEIGHT,
            },
        );

        if !is_occluded(&drag_bar_bounds) {
            h_flex()
                .id("drag-bar")
                .absolute()
                .w_full()
                .h(DRAG_BAR_HEIGHT)
                .bg(cx.theme().transparent)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        let last_position = event.position;
                        this.update_initial_position(last_position, window, cx);
                        if let Some((_, new_ix)) = this.bring_to_front(this.dragging_index, cx) {
                            this.dragging_index = Some(new_ix);
                        }
                    }),
                )
                .on_drag(DragMoving(entity_id), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &DragMoveEvent<DragMoving>, window, cx| match e.drag(cx) {
                        DragMoving(id) => {
                            if *id != entity_id {
                                return;
                            }
                            this.update_position(e.event.position, window, cx);
                        }
                    },
                ))
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }

    fn render_panel(
        &mut self,
        item: &TileItem,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entity_id = cx.entity_id();
        let panel_view = item.panel.view();
        let is_occluded = {
            let panels = self.panels.clone();
            move |bounds: &Bounds<Pixels>| {
                let this_z = panels[ix].z_index;
                let this_ix = ix;
                panels.iter().enumerate().any(|(sub_ix, other_item)| {
                    if sub_ix == this_ix {
                        return false;
                    }
                    let other_is_above = (other_item.z_index > this_z)
                        || (other_item.z_index == this_z && sub_ix > this_ix);

                    other_is_above && other_item.bounds.intersects(bounds)
                })
            }
        };

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
            .rounded(cx.theme().radius)
            .child(h_flex().overflow_hidden().size_full().child(panel_view))
            .children(self.render_resize_handles(window, cx, entity_id, &item, &is_occluded))
            .child(self.render_drag_bar(window, cx, entity_id, &item, &is_occluded))
            // Here must be mouse up for avoid conflict with Drag event
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.bring_to_front(Some(ix), cx);
                }),
            )
    }

    /// Handle the mouse up event to finalize drag or resize operations
    fn on_mouse_up(&mut self, _: &mut Window, cx: &mut Context<'_, Tiles>) {
        // Check if a drag or resize was active
        if self.dragging_index.is_some()
            || self.resizing_index.is_some()
            || self.resizing_drag_data.is_some()
        {
            let mut changes_to_push = vec![];

            // Handle dragging
            if let Some(index) = self.dragging_index {
                if let Some(item) = self.panels.get(index) {
                    let initial_bounds = self.dragging_initial_bounds;
                    let current_bounds = item.bounds;
                    if initial_bounds.origin != current_bounds.origin
                        || initial_bounds.size != current_bounds.size
                    {
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

            // Handle resizing
            if let Some(index) = self.resizing_index {
                if let Some(drag_data) = &self.resizing_drag_data {
                    if let Some(item) = self.panels.get(index) {
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
        let view_id = view.entity_id();
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
                    .overflow_scroll()
                    .children(
                        panels
                            .into_iter()
                            .enumerate()
                            .map(|(ix, item)| self.render_panel(&item, ix, window, cx)),
                    )
                    .child({
                        canvas(
                            move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full()
                    })
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
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    if this.resizing_index.is_none() && this.dragging_index.is_none() {
                        let position = event.position;
                        if let Some((index, _)) = this.find_at_position(position) {
                            this.bring_to_front(Some(index), cx);
                            cx.notify();
                        }
                    }
                }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .child(Scrollbar::both(
                        view_id,
                        self.scroll_state.clone(),
                        self.scroll_handle.clone(),
                        scroll_size,
                    )),
            )
            .size_full()
    }
}
