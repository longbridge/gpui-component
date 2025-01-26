mod dock;
mod invalid_panel;
mod panel;
mod stack_panel;
mod state;
mod tab_panel;
mod tiles;

use anyhow::Result;
use gpui::{
    actions, canvas, div, prelude::FluentBuilder, AnyElement, AnyView, AppContext, Axis, Bounds,
    Edges, Entity as _, EntityId, EventEmitter, InteractiveElement as _, IntoElement, Model,
    ModelContext, ParentElement as _, Pixels, Render, SharedString, Styled, Subscription,
    VisualContext, Window,
};
use std::sync::Arc;

pub use dock::*;
pub use panel::*;
pub use stack_panel::*;
pub use state::*;
pub use tab_panel::*;
pub use tiles::*;

pub fn init(cx: &mut App) {
    cx.set_global(PanelRegistry::new());
}

actions!(dock, [ToggleZoom, ClosePanel]);

pub enum DockEvent {
    /// The layout of the dock has changed, subscribers this to save the layout.
    ///
    /// This event is emitted when every time the layout of the dock has changed,
    /// So it emits may be too frequently, you may want to debounce the event.
    LayoutChanged,
}

/// The main area of the dock.
pub struct DockArea {
    id: SharedString,
    /// The version is used to special the default layout, this is like the `panel_version` in [`Panel`](Panel).
    version: Option<usize>,
    pub(crate) bounds: Bounds<Pixels>,

    /// The center view of the dockarea.
    items: DockItem,

    /// The entity_id of the [`TabPanel`](TabPanel) where each toggle button should be displayed,
    toggle_button_panels: Edges<Option<EntityId>>,

    /// The left dock of the dock_area.
    left_dock: Option<Entity<Dock>>,
    /// The bottom dock of the dock_area.
    bottom_dock: Option<Entity<Dock>>,
    /// The right dock of the dock_area.
    right_dock: Option<Entity<Dock>>,
    /// The top zoom view of the dock_area, if any.
    zoom_view: Option<AnyView>,

    /// Lock panels layout, but allow to resize.
    is_locked: bool,

    /// The panel style, default is [`PanelStyle::Default`](PanelStyle::Default).
    pub(crate) panel_style: PanelStyle,

    _subscriptions: Vec<Subscription>,
}

/// DockItem is a tree structure that represents the layout of the dock.
#[derive(Clone)]
pub enum DockItem {
    /// Split layout
    Split {
        axis: Axis,
        items: Vec<DockItem>,
        sizes: Vec<Option<Pixels>>,
        view: Entity<StackPanel>,
    },
    /// Tab layout
    Tabs {
        items: Vec<Arc<dyn PanelView>>,
        active_ix: usize,
        view: Entity<TabPanel>,
    },
    /// Panel layout
    Panel { view: Arc<dyn PanelView> },
    /// Tiles layout
    Tiles {
        items: Vec<TileItem>,
        view: Entity<Tiles>,
    },
}

impl std::fmt::Debug for DockItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DockItem::Split {
                axis, items, sizes, ..
            } => f
                .debug_struct("Split")
                .field("axis", axis)
                .field("items", &items.len())
                .field("sizes", sizes)
                .finish(),
            DockItem::Tabs {
                items, active_ix, ..
            } => f
                .debug_struct("Tabs")
                .field("items", &items.len())
                .field("active_ix", active_ix)
                .finish(),
            DockItem::Panel { .. } => f.debug_struct("Panel").finish(),
            DockItem::Tiles { .. } => f.debug_struct("Tiles").finish(),
        }
    }
}

impl DockItem {
    /// Create DockItem with split layout, each item of panel have equal size.
    pub fn split(
        axis: Axis,
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window, cx: &mut App,
    ) -> Self {
        let sizes = vec![None; items.len()];
        Self::split_with_sizes(axis, items, sizes, dock_area, cx)
    }

