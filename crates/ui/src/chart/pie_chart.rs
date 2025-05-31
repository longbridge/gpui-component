use gpui::{App, Bounds, Hsla, Pixels, Window};
use macros::IntoPlot;
use num_traits::Zero;

use crate::plot::{
    shape::{Arc, Pie},
    Plot,
};

pub trait PieChartDelegate {
    fn value(&self) -> Option<f64>;
    fn color(&self) -> impl Into<Hsla>;
}

#[derive(IntoPlot)]
pub struct PieChart<T: PieChartDelegate + 'static> {
    data: Vec<T>,
    inner_radius: f64,
    outer_radius: f64,
    pad_angle: f64,
}

impl<T: PieChartDelegate + 'static> PieChart<T> {
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            inner_radius: 0.,
            outer_radius: 0.,
            pad_angle: 0.,
        }
    }

    pub fn inner_radius(mut self, inner_radius: f64) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    pub fn outer_radius(mut self, outer_radius: f64) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    pub fn pad_angle(mut self, pad_angle: f64) -> Self {
        self.pad_angle = pad_angle;
        self
    }
}

impl<T: PieChartDelegate + 'static> Plot for PieChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, _: &mut App) {
        let outer_radius = if self.outer_radius.is_zero() {
            bounds.size.height.to_f64() * 0.4
        } else {
            self.outer_radius
        };

        let arc = Arc::new()
            .inner_radius(self.inner_radius)
            .outer_radius(outer_radius);
        let mut pie = Pie::<T>::new().value(|d| d.value());
        pie = pie.pad_angle(self.pad_angle);
        let arcs = pie.arcs(&self.data);

        for a in &arcs {
            arc.paint(a, a.data.color(), &bounds, window);
        }
    }
}
