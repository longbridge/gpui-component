use gpui::{
    point, px, App, Bounds, FontWeight, Hsla, PathBuilder, Pixels, Point, SharedString, TextAlign,
    Window,
};

use super::{label::Label, label::Text, label::TEXT_GAP, label::TEXT_SIZE, origin_point};

pub struct AxisText {
    pub text: SharedString,
    pub tick: Pixels,
    pub color: Hsla,
    pub font_size: Pixels,
    pub align: TextAlign,
}

impl AxisText {
    pub fn new(text: SharedString, tick: impl Into<Pixels>, color: Hsla) -> Self {
        Self {
            text,
            tick: tick.into(),
            color,
            font_size: TEXT_SIZE.into(),
            align: TextAlign::Left,
        }
    }

    pub fn font_size(mut self, font_size: impl Into<Pixels>) -> Self {
        self.font_size = font_size.into();
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }
}

#[derive(Default)]
pub struct Axis {
    x: Option<Pixels>,
    x_label: Label,
    show_x_axis: bool,
    y: Option<Pixels>,
    y_label: Label,
    show_y_axis: bool,
    stroke: Hsla,
}

impl Axis {
    pub fn new() -> Self {
        Self {
            show_x_axis: true,
            ..Default::default()
        }
    }

    pub fn x(mut self, x: impl Into<Pixels>) -> Self {
        self.x = Some(x.into());
        self
    }

    pub fn hide_x_axis(mut self) -> Self {
        self.show_x_axis = false;
        self
    }

    pub fn x_label(mut self, label: impl IntoIterator<Item = AxisText>) -> Self {
        if let Some(x) = self.x {
            self.x_label = label
                .into_iter()
                .map(|t| Text {
                    text: t.text,
                    origin: point(t.tick, x + px(TEXT_GAP as f32)),
                    color: t.color,
                    font_size: t.font_size,
                    font_weight: FontWeight::NORMAL,
                    align: t.align,
                })
                .into();
        }
        self
    }

    pub fn y(mut self, y: impl Into<Pixels>) -> Self {
        self.y = Some(y.into());
        self
    }

    pub fn hide_y_axis(mut self) -> Self {
        self.show_y_axis = false;
        self
    }

    pub fn y_label(mut self, label: impl IntoIterator<Item = AxisText>) -> Self {
        if let Some(y) = self.y {
            self.y_label = label
                .into_iter()
                .map(|t| Text {
                    text: t.text,
                    origin: point(y + px(TEXT_GAP as f32), t.tick),
                    color: t.color,
                    font_size: t.font_size,
                    font_weight: FontWeight::NORMAL,
                    align: t.align,
                })
                .into();
        }
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.stroke = stroke.into();
        self
    }

    fn draw_axis(&self, start_point: Point<Pixels>, end_point: Point<Pixels>, window: &mut Window) {
        let mut builder = PathBuilder::stroke(px(1.));
        builder.move_to(start_point);
        builder.line_to(end_point);
        if let Ok(path) = builder.build() {
            window.paint_path(path, self.stroke);
        }
    }

    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let origin = bounds.origin;

        // X axis
        if let Some(x) = self.x {
            if self.show_x_axis {
                self.draw_axis(
                    origin_point(px(0.), x, origin),
                    origin_point(bounds.size.width, x, origin),
                    window,
                );
            }
        }
        self.x_label.paint(bounds, window, cx);

        // Y axis
        if let Some(y) = self.y {
            if self.show_y_axis {
                self.draw_axis(
                    origin_point(y, px(0.), origin),
                    origin_point(y, bounds.size.height, origin),
                    window,
                );
            }
        }
        self.y_label.paint(bounds, window, cx);
    }
}
