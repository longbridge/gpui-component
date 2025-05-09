use std::{ops::Deref, rc::Rc};

use gpui::{
    canvas, div, prelude::FluentBuilder, px, relative, Along, AnyElement, AnyView, App, AppContext,
    Axis, Bounds, Context, Element, Empty, Entity, EntityId, EventEmitter, IntoElement, IsZero,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, Style, Styled, WeakEntity, Window,
};

use crate::{h_flex, v_flex, AxisExt};

use super::resize_handle;

pub(crate) const PANEL_MIN_SIZE: Pixels = px(100.);

pub enum ResizablePanelEvent {
    Resized,
}

#[derive(Clone)]
pub struct DragPanel(pub (EntityId, usize, Axis));

impl Render for DragPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
pub struct ResizablePanelGroup {
    panels: Vec<Entity<ResizablePanel>>,
    sizes: Vec<Pixels>,
    axis: Axis,
    size: Option<Pixels>,
    bounds: Bounds<Pixels>,
    resizing_panel_ix: Option<usize>,
}

impl ResizablePanelGroup {
    pub(super) fn new() -> Self {
        Self {
            axis: Axis::Horizontal,
            sizes: Vec::new(),
            panels: Vec::new(),
            size: None,
            bounds: Bounds::default(),
            resizing_panel_ix: None,
        }
    }

    pub fn load(&mut self, sizes: Vec<Pixels>, panels: Vec<Entity<ResizablePanel>>) {
        self.sizes = sizes;
        self.panels = panels;
    }