    /// Create DockItem with split layout, each item of panel have specified size.
    ///
    /// Please note that the `items` and `sizes` must have the same length.
    /// Set `None` in `sizes` to make the index of panel have auto size.
    pub fn split_with_sizes(
        axis: Axis,
        items: Vec<DockItem>,
        sizes: Vec<Option<Pixels>>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window, cx: &mut App,
    ) -> Self {
        let mut items = items;
        let stack_panel = cx.new(|cx| {
            let mut stack_panel = StackPanel::new(axis, cx);
            for (i, item) in items.iter_mut().enumerate() {
                let view = item.view();
                let size = sizes.get(i).copied().flatten();
                stack_panel.add_panel(view.clone(), size, dock_area.clone(), cx)
            }

            for (i, item) in items.iter().enumerate() {
                let view = item.view();
                let size = sizes.get(i).copied().flatten();
                stack_panel.add_panel(view.clone(), size, dock_area.clone(), cx)
            }

            stack_panel
        });

        cx.defer({
            let stack_panel = stack_panel.clone();
            let dock_area = dock_area.clone();
            move |cx| {
                _ = dock_area.update(cx, |this, cx| {
                    this.subscribe_panel(&stack_panel, cx);
                });
            }
        });

        Self::Split {
            axis,
            items,
            sizes,
            view: stack_panel,
        }
    }

    /// Create DockItem with panel layout
    pub fn panel(panel: Arc<dyn PanelView>) -> Self {
        Self::Panel { view: panel }
    }

    /// Create DockItem with tiles layout
    ///
    /// This items and metas should have the same length.
    pub fn tiles(
        items: Vec<DockItem>,
        metas: Vec<impl Into<TileMeta> + Copy>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window, cx: &mut App,
    ) -> Self {
        assert!(items.len() == metas.len());

        let tile_panel = cx.new(|cx| {
            let mut tiles = Tiles::new(cx);
            for (ix, item) in items.clone().into_iter().enumerate() {
                match item {
                    DockItem::Tabs { view, .. } => {
                        let meta: TileMeta = metas[ix].into();
                        let tile_item =
                            TileItem::new(Arc::new(view), meta.bounds).z_index(meta.z_index);
                        tiles.add_item(tile_item, dock_area, cx);
                    }
                    _ => {
                        // Ignore non-tabs items
                    }
                }
            }
            tiles
        });

        cx.defer({
            let tile_panel = tile_panel.clone();
            let dock_area = dock_area.clone();
            move |cx| {
                _ = dock_area.update(cx, |this, cx| {
                    this.subscribe_panel(&tile_panel, cx);
                });
            }
        });

        Self::Tiles {
            items: tile_panel.read(cx).panels.clone(),
            view: tile_panel,
        }
    }

    /// Create DockItem with tabs layout, items are displayed as tabs.
    ///
    /// The `active_ix` is the index of the active tab, if `None` the first tab is active.
    pub fn tabs(
        items: Vec<Arc<dyn PanelView>>,
        active_ix: Option<usize>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window, cx: &mut App,
    ) -> Self {
        let mut new_items: Vec<Arc<dyn PanelView>> = vec![];
        for item in items.into_iter() {
            new_items.push(item)
        }
        Self::new_tabs(new_items, active_ix, dock_area, cx)
    }

    pub fn tab<P: Panel>(
        item: Entity<P>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window, cx: &mut App,
    ) -> Self {
        Self::new_tabs(vec![Arc::new(item.clone())], None, dock_area, cx)
    }

    fn new_tabs(
        items: Vec<Arc<dyn PanelView>>,
        active_ix: Option<usize>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window, cx: &mut App,
    ) -> Self {
        let active_ix = active_ix.unwrap_or(0);
        let tab_panel = cx.new(|cx| {
            let mut tab_panel = TabPanel::new(None, dock_area.clone(), cx);
            for item in items.iter() {
                tab_panel.add_panel(item.clone(), cx)
            }
            tab_panel.active_ix = active_ix;
            tab_panel
        });

        Self::Tabs {
            items,
            active_ix,
            view: tab_panel,
        }
    }

