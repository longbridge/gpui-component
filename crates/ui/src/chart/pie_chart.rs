use std::rc::Rc;

use gpui::{App, Bounds, Hsla, Pixels, Point, SharedString, TextAlign, Window, point};
use gpui_component_macros::IntoPlot;
use num_traits::Zero;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_SIZE, Text},
        polygon,
        shape::{Arc, ArcData, Pie},
    },
};

/// The default extra gap (in pixels) between `outer_radius` and the label radius.
const DEFAULT_LABEL_GAP: f32 = 15.;

#[derive(IntoPlot)]
pub struct PieChart<T: 'static> {
    data: Vec<T>,
    inner_radius: f32,
    inner_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    outer_radius: f32,
    outer_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    pad_angle: f32,
    value: Option<Rc<dyn Fn(&T) -> f32>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    label: Option<Rc<dyn Fn(&T) -> SharedString + 'static>>,
    label_line_color: Option<Rc<dyn Fn(&T) -> Hsla + 'static>>,
    label_color: Option<Hsla>,
    label_gap: f32,
}

impl<T> PieChart<T> {
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            inner_radius: 0.,
            inner_radius_fn: None,
            outer_radius: 0.,
            outer_radius_fn: None,
            pad_angle: 0.,
            value: None,
            color: None,
            label: None,
            label_line_color: None,
            label_color: None,
            label_gap: DEFAULT_LABEL_GAP,
        }
    }

    /// Set the inner radius of the pie chart.
    pub fn inner_radius(mut self, inner_radius: f32) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    /// Set the inner radius of the pie chart based on the arc data.
    pub fn inner_radius_fn(
        mut self,
        inner_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.inner_radius_fn = Some(Rc::new(inner_radius_fn));
        self
    }

    fn get_inner_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(inner_radius_fn) = self.inner_radius_fn.as_ref() {
            inner_radius_fn(arc)
        } else {
            self.inner_radius
        }
    }

    /// Set the outer radius of the pie chart.
    pub fn outer_radius(mut self, outer_radius: f32) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    /// Set the outer radius of the pie chart based on the arc data.
    pub fn outer_radius_fn(
        mut self,
        outer_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.outer_radius_fn = Some(Rc::new(outer_radius_fn));
        self
    }

    fn get_outer_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(outer_radius_fn) = self.outer_radius_fn.as_ref() {
            outer_radius_fn(arc)
        } else {
            self.outer_radius
        }
    }

    /// Set the pad angle of the pie chart.
    pub fn pad_angle(mut self, pad_angle: f32) -> Self {
        self.pad_angle = pad_angle;
        self
    }

    pub fn value(mut self, value: impl Fn(&T) -> f32 + 'static) -> Self {
        self.value = Some(Rc::new(value));
        self
    }

    /// Set the color of the pie chart.
    pub fn color<H>(mut self, color: impl Fn(&T) -> H + 'static) -> Self
    where
        H: Into<Hsla> + 'static,
    {
        self.color = Some(Rc::new(move |t| color(t).into()));
        self
    }

    /// Set the label text for each slice.
    ///
    /// Once set, a "leader line + text" is drawn outside the ring for every
    /// slice.
    pub fn label(mut self, label: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self
    }

    /// Set the leader line color per slice (defaults to `cx.theme().border`).
    pub fn label_line_color(mut self, color: impl Fn(&T) -> Hsla + 'static) -> Self {
        self.label_line_color = Some(Rc::new(color));
        self
    }

    /// Set the label text color (defaults to `cx.theme().foreground`).
    pub fn label_color(mut self, color: Hsla) -> Self {
        self.label_color = Some(color);
        self
    }

    /// Set the extra gap between `outer_radius` and the label radius
    /// (defaults to 15px).
    pub fn label_gap(mut self, gap: f32) -> Self {
        self.label_gap = gap;
        self
    }
}

