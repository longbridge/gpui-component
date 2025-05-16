use crate::{h_flex, tooltip::Tooltip, ActiveTheme, AxisExt};
use gpui::{
    canvas, div, prelude::FluentBuilder as _, px, App, AppContext as _, Axis, Bounds, Context,
    DragMoveEvent, Empty, Entity, EntityId, EventEmitter, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement as _, Pixels, Point, Render, RenderOnce,
    StatefulInteractiveElement as _, Styled, Window,
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

/// State of the [`Slider`].
pub struct SliderState {
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    bounds: Bounds<Pixels>,
    percentage: f32,
}

impl SliderState {
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: 0.0,
            percentage: 0.0,
            bounds: Bounds::default(),
        }
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
        axis: Axis,
        reverse: bool,
        position: Point<Pixels>,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let bounds = self.bounds;
        let min = self.min;
        let max = self.max;
        let step = self.step;

        let percentage = match axis {
            Axis::Horizontal => {
                if reverse {
                    1. - (position.x - bounds.left()).clamp(px(0.), bounds.size.width)
                        / bounds.size.width
                } else {
                    (position.x - bounds.left()).clamp(px(0.), bounds.size.width)
                        / bounds.size.width
                }
            }
            Axis::Vertical => {
                if reverse {
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
}

impl EventEmitter<SliderEvent> for SliderState {}
impl Render for SliderState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// A Slider element.
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    axis: Axis,
    reverse: bool,
    disabled: bool,
}

impl Slider {
    /// Create a new [`Slider`] element bind to the [`SliderState`].
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            axis: Axis::Horizontal,
            reverse: false,
            state: state.clone(),
            disabled: false,
        }
    }

    /// As a horizontal slider.
    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    /// As a vertical slider.
    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    /// Set the reverse direction of the slider, default: false
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Set the disabled state of the slider, default: false
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn render_thumb(
        &self,
        thumb_bar_size: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> impl gpui::IntoElement {
        let state = self.state.read(cx);
        let entity_id = self.state.entity_id();
        let value = state.value;
        let reverse = self.reverse;
        let axis = self.axis;

        if self.disabled {
            return div().id("slider-thumb");
        }

        div()
            .id("slider-thumb")
            .on_drag(DragThumb(entity_id), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move(window.listener_for(
                &self.state,
                move |view, e: &DragMoveEvent<DragThumb>, window, cx| {
                    match e.drag(cx) {
                        DragThumb(id) => {
                            if *id != entity_id {
                                return;
                            }

                            // set value by mouse position
                            view.update_value_by_position(
                                axis,
                                reverse,
                                e.event.position,
                                window,
                                cx,
                            )
                        }
                    }
                },
            ))
            .absolute()
            .map(|this| match reverse {
                true => this
                    .when(axis.is_horizontal(), |this| {
                        this.bottom(px(-5.)).right(thumb_bar_size).mr(-px(8.))
                    })
                    .when(axis.is_vertical(), |this| {
                        this.bottom(thumb_bar_size).right(px(-5.)).mb(-px(8.))
                    }),
                false => this
                    .when(axis.is_horizontal(), |this| {
                        this.top(px(-5.)).left(thumb_bar_size).ml(-px(8.))
                    })
                    .when(axis.is_vertical(), |this| {
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
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let state = self.state.read(cx);
        let axis = self.axis;
        let reverse = self.reverse;
        let thumb_bar_size = match axis {
            Axis::Horizontal => state.percentage * state.bounds.size.width,
            Axis::Vertical => state.percentage * state.bounds.size.height,
        };

        div()
            .id(("slider", self.state.entity_id()))
            .flex_1()
            .when(axis.is_vertical(), |this| {
                this.flex().items_center().justify_center()
            })
            .child(
                h_flex()
                    .when(!self.disabled, |this| {
                        this.on_mouse_down(
                            MouseButton::Left,
                            window.listener_for(
                                &self.state,
                                move |view, e: &MouseDownEvent, window, cx| {
                                    view.update_value_by_position(
                                        axis, reverse, e.position, window, cx,
                                    )
                                },
                            ),
                        )
                    })
                    .when(axis.is_horizontal(), |this| {
                        this.items_center().h_6().w_full()
                    })
                    .when(axis.is_vertical(), |this| {
                        this.justify_center().w_6().h_full()
                    })
                    .flex_shrink_0()
                    .child(
                        div()
                            .id("slider-bar")
                            .relative()
                            .when(axis.is_horizontal(), |this| this.w_full().h_1p5())
                            .when(axis.is_vertical(), |this| this.h_full().w_1p5())
                            .bg(cx.theme().slider_bar.opacity(0.2))
                            .active(|this| this.bg(cx.theme().slider_bar.opacity(0.4)))
                            .rounded(px(3.))
                            .child(
                                div()
                                    .absolute()
                                    .when(!reverse, |this| this.top_0().left_0())
                                    .when(reverse, |this| this.bottom_0().right_0())
                                    .when(axis.is_horizontal(), |this| {
                                        this.h_full().w(thumb_bar_size)
                                    })
                                    .when(axis.is_vertical(), |this| {
                                        this.w_full().h(thumb_bar_size)
                                    })
                                    .bg(cx.theme().slider_bar)
                                    .rounded_full(),
                            )
                            .child(self.render_thumb(thumb_bar_size, window, cx))
                            .child({
                                let state = self.state.clone();
                                canvas(
                                    move |bounds, _, cx| state.update(cx, |r, _| r.bounds = bounds),
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .size_full()
                            }),
                    ),
            )
    }
}
