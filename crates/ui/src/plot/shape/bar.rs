use gpui::{App, Bounds, Hsla, PaintQuad, Pixels, Point, Window, fill, point, px};

use crate::plot::{
    label::{PlotLabel, TEXT_GAP, TEXT_HEIGHT, TEXT_SIZE, Text},
    origin_point,
};

/// Alignment of bars within a [`Bar`] shape, controlling both the orientation
/// (vertical vs horizontal) and the side where the baseline lives.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarAlignment {
    /// Vertical bars with the baseline at the bottom; bars grow upward.
    #[default]
    Bottom,
    /// Vertical bars with the baseline at the top; bars grow downward.
    Top,
    /// Horizontal bars with the baseline at the left; bars grow rightward.
    Left,
    /// Horizontal bars with the baseline at the right; bars grow leftward.
    Right,
}

impl BarAlignment {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    pub fn is_vertical(self) -> bool {
        !self.is_horizontal()
    }
}

#[allow(clippy::type_complexity)]
pub struct Bar<T> {
    data: Vec<T>,
    alignment: BarAlignment,
    cross: Box<dyn Fn(&T) -> Option<f32>>,
    band_width: f32,
    base: Box<dyn Fn(&T) -> f32>,
    value: Box<dyn Fn(&T) -> Option<f32>>,
    fill: Box<dyn Fn(&T) -> Hsla>,
    label: Option<Box<dyn Fn(&T, Point<Pixels>) -> Vec<Text>>>,
}

impl<T> Default for Bar<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            alignment: BarAlignment::default(),
            cross: Box::new(|_| None),
            band_width: 0.,
            base: Box::new(|_| 0.),
            value: Box::new(|_| None),
            fill: Box::new(|_| gpui::black()),
            label: None,
        }
    }
}

impl<T> Bar<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the data of the Bar.
    pub fn data<I>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.data = data.into_iter().collect();
        self
    }

    /// Set the alignment of the Bar.
    ///
    /// Default is [`BarAlignment::Bottom`].
    pub fn alignment(mut self, alignment: BarAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the cross-axis position of each bar (in pixels).
    ///
    /// For vertical alignments this is the X coordinate; for horizontal
    /// alignments this is the Y coordinate.
    pub fn cross<F>(mut self, cross: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.cross = Box::new(cross);
        self
    }

    /// Set the band width of the Bar (the bar thickness along the cross axis).
    pub fn band_width(mut self, band_width: f32) -> Self {
        self.band_width = band_width;
        self
    }

    /// Set the baseline position of each bar (in pixels along the value axis).
    pub fn base<F>(mut self, base: F) -> Self
    where
        F: Fn(&T) -> f32 + 'static,
    {
        self.base = Box::new(base);
        self
    }

    /// Set the value-end position of each bar (in pixels along the value axis).
    pub fn value<F>(mut self, value: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.value = Box::new(value);
        self
    }

    /// Set the fill color of the Bar.
    pub fn fill<F, C>(mut self, fill: F) -> Self
    where
        F: Fn(&T) -> C + 'static,
        C: Into<Hsla>,
    {
        self.fill = Box::new(move |v| fill(v).into());
        self
    }

    /// Set the label of the Bar.
    pub fn label<F>(mut self, label: F) -> Self
    where
        F: Fn(&T, Point<Pixels>) -> Vec<Text> + 'static,
    {
        self.label = Some(Box::new(label));
        self
    }

    fn path(&self, bounds: &Bounds<Pixels>) -> (Vec<PaintQuad>, PlotLabel) {
        let origin = bounds.origin;
        let mut graph = vec![];
        let mut labels = vec![];

        for v in &self.data {
            let Some(cross) = (self.cross)(v) else {
                continue;
            };
            let Some(value) = (self.value)(v) else {
                continue;
            };
            let base = (self.base)(v);

            let bw = self.band_width;
            let (p1, p2) = if self.alignment.is_vertical() {
                let x0 = cross;
                let x1 = cross + bw;
                let y_min = value.min(base);
                let y_max = value.max(base);
                (
                    origin_point(px(x0), px(y_min), origin),
                    origin_point(px(x1), px(y_max), origin),
                )
            } else {
                let y0 = cross;
                let y1 = cross + bw;
                let x_min = value.min(base);
                let x_max = value.max(base);
                (
                    origin_point(px(x_min), px(y0), origin),
                    origin_point(px(x_max), px(y1), origin),
                )
            };

            let color = (self.fill)(v);
            graph.push(fill(Bounds::from_corners(p1, p2), color));

            if let Some(label) = &self.label {
                let label_origin = label_origin(self.alignment, cross, base, value, bw);
                labels.extend(label(v, label_origin));
            }
        }

        (graph, PlotLabel::new(labels))
    }

    /// Paint the Bar.
    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (graph, labels) = self.path(bounds);
        for quad in graph {
            window.paint_quad(quad);
        }
        labels.paint(bounds, window, cx);
    }
}

/// Origin point for a bar label, positioned outside the bar at the value end.
///
/// The caller chooses the [`gpui::TextAlign`] (typically `Center` for vertical
/// bars, `Left` for `BarAlignment::Left`, `Right` for `BarAlignment::Right`).
fn label_origin(
    alignment: BarAlignment,
    cross: f32,
    base: f32,
    value: f32,
    band_width: f32,
) -> Point<Pixels> {
    match alignment {
        BarAlignment::Bottom => {
            let cx = cross + band_width / 2.;
            // Normal: value < base (bar grows up). Label above bar end.
            if value <= base {
                point(px(cx), px(value - TEXT_HEIGHT))
            } else {
                point(px(cx), px(value + TEXT_GAP))
            }
        }
        BarAlignment::Top => {
            let cx = cross + band_width / 2.;
            // Normal: value > base (bar grows down). Label below bar end.
            if value >= base {
                point(px(cx), px(value + TEXT_GAP))
            } else {
                point(px(cx), px(value - TEXT_HEIGHT))
            }
        }
        BarAlignment::Left => {
            // Vertical centering: text origin is the top of the glyph cell.
            let cy = cross + band_width / 2. - TEXT_SIZE / 2.;
            // Normal: value > base (bar grows right). Label to the right of bar end.
            if value >= base {
                point(px(value + TEXT_GAP), px(cy))
            } else {
                point(px(value - TEXT_GAP), px(cy))
            }
        }
        BarAlignment::Right => {
            let cy = cross + band_width / 2. - TEXT_SIZE / 2.;
            // Normal: value < base (bar grows left). Label to the left of bar end.
            if value <= base {
                point(px(value - TEXT_GAP), px(cy))
            } else {
                point(px(value + TEXT_GAP), px(cy))
            }
        }
    }
}