    /// Returns the views of the dock item.
    pub fn view(&self) -> Arc<dyn PanelView> {
        match self {
            Self::Split { view, .. } => Arc::new(view.clone()),
            Self::Tabs { view, .. } => Arc::new(view.clone()),
            Self::Tiles { view, .. } => Arc::new(view.clone()),
            Self::Panel { view, .. } => view.clone(),
        }
    }

    /// Find existing panel in the dock item.
    pub fn find_panel(&self, panel: Arc<dyn PanelView>) -> Option<Arc<dyn PanelView>> {
        match self {
            Self::Split { items, .. } => {
                items.iter().find_map(|item| item.find_panel(panel.clone()))
            }
            Self::Tabs { items, .. } => items.iter().find(|item| *item == &panel).cloned(),
            Self::Panel { view } => Some(view.clone()),
            Self::Tiles { items, .. } => items.iter().find_map(|item| {
                if &item.panel == &panel {
                    Some(item.panel.clone())
                } else {
                    None
                }
            }),
        }
    }

    /// Add a panel to the dock item.
    pub fn add_panel(
        &mut self,
        panel: Arc<dyn PanelView>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window, cx: &mut App,
    ) {
        match self {
            Self::Tabs { view, items, .. } => {
                items.push(panel.clone());
                view.update(cx, |tab_panel, cx| {
                    tab_panel.add_panel(panel, cx);
                });
            }
            Self::Split { view, items, .. } => {
                // Iter items to add panel to the first tabs
                for item in items.into_iter() {
                    if let DockItem::Tabs { view, .. } = item {
                        view.update(cx, |tab_panel, cx| {
                            tab_panel.add_panel(panel.clone(), cx);
                        });
                        return;
                    }
                }

                // Unable to find tabs, create new tabs
                let new_item = Self::tabs(vec![panel.clone()], None, dock_area, cx);
                items.push(new_item.clone());
                view.update(cx, |stack_panel, cx| {
                    stack_panel.add_panel(new_item.view(), None, dock_area.clone(), cx);
                });
            }
            Self::Tiles { .. } => {}
            Self::Panel { .. } => {}
        }
    }

    pub fn set_collapsed(&self, collapsed: bool, window: &mut Window, cx: &mut App) {
        match self {
            DockItem::Tabs { view, .. } => {
                view.update(cx, |tab_panel, cx| {
                    tab_panel.set_collapsed(collapsed, cx);
                });
            }
            DockItem::Split { items, .. } => {
                // For each child item, set collapsed state
                for item in items {
                    item.set_collapsed(collapsed, cx);
                }
            }
            DockItem::Tiles { .. } => {}
            DockItem::Panel { .. } => {}
        }
    }

    /// Recursively traverses to find the left-most and top-most TabPanel.
    pub(crate) fn left_top_tab_panel(&self, cx: &App) -> Option<Entity<TabPanel>> {
        match self {
            DockItem::Tabs { view, .. } => Some(view.clone()),
            DockItem::Split { view, .. } => view.read(cx).left_top_tab_panel(true, cx),
            DockItem::Tiles { .. } => None,
            DockItem::Panel { .. } => None,
        }
    }

    /// Recursively traverses to find the right-most and top-most TabPanel.
    pub(crate) fn right_top_tab_panel(&self, cx: &App) -> Option<Entity<TabPanel>> {
        match self {
            DockItem::Tabs { view, .. } => Some(view.clone()),
            DockItem::Split { view, .. } => view.read(cx).right_top_tab_panel(true, cx),
            DockItem::Tiles { .. } => None,
            DockItem::Panel { .. } => None,
        }
    }
}

