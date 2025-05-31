use std::marker::PhantomData;

use gpui::{px, rgb, App, Bounds, Pixels, SharedString, TextAlign, Window};
use macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    plot::{
        scale::{Scale, ScaleLinear, ScalePoint, Sealed},
        shape::Line,
        Axis, AxisText, Grid, Plot, StrokeStyle, AXIS_GAP,
    },
    ActiveTheme,
};

use super::ChartDelegate;

#[derive(IntoPlot)]
pub struct LineChart<T, X, Y>
where
    T: ChartDelegate<X, Y> + 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    stroke_style: StrokeStyle,
    point: bool,
    _phantom: PhantomData<(X, Y)>,
}

impl<T, X, Y> LineChart<T, X, Y>
where
    T: ChartDelegate<X, Y> + 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            stroke_style: Default::default(),
            point: false,
            _phantom: PhantomData,
        }
    }

    pub fn linear(mut self) -> Self {
        self.stroke_style = StrokeStyle::Linear;
        self
    }

    pub fn point(mut self) -> Self {
        self.point = true;
        self
    }
}

impl<T, X, Y> Plot for LineChart<T, X, Y>
where
    T: ChartDelegate<X, Y> + 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let width = bounds.size.width.to_f64();
        let height = bounds.size.height.to_f64() - AXIS_GAP;

        // X scale
        let x = ScalePoint::new(self.data.iter().map(|v| v.x()).collect(), vec![0., width]);

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
        let data_len = self.data.len();
        let x_label = self.data.iter().enumerate().filter_map(|(i, d)| {
            x.tick(&d.x()).map(|x_tick| {
                let align = match i {
                    0 => TextAlign::Left,
                    i if i == data_len - 1 => TextAlign::Right,
                    _ => TextAlign::Center,
                };
                AxisText::new(d.x(), x_tick, cx.theme().muted_foreground).align(align)
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

        // Draw line
        let mut line = Line::new()
            .data(&self.data)
            .x(move |d| x.tick(&d.x()))
            .y(move |d| y.tick(&d.y()))
            .stroke(rgb(0x2563eb))
            .stroke_style(self.stroke_style)
            .stroke_width(2.);

        if self.point {
            line = line.point().point_size(8.).point_fill_color(rgb(0x2563eb));
        }

        line.paint(&bounds, window);
    }
}
