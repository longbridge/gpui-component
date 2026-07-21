use std::{
    f32::consts::{PI, TAU},
    rc::Rc,
};

use gpui::{
    AnyElement, App, Background, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, SharedString,
    TextAlign, Window, point, px,
};
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
        tooltip::{Dot, Tooltip, TooltipState},
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
    names: Vec<SharedString>,
    label: Option<Rc<dyn Fn(&T) -> SharedString + 'static>>,
    label_color: Option<Hsla>,
    label_gap: f32,
    max_value: Option<Y>,
    outer_radius: f32,
    grid: bool,
    grid_levels: usize,
    dot: bool,
    id: Option<ElementId>,
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
            names: vec![],
            label: None,
            label_color: None,
            label_gap: DEFAULT_LABEL_GAP,
            max_value: None,
            outer_radius: 0.,
            grid: true,
            grid_levels: DEFAULT_GRID_LEVELS,
            dot: false,
            id: None,
        }
    }

    /// Enable an interactive hover tooltip (a dot and row per series at the
    /// hovered dimension).
    ///
    /// The `id` must be unique among sibling elements. Without it, the chart
    /// stays a non-interactive plot.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the name of the most recently added series, shown in its tooltip row.
    ///
    /// Call after the matching [`RadarChart::value`]
    /// (e.g. `.value(..).stroke(..).name("Desktop")`).
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.names.push(name.into());
        self
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

    /// The stroke color of the series at the given index, set or default.
    ///
    /// Defaults to the theme chart colors, cycled per series.
    fn series_stroke(&self, ix: usize, cx: &App) -> Hsla {
        let colors = [
            cx.theme().chart_1,
            cx.theme().chart_2,
            cx.theme().chart_3,
            cx.theme().chart_4,
            cx.theme().chart_5,
        ];

        self.strokes
            .get(ix)
            .copied()
            .unwrap_or(colors[ix % colors.len()])
    }

    /// The resolved outer radius for the given bounds.
    fn resolve_outer_radius(&self, bounds: &Bounds<Pixels>) -> f32 {
        if self.outer_radius.is_zero() {
            bounds.size.height.as_f32() * 0.4
        } else {
            self.outer_radius
        }
    }

    /// Build the radius scale from the center to the outer ring.
    ///
    /// The domain includes zero so non-negative data starts at the center.
    /// Shared by `paint` and `tooltip_state` so the two stay in sync.
    fn scale(&self, outer_radius: f32) -> ScaleLinear<Y> {
        let domain = if let Some(max_value) = self.max_value {
            vec![Y::zero(), max_value]
        } else {
            self.data
                .iter()
                .flat_map(|d| self.values.iter().map(|value_fn| value_fn(d)))
                .chain(Some(Y::zero()))
                .collect()
        };

        ScaleLinear::new(domain, vec![0., outer_radius])
    }

    /// Map a cursor position to the nearest spoke index, or `None` when the
    /// cursor is outside the radar.
    fn hovered_index(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> Option<usize> {
        let n = self.data.len();
        if n == 0 {
            return None;
        }

        let outer_radius = self.resolve_outer_radius(&bounds);
        let dx = position.x.as_f32() - bounds.size.width.as_f32() / 2.;
        let dy = position.y.as_f32() - bounds.size.height.as_f32() / 2.;
        if dx.hypot(dy) > outer_radius + self.label_gap {
            return None;
        }

        // Screen angle -> chart angle (0 at 12 o'clock, clockwise).
        let angle = (dy.atan2(dx) + HALF_PI).rem_euclid(TAU);
        Some((angle * n as f32 / TAU).round() as usize % n)
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

        let outer_radius = self.resolve_outer_radius(&bounds);
        let angle_step = TAU / n as f32;
        let center_x = bounds.size.width.as_f32() / 2.;
        let center_y = bounds.size.height.as_f32() / 2.;
        let scale = self.scale(outer_radius);

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
            let stroke = self.series_stroke(i, cx);
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

    fn id(&self) -> Option<ElementId> {
        self.id.clone()
    }

    fn tooltip_state(
        &self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _cx: &App,
    ) -> Option<TooltipState> {
        if self.values.is_empty() {
            return None;
        }
        let index = self.hovered_index(position, bounds)?;
        let d = self.data.get(index)?;

        let outer_radius = self.resolve_outer_radius(&bounds);
        let scale = self.scale(outer_radius);
        let center_x = bounds.size.width.as_f32() / 2.;
        let center_y = bounds.size.height.as_f32() / 2.;
        let angle = index as f32 * TAU / self.data.len() as f32 - HALF_PI;

        // One dot per series at the hovered dimension's vertex.
        let dots = self
            .values
            .iter()
            .filter_map(|value_fn| {
                let radius = scale.tick(&value_fn(d))?;
                Some(point(
                    px(center_x + radius * angle.cos()),
                    px(center_y + radius * angle.sin()),
                ))
            })
            .collect();

        Some(TooltipState::new(index, position, dots))
    }

    fn tooltip(
        &self,
        state: &TooltipState,
        cursor: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let d = self.data.get(state.index)?;

        let dot_stroke = cx.theme().background;

        // No crosshair: a radar has no cartesian axis to snap to; the dots mark
        // the hovered dimension's vertices instead.
        let mut tooltip =
            Tooltip::new(cursor, bounds.size)
                .gap(px(8.))
                .dots(state.dots.iter().enumerate().map(|(i, p)| {
                    Dot::new(*p)
                        .stroke(dot_stroke)
                        .fill(self.series_stroke(i, cx))
                }));

        if let Some(label_fn) = self.label.as_ref() {
            tooltip = tooltip.title(label_fn(d));
        }

        // One row per series: swatch + label + value.
        for (i, value_fn) in self.values.iter().enumerate() {
            let name = self.names.get(i).cloned().unwrap_or_default();
            let value = value_fn(d).to_f64()?;
            tooltip = tooltip.row(self.series_stroke(i, cx), name, format!("{}", value));
        }

        Some(tooltip.into_any_element())
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
            .name("A")
            .value(|d| d.b)
            .max_value(100.)
            .outer_radius(120.)
            .label_gap(8.)
            .grid(false)
            .grid_levels(5)
            .dot()
            .id("radar");

        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.values.len(), 2);
        assert_eq!(chart.strokes.len(), 1);
        assert_eq!(chart.fills.len(), 1);
        assert_eq!(chart.names.len(), 1);
        assert!(chart.label.is_some());
        assert_eq!(chart.max_value, Some(100.));
        assert_eq!(chart.outer_radius, 120.);
        assert_eq!(chart.label_gap, 8.);
        assert!(!chart.grid);
        assert_eq!(chart.grid_levels, 5);
        assert!(chart.dot);
        assert!(chart.id.is_some());

        let values = (chart.values[0](&data[0]), chart.values[1](&data[0]));
        assert_eq!(values, (80., 60.));
    }

    #[test]
    fn test_radar_chart_grid_levels_min() {
        let chart: RadarChart<Item, f64> = RadarChart::new(vec![]).grid_levels(0);
        assert_eq!(chart.grid_levels, 1);
    }

    #[test]
    fn test_radar_chart_hovered_index() {
        let data = (0..4)
            .map(|i| Item {
                subject: format!("S{}", i).into(),
                a: 50.,
                b: 50.,
            })
            .collect::<Vec<_>>();

        // Bounds 200x200 => center (100, 100), default outer radius 80,
        // hover region 80 + 10 (label gap) = 90.
        let chart: RadarChart<Item, f64> = RadarChart::new(data).value(|d| d.a);
        let bounds = gpui::Bounds::new(point(px(0.), px(0.)), gpui::size(px(200.), px(200.)));

        // The four spokes point at 12, 3, 6 and 9 o'clock.
        assert_eq!(
            chart.hovered_index(point(px(100.), px(30.)), bounds),
            Some(0)
        );
        assert_eq!(
            chart.hovered_index(point(px(170.), px(100.)), bounds),
            Some(1)
        );
        assert_eq!(
            chart.hovered_index(point(px(100.), px(170.)), bounds),
            Some(2)
        );
        assert_eq!(
            chart.hovered_index(point(px(30.), px(100.)), bounds),
            Some(3)
        );

        // Nearest spoke wins between two spokes.
        assert_eq!(
            chart.hovered_index(point(px(110.), px(40.)), bounds),
            Some(0)
        );
        assert_eq!(
            chart.hovered_index(point(px(160.), px(90.)), bounds),
            Some(1)
        );

        // Outside the radar.
        assert_eq!(chart.hovered_index(point(px(100.), px(5.)), bounds), None);
        assert_eq!(chart.hovered_index(point(px(5.), px(5.)), bounds), None);
    }
}
