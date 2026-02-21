use gpui::{prelude::*, *};
use gpui_component::PixelsExt as _;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ThumbShape {
    #[default]
    Circle,
    Square,
    Bar,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ThumbAxis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum TrackEndcaps {
    #[default]
    Rounded,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbLayoutHint {
    pub supported_positions: &'static [super::slider::ThumbPosition],
    pub preferred_position: super::slider::ThumbPosition,
    pub preferred_track_endcaps: TrackEndcaps,
}

impl ThumbShape {
    pub fn layout_hint(self) -> ThumbLayoutHint {
        use super::slider::ThumbPosition;
        const ALL_POSITIONS: &[ThumbPosition] =
            &[ThumbPosition::InsideSlider, ThumbPosition::EdgeToEdge];
        const INSIDE_ONLY: &[ThumbPosition] = &[ThumbPosition::InsideSlider];

        match self {
            ThumbShape::Circle | ThumbShape::Square => ThumbLayoutHint {
                supported_positions: ALL_POSITIONS,
                preferred_position: ThumbPosition::InsideSlider,
                preferred_track_endcaps: TrackEndcaps::Rounded,
            },
            ThumbShape::Bar => ThumbLayoutHint {
                supported_positions: INSIDE_ONLY,
                preferred_position: ThumbPosition::InsideSlider,
                preferred_track_endcaps: TrackEndcaps::Rounded,
            },
        }
    }
}

pub(crate) fn bar_main_axis_size(thumb_size: Pixels) -> Pixels {
    // Bar width should be roughly 30% of thumb size
    px((thumb_size.as_f32() * 0.3).max(4.0))
}

#[derive(IntoElement)]
pub struct ColorThumb {
    style: ThumbStyle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbStyle {
    pub size: Pixels,
    pub color: Option<Hsla>,
    pub shape: ThumbShape,
    pub axis: ThumbAxis,
    pub active: bool,
    pub border_outer: Hsla,
    pub border_inner: Hsla,
    pub border_width: Pixels,
    pub inner_inset: Pixels,
    pub active_shadow: bool,
    pub show_inner_border: bool,
    pub show_outer_border: bool,
}

impl ThumbStyle {
    pub fn new(size: impl Into<Pixels>) -> Self {
        Self {
            size: size.into(),
            color: None,
            shape: ThumbShape::default(),
            axis: ThumbAxis::default(),
            active: false,
            border_outer: black(),
            border_inner: white(),
            border_width: px(1.0),
            inner_inset: px(2.0),
            active_shadow: true,
            show_inner_border: true,
            show_outer_border: true,
        }
    }

    fn metrics(self) -> ThumbMetrics {
        let thumb_inner_size = self.size - self.inner_inset;
        let bar_outer_main = bar_main_axis_size(self.size);
        let bar_inner_main = bar_outer_main - self.inner_inset;
        let thumb_inner_cross = self.size - self.inner_inset;

        ThumbMetrics {
            thumb_inner_size,
            bar_outer_main,
            bar_inner_main,
            thumb_inner_cross,
        }
    }
}

impl Default for ThumbStyle {
    fn default() -> Self {
        Self::new(px(0.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ThumbMetrics {
    thumb_inner_size: Pixels,
    bar_outer_main: Pixels,
    bar_inner_main: Pixels,
    thumb_inner_cross: Pixels,
}

impl ColorThumb {
    pub fn new(size: impl Into<Pixels>) -> Self {
        Self {
            style: ThumbStyle::new(size),
        }
    }

    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.style.color = Some(color.into());
        self
    }

    pub fn shape(mut self, shape: ThumbShape) -> Self {
        self.style.shape = shape;
        self
    }

    pub fn axis(mut self, axis: ThumbAxis) -> Self {
        self.style.axis = axis;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.style.active = active;
        self
    }

    #[allow(dead_code)] // Kept for future thumb-style experiments in stories.
    pub fn inner_border(mut self, show: bool) -> Self {
        self.style.show_inner_border = show;
        self
    }

    #[allow(dead_code)] // Kept for future thumb-style experiments in stories.
    pub fn outer_border(mut self, show: bool) -> Self {
        self.style.show_outer_border = show;
        self
    }

    fn render_bar(style: ThumbStyle, metrics: ThumbMetrics) -> Div {
        div()
            .when(style.axis == ThumbAxis::Vertical, |this| {
                this.w(style.size).h(metrics.bar_outer_main)
            })
            .when(style.axis == ThumbAxis::Horizontal, |this| {
                this.w(metrics.bar_outer_main).h(style.size)
            })
            .child(
                div()
                    .when(style.axis == ThumbAxis::Vertical, |this| {
                        this.w(style.size).h(metrics.bar_outer_main)
                    })
                    .when(style.axis == ThumbAxis::Horizontal, |this| {
                        this.w(metrics.bar_outer_main).h(style.size)
                    })
                    .when(style.show_outer_border, |this| {
                        this.border(style.border_width)
                            .border_color(style.border_outer)
                    })
                    .when_some(style.color, |this, color| this.bg(color))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .when(style.axis == ThumbAxis::Vertical, |this| {
                                this.w(metrics.thumb_inner_cross).h(metrics.bar_inner_main)
                            })
                            .when(style.axis == ThumbAxis::Horizontal, |this| {
                                this.w(metrics.bar_inner_main).h(metrics.thumb_inner_cross)
                            })
                            .when(style.show_inner_border, |this| {
                                this.border(style.border_width)
                                    .border_color(style.border_inner)
                            })
                            .when_some(style.color, |this, color| this.bg(color))
                            .when(style.active && style.active_shadow, |this| this.shadow_md()),
                    ),
            )
    }

    fn render_circle_or_square(style: ThumbStyle, metrics: ThumbMetrics) -> Div {
        let is_circle = style.shape == ThumbShape::Circle;

        div()
            .size(style.size)
            .map(|this| if is_circle { this.rounded_full() } else { this })
            .when(style.show_outer_border, |this| {
                this.border(style.border_width)
                    .border_color(style.border_outer)
            })
            .when_some(style.color, |this, color| this.bg(color))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .size(metrics.thumb_inner_size)
                    .map(|this| if is_circle { this.rounded_full() } else { this })
                    .when(style.show_inner_border, |this| {
                        this.border(style.border_width)
                            .border_color(style.border_inner)
                    })
                    .when_some(style.color, |this, color| this.bg(color))
                    .when(style.active && style.active_shadow, |this| this.shadow_md()),
            )
    }
}

impl RenderOnce for ColorThumb {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        let style = self.style;
        let metrics = style.metrics();

        match style.shape {
            ThumbShape::Bar => Self::render_bar(style, metrics),
            ThumbShape::Circle | ThumbShape::Square => {
                Self::render_circle_or_square(style, metrics)
            }
        }
    }
}
