use std::{ops::Deref, rc::Rc};

use gpui::{
    canvas, div, prelude::FluentBuilder, px, relative, Along, AnyElement, AnyView, App, AppContext,
    Axis, Bounds, Context, Element, Empty, Entity, EntityId, EventEmitter, IntoElement,
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
    ratios: Vec<f32>,
    axis: Axis,
    ratio: Option<f32>,
    bounds: Bounds<Pixels>,
    resizing_panel_ix: Option<usize>,
}

impl ResizablePanelGroup {
    pub(super) fn new() -> Self {
        Self {
            axis: Axis::Horizontal,
            ratios: Vec::new(),
            panels: Vec::new(),
            ratio: None,
            bounds: Bounds::default(),
            resizing_panel_ix: None,
        }
    }

    pub fn load(&mut self, ratios: Vec<f32>, panels: Vec<Entity<ResizablePanel>>) {
        self.ratios = ratios;
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
        let ratio = group.ratio;
        let panel = ResizablePanel::new()
            .content_view(cx.new(|_| group).into())
            .when_some(ratio, |this, ratio| this.ratio(ratio));
        self.child(panel, cx)
    }

    /// Set size of the resizable panel group
    ///
    /// - When the axis is horizontal, the size is the height of the group.
    /// - When the axis is vertical, the size is the width of the group.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = Some(ratio);
        self
    }

    /// Returns the ratios of the panels.
    pub(crate) fn ratios(&self) -> &Vec<f32> {
        &self.ratios
    }

    /// Calculates the sum of all panel sizes within the group.
    pub fn total_size(&self) -> Pixels {
        self.bounds.size.along(self.axis)
    }

    pub fn add_child(&mut self, panel: ResizablePanel, cx: &mut Context<Self>) {
        let mut panel = panel;
        panel.axis = self.axis;
        panel.group = Some(cx.entity().downgrade());
        let ratio = match panel.ratio {
            Some(ratio) => ratio,
            None => panel.initial_radio.unwrap_or_default(),
        };

        self.ratios.push(ratio);
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
        let ratio = match panel.ratio {
            Some(ratio) => ratio,
            None => panel.initial_radio.unwrap_or_default(),
        };
        self.ratios.insert(ix, ratio);
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
        let old_panel_initial_ratio = old_panel.read(cx).initial_radio;
        let old_panel_ratio = old_panel.read(cx).ratio;

        panel.initial_radio = old_panel_initial_ratio;
        panel.ratio = old_panel_ratio;
        panel.axis = self.axis;
        panel.group = Some(cx.entity().downgrade());

        let ratio = match panel.ratio {
            Some(ratio) => ratio,
            None => panel.initial_radio.unwrap_or_default(),
        };

        self.ratios[ix] = ratio;
        self.panels[ix] = cx.new(|_| panel);
        cx.notify()
    }

    pub fn remove_child(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.ratios.remove(ix);
        self.panels.remove(ix);
        cx.notify()
    }

    pub(crate) fn remove_all_children(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.ratios.clear();
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

    fn sync_real_panel_sizes(&mut self, _: &Window, cx: &mut App) {
        let total_size = self.total_size();
        for (i, panel) in self.panels.iter().enumerate() {
            self.ratios[i] = panel.read(cx).bounds.size.along(self.axis) / total_size;
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

        let new_ratio = size / container_size;
        let mut changed = new_ratio - self.ratios[ix];
        let is_expand = changed > 0.;

        let main_ix = ix;
        let mut new_ratios = self.ratios.clone();
        let min_ratio = PANEL_MIN_SIZE / container_size;

        if is_expand {
            new_ratios[ix] = new_ratio;

            // Now to expand logic is correct.
            while changed > 0. && ix < self.panels.len() - 1 {
                ix += 1;
                let available_ratio = (new_ratios[ix] - min_ratio).max(0.);
                let to_reduce = changed.min(available_ratio);
                new_ratios[ix] -= to_reduce;
                changed -= to_reduce;
            }
        } else {
            let new_size = new_ratio.max(min_ratio);
            new_ratios[ix] = new_size;
            changed = new_ratio - min_ratio;
            new_ratios[ix + 1] += self.ratios[ix] - new_size;

            while changed < 0. && ix > 0 {
                ix -= 1;
                let available_ratio = self.ratios[ix] - min_ratio;
                let to_increase = (changed).min(available_ratio);
                new_ratios[ix] += to_increase;
                changed += to_increase;
            }
        }

        // If total size exceeds container size, adjust the main panel
        let total_ratio: f32 = new_ratios.iter().sum();
        if total_ratio > 1. {
            let overflow = 1.0 - total_ratio;
            new_ratios[main_ix] = (new_ratios[main_ix] - overflow).max(min_ratio);
        }

        self.ratios = new_ratios;
        for (i, panel) in self.panels.iter().enumerate() {
            let ratio = self.ratios[i];
            if ratio > 0. {
                panel.update(cx, |this, _| {
                    this.size = Some(container_size * ratio);
                    this.ratio = Some(ratio);
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
    initial_radio: Option<f32>,
    /// size is the size that the panel has when it is resized or adjusted by flex layout.
    size: Option<Pixels>,
    /// the size ratio that the panel has relative to its group
    ratio: Option<f32>,
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
            initial_radio: None,
            size: None,
            ratio: None,
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

    // /// Set the initial size of the panel.
    // pub fn size(mut self, size: Pixels) -> Self {
    //     self.initial_size = Some(size);
    //     self
    // }

    /// Set the flex ratio of the panel, the ratio is relative to the total size of the group.
    ///
    /// The `ratio` is 0.0 to 1.0.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.initial_radio = Some(ratio);
        self.ratio = Some(ratio);
        self
    }

    /// Save the real panel size, and update group sizes
    fn update_size(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let new_size = bounds.size.along(self.axis);
        self.bounds = bounds;
        self.ratio = None;
        self.size = Some(new_size);
        cx.notify();

        if let Some(group) = self.group.clone() {
            window.defer(cx, move |window, cx| {
                _ = group.update(cx, |view, cx| {
                    view.sync_real_panel_sizes(window, cx);
                });
            });
        }
    }
}

impl FluentBuilder for ResizablePanel {}

impl Render for ResizablePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !(self.content_visible)(window, cx) {
            // To keep size as initial size, to make sure the size will not be changed.
            self.initial_radio = self.ratio;
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
            .when(self.initial_radio.is_none(), |this| this.flex_shrink())
            .when(self.axis.is_vertical(), |this| this.min_h(PANEL_MIN_SIZE))
            .when(self.axis.is_horizontal(), |this| this.min_w(PANEL_MIN_SIZE))
            .when_some(self.initial_radio, |this, radio| {
                if radio == 0. {
                    this
                } else {
                    // The `self.size` is None, that mean the initial size for the panel, so we need set flex_shrink_0
                    // To let it keep the initial size.
                    this.when(self.size.is_none() && radio > 0., |this| {
                        this.flex_shrink_0()
                    })
                    .flex_basis(relative(radio))
                }
            })
            .map(|this| match (self.ratio, self.size, total_size) {
                (Some(ratio), _, _) => this.flex_basis(relative(ratio)),
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
