// @reference: https://d3js.org/d3-shape/area

use gpui::{px, Background, Bounds, Path, PathBuilder, Pixels, Point, Window};

use crate::plot::{origin_point, StrokeStyle};

#[allow(clippy::type_complexity)]
pub struct Area<T> {
    data: Vec<T>,
    x: Box<dyn Fn(&T) -> Option<f64>>,
    y0: Option<f64>,
    y1: Box<dyn Fn(&T) -> Option<f64>>,
    fill: Background,
    stroke: Background,
    stroke_style: StrokeStyle,
}

impl<T> Default for Area<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            x: Box::new(|_| None),
            y0: None,
            y1: Box::new(|_| None),
            fill: Default::default(),
            stroke: Default::default(),
            stroke_style: Default::default(),
        }
    }
}

impl<T> Area<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data<I>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.data = data.into_iter().collect();
        self
    }

    pub fn x<F>(mut self, x: F) -> Self
    where
        F: Fn(&T) -> Option<f64> + 'static,
    {
        self.x = Box::new(x);
        self
    }

    pub fn y0(mut self, y0: f64) -> Self {
        self.y0 = Some(y0);
        self
    }

    pub fn y1<F>(mut self, y1: F) -> Self
    where
        F: Fn(&T) -> Option<f64> + 'static,
    {
        self.y1 = Box::new(y1);
        self
    }

    pub fn fill(mut self, fill: impl Into<Background>) -> Self {
        self.fill = fill.into();
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Background>) -> Self {
        self.stroke = stroke.into();
        self
    }

    pub fn stroke_style(mut self, stroke_style: StrokeStyle) -> Self {
        self.stroke_style = stroke_style;
        self
    }

    fn path(&self, bounds: &Bounds<Pixels>) -> (Option<Path<Pixels>>, Option<Path<Pixels>>) {
        let origin = bounds.origin;
        let mut area_builder = PathBuilder::fill();
        let mut line_builder = PathBuilder::stroke(px(1.));

        let mut points = vec![];

        for v in self.data.iter() {
            let x_tick = (self.x)(v);
            let y_tick = (self.y1)(v);

            if let (Some(x), Some(y)) = (x_tick, y_tick) {
                let pos = origin_point(px(x as f32), px(y as f32), origin);

                points.push(pos);
            }
        }

        if points.is_empty() {
            return (None, None);
        }

        if points.len() == 1 {
            area_builder.move_to(points[0]);
            line_builder.move_to(points[0]);
            return (area_builder.build().ok(), line_builder.build().ok());
        }

        match self.stroke_style {
            StrokeStyle::Natural => {
                area_builder.move_to(points[0]);
                line_builder.move_to(points[0]);
                let n = points.len();
                for i in 0..n - 1 {
                    let p0 = if i == 0 { points[0] } else { points[i - 1] };
                    let p1 = points[i];
                    let p2 = points[i + 1];
                    let p3 = if i + 2 < n {
                        points[i + 2]
                    } else {
                        points[n - 1]
                    };

                    // Catmull-Rom to Bezier
                    let c1 = Point::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
                    let c2 = Point::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);

                    area_builder.cubic_bezier_to(p2, c1, c2);
                    line_builder.cubic_bezier_to(p2, c1, c2);
                }
            }
            StrokeStyle::Linear => {
                area_builder.move_to(points[0]);
                line_builder.move_to(points[0]);
                for p in &points[1..] {
                    area_builder.line_to(*p);
                    line_builder.line_to(*p);
                }
            }
        }

        // Close path
        if let Some(last) = self.data.last() {
            let x_tick = (self.x)(last);
            if let (Some(x), Some(y)) = (x_tick, self.y0) {
                area_builder.line_to(origin_point(px(x as f32), px(y as f32), bounds.origin));
                area_builder.line_to(origin_point(px(0.), px(y as f32), bounds.origin));
                area_builder.close();
            }
        }

        (area_builder.build().ok(), line_builder.build().ok())
    }

    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window) {
        let (area, line) = self.path(bounds);

        if let Some(area) = area {
            window.paint_path(area, self.fill);
        }
        if let Some(line) = line {
            window.paint_path(line, self.stroke);
        }
    }
}
