use std::rc::Rc;

use gpui::{App, Bounds, Corners, Hsla, Pixels, SharedString, TextAlign, Window, px};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive};

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, AxisLabelSide, Grid, Plot, PlotAxis,
        label::{Text, TEXT_GAP, TEXT_SIZE, measure_text_width},
        scale::{Scale, ScaleBand, ScaleLinear, Sealed},
        shape::{Bar, BarAlignment},
    },
};

use super::build_band_labels;

#[derive(IntoPlot)]
pub struct BarChart<T, B, V>
where
    T: 'static,
    B: PartialEq + Into<SharedString> + 'static,
    V: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    band: Option<Rc<dyn Fn(&T) -> B>>,
    value: Option<Rc<dyn Fn(&T) -> V>>,
    fill: Option<Rc<dyn Fn(&T) -> Hsla>>,
    tick_margin: usize,
    label: Option<Rc<dyn Fn(&T) -> SharedString>>,
    label_axis: bool,
    grid: bool,
    alignment: BarAlignment,
    corner_radii: Corners<Pixels>,
}

impl<T, B, V> BarChart<T, B, V>
where
    B: PartialEq + Into<SharedString> + 'static,
    V: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            band: None,
            value: None,
            fill: None,
            tick_margin: 1,
            label: None,
            label_axis: true,
            grid: true,
            alignment: BarAlignment::default(),
            corner_radii: Corners::all(px(0.)),
        }
    }

    /// Map each datum to its band-axis value (the categorical/ordinal axis).
    pub fn band(mut self, band: impl Fn(&T) -> B + 'static) -> Self {
        self.band = Some(Rc::new(band));
        self
    }

    /// Map each datum to its numeric value along the value axis.
    pub fn value(mut self, value: impl Fn(&T) -> V + 'static) -> Self {
        self.value = Some(Rc::new(value));
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

    /// Show or hide the band-axis line and labels.
    ///
    /// Default is true.
    pub fn label_axis(mut self, label_axis: bool) -> Self {
        self.label_axis = label_axis;
        self
    }

    pub fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }

    /// Set the bar alignment.
    ///
    /// Default is [`BarAlignment::Bottom`].
    pub fn alignment(mut self, alignment: BarAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the corner radii applied to every bar rectangle.
    ///
    /// Use [`Corners::all`] for uniform rounding, or construct [`Corners`] manually
    /// to round only specific corners (e.g. just the tip end of each bar).
    pub fn corner_radii(mut self, corner_radii: impl Into<Corners<Pixels>>) -> Self {
        self.corner_radii = corner_radii.into();
        self
    }
}

