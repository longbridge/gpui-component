//! Dock is a fixed container that places at left, bottom, right of the Windows.

use std::sync::Arc;

use gpui::{
    div, prelude::FluentBuilder as _, px, Axis, Element, Entity, InteractiveElement as _,
    IntoElement, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point, Render,
    StatefulInteractiveElement, Style, Styled as _, View, ViewContext, VisualContext as _,
    WeakView, WindowContext,
};
use serde::{Deserialize, Serialize};

use crate::{
    resizable::{HANDLE_PADDING, HANDLE_SIZE, PANEL_MIN_SIZE},
    ActiveTheme as _, AxisExt as _, StyledExt,
};

use super::{DockArea, DockItem, PanelView, TabPanel};

#[derive(Clone, Render)]
struct ResizePanel;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DockPlacement {
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "right")]
    Right,
}

impl DockPlacement {
    fn axis(&self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Horizontal,
            Self::Bottom => Axis::Vertical,
            Self::Center => unreachable!(),
        }
    }

    pub fn is_left(&self) -> bool {
        matches!(self, Self::Left)
    }

    pub fn is_bottom(&self) -> bool {
        matches!(self, Self::Bottom)
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right)
    }
}

/// The Dock is a fixed container that places at left, bottom, right of the Windows.
///
/// This is unlike Panel, it can't be move or add any other panel.
pub struct Dock {
    pub(super) placement: DockPlacement,
    dock_area: WeakView<DockArea>,
    pub(crate) panel: DockItem,
    /// The size is means the width or height of the Dock, if the placement is left or right, the size is width, otherwise the size is height.
    pub(super) size: Pixels,
    pub(super) open: bool,
    /// Whether the Dock is collapsible, default: true
    pub(super) collapsible: bool,

    // Runtime state
    /// Whether the Dock is resizing
    is_resizing: bool,
}

impl Dock {
    pub(crate) fn new(
        dock_area: WeakView<DockArea>,
        placement: DockPlacement,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let panel = cx.new_view(|cx| {
            let mut tab = TabPanel::new(None, dock_area.clone(), cx);
            tab.closable = false;
            tab
        });

        let panel = DockItem::Tabs {
            items: Vec::new(),
            active_ix: 0,
            view: panel.clone(),
        };

        Self::subscribe_panel_events(dock_area.clone(), &panel, cx);

        Self {
            placement,
            dock_area,
            panel,
            open: true,
            collapsible: true,
            size: px(200.0),
            is_resizing: false,
        }
    }

    pub fn left(dock_area: WeakView<DockArea>, cx: &mut ViewContext<Self>) -> Self {
        Self::new(dock_area, DockPlacement::Left, cx)
    }

    pub fn bottom(dock_area: WeakView<DockArea>, cx: &mut ViewContext<Self>) -> Self {
        Self::new(dock_area, DockPlacement::Bottom, cx)
    }

    pub fn right(dock_area: WeakView<DockArea>, cx: &mut ViewContext<Self>) -> Self {
        Self::new(dock_area, DockPlacement::Right, cx)
    }

    /// Update the Dock to be collapsible or not.
    ///
    /// And if the Dock is not collapsible, it will be open.
    pub fn set_collapsible(&mut self, collapsible: bool, cx: &mut ViewContext<Self>) {
        self.collapsible = collapsible;
        if !collapsible {
            self.open = true
        }
        cx.notify();
    }

    pub(super) fn from_state(
        dock_area: WeakView<DockArea>,
        placement: DockPlacement,
        size: Pixels,
        panel: DockItem,
        open: bool,
        cx: &mut WindowContext,
    ) -> Self {
        Self::subscribe_panel_events(dock_area.clone(), &panel, cx);

        if !open {
            match panel.clone() {
                DockItem::Tabs { view, .. } => {
                    view.update(cx, |panel, cx| {
                        panel.set_collapsed(true, cx);
                    });
                }
                DockItem::Split { items, .. } => {
                    for item in items {
                        item.set_collapsed(true, cx);
                    }
                }
                _ => {}
            }
        }

        Self {
            placement,
            dock_area,
            panel,
            open,
            size,
            collapsible: true,
            is_resizing: false,
        }
    }

