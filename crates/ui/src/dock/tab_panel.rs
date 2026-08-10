use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use gpui::{
    Anchor, Animation, AnimationExt as _, App, AppContext, Context, DismissEvent, Div,
    DragMoveEvent, Empty, Entity, EntityId, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, Point, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, WeakEntity, Window, div,
    point, prelude::FluentBuilder, px, rems,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, AxisExt, IconName, Placement, Selectable, Sizable,
    animation::{Lerp, ease_out_cubic},
    button::{Button, ButtonVariants as _},
    dock::PanelInfo,
    h_flex,
    menu::{DropdownMenu, PopupMenu},
    tab::{Tab, TabBar},
    v_flex,
};

use super::{
    ClosePanel, DockArea, DockPlacement, Panel, PanelControl, PanelEvent, PanelState, PanelStyle,
    PanelView, StackPanel, ToggleZoom,
};

#[derive(Clone)]
struct TabState {
    closable: bool,
    zoomable: Option<PanelControl>,
    draggable: bool,
    droppable: bool,
    active_panel: Option<Arc<dyn PanelView>>,
}

#[derive(Clone)]
pub(crate) struct DragPanel {
    pub(crate) panel: Arc<dyn PanelView>,
    pub(crate) tab_panel: Entity<TabPanel>,
    drag_offset: Rc<Cell<Point<Pixels>>>,
    drag_session_id: u64,
}

static NEXT_DRAG_SESSION_ID: AtomicU64 = AtomicU64::new(1);

