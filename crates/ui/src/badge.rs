use gpui::{
    div, prelude::FluentBuilder, px, relative, AnyElement, App, Hsla, IntoElement, ParentElement,
    RenderOnce, StyleRefinement, Styled, Window,
};

use crate::{h_flex, red_500, white, StyledExt};

#[derive(Default)]
enum BadgeVariant {
    Dot,
    #[default]
    Number,
}

#[derive(IntoElement)]
pub struct Badge {
    style: StyleRefinement,
    count: usize,
    max: usize,
    variant: BadgeVariant,
    children: Vec<AnyElement>,
    color: Hsla,
}

impl Badge {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            count: 0,
            max: 99,
            variant: Default::default(),
            color: red_500(),
            children: Vec::new(),
        }
    }

    pub fn dot(mut self) -> Self {
        self.variant = BadgeVariant::Dot;
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

    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = color.into();
        self
    }
}

impl ParentElement for Badge {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .relative()
            .refine_style(&self.style)
            .children(self.children)
            .when(self.count > 0, |this| {
                this.child(
                    h_flex()
                        .absolute()
                        .justify_center()
                        .rounded_full()
                        .bg(self.color)
                        .map(|this| match self.variant {
                            BadgeVariant::Dot => this.top_0().right_0().size(px(6.)),
                            BadgeVariant::Number => {
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
