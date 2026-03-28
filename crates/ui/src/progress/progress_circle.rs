use crate::{ActiveTheme, Sizable, Size, StyledExt};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, ElementId, Hsla, InteractiveElement as _,
    IntoElement, ParentElement, Pixels, RenderOnce, StyleRefinement, Styled, Window, canvas,
    ease_in_out, px, relative,
};
use gpui::{Bounds, div};
use instant::Duration;
use std::f32::consts::TAU;

use super::ProgressState;
use crate::plot::shape::{Arc, ArcData};

/// A circular progress indicator element.
#[derive(IntoElement)]
pub struct ProgressCircle {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
    size: Size,
    children: Vec<AnyElement>,
    loading: bool,
}

impl ProgressCircle {
    /// Create a new circular progress indicator.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: Default::default(),
            color: None,
            style: StyleRefinement::default(),
            size: Size::default(),
            children: Vec::new(),
            loading: false,
        }
    }

    /// Enable indeterminate loading animation.
    ///
    /// When `loading` is `true`, the `value` is ignored and an infinite
    /// rotating arc animation is shown instead.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set the color of the progress circle.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the percentage value of the progress circle.
    ///
    /// The value should be between 0.0 and 100.0.
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0., 100.);
        self
    }

    /// Render the arc canvas. `start_value` and `end_value` are in 0.0–100.0 percentage.
    /// The progress arc is skipped when `end_value <= 0`.
    fn render_circle(start_value: f32, end_value: f32, color: Hsla) -> impl IntoElement {
        struct PrepaintState {
            start_value: f32,
            end_value: f32,
            actual_inner_radius: f32,
            actual_outer_radius: f32,
            bounds: Bounds<Pixels>,
        }

        canvas(
            move |bounds: Bounds<Pixels>, _window: &mut Window, _cx: &mut App| {
                let stroke_width = (bounds.size.width * 0.15).min(px(5.));
                let actual_size = bounds.size.width.min(bounds.size.height);
                let actual_radius = (actual_size.as_f32() - stroke_width.as_f32()) / 2.;
                PrepaintState {
                    start_value,
                    end_value,
                    actual_inner_radius: actual_radius - stroke_width.as_f32() / 2.,
                    actual_outer_radius: actual_radius + stroke_width.as_f32() / 2.,
                    bounds,
                }
            },
            move |_bounds, prepaint, window: &mut Window, _cx: &mut App| {
                let arc = Arc::new()
                    .inner_radius(prepaint.actual_inner_radius)
                    .outer_radius(prepaint.actual_outer_radius);

                arc.paint(
                    &ArcData {
                        data: &(),
                        index: 0,
                        value: 100.,
                        start_angle: 0.,
                        end_angle: TAU,
                        pad_angle: 0.,
                    },
                    color.opacity(0.2),
                    None,
                    None,
                    &prepaint.bounds,
                    window,
                );

                if prepaint.end_value > 0. {
                    let start_angle = (prepaint.start_value / 100.) * TAU;
                    let end_angle = (prepaint.end_value / 100.) * TAU;
                    arc.paint(
                        &ArcData {
                            data: &(),
                            index: 1,
                            value: prepaint.end_value,
                            start_angle,
                            end_angle,
                            pad_angle: 0.,
                        },
                        color,
                        None,
                        None,
                        &prepaint.bounds,
                        window,
                    );
                }
            },
        )
        .absolute()
        .size_full()
    }
}

impl Styled for ProgressCircle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for ProgressCircle {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for ProgressCircle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ProgressCircle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let value = self.value;
        let loading = self.loading;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ProgressState { value });
        let prev_value = state.read(cx).value;
        let has_changed = prev_value != value;

        let color = self.color.unwrap_or(cx.theme().progress_bar);

        div()
            .id(self.id.clone())
            .flex()
            .items_center()
            .justify_center()
            .line_height(relative(1.))
            .map(|this| match self.size {
                Size::XSmall => this.size_2(),
                Size::Small => this.size_3(),
                Size::Medium => this.size_4(),
                Size::Large => this.size_5(),
                Size::Size(s) => this.size(s * 0.75),
            })
            .refine_style(&self.style)
            .children(self.children)
            .map(|this| {
                if has_changed {
                    let duration = Duration::from_secs_f64(0.15);
                    cx.spawn({
                        let state = state.clone();
                        async move |cx| {
                            cx.background_executor().timer(duration).await;
                            _ = state.update(cx, |this, _| this.value = value);
                        }
                    })
                    .detach();

                    this.with_animation(
                        format!("progress-circle-{}", prev_value),
                        Animation::new(duration),
                        move |this, delta| {
                            let v = prev_value + (value - prev_value) * delta;
                            this.child(Self::render_circle(0., v, color))
                        },
                    )
                    .into_any_element()
                } else if loading {
                    this.with_animation(
                        "progress-circle-loading",
                        Animation::new(Duration::from_secs(1)).repeat(),
                        move |this, delta| {
                            let end = ease_in_out(delta) * 100.;
                            let start = ease_in_out(((delta - 0.5) / 0.5).clamp(0., 1.)) * 100.;
                            this.child(Self::render_circle(start, end, color))
                        },
                    )
                    .into_any_element()
                } else {
                    this.child(Self::render_circle(0., value, color))
                        .into_any_element()
                }
            })
    }
}