impl<T> Plot for PieChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(value_fn) = self.value.as_ref() else {
            return;
        };

        let outer_radius = if self.outer_radius.is_zero() {
            bounds.size.height.as_f32() * 0.4
        } else {
            self.outer_radius
        };

        let arc = Arc::new()
            .inner_radius(self.inner_radius)
            .outer_radius(outer_radius);
        let value_fn = value_fn.clone();
        let mut pie = Pie::<T>::new().value(move |d| Some(value_fn(d)));
        pie = pie.pad_angle(self.pad_angle);
        let arcs = pie.arcs(&self.data);

        for a in &arcs {
            let inner_radius = self.get_inner_radius(a);
            let outer_radius = self.get_outer_radius(a);
            arc.paint(
                a,
                if let Some(color_fn) = self.color.as_ref() {
                    color_fn(a.data)
                } else {
                    cx.theme().chart_2
                },
                Some(inner_radius),
                Some(outer_radius),
                &bounds,
                window,
            );
        }

        // Draw leader-line labels outside the ring (only when `label` is set).
        let Some(label_fn) = self.label.as_ref() else {
            return;
        };

        let label_radius = outer_radius + self.label_gap;
        let center_x = bounds.size.width.as_f32() / 2.;
        let center_y = bounds.size.height.as_f32() / 2.;
        let label_arc = Arc::new()
            .inner_radius(label_radius)
            .outer_radius(label_radius);
        let edge_arc = Arc::new()
            .inner_radius(outer_radius)
            .outer_radius(outer_radius);

        let label_color = self.label_color.unwrap_or(cx.theme().foreground);
        let default_line_color = cx.theme().border;

        let mut labels = vec![];
        let mut polylines = vec![];
        let mut last_end_angle = 0.;
        let mut last_point = Point::<f32>::default();
        for a in &arcs {
            // Skip tiny slices (< 0.5°).
            if a.end_angle - last_end_angle < std::f32::consts::PI / 360. {
                continue;
            }

            let centroid = label_arc.centroid(a);
            let Point {
                x: label_x,
                y: label_y,
            } = centroid;
            let Point { x: arc_x, y: arc_y } = edge_arc.centroid(a);
            let is_right = label_x > 0.;

            // Adjacent labels on the same side that are too close get nudged
            // vertically by half a text height.
            let safe_label_y = if centroid.x.signum() == last_point.x.signum()
                && (last_point.y - centroid.y).abs() < TEXT_SIZE
            {
                if centroid.y < last_point.y {
                    label_y - TEXT_SIZE
                } else {
                    label_y + TEXT_SIZE
                }
            } else {
                label_y
            };

            // Leader line: ring edge -> centroid -> horizontal pull to ±label_radius.
            let pts = [
                point(arc_x + center_x, arc_y + center_y),
                point(label_x + center_x, safe_label_y + center_y),
                point(
                    (if is_right {
                        label_radius
                    } else {
                        -label_radius
                    }) + center_x,
                    safe_label_y + center_y,
                ),
            ];
            if let Some(p) = polygon(&pts, &bounds) {
                let line_color = if let Some(line_color_fn) = self.label_line_color.as_ref() {
                    line_color_fn(a.data)
                } else {
                    default_line_color
                };
                polylines.push((p, line_color));
            }

            // Text sits 4px further out, aligned by side.
            let origin = point(
                (if is_right {
                    label_radius + 4.
                } else {
                    -label_radius - 4.
                }) + center_x,
                safe_label_y - TEXT_SIZE / 2. + center_y,
            );
            labels.push(
                Text::new(label_fn(a.data), origin, label_color).align(if is_right {
                    TextAlign::Left
                } else {
                    TextAlign::Right
                }),
            );

            last_end_angle = a.end_angle;
            last_point = centroid;
        }

        for (path, color) in polylines {
            window.paint_path(path, color);
        }
        PlotLabel::new(labels).paint(&bounds, window, cx);
    }
}