impl DockArea {
    pub fn new(
        id: impl Into<SharedString>,
        version: Option<usize>,
        window: &mut Window, cx: &mut Context<Self>,
    ) -> Self {
        let stack_panel = cx.new(|cx| StackPanel::new(Axis::Horizontal, cx));

        let dock_item = DockItem::Split {
            axis: Axis::Horizontal,
            items: vec![],
            sizes: vec![],
            view: stack_panel.clone(),
        };

        let mut this = Self {
            id: id.into(),
            version,
            bounds: Bounds::default(),
            items: dock_item,
            zoom_view: None,
            toggle_button_panels: Edges::default(),
            left_dock: None,
            right_dock: None,
            bottom_dock: None,
            is_locked: false,
            panel_style: PanelStyle::Default,
            _subscriptions: vec![],
        };

        this.subscribe_panel(&stack_panel, cx);

        this
    }

    /// Set the panel style of the dock area.
    pub fn panel_style(mut self, style: PanelStyle) -> Self {
        self.panel_style = style;
        self
    }

    /// Set version of the dock area.
    pub fn set_version(&mut self, version: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.version = Some(version);
        cx.notify();
    }

    // FIXME: Remove this method after 2025-01-01
    #[deprecated(note = "Use `set_center` instead")]
    pub fn set_root(&mut self, item: DockItem, window: &mut Window, cx: &mut Context<Self>) {
        self.set_center(item, cx);
    }

    /// The the DockItem as the center of the dock area.
    ///
    /// This is used to render at the Center of the DockArea.
    pub fn set_center(&mut self, item: DockItem, window: &mut Window, cx: &mut Context<Self>) {
        self.subscribe_item(&item, cx);
        self.items = item;
        self.update_toggle_button_tab_panels(cx);
        cx.notify();
    }

    pub fn set_left_dock(
        &mut self,
        panel: DockItem,
        size: Option<Pixels>,
        open: bool,
        window: &mut Window, cx: &mut Context<Self>,
    ) {
        self.subscribe_item(&panel, cx);
        let weak_self = cx.model().downgrade();
        self.left_dock = Some(cx.new(|cx| {
            let mut dock = Dock::left(weak_self.clone(), cx);
            if let Some(size) = size {
                dock.set_size(size, cx);
            }
            dock.set_panel(panel, cx);
            dock.set_open(open, cx);
            dock
        }));
        self.update_toggle_button_tab_panels(cx);
    }

    pub fn set_bottom_dock(
        &mut self,
        panel: DockItem,
        size: Option<Pixels>,
        open: bool,
        window: &mut Window, cx: &mut Context<Self>,
    ) {
        self.subscribe_item(&panel, cx);
        let weak_self = cx.model().downgrade();
        self.bottom_dock = Some(cx.new(|cx| {
            let mut dock = Dock::bottom(weak_self.clone(), cx);
            if let Some(size) = size {
                dock.set_size(size, cx);
            }
            dock.set_panel(panel, cx);
            dock.set_open(open, cx);
            dock
        }));
        self.update_toggle_button_tab_panels(cx);
    }

    pub fn set_right_dock(
        &mut self,
        panel: DockItem,
        size: Option<Pixels>,
        open: bool,
        window: &mut Window, cx: &mut Context<Self>,
    ) {
        self.subscribe_item(&panel, cx);
        let weak_self = cx.model().downgrade();
        self.right_dock = Some(cx.new(|cx| {
            let mut dock = Dock::right(weak_self.clone(), cx);
            if let Some(size) = size {
                dock.set_size(size, cx);
            }
            dock.set_panel(panel, cx);
            dock.set_open(open, cx);
            dock
        }));
        self.update_toggle_button_tab_panels(cx);
    }

    /// Set locked state of the dock area, if locked, the dock area cannot be split or move, but allows to resize panels.
    pub fn set_locked(&mut self, locked: bool, _window: &mut Window, _cx: &mut App) {
        self.is_locked = locked;
    }