    /// Set the axis of the resizable panel group, default is horizontal.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub(crate) fn set_axis(&mut self, axis: Axis, _: &mut Window, cx: &mut Context<Self>) {
        self.axis = axis;
        cx.notify();
    }

    /// Add a resizable panel to the group.
    pub fn child(mut self, panel: ResizablePanel, cx: &mut Context<Self>) -> Self {
        self.add_child(panel, cx);
        self
    }

    /// Add a ResizablePanelGroup as a child to the group.
    pub fn group(self, group: ResizablePanelGroup, cx: &mut Context<Self>) -> Self {
        let group: ResizablePanelGroup = group;
        let size = group.size;
        let panel = ResizablePanel::new()
            .content_view(cx.new(|_| group).into())
            .when_some(size, |this, size| this.size(size));
        self.child(panel, cx)
    }

    /// Set size of the resizable panel group
    ///
    /// - When the axis is horizontal, the size is the height of the group.
    /// - When the axis is vertical, the size is the width of the group.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = Some(size);
        self
    }

    /// Returns the sizes of the resizable panels.
    pub(crate) fn sizes(&self) -> Vec<Pixels> {
        self.sizes.clone()
    }

    /// Calculates the sum of all panel sizes within the group.
    pub fn total_size(&self) -> Pixels {
        self.sizes.iter().fold(px(0.0), |acc, &size| acc + size)
    }

    pub fn add_child(&mut self, panel: ResizablePanel, cx: &mut Context<Self>) {
        let mut panel = panel;
        panel.axis = self.axis;
        panel.group = Some(cx.entity().downgrade());
        self.sizes.push(panel.initial_size.unwrap_or_default());
        self.panels.push(cx.new(|_| panel));
    }

    pub fn insert_child(
        &mut self,
        panel: ResizablePanel,
        ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut panel = panel;
        panel.axis = self.axis;
        panel.group = Some(cx.entity().downgrade());

        self.sizes
            .insert(ix, panel.initial_size.unwrap_or_default());
        self.panels.insert(ix, cx.new(|_| panel));
        cx.notify()
    }

    /// Replace a child panel with a new panel at the given index.
    pub(crate) fn replace_child(
        &mut self,
        panel: ResizablePanel,
        ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut panel = panel;

        let old_panel = self.panels[ix].clone();
        let old_panel_initial_size = old_panel.read(cx).initial_size;
        let old_panel_size_ratio = old_panel.read(cx).size_ratio;

        panel.initial_size = old_panel_initial_size;
        panel.size_ratio = old_panel_size_ratio;
        panel.axis = self.axis;
        panel.group = Some(cx.entity().downgrade());
        self.sizes[ix] = panel.initial_size.unwrap_or_default();
        self.panels[ix] = cx.new(|_| panel);
        cx.notify()
    }

    pub fn remove_child(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.sizes.remove(ix);
        self.panels.remove(ix);
        cx.notify()
    }

    pub(crate) fn remove_all_children(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.sizes.clear();
        self.panels.clear();
        cx.notify()
    }

    fn render_resize_handle(
        &self,
        ix: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();
        resize_handle(("resizable-handle", ix), self.axis).on_drag(
            DragPanel((cx.entity_id(), ix, self.axis)),
            move |drag_panel, _, _, cx| {
                cx.stop_propagation();
                // Set current resizing panel ix
                view.update(cx, |view, _| {
                    view.resizing_panel_ix = Some(ix);
                });
                cx.new(|_| drag_panel.deref().clone())
            },
        )
    }

    fn done_resizing(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(ResizablePanelEvent::Resized);
        self.resizing_panel_ix = None;
    }

    fn sync_real_panel_sizes(&mut self, _: &Window, cx: &App) {
        for (i, panel) in self.panels.iter().enumerate() {
            self.sizes[i] = panel.read(cx).bounds.size.along(self.axis)
        }
    }

    /// The `ix`` is the index of the panel to resize,
    /// and the `size` is the new size for the panel.
    fn resize_panels(
        &mut self,
        ix: usize,
        size: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut ix = ix;
        // Only resize the left panels.
        if ix >= self.panels.len() - 1 {
            return;
        }
        let size = size.floor();
        let container_size = self.bounds.size.along(self.axis);

        self.sync_real_panel_sizes(window, cx);

        let mut changed = size - self.sizes[ix];
        let is_expand = changed > px(0.);

        let main_ix = ix;
        let mut new_sizes = self.sizes.clone();

        if is_expand {
            new_sizes[ix] = size;

            // Now to expand logic is correct.
            while changed > px(0.) && ix < self.panels.len() - 1 {
                ix += 1;
                let available_size = (new_sizes[ix] - PANEL_MIN_SIZE).max(px(0.));
                let to_reduce = changed.min(available_size);
                new_sizes[ix] -= to_reduce;
                changed -= to_reduce;
            }
        } else {
            let new_size = size.max(PANEL_MIN_SIZE);
            new_sizes[ix] = new_size;
            changed = size - PANEL_MIN_SIZE;
            new_sizes[ix + 1] += self.sizes[ix] - new_size;

            while changed < px(0.) && ix > 0 {
                ix -= 1;
                let available_size = self.sizes[ix] - PANEL_MIN_SIZE;
                let to_increase = (changed).min(available_size);
                new_sizes[ix] += to_increase;
                changed += to_increase;
            }
        }

        // If total size exceeds container size, adjust the main panel
        let total_size: Pixels = new_sizes.iter().map(|s| s.0).sum::<f32>().into();
        if total_size > container_size {
            let overflow = total_size - container_size;
            new_sizes[main_ix] = (new_sizes[main_ix] - overflow).max(PANEL_MIN_SIZE);
        }

        let total_size = new_sizes.iter().fold(px(0.0), |acc, &size| acc + size);
        self.sizes = new_sizes;
        for (i, panel) in self.panels.iter().enumerate() {
            let size = self.sizes[i];
            if size > px(0.) {
                panel.update(cx, |this, _| {
                    this.size = Some(size);
                    this.size_ratio = Some(size / total_size);
                });
            }
        }
    }
}
impl EventEmitter<ResizablePanelEvent> for ResizablePanelGroup {}
impl Render for ResizablePanelGroup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let container = if self.axis.is_horizontal() {
            h_flex()
        } else {
            v_flex()
        };

        container
            .size_full()
            .children(self.panels.iter().enumerate().map(|(ix, panel)| {
                if ix > 0 {
                    let handle = self.render_resize_handle(ix - 1, window, cx);
                    panel.update(cx, |view, _| {
                        view.resize_handle = Some(handle.into_any_element())
                    });
                }

                panel.clone()
            }))
            .child({
                canvas(
                    move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .child(ResizePanelGroupElement {
                view: cx.entity().clone(),
                axis: self.axis,
            })
    }
}

pub struct ResizablePanel {
    group: Option<WeakEntity<ResizablePanelGroup>>,
    /// Initial size is the size that the panel has when it is created.
    initial_size: Option<Pixels>,
    /// size is the size that the panel has when it is resized or adjusted by flex layout.
    size: Option<Pixels>,
    /// the size ratio that the panel has relative to its group
    size_ratio: Option<f32>,
    axis: Axis,
    content_builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
    content_view: Option<AnyView>,
    content_visible: Rc<Box<dyn Fn(&Window, &App) -> bool>>,
    /// The bounds of the resizable panel, when render the bounds will be updated.
    bounds: Bounds<Pixels>,
    resize_handle: Option<AnyElement>,
}

impl ResizablePanel {
    pub(super) fn new() -> Self {
        Self {
            group: None,
            initial_size: None,
            size: None,
            size_ratio: None,
            axis: Axis::Horizontal,
            content_builder: None,
            content_view: None,
            content_visible: Rc::new(Box::new(|_, _| true)),
            bounds: Bounds::default(),
            resize_handle: None,
        }
    }

    pub fn content<F>(mut self, content: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> AnyElement + 'static,
    {
        self.content_builder = Some(Rc::new(content));
        self
    }

    pub(crate) fn content_visible<F>(mut self, content_visible: F) -> Self
    where
        F: Fn(&Window, &App) -> bool + 'static,
    {
        self.content_visible = Rc::new(Box::new(content_visible));
        self
    }

    pub fn content_view(mut self, content: AnyView) -> Self {
        self.content_view = Some(content);
        self
    }

    /// Set the initial size of the panel.
    pub fn size(mut self, size: Pixels) -> Self {
        self.initial_size = Some(size);
        self
    }

    /// Save the real panel size, and update group sizes
    fn update_size(&mut self, bounds: Bounds<Pixels>, _: &mut Window, cx: &mut Context<Self>) {
        let new_size = bounds.size.along(self.axis);
        self.bounds = bounds;
        self.size_ratio = None;
        self.size = Some(new_size);

        let entity_id = cx.entity().entity_id();
        if let Some(group) = self.group.as_ref() {
            _ = group.update(cx, |view, _| {
                if let Some(ix) = view.panels.iter().position(|v| v.entity_id() == entity_id) {
                    view.sizes[ix] = new_size;
                }
            });
        }
        cx.notify();
    }
}

impl FluentBuilder for ResizablePanel {}

impl Render for ResizablePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !(self.content_visible)(window, cx) {
            // To keep size as initial size, to make sure the size will not be changed.
            self.initial_size = self.size;
            self.size = None;
            return div();
        }

        let view = cx.entity().clone();
        let total_size = self
            .group
            .as_ref()
            .and_then(|group| group.upgrade())
            .map(|group| group.read(cx).total_size());

        div()
            .flex()
            .flex_grow()
            .size_full()
            .relative()
            .when(self.initial_size.is_none(), |this| this.flex_shrink())
            .when(self.axis.is_vertical(), |this| this.min_h(PANEL_MIN_SIZE))
            .when(self.axis.is_horizontal(), |this| this.min_w(PANEL_MIN_SIZE))
            .when_some(self.initial_size, |this, size| {
                if size.is_zero() {
                    this
                } else {
                    // The `self.size` is None, that mean the initial size for the panel, so we need set flex_shrink_0
                    // To let it keep the initial size.
                    this.when(self.size.is_none() && size > px(0.), |this| {
                        this.flex_shrink_0()
                    })
                    .flex_basis(size)
                }
            })
            .map(|this| match (self.size_ratio, self.size, total_size) {
                (Some(size_ratio), _, _) => this.flex_basis(relative(size_ratio)),
                (None, Some(size), Some(total_size)) => {
                    this.flex_basis(relative(size / total_size))
                }
                (None, Some(size), None) => this.flex_basis(size),
                _ => this,
            })
            .child({
                canvas(
                    move |bounds, window, cx| {
                        view.update(cx, |r, cx| r.update_size(bounds, window, cx))
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .when_some(self.content_builder.clone(), |this, c| {
                this.child(c(window, cx))
            })
            .when_some(self.content_view.clone(), |this, c| this.child(c))
            .when_some(self.resize_handle.take(), |this, c| this.child(c))
    }
}

struct ResizePanelGroupElement {
    axis: Axis,
    view: Entity<ResizablePanelGroup>,
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
            let view = self.view.clone();
            let axis = self.axis;
            let current_ix = view.read(cx).resizing_panel_ix;
            move |e: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                let Some(ix) = current_ix else { return };

                view.update(cx, |view, cx| {
                    let panel = view
                        .panels
                        .get(ix)
                        .expect("BUG: invalid panel index")
                        .read(cx);

                    match axis {
                        Axis::Horizontal => {
                            view.resize_panels(ix, e.position.x - panel.bounds.left(), window, cx)
                        }
                        Axis::Vertical => {
                            view.resize_panels(ix, e.position.y - panel.bounds.top(), window, cx);
                        }
                    }
                })
            }
        });

        // When any mouse up, stop dragging
        window.on_mouse_event({
            let view = self.view.clone();
            let current_ix = view.read(cx).resizing_panel_ix;
            move |_: &MouseUpEvent, phase, window, cx| {
                if current_ix.is_none() {
                    return;
                }
                if phase.bubble() {
                    view.update(cx, |view, cx| view.done_resizing(window, cx));
                }
            }
        })
    }
}
