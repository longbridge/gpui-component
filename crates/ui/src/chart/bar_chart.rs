use std::{ops::RangeInclusive, rc::Rc};

use gpui::{
    App, Background, Bounds, Corners, Hsla, LinearColorStop, Pixels, Point, SharedString, Size, TextAlign,
    Window, linear_gradient, px,
};
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
    fill: Option<Rc<dyn Fn(&T, Bounds<f32>, Bounds<f32>, BarAlignment) -> Background>>,
    #[allow(clippy::type_complexity)]
    fill_gradient: Option<
        Rc<dyn Fn(&T, RangeInclusive<f32>, &dyn Fn(f32) -> f32) -> [LinearColorStop; 2]>,
    >,
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
            fill_gradient: None,
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

    /// Set a per-datum verbatim fill.
    ///
    /// The closure receives:
    ///
    /// 1. the datum,
    /// 2. the **bar's bounds** in pixel space, expressed relative to the
    ///    chart's origin (i.e. the bar's painted rectangle within the chart),
    /// 3. the **chart's bounds** in pixel space with origin `(0, 0)` and size
    ///    equal to the full chart extent, and
    /// 4. the bar's [`BarAlignment`] (so callers can branch on orientation,
    ///    e.g. flip a gradient angle).
    ///
    /// Both rectangles share the same coordinate system, so callers can
    /// implement arbitrary chart-aware backgrounds — bar-local gradients,
    /// chart-wide gradients, patterns, sampled colormaps, etc. — without any
    /// help from the library.
    ///
    /// Accepts any type convertible to [`Background`]. Setting this clears any
    /// previously set [`BarChart::fill_gradient`].
    pub fn fill<Bg>(
        mut self,
        fill: impl Fn(&T, Bounds<f32>, Bounds<f32>, BarAlignment) -> Bg + 'static,
    ) -> Self
    where
        Bg: Into<Background> + 'static,
    {
        self.fill = Some(Rc::new(move |t, bar_bounds, chart_bounds, alignment| {
            fill(t, bar_bounds, chart_bounds, alignment).into()
        }));
        self.fill_gradient = None;
        self
    }

    /// Set a per-datum auto-oriented linear gradient fill.
    ///
    /// The closure receives the datum, the chart's full data range
    /// (`chart_range`, derived from all data values), and a `chart_to_bar`
    /// remap helper that maps a chart-value coordinate to a bar-local
    /// gradient position (where `0.0` is the bar's base and `1.0` is its tip).
    ///
    /// Use bar-local positions directly for per-bar gradients (every bar
    /// looks the same regardless of its value):
    ///
    /// ```ignore
    /// .fill_gradient(|_, _, _| [
    ///     linear_color_stop(c.opacity(0.3), 0.0),
    ///     linear_color_stop(c, 1.0),
    /// ])
    /// ```
    ///
    /// Or use `chart_to_bar` to position stops at chart-relative values, so
    /// each bar shows the slice of a chart-wide gradient corresponding to
    /// its own `[base, value]` span:
    ///
    /// ```ignore
    /// .fill_gradient(|_, chart_range, chart_to_bar| [
    ///     linear_color_stop(c.opacity(0.3), chart_to_bar(*chart_range.start())),
    ///     linear_color_stop(c,              chart_to_bar(*chart_range.end())),
    /// ])
    /// ```
    ///
    /// Stop positions returned outside `[0, 1]` are clipped to the bar; the
    /// library interpolates colors at the clip points so the on-bar gradient
    /// still matches the chart-wide one.
    ///
    /// The gradient angle is derived from [`BarAlignment`] so stop-0 is at the
    /// base and stop-1 at the tip. Setting this clears any previously set
    /// [`BarChart::fill`].
    pub fn fill_gradient(
        mut self,
        fill: impl Fn(&T, RangeInclusive<f32>, &dyn Fn(f32) -> f32) -> [LinearColorStop; 2]
            + 'static,
    ) -> Self {
        self.fill_gradient = Some(Rc::new(fill));
        self.fill = None;
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

        // Far edge of the value axis in pixel space (opposite the baseline).
        let far = match alignment {
            BarAlignment::Bottom => 10.,
            BarAlignment::Top => value_dim - 10.,
            BarAlignment::Left => value_dim - value_end_gap,
            BarAlignment::Right => value_end_gap,
        };

        // Draw grid: lines perpendicular to the value axis, evenly spaced
        // across the value range and excluding the line at the baseline.
        if self.grid {
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
        let default_fill: Background = cx.theme().chart_2.into();
        let fill = self.fill.clone();
        let fill_gradient = self.fill_gradient.clone();
        let label_color = cx.theme().foreground;

        // Chart bounds in pixel space, with origin (0, 0) and size equal to
        // the full chart extent. Passed to user `fill` closures so they can
        // position chart-wide backgrounds (gradients, patterns, etc.).
        let chart_bounds: Bounds<f32> = Bounds {
            origin: Point::new(0., 0.),
            size: Size::new(total_width, total_height),
        };

        // Chart data range in f32 — passed to `fill_gradient` callers and used
        // by the `chart_to_bar` remap helper.
        let chart_range = {
            let mut lo = 0.0_f32;
            let mut hi = 0.0_f32;
            for v in &self.data {
                if let Some(f) = value_fn(v).to_f32() {
                    lo = lo.min(f);
                    hi = hi.max(f);
                }
            }
            lo..=hi
        };

        let mut bar = Bar::new()
            .data(&self.data)
            .alignment(alignment)
            .band_width(band_width)
            .cross(move |d| band_scale.tick(&band_fn_cloned(d)))
            .base(move |_| baseline)
            .value(move |d| value_scale.tick(&value_fn_cloned(d)))
            .corner_radii(self.corner_radii);

        bar = match (fill, fill_gradient) {
            (_, Some(fg)) => {
                let value_fn_for_grad = value_fn.clone();
                bar.fill(move |d, _frame, alignment| {
                    let v = value_fn_for_grad(d).to_f32().unwrap_or(0.);
                    let base_v = 0.0_f32;
                    let bar_lo = base_v.min(v);
                    let bar_hi = base_v.max(v);
                    let bar_span = (bar_hi - bar_lo).max(f32::EPSILON);
                    let chart_to_bar = |chart_value: f32| (chart_value - bar_lo) / bar_span;
                    let stops = fg(d, chart_range.clone(), &chart_to_bar);
                    let [s0, s1] = clip_stops_to_bar(stops);
                    let bg: Background = linear_gradient(alignment.gradient_angle(), s0, s1);
                    bg
                })
            }
            (Some(f), _) => {
                bar.fill(move |d, frame, alignment| f(d, frame, chart_bounds, alignment))
            }
            _ => bar.fill(move |_, _, _| default_fill),
        };

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

/// Clip a two-stop gradient to bar-local `[0, 1]`, interpolating colors at the
/// clip points so the on-bar gradient matches the (possibly broader) gradient
/// the caller defined.
///
/// When a stop position falls outside `[0, 1]` (e.g. because `chart_to_bar`
/// returned a value past the bar's edge for a chart-relative gradient),
/// gpui's renderer would clamp the position and lose the gradient effect.
/// This function instead replaces such a stop with the color sampled along
/// the line through both stops at position `0.0` or `1.0`, preserving the
/// visual slice.
fn clip_stops_to_bar(stops: [LinearColorStop; 2]) -> [LinearColorStop; 2] {
    let [a, b] = stops;
    let p0 = a.percentage;
    let p1 = b.percentage;
    let lerp = |t: f32| -> Hsla {
        Hsla {
            h: a.color.h + (b.color.h - a.color.h) * t,
            s: a.color.s + (b.color.s - a.color.s) * t,
            l: a.color.l + (b.color.l - a.color.l) * t,
            a: a.color.a + (b.color.a - a.color.a) * t,
        }
    };
    let span = p1 - p0;
    let sample = |target: f32| -> Hsla {
        if span.abs() < f32::EPSILON {
            a.color
        } else {
            lerp((target - p0) / span)
        }
    };
    let new_a = if (0. ..=1.).contains(&p0) {
        a
    } else {
        LinearColorStop {
            color: sample(p0.clamp(0., 1.)),
            percentage: p0.clamp(0., 1.),
        }
    };
    let new_b = if (0. ..=1.).contains(&p1) {
        b
    } else {
        LinearColorStop {
            color: sample(p1.clamp(0., 1.)),
            percentage: p1.clamp(0., 1.),
        }
    };
    [new_a, new_b]
}