impl<T, B, V> Plot for BarChart<T, B, V>
where
    B: PartialEq + Into<SharedString> + 'static,
    V: Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(band_fn), Some(value_fn)) = (self.band.as_ref(), self.value.as_ref()) else {
            return;
        };

        let total_width = bounds.size.width.as_f32();
        let total_height = bounds.size.height.as_f32();
        let axis_gap = if self.label_axis { AXIS_GAP } else { 0. };
        let alignment = self.alignment;
        let is_horizontal = alignment.is_horizontal();

        // Band scale spans the full extent perpendicular to the value axis.
        let band_extent = if is_horizontal {
            total_height
        } else {
            total_width
        };
        let band_scale = ScaleBand::new(
            self.data.iter().map(|v| band_fn(v)).collect(),
            vec![0., band_extent],
        )
        .padding_inner(0.4)
        .padding_outer(0.2);
        let band_width = band_scale.band_width();

        let value_dim = if is_horizontal {
            total_width
        } else {
            total_height
        };
        // For horizontal charts the band labels (category names) are rendered
        // along the value axis and can be arbitrarily wide, so we measure the
        // actual maximum label width instead of using a fixed constant.
        // Similarly, value labels (numbers) at the bar ends are measured so the
        // scale range is always shrunk by exactly the right amount.
        let (band_gap, value_end_gap) = if is_horizontal {
            let font_size = px(TEXT_SIZE);
            let band_gap = if self.label_axis {
                let max_w = self
                    .data
                    .iter()
                    .map(|v| {
                        let s: SharedString = band_fn(v).into();
                        measure_text_width(&s, font_size, window)
                    })
                    .fold(0f32, f32::max);
                // TEXT_GAP: space between axis line and label start/end.
                max_w + TEXT_GAP * 2.
            } else {
                0.
            };
            let value_end_gap = if let Some(label_fn) = self.label.as_ref() {
                let max_w = self
                    .data
                    .iter()
                    .map(|v| measure_text_width(&label_fn(v), font_size, window))
                    .fold(0f32, f32::max);
                max_w + TEXT_GAP * 2.
            } else {
                TEXT_GAP * 4.
            };
            (band_gap, value_end_gap)
        } else {
            (axis_gap, 10.)
        };
        let (range, baseline) = match alignment {
            BarAlignment::Bottom => {
                let baseline = value_dim - axis_gap;
                (vec![baseline, 10.], baseline)
            }
            BarAlignment::Top => {
                let baseline = axis_gap;
                (vec![baseline, value_dim - 10.], baseline)
            }
            BarAlignment::Left => {
                let baseline = band_gap;
                (vec![baseline, value_dim - value_end_gap], baseline)
            }
            BarAlignment::Right => {
                let baseline = value_dim - band_gap;
                (vec![baseline, value_end_gap], baseline)
            }
        };
        let value_scale = ScaleLinear::new(
            self.data
                .iter()
                .map(|v| value_fn(v))
                .chain(Some(V::zero()))
                .collect(),
            range,
        );

        // Draw band axis (with categorical labels).
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.label_axis {
            let labels = build_band_labels(
                &self.data,
                band_fn.as_ref(),
                &band_scale,
                band_width,
                self.tick_margin,
                cx.theme().muted_foreground,
            );
            axis = match alignment {
                BarAlignment::Bottom => axis.x(baseline).x_label(labels),
                BarAlignment::Top => axis
                    .x(baseline)
                    .x_label_side(AxisLabelSide::Start)
                    .x_label(labels),
                BarAlignment::Left => axis
                    .y(baseline)
                    .y_label_side(AxisLabelSide::Start)
                    .y_label(labels.into_iter().map(|t| t.align(TextAlign::Right))),
                BarAlignment::Right => axis
                    .y(baseline)
                    .y_label(labels.into_iter().map(|t| t.align(TextAlign::Left))),
            };
        }
        axis.paint(&bounds, window, cx);

        // Draw grid: lines perpendicular to the value axis, evenly spaced
        // across the value range and excluding the line at the baseline.
        if self.grid {
            let far = match alignment {
                BarAlignment::Bottom => 10.,
                BarAlignment::Top => value_dim - 10.,
                BarAlignment::Left => value_dim - value_end_gap,
                BarAlignment::Right => value_end_gap,
            };
            let grid_steps: Vec<f32> = (0..4)
                .map(|i| far + (baseline - far) * i as f32 / 4.0)
                .collect();
            let grid = Grid::new()
                .stroke(cx.theme().border)
                .dash_array(&[px(4.), px(2.)]);
            let grid = if is_horizontal {
                grid.x(grid_steps)
            } else {
                grid.y(grid_steps)
            };
            grid.paint(&bounds, window);
        }

        // Draw bars.
        let band_fn_cloned = band_fn.clone();
        let value_fn_cloned = value_fn.clone();
        let default_fill = cx.theme().chart_2;
        let fill = self.fill.clone();
        let label_color = cx.theme().foreground;

        let mut bar = Bar::new()
            .data(&self.data)
            .alignment(alignment)
            .band_width(band_width)
            .cross(move |d| band_scale.tick(&band_fn_cloned(d)))
            .base(move |_| baseline)
            .value(move |d| value_scale.tick(&value_fn_cloned(d)))
            .corner_radii(self.corner_radii)
            .fill(move |d| fill.as_ref().map(|f| f(d)).unwrap_or(default_fill));

        if let Some(label) = self.label.as_ref() {
            let label = label.clone();
            let text_align = match alignment {
                BarAlignment::Bottom | BarAlignment::Top => TextAlign::Center,
                BarAlignment::Left => TextAlign::Left,
                BarAlignment::Right => TextAlign::Right,
            };
            bar =
                bar.label(move |d, p| vec![Text::new(label(d), p, label_color).align(text_align)]);
        }

        bar.paint(&bounds, window, cx);
    }
}