    /// Determine if the dock area is locked.
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    /// Determine if the dock area has a dock at the given placement.
    pub fn has_dock(&self, placement: DockPlacement) -> bool {
        match placement {
            DockPlacement::Left => self.left_dock.is_some(),
            DockPlacement::Bottom => self.bottom_dock.is_some(),
            DockPlacement::Right => self.right_dock.is_some(),
            DockPlacement::Center => false,
        }
    }

    /// Determine if the dock at the given placement is open.
    pub fn is_dock_open(&self, placement: DockPlacement, cx: &App) -> bool {
        match placement {
            DockPlacement::Left => self
                .left_dock
                .as_ref()
                .map(|dock| dock.read(cx).is_open())
                .unwrap_or(false),
            DockPlacement::Bottom => self
                .bottom_dock
                .as_ref()
                .map(|dock| dock.read(cx).is_open())
                .unwrap_or(false),
            DockPlacement::Right => self
                .right_dock
                .as_ref()
                .map(|dock| dock.read(cx).is_open())
                .unwrap_or(false),
            DockPlacement::Center => false,
        }
    }

    /// Set the dock at the given placement to be open or closed.
    ///
    /// Only the left, bottom, right dock can be toggled.
    pub fn set_dock_collapsible(
        &mut self,
        collapsible_edges: Edges<bool>,
        window: &mut Window, cx: &mut Context<Self>,
    ) {
        if let Some(left_dock) = self.left_dock.as_ref() {
            left_dock.update(cx, |dock, cx| {
                dock.set_collapsible(collapsible_edges.left, cx);
            });
        }

        if let Some(bottom_dock) = self.bottom_dock.as_ref() {
            bottom_dock.update(cx, |dock, cx| {
                dock.set_collapsible(collapsible_edges.bottom, cx);
            });
        }

        if let Some(right_dock) = self.right_dock.as_ref() {
            right_dock.update(cx, |dock, cx| {
                dock.set_collapsible(collapsible_edges.right, cx);
            });
        }
    }

    /// Determine if the dock at the given placement is collapsible.
    pub fn is_dock_collapsible(&self, placement: DockPlacement, cx: &App) -> bool {
        match placement {
            DockPlacement::Left => self
                .left_dock
                .as_ref()
                .map(|dock| dock.read(cx).collapsible)
                .unwrap_or(false),
            DockPlacement::Bottom => self
                .bottom_dock
                .as_ref()
                .map(|dock| dock.read(cx).collapsible)
                .unwrap_or(false),
            DockPlacement::Right => self
                .right_dock
                .as_ref()
                .map(|dock| dock.read(cx).collapsible)
                .unwrap_or(false),
            DockPlacement::Center => false,
        }
    }

    pub fn toggle_dock(&self, placement: DockPlacement, window: &mut Window, cx: &mut Context<Self>) {
        let dock = match placement {
            DockPlacement::Left => &self.left_dock,
            DockPlacement::Bottom => &self.bottom_dock,
            DockPlacement::Right => &self.right_dock,
            DockPlacement::Center => return,
        };

        if let Some(dock) = dock {
            dock.update(cx, |view, cx| {
                view.toggle_open(cx);
            })
        }
    }

    /// Add a panel item to the dock area at the given placement.
    ///
    /// If the left, bottom, right dock is not present, it will set the dock at the placement.
    pub fn add_panel(
        &mut self,
        panel: Arc<dyn PanelView>,
        placement: DockPlacement,
        window: &mut Window, cx: &mut Context<Self>,
    ) {
        let weak_self = cx.model().downgrade();
        match placement {
            DockPlacement::Left => {
                if let Some(dock) = self.left_dock.as_ref() {
                    dock.update(cx, |dock, cx| dock.add_panel(panel, cx))
                } else {
                    self.set_left_dock(
                        DockItem::tabs(vec![panel], None, &weak_self, cx),
                        None,
                        true,
                        cx,
                    );
                }
            }
            DockPlacement::Bottom => {
                if let Some(dock) = self.bottom_dock.as_ref() {
                    dock.update(cx, |dock, cx| dock.add_panel(panel, cx))
                } else {
                    self.set_bottom_dock(
                        DockItem::tabs(vec![panel], None, &weak_self, cx),
                        None,
                        true,
                        cx,
                    );
                }
            }
            DockPlacement::Right => {
                if let Some(dock) = self.right_dock.as_ref() {
                    dock.update(cx, |dock, cx| dock.add_panel(panel, cx))
                } else {
                    self.set_right_dock(
                        DockItem::tabs(vec![panel], None, &weak_self, cx),
                        None,
                        true,
                        cx,
                    );
                }
            }
            DockPlacement::Center => {
                self.items.add_panel(panel, &cx.model().downgrade(), cx);
            }
        }
    }

