use crate::{ActiveTheme, Sizable, Size, StyledExt};
use gpui::{
    Animation, AnimationExt as _, App, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StyleRefinement, Styled, Window, div, ease_in_out,
    prelude::FluentBuilder, px, relative,
};
use instant::Duration;

use super::ProgressState;

/// A linear horizontal progress bar element.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
    size: Size,
    loading: bool,
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
            loading: false,
        }
    }

    /// Enable indeterminate loading animation.
    ///
    /// When `loading` is `true`, the `value` is ignored and an infinite
    /// sliding animation is shown instead.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
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
        let loading = self.loading;

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

        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ProgressState::new(value));
        let prev_target = state.read(cx).target();
        let has_changed = prev_target != value;

        div()
            .id(self.id)
            .w_full()
            .relative()
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
                        v if v >= 100. || loading => this,
                        _ => this.rounded_r_none(),
                    })
                    .map(|this| {
                        if has_changed {
                            let from = prev_target;
                            state.read(cx).set_target(value);

                            let duration = Duration::from_secs_f64(0.15);
                            cx.spawn({
                                let state = state.clone();
                                async move |cx| {
                                    cx.background_executor().timer(duration).await;
                                    _ = state.update(cx, |this, _| {
                                        this.value = this.target();
                                    });
                                }
                            })
                            .detach();

                            this.with_animation(
                                "progress-animation",
                                Animation::new(duration),
                                move |this, delta| {
                                    let current_value = from + (value - from) * delta;
                                    let w = relative((current_value / 100.).clamp(0., 1.));
                                    this.w(w)
                                },
                            )
                            .into_any_element()
                        } else if loading {
                            this.with_animation(
                                "progress-loading",
                                Animation::new(Duration::from_secs(1)).repeat(),
                                move |this, delta| {
                                    let start =
                                        relative(ease_in_out(((delta - 0.5) / 0.5).clamp(0., 1.)));
                                    let end = relative(ease_in_out(1.0 - delta));
                                    this.when(delta > 0.5, |this| this.left(start)).right(end)
                                },
                            )
                            .into_any_element()
                        } else {
                            this.w(relative((value / 100.).clamp(0., 1.)))
                                .into_any_element()
                        }
                    }),
            )
    }
}
