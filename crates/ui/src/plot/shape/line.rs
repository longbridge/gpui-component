// @reference: https://d3js.org/d3-shape/line

use gpui::{
    px, quad, size, Background, BorderStyle, Bounds, Hsla, PaintQuad, Path, PathBuilder, Pixels,
    Point, Window,
};

use crate::plot::{origin_point, StrokeStyle};

#[allow(clippy::type_complexity)]
pub struct Line<T> {
    data: Vec<T>,
    x: Box<dyn Fn(&T) -> Option<f64>>,
    y: Box<dyn Fn(&T) -> Option<f64>>,
    stroke: Background,
    stroke_width: Pixels,
    stroke_style: StrokeStyle,
    point: bool,
    point_size: Pixels,
    point_fill_color: Hsla,
    point_stroke_color: Option<Hsla>,
}

impl<T> Default for Line<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            x: Box::new(|_| None),
            y: Box::new(|_| None),
            stroke: Default::default(),
            stroke_width: px(1.),
            stroke_style: Default::default(),
            point: false,
            point_size: px(4.),
            point_fill_color: gpui::transparent_black(),
            point_stroke_color: None,
        }
    }
}

impl<T> Line<T> {
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

    pub fn y<F>(mut self, y: F) -> Self
    where
        F: Fn(&T) -> Option<f64> + 'static,
    {
        self.y = Box::new(y);
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Background>) -> Self {
        self.stroke = stroke.into();
        self
    }

    pub fn stroke_width(mut self, stroke_width: impl Into<Pixels>) -> Self {
        self.stroke_width = stroke_width.into();
        self
    }

    pub fn stroke_style(mut self, stroke_style: StrokeStyle) -> Self {
        self.stroke_style = stroke_style;
        self
    }

    pub fn point(mut self) -> Self {
        self.point = true;
        self
    }

    pub fn point_size(mut self, point_size: impl Into<Pixels>) -> Self {
        self.point_size = point_size.into();
        self
    }

    pub fn point_fill_color(mut self, point_fill_color: impl Into<Hsla>) -> Self {
        self.point_fill_color = point_fill_color.into();
        self
    }

    pub fn point_stroke_color(mut self, point_stroke_color: impl Into<Hsla>) -> Self {
        self.point_stroke_color = Some(point_stroke_color.into());
        self
    }

    fn paint_point(&self, point: Point<Pixels>) -> PaintQuad {
        quad(
            gpui::bounds(point, size(self.point_size, self.point_size)),
            self.point_size / 2.,
            self.point_fill_color,
            px(1.),
            self.point_stroke_color.unwrap_or(self.point_fill_color),
            BorderStyle::default(),
        )
    }

    fn path(&self, bounds: &Bounds<Pixels>) -> (Option<Path<Pixels>>, Vec<PaintQuad>) {
        let origin = bounds.origin;
        let mut builder = PathBuilder::stroke(self.stroke_width);
        let mut points = vec![];
        let mut paint_points = vec![];

        for v in self.data.iter() {
            let x_tick = (self.x)(v);
            let y_tick = (self.y)(v);

            if let (Some(x), Some(y)) = (x_tick, y_tick) {
                let pos = origin_point(px(x as f32), px(y as f32), origin);

                if self.point {
                    let point_radius = self.point_size.to_f64() / 2.;
                    let point_pos = origin_point(
                        px((x - point_radius) as f32),
                        px((y - point_radius) as f32),
                        origin,
                    );
                    paint_points.push(self.paint_point(point_pos));
                }

                points.push(pos);
            }
        }

        if points.is_empty() {
            return (None, paint_points);
        }

        if points.len() == 1 {
            builder.move_to(points[0]);
            return (builder.build().ok(), paint_points);
        }

        match self.stroke_style {
            StrokeStyle::Natural => {
                builder.move_to(points[0]);
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

                    builder.cubic_bezier_to(p2, c1, c2);
                }
            }
            StrokeStyle::Linear => {
                builder.move_to(points[0]);
                for p in &points[1..] {
                    builder.line_to(*p);
                }
            }
        }

        (builder.build().ok(), paint_points)
    }

    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window) {
        let (path, point) = self.path(bounds);
        if let Some(path) = path {
            window.paint_path(path, self.stroke);
        }
        for p in point {
            window.paint_quad(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use gpui::{point, px, Bounds};

    #[test]
    fn test_line_path() {
        let data = vec![1., 2., 3.];
        let line = Line::new()
            .data(data.clone())
            .x(|v| Some(*v))
            .y(|v| Some(*v * 2.));

        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(100.)));
        let (path, points) = line.path(&bounds);

        assert!(path.is_some());
        assert!(points.is_empty());

        let line_with_points = Line::new()
            .data(data)
            .x(|v| Some(*v))
            .y(|v| Some(*v * 2.))
            .point();

        let (_, points) = line_with_points.path(&bounds);
        assert_eq!(points.len(), 3);
    }
}
