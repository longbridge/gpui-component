use gpui::{
    div, prelude::FluentBuilder, rems, App, Div, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};

use crate::{h_flex, ActiveTheme};

const MASKED: &'static str = "•";

#[derive(Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(IntoElement)]
pub struct Label {
    base: Div,
    label: SharedString,
    chars_count: usize,
    align: TextAlign,
    marked: bool,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        let chars_count = label.chars().count();
        Self {
            base: h_flex().line_height(rems(1.25)),
            label,
            chars_count,
            align: TextAlign::default(),
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
            .child(self.base.map(|this| {
                if self.align == TextAlign::Left {
                    this.child(div().size_full().child(text))
                } else {
                    this.child(text)
                }
            }))
    }
}