impl DragPanel {
    pub(crate) fn new(panel: Arc<dyn PanelView>, tab_panel: Entity<TabPanel>) -> Self {
        Self {
            panel,
            tab_panel,
            drag_offset: Rc::new(Cell::new(Point::default())),
            drag_session_id: NEXT_DRAG_SESSION_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DropPlaceholderBounds {
    origin: Point<Pixels>,
    size: gpui::Size<Pixels>,
}

impl DropPlaceholderBounds {
    fn for_placement(bounds: gpui::Bounds<Pixels>, placement: Option<Placement>) -> Self {
        let half_width = bounds.size.width * 0.5;
        let half_height = bounds.size.height * 0.5;

        match placement {
            Some(Placement::Left) => Self {
                origin: Point::default(),
                size: gpui::size(half_width, bounds.size.height),
            },
            Some(Placement::Right) => Self {
                origin: point(half_width, px(0.)),
                size: gpui::size(half_width, bounds.size.height),
            },
            Some(Placement::Top) => Self {
                origin: Point::default(),
                size: gpui::size(bounds.size.width, half_height),
            },
            Some(Placement::Bottom) => Self {
                origin: point(px(0.), half_height),
                size: gpui::size(bounds.size.width, half_height),
            },
            None => Self {
                origin: Point::default(),
                size: bounds.size,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DropPlaceholderAnimation {
    drag_session_id: u64,
    placement: Option<Placement>,
    from: DropPlaceholderBounds,
    to: DropPlaceholderBounds,
    epoch: u64,
}

impl Render for DragPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("drag-panel")
            .cursor_grab()
            .py_1()
            .px_3()
            .w_24()
            .overflow_hidden()
            .whitespace_nowrap()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .text_color(cx.theme().tab_foreground)
            .bg(cx.theme().tokens.tab_active)
            .opacity(0.75)
            .child(self.panel.title(window, cx))
    }
}

pub struct TabPanel {
    focus_handle: FocusHandle,
    dock_area: WeakEntity<DockArea>,
    /// The stock_panel can be None, if is None, that means the panels can't be split or move
    stack_panel: Option<WeakEntity<StackPanel>>,
    pub(crate) panels: Vec<Arc<dyn PanelView>>,
    pub(crate) active_ix: usize,
    /// What each panel was last told via `set_active`, keyed by EntityId; absent means `false`.
    notified_active: HashMap<EntityId, bool>,
    /// Whether an active-state reconcile task is already queued for this frame.
    active_sync_scheduled: bool,
    /// If this is true, the Panel closable will follow the active panel's closable,
    /// otherwise this TabPanel will not able to close
    ///
    /// This is used for Dock to limit the last TabPanel not able to close, see [`super::Dock::new`].
    pub(crate) closable: bool,

    tab_bar_scroll_handle: ScrollHandle,
    pending_scroll_to_ix: Option<usize>,
    zoomed: bool,
    collapsed: bool,
    /// When drag move, will get the placement of the panel to be split
    will_split_placement: Option<Placement>,
    drop_placeholder_animation: Option<DropPlaceholderAnimation>,
    drop_placeholder_animation_name: SharedString,
    /// Is TabPanel used in Tiles.
    in_tiles: bool,
}

impl Panel for TabPanel {
    fn panel_name(&self) -> &'static str {
        "TabPanel"
    }

    fn title(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.active_panel(cx)
            .map(|panel| panel.title(window, cx))
            .unwrap_or("Empty Tab".into_any_element())
    }

    fn closable(&self, cx: &App) -> bool {
        if !self.closable {
            return false;
        }

        // 1. When is the final panel in the dock, it will not able to close.
        // 2. When is in the Tiles, it will always able to close (by active panel state).
        if !self.draggable(cx) && !self.in_tiles {
            return false;
        }

        self.active_panel(cx)
            .map(|panel| panel.closable(cx))
            .unwrap_or(false)
    }

    fn zoomable(&self, cx: &App) -> Option<PanelControl> {
        self.active_panel(cx).and_then(|panel| panel.zoomable(cx))
    }

    fn visible(&self, cx: &App) -> bool {
        self.visible_panels(cx).next().is_some()
    }

    fn dropdown_menu(
        &mut self,
        menu: PopupMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PopupMenu {
        if let Some(panel) = self.active_panel(cx) {
            panel.dropdown_menu(menu, window, cx)
        } else {
            menu
        }
    }

    fn toolbar_buttons(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Vec<Button>> {
        self.active_panel(cx)
            .and_then(|panel| panel.toolbar_buttons(window, cx))
    }

    fn dump(&self, cx: &App) -> PanelState {
        let mut state = PanelState::new(self);
        for panel in self.panels.iter() {
            state.add_child(panel.dump(cx));
            state.info = PanelInfo::tabs(self.active_ix);
        }
        state
    }

    fn inner_padding(&self, cx: &App) -> bool {
        self.active_panel(cx)
            .map_or(true, |panel| panel.inner_padding(cx))
    }
}

impl TabPanel {
    pub fn new(
        stack_panel: Option<WeakEntity<StackPanel>>,
        dock_area: WeakEntity<DockArea>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let entity_id = cx.entity_id();
        Self {
            focus_handle: cx.focus_handle(),
            dock_area,
            stack_panel,
            panels: Vec::new(),
            active_ix: 0,
            notified_active: HashMap::new(),
            active_sync_scheduled: false,
            tab_bar_scroll_handle: ScrollHandle::new(),
            pending_scroll_to_ix: None,
            will_split_placement: None,
            drop_placeholder_animation: None,
            drop_placeholder_animation_name: format!("dock-drop-placeholder-{entity_id}").into(),
            zoomed: false,
            collapsed: false,
            closable: true,
            in_tiles: false,
        }
    }

    /// Mark the TabPanel as being used in Tiles.
    pub(super) fn set_in_tiles(&mut self, in_tiles: bool) {
        self.in_tiles = in_tiles;
    }

    pub(super) fn set_parent(&mut self, view: WeakEntity<StackPanel>) {
        self.stack_panel = Some(view);
    }

    /// Return current active_panel View
    pub fn active_panel(&self, cx: &App) -> Option<Arc<dyn PanelView>> {
        let panel = self.panels.get(self.active_ix);

        if let Some(panel) = panel {
            if panel.visible(cx) {
                Some(panel.clone())
            } else {
                // Return the first visible panel
                self.visible_panels(cx).next()
            }
        } else {
            None
        }
    }

    pub fn active_ix(&self) -> usize {
        self.active_ix
    }

    fn set_active_ix(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix == self.active_ix {
            return;
        }

        self.active_ix = ix;
        self.pending_scroll_to_ix = Some(ix);
        self.focus_active_panel(window, cx);
        self.schedule_active_sync(window, cx);

        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    /// Queue one reconcile task per frame that notifies panels of their
    /// frame-end net active state. Using a spawned task (not `defer`) is what
    /// guarantees the task runs after every same-frame mutation, including
    /// deferred `set_collapsed` from [`super::Dock::set_open`].
    fn schedule_active_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_sync_scheduled {
            return;
        }
        self.active_sync_scheduled = true;

        cx.spawn_in(window, async move |view, cx| {
            _ = cx.update(|window, cx| {
                let Ok(changes) = view.update(cx, |view, _| view.reconcile_active_states()) else {
                    return;
                };
                // Dispatch outside the TabPanel update so a `set_active`
                // handler may call back into this TabPanel without panicking.
                for (panel, active) in changes {
                    panel.set_active(active, window, cx);
                }
            });
        })
        .detach();
    }

    /// Diff every panel's target state (`ix == active_ix && !collapsed`)
    /// against what it was last told, returning the deliveries to make —
    /// all `false` first, the single `true` last. Panels no longer in the
    /// group are pruned without a `false`: `on_removed` is their signal.
    fn reconcile_active_states(&mut self) -> Vec<(Arc<dyn PanelView>, bool)> {
        self.active_sync_scheduled = false;

        let mut notified = HashMap::with_capacity(self.panels.len());
        let mut changes = Vec::new();
        let mut activated = None;
        for (ix, panel) in self.panels.iter().enumerate() {
            let id = panel.view().entity_id();
            let target = ix == self.active_ix && !self.collapsed;
            let last = self.notified_active.get(&id).copied().unwrap_or(false);
            if target != last {
                if target {
                    activated = Some((panel.clone(), true));
                } else {
                    changes.push((panel.clone(), false));
                }
            }
            notified.insert(id, target);
        }
        self.notified_active = notified;
        changes.extend(activated);
        changes
    }

    /// Add a panel to the end of the tabs
    pub fn add_panel(
        &mut self,
        panel: Arc<dyn PanelView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_panel_with_active(panel, true, window, cx);
    }

    fn add_panel_with_active(
        &mut self,
        panel: Arc<dyn PanelView>,
        active: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        assert_ne!(
            panel.panel_name(cx),
            "StackPanel",
            "can not allows add `StackPanel` to `TabPanel`"
        );

        if self
            .panels
            .iter()
            .any(|p| p.view().entity_id() == panel.view().entity_id())
        {
            return;
        }

        panel.on_added_to(cx.entity().downgrade(), window, cx);
        self.panels.push(panel);
        // set the active panel to the new panel
        if active {
            self.set_active_ix(self.panels.len() - 1, window, cx);
        }
        // Unconditional: set_active_ix early-returns for the first panel,
        // which is displayed regardless of `active`.
        self.schedule_active_sync(window, cx);
        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    /// Add panel to try to split
    pub fn add_panel_at(
        &mut self,
        panel: Arc<dyn PanelView>,
        placement: Placement,
        size: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |view, cx| {
            cx.update(|window, cx| {
                view.update(cx, |view, cx| {
                    view.will_split_placement = Some(placement);
                    view.split_panel(panel, placement, size, None, window, cx)
                })
                .ok()
            })
            .ok()
        })
        .detach();
        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    fn insert_panel_at(
        &mut self,
        panel: Arc<dyn PanelView>,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .panels
            .iter()
            .any(|p| p.view().entity_id() == panel.view().entity_id())
        {
            return;
        }

        panel.on_added_to(cx.entity().downgrade(), window, cx);
        self.panels.insert(ix, panel);
        self.set_active_ix(ix, window, cx);
        // set_active_ix early-returns when ix == active_ix, yet the
        // displayed panel just changed.
        self.schedule_active_sync(window, cx);
        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    /// Remove a panel from the tab panel
    pub fn remove_panel(
        &mut self,
        panel: Arc<dyn PanelView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detach_panel(panel, window, cx);
        self.remove_self_if_empty(window, cx);
        cx.emit(PanelEvent::ZoomOut);
        cx.emit(PanelEvent::LayoutChanged);
    }

    /// Detach the panel, returning what it was last told via `set_active` so
    /// drag-and-drop can carry that belief into the target `TabPanel`.
    fn detach_panel(
        &mut self,
        panel: Arc<dyn PanelView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        panel.on_removed(window, cx);
        let panel_view = panel.view();
        let removed_ix = self.panels.iter().position(|p| p.view() == panel_view);
        self.panels.retain(|p| p.view() != panel_view);
        // Keep following the same displayed panel.
        if removed_ix.is_some_and(|ix| ix < self.active_ix) {
            self.active_ix -= 1;
        }
        if self.active_ix >= self.panels.len() {
            self.set_active_ix(self.panels.len().saturating_sub(1), window, cx)
        }
        self.schedule_active_sync(window, cx);
        self.notified_active.remove(&panel_view.entity_id())
    }

    /// Check to remove self from the parent StackPanel, if there is no panel left
    fn remove_self_if_empty(&self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.panels.is_empty() {
            return;
        }

        let tab_view = cx.entity().clone();
        if let Some(stack_panel) = self.stack_panel.as_ref() {
            _ = stack_panel.update(cx, |view, cx| {
                view.remove_panel(Arc::new(tab_view), window, cx);
            });
        }
    }

    pub(super) fn set_collapsed(
        &mut self,
        collapsed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.collapsed = collapsed;
        self.schedule_active_sync(window, cx);
        cx.notify();
    }

    fn is_locked(&self, cx: &App) -> bool {
        let Some(dock_area) = self.dock_area.upgrade() else {
            return true;
        };

        if dock_area.read(cx).is_locked() {
            return true;
        }

        if self.zoomed {
            return true;
        }

        self.stack_panel.is_none()
    }

    /// Return true if self or parent only have last panel.
    ///
    /// Only visible panels are counted, so a hidden panel does not keep the
    /// last visible panel draggable/closable (which could otherwise leave the
    /// dock visually empty and undroppable).
    fn is_last_panel(&self, cx: &App) -> bool {
        if let Some(parent) = &self.stack_panel {
            if let Some(stack_panel) = parent.upgrade() {
                if !stack_panel.read(cx).is_last_panel(cx) {
                    return false;
                }
            }
        }

        self.visible_panels(cx).count() <= 1
    }

    /// Return all visible panels
    fn visible_panels<'a>(&'a self, cx: &'a App) -> impl Iterator<Item = Arc<dyn PanelView>> + 'a {
        self.panels.iter().filter_map(|panel| {
            if panel.visible(cx) {
                Some(panel.clone())
            } else {
                None
            }
        })
    }

    /// Return true if the tab panel is draggable.
    ///
    /// E.g. if the parent and self only have one panel, it is not draggable.
    fn draggable(&self, cx: &App) -> bool {
        !self.is_locked(cx) && !self.is_last_panel(cx)
    }

    /// Return true if the tab panel is droppable.
    ///
    /// E.g. if the tab panel is locked, it is not droppable.
    fn droppable(&self, cx: &App) -> bool {
        !self.is_locked(cx)
    }

    fn render_toolbar(
        &mut self,
        state: &TabState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.collapsed {
            return div();
        }

        let zoomed = self.zoomed;
        let view = cx.entity().clone();
        let zoomable_toolbar_visible = state.zoomable.map_or(false, |v| v.toolbar_visible());

        h_flex()
            .gap_1()
            .occlude()
            .when_some(self.toolbar_buttons(window, cx), |this, buttons| {
                this.children(
                    buttons
                        .into_iter()
                        .map(|btn| btn.xsmall().ghost().tab_stop(false)),
                )
            })
            .map(|this| {
                let value = if zoomed {
                    Some(("zoom-out", IconName::Minimize, t!("Dock.Zoom Out")))
                } else if zoomable_toolbar_visible {
                    Some(("zoom-in", IconName::Maximize, t!("Dock.Zoom In")))
                } else {
                    None
                };

                if let Some((id, icon, tooltip)) = value {
                    this.child(
                        Button::new(id)
                            .icon(icon)
                            .xsmall()
                            .ghost()
                            .tab_stop(false)
                            .tooltip_with_action(tooltip, &ToggleZoom, None)
                            .selected(zoomed)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.on_action_toggle_zoom(&ToggleZoom, window, cx)
                            })),
                    )
                } else {
                    this
                }
            })
            .child(
                Button::new("menu")
                    .icon(IconName::Ellipsis)
                    .xsmall()
                    .ghost()
                    .tab_stop(false)
                    .dropdown_menu({
                        let zoomable = state.zoomable.map_or(false, |v| v.menu_visible());
                        let closable = state.closable;

                        move |menu, window, cx| {
                            view.update(cx, |this, cx| {
                                this.dropdown_menu(menu, window, cx)
                                    .separator()
                                    .menu_with_disabled(
                                        if zoomed {
                                            t!("Dock.Zoom Out")
                                        } else {
                                            t!("Dock.Zoom In")
                                        },
                                        Box::new(ToggleZoom),
                                        !zoomable,
                                    )
                                    .when(closable, |this| {
                                        this.separator()
                                            .menu(t!("Dock.Close"), Box::new(ClosePanel))
                                    })
                            })
                        }
                    })
                    .anchor(Anchor::TopRight),
            )
    }

    fn render_dock_toggle_button(
        &self,
        placement: DockPlacement,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Button> {
        if self.zoomed {
            return None;
        }

        let dock_area = self.dock_area.upgrade()?.read(cx);
        if !dock_area.toggle_button_visible {
            return None;
        }
        if !dock_area.is_dock_collapsible(placement, cx) {
            return None;
        }

        let view_entity_id = cx.entity().entity_id();
        let toggle_button_panels = dock_area.toggle_button_panels;

        // Check if current TabPanel's entity_id matches the one stored in DockArea for this placement
        if !match placement {
            DockPlacement::Left => {
                dock_area.left_dock.is_some() && toggle_button_panels.left == Some(view_entity_id)
            }
            DockPlacement::Right => {
                dock_area.right_dock.is_some() && toggle_button_panels.right == Some(view_entity_id)
            }
            DockPlacement::Bottom => {
                dock_area.bottom_dock.is_some()
                    && toggle_button_panels.bottom == Some(view_entity_id)
            }
            DockPlacement::Center => unreachable!(),
        } {
            return None;
        }

        let is_open = dock_area.is_dock_open(placement, cx);

        let icon = match placement {
            DockPlacement::Left => {
                if is_open {
                    IconName::PanelLeft
                } else {
                    IconName::PanelLeftOpen
                }
            }
            DockPlacement::Right => {
                if is_open {
                    IconName::PanelRight
                } else {
                    IconName::PanelRightOpen
                }
            }
            DockPlacement::Bottom => {
                if is_open {
                    IconName::PanelBottom
                } else {
                    IconName::PanelBottomOpen
                }
            }
            DockPlacement::Center => unreachable!(),
        };

        Some(
            Button::new(SharedString::from(format!("toggle-dock:{:?}", placement)))
                .icon(icon)
                .xsmall()
                .ghost()
                .tab_stop(false)
                .tooltip(match is_open {
                    true => t!("Dock.Collapse"),
                    false => t!("Dock.Expand"),
                })
                .on_click(cx.listener({
                    let dock_area = self.dock_area.clone();
                    move |_, _, window, cx| {
                        _ = dock_area.update(cx, |dock_area, cx| {
                            dock_area.toggle_dock(placement, window, cx);
                        });
                    }
                })),
        )
    }

    fn render_title_bar(
        &mut self,
        state: &TabState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();

        let Some(dock_area) = self.dock_area.upgrade() else {
            return div().into_any_element();
        };

        let left_dock_button = self.render_dock_toggle_button(DockPlacement::Left, window, cx);
        let bottom_dock_button = self.render_dock_toggle_button(DockPlacement::Bottom, window, cx);
        let right_dock_button = self.render_dock_toggle_button(DockPlacement::Right, window, cx);
        let has_extend_dock_button = left_dock_button.is_some() || bottom_dock_button.is_some();

        let is_bottom_dock = bottom_dock_button.is_some();

        let panel_style = dock_area.read(cx).panel_style;
        let visible_panels = self.visible_panels(cx).collect::<Vec<_>>();

        if visible_panels.len() == 1 && panel_style == PanelStyle::default() {
            let panel = visible_panels.get(0).unwrap();

            if !panel.visible(cx) {
                return div().into_any_element();
            }

            let title_style = panel.title_style(cx);

            return h_flex()
                .justify_between()
                .line_height(rems(1.0))
                .h(px(30.))
                .py_2()
                .pl_3()
                .pr_2()
                .when(left_dock_button.is_some(), |this| this.pl_2())
                .when(right_dock_button.is_some(), |this| this.pr_2())
                .when_some(title_style, |this, theme| {
                    this.bg(theme.background).text_color(theme.foreground)
                })
                .when(has_extend_dock_button, |this| {
                    this.child(
                        h_flex()
                            .flex_shrink_0()
                            .mr_1()
                            .gap_1()
                            .children(left_dock_button)
                            .children(bottom_dock_button),
                    )
                })
                .child(
                    div()
                        .id("tab")
                        .flex_1()
                        .min_w_16()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(panel.title(window, cx))
                        .when(state.draggable, |this| {
                            this.on_drag(
                                DragPanel::new(panel.clone(), view),
                                |drag, offset, _, cx| {
                                    cx.stop_propagation();
                                    drag.drag_offset.set(offset);
                                    cx.new(|_| drag.clone())
                                },
                            )
                        }),
                )
                .children(panel.title_suffix(window, cx))
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .ml_1()
                        .gap_1()
                        .child(self.render_toolbar(&state, window, cx))
                        .children(right_dock_button),
                )
                .into_any_element();
        }

        if let Some(panel_ix) = self.pending_scroll_to_ix.take() {
            if let Some(visible_ix) = self
                .panels
                .iter()
                .enumerate()
                .filter(|(_, p)| p.visible(cx))
                .position(|(ix, _)| ix == panel_ix)
            {
                self.tab_bar_scroll_handle.scroll_to_item(visible_ix);
            }
        }

        let tabs_count = self.panels.len();

        TabBar::new("tab-bar")
            .track_scroll(&self.tab_bar_scroll_handle)
            .when(has_extend_dock_button, |this| {
                this.prefix(
                    h_flex()
                        .items_center()
                        .top_0()
                        // Right -1 for avoid border overlap with the first tab
                        .right(-px(1.))
                        .border_r_1()
                        .border_b_1()
                        .h_full()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().tokens.tab_bar)
                        .px_2()
                        .children(left_dock_button)
                        .children(bottom_dock_button),
                )
            })
            .children(self.panels.iter().enumerate().filter_map(|(ix, panel)| {
                let mut active = state.active_panel.as_ref() == Some(panel);
                let droppable = self.collapsed;

                if !panel.visible(cx) {
                    return None;
                }

                // Always not show active tab style, if the panel is collapsed
                if self.collapsed {
                    active = false;
                }

                Some(
                    Tab::new()
                        .ix(ix)
                        .tab_bar_prefix(has_extend_dock_button)
                        .map(|this| {
                            if let Some(tab_name) = panel.tab_name(cx) {
                                this.child(tab_name)
                            } else {
                                this.child(panel.title(window, cx))
                            }
                        })
                        .selected(active)
                        .on_click(cx.listener({
                            let is_collapsed = self.collapsed;
                            let dock_area = self.dock_area.clone();
                            move |view, _, window, cx| {
                                view.set_active_ix(ix, window, cx);

                                // Open dock if clicked on the collapsed bottom dock
                                if is_bottom_dock && is_collapsed {
                                    _ = dock_area.update(cx, |dock_area, cx| {
                                        dock_area.toggle_dock(DockPlacement::Bottom, window, cx);
                                    });
                                }
                            }
                        }))
                        .when(!droppable, |this| {
                            this.when(state.draggable, |this| {
                                this.on_drag(
                                    DragPanel::new(panel.clone(), view.clone()),
                                    |drag, offset, _, cx| {
                                        cx.stop_propagation();
                                        drag.drag_offset.set(offset);
                                        cx.new(|_| drag.clone())
                                    },
                                )
                            })
                            .when(state.droppable, |this| {
                                this.drag_over::<DragPanel>(|this, _, _, cx| {
                                    this.rounded_l_none()
                                        .border_l_2()
                                        .border_r_0()
                                        .border_color(cx.theme().drag_border)
                                })
                                .on_drop(cx.listener(
                                    move |this, drag: &DragPanel, window, cx| {
                                        this.will_split_placement = None;
                                        this.on_drop(drag, Some(ix), true, window, cx)
                                    },
                                ))
                            })
                        }),
                )
            }))
            .last_empty_space(
                // empty space to allow move to last tab right
                div()
                    .id("tab-bar-empty-space")
                    .h_full()
                    .flex_grow_1()
                    .min_w_16()
                    .when(state.droppable, |this| {
                        this.drag_over::<DragPanel>(|this, _, _, cx| {
                            this.bg(cx.theme().tokens.drop_target)
                        })
                        .on_drop(cx.listener(
                            move |this, drag: &DragPanel, window, cx| {
                                this.will_split_placement = None;

                                let ix = if drag.tab_panel == view {
                                    Some(tabs_count - 1)
                                } else {
                                    None
                                };

                                this.on_drop(drag, ix, false, window, cx)
                            },
                        ))
                    }),
            )
            .when(!self.collapsed, |this| {
                this.suffix(
                    h_flex()
                        .items_center()
                        .top_0()
                        .right_0()
                        .border_l_1()
                        .border_b_1()
                        .h_full()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().tokens.tab_bar)
                        .px_2()
                        .gap_1()
                        .children(
                            self.active_panel(cx)
                                .and_then(|panel| panel.title_suffix(window, cx)),
                        )
                        .child(self.render_toolbar(state, window, cx))
                        .when_some(right_dock_button, |this, btn| this.child(btn)),
                )
            })
            .into_any_element()
    }

    fn render_active_panel(
        &self,
        state: &TabState,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.collapsed {
            return Empty {}.into_any_element();
        }

        let Some(active_panel) = state.active_panel.as_ref() else {
            return Empty {}.into_any_element();
        };

        let is_render_in_tabs = self.panels.len() > 1 && self.inner_padding(cx);

        let placeholder = self.drop_placeholder_animation;
        let placeholder_animation_name = self.drop_placeholder_animation_name.clone();

        v_flex()
            .id("active-panel")
            .group("")
            .flex_1()
            .when(is_render_in_tabs, |this| this.pt_2())
            .child(
                div()
                    .id("tab-content")
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .flex_1()
                    .child(
                        active_panel
                            .view()
                            .cached(StyleRefinement::default().absolute().size_full()),
                    ),
            )
            .when(state.droppable, |this| {
                this.on_drag_move(cx.listener(Self::on_panel_drag_move))
                    .child(
                        div()
                            .invisible()
                            .absolute()
                            .map(|this| match placeholder {
                                Some(animation) => this
                                    .left(animation.to.origin.x)
                                    .top(animation.to.origin.y)
                                    .w(animation.to.size.width)
                                    .h(animation.to.size.height),
                                None => this.top_0().left_0().size_full(),
                            })
                            .group_drag_over::<DragPanel>("", |this| this.visible())
                            .on_drop(cx.listener(|this, drag: &DragPanel, window, cx| {
                                this.on_drop(drag, None, true, window, cx)
                            }))
                            .when_some(placeholder, |this, animation| {
                                let from = animation.from.origin - animation.to.origin;
                                this.child(
                                    div()
                                        .absolute()
                                        .bg(cx.theme().tokens.drop_target)
                                        .with_animation(
                                            gpui::ElementId::NamedInteger(
                                                placeholder_animation_name,
                                                animation.epoch,
                                            ),
                                            Animation::new(Duration::from_millis(150))
                                                .with_easing(ease_out_cubic),
                                            move |this, delta| {
                                                let origin = from.lerp(&Point::default(), delta);
                                                let width = animation
                                                    .from
                                                    .size
                                                    .width
                                                    .lerp(&animation.to.size.width, delta);
                                                let height = animation
                                                    .from
                                                    .size
                                                    .height
                                                    .lerp(&animation.to.size.height, delta);

                                                this.left(origin.x).top(origin.y).w(width).h(height)
                                            },
                                        ),
                                )
                            }),
                    )
            })
            .into_any_element()
    }

    /// Calculate the split direction based on the current mouse position
    fn on_panel_drag_move(
        &mut self,
        drag: &DragMoveEvent<DragPanel>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = drag.bounds;
        let position = drag.event.position;

        // Check the mouse position to determine the split direction.
        let placement = if position.x < bounds.left() + bounds.size.width * 0.35 {
            Some(Placement::Left)
        } else if position.x > bounds.left() + bounds.size.width * 0.65 {
            Some(Placement::Right)
        } else if position.y < bounds.top() + bounds.size.height * 0.35 {
            Some(Placement::Top)
        } else if position.y > bounds.top() + bounds.size.height * 0.65 {
            Some(Placement::Bottom)
        } else {
            // center to merge into the current tab
            None
        };

        let dragged_panel = drag.drag(cx);
        let drag_session_id = dragged_panel.drag_session_id;
        let to = DropPlaceholderBounds::for_placement(bounds, placement);
        let source_origin = position - dragged_panel.drag_offset.get() - bounds.origin;
        let source = DropPlaceholderBounds {
            origin: source_origin,
            size: gpui::size(px(96.), px(30.)),
        };

        let should_restart = self.drop_placeholder_animation.is_none_or(|animation| {
            animation.drag_session_id != drag_session_id || animation.placement != placement
        });
        if should_restart {
            let (from, epoch) = self
                .drop_placeholder_animation
                .map(|animation| {
                    let from = if animation.drag_session_id == drag_session_id {
                        animation.to
                    } else {
                        source
                    };
                    (from, animation.epoch.wrapping_add(1))
                })
                .unwrap_or((source, 0));

            self.drop_placeholder_animation = Some(DropPlaceholderAnimation {
                drag_session_id,
                placement,
                from,
                to,
                epoch,
            });
        }
        self.will_split_placement = placement;
        cx.notify()
    }

    /// Handle the drop event when dragging a panel
    ///
    /// - `active` - When true, the panel will be active after the drop
    fn on_drop(
        &mut self,
        drag: &DragPanel,
        ix: Option<usize>,
        active: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel = drag.panel.clone();
        let is_same_tab = drag.tab_panel == cx.entity();

        // If target is same tab, and it is only one panel, do nothing.
        if is_same_tab && ix.is_none() {
            if self.will_split_placement.is_none() {
                return;
            } else {
                if self.panels.len() == 1 {
                    return;
                }
            }
        }

        // Here is looks like remove_panel on a same item, but it difference.
        //
        // We must to split it to remove_panel, unless it will be crash by error:
        // Cannot update ui::dock::tab_panel::TabPanel while it is already being updated
        let last_notified = if is_same_tab {
            self.detach_panel(panel.clone(), window, cx)
        } else {
            drag.tab_panel.update(cx, |view, cx| {
                let last_notified = view.detach_panel(panel.clone(), window, cx);
                view.remove_self_if_empty(window, cx);
                last_notified
            })
        };

        // Insert into new tabs, seeding the target map with what the panel
        // was last told so the move only notifies on a real state change.
        let panel_id = panel.view().entity_id();
        if let Some(placement) = self.will_split_placement {
            self.split_panel(panel, placement, None, last_notified, window, cx);
        } else {
            if let Some(ix) = ix {
                self.insert_panel_at(panel, ix, window, cx)
            } else {
                self.add_panel_with_active(panel, active, window, cx)
            }
            if let Some(last_notified) = last_notified {
                self.notified_active.insert(panel_id, last_notified);
            }
        }

        self.remove_self_if_empty(window, cx);
        cx.emit(PanelEvent::LayoutChanged);
    }

    /// Add panel with split placement
    fn split_panel(
        &self,
        panel: Arc<dyn PanelView>,
        placement: Placement,
        size: Option<Pixels>,
        last_notified: Option<bool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dock_area = self.dock_area.clone();
        let panel_id = panel.view().entity_id();
        // wrap the panel in a TabPanel
        let new_tab_panel = cx.new(|cx| Self::new(None, dock_area.clone(), window, cx));
        new_tab_panel.update(cx, |view, cx| {
            view.add_panel(panel, window, cx);
            if let Some(last_notified) = last_notified {
                view.notified_active.insert(panel_id, last_notified);
            }
        });

        let stack_panel = match self.stack_panel.as_ref().and_then(|panel| panel.upgrade()) {
            Some(panel) => panel,
            None => return,
        };

        let parent_axis = stack_panel.read(cx).axis;

        let ix = stack_panel
            .read(cx)
            .index_of_panel(Arc::new(cx.entity().clone()))
            .unwrap_or_default();

        if parent_axis.is_vertical() && placement.is_vertical() {
            stack_panel.update(cx, |view, cx| {
                view.insert_panel_at(
                    Arc::new(new_tab_panel),
                    ix,
                    placement,
                    size,
                    dock_area.clone(),
                    window,
                    cx,
                );
            });
        } else if parent_axis.is_horizontal() && placement.is_horizontal() {
            stack_panel.update(cx, |view, cx| {
                view.insert_panel_at(
                    Arc::new(new_tab_panel),
                    ix,
                    placement,
                    size,
                    dock_area.clone(),
                    window,
                    cx,
                );
            });
        } else {
            // 1. Create new StackPanel with new axis
            // 2. Move cx.entity() from parent StackPanel to the new StackPanel
            // 3. Add the new TabPanel to the new StackPanel at the correct index
            // 4. Add new StackPanel to the parent StackPanel at the correct index
            let tab_panel = cx.entity().clone();

            // Try to use the old stack panel, not just create a new one, to avoid too many nested stack panels
            let new_stack_panel = if stack_panel.read(cx).panels_len() <= 1 {
                stack_panel.update(cx, |view, cx| {
                    view.remove_all_panels(window, cx);
                    view.set_axis(placement.axis(), window, cx);
                });
                stack_panel.clone()
            } else {
                cx.new(|cx| {
                    let mut panel = StackPanel::new(placement.axis(), window, cx);
                    panel.parent = Some(stack_panel.downgrade());
                    panel
                })
            };

            new_stack_panel.update(cx, |view, cx| match placement {
                Placement::Left | Placement::Top => {
                    view.add_panel(Arc::new(new_tab_panel), size, dock_area.clone(), window, cx);
                    view.add_panel(
                        Arc::new(tab_panel.clone()),
                        None,
                        dock_area.clone(),
                        window,
                        cx,
                    );
                }
                Placement::Right | Placement::Bottom => {
                    view.add_panel(
                        Arc::new(tab_panel.clone()),
                        None,
                        dock_area.clone(),
                        window,
                        cx,
                    );
                    view.add_panel(Arc::new(new_tab_panel), size, dock_area.clone(), window, cx);
                }
            });

            if stack_panel != new_stack_panel {
                stack_panel.update(cx, |view, cx| {
                    view.replace_panel(
                        Arc::new(tab_panel.clone()),
                        new_stack_panel.clone(),
                        window,
                        cx,
                    );
                });
            }

            cx.spawn_in(window, async move |_, cx| {
                cx.update(|window, cx| {
                    tab_panel.update(cx, |view, cx| view.remove_self_if_empty(window, cx))
                })
            })
            .detach()
        }

        cx.emit(PanelEvent::LayoutChanged);
    }

    fn focus_active_panel(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_panel) = self.active_panel(cx) {
            active_panel.focus_handle(cx).focus(window, cx);
        }
    }

    fn on_action_toggle_zoom(
        &mut self,
        _: &ToggleZoom,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.zoomable(cx).is_none() {
            return;
        }

        if !self.zoomed {
            cx.emit(PanelEvent::ZoomIn)
        } else {
            cx.emit(PanelEvent::ZoomOut)
        }
        self.zoomed = !self.zoomed;

        cx.spawn_in(window, {
            let zoomed = self.zoomed;
            async move |view, cx| {
                _ = cx.update(|window, cx| {
                    _ = view.update(cx, |view, cx| {
                        view.set_zoomed(zoomed, window, cx);
                    });
                });
            }
        })
        .detach();
    }

    fn on_action_close_panel(
        &mut self,
        _: &ClosePanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.closable(cx) {
            return;
        }
        if let Some(panel) = self.active_panel(cx) {
            self.remove_panel(panel, window, cx);
        }

        // Remove self from the parent DockArea.
        // This is ensure to remove from Tiles
        if self.panels.is_empty() && self.in_tiles {
            let tab_panel = Arc::new(cx.entity());
            window.defer(cx, {
                let dock_area = self.dock_area.clone();
                move |window, cx| {
                    _ = dock_area.update(cx, |this, cx| {
                        this.remove_panel_from_all_docks(tab_panel, window, cx);
                    });
                }
            });
        }
    }

    // Bind actions to the tab panel, only when the tab panel is not collapsed.
    fn bind_actions(&self, cx: &mut Context<Self>) -> Div {
        v_flex().when(!self.collapsed, |this| {
            this.on_action(cx.listener(Self::on_action_toggle_zoom))
                .on_action(cx.listener(Self::on_action_close_panel))
        })
    }
}

impl Focusable for TabPanel {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        if let Some(active_panel) = self.active_panel(cx) {
            active_panel.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
impl EventEmitter<DismissEvent> for TabPanel {}
impl EventEmitter<PanelEvent> for TabPanel {}
impl Render for TabPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let focus_handle = self.focus_handle(cx);
        let active_panel = self.active_panel(cx);
        let state = TabState {
            closable: self.closable(cx),
            draggable: self.draggable(cx),
            droppable: self.droppable(cx),
            zoomable: self.zoomable(cx),
            active_panel,
        };

        self.bind_actions(cx)
            .id("tab-panel")
            .track_focus(&focus_handle)
            .tab_group()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().tokens.background)
            .child(self.render_title_bar(&state, window, cx))
            .child(self.render_active_panel(&state, window, cx))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use gpui::{TestAppContext, VisualTestContext, WindowHandle};

    use super::*;
    use crate::{Root, Theme, dock::DockItem};

    #[test]
    fn drop_placeholder_bounds_cover_each_target_placement() {
        let bounds = gpui::Bounds {
            origin: point(px(120.), px(80.)),
            size: gpui::size(px(400.), px(300.)),
        };

        assert_eq!(
            DropPlaceholderBounds::for_placement(bounds, Some(Placement::Left)),
            DropPlaceholderBounds {
                origin: point(px(0.), px(0.)),
                size: gpui::size(px(200.), px(300.)),
            }
        );
        assert_eq!(
            DropPlaceholderBounds::for_placement(bounds, Some(Placement::Right)),
            DropPlaceholderBounds {
                origin: point(px(200.), px(0.)),
                size: gpui::size(px(200.), px(300.)),
            }
        );
        assert_eq!(
            DropPlaceholderBounds::for_placement(bounds, Some(Placement::Top)),
            DropPlaceholderBounds {
                origin: point(px(0.), px(0.)),
                size: gpui::size(px(400.), px(150.)),
            }
        );
        assert_eq!(
            DropPlaceholderBounds::for_placement(bounds, Some(Placement::Bottom)),
            DropPlaceholderBounds {
                origin: point(px(0.), px(150.)),
                size: gpui::size(px(400.), px(150.)),
            }
        );
        assert_eq!(
            DropPlaceholderBounds::for_placement(bounds, None),
            DropPlaceholderBounds {
                origin: point(px(0.), px(0.)),
                size: gpui::size(px(400.), px(300.)),
            }
        );
    }

    /// Shared, cross-panel ordered log of every `set_active` delivery.
    type Log = Arc<Mutex<Vec<(&'static str, bool)>>>;

    struct TestPanel {
        name: &'static str,
        focus_handle: FocusHandle,
        log: Log,
    }

    impl Panel for TestPanel {
        fn panel_name(&self) -> &'static str {
            "TestPanel"
        }

        fn set_active(&mut self, active: bool, _: &mut Window, _: &mut Context<Self>) {
            self.log.lock().unwrap().push((self.name, active));
        }
    }

    impl EventEmitter<PanelEvent> for TestPanel {}

    impl Focusable for TestPanel {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for TestPanel {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Empty
        }
    }

    struct DockFixture {
        dock_area: Entity<DockArea>,
        window: WindowHandle<Root>,
        log: Log,
    }

    fn setup(cx: &mut TestAppContext) -> DockFixture {
        let log = Log::default();
        let mut dock_area = None;
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.set_global(Theme::default());
                let area = cx.new(|cx| DockArea::new("test-dock", None, window, cx));
                dock_area = Some(area.clone());
                cx.new(|cx| Root::new(area, window, cx))
            })
            .unwrap()
        });
        DockFixture {
            dock_area: dock_area.unwrap(),
            window,
            log,
        }
    }

    fn test_panel(name: &'static str, log: &Log, cx: &mut App) -> Entity<TestPanel> {
        let log = log.clone();
        cx.new(|cx| TestPanel {
            name,
            focus_handle: cx.focus_handle(),
            log,
        })
    }

    /// Build a tab group holding `names`, returning the kept-alive DockItem,
    /// its TabPanel, and the panel entities.
    fn build_tabs(
        fixture: &DockFixture,
        names: &[&'static str],
        active_ix: Option<usize>,
        cx: &mut VisualTestContext,
    ) -> (DockItem, Entity<TabPanel>, Vec<Entity<TestPanel>>) {
        let weak_dock_area = fixture.dock_area.downgrade();
        let log = fixture.log.clone();
        let names = names.to_vec();
        let (item, panels) = cx.update(move |window, cx| {
            let panels: Vec<_> = names
                .iter()
                .map(|name| test_panel(name, &log, cx))
                .collect();
            let items = panels
                .iter()
                .map(|panel| Arc::new(panel.clone()) as Arc<dyn PanelView>)
                .collect();
            let mut item = DockItem::tabs(items, &weak_dock_area, window, cx);
            if let Some(ix) = active_ix {
                item = item.active_index(ix, cx);
            }
            (item, panels)
        });
        let DockItem::Tabs { view, .. } = &item else {
            unreachable!("DockItem::tabs must return DockItem::Tabs");
        };
        let tab_panel = view.clone();
        (item, tab_panel, panels)
    }

    fn drain(log: &Log) -> Vec<(&'static str, bool)> {
        std::mem::take(&mut *log.lock().unwrap())
    }

    #[gpui::test]
    fn single_panel_group_receives_initial_active(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let _keep = build_tabs(&fixture, &["A"], None, &mut cx);
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), [("A", true)]);
    }

    #[gpui::test]
    fn multi_tab_construction_notifies_only_displayed_panel(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let _keep = build_tabs(&fixture, &["A", "B", "C"], None, &mut cx);
        cx.run_until_parked();

        // No false→true flip on A, no duplicate true, B/C silent.
        assert_eq!(drain(&fixture.log), [("A", true)]);
    }

    #[gpui::test]
    fn active_index_restore_notifies_that_panel_only(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let _keep = build_tabs(&fixture, &["A", "B", "C"], Some(2), &mut cx);
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), [("C", true)]);
    }

    #[gpui::test]
    fn switching_tabs_sends_false_then_true(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep, tab_panel, _) = build_tabs(&fixture, &["A", "B"], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);

        cx.update(|window, cx| {
            tab_panel.update(cx, |tab_panel, cx| tab_panel.set_active_ix(1, window, cx));
        });
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), [("A", false), ("B", true)]);
    }

    #[gpui::test]
    fn reselecting_active_tab_stays_silent(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep, tab_panel, _) = build_tabs(&fixture, &["A", "B"], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);

        cx.update(|window, cx| {
            tab_panel.update(cx, |tab_panel, cx| tab_panel.set_active_ix(0, window, cx));
        });
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), []);
    }

    #[gpui::test]
    fn inserting_at_active_ix_swaps_notifications(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep, tab_panel, _) = build_tabs(&fixture, &["A", "B"], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);

        let log = fixture.log.clone();
        cx.update(|window, cx| {
            let c = test_panel("C", &log, cx);
            tab_panel.update(cx, |tab_panel, cx| {
                tab_panel.insert_panel_at(Arc::new(c), 0, window, cx)
            });
        });
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), [("A", false), ("C", true)]);
    }

    #[gpui::test]
    fn removing_before_active_keeps_displayed_panel(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep, tab_panel, panels) = build_tabs(&fixture, &["A", "B", "C"], None, &mut cx);
        cx.update(|window, cx| {
            tab_panel.update(cx, |tab_panel, cx| tab_panel.set_active_ix(1, window, cx));
        });
        cx.run_until_parked();
        drain(&fixture.log);

        cx.update(|window, cx| {
            tab_panel.update(cx, |tab_panel, cx| {
                tab_panel.remove_panel(Arc::new(panels[0].clone()), window, cx)
            });
        });
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), []);
        cx.update(|_, cx| {
            tab_panel.read_with(cx, |tab_panel, _| {
                assert_eq!(tab_panel.active_ix, 0);
                assert_eq!(
                    tab_panel.panels[0].view().entity_id(),
                    panels[1].entity_id()
                );
            });
        });
    }

    #[gpui::test]
    fn collapse_and_expand_notify_active_panel(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep, tab_panel, _) = build_tabs(&fixture, &["A", "B"], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);

        cx.update(|window, cx| {
            tab_panel.update(cx, |tab_panel, cx| {
                tab_panel.set_collapsed(true, window, cx)
            });
        });
        cx.run_until_parked();
        assert_eq!(drain(&fixture.log), [("A", false)]);

        cx.update(|window, cx| {
            tab_panel.update(cx, |tab_panel, cx| {
                tab_panel.set_collapsed(false, window, cx)
            });
        });
        cx.run_until_parked();
        assert_eq!(drain(&fixture.log), [("A", true)]);
    }

    #[gpui::test]
    fn background_add_is_silent_but_first_panel_is_not(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep, tab_panel, _) = build_tabs(&fixture, &["A", "B"], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);

        let log = fixture.log.clone();
        cx.update(|window, cx| {
            let c = test_panel("C", &log, cx);
            tab_panel.update(cx, |tab_panel, cx| {
                tab_panel.add_panel_with_active(Arc::new(c), false, window, cx)
            });
        });
        cx.run_until_parked();
        assert_eq!(drain(&fixture.log), []);

        // The first panel of an empty group is displayed regardless of the
        // `active` flag, so it must be told.
        let (_keep2, empty_tab_panel, _) = build_tabs(&fixture, &[], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);
        let log = fixture.log.clone();
        cx.update(|window, cx| {
            let d = test_panel("D", &log, cx);
            empty_tab_panel.update(cx, |tab_panel, cx| {
                tab_panel.add_panel_with_active(Arc::new(d), false, window, cx)
            });
        });
        cx.run_until_parked();
        assert_eq!(drain(&fixture.log), [("D", true)]);
    }

    #[gpui::test]
    fn drag_active_panel_to_other_group_stays_silent_for_it(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep_src, source, source_panels) = build_tabs(&fixture, &["A", "B"], None, &mut cx);
        let (_keep_dst, target, _) = build_tabs(&fixture, &["C"], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);

        // A was already told `true`; becoming the target's active tab must
        // not repeat it.
        cx.update(|window, cx| {
            let drag = DragPanel::new(Arc::new(source_panels[0].clone()), source.clone());
            target.update(cx, |tab_panel, cx| {
                tab_panel.on_drop(&drag, None, true, window, cx)
            });
        });
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), [("B", true), ("C", false)]);
    }

    #[gpui::test]
    fn drag_active_panel_to_background_slot_deactivates_it(cx: &mut TestAppContext) {
        let fixture = setup(cx);
        let mut cx = VisualTestContext::from_window(fixture.window.into(), cx);

        let (_keep_src, source, source_panels) = build_tabs(&fixture, &["A"], None, &mut cx);
        let (_keep_dst, target, _) = build_tabs(&fixture, &["C", "D"], None, &mut cx);
        cx.run_until_parked();
        drain(&fixture.log);

        // A was told `true` and becomes a background tab, so it gets one `false`.
        cx.update(|window, cx| {
            let drag = DragPanel::new(Arc::new(source_panels[0].clone()), source.clone());
            target.update(cx, |tab_panel, cx| {
                tab_panel.on_drop(&drag, None, false, window, cx)
            });
        });
        cx.run_until_parked();

        assert_eq!(drain(&fixture.log), [("A", false)]);
    }
}
