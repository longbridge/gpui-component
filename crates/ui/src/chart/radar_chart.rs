use std::{
    f32::consts::{PI, TAU},
    rc::Rc,
};

use gpui::{App, Background, Bounds, Hsla, Pixels, SharedString, TextAlign, Window, point, px};
use gpui_component_macros::IntoPlot;
use num_traits::{Num, ToPrimitive, Zero};

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_SIZE, Text},
        polygon,
        scale::{Scale, ScaleLinear, Sealed},
        shape::RadialLine,
    },
};

const HALF_PI: f32 = PI / 2.;

/// The default extra gap (in pixels) between the outer grid ring and the labels.
const DEFAULT_LABEL_GAP: f32 = 10.;

/// The default number of concentric grid rings.
const DEFAULT_GRID_LEVELS: usize = 4;

/// A radar (spider) chart.
///
/// Each datum is one dimension (a spoke), placed clockwise around the center
/// starting at 12 o'clock. Add one series per [`RadarChart::value`] call; each
/// series is drawn as a closed polygon connecting its values on every spoke.
#[derive(IntoPlot)]
pub struct RadarChart<T, Y>
where
    T: 'static,
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    data: Vec<T>,
    values: Vec<Rc<dyn Fn(&T) -> Y>>,
    strokes: Vec<Hsla>,
    fills: Vec<Background>,
    label: Option<Rc<dyn Fn(&T) -> SharedString + 'static>>,
    label_color: Option<Hsla>,
    label_gap: f32,
    max_value: Option<Y>,
    outer_radius: f32,
    grid: bool,
    grid_levels: usize,
    dot: bool,
}

impl<T, Y> RadarChart<T, Y>
where
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            values: vec![],
            strokes: vec![],
            fills: vec![],
            label: None,
            label_color: None,
            label_gap: DEFAULT_LABEL_GAP,
            max_value: None,
            outer_radius: 0.,
            grid: true,
            grid_levels: DEFAULT_GRID_LEVELS,
            dot: false,
        }
    }

    /// Add a series to the radar chart.
    ///
    /// Call multiple times to overlay multiple series, each paired with the
    /// matching [`RadarChart::stroke`] and [`RadarChart::fill`] calls.
    pub fn value(mut self, value: impl Fn(&T) -> Y + 'static) -> Self {
        self.values.push(Rc::new(value));
        self
    }

    /// Set the stroke color of the most recently added series.
    ///
    /// Defaults to the theme chart colors, cycled per series.
    pub fn stroke(mut self, stroke: impl Into<Hsla>) -> Self {
        self.strokes.push(stroke.into());
        self
    }

    /// Set the fill color of the most recently added series.
    ///
    /// Defaults to the series stroke color with 0.3 opacity.
    pub fn fill(mut self, fill: impl Into<Background>) -> Self {
        self.fills.push(fill.into());
        self
    }

    /// Set the label text for each dimension, shown outside the outer ring.
    pub fn label(mut self, label: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self
    }

    /// Set the label text color (defaults to `cx.theme().muted_foreground`).
    pub fn label_color(mut self, color: impl Into<Hsla>) -> Self {
        self.label_color = Some(color.into());
        self
    }

    /// Set the extra gap between the outer ring and the labels
    /// (defaults to 10px).
    pub fn label_gap(mut self, gap: f32) -> Self {
        self.label_gap = gap;
        self
    }

    /// Set the value at the outer ring.
    ///
    /// Defaults to the maximum value across all series.
    pub fn max_value(mut self, max_value: Y) -> Self {
        self.max_value = Some(max_value);
        self
    }

    /// Set the outer radius of the radar chart.
    ///
    /// Defaults to 40% of the bounds height.
    pub fn outer_radius(mut self, outer_radius: f32) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    /// Show or hide the grid rings and spokes.
    ///
    /// Default is true.
    pub fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }

    /// Set the number of concentric grid rings (defaults to 4).
    pub fn grid_levels(mut self, grid_levels: usize) -> Self {
        self.grid_levels = grid_levels.max(1);
        self
    }

    /// Show dots on the vertices of each series.
    pub fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    /// The default stroke color of the series at the given index.
    fn default_stroke(&self, ix: usize, cx: &App) -> Hsla {
        let colors = [
            cx.theme().chart_1,
            cx.theme().chart_2,
            cx.theme().chart_3,
            cx.theme().chart_4,
            cx.theme().chart_5,
        ];
        colors[ix % colors.len()]
    }
}

