use crate::{ActiveTheme, PixelsExt, Sizable, Size, StyledExt};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement, Pixels, RenderOnce,
    StyleRefinement, Styled, Window, canvas, px,
};
use gpui::{Bounds, div};
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
}

impl ProgressCircle {
    /// Create a new circular progress indicator.
    pub fn new(id: impl Into<ElementId>) -> Self {
        ProgressCircle {
            id: id.into(),
            value: Default::default(),
            color: None,
            style: StyleRefinement::default(),
            size: Size::default(),
        }
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

impl RenderOnce for ProgressCircle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().progress_bar);
        let value = self.value;

        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ProgressState { value });

        let id = self.id.clone();
        let state_clone = state.clone();
        let color_clone = color;
        let target_value = value;

        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
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
