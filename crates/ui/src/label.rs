use gpui::{
    div, rems, App, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window,
};

use crate::{ActiveTheme, StyledExt};

const MASKED: &'static str = "•";

#[derive(IntoElement)]
pub struct Label {
    style: StyleRefinement,
    label: SharedString,
    chars_count: usize,
    masked: bool,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        let chars_count = label.chars().count();
        Self {
            style: Default::default(),
            label,
            chars_count,
            masked: false,
        }
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = if self.masked {
            SharedString::from(MASKED.repeat(self.chars_count))
        } else {
            self.label
        };

        div()
            .line_height(rems(1.25))
            .text_color(cx.theme().foreground)
            .refine_style(&self.style)
            .child(text)
    }
}
