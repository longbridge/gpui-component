use crate::ActiveTheme;
use gpui::{
    div, prelude::FluentBuilder, px, relative, App, IntoElement, ParentElement, RenderOnce, Styled,
    Window,
};

/// A Progress bar element.
#[derive(IntoElement)]
pub struct Progress {
    value: f32,
    height: f32,
}

impl Progress {
    pub fn new() -> Self {
        Progress {
            value: Default::default(),
            height: 8.,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }
}

impl RenderOnce for Progress {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // Match the theme radius, if theme radius is zero use it.
        let radius = px(self.height / 2.).min(cx.theme().radius);
        let relative_w = relative(match self.value {
            v if v < 0. => 0.,
            v if v > 100. => 1.,
            v => v / 100.,
        });

        div()
            .relative()
            .h(px(self.height))
            .rounded(radius)
            .bg(cx.theme().progress_bar.opacity(0.2))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(relative_w)
                    .bg(cx.theme().progress_bar)
                    .map(|this| match self.value {
                        v if v >= 100. => this.rounded(radius),
                        _ => this.rounded_l(radius),
                    }),
            )
    }
}
