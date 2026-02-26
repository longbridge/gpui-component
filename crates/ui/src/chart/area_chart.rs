use std::rc::Rc;

use gpui::{App, Background, Bounds, Hsla, Pixels, SharedString, Window, px};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Grid, Plot, PlotAxis, StrokeStyle,
        scale::{Scale, ScaleLinear, ScalePoint, Sealed},
        shape::Area,
    },
};

use super::build_point_x_labels;

#[derive(IntoPlot)]
pub struct AreaChart<T, X, Y>
where
    T: 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Vec<Rc<dyn Fn(&T) -> Y>>,
    strokes: Vec<Hsla>,
    stroke_styles: Vec<StrokeStyle>,
    fills: Vec<Background>,
    tick_margin: usize,
    x_axis: bool,
}

impl<T, X, Y> AreaChart<T, X, Y>
where
    X: Clone + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            stroke_styles: vec![],
            strokes: vec![],
            fills: vec![],
            tick_margin: 1,
            x: None,
            y: vec![],
            x_axis: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> Y + 'static) -> Self {
        self.y.push(Rc::new(y));
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.strokes.push(stroke.into());
        self
    }

    pub fn fill(mut self, fill: impl Into<Background>) -> Self {
        self.fills.push(fill.into());
        self
    }

    pub fn natural(mut self) -> Self {
        self.stroke_styles.push(StrokeStyle::Natural);
        self
    }

    pub fn linear(mut self) -> Self {
        self.stroke_styles.push(StrokeStyle::Linear);
        self
    }

    pub fn step_after(mut self) -> Self {
        self.stroke_styles.push(StrokeStyle::StepAfter);
        self
    }

    pub fn tick_margin(mut self, tick_margin: usize) -> Self {
        self.tick_margin = tick_margin;
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

impl<T, X, Y> Plot for AreaChart<T, X, Y>
where
    X: Clone + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(x_fn) = self.x.as_ref() else {
            return;
        };

        if self.y.len() == 0 {
            return;
        }

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // X scale
        let x = ScalePoint::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width]);

        // Y scale
        let domain = self
            .data
            .iter()
            .flat_map(|v| self.y.iter().map(|y_fn| y_fn(v)))
            .chain(Some(Y::zero()))
            .collect::<Vec<_>>();
        let y = ScaleLinear::new(domain, vec![height, 10.]);

        // Draw X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
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

        // Draw area
        for (i, y_fn) in self.y.iter().enumerate() {
            let x = x.clone();
            let y = y.clone();
            let x_fn = x_fn.clone();
            let y_fn = y_fn.clone();

            let fill = *self
                .fills
                .get(i)
                .unwrap_or(&cx.theme().chart_2.opacity(0.4).into());

            let stroke = *self.strokes.get(i).unwrap_or(&cx.theme().chart_2);

            let stroke_style = *self
                .stroke_styles
                .get(i)
                .unwrap_or(self.stroke_styles.first().unwrap_or(&Default::default()));

            Area::new()
                .data(&self.data)
                .x(move |d| x.tick(&x_fn(d)))
                .y0(height)
                .y1(move |d| y.tick(&y_fn(d)))
                .stroke(stroke)
                .stroke_style(stroke_style)
                .fill(fill)
                .paint(&bounds, window);
        }
    }
}
