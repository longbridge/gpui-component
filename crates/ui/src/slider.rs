use crate::{h_flex, theme::ActiveTheme, tooltip::Tooltip, StyledExt};
use gpui::{
    canvas, div, prelude::FluentBuilder as _, px, relative, Axis, Bounds, DragMoveEvent, EntityId,
    EventEmitter, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement as _,
    Pixels, Point, Render, StatefulInteractiveElement as _, Styled, ViewContext,
    VisualContext as _,
};

#[derive(Clone, Render)]
pub struct DragThumb(EntityId);

pub enum SliderEvent {
    Change(f32),
}

/// A Slider element.
pub struct Slider {
    axis: Axis,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    thumb_pos: Pixels,
    bounds: Bounds<Pixels>,
}

impl Slider {
    fn new(axis: Axis) -> Self {
        Self {
            axis,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: 0.0,
            thumb_pos: px(0.),
            bounds: Bounds::default(),
        }
    }

    pub fn horizontal() -> Self {
        Self::new(Axis::Horizontal)
    }

    /// Set the minimum value of the slider, default: 0.0
    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    /// Set the maximum value of the slider, default: 100.0
    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    /// Set the step value of the slider, default: 1.0
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the default value of the slider, default: 0.0
    pub fn default_value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    /// Set the value of the slider.
    pub fn set_value(&mut self, value: f32, cx: &mut gpui::ViewContext<Self>) {
        self.value = value;
        cx.notify();
    }

    /// Update value by mouse position
    fn update_value_by_position(
        &mut self,
        position: Point<Pixels>,
        cx: &mut gpui::ViewContext<Self>,
    ) {
        let bounds = self.bounds;
        let axis = self.axis;
        let min = self.min;
        let max = self.max;
        let step = self.step;

        match axis {
            Axis::Horizontal => {
                self.thumb_pos = (position.x - bounds.left()).clamp(px(0.), bounds.size.width);
            }
            Axis::Vertical => {
                self.thumb_pos = (position.y - bounds.top()).clamp(px(0.), bounds.size.height);
            }
        }

        let value = match axis {
            Axis::Horizontal => {
                let relative = (self.thumb_pos) / bounds.size.width;
                min + (max - min) * relative
            }
            Axis::Vertical => {
                let relative = (self.thumb_pos) / bounds.size.height;
                max - (max - min) * relative
            }
        };

        let value = (value / step).round() * step;

        self.value = value.clamp(self.min, self.max);
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    fn render_thumb(&self, cx: &mut ViewContext<Self>) -> impl gpui::IntoElement {
        let value = self.value;
        let entity_id = cx.entity_id();

        div()
            .id("slider-thumb")
            .on_drag(DragThumb(entity_id), |drag, _, cx| {
                cx.stop_propagation();
                cx.new_view(|_| drag.clone())
            })
            .on_drag_move(cx.listener(
                move |view, e: &DragMoveEvent<DragThumb>, cx| match e.drag(cx) {
                    DragThumb(id) => {
                        if *id != entity_id {
                            return;
                        }

                        // set value by mouse position
                        view.update_value_by_position(e.event.position, cx)
                    }
                },
            ))
            .absolute()
            .top(px(-5.))
            .left(self.thumb_pos)
            .ml(-px(8.))
            .size_4()
            .rounded_full()
            .border_1()
            .border_color(cx.theme().slider_bar.opacity(0.9))
            .when(cx.theme().shadow, |this| this.shadow_md())
            .bg(cx.theme().slider_thumb)
            .tooltip(move |cx| Tooltip::new(format!("{}", value), cx))
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut gpui::ViewContext<Self>) {
        self.update_value_by_position(event.position, cx);
    }
}

impl EventEmitter<SliderEvent> for Slider {}

impl Render for Slider {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        h_flex()
            .id("slider")
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .w_full()
            .h_6()
            .flex_shrink_0()
            .items_center()
            .child(
                div()
                    .id("slider-bar")
                    .relative()
                    .w_full()
                    .h_1p5()
                    .bg(cx.theme().slider_bar.opacity(0.2))
                    .active(|this| this.bg(cx.theme().slider_bar.opacity(0.4)))
                    .rounded(px(3.))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(self.thumb_pos)
                            .bg(cx.theme().slider_bar)
                            .rounded_l(px(3.)),
                    )
                    .child(self.render_thumb(cx))
                    .child({
                        let view = cx.view().clone();
                        canvas(
                            move |bounds, cx| view.update(cx, |r, _| r.bounds = bounds),
                            |_, _, _| {},
                        )
                        .absolute()
                        .size_full()
                    }),
            )
    }
}