    fn subscribe_panel_events(
        dock_area: WeakView<DockArea>,
        panel: &DockItem,
        cx: &mut WindowContext,
    ) {
        match panel {
            DockItem::Tabs { view, .. } => {
                cx.defer({
                    let view = view.clone();
                    move |cx| {
                        _ = dock_area.update(cx, |this, cx| {
                            this.subscribe_panel(&view, cx);
                        });
                    }
                });
            }
            DockItem::Split { items, view, .. } => {
                for item in items {
                    Self::subscribe_panel_events(dock_area.clone(), item, cx);
                }
                cx.defer({
                    let view = view.clone();
                    move |cx| {
                        _ = dock_area.update(cx, |this, cx| {
                            this.subscribe_panel(&view, cx);
                        });
                    }
                });
            }
            DockItem::Tiles { view, .. } => {
                cx.defer({
                    let view = view.clone();
                    move |cx| {
                        _ = dock_area.update(cx, |this, cx| {
                            this.subscribe_panel(&view, cx);
                        });
                    }
                });
            }
            DockItem::Panel { .. } => {
                // Not supported
            }
        }
    }

    pub fn set_panel(&mut self, panel: DockItem, cx: &mut ViewContext<Self>) {
        self.panel = panel;
        cx.notify();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle_open(&mut self, cx: &mut ViewContext<Self>) {
        self.set_open(!self.open, cx);
    }

    /// Returns the size of the Dock, the size is means the width or height of
    /// the Dock, if the placement is left or right, the size is width,
    /// otherwise the size is height.
    pub fn size(&self) -> Pixels {
        self.size
    }

    /// Set the size of the Dock.
    pub fn set_size(&mut self, size: Pixels, cx: &mut ViewContext<Self>) {
        self.size = size.max(PANEL_MIN_SIZE);
        cx.notify();
    }

    /// Set the open state of the Dock.
    pub fn set_open(&mut self, open: bool, cx: &mut ViewContext<Self>) {
        self.open = open;
        let item = self.panel.clone();
        cx.defer(move |_, cx| {
            item.set_collapsed(!open, cx);
        });
        cx.notify();
    }

    /// Add item to the Dock.
    pub fn add_panel(&mut self, panel: Arc<dyn PanelView>, cx: &mut ViewContext<Self>) {
        self.panel.add_panel(panel, &self.dock_area, cx);
        cx.notify();
    }

    fn render_resize_handle(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let axis = self.placement.axis();
        let neg_offset = -HANDLE_PADDING;
        let view = cx.view().clone();

        div()
            .id("resize-handle")
            .occlude()
            .absolute()
            .flex_shrink_0()
            .when(self.placement.is_left(), |this| {
                // FIXME: Improve this to let the scroll bar have px(HANDLE_PADDING)
                this.cursor_col_resize()
                    .top_0()
                    .right(px(1.))
                    .h_full()
                    .w(HANDLE_SIZE)
                    .pl(HANDLE_PADDING)
            })
            .when(self.placement.is_right(), |this| {
                this.cursor_col_resize()
                    .top_0()
                    .left(neg_offset)
                    .h_full()
                    .w(HANDLE_SIZE)
                    .px(HANDLE_PADDING)
            })
            .when(self.placement.is_bottom(), |this| {
                this.cursor_row_resize()
                    .top(neg_offset)
                    .left_0()
                    .w_full()
                    .h(HANDLE_SIZE)
                    .py(HANDLE_PADDING)
            })
            .child(
                div()
                    .bg(cx.theme().border)
                    .when(axis.is_horizontal(), |this| this.h_full().w(HANDLE_SIZE))
                    .when(axis.is_vertical(), |this| this.w_full().h(HANDLE_SIZE)),
            )
            .on_drag(ResizePanel {}, move |info, _, cx| {
                cx.stop_propagation();
                view.update(cx, |view, _| {
                    view.is_resizing = true;
                });
                cx.new_view(|_| info.clone())
            })
    }
    fn resize(&mut self, mouse_position: Point<Pixels>, cx: &mut ViewContext<Self>) {
        if !self.is_resizing {
            return;
        }

        let dock_area = self
            .dock_area
            .upgrade()
            .expect("DockArea is missing")
            .read(cx);
        let area_bounds = dock_area.bounds;
        let mut left_dock_size = Pixels(0.0);
        let mut right_dock_size = Pixels(0.0);

        // Get the size of the left dock if it's open and not the current dock
        if let Some(left_dock) = &dock_area.left_dock {
            if left_dock.entity_id() != cx.view().entity_id() {
                let left_dock_read = left_dock.read(cx);
                if left_dock_read.is_open() {
                    left_dock_size = left_dock_read.size;
                }
            }
        }

        // Get the size of the right dock if it's open and not the current dock
        if let Some(right_dock) = &dock_area.right_dock {
            if right_dock.entity_id() != cx.view().entity_id() {
                let right_dock_read = right_dock.read(cx);
                if right_dock_read.is_open() {
                    right_dock_size = right_dock_read.size;
                }
            }
        }

        let size = match self.placement {
            DockPlacement::Left => mouse_position.x - area_bounds.left(),
            DockPlacement::Right => area_bounds.right() - mouse_position.x,
            DockPlacement::Bottom => area_bounds.bottom() - mouse_position.y,
            DockPlacement::Center => unreachable!(),
        };
        match self.placement {
            DockPlacement::Left => {
                let max_size = area_bounds.size.width - PANEL_MIN_SIZE - right_dock_size;
                self.size = size.clamp(PANEL_MIN_SIZE, max_size);
            }
            DockPlacement::Right => {
                let max_size = area_bounds.size.width - PANEL_MIN_SIZE - left_dock_size;
                self.size = size.clamp(PANEL_MIN_SIZE, max_size);
            }
            DockPlacement::Bottom => {
                let max_size = area_bounds.size.height - PANEL_MIN_SIZE;
                self.size = size.clamp(PANEL_MIN_SIZE, max_size);
            }
            DockPlacement::Center => unreachable!(),
        }

        cx.notify();
    }

    fn done_resizing(&mut self, _: &mut ViewContext<Self>) {
        self.is_resizing = false;
    }
}

impl Render for Dock {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl gpui::IntoElement {
        if !self.open && !self.placement.is_bottom() {
            return div();
        }