    /// Load the state of the DockArea from the DockAreaState.
    ///
    /// See also [DockeArea::dump].
    pub fn load(&mut self, state: DockAreaState, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.version = state.version;
        let weak_self = cx.model().downgrade();

        if let Some(left_dock_state) = state.left_dock {
            self.left_dock = Some(left_dock_state.to_dock(weak_self.clone(), cx));
        }

        if let Some(right_dock_state) = state.right_dock {
            self.right_dock = Some(right_dock_state.to_dock(weak_self.clone(), cx));
        }

        if let Some(bottom_dock_state) = state.bottom_dock {
            self.bottom_dock = Some(bottom_dock_state.to_dock(weak_self.clone(), cx));
        }

        self.items = state.center.to_item(weak_self, cx);
        self.update_toggle_button_tab_panels(cx);
        Ok(())
    }

    /// Dump the dock panels layout to PanelState.
    ///
    /// See also [DockArea::load].
    pub fn dump(&self, cx: &App) -> DockAreaState {
        let root = self.items.view();
        let center = root.dump(cx);

        let left_dock = self
            .left_dock
            .as_ref()
            .map(|dock| DockState::new(dock.clone(), cx));
        let right_dock = self
            .right_dock
            .as_ref()
            .map(|dock| DockState::new(dock.clone(), cx));
        let bottom_dock = self
            .bottom_dock
            .as_ref()
            .map(|dock| DockState::new(dock.clone(), cx));

        DockAreaState {
            version: self.version,
            center,
            left_dock,
            right_dock,
            bottom_dock,
        }
    }

    /// Subscribe event on the panels
    #[allow(clippy::only_used_in_recursion)]
    fn subscribe_item(&mut self, item: &DockItem, window: &mut Window, cx: &mut Context<Self>) {
        match item {
            DockItem::Split { items, view, .. } => {
                for item in items {
                    self.subscribe_item(item, cx);
                }

                self._subscriptions
                    .push(cx.subscribe(view, move |_, _, event, cx| match event {
                        PanelEvent::LayoutChanged => {
                            let dock_area = cx.model().clone();
                            cx.spawn(|_, mut cx| async move {
                                let _ = cx.update(|cx| {
                                    let _ = dock_area.update(cx, |view, cx| {
                                        view.update_toggle_button_tab_panels(cx)
                                    });
                                });
                            })
                            .detach();
                            cx.emit(DockEvent::LayoutChanged);
                        }
                        _ => {}
                    }));
            }
            DockItem::Tabs { .. } => {
                // We subscribe to the tab panel event in StackPanel's insert_panel
            }
            DockItem::Tiles { .. } => {
                // We subscribe to the tab panel event in Tiles's [`add_item`](Tiles::add_item)
            }
            DockItem::Panel { .. } => {
                // Not supported
            }
        }
    }

