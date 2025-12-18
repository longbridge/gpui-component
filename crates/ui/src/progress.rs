use crate::{ActiveTheme, StyledExt};
use gpui::{
    Animation, AnimationExt as _, App, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
    relative,
};
use std::time::Duration;

/// A Progress bar element.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
}

impl Progress {
    /// Create a new Progress bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Progress {
            id: id.into(),
            value: Default::default(),
            color: None,
            style: StyleRefinement::default().h(px(8.)).rounded(px(4.)),
        }
    }

    /// Set the color of the progress bar.
    pub fn bg(mut self, color: impl Into<Hsla>) -> Self {
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

struct ProgressState {
    value: f32,
}

impl RenderOnce for Progress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let radius = self.style.corner_radii.clone();
        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii = radius;

        let color = self.color.unwrap_or(cx.theme().progress_bar);
        let value = self.value;

        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ProgressState { value });
        let prev_value = state.read(cx).value;

        div()
            .id(self.id)
            .w_full()
            .relative()
            .rounded_full()
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
                                    let current_value =
                                        prev_value + (value - prev_value) * delta;
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
