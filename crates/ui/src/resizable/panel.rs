use std::ops::{Deref, Range};

use gpui::{
    canvas, div, prelude::FluentBuilder, AnyElement, App, AppContext, Axis, Bounds, Context,
    Element, ElementId, Empty, Entity, EventEmitter, InteractiveElement as _, IntoElement, IsZero,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, RenderOnce, Style, Styled, Window,
};

use crate::{h_flex, resizable::PANEL_MIN_SIZE, v_flex, AxisExt};

use super::{resizable_panel, resize_handle, ResizableState};

pub enum ResizablePanelEvent {
    Resized,
}

#[derive(Clone)]
pub struct DragPanel(pub (usize, Axis));

impl Render for DragPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(IntoElement)]
pub struct ResizablePanelGroup {
    id: ElementId,
    state: Entity<ResizableState>,
    axis: Axis,
    size: Option<Pixels>,
    children: Vec<ResizablePanel>,
}

impl ResizablePanelGroup {
    pub(crate) fn new(id: impl Into<ElementId>, state: Entity<ResizableState>) -> Self {
        Self {
            id: id.into(),
            axis: Axis::Horizontal,
            children: vec![],
            state,
            size: None,
        }
    }

    /// Set the axis of the resizable panel group, default is horizontal.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Add a panel to the group.
    ///
    /// - The `axis` will be set to the same axis as the group.
    /// - The `initial_size` will be set to the average size of all panels if not provided.
    /// - The `group` will be set to the group entity.
    pub fn child(mut self, panel: impl Into<ResizablePanel>) -> Self {
        self.children.push(panel.into());
        self
    }

    pub fn children<I>(mut self, panels: impl IntoIterator<Item = I>) -> Self
    where
        I: Into<ResizablePanel>,
    {
        self.children = panels.into_iter().map(|panel| panel.into()).collect();
        self
    }

    /// Add a ResizablePanelGroup as a child to the group.
    pub fn group(self, group: ResizablePanelGroup) -> Self {
        self.child(resizable_panel().child(group.into_any_element()))
    }

    /// Set size of the resizable panel group
    ///
    /// - When the axis is horizontal, the size is the height of the group.
    /// - When the axis is vertical, the size is the width of the group.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = Some(size);
        self
    }
}
impl<T> From<T> for ResizablePanel
where
    T: Into<AnyElement>,
{
    fn from(value: T) -> Self {
        resizable_panel().child(value.into())
    }
}

impl EventEmitter<ResizablePanelEvent> for ResizablePanelGroup {}

