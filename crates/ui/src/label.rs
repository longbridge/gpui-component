use gpui::{
    div, rems, App, Div, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::ActiveTheme;

const MASKED: &'static str = "•";

#[derive(IntoElement)]
pub struct Label {
    base: Div,
    label: SharedString,
    chars_count: usize,
    marked: bool,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        let chars_count = label.chars().count();
        Self {
            base: div().line_height(rems(1.25)),
            label,
            chars_count,
            marked: false,
        }
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.marked = masked;
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = if self.marked {
            SharedString::from(MASKED.repeat(self.chars_count))
        } else {
            self.label
        };

        div()
            .text_color(cx.theme().foreground)
            .child(self.base.child(text))
    }
}
