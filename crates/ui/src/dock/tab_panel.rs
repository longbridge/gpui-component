use std::sync::Arc;

use gpui::{
    div, prelude::FluentBuilder, px, rems, AnchorCorner, AppContext, DefiniteLength, DismissEvent,
    DragMoveEvent, Empty, Entity, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, View, ViewContext, VisualContext as _,
    WeakView, WindowContext,
};
use rust_i18n::t;

use crate::{
    button::{Button, ButtonStyled as _},
    dock::DockItemInfo,
    h_flex,
    popup_menu::{PopupMenu, PopupMenuExt},
    tab::{Tab, TabBar},
    theme::ActiveTheme,
    v_flex, AxisExt, IconName, Placement, Selectable, Sizable,
};

use super::{
    ClosePanel, DockArea, DockItemState, DockPlacement, Panel, PanelEvent, PanelView, StackPanel,
    ToggleZoom,
};

#[derive(Clone)]
pub(crate) struct DragPanel {
    pub(crate) panel: Arc<dyn PanelView>,
    pub(crate) tab_panel: View<TabPanel>,
}

impl DragPanel {
    pub(crate) fn new(panel: Arc<dyn PanelView>, tab_panel: View<TabPanel>) -> Self {
        Self { panel, tab_panel }
    }
}

impl Render for DragPanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
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
            .rounded_md()
            .text_color(cx.theme().tab_foreground)
            .bg(cx.theme().tab_active)
            .opacity(0.75)
            .child(self.panel.title(cx))
    }
}

pub struct TabPanel {
    focus_handle: FocusHandle,
    dock_area: WeakView<DockArea>,
    /// The stock_panel can be None, if is None, that means the panels can't be split or move
    stack_panel: Option<WeakView<StackPanel>>,
    pub(crate) panels: Vec<Arc<dyn PanelView>>,
    pub(crate) active_ix: usize,
    /// If this is true, the Panel closeable will follow the active panel's closeable,
    /// otherwise this TabPanel will not able to close
    pub(crate) closeable: bool,

    tab_bar_scroll_handle: ScrollHandle,
    is_zoomed: bool,
    is_collapsed: bool,

    /// When drag move, will get the placement of the panel to be split
    will_split_placement: Option<Placement>,
}

impl Panel for TabPanel {
    fn panel_name(&self) -> &'static str {
        "TabPanel"
    }

    fn title(&self, cx: &WindowContext) -> gpui::AnyElement {
        self.active_panel()
            .map(|panel| panel.title(cx))
            .unwrap_or("Empty Tab".into_any_element())
    }

    fn closeable(&self, cx: &WindowContext) -> bool {
        if !self.closeable {
            return false;
        }

        self.active_panel()
            .map(|panel| panel.closeable(cx))
            .unwrap_or(false)
    }

    fn zoomable(&self, cx: &WindowContext) -> bool {
        self.active_panel()
            .map(|panel| panel.zoomable(cx))
            .unwrap_or(false)
    }

    fn collapsible(&self, cx: &WindowContext) -> bool {
        self.active_panel()
            .map(|panel| panel.collapsible(cx))
            .unwrap_or(false)
    }

    fn popup_menu(&self, menu: PopupMenu, cx: &WindowContext) -> PopupMenu {
        if let Some(panel) = self.active_panel() {
            panel.popup_menu(menu, cx)
        } else {
            menu
        }
    }

    fn dump(&self, cx: &AppContext) -> DockItemState {
        let mut state = DockItemState::new(self);
        for panel in self.panels.iter() {
            state.add_child(panel.dump(cx));
            state.info = DockItemInfo::tabs(self.active_ix);
        }
        state
    }
}

