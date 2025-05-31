mod axis;
mod grid;
pub mod label;
pub mod scale;
pub mod shape;
pub mod tooltip;

pub use macros::IntoPlot;

use std::{fmt::Debug, ops::Add};

use gpui::{
    point, px, App, Bounds, IntoElement, Path, PathBuilder, PathStyle, Pixels, Point,
    StrokeOptions, Window,
};

pub use axis::{Axis, AxisText, AXIS_GAP};
pub use grid::Grid;
pub use label::Label;

pub trait Plot: 'static + IntoElement {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App);
}

#[derive(Clone, Copy, Default)]
pub enum StrokeStyle {
    #[default]
    Natural,
    Linear,
}

pub fn origin_point<T>(x: T, y: T, origin: Point<T>) -> Point<T>
where
    T: Default + Clone + Debug + PartialEq + Add<Output = T>,
{
    point(x, y) + origin
}

// TODO: Move into gpui
//
// https://github.com/zed-industries/zed/pull/31678
pub fn dash_line<T>(start: Point<T>, end: Point<T>, dash_array: [T; 2]) -> Option<Path<Pixels>>
where
    T: Default + Clone + Copy + Debug + PartialEq + Add<Output = T> + Into<f64>,
{
    let mut path = lyon::path::Path::builder();
    path.begin(lyon::geom::point(
        start.x.into() as f32,
        start.y.into() as f32,
    ));
    path.line_to(lyon::geom::point(end.x.into() as f32, end.y.into() as f32));
    path.end(false);
    let path = path.build();

    // Make path dashable.
    let measure = lyon::algorithms::measure::PathMeasurements::from_path(&path, 0.01);
    let mut sampler =
        measure.create_sampler(&path, lyon::algorithms::measure::SampleType::Normalized);
    let mut dashes = lyon::path::Path::builder();
    let length = sampler.length();
    let dash_length = dash_array[0].into() as f32;
    let gap_length = dash_array[1].into() as f32;
    let pattern_length = dash_length + gap_length;
    let num_patterns = (length / pattern_length).ceil() as usize;
    for i in 0..num_patterns {
        let start = i as f32 * pattern_length / length;
        let end = (i as f32 * pattern_length + dash_length) / length;
        sampler.split_range(start..end.min(1.), &mut dashes);
    }

    let mut path: PathBuilder = dashes.into();
    path = path.with_style(PathStyle::Stroke(
        StrokeOptions::default().with_line_width(1.),
    ));
    path.build().ok()
}

pub fn polygon<T>(points: &[Point<T>], bounds: &Bounds<Pixels>) -> Option<Path<Pixels>>
where
    T: Default + Clone + Copy + Debug + Into<f64> + PartialEq,
{
    let mut path = PathBuilder::stroke(px(1.));
    let points = &points
        .iter()
        .map(|p| {
            point(
                px((p.x.into() + bounds.origin.x.to_f64()) as f32),
                px((p.y.into() + bounds.origin.y.to_f64()) as f32),
            )
        })
        .collect::<Vec<_>>();
    path.add_polygon(points, false);
    path.build().ok()
}
