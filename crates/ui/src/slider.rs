use crate::{h_flex, tooltip::Tooltip, ActiveTheme, AxisExt};
use gpui::{
    canvas, div, prelude::FluentBuilder as _, px, AppContext as _, Axis, Bounds, Context,
    DragMoveEvent, Empty, EntityId, EventEmitter, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement as _, Pixels, Point, Render, StatefulInteractiveElement as _,
    Styled, Window,
};

#[derive(Clone)]
pub struct DragThumb(EntityId);

impl Render for DragThumb {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

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
    percentage: f32,
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
            percentage: 0.0,
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
        self.update_thumb_pos();
        self
    }

    /// Set the maximum value of the slider, default: 100.0
    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self.update_thumb_pos();
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
    pub fn set_value(&mut self, value: f32, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.value = value;
        self.update_thumb_pos();
        cx.notify();
    }

    /// Get the value of the slider.
    pub fn value(&self) -> f32 {
        self.value
    }

    fn update_thumb_pos(&mut self) {
        self.percentage = self.value.clamp(self.min, self.max) / self.max;
    }

    /// Update value by mouse position
    fn update_value_by_position(
        &mut self,
        position: Point<Pixels>,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
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

        self.percentage = percentage;
        self.value = value.clamp(self.min, self.max);
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    fn render_thumb(
        &self,
        thumb_bar_size: Pixels,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let value = self.value;
        let entity_id = cx.entity_id();

        div()
            .id("slider-thumb")
            .on_drag(DragThumb(entity_id), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move(
                cx.listener(move |view, e: &DragMoveEvent<DragThumb>, window, cx| {
                    match e.drag(cx) {
                        DragThumb(id) => {
                            if *id != entity_id {
                                return;
                            }

                            // set value by mouse position
                            view.update_value_by_position(e.event.position, window, cx)
                        }
                    }
                }),
            )
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
            .tooltip(move |window, cx| Tooltip::new(format!("{}", value)).build(window, cx))
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.update_value_by_position(event.position, window, cx);
    }
}

impl EventEmitter<SliderEvent> for Slider {}

impl Render for Slider {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let thumb_bar_size = match self.axis {
            Axis::Horizontal => self.percentage * self.bounds.size.width,
            Axis::Vertical => self.percentage * self.bounds.size.height,
        };

        div()
            .id("slider")
            .flex_1()
            .when(self.axis.is_vertical(), |this| {
                this.flex().items_center().justify_center()
            })
            .child(
                h_flex()
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
                            .child(self.render_thumb(thumb_bar_size, window, cx))
                            .child({
                                let view = cx.entity().clone();
                                canvas(
                                    move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .size_full()
                            }),
                    ),
            )
    }
}
