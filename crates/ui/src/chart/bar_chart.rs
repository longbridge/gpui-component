use std::marker::PhantomData;

use gpui::{px, rgb, App, Bounds, Pixels, SharedString, TextAlign, Window};
use macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    plot::{
        scale::{Scale, ScaleBand, ScaleLinear, Sealed},
        shape::Bar,
        Axis, AxisText, Grid, Plot, AXIS_GAP,
    },
    ActiveTheme,
};

use super::ChartDelegate;

#[derive(IntoPlot)]
pub struct BarChart<T, X, Y>
where
    T: ChartDelegate<X, Y> + 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    _phantom: PhantomData<(X, Y)>,
}

impl<T, X, Y> BarChart<T, X, Y>
where
    T: ChartDelegate<X, Y> + 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            _phantom: PhantomData,
        }
    }
}

impl<T, X, Y> Plot for BarChart<T, X, Y>
where
    T: ChartDelegate<X, Y> + 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let width = bounds.size.width.to_f64();
        let height = bounds.size.height.to_f64() - AXIS_GAP;

        // X scale
        let x = ScaleBand::new(self.data.iter().map(|v| v.x()).collect(), vec![0., width])
            .padding_inner(0.4)
            .padding_outer(0.2);
        let band_width = x.band_width();

        // Y scale, ensure start from 0.
        let y = ScaleLinear::new(
            self.data
                .iter()
                .map(|v| v.y())
                .chain(Some(Y::zero()))
                .collect(),
            vec![0., height],
        );

        // Draw X axis
        let x_label = self.data.iter().filter_map(|d| {
            x.tick(&d.x()).map(|x_tick| {
                AxisText::new(
                    d.label(),
                    x_tick + band_width / 2.,
                    cx.theme().muted_foreground,
                )
                .align(TextAlign::Center)
            })
        });

        Axis::new()
            .x(height)
            .x_label(x_label)
            .stroke(rgb(0xf0f0f0))
            .paint(&bounds, window, cx);

        // Draw grid
        Grid::new()
            .y((0..=3).map(|i| height * i as f64 / 4.0).collect())
            .stroke(rgb(0xf0f0f0))
            .dash_array([px(4.), px(2.)])
            .paint(&bounds, window);

        // Draw bars
        Bar::new()
            .data(&self.data)
            .band_width(band_width)
            .x(move |d| x.tick(&d.x()))
            .y0(height)
            .y1(move |d| y.tick(&d.y()))
            .fill(move |_| rgb(0x2563eb))
            .paint(&bounds, window, cx);
    }
}