    /// Subscribe zoom event on the panel
    pub(crate) fn subscribe_panel<P: Panel>(
        &mut self,
        view: &Entity<P>,
        window: &mut Window, cx: &mut Context<DockArea>,
    ) {
        let subscription = cx.subscribe(view, move |_, panel, event, cx| match event {
            PanelEvent::ZoomIn => {
                let dock_area = cx.model().clone();
                let panel = panel.clone();
                cx.spawn(|_, mut cx| async move {
                    let _ = cx.update(|cx| {
                        let _ = dock_area.update(cx, |dock, cx| {
                            dock.set_zoomed_in(panel, cx);
                            cx.notify();
                        });
                    });
                })
                .detach();
            }
            PanelEvent::ZoomOut => {
                let dock_area = cx.model().clone();
                cx.spawn(|_, mut cx| async move {
                    let _ = cx.update(|cx| {
                        let _ = dock_area.update(cx, |view, cx| view.set_zoomed_out(cx));
                    });
                })
                .detach()
            }
            PanelEvent::LayoutChanged => {
                let dock_area = cx.model().clone();
                cx.spawn(|_, mut cx| async move {
                    let _ = cx.update(|cx| {
                        let _ = dock_area
                            .update(cx, |view, cx| view.update_toggle_button_tab_panels(cx));
                    });
                })
                .detach();
                cx.emit(DockEvent::LayoutChanged);
            }
        });

        self._subscriptions.push(subscription);
    }

    /// Returns the ID of the dock area.
    pub fn id(&self) -> SharedString {
        self.id.clone()
    }

    pub fn set_zoomed_in<P: Panel>(&mut self, panel: Entity<P>, window: &mut Window, cx: &mut Context<Self>) {
        self.zoom_view = Some(panel.into());
        cx.notify();
    }

    pub fn set_zoomed_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.zoom_view = None;
        cx.notify();
    }

    fn render_items(&self, _window: &mut Window, _cx: &mut Context<Self>) -> AnyElement {
        match &self.items {
            DockItem::Split { view, .. } => view.clone().into_any_element(),
            DockItem::Tabs { view, .. } => view.clone().into_any_element(),
            DockItem::Tiles { view, .. } => view.clone().into_any_element(),
            DockItem::Panel { view, .. } => view.clone().view().into_any_element(),
        }
    }

    pub fn update_toggle_button_tab_panels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Left toggle button
        self.toggle_button_panels.left = self
            .items
            .left_top_tab_panel(cx)
            .map(|view| view.entity_id());

        // Right toggle button
        self.toggle_button_panels.right = self
            .items
            .right_top_tab_panel(cx)
            .map(|view| view.entity_id());

        // Bottom toggle button
        self.toggle_button_panels.bottom = self
            .bottom_dock
            .as_ref()
            .and_then(|dock| dock.read(cx).panel.left_top_tab_panel(cx))
            .map(|view| view.entity_id());
    }
}
impl EventEmitter<DockEvent> for DockArea {}
impl Render for DockArea {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.model().clone();

        div()
            .id("dock-area")
            .relative()
            .size_full()
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, window, cx| view.update(cx, |r, _| r.bounds = bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .map(|this| {
                if let Some(zoom_view) = self.zoom_view.clone() {
                    this.child(zoom_view)
                } else {
                    match &self.items {
                        DockItem::Tiles { view, .. } => {
                            // render tiles
                            this.child(view.clone())
                        }
                        _ => {
                            // render dock
                            this.child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .h_full()
                                    // Left dock
                                    .when_some(self.left_dock.clone(), |this, dock| {
                                        this.child(div().flex().flex_none().child(dock))
                                    })
                                    // Center
                                    .child(
                                        div()
                                            .flex()
                                            .flex_1()
                                            .flex_col()
                                            .overflow_hidden()
                                            // Top center
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .overflow_hidden()
                                                    .child(self.render_items(cx)),
                                            )
                                            // Bottom Dock
                                            .when_some(self.bottom_dock.clone(), |this, dock| {
                                                this.child(dock)
                                            }),
                                    )
                                    // Right Dock
                                    .when_some(self.right_dock.clone(), |this, dock| {
                                        this.child(div().flex().flex_none().child(dock))
                                    }),
                            )
                        }
                    }
                }
            })
    }
}
