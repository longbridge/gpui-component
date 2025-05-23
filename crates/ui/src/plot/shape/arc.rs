// @reference: https://d3js.org/d3-shape/arc

use std::{f64::consts::PI, fmt::Debug};

use gpui::{point, Bounds, Hsla, Path, PathBuilder, Pixels, Point, Window};
use lyon::path::{traits::SvgPathBuilder, ArcFlags};

const EPSILON: f64 = 1e-12;
const HALF_PI: f64 = PI / 2.;

pub struct ArcData<'a, T> {
    pub data: &'a T,
    pub index: usize,
    pub value: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub pad_angle: f64,
}

impl<T> Debug for ArcData<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ArcData {{ index: {}, value: {}, start_angle: {}, end_angle: {}, pad_angle: {} }}",
            self.index, self.value, self.start_angle, self.end_angle, self.pad_angle
        )
    }
}

pub struct Arc {
    inner_radius: f64,
    outer_radius: f64,
}

impl Default for Arc {
    fn default() -> Self {
        Self {
            inner_radius: 0.,
            outer_radius: 0.,
        }
    }
}

impl Arc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inner_radius(mut self, inner_radius: f64) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    pub fn outer_radius(mut self, outer_radius: f64) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    pub fn centroid<T>(&self, arc: &ArcData<T>) -> Point<f64> {
        let start_angle = arc.start_angle - HALF_PI;
        let end_angle = arc.end_angle - HALF_PI;
        let r = (self.inner_radius + self.outer_radius) / 2.;
        let a = (start_angle + end_angle) / 2.;

        point(r * a.cos(), r * a.sin())
    }

    fn path<T>(&self, arc: &ArcData<T>, bounds: &Bounds<Pixels>) -> Option<Path<Pixels>> {
        let start_angle = arc.start_angle - HALF_PI;
        let end_angle = arc.end_angle - HALF_PI;
        let pad_angle = arc.pad_angle;
        let r0 = self.inner_radius.max(0.);
        let r1 = self.outer_radius.max(0.);

        // Calculate the center point.
        let center_x = bounds.origin.x.to_f64() + bounds.size.width.to_f64() / 2.;
        let center_y = bounds.origin.y.to_f64() + bounds.size.height.to_f64() / 2.;

        // Angle difference.
        let da = end_angle - start_angle;
        if r1 < EPSILON || da.abs() < EPSILON {
            return None;
        }

        // In order to avoid gaps between sectors and ensure smooth boundaries,
        // a relative radius fine-tuning method is used here.
        // By slightly expanding the outer radius and shrinking the inner radius,
        // pixel-level gaps are reduced.
        let fudge = (r1.max(r0) * 0.1).clamp(3., 10.);
        let r1 = r1 + fudge;
        let r0 = if r0 > EPSILON {
            (r0 - fudge).max(0.0)
        } else {
            0.0
        };

        // Handle pad angle.
        let (a0_outer, a1_outer, a0_inner, a1_inner) = if r0 > EPSILON && pad_angle > 0.0 {
            let pad_width = r1 * pad_angle;
            let pad_angle_outer = pad_width / r1;
            let mut pad_angle_inner = pad_width / r0;
            let max_inner_pad = da * 0.8;
            if pad_angle_inner > max_inner_pad {
                pad_angle_inner = max_inner_pad;
            }
            (
                start_angle + pad_angle_outer * 0.5,
                end_angle - pad_angle_outer * 0.5,
                start_angle + pad_angle_inner * 0.5,
                end_angle - pad_angle_inner * 0.5,
            )
        } else {
            let pad = pad_angle * 0.5;
            (
                start_angle + pad,
                end_angle - pad,
                start_angle + pad,
                end_angle - pad,
            )
        };

        let da_outer = a1_outer - a0_outer;
        if da_outer <= 0. {
            return None;
        }

        // Calculate the start and end points of the outer arc.
        let x01 = center_x + r1 * a0_outer.cos();
        let y01 = center_y + r1 * a0_outer.sin();
        let x11 = center_x + r1 * a1_outer.cos();
        let y11 = center_y + r1 * a1_outer.sin();

        let mut builder = lyon::path::Path::svg_builder();

        // Move to the start point of the outer arc.
        builder.move_to(lyon::geom::point(x01 as f32, y01 as f32));

        // Draw the outer arc.
        let large_arc = (a1_outer - a0_outer).abs() > PI;
        builder.arc_to(
            lyon::geom::vector(r1 as f32, r1 as f32),
            lyon::geom::Angle::zero(),
            ArcFlags {
                large_arc,
                sweep: true,
            },
            lyon::geom::point(x11 as f32, y11 as f32),
        );

        if r0 > EPSILON {
            // End point of the inner arc.
            let x10 = center_x + r0 * a1_inner.cos();
            let y10 = center_y + r0 * a1_inner.sin();
            builder.line_to(lyon::geom::point(x10 as f32, y10 as f32));

            // Draw the inner arc.
            let x00 = center_x + r0 * a0_inner.cos();
            let y00 = center_y + r0 * a0_inner.sin();
            let large_arc_inner = (a1_inner - a0_inner).abs() > PI;
            builder.arc_to(
                lyon::geom::vector(r0 as f32, r0 as f32),
                lyon::geom::Angle::zero(),
                ArcFlags {
                    large_arc: large_arc_inner,
                    sweep: false,
                },
                lyon::geom::point(x00 as f32, y00 as f32),
            );
        } else {
            // If there is no inner radius, draw a line to the center.
            builder.line_to(lyon::geom::point(center_x as f32, center_y as f32));
        }

        builder.close();
        let path: PathBuilder = builder.into();
        path.build().ok()
    }

    pub fn paint<T>(
        &self,
        arc: &ArcData<T>,
        color: impl Into<Hsla>,
        bounds: &Bounds<Pixels>,
        window: &mut Window,
    ) {
        let path = self.path(arc, bounds);
        if let Some(path) = path {
            window.paint_path(path, color.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_default() {
        let arc = Arc::default();
        assert_eq!(arc.inner_radius, 0.);
        assert_eq!(arc.outer_radius, 0.);
    }

    #[test]
    fn test_arc_builder() {
        let arc = Arc::new().inner_radius(10.).outer_radius(20.);

        assert_eq!(arc.inner_radius, 10.);
        assert_eq!(arc.outer_radius, 20.);
    }

    #[test]
    fn test_arc_centroid() {
        let arc = Arc::new().inner_radius(10.).outer_radius(20.);

        let arc_data = ArcData {
            data: &(),
            index: 0,
            value: 1.,
            start_angle: 0.,
            end_angle: PI,
            pad_angle: 0.,
        };

        let centroid = arc.centroid(&arc_data);
        let expected_radius = (10. + 20.) / 2.;
        let expected_angle = (0. + PI - 2. * HALF_PI) / 2.;

        assert_eq!(centroid.x, expected_radius * expected_angle.cos());
        assert_eq!(centroid.y, expected_radius * expected_angle.sin());
    }
}