impl RenderOnce for ResizablePanelGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.clone();
        let container = if self.axis.is_horizontal() {
            h_flex()
        } else {
            v_flex()
        };

        // Sync panels to the state
        let panels_count = self.children.len();
        self.state.update(cx, |state, _| {
            state.sync_panels_count(self.axis, panels_count);
        });

        container
            .id(self.id)
            .size_full()
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, mut panel)| {
                        panel.panel_ix = ix;
                        panel.axis = self.axis;
                        panel.state = Some(self.state.clone());
                        panel
                    }),
            )
            .child({
                canvas(
                    move |bounds, _, cx| state.update(cx, |state, _| state.bounds = bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .child(ResizePanelGroupElement {
                state: self.state.clone(),
                axis: self.axis,
            })
    }
}

#[derive(IntoElement)]
pub struct ResizablePanel {
    axis: Axis,
    panel_ix: usize,
    state: Option<Entity<ResizableState>>,
    /// Initial size is the size that the panel has when it is created.
    initial_size: Option<Pixels>,
    /// size range limit of this panel.
    size_range: Range<Pixels>,
    children: Vec<AnyElement>,
    visible: bool,
}

impl ResizablePanel {
    pub(super) fn new() -> Self {
        Self {
            panel_ix: 0,
            initial_size: None,
            state: None,
            size_range: (PANEL_MIN_SIZE..Pixels::MAX),
            axis: Axis::Horizontal,
            children: vec![],
            visible: true,
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set the initial size of the panel.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.initial_size = Some(size.into());
        self
    }

    /// Set the size range to limit panel resize.
    ///
    /// Default is [`PANEL_MIN_SIZE`] to [`Pixels::MAX`].
    pub fn size_range(mut self, range: impl Into<Range<Pixels>>) -> Self {
        self.size_range = range.into();
        self
    }
}

impl RenderOnce for ResizablePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.visible {
            return div().id(("resizable-panel", self.panel_ix));
        }

        let state = self
            .state
            .expect("BUG: The `state` in ResizablePanel should be present.");
        let panel_state = state
            .read(cx)
            .panels
            .get(self.panel_ix)
            .expect("BUG: The `index` of ResizablePanel should be one of in `state`.");
        let size_range = self.size_range.clone();

        div()
            .id(("resizable-panel", self.panel_ix))
            .flex()
            .flex_grow()
            .size_full()
            .relative()
            .when(self.axis.is_vertical(), |this| {
                this.min_h(size_range.start).max_h(size_range.end)
            })
            .when(self.axis.is_horizontal(), |this| {
                this.min_w(size_range.start).max_w(size_range.end)
            })
            // 1. initial_size is None, to use auto size.
            // 2. initial_size is Some and size is none, to use the initial size of the panel for first time render.
            // 3. initial_size is Some and size is Some, use `size`.
            .when(self.initial_size.is_none(), |this| this.flex_shrink())
            .when_some(self.initial_size, |this, initial_size| {
                // The `self.size` is None, that mean the initial size for the panel,
                // so we need set `flex_shrink_0` To let it keep the initial size.
                this.when(
                    panel_state.size.is_none() && !initial_size.is_zero(),
                    |this| this.flex_none(),
                )
                .flex_basis(initial_size)
            })
            .map(|this| match panel_state.size {
                Some(size) => this.flex_basis(size),
                None => this,
            })
            .child({
                canvas(
                    {
                        let state = state.clone();
                        move |bounds, _, cx| {
                            state.update(cx, |state, cx| {
                                state.update_panel_size(self.panel_ix, bounds, self.size_range, cx)
                            })
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .children(self.children)
            .when(self.panel_ix > 0, |this| {
                let ix = self.panel_ix - 1;
                this.child(resize_handle(("resizable-handle", ix), self.axis).on_drag(
                    DragPanel((ix, self.axis)),
                    move |drag_panel, _, _, cx| {
                        cx.stop_propagation();
                        // Set current resizing panel ix
                        state.update(cx, |state, _| {
                            state.resizing_panel_ix = Some(ix);
                        });
                        cx.new(|_| drag_panel.deref().clone())
                    },
                ))
            })
    }
}

struct ResizePanelGroupElement {
    state: Entity<ResizableState>,
    axis: Axis,
}

impl IntoElement for ResizePanelGroupElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ResizePanelGroupElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        ()
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.on_mouse_event({
            let state = self.state.clone();
            let axis = self.axis;
            let current_ix = state.read(cx).resizing_panel_ix;
            move |e: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                let Some(ix) = current_ix else { return };

                state.update(cx, |state, cx| {
                    let panel = state.panels.get(ix).expect("BUG: invalid panel index");

                    match axis {
                        Axis::Horizontal => {
                            state.resize_panel(ix, e.position.x - panel.bounds.left(), window, cx)
                        }
                        Axis::Vertical => {
                            state.resize_panel(ix, e.position.y - panel.bounds.top(), window, cx);
                        }
                    }
                    cx.notify();
                })
            }
        });

        // When any mouse up, stop dragging
        window.on_mouse_event({
            let state = self.state.clone();
            let current_ix = state.read(cx).resizing_panel_ix;
            move |_: &MouseUpEvent, phase, _, cx| {
                if current_ix.is_none() {
                    return;
                }
                if phase.bubble() {
                    state.update(cx, |state, cx| state.done_resizing(cx));
                }
            }
        })
    }
}
