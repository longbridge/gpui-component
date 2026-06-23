use gpui::{
    AnyElement, App, Div, Hsla, IntoElement, ParentElement, Pixels, Point, RenderOnce,
    SharedString, Size, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{ActiveTheme, StyledExt, h_flex, v_flex};

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
    /// Start offset along the cross axis (vertical line: y; horizontal line: x).
    start: f32,
    /// Length along the cross axis; `None` spans the full extent.
    length: Option<f32>,
    /// Band thickness perpendicular to the line (solid band mode only).
    thickness: Pixels,
    /// Explicit color; when `None`, falls back to a themed default (a subtle `border` for
    /// the dashed hairline, a translucent highlight for the solid band).
    color: Option<Hsla>,
    /// `true` (default) draws a dashed hairline; `false` a solid band of `thickness`.
    dashed: bool,
    direction: CrossLineAxis,
}

impl CrossLine {
    pub fn new(point: Point<Pixels>) -> Self {
        Self {
            point,
            start: 0.,
            length: None,
            thickness: px(1.),
            color: None,
            dashed: true,
            direction: Default::default(),
        }
    }

    /// Render a solid highlight band of `thickness` (centered on `point`) instead of the
    /// default dashed hairline. Use the bar/band width to highlight the hovered column or
    /// row. The fill defaults to a translucent highlight; use [`CrossLine::color`] to override.
    pub fn band(mut self, thickness: impl Into<Pixels>) -> Self {
        self.thickness = thickness.into();
        self.dashed = false;
        self
    }

    /// Override the line/band color. Defaults: a subtle `border` for the dashed hairline,
    /// a translucent highlight for the solid band.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
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

    /// Set the length of the cross line along its axis (from the start edge).
    pub fn height(mut self, height: f32) -> Self {
        self.length = Some(height);
        self
    }

    /// Confine the cross line to `[start, start + length]` along its axis (vertical
    /// line: y; horizontal line: x), so it stays within the plot area.
    pub fn span(mut self, start: f32, length: f32) -> Self {
        self.start = start;
        self.length = Some(length);
        self
    }
}

impl From<Point<Pixels>> for CrossLine {
    fn from(value: Point<Pixels>) -> Self {
        Self::new(value)
    }
}

impl CrossLine {
    /// Build a single line along one axis: `vertical` runs top→bottom at the data point's
    /// `x`; otherwise left→right at its `y`. A dashed hairline draws a 1px dashed border; a
    /// solid band fills a `thickness`-wide strip centered on the data point.
    fn line(&self, vertical: bool, cx: &App) -> Div {
        let color = self.color.unwrap_or_else(|| {
            if self.dashed {
                cx.theme().border
            } else {
                cx.theme().foreground.opacity(0.08)
            }
        });
        // The dashed hairline is a zero-width strip drawn entirely by its 1px border.
        let thickness = if self.dashed { px(0.) } else { self.thickness };

        let el = div().absolute();
        let el = if vertical {
            el.left(self.point.x - thickness * 0.5)
                .w(thickness)
                .top(px(self.start))
                .map(|el| match self.length {
                    Some(length) => el.h(px(length)),
                    None => el.h_full(),
                })
        } else {
            el.top(self.point.y - thickness * 0.5)
                .h(thickness)
                .left(px(self.start))
                .map(|el| match self.length {
                    Some(length) => el.w(px(length)),
                    None => el.w_full(),
                })
        };

        if self.dashed {
            let el = if vertical {
                el.border_l_1()
            } else {
                el.border_t_1()
            };
            el.border_dashed().border_color(color)
        } else {
            el.bg(color)
        }
    }
}

impl RenderOnce for CrossLine {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let vertical = self.direction.show_vertical().then(|| self.line(true, cx));
        let horizontal = self
            .direction
            .show_horizontal()
            .then(|| self.line(false, cx));

        div()
            .size_full()
            .absolute()
            .top_0()
            .left_0()
            .children(vertical)
            .children(horizontal)
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

impl TooltipPosition {
    /// Place the tooltip on the right while the hovered `index` is in the first half of
    /// `len` items, otherwise on the left, so it stays clear of the screen edge.
    pub fn for_index(index: usize, len: usize) -> Self {
        if index < len / 2 {
            Self::Right
        } else {
            Self::Left
        }
    }
}

/// The axis the data points are laid out along; the follow animation slides along it.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TooltipAxis {
    /// Data points run along the x-axis (line, area, vertical bars). The default.
    #[default]
    X,
    /// Data points run along the y-axis (horizontal bars).
    Y,
}

