use gpui::{fill, point, px, App, Bounds, Hsla, PaintQuad, Pixels, Point, Window};

use crate::plot::{
    label::{Label, Text, TEXT_GAP, TEXT_HEIGHT},
    origin_point,
};

#[allow(clippy::type_complexity)]
pub struct Bar<T> {
    data: Vec<T>,
    x: Box<dyn Fn(&T) -> Option<f64>>,
    band_width: f64,
    y0: f64,
    y1: Box<dyn Fn(&T) -> Option<f64>>,
    fill: Box<dyn Fn(&T) -> Hsla>,
    label: Option<Box<dyn Fn(&T, Point<Pixels>) -> Text>>,
}

impl<T> Default for Bar<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            x: Box::new(|_| None),
            band_width: 0.,
            y0: 0.,
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

    pub fn data<I>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.data = data.into_iter().collect();
        self
    }

    pub fn x<F>(mut self, x: F) -> Self
    where
        F: Fn(&T) -> Option<f64> + 'static,
    {
        self.x = Box::new(x);
        self
    }

    pub fn band_width(mut self, band_width: f64) -> Self {
        self.band_width = band_width;
        self
    }

    pub fn y0(mut self, y0: f64) -> Self {
        self.y0 = y0;
        self
    }

    pub fn y1<F>(mut self, y: F) -> Self
    where
        F: Fn(&T) -> Option<f64> + 'static,
    {
        self.y1 = Box::new(y);
        self
    }

    pub fn fill<F, C>(mut self, fill: F) -> Self
    where
        F: Fn(&T) -> C + 'static,
        C: Into<Hsla>,
    {
        self.fill = Box::new(move |v| fill(v).into());
        self
    }

    pub fn label<F>(mut self, label: F) -> Self
    where
        F: Fn(&T, Point<Pixels>) -> Text + 'static,
    {
        self.label = Some(Box::new(label));
        self
    }

    fn path(&self, bounds: &Bounds<Pixels>) -> (Vec<PaintQuad>, Label) {
        let origin = bounds.origin;
        let mut graph = vec![];
        let mut labels = vec![];

        for v in &self.data {
            let x_tick = (self.x)(v);
            let y_tick = (self.y1)(v);

            if let (Some(x_tick), Some(y_tick)) = (x_tick, y_tick) {
                let is_negative = y_tick > self.y0;
                let (p1, p2) = if is_negative {
                    (
                        origin_point(px(x_tick as f32), px(self.y0 as f32), origin),
                        origin_point(
                            px((x_tick + self.band_width) as f32),
                            px(y_tick as f32),
                            origin,
                        ),
                    )
                } else {
                    (
                        origin_point(px(x_tick as f32), px(y_tick as f32), origin),
                        origin_point(
                            px((x_tick + self.band_width) as f32),
                            px(self.y0 as f32),
                            origin,
                        ),
                    )
                };

                let color = (self.fill)(v);

                graph.push(fill(Bounds::from_corners(p1, p2), color));

                if let Some(label) = &self.label {
                    labels.push(label(
                        v,
                        point(
                            px((x_tick + self.band_width / 2.) as f32),
                            if is_negative {
                                px((y_tick + TEXT_GAP) as f32)
                            } else {
                                px((y_tick - TEXT_HEIGHT) as f32)
                            },
                        ),
                    ));
                }
            }
        }

        (graph, Label::new(labels))
    }

    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (graph, labels) = self.path(bounds);
        for quad in graph {
            window.paint_quad(quad);
        }
        labels.paint(bounds, window, cx);
    }
}