impl TabPanel {
    pub fn new(
        stack_panel: Option<WeakView<StackPanel>>,
        dock_area: WeakView<DockArea>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            dock_area,
            stack_panel,
            panels: Vec::new(),
            active_ix: 0,
            tab_bar_scroll_handle: ScrollHandle::new(),
            will_split_placement: None,
            is_zoomed: false,
            is_collapsed: false,
            closeable: true,
        }
    }

    pub(super) fn set_parent(&mut self, view: WeakView<StackPanel>) {
        self.stack_panel = Some(view);
    }

    /// Return current active_panel View
    pub fn active_panel(&self) -> Option<Arc<dyn PanelView>> {
        self.panels.get(self.active_ix).cloned()
    }

    fn set_active_ix(&mut self, ix: usize, cx: &mut ViewContext<Self>) {
        self.active_ix = ix;
        self.tab_bar_scroll_handle.scroll_to_item(ix);
        self.focus_active_panel(cx);
        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    /// Add a panel to the end of the tabs
    pub fn add_panel(&mut self, panel: Arc<dyn PanelView>, cx: &mut ViewContext<Self>) {
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

        self.panels.push(panel);
        // set the active panel to the new panel
        self.set_active_ix(self.panels.len() - 1, cx);
        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    /// Add panel to try to split
    pub fn add_panel_at(
        &mut self,
        panel: Arc<dyn PanelView>,
        placement: Placement,
        size: Option<Pixels>,
        cx: &mut ViewContext<Self>,
    ) {
        cx.spawn(|view, mut cx| async move {
            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    view.will_split_placement = Some(placement);
                    view.split_panel(panel, placement, size, cx)
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
        cx: &mut ViewContext<Self>,
    ) {
        if self
            .panels
            .iter()
            .any(|p| p.view().entity_id() == panel.view().entity_id())
        {
            return;
        }

        self.panels.insert(ix, panel);
        self.set_active_ix(ix, cx);
        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    /// Remove a panel from the tab panel
    pub fn remove_panel(&mut self, panel: Arc<dyn PanelView>, cx: &mut ViewContext<Self>) {
        self.detach_panel(panel, cx);
        self.remove_self_if_empty(cx);
        cx.emit(PanelEvent::ZoomOut);
        cx.emit(PanelEvent::LayoutChanged);
    }

    fn detach_panel(&mut self, panel: Arc<dyn PanelView>, cx: &mut ViewContext<Self>) {
        let panel_view = panel.view();
        self.panels.retain(|p| p.view() != panel_view);
        if self.active_ix >= self.panels.len() {
            self.set_active_ix(self.panels.len().saturating_sub(1), cx)
        }
    }

    /// Check to remove self from the parent StackPanel, if there is no panel left
    fn remove_self_if_empty(&self, cx: &mut ViewContext<Self>) {
        if !self.panels.is_empty() {
            return;
        }

        let tab_view = cx.view().clone();
        if let Some(stack_panel) = self.stack_panel.as_ref() {
            _ = stack_panel.update(cx, |view, cx| {
                view.remove_panel(Arc::new(tab_view), cx);
            });
        }
    }

    /// Return true if the panel can be split or move
    fn can_split(&self) -> bool {
        self.stack_panel.is_some() && !self.is_zoomed
    }

    pub(super) fn set_collapsed(&mut self, collapsed: bool, cx: &mut ViewContext<Self>) {
        self.is_collapsed = collapsed;
        cx.notify();
    }

    fn render_menu_button(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let closeable = self.closeable(cx);
        let zoomable = self.zoomable(cx);

        let is_zoomed = self.is_zoomed && zoomable;
        let view = cx.view().clone();
        let build_popup_menu = move |this, cx: &WindowContext| view.read(cx).popup_menu(this, cx);

        // TODO: Do not show MenuButton if there is no menu items

        h_flex()
            .gap_2()
            .occlude()
            .items_center()
            .when(self.is_zoomed, |this| {
                this.child(
                    Button::new("zoom")
                        .icon(IconName::Minimize)
                        .xsmall()
                        .ghost()
                        .tooltip(t!("Dock.Zoom Out"))
                        .on_click(
                            cx.listener(|view, _, cx| view.on_action_toggle_zoom(&ToggleZoom, cx)),
                        ),
                )
            })
            .child(
                Button::new("menu")
                    .icon(IconName::Ellipsis)
                    .xsmall()
                    .ghost()
                    .popup_menu(move |this, cx| {
                        build_popup_menu(this, cx)
                            .when(zoomable, |this| {
                                let name = if is_zoomed {
                                    t!("Dock.Zoom Out")
                                } else {
                                    t!("Dock.Zoom In")
                                };
                                this.separator().menu(name, Box::new(ToggleZoom))
                            })
                            .when(closeable, |this| {
                                this.separator()
                                    .menu(t!("Dock.Close"), Box::new(ClosePanel))
                            })
                    })
                    .anchor(AnchorCorner::TopRight),
            )
    }

    fn render_dock_toggle_button(
        &self,
        placement: DockPlacement,
        cx: &mut ViewContext<Self>,
    ) -> Option<impl IntoElement> {
        let dock_area = self.dock_area.upgrade().expect("BUG: DockArea is missing");

        if self.is_zoomed {
            return None;
        }

        let mut has_left_dock = false;
        let mut has_right_dock = false;
        let mut has_bottom_dock = false;
        let mut self_is_left_dock = false;
        let mut self_is_right_dock = false;
        let mut self_is_bottom_dock = false;
        if let Some(left_view) = &dock_area.read(cx).left_dock {
            has_left_dock = true;
            if left_view.read(cx).panel.entity_id() == cx.view().entity_id() {
                self_is_left_dock = true;
            }
        }
        if let Some(right_view) = &dock_area.read(cx).right_dock {
            has_right_dock = true;
            if right_view.read(cx).panel.entity_id() == cx.view().entity_id() {
                self_is_right_dock = true;
            }
        }
        if let Some(bottom_view) = &dock_area.read(cx).bottom_dock {
            has_bottom_dock = true;
            if bottom_view.read(cx).panel.entity_id() == cx.view().entity_id() {
                self_is_bottom_dock = true;
            }
        }

        // Check the dock origin vs self.bounds.origin, if they are in the same line, then render the ToggleButton
        match placement {
            DockPlacement::Left => {
                if !has_left_dock {
                    return None;
                }

                if self_is_left_dock || self_is_right_dock || self_is_bottom_dock {
                    return None;
                }
                if let Some(parent) = self
                    .stack_panel
                    .as_ref()
                    .and_then(|parent| parent.upgrade())
                {
                    if !parent
                        .read(cx)
                        .is_top_left_panel(cx.view().clone(), true, cx)
                    {
                        return None;
                    }
                }
            }
            DockPlacement::Right => {
                if !has_right_dock {
                    return None;
                }

                if self_is_left_dock || self_is_right_dock || self_is_bottom_dock {
                    return None;
                }

                if let Some(parent) = self
                    .stack_panel
                    .as_ref()
                    .and_then(|parent| parent.upgrade())
                {
                    if !parent
                        .read(cx)
                        .is_top_right_panel(cx.view().clone(), true, cx)
                    {
                        return None;
                    }
                }
            }
            DockPlacement::Bottom => {
                if !has_bottom_dock {
                    return None;
                }
                if !self_is_bottom_dock {
                    return None;
                }
            }
        }

        let is_left_dock_open = dock_area
            .read(cx)
            .is_dock_open(super::DockPlacement::Left, cx);
        let is_right_dock_open = dock_area
            .read(cx)
            .is_dock_open(super::DockPlacement::Right, cx);
        let is_bottom_dock_open = dock_area
            .read(cx)
            .is_dock_open(super::DockPlacement::Bottom, cx);

        let (icon, is_open) = match placement {
            DockPlacement::Left => {
                if is_left_dock_open {
                    (IconName::PanelLeft, true)
                } else {
                    (IconName::PanelLeftOpen, false)
                }
            }
            DockPlacement::Right => {
                if is_right_dock_open {
                    (IconName::PanelRight, true)
                } else {
                    (IconName::PanelRightOpen, false)
                }
            }
            DockPlacement::Bottom => {
                if is_bottom_dock_open {
                    (IconName::PanelBottom, true)
                } else {
                    (IconName::PanelBottomOpen, false)
                }
            }
        };

        Some(
            Button::new(SharedString::from(format!("toggle-dock:{:?}", placement)))
                .icon(icon)
                .xsmall()
                .ghost()
                .tooltip(match is_open {
                    true => t!("Dock.Collapse"),
                    false => t!("Dock.Expand"),
                })
                .on_click(cx.listener({
                    let dock_area = dock_area.clone();
                    move |_, _, cx| {
                        dock_area.update(cx, |dock_area, cx| {
                            dock_area.toggle_dock(placement, cx);
                        });
                    }
                })),
        )
    }

    fn render_tabs(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let view = cx.view().clone();

        let left_dock_button = self.render_dock_toggle_button(DockPlacement::Left, cx);
        let bottom_dock_button = self.render_dock_toggle_button(DockPlacement::Bottom, cx);
        let right_dock_button = self.render_dock_toggle_button(DockPlacement::Right, cx);

        if self.panels.len() == 1 {
            let panel = self.panels.get(0).unwrap();
            let title_style = panel.title_style(cx);

            return h_flex()
                .justify_between()
                .items_center()
                .line_height(rems(1.0))
                .h(px(30.))
                .py_2()
                .px_3()
                .when(left_dock_button.is_some(), |this| this.pl_2())
                .when(right_dock_button.is_some(), |this| this.pr_2())
                .when_some(title_style, |this, theme| {
                    this.bg(theme.background).text_color(theme.foreground)
                })
                .when(
                    left_dock_button.is_some() || bottom_dock_button.is_some(),
                    |this| {
                        this.child(
                            h_flex()
                                .flex_shrink_0()
                                .mr_1()
                                .gap_1()
                                .children(left_dock_button)
                                .children(bottom_dock_button),
                        )
                    },
                )
                .child(
                    div()
                        .id("tab")
                        .flex_1()
                        .min_w_16()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(panel.title(cx))
                        .when(self.can_split(), |this| {
                            this.on_drag(
                                DragPanel {
                                    panel: panel.clone(),
                                    tab_panel: view,
                                },
                                |drag, cx| {
                                    cx.stop_propagation();
                                    cx.new_view(|_| drag.clone())
                                },
                            )
                        }),
                )
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .ml_1()
                        .gap_1()
                        .child(self.render_menu_button(cx))
                        .children(right_dock_button),
                )
                .into_any_element();
        }

        let tabs_count = self.panels.len();

        TabBar::new("tab-bar")
            .track_scroll(self.tab_bar_scroll_handle.clone())
            .when(
                left_dock_button.is_some() || bottom_dock_button.is_some(),
                |this| {
                    this.prefix(
                        h_flex()
                            .items_center()
                            .top_0()
                            .right_0()
                            .border_r_1()
                            .border_b_1()
                            .h_full()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().tab_bar)
                            .px_2()
                            .children(left_dock_button)
                            .children(bottom_dock_button),
                    )
                },
            )
            .children(self.panels.iter().enumerate().map(|(ix, panel)| {
                let mut active = ix == self.active_ix;

                // Always not show active tab style, if the panel is collapsed
                if self.is_collapsed {
                    active = false;
                }

                Tab::new(("tab", ix), panel.title(cx))
                    .py_2()
                    .selected(active)
                    .on_click(cx.listener(move |view, _, cx| {
                        view.set_active_ix(ix, cx);
                    }))
                    .when(self.can_split(), |this| {
                        this.on_drag(DragPanel::new(panel.clone(), view.clone()), |drag, cx| {
                            cx.stop_propagation();
                            cx.new_view(|_| drag.clone())
                        })
                        .drag_over::<DragPanel>(|this, _, cx| {
                            this.rounded_l_none()
                                .border_l_2()
                                .border_r_0()
                                .border_color(cx.theme().drag_border)
                        })
                        .on_drop(cx.listener(
                            move |this, drag: &DragPanel, cx| {
                                this.will_split_placement = None;
                                this.on_drop(drag, Some(ix), cx)
                            },
                        ))
                    })
            }))
            .child(
                // empty space to allow move to last tab right
                div()
                    .id("tab-bar-empty-space")
                    .h_full()
                    .flex_grow()
                    .min_w_16()
                    .when(self.can_split(), |this| {
                        this.drag_over::<DragPanel>(|this, _, cx| this.bg(cx.theme().drop_target))
                            .on_drop(cx.listener(move |this, drag: &DragPanel, cx| {
                                this.will_split_placement = None;

                                let ix = if drag.tab_panel == view {
                                    Some(tabs_count - 1)
                                } else {
                                    None
                                };

                                this.on_drop(drag, ix, cx)
                            }))
                    }),
            )
            .suffix(
                h_flex()
                    .items_center()
                    .top_0()
                    .right_0()
                    .border_l_1()
                    .border_b_1()
                    .h_full()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().tab_bar)
                    .px_2()
                    .gap_1()
                    .child(self.render_menu_button(cx))
                    .when_some(right_dock_button, |this, btn| this.child(btn)),
            )
            .into_any_element()
    }

    fn render_active_panel(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.active_panel()
            .map(|panel| {
                div()
                    .id("tab-content")
                    .group("")
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .flex_1()
                    .child(panel.view())
                    .when(self.can_split(), |this| {
                        this.on_drag_move(cx.listener(Self::on_panel_drag_move))
                            .child(
                                div()
                                    .invisible()
                                    .absolute()
                                    .bg(cx.theme().drop_target)
                                    .map(|this| match self.will_split_placement {
                                        Some(placement) => {
                                            let size = DefiniteLength::Fraction(0.35);
                                            match placement {
                                                Placement::Left => {
                                                    this.left_0().top_0().bottom_0().w(size)
                                                }
                                                Placement::Right => {
                                                    this.right_0().top_0().bottom_0().w(size)
                                                }
                                                Placement::Top => {
                                                    this.top_0().left_0().right_0().h(size)
                                                }
                                                Placement::Bottom => {
                                                    this.bottom_0().left_0().right_0().h(size)
                                                }
                                            }
                                        }
                                        None => this.top_0().left_0().size_full(),
                                    })
                                    .group_drag_over::<DragPanel>("", |this| this.visible())
                                    .on_drop(cx.listener(|this, drag: &DragPanel, cx| {
                                        this.on_drop(drag, None, cx)
                                    })),
                            )
                    })
                    .into_any_element()
            })
            .unwrap_or(Empty {}.into_any_element())
    }

    /// Calculate the split direction based on the current mouse position
    fn on_panel_drag_move(&mut self, drag: &DragMoveEvent<DragPanel>, cx: &mut ViewContext<Self>) {
        let bounds = drag.bounds;
        let position = drag.event.position;

        // Check the mouse position to determine the split direction
        if position.x < bounds.left() + bounds.size.width * 0.35 {
            self.will_split_placement = Some(Placement::Left);
        } else if position.x > bounds.left() + bounds.size.width * 0.65 {
            self.will_split_placement = Some(Placement::Right);
        } else if position.y < bounds.top() + bounds.size.height * 0.35 {
            self.will_split_placement = Some(Placement::Top);
        } else if position.y > bounds.top() + bounds.size.height * 0.65 {
            self.will_split_placement = Some(Placement::Bottom);
        } else {
            // center to merge into the current tab
            self.will_split_placement = None;
        }
        cx.notify()
    }

    fn on_drop(&mut self, drag: &DragPanel, ix: Option<usize>, cx: &mut ViewContext<Self>) {
        let panel = drag.panel.clone();
        let is_same_tab = drag.tab_panel == *cx.view();

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
        if is_same_tab {
            self.detach_panel(panel.clone(), cx);
        } else {
            let _ = drag.tab_panel.update(cx, |view, cx| {
                view.detach_panel(panel.clone(), cx);
                view.remove_self_if_empty(cx);
            });
        }

        // Insert into new tabs
        if let Some(placement) = self.will_split_placement {
            self.split_panel(panel, placement, None, cx);
        } else {
            if let Some(ix) = ix {
                self.insert_panel_at(panel, ix, cx)
            } else {
                self.add_panel(panel, cx)
            }
        }

        self.remove_self_if_empty(cx);
        cx.emit(PanelEvent::LayoutChanged);
    }

    /// Add panel with split placement
    fn split_panel(
        &self,
        panel: Arc<dyn PanelView>,
        placement: Placement,
        size: Option<Pixels>,
        cx: &mut ViewContext<Self>,
    ) {
        let dock_area = self.dock_area.clone();
        // wrap the panel in a TabPanel
        let new_tab_panel = cx.new_view(|cx| Self::new(None, dock_area.clone(), cx));
        new_tab_panel.update(cx, |view, cx| {
            view.add_panel(panel, cx);
        });

        let stack_panel = match self.stack_panel.as_ref().and_then(|panel| panel.upgrade()) {
            Some(panel) => panel,
            None => return,
        };

        let parent_axis = stack_panel.read(cx).axis;

        let ix = stack_panel
            .read(cx)
            .index_of_panel(Arc::new(cx.view().clone()))
            .unwrap_or_default();

        if parent_axis.is_vertical() && placement.is_vertical() {
            stack_panel.update(cx, |view, cx| {
                view.insert_panel_at(
                    Arc::new(new_tab_panel),
                    ix,
                    placement,
                    size,
                    dock_area.clone(),
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
                    cx,
                );
            });
        } else {
            // 1. Create new StackPanel with new axis
            // 2. Move cx.view() from parent StackPanel to the new StackPanel
            // 3. Add the new TabPanel to the new StackPanel at the correct index
            // 4. Add new StackPanel to the parent StackPanel at the correct index
            let tab_panel = cx.view().clone();

            // Try to use the old stack panel, not just create a new one, to avoid too many nested stack panels
            let new_stack_panel = if stack_panel.read(cx).panels_len() <= 1 {
                stack_panel.update(cx, |view, cx| {
                    view.remove_all_panels(cx);
                    view.set_axis(placement.axis(), cx);
                });
                stack_panel.clone()
            } else {
                cx.new_view(|cx| {
                    let mut panel = StackPanel::new(placement.axis(), cx);
                    panel.parent = Some(stack_panel.downgrade());
                    panel
                })
            };

            new_stack_panel.update(cx, |view, cx| match placement {
                Placement::Left | Placement::Top => {
                    view.add_panel(Arc::new(new_tab_panel), size, dock_area.clone(), cx);
                    view.add_panel(Arc::new(tab_panel.clone()), None, dock_area.clone(), cx);
                }
                Placement::Right | Placement::Bottom => {
                    view.add_panel(Arc::new(tab_panel.clone()), None, dock_area.clone(), cx);
                    view.add_panel(Arc::new(new_tab_panel), size, dock_area.clone(), cx);
                }
            });

            if stack_panel != new_stack_panel {
                stack_panel.update(cx, |view, cx| {
                    view.replace_panel(Arc::new(tab_panel.clone()), new_stack_panel.clone(), cx);
                });
            }

            cx.spawn(|_, mut cx| async move {
                cx.update(|cx| tab_panel.update(cx, |view, cx| view.remove_self_if_empty(cx)))
            })
            .detach()
        }

        cx.emit(PanelEvent::LayoutChanged);
    }

    fn focus_active_panel(&self, cx: &mut ViewContext<Self>) {
        if let Some(active_panel) = self.active_panel() {
            active_panel.focus_handle(cx).focus(cx);
        }
    }

    fn on_action_toggle_zoom(&mut self, _: &ToggleZoom, cx: &mut ViewContext<Self>) {
        if !self.zoomable(cx) {
            return;
        }

        if !self.is_zoomed {
            cx.emit(PanelEvent::ZoomIn)
        } else {
            cx.emit(PanelEvent::ZoomOut)
        }
        self.is_zoomed = !self.is_zoomed;
    }

    fn on_action_close_panel(&mut self, _: &ClosePanel, cx: &mut ViewContext<Self>) {
        if let Some(panel) = self.active_panel() {
            self.remove_panel(panel, cx);
        }
    }
}

impl FocusableView for TabPanel {
    fn focus_handle(&self, cx: &AppContext) -> gpui::FocusHandle {
        if let Some(active_panel) = self.active_panel() {
            active_panel.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
impl EventEmitter<DismissEvent> for TabPanel {}
impl EventEmitter<PanelEvent> for TabPanel {}
impl Render for TabPanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl gpui::IntoElement {
        let focus_handle = self.focus_handle(cx);

        v_flex()
            .id("tab-panel")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_action_toggle_zoom))
            .on_action(cx.listener(Self::on_action_close_panel))
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            .child(self.render_tabs(cx))
            .child(self.render_active_panel(cx))
    }
}
