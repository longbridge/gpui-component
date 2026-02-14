use gpui::{
    AnyElement, App, Div, Half as _, Hsla, IntoElement, ParentElement, Pixels, Point, RenderOnce,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{ActiveTheme, v_flex};

#[derive(Default)]
pub enum CrossLineAxis {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

impl CrossLineAxis {
    /// Returns true if the cross line axis is vertical or both.
    #[inline]
    pub fn show_vertical(&self) -> bool {
        matches!(self, CrossLineAxis::Vertical | CrossLineAxis::Both)
    }

    /// Returns true if the cross line axis is horizontal or both.
    #[inline]
    pub fn show_horizontal(&self) -> bool {
        matches!(self, CrossLineAxis::Horizontal | CrossLineAxis::Both)
    }
}

#[derive(IntoElement)]
pub struct CrossLine {
    point: Point<Pixels>,
    height: Option<f32>,
    direction: CrossLineAxis,
}

impl CrossLine {
    pub fn new(point: Point<Pixels>) -> Self {
        Self {
            point,
            height: None,
            direction: Default::default(),
        }
    }

    /// Set the cross line axis to horizontal.
    pub fn horizontal(mut self) -> Self {
        self.direction = CrossLineAxis::Horizontal;
        self
    }

    /// Set the cross line axis to both.
    pub fn both(mut self) -> Self {
        self.direction = CrossLineAxis::Both;
        self
    }

    /// Set the height of the cross line.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
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
            .when(self.direction.show_vertical(), |this| {
                this.child(
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
            })
            .when(self.direction.show_horizontal(), |this| {
                this.child(
                    div()
                        .absolute()
                        .w_full()
                        .h(px(1.))
                        .bg(cx.theme().border)
                        .left_0()
                        .top(self.point.y),
                )
            })
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

    /// Set the size of the dot.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// Set the stroke of the dot.
    pub fn stroke(mut self, stroke: Hsla) -> Self {
        self.stroke = stroke;
        self
    }

    /// Set the fill of the dot.
    pub fn fill(mut self, fill: Hsla) -> Self {
        self.fill = fill;
        self
    }
}

impl RenderOnce for Dot {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let border_width = px(1.);
        let offset = self.size / 2. - border_width / 2.;

        div()
            .absolute()
            .w(self.size)
            .h(self.size)
            .rounded_full()
            .border(border_width)
            .border_color(self.stroke)
            .bg(self.fill)
            .left(self.point.x - offset)
            .top(self.point.y - offset)
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
    pub dots: Vec<Point<Pixels>>,
    pub position: TooltipPosition,
}

impl TooltipState {
    pub fn new(
        index: usize,
        cross_line: Point<Pixels>,
        dots: Vec<Point<Pixels>>,
        position: TooltipPosition,
    ) -> Self {
        Self {
            index,
            cross_line,
            dots,
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
    dots: Option<Vec<Dot>>,
    appearance: bool,
}

impl Tooltip {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            base: v_flex().top_0(),
            position: Default::default(),
            gap: px(0.),
            cross_line: None,
            dots: None,
            appearance: true,
        }
    }

    /// Set the position of the tooltip.
    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = Some(position);
        self
    }

    /// Set the gap of the tooltip.
    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into();
        self
    }

    /// Set the cross line of the tooltip.
    pub fn cross_line(mut self, cross_line: CrossLine) -> Self {
        self.cross_line = Some(cross_line);
        self
    }

    /// Set the dots of the tooltip.
    pub fn dots(mut self, dots: impl IntoIterator<Item = Dot>) -> Self {
        self.dots = Some(dots.into_iter().collect());
        self
    }

    /// Set the appearance of the tooltip.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
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
            .when_some(self.dots, |this, dots| this.children(dots))
            .child(self.base.map(|this| {
                if self.appearance {
                    this.absolute()
                        .min_w(px(168.))
                        .p_2()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius.half())
                        .bg(cx.theme().background.opacity(0.9))
                        .when_some(self.position, |this, position| {
                            if position == TooltipPosition::Left {
                                this.left(self.gap)
                            } else {
                                this.right(self.gap)
                            }
                        })
                } else {
                    this.size_full().relative()
                }
            }))
    }
}
