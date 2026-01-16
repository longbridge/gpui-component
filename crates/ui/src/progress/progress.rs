use crate::{ActiveTheme, Sizable, Size, StyledExt};
use gpui::{
    Animation, AnimationExt as _, App, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
    relative,
};
use std::time::Duration;

use super::ProgressState;

/// A linear horizontal progress bar element.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
    size: Size,
}

impl Progress {
    /// Create a new Progress bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: Default::default(),
            color: None,
            style: StyleRefinement::default(),
            size: Size::default(),
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

impl RenderOnce for Progress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().progress_bar);
        let value = self.value;

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
                    .rounded(radius)
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
}
