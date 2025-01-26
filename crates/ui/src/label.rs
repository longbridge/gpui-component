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
    align: TextAlign,
    marked: bool,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            base: h_flex().line_height(rems(1.25)),
            label: label.into(),
            align: TextAlign::default(),
            marked: false,
        }
    }

    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn text_left(mut self) -> Self {
        self.align = TextAlign::Left;
        self
    }

    pub fn text_center(mut self) -> Self {
        self.align = TextAlign::Center;
        self
    }

    pub fn text_right(mut self) -> Self {
        self.align = TextAlign::Right;
        self
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
        let text = self.label;

        let text_display = if self.marked {
            MASKED.repeat(text.chars().count())
        } else {
            text.to_string()
        };

        div().text_color(cx.theme().foreground).child(
            self.base
                .map(|this| match self.align {
                    TextAlign::Left => this.justify_start(),
                    TextAlign::Center => this.justify_center(),
                    TextAlign::Right => this.justify_end(),
                })
                .map(|this| {
                    if self.align == TextAlign::Left {
                        this.child(div().size_full().child(text_display))
                    } else {
                        this.child(text_display)
                    }
                }),
        )
    }
}
