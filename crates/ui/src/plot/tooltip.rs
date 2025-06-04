use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, Div, Hsla, IntoElement, ParentElement,
    Pixels, Point, RenderOnce, StyleRefinement, Styled, Window,
};

use crate::{v_flex, ActiveTheme};

#[derive(IntoElement)]
pub struct CrossLine {
    point: Point<Pixels>,
    height: Option<f32>,
}

impl CrossLine {
    pub fn new(point: Point<Pixels>) -> Self {
        Self {
            point,
            height: None,
        }
    }

    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height as f32);
        self
    }
}

impl From<Point<Pixels>> for CrossLine {
    fn from(value: Point<Pixels>) -> Self {
        Self::new(value)
    }
}

impl RenderOnce for CrossLine {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .absolute()
            .top_0()
            .left_0()
            .child(
                div()
                    .absolute()
                    .w(px(1.))
                    .bg(cx.theme().border)
                    .top_0()
                    .left(self.point.x)
                    .map(|this| {
                        if let Some(height) = self.height {
                            this.h(px(height))
                        } else {
                            this.h_full()
                        }
                    }),
            )
            .child(
                div()
                    .absolute()
                    .w_full()
                    .h(px(1.))
                    .bg(cx.theme().border)
                    .left_0()
                    .top(self.point.y),
            )
    }
}

#[derive(IntoElement)]
pub struct Dot {
    point: Point<Pixels>,
    size: Pixels,
    stroke: Hsla,
    fill: Hsla,
}

impl Dot {
    pub fn new(point: Point<Pixels>) -> Self {
        Self {
            point,
            size: px(6.),
            stroke: gpui::transparent_black(),
            fill: gpui::transparent_black(),
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    pub fn stroke(mut self, stroke: Hsla) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn fill(mut self, fill: Hsla) -> Self {
        self.fill = fill;
        self
    }
}

impl RenderOnce for Dot {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .absolute()
            .w(self.size)
            .h(self.size)
            .rounded_full()
            .border_1()
            .border_color(self.stroke)
            .bg(self.fill)
            .left(self.point.x)
            .top(self.point.y - self.size / 2.)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TooltipPosition {
    #[default]
    Left,
    Right,
}

#[derive(Clone)]
pub struct TooltipState {
    pub index: usize,
    pub cross_line: Point<Pixels>,
    pub dot: Point<Pixels>,
    pub position: TooltipPosition,
}

impl TooltipState {
    pub fn new(
        index: usize,
        cross_line: Point<Pixels>,
        dot: Point<Pixels>,
        position: TooltipPosition,
    ) -> Self {
        Self {
            index,
            cross_line,
            dot,
            position,
        }
    }
}

#[derive(IntoElement)]
pub struct Tooltip {
    base: Div,
    position: Option<TooltipPosition>,
    gap: Pixels,
    cross_line: Option<CrossLine>,
    dot: Option<Dot>,
}

impl Tooltip {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            base: v_flex().top_0(),
            position: Default::default(),
            gap: px(0.),
            cross_line: None,
            dot: None,
        }
    }

    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = Some(position);
        self
    }

    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into();
        self
    }

    pub fn cross_line(mut self, cross_line: CrossLine) -> Self {
        self.cross_line = Some(cross_line);
        self
    }

    pub fn dot(mut self, dot: Dot) -> Self {
        self.dot = Some(dot);
        self
    }
}

impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Tooltip {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Tooltip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .absolute()
            .top_0()
            .left_0()
            .when_some(self.cross_line, |this, cross_line| this.child(cross_line))
            .child(
                self.base
                    .absolute()
                    .min_w(px(168.))
                    .p_2()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_sm()
                    .bg(cx.theme().background.opacity(0.9))
                    .when_some(self.position, |this, position| {
                        if position == TooltipPosition::Left {
                            this.left(self.gap)
                        } else {
                            this.right(self.gap)
                        }
                    }),
            )
            .when_some(self.dot, |this, dot| this.child(dot))
    }
}
