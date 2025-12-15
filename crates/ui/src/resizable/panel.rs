use std::{
    ops::{Deref, Range},
    rc::Rc,
};

use gpui::{
    Along, AnyElement, App, AppContext, Axis, Bounds, Context, Element, ElementId, Empty, Entity,
    EventEmitter, InteractiveElement as _, IntoElement, IsZero as _, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Render, RenderOnce, Style, Styled, Window, div, prelude::FluentBuilder,
};

use crate::{AxisExt, ElementExt, h_flex, resizable::PANEL_MIN_SIZE, v_flex};

use super::{ResizableState, resizable_panel, resize_handle};

pub enum ResizablePanelEvent {
    Resized,
}

#[derive(Clone)]
pub(crate) struct DragPanel;
impl Render for DragPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        Empty
    }
}

/// A group of resizable panels.
#[derive(IntoElement)]
pub struct ResizablePanelGroup {
    id: ElementId,
    state: Option<Entity<ResizableState>>,
    axis: Axis,
    size: Option<Pixels>,
    children: Vec<ResizablePanel>,
    on_resize: Rc<dyn Fn(&Entity<ResizableState>, &mut Window, &mut App)>,
}

impl ResizablePanelGroup {
    /// Create a new resizable panel group.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            axis: Axis::Horizontal,
            children: vec![],
            state: None,
            size: None,
            on_resize: Rc::new(|_, _, _| {}),
        }
    }

    /// Bind yourself to a resizable state entity.
    ///
    /// If not provided, it will handle its own state internally.
    pub fn with_state(mut self, state: &Entity<ResizableState>) -> Self {
        self.state = Some(state.clone());
        self
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

    /// Add multiple panels to the group.
    pub fn children<I>(mut self, panels: impl IntoIterator<Item = I>) -> Self
    where
        I: Into<ResizablePanel>,
    {
        self.children = panels.into_iter().map(|panel| panel.into()).collect();
        self
    }

    /// Set size of the resizable panel group
    ///
    /// - When the axis is horizontal, the size is the height of the group.
    /// - When the axis is vertical, the size is the width of the group.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the callback to be called when the panels are resized.
    ///
    /// ## Callback arguments
    ///
    /// - Entity<ResizableState>: The state of the ResizablePanelGroup.
    pub fn on_resize(
        mut self,
        on_resize: impl Fn(&Entity<ResizableState>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Rc::new(on_resize);
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

impl From<ResizablePanelGroup> for ResizablePanel {
    fn from(value: ResizablePanelGroup) -> Self {
        resizable_panel().child(value)
    }
}

impl EventEmitter<ResizablePanelEvent> for ResizablePanelGroup {}

impl RenderOnce for ResizablePanelGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.unwrap_or(
            window.use_keyed_state(self.id.clone(), cx, |_, _| ResizableState::default()),
        );
        let container = if self.axis.is_horizontal() {
            h_flex()
        } else {
            v_flex()
        };

        // Sync panels to the state
        let panels_count = self.children.len();
        state.update(cx, |state, cx| {
            state.sync_panels_count(self.axis, panels_count, cx);
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
                        panel.state = Some(state.clone());
                        panel
                    }),
            )
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, cx| {
                        let size_changed =
                            state.bounds.size.along(self.axis) != bounds.size.along(self.axis);

                        state.bounds = bounds;

                        if size_changed {
                            state.adjust_to_container_size(cx);
                        }
                    })
                }
            })
            .child(ResizePanelGroupElement {
                state: state.clone(),
                axis: self.axis,
                on_resize: self.on_resize.clone(),
            })
    }
}

/// A resizable panel inside a [`ResizablePanelGroup`].
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
    /// Create a new resizable panel.
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

    /// Set the visibility of the panel, default is true.
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

impl ParentElement for ResizablePanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
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
                Some(size) => this.flex_basis(size.min(size_range.end).max(size_range.start)),
                None => this,
            })
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, cx| {
                        state.update_panel_size(self.panel_ix, bounds, self.size_range, cx)
                    })
                }
            })
            .children(self.children)
            .when(self.panel_ix > 0, |this| {
                let ix = self.panel_ix - 1;
                this.child(resize_handle(("resizable-handle", ix), self.axis).on_drag(
                    DragPanel,
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
    on_resize: Rc<dyn Fn(&Entity<ResizableState>, &mut Window, &mut App)>,
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

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
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
        _: Option<&gpui::InspectorElementId>,
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
            let on_resize = self.on_resize.clone();
            move |_: &MouseUpEvent, phase, window, cx| {
                if current_ix.is_none() {
                    return;
                }
                if phase.bubble() {
                    state.update(cx, |state, cx| state.done_resizing(cx));
                    on_resize(&state, window, cx);
                }
            }
        })
    }
}
