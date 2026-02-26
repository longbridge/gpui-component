use std::rc::Rc;

use gpui::{App, Bounds, Hsla, Pixels, SharedString, Window, px};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Grid, Plot, PlotAxis, StrokeStyle,
        scale::{Scale, ScaleLinear, ScalePoint, Sealed},
        shape::Line,
    },
};

use super::build_point_x_labels;

#[derive(IntoPlot)]
pub struct LineChart<T, X, Y>
where
    T: 'static,
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Option<Rc<dyn Fn(&T) -> Y>>,
    stroke: Option<Hsla>,
    stroke_style: StrokeStyle,
    dot: bool,
    tick_margin: usize,
    show_x_axis: bool,
}

impl<T, X, Y> LineChart<T, X, Y>
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
            stroke: None,
            stroke_style: Default::default(),
            dot: false,
            x: None,
            y: None,
            tick_margin: 1,
            show_x_axis: true,
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

    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    pub fn natural(mut self) -> Self {
        self.stroke_style = StrokeStyle::Natural;
        self
    }

    pub fn linear(mut self) -> Self {
        self.stroke_style = StrokeStyle::Linear;
        self
    }

    pub fn step_after(mut self) -> Self {
        self.stroke_style = StrokeStyle::StepAfter;
        self
    }

    pub fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    pub fn tick_margin(mut self, tick_margin: usize) -> Self {
        self.tick_margin = tick_margin;
        self
    }

    /// Show or hide the x-axis line and labels.
    ///
    /// Default is true.
    pub fn x_axis(mut self, show_x_axis: bool) -> Self {
        self.show_x_axis = show_x_axis;
        self
    }
}

impl<T, X, Y> Plot for LineChart<T, X, Y>
where
    X: PartialEq + Into<SharedString> + 'static,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(x_fn), Some(y_fn)) = (self.x.as_ref(), self.y.as_ref()) else {
            return;
        };

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.show_x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // X scale
        let x = ScalePoint::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width]);

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
        if self.show_x_axis {
            let labels = build_point_x_labels(
                &self.data,
                x_fn.as_ref(),
                &x,
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

        // Draw line
        let stroke = self.stroke.unwrap_or(cx.theme().chart_2);
        let x_fn = x_fn.clone();
        let y_fn = y_fn.clone();
        let mut line = Line::new()
            .data(&self.data)
            .x(move |d| x.tick(&x_fn(d)))
            .y(move |d| y.tick(&y_fn(d)))
            .stroke(stroke)
            .stroke_style(self.stroke_style)
            .stroke_width(2.);

        if self.dot {
            line = line.dot().dot_size(8.).dot_fill_color(stroke);
        }

        line.paint(&bounds, window);
    }
}
