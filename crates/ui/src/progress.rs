use crate::ActiveTheme;
use gpui::{Window, AppContext, 
    div, prelude::FluentBuilder, px, relative, IntoElement, ParentElement, RenderOnce, Styled,
    
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rounded = px(self.height / 2.);
        let relative_w = relative(match self.value {
            v if v < 0. => 0.,
            v if v > 100. => 1.,
            v => v / 100.,
        });

        div()
            .relative()
            .h(px(self.height))
            .rounded(rounded)
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
                        v if v >= 100. => this.rounded(rounded),
                        _ => this.rounded_l(rounded),
                    }),
            )
    }
}
