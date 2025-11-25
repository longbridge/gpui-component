use crate::{ActiveTheme, StyledExt};
use gpui::{
    App, Hsla, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder, px, relative,
};

/// A Progress bar element.
#[derive(IntoElement)]
pub struct Progress {
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
}

impl Progress {
    /// Create a new Progress bar.
    pub fn new() -> Self {
        Progress {
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

impl RenderOnce for Progress {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let radius = self.style.corner_radii.clone();
        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii = radius;

        let color = self.color.unwrap_or(cx.theme().progress_bar);

        let relative_w = relative(match self.value {
            v if v < 0. => 0.,
            v if v > 100. => 1.,
            v => v / 100.,
        });

        div()
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
                    .w(relative_w)
                    .bg(color)
                    .map(|this| match self.value {
                        v if v >= 100. => this.refine_style(&inner_style),
                        _ => this.refine_style(&inner_style).rounded_r_none(),
                    }),
            )
    }
}
