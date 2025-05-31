use gpui::{px, Bounds, Hsla, PathBuilder, Pixels, Point, Window};

use super::{dash_line, origin_point};

pub struct Grid {
    x: Vec<Pixels>,
    y: Vec<Pixels>,
    stroke: Hsla,
    dash_array: Option<[Pixels; 2]>,
}

impl Grid {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            x: vec![],
            y: vec![],
            stroke: Default::default(),
            dash_array: None,
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

    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.stroke = stroke.into();
        self
    }

    pub fn dash_array(mut self, dash_array: [Pixels; 2]) -> Self {
        self.dash_array = Some(dash_array);
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

        if let Some(dash_array) = self.dash_array {
            for (start, end) in points {
                if let Some(line) = dash_line(start, end, dash_array) {
                    window.paint_path(line, self.stroke);
                }
            }
        } else {
            for (start, end) in points {
                let mut builder = PathBuilder::stroke(px(1.));
                builder.move_to(start);
                builder.line_to(end);
                if let Ok(line) = builder.build() {
                    window.paint_path(line, self.stroke);
                }
            }
        }
    }
}
