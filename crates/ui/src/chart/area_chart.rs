use std::rc::Rc;

use gpui::{px, rgb, rgba, App, Background, Bounds, Pixels, SharedString, TextAlign, Window};
use macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    plot::{
        scale::{Scale, ScaleLinear, ScalePoint, Sealed},
        shape::Area,
        Axis, AxisText, Grid, Plot, StrokeStyle, AXIS_GAP,
    },
    ActiveTheme,
};

#[derive(IntoPlot)]
pub struct AreaChart<T, X, Y>
where
    T: 'static,
    X: Clone + Copy + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    stroke_style: StrokeStyle,
    fill: Vec<Background>,
    tick_margin: usize,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Vec<Rc<dyn Fn(&T) -> Y>>,
}

impl<T, X, Y> AreaChart<T, X, Y>
where
    T: 'static,
    X: Clone + Copy + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            stroke_style: Default::default(),
            fill: vec![],
            tick_margin: 1,
            x: None,
            y: vec![],
        }
    }

    pub fn fill(mut self, fill: impl Into<Background>) -> Self {
        self.fill.push(fill.into());
        self
    }

    pub fn linear(mut self) -> Self {
        self.stroke_style = StrokeStyle::Linear;
        self
    }

    pub fn tick_margin(mut self, tick_margin: usize) -> Self {
        self.tick_margin = tick_margin;
        self
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> Y + 'static) -> Self {
        self.y.push(Rc::new(y));
        self
    }
}

impl<T, X, Y> Plot for AreaChart<T, X, Y>
where
    T: 'static,
    X: Clone + Copy + PartialEq + Into<SharedString> + 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(x_fn) = self.x.as_ref() else {
            return;
        };

        if self.y.len() == 0 {
            return;
        }

        let width = bounds.size.width.to_f64();
        let height = bounds.size.height.to_f64() - AXIS_GAP;

        // X scale
        let x = ScalePoint::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width]);

        // Y scale
        let y = ScaleLinear::new(
            self.data
                .iter()
                .flat_map(|v| self.y.iter().map(|y_fn| y_fn(v)))
                .chain(Some(Y::zero()))
                .collect::<Vec<_>>(),
            vec![0., height],
        );

        // Draw X axis
        let x_fn = x_fn.clone();
        let data_len = self.data.len();
        let x_label = self.data.iter().enumerate().filter_map(|(i, d)| {
            if (i + 1) % self.tick_margin == 0 {
                x.tick(&x_fn(d)).map(|x_tick| {
                    let align = match i {
                        0 => TextAlign::Left,
                        i if i == data_len - 1 => TextAlign::Right,
                        _ => TextAlign::Center,
                    };
                    AxisText::new(x_fn(d).into(), x_tick, cx.theme().muted_foreground).align(align)
                })
            } else {
                None
            }
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

        // Draw area
        for (i, y_fn) in self.y.iter().enumerate() {
            let x = x.clone();
            let y = y.clone();
            let x_fn = x_fn.clone();
            let y_fn = y_fn.clone();

            let fill = *self.fill.get(i).unwrap_or(&rgba(0x2563eb66).into());

            Area::new()
                .data(&self.data)
                .x(move |d| x.tick(&x_fn(d)))
                .y0(height)
                .y1(move |d| y.tick(&y_fn(d)))
                .fill(fill)
                .stroke(rgb(0x2563eb))
                .stroke_style(self.stroke_style)
                .paint(&bounds, window);
        }
    }
}
