use crate::{ActiveTheme, PixelsExt, Sizable, Size, StyledExt, h_flex};
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Bounds, ElementId, Hsla,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, RenderOnce, StyleRefinement,
    Styled, Window, canvas, div, prelude::FluentBuilder, px, relative,
};
use std::f32::consts::TAU;
use std::time::Duration;

use crate::plot::shape::{Arc, ArcData};

/// Progress bar display mode.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressMode {
    /// Linear horizontal progress bar (default).
    #[default]
    Linear,
    /// Circular progress indicator.
    Circle,
}

/// A Progress bar element.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
    size: Size,
    mode: ProgressMode,
}

impl Progress {
    /// Create a new Progress bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Progress {
            id: id.into(),
            value: Default::default(),
            color: None,
            style: StyleRefinement::default(),
            size: Size::default(),
            mode: ProgressMode::Linear,
        }
    }

    /// Set the color of the progress bar.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the percentage value of the progress bar.
    ///
    /// The value should be between 0.0 and 100.0.
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0., 100.);
        self
    }

    /// Set the progress bar to circle mode.
    ///
    /// In circle mode, the progress is displayed as a circular arc.
    /// The size can be controlled using `w()` and `h()` style methods.
    pub fn circle(mut self) -> Self {
        self.mode = ProgressMode::Circle;
        self
    }
}

impl Styled for Progress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Progress {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

struct ProgressState {
    value: f32,
}

impl RenderOnce for Progress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().progress_bar);
        let value = self.value;
        let mode = self.mode;

        match mode {
            ProgressMode::Linear => self
                .render_linear(window, cx, color, value)
                .into_any_element(),
            ProgressMode::Circle => self
                .render_circle(window, cx, color, value)
                .into_any_element(),
        }
    }
}

impl Progress {
    fn render_linear(
        self,
        window: &mut Window,
        cx: &mut App,
        color: Hsla,
        value: f32,
    ) -> impl IntoElement {
        let radius = self.style.corner_radii.clone();
        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii = radius;

        let (height, radius) = match self.size {
            Size::XSmall => (px(4.), px(2.)),
            Size::Small => (px(6.), px(3.)),
            Size::Medium => (px(8.), px(4.)),
            Size::Large => (px(10.), px(5.)),
            Size::Size(s) => (s, s / 2.),
        };

        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ProgressState { value });
        let prev_value = state.read(cx).value;

        div()
            .id(self.id)
            .w_full()
            .relative()
            .rounded_full()
            .h(height)
            .rounded(radius)
            .refine_style(&self.style)
            .bg(color.opacity(0.2))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .bg(color)
                    .refine_style(&inner_style)
                    .map(|this| match value {
                        v if v >= 100. => this,
                        _ => this.rounded_r_none(),
                    })
                    .map(|this| {
                        if prev_value != value {
                            // Animate from prev_value to value
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
                                "progress-animation",
                                Animation::new(duration),
                                move |this, delta| {
                                    let current_value = prev_value + (value - prev_value) * delta;
                                    let relative_w = relative(match current_value {
                                        v if v < 0. => 0.,
                                        v if v > 100. => 1.,
                                        v => v / 100.,
                                    });
                                    this.w(relative_w)
                                },
                            )
                            .into_any_element()
                        } else {
                            let relative_w = relative(match value {
                                v if v < 0. => 0.,
                                v if v > 100. => 1.,
                                v => v / 100.,
                            });
                            this.w(relative_w).into_any_element()
                        }
                    }),
            )
    }

    fn render_circle(
        self,
        window: &mut Window,
        cx: &mut App,
        color: Hsla,
        value: f32,
    ) -> AnyElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ProgressState { value });

        let id = self.id.clone();
        let state_clone = state.clone();
        let color_clone = color;
        let target_value = value;

        h_flex()
            .items_center()
            .justify_center()
            .id(id)
            .map(|this| match self.size {
                Size::XSmall => this.size_2(),
                Size::Small => this.size_3(),
                Size::Medium => this.size_4(),
                Size::Large => this.size_5(),
                Size::Size(s) => this.size(s * 0.75),
            })
            .refine_style(&self.style)
            .child(
                canvas(
                    // Prepaint callback: prepare data
                    move |bounds: Bounds<Pixels>, _window: &mut Window, cx: &mut App| {
                        // Update state in prepaint
                        let current_state_value = state_clone.read(cx).value;
                        if current_state_value != target_value {
                            _ = state_clone.update(cx, |this, _| this.value = target_value);
                        }
                        let current_value = state_clone.read(cx).value;
                        // Use 15% of width as stroke width, but max 5px
                        let stroke_width = (bounds.size.width * 0.15).min(px(5.));

                        // Calculate actual size from bounds
                        let actual_size = bounds.size.width.min(bounds.size.height);
                        let actual_radius = (actual_size.as_f32() - stroke_width.as_f32()) / 2.;
                        let actual_inner_radius = actual_radius - stroke_width.as_f32() / 2.;
                        let actual_outer_radius = actual_radius + stroke_width.as_f32() / 2.;

                        (
                            current_value,
                            actual_inner_radius,
                            actual_outer_radius,
                            bounds,
                        )
                    },
                    // Paint callback: actually draw
                    move |_bounds,
                          (
                        current_value,
                        actual_inner_radius,
                        actual_outer_radius,
                        prepaint_bounds,
                    ),
                          window: &mut Window,
                          _cx: &mut App| {
                        // Draw background circle
                        let bg_arc_data = ArcData {
                            data: &(),
                            index: 0,
                            value: 100.,
                            start_angle: 0.,
                            end_angle: TAU,
                            pad_angle: 0.,
                        };

                        let bg_arc = Arc::new()
                            .inner_radius(actual_inner_radius)
                            .outer_radius(actual_outer_radius);

                        bg_arc.paint(
                            &bg_arc_data,
                            color_clone.opacity(0.2),
                            None,
                            None,
                            &prepaint_bounds,
                            window,
                        );

                        // Draw progress arc
                        if current_value > 0. {
                            let progress_angle = (current_value / 100.) * TAU;
                            let progress_arc_data = ArcData {
                                data: &(),
                                index: 1,
                                value: current_value,
                                start_angle: 0.,
                                end_angle: progress_angle,
                                pad_angle: 0.,
                            };

                            let progress_arc = Arc::new()
                                .inner_radius(actual_inner_radius)
                                .outer_radius(actual_outer_radius);

                            progress_arc.paint(
                                &progress_arc_data,
                                color_clone,
                                None,
                                None,
                                &prepaint_bounds,
                                window,
                            );
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .into_any_element()
    }
}
