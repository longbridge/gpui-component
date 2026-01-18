// You can draw any chart you want by using the `Plot`.

use gpui::{App, Bounds, Pixels, TextAlign, Window, px};
use gpui_component::{
    ActiveTheme, PixelsExt,
    plot::{
        AXIS_GAP, AxisText, Grid, IntoPlot, Plot, PlotAxis,
        scale::{Scale, ScaleBand, ScaleLinear, ScaleOrdinal},
        shape::{Bar, Stack, StackSeries},
    },
};

use super::DailyDevice;

#[derive(IntoPlot)]
pub struct StackedBarChart {
    data: Vec<DailyDevice>,
    series: Vec<StackSeries<DailyDevice>>,
}

impl StackedBarChart {
    pub fn new(data: Vec<DailyDevice>) -> Self {
        // 1. Calculate the stacked data
        let series = Stack::new()
            .data(data.clone())
            .keys(vec!["desktop", "mobile", "tablet", "watch"])
            .value(move |d: &DailyDevice, key| match key {
                "desktop" => Some(d.desktop as f32),
                "mobile" => Some(d.mobile as f32),
                "tablet" => Some(d.tablet as f32),
                "watch" => Some(d.watch as f32),
                _ => None,
            })
            .series();

        Self { data, series }
    }
}

impl Plot for StackedBarChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32() - AXIS_GAP;

        // 2. Calculate X/Y scales
        let x = ScaleBand::new(
            self.data.iter().map(|v| v.date.clone()).collect(),
            vec![0., width],
        )
        .padding_inner(0.4)
        .padding_outer(0.2);
        let band_width = x.band_width();

        let max = self
            .series
            .iter()
            .flat_map(|s| s.points.iter().map(|p| p.y1))
            .fold(0., f32::max) as f64;

        let y = ScaleLinear::new(vec![0., max], vec![height, 10.]);

        // 3. Draw X axis labels
        let x_label = self.data.iter().filter_map(|d| {
            x.tick(&d.date.clone()).map(|x_tick| {
                AxisText::new(
                    d.date.clone(),
                    x_tick + band_width / 2.,
                    cx.theme().muted_foreground,
                )
                .align(TextAlign::Center)
            })
        });
        PlotAxis::new()
            .x(height)
            .x_label(x_label)
            .stroke(cx.theme().border)
            .paint(&bounds, window, cx);

        // 4. Setup color scale
        let keys = self.series.iter().map(|s| s.key.clone()).collect();
        let colors = vec![
            cx.theme().chart_4,
            cx.theme().chart_3,
            cx.theme().chart_2,
            cx.theme().chart_1,
        ];
        let ordinal = ScaleOrdinal::new(keys, colors);

        // 5. Draw grid lines
        Grid::new()
            .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
            .stroke(cx.theme().border)
            .dash_array(&[px(4.), px(2.)])
            .paint(&bounds, window);

        // 6. Draw stacked bars
        for series in self.series.iter() {
            let x = x.clone();
            let y0 = y.clone();
            let y1 = y.clone();

            let key = &series.key;
            let fill = ordinal.map(&key).unwrap_or(cx.theme().chart_4);

            Bar::new()
                .data(&series.points)
                .band_width(band_width)
                .x(move |d| x.tick(&d.data.date.clone()))
                .y0(move |d| y0.tick(&(d.y0 as f64)).unwrap_or(height))
                .y1(move |d| y1.tick(&(d.y1 as f64)))
                .fill(move |_| fill)
                .paint(&bounds, window, cx);
        }
    }
}