impl<T, Y> Plot for RadarChart<T, Y>
where
    Y: Clone + Copy + PartialOrd + Num + ToPrimitive + Sealed + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let n = self.data.len();
        if n == 0 || self.values.is_empty() {
            return;
        }

        let outer_radius = if self.outer_radius.is_zero() {
            bounds.size.height.as_f32() * 0.4
        } else {
            self.outer_radius
        };
        let angle_step = TAU / n as f32;
        let center_x = bounds.size.width.as_f32() / 2.;
        let center_y = bounds.size.height.as_f32() / 2.;

        // Radius scale from the center to the outer ring. The domain includes
        // zero so non-negative data starts at the center.
        let domain = if let Some(max_value) = self.max_value {
            vec![Y::zero(), max_value]
        } else {
            self.data
                .iter()
                .flat_map(|d| self.values.iter().map(|value_fn| value_fn(d)))
                .chain(Some(Y::zero()))
                .collect()
        };
        let scale = ScaleLinear::new(domain, vec![0., outer_radius]);

        // Draw grid rings and spokes
        if self.grid {
            let stroke = cx.theme().border;

            for level in 1..=self.grid_levels {
                let radius = outer_radius * level as f32 / self.grid_levels as f32;
                RadialLine::new()
                    .data(0..n)
                    .angle(move |_, i| Some(i as f32 * angle_step))
                    .radius(move |_, _| Some(radius))
                    .closed()
                    .stroke(stroke)
                    .paint(&bounds, window);
            }

            for i in 0..n {
                let angle = i as f32 * angle_step - HALF_PI;
                let points = [
                    point(center_x, center_y),
                    point(
                        center_x + outer_radius * angle.cos(),
                        center_y + outer_radius * angle.sin(),
                    ),
                ];
                if let Some(path) = polygon(&points, &bounds) {
                    window.paint_path(path, stroke);
                }
            }
        }

        // Draw series
        for (i, value_fn) in self.values.iter().enumerate() {
            let stroke = self
                .strokes
                .get(i)
                .copied()
                .unwrap_or_else(|| self.default_stroke(i, cx));
            let fill = self
                .fills
                .get(i)
                .copied()
                .unwrap_or_else(|| stroke.opacity(0.3).into());

            let scale = scale.clone();
            let value_fn = value_fn.clone();
            let mut line = RadialLine::new()
                .data(&self.data)
                .angle(move |_, i| Some(i as f32 * angle_step))
                .radius(move |d, _| scale.tick(&value_fn(d)))
                .closed()
                .fill(fill)
                .stroke(stroke)
                .stroke_width(2.);
            if self.dot {
                line = line.dot().dot_size(8.).dot_fill_color(stroke);
            }
            line.paint(&bounds, window);
        }

        // Draw dimension labels outside the outer ring (only when `label` is set).
        let Some(label_fn) = self.label.as_ref() else {
            return;
        };

        let label_radius = outer_radius + self.label_gap;
        let label_color = self.label_color.unwrap_or(cx.theme().muted_foreground);

        let labels = self.data.iter().enumerate().map(|(i, d)| {
            let angle = i as f32 * angle_step - HALF_PI;
            let dx = label_radius * angle.cos();
            let dy = label_radius * angle.sin();
            // Labels on the right are left-aligned, on the left right-aligned,
            // and near the vertical axis centered.
            let align = if dx > 1. {
                TextAlign::Left
            } else if dx < -1. {
                TextAlign::Right
            } else {
                TextAlign::Center
            };

            Text::new(
                label_fn(d),
                point(px(center_x + dx), px(center_y + dy - TEXT_SIZE / 2.)),
                label_color,
            )
            .align(align)
        });

        PlotLabel::new(labels.collect()).paint(&bounds, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Item {
        subject: SharedString,
        a: f64,
        b: f64,
    }

    #[test]
    fn test_radar_chart_builder() {
        let data = vec![
            Item {
                subject: "Sales".into(),
                a: 80.,
                b: 60.,
            },
            Item {
                subject: "Marketing".into(),
                a: 50.,
                b: 90.,
            },
        ];

        let chart = RadarChart::new(data.clone())
            .label(|d| d.subject.clone())
            .value(|d| d.a)
            .stroke(gpui::red())
            .fill(gpui::red())
            .value(|d| d.b)
            .max_value(100.)
            .outer_radius(120.)
            .label_gap(8.)
            .grid(false)
            .grid_levels(5)
            .dot();

        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.values.len(), 2);
        assert_eq!(chart.strokes.len(), 1);
        assert_eq!(chart.fills.len(), 1);
        assert!(chart.label.is_some());
        assert_eq!(chart.max_value, Some(100.));
        assert_eq!(chart.outer_radius, 120.);
        assert_eq!(chart.label_gap, 8.);
        assert!(!chart.grid);
        assert_eq!(chart.grid_levels, 5);
        assert!(chart.dot);

        let values = (chart.values[0](&data[0]), chart.values[1](&data[0]));
        assert_eq!(values, (80., 60.));
    }

    #[test]
    fn test_radar_chart_grid_levels_min() {
        let chart: RadarChart<Item, f64> = RadarChart::new(vec![]).grid_levels(0);
        assert_eq!(chart.grid_levels, 1);
    }
}
