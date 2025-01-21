use crate::ActiveTheme;
use gpui::{
    bounce, div, ease_in_out, Animation, AnimationExt, Div, IntoElement, RenderOnce, Styled,
};
use std::time::Duration;

#[derive(IntoElement)]
pub struct Skeleton {
    base: Div,
    secondary: bool,
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            base: div().w_full().h_4().rounded_md(),
            secondary: false,
        }
    }

    /// Set use secondary color.
    pub fn secondary(mut self, secondary: bool) -> Self {
        self.secondary = secondary;
        self
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Skeleton {
    fn render(self, cx: &mut gpui::WindowContext) -> impl IntoElement {
        let color = if self.secondary {
            cx.theme().skeleton.opacity(0.5)
        } else {
            cx.theme().skeleton
        };

        self.base.bg(color).with_animation(
            "skeleton",
            Animation::new(Duration::from_secs(2))
                .repeat()
                .with_easing(bounce(ease_in_out)),
            move |this, delta| {
                let v = 1.0 - delta * 0.5;
                this.opacity(v)
            },
        )
    }
}
