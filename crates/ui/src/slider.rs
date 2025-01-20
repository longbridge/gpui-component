use crate::{h_flex, theme::ActiveTheme, tooltip::Tooltip, AxisExt};
use gpui::{
    canvas, div, prelude::FluentBuilder as _, px, Axis, Bounds, DragMoveEvent, EntityId,
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
    reverse: bool,
    /// Percentage of the thumb position relative to the slider bar.
    percentage: Option<f32>,
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
            percentage: None,
            reverse: false,
            bounds: Bounds::default(),
        }
    }

    /// Create a horizontal slider.
    pub fn horizontal() -> Self {
        Self::new(Axis::Horizontal)
    }

    /// Create a vertical slider.
    pub fn vertical() -> Self {
        Self::new(Axis::Vertical)
    }

    /// Set the reverse direction of the slider, default: false
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
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
        self.update_thumb_pos();
        self
    }

    /// Set the value of the slider.
    pub fn set_value(&mut self, value: f32, cx: &mut gpui::ViewContext<Self>) {
        self.value = value;
        self.update_thumb_pos();
        cx.notify();
    }

    fn update_thumb_pos(&mut self) {
        self.percentage = Some(self.value.clamp(self.min, self.max) / self.max);
    }

    /// Get the value of the slider.
    pub fn value(&self) -> f32 {
        self.value
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

        let percentage = match axis {
            Axis::Horizontal => {
                if self.reverse {
                    1. - (position.x - bounds.left()).clamp(px(0.), bounds.size.width)
                        / bounds.size.width
                } else {
                    (position.x - bounds.left()).clamp(px(0.), bounds.size.width)
                        / bounds.size.width
                }
            }
            Axis::Vertical => {
                if self.reverse {
                    1. - (position.y - bounds.top()).clamp(px(0.), bounds.size.height)
                        / bounds.size.height
                } else {
                    (position.y - bounds.top()).clamp(px(0.), bounds.size.height)
                        / bounds.size.height
                }
            }
        };

        let value = match axis {
            Axis::Horizontal => min + (max - min) * percentage,
            Axis::Vertical => max - (max - min) * percentage,
        };

        let value = (value / step).round() * step;

        self.percentage = Some(percentage);
        self.value = value.clamp(self.min, self.max);
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    fn render_thumb(
        &self,
        thumb_bar_size: Pixels,
        cx: &mut ViewContext<Self>,
    ) -> impl gpui::IntoElement {
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
            .map(|this| match self.reverse {
                true => this
                    .when(self.axis.is_horizontal(), |this| {
                        this.bottom(px(-5.)).right(thumb_bar_size).mr(-px(8.))
                    })
                    .when(self.axis.is_vertical(), |this| {
                        this.bottom(thumb_bar_size).right(px(-5.)).mb(-px(8.))
                    }),
                false => this
                    .when(self.axis.is_horizontal(), |this| {
                        this.top(px(-5.)).left(thumb_bar_size).ml(-px(8.))
                    })
                    .when(self.axis.is_vertical(), |this| {
                        this.top(thumb_bar_size).left(px(-5.)).mt(-px(8.))
                    }),
            })
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
        let thumb_bar_size = match self.axis {
            Axis::Horizontal => self.percentage.unwrap_or(0.) * self.bounds.size.width,
            Axis::Vertical => self.percentage.unwrap_or(0.) * self.bounds.size.height,
        };

        h_flex()
            .id("slider")
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .when(self.axis.is_horizontal(), |this| {
                this.items_center().h_6().w_full()
            })
            .when(self.axis.is_vertical(), |this| {
                this.justify_center().w_6().h_full()
            })
            .flex_shrink_0()
            .child(
                div()
                    .id("slider-bar")
                    .relative()
                    .when(self.axis.is_horizontal(), |this| this.w_full().h_1p5())
                    .when(self.axis.is_vertical(), |this| this.h_full().w_1p5())
                    .bg(cx.theme().slider_bar.opacity(0.2))
                    .active(|this| this.bg(cx.theme().slider_bar.opacity(0.4)))
                    .rounded(px(3.))
                    .child(
                        div()
                            .absolute()
                            .when(!self.reverse, |this| this.top_0().left_0())
                            .when(self.reverse, |this| this.bottom_0().right_0())
                            .when(self.axis.is_horizontal(), |this| {
                                this.h_full().w(thumb_bar_size)
                            })
                            .when(self.axis.is_vertical(), |this| {
                                this.w_full().h(thumb_bar_size)
                            })
                            .bg(cx.theme().slider_bar)
                            .rounded_full(),
                    )
                    .child(self.render_thumb(thumb_bar_size, cx))
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
