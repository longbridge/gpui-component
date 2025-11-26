use gpui::{App, Bounds, Hsla, PaintQuad, Pixels, Point, Window, fill, point, px};

use crate::plot::{
    label::{PlotLabel, TEXT_GAP, TEXT_HEIGHT, Text},
    origin_point,
};

#[allow(clippy::type_complexity)]
pub struct Bar<T> {
    data: Vec<T>,
    x: Box<dyn Fn(&T) -> Option<f32>>,
    band_width: f32,
    y0: Box<dyn Fn(&T) -> f32>,
    y1: Box<dyn Fn(&T) -> Option<f32>>,
    fill: Box<dyn Fn(&T) -> Hsla>,
    label: Option<Box<dyn Fn(&T, Point<Pixels>) -> Vec<Text>>>,
}

impl<T> Default for Bar<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            x: Box::new(|_| None),
            band_width: 0.,
            y0: Box::new(|_| 0.),
            y1: Box::new(|_| None),
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

    /// Set the x of the Bar.
    pub fn x<F>(mut self, x: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.x = Box::new(x);
        self
    }

    /// Set the band width of the Bar.
    pub fn band_width(mut self, band_width: f32) -> Self {
        self.band_width = band_width;
        self
    }

    /// Set the y0 of the Bar.
    pub fn y0<F>(mut self, y: F) -> Self
    where
        F: Fn(&T) -> f32 + 'static,
    {
        self.y0 = Box::new(y);
        self
    }

    /// Set the y1 of the Bar.
    pub fn y1<F>(mut self, y: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.y1 = Box::new(y);
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
            let x_tick = (self.x)(v);
            let y_tick = (self.y1)(v);
            let y0 = (self.y0)(v);

            if let (Some(x_tick), Some(y_tick)) = (x_tick, y_tick) {
                let is_negative = y_tick > y0;
                let (p1, p2) = if is_negative {
                    (
                        origin_point(px(x_tick), px(y0), origin),
                        origin_point(px(x_tick + self.band_width), px(y_tick), origin),
                    )
                } else {
                    (
                        origin_point(px(x_tick), px(y_tick), origin),
                        origin_point(px(x_tick + self.band_width), px(y0), origin),
                    )
                };

                let color = (self.fill)(v);

                graph.push(fill(Bounds::from_corners(p1, p2), color));

                if let Some(label) = &self.label {
                    labels.extend(label(
                        v,
                        point(
                            px(x_tick + self.band_width / 2.),
                            if is_negative {
                                px(y_tick + TEXT_GAP)
                            } else {
                                px(y_tick - TEXT_HEIGHT)
                            },
                        ),
                    ));
                }
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
