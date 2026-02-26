use std::rc::Rc;

use gpui::{App, Bounds, Hsla, Pixels, SharedString, TextAlign, Window, px};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Grid, Plot, PlotAxis,
        label::Text,
        scale::{Scale, ScaleBand, ScaleLinear, Sealed},
        shape::Bar,
    },
};

use super::build_band_x_labels;

#[derive(IntoPlot)]
pub struct BarChart<T, X, Y>
where
    T: 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Option<Rc<dyn Fn(&T) -> Y>>,
    fill: Option<Rc<dyn Fn(&T) -> Hsla>>,
    tick_margin: usize,
    label: Option<Rc<dyn Fn(&T) -> SharedString>>,
    x_axis: bool,
}

impl<T, X, Y> BarChart<T, X, Y>
where
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            x: None,
            y: None,
            fill: None,
            tick_margin: 1,
            label: None,
            x_axis: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> Y + 'static) -> Self {
        self.y = Some(Rc::new(y));
        self
    }

    pub fn fill<H>(mut self, fill: impl Fn(&T) -> H + 'static) -> Self
    where
        H: Into<Hsla> + 'static,
    {
        self.fill = Some(Rc::new(move |t| fill(t).into()));
        self
    }

    pub fn tick_margin(mut self, tick_margin: usize) -> Self {
        self.tick_margin = tick_margin;
        self
    }

    pub fn label<S>(mut self, label: impl Fn(&T) -> S + 'static) -> Self
    where
        S: Into<SharedString> + 'static,
    {
        self.label = Some(Rc::new(move |t| label(t).into()));
        self
    }

    /// Show or hide the x-axis line and labels.
    ///
    /// Default is true.
    pub fn x_axis(mut self, x_axis: bool) -> Self {
        self.x_axis = x_axis;
        self
    }
}

impl<T, X, Y> Plot for BarChart<T, X, Y>
where
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(x_fn), Some(y_fn)) = (self.x.as_ref(), self.y.as_ref()) else {
            return;
        };

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // X scale
        let x = ScaleBand::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width])
            .padding_inner(0.4)
            .padding_outer(0.2);
        let band_width = x.band_width();

        // Y scale, ensure start from 0.
        let y = ScaleLinear::new(
            self.data
                .iter()
                .map(|v| y_fn(v))
                .chain(Some(Y::zero()))
                .collect(),
            vec![height, 10.],
        );

        // Draw X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let labels = build_band_x_labels(
                &self.data,
                x_fn.as_ref(),
                &x,
                band_width,
                self.tick_margin,
                cx.theme().muted_foreground,
            );
            axis = axis.x(height).x_label(labels);
        }
        axis.paint(&bounds, window, cx);

        // Draw grid
        Grid::new()
            .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
            .stroke(cx.theme().border)
            .dash_array(&[px(4.), px(2.)])
            .paint(&bounds, window);

        // Draw bars
        let x_fn = x_fn.clone();
        let y_fn = y_fn.clone();
        let default_fill = cx.theme().chart_2;
        let fill = self.fill.clone();
        let label_color = cx.theme().foreground;
        let mut bar = Bar::new()
            .data(&self.data)
            .band_width(band_width)
            .x(move |d| x.tick(&x_fn(d)))
            .y0(move |_| height)
            .y1(move |d| y.tick(&y_fn(d)))
            .fill(move |d| fill.as_ref().map(|f| f(d)).unwrap_or(default_fill));

        if let Some(label) = self.label.as_ref() {
            let label = label.clone();
            bar = bar.label(move |d, p| {
                vec![Text::new(label(d), p, label_color).align(TextAlign::Center)]
            });
        }

        bar.paint(&bounds, window, cx);
    }
}
