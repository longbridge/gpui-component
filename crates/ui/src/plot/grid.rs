use gpui::{px, Bounds, Hsla, PathBuilder, Pixels, Point, Window};

use super::{dash_line, origin_point};

pub enum StrokeStyle {
    Solid(Hsla),
    Dashed(Hsla, [Pixels; 2]),
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self::Solid(Default::default())
    }
}

pub struct Grid {
    x: Vec<Pixels>,
    y: Vec<Pixels>,
    stroke: StrokeStyle,
}

impl Grid {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            x: vec![],
            y: vec![],
            stroke: Default::default(),
        }
    }

    pub fn x(mut self, x: Vec<impl Into<Pixels>>) -> Self {
        self.x = x.into_iter().map(|v| v.into()).collect();
        self
    }

    pub fn y(mut self, y: Vec<impl Into<Pixels>>) -> Self {
        self.y = y.into_iter().map(|v| v.into()).collect();
        self
    }

    pub fn stroke(mut self, stroke: StrokeStyle) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn solid(mut self, stroke: impl Into<Hsla>) -> Self {
        self.stroke = StrokeStyle::Solid(stroke.into());
        self
    }

    pub fn dashed<T: Into<Pixels>>(mut self, stroke: impl Into<Hsla>, dash_array: [T; 2]) -> Self {
        self.stroke = StrokeStyle::Dashed(stroke.into(), dash_array.map(|v| v.into()));
        self
    }

    fn points(&self, bounds: &Bounds<Pixels>) -> Vec<(Point<Pixels>, Point<Pixels>)> {
        let size = bounds.size;
        let origin = bounds.origin;

        let mut x = self
            .x
            .iter()
            .map(|x| {
                (
                    origin_point(*x, px(0.), origin),
                    origin_point(*x, size.height, origin),
                )
            })
            .collect::<Vec<_>>();

        let y = self
            .y
            .iter()
            .map(|y| {
                (
                    origin_point(px(0.), *y, origin),
                    origin_point(size.width, *y, origin),
                )
            })
            .collect::<Vec<_>>();

        x.extend(y);
        x
    }

    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window) {
        let points = self.points(bounds);

        match self.stroke {
            StrokeStyle::Solid(stroke) => {
                for (start, end) in points {
                    let mut builder = PathBuilder::stroke(px(1.));
                    builder.move_to(start);
                    builder.line_to(end);
                    if let Ok(line) = builder.build() {
                        window.paint_path(line, stroke);
                    }
                }
            }
            StrokeStyle::Dashed(color, dash_array) => {
                for (start, end) in points {
                    if let Some(line) = dash_line(start, end, dash_array) {
                        window.paint_path(line, color);
                    }
                }
            }
        }
    }
}