        div()
            .relative()
            .overflow_hidden()
            .map(|this| match self.placement {
                DockPlacement::Left | DockPlacement::Right => this.h_flex().h_full().w(self.size),
                DockPlacement::Bottom => this.w_full().h(self.size),
                DockPlacement::Center => unreachable!(),
            })
            // Bottom Dock should keep the title bar, then user can click the Toggle button
            .when(!self.open && self.placement.is_bottom(), |this| {
                this.h(px(29.))
            })
            .map(|this| match &self.panel {
                DockItem::Split { view, .. } => this.child(view.clone()),
                DockItem::Tabs { view, .. } => this.child(view.clone()),
                DockItem::Panel { view, .. } => this.child(view.clone().view()),
                // Not support to render Tiles and Tile into Dock
                DockItem::Tiles { .. } => this,
            })
            .child(self.render_resize_handle(cx))
            .child(DockElement {
                view: cx.view().clone(),
            })
    }
}

struct DockElement {
    view: View<Dock>,
}

impl IntoElement for DockElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DockElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        cx: &mut gpui::WindowContext,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (cx.request_layout(Style::default(), None), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut gpui::WindowContext,
    ) -> Self::PrepaintState {
        ()
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        cx: &mut gpui::WindowContext,
    ) {
        cx.on_mouse_event({
            let view = self.view.clone();
            move |e: &MouseMoveEvent, phase, cx| {
                if phase.bubble() {
                    view.update(cx, |view, cx| view.resize(e.position, cx))
                }
            }
        });

        // When any mouse up, stop dragging
        cx.on_mouse_event({
            let view = self.view.clone();
            move |_: &MouseUpEvent, phase, cx| {
                if phase.bubble() {
                    view.update(cx, |view, cx| view.done_resizing(cx));
                }
            }
        })
    }
}
