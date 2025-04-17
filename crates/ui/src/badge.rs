use gpui::{
    div, prelude::FluentBuilder, px, relative, AnyElement, App, Div, IntoElement, ParentElement,
    RenderOnce, Styled, Window,
};

use crate::{h_flex, red_500, white};

#[derive(Default)]
enum BadgeStyle {
    Dot,
    #[default]
    Number,
}

#[derive(IntoElement)]
pub struct Badge {
    base: Div,
    count: usize,
    max: usize,
    style: BadgeStyle,
}

impl Badge {
    pub fn new() -> Self {
        Self {
            base: div(),
            count: 0,
            max: 99,
            style: Default::default(),
        }
    }

    pub fn dot(mut self) -> Self {
        self.style = BadgeStyle::Dot;
        self
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }
}

impl ParentElement for Badge {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.base.relative().when(self.count > 0, |this| {
            this.child(
                h_flex()
                    .absolute()
                    .justify_center()
                    .rounded_full()
                    .bg(red_500())
                    .map(|this| match self.style {
                        BadgeStyle::Dot => this.top(px(0.)).right(px(0.)).size(px(6.)),
                        BadgeStyle::Number => {
                            let count = if self.count > self.max {
                                format!("{}+", self.max)
                            } else {
                                self.count.to_string()
                            };

                            this.top(px(-3.))
                                .right(-px(3. * count.len() as f32))
                                .py_0p5()
                                .px_0p5()
                                .min_w_3p5()
                                .text_color(white())
                                .text_size(px(10.))
                                .line_height(relative(1.))
                                .child(count)
                        }
                    }),
            )
        })
    }
}