#[derive(Clone)]
pub struct TooltipState {
    pub index: usize,
    pub cross_line: Point<Pixels>,
    pub dots: Vec<Point<Pixels>>,
    pub position: TooltipPosition,
    /// The axis the data points snap along. Describes how `cross_line` is read: the
    /// coordinate on this axis is the snapped (data-point) one the follow animation eases,
    /// while the other tracks the cursor live.
    pub axis: TooltipAxis,
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
            axis: TooltipAxis::X,
        }
    }

    /// Set the axis the data points snap along (default [`TooltipAxis::X`]).
    pub fn axis(mut self, axis: TooltipAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Whether the follow animation slides vertically (i.e. the snap axis is y).
    pub fn slides_vertically(&self) -> bool {
        matches!(self.axis, TooltipAxis::Y)
    }
}

/// A single labelled row in a [`Tooltip`]: a colored swatch, a muted label, and a value.
struct TooltipRow {
    color: Hsla,
    label: SharedString,
    value: SharedString,
}

#[derive(IntoElement)]
pub struct Tooltip {
    base: Div,
    position: Option<TooltipPosition>,
    gap: Pixels,
    cross_line: Option<CrossLine>,
    dots: Option<Vec<Dot>>,
    appearance: bool,
    title: Option<SharedString>,
    rows: Vec<TooltipRow>,
    /// When set, the tooltip follows the cursor: `(anchor point, container size)`.
    /// The box hugs the anchor and flips toward the center near each edge.
    anchor: Option<(Point<Pixels>, Size<Pixels>)>,
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
            title: None,
            rows: Vec::new(),
            anchor: None,
        }
    }

    /// Make the tooltip follow the cursor, anchored at `point` within a `within`-sized plot.
    ///
    /// The box hugs the anchor and flips toward the center near each edge (so it never
    /// overflows the near side). Without this, the tooltip is pinned to the plot edge via
    /// [`Tooltip::position`].
    pub fn anchor(mut self, point: Point<Pixels>, within: Size<Pixels>) -> Self {
        self.anchor = Some((point, within));
        self
    }

    /// Set a bold title row shown at the top of the tooltip (e.g. the hovered x value).
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Append a series row: a colored swatch, a muted `label`, and a right-aligned `value`.
    pub fn row(
        mut self,
        color: impl Into<Hsla>,
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        self.rows.push(TooltipRow {
            color: color.into(),
            label: label.into(),
            value: value.into(),
        });
        self
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
        let Tooltip {
            base,
            position,
            gap,
            cross_line,
            dots,
            appearance,
            title,
            rows,
            anchor,
        } = self;

        // Structured content (title + rows) takes precedence over freeform `base` children.
        let content = if title.is_some() || !rows.is_empty() {
            v_flex()
                .text_sm()
                .gap_1()
                .when_some(title, |this, title| {
                    this.child(div().font_semibold().child(title))
                })
                .children(rows.into_iter().map(|row| {
                    h_flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_1p5()
                                .child(div().size_2().rounded_sm().bg(row.color))
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(row.label),
                                ),
                        )
                        .child(div().child(row.value))
                }))
        } else {
            base
        };

        div()
            .size_full()
            .absolute()
            .top_0()
            .left_0()
            .when_some(cross_line, |this, cross_line| this.child(cross_line))
            .when_some(dots, |this, dots| this.children(dots))
            .child(content.map(|this| {
                if !appearance {
                    return this.size_full().relative();
                }

                let card = this.absolute().min_w(px(150.)).popover_style(cx).p_2();

                match anchor {
                    // Follow mode: hug the anchor, flipping toward the center near each edge.
                    Some((p, within)) => card
                        .map(|c| {
                            if p.x < within.width * 0.5 {
                                c.left(p.x + gap)
                            } else {
                                c.right(within.width - p.x + gap)
                            }
                        })
                        .map(|c| {
                            if p.y < within.height * 0.5 {
                                c.top(p.y + gap)
                            } else {
                                c.bottom(within.height - p.y + gap)
                            }
                        }),
                    // Edge-pinned mode (backward compatible default).
                    None => card.when_some(position, |c, position| {
                        if position == TooltipPosition::Left {
                            c.left(gap)
                        } else {
                            c.right(gap)
                        }
                    }),
                }
            }))
    }
}
