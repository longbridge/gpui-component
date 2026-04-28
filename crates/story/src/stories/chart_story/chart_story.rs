use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, IntoElement, ParentElement,
    Render, SharedString, Styled, Window, div, linear_color_stop, linear_gradient,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, StyledExt,
    chart::{AreaChart, BarChart, CandlestickChart, LineChart, PieChart},
    divider::Divider,
    dock::PanelControl,
    h_flex,
    plot::shape::BarAlignment,
    v_flex,
};
use serde::Deserialize;

use super::StackedBarChart;
use crate::Story;

#[derive(Clone, Deserialize)]
struct MonthlyDevice {
    pub month: SharedString,
    pub desktop: f64,
    pub color_alpha: f32,
}

impl MonthlyDevice {
    pub fn color(&self, color: Hsla) -> Hsla {
        color.alpha(self.color_alpha)
    }
}

#[derive(Clone, Deserialize)]
pub struct DailyDevice {
    pub date: SharedString,
    pub desktop: f64,
    pub mobile: f64,
    pub tablet: f64,
    pub watch: f64,
}

#[derive(Clone, Deserialize)]
pub struct StockPrice {
    pub date: SharedString,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

pub struct ChartStory {
    focus_handle: FocusHandle,
    daily_devices: Vec<DailyDevice>,
    monthly_devices: Vec<MonthlyDevice>,
    stock_prices: Vec<StockPrice>,
}

impl ChartStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let daily_devices = serde_json::from_str::<Vec<DailyDevice>>(include_str!(
            "../../fixtures/daily-devices.json"
        ))
        .unwrap();
        let monthly_devices = serde_json::from_str::<Vec<MonthlyDevice>>(include_str!(
            "../../fixtures/monthly-devices.json"
        ))
        .unwrap();
        let stock_prices = serde_json::from_str::<Vec<StockPrice>>(include_str!(
            "../../fixtures/stock-prices.json"
        ))
        .unwrap();

        Self {
            daily_devices,
            monthly_devices,
            stock_prices,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for ChartStory {
    fn title() -> &'static str {
        "Chart"
    }

    fn description() -> &'static str {
        "Beautiful Charts & Graphs."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for ChartStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn chart_container(
    title: &str,
    chart: impl IntoElement,
    center: bool,
    cx: &mut Context<ChartStory>,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .h(px(400.))
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .p_4()
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .child(title.to_string()),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("January-June 2025"),
        )
        .child(div().flex_1().py_4().child(chart))
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .text_sm()
                .child("Trending up by 5.2% this month"),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("Showing total visitors for the last 6 months"),
        )
}

impl Render for ChartStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let color = cx.theme().chart_3;
        v_flex()
            .size_full()
            .gap_y_4()
            .bg(cx.theme().background)
            .child(
                div().child(chart_container(
                    "Area Chart - Stacked",
                    AreaChart::new(self.daily_devices.clone())
                        .x(|d| d.date.clone())
                        .y(|d| d.desktop)
                        .stroke(cx.theme().chart_1)
                        .fill(linear_gradient(
                            0.,
                            linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
                            linear_color_stop(cx.theme().background.opacity(0.3), 0.),
                        ))
                        .y(|d| d.mobile)
                        .stroke(cx.theme().chart_2)
                        .fill(linear_gradient(
                            0.,
                            linear_color_stop(cx.theme().chart_2.opacity(0.4), 1.),
                            linear_color_stop(cx.theme().background.opacity(0.3), 0.),
                        ))
                        .tick_margin(8),
                    false,
                    cx,
                )),
            )
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_4()
                    .child(chart_container(
                        "Pie Chart",
                        PieChart::new(self.monthly_devices.clone())
                            .value(|d| d.desktop as f32)
                            .outer_radius(100.)
                            .color(move |d| d.color(color)),
                        true,
                        cx,
                    ))
                    .child(chart_container(
                        "Pie Chart - Donut",
                        PieChart::new(self.monthly_devices.clone())
                            .value(|d| d.desktop as f32)
                            .inner_radius(60.)
                            .outer_radius_fn(|d| 100. - d.index as f32 * 4.)
                            .color(move |d| d.color(color)),
                        true,
                        cx,
                    ))
                    .child(chart_container(
                        "Pie Chart - Pad Angle",
                        PieChart::new(self.monthly_devices.clone())
                            .value(|d| d.desktop as f32)
                            .inner_radius(60.)
                            .outer_radius(100.)
                            .pad_angle(4. / 100.)
                            .color(move |d| d.color(color)),
                        true,
                        cx,
                    )),
            )
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_4()
                    .child(chart_container(
                        "Bar Chart",
                        BarChart::new(self.monthly_devices.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.desktop),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart - Mixed",
                        BarChart::new(self.monthly_devices.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.desktop)
                            .fill(move |d, _, _, _| d.color(color)),
                        false,
                        cx,
                    ))
                    .child({
                        let data = self.daily_devices.iter().take(8).cloned().collect();
                        chart_container(
                            "Bar Chart - Stacked",
                            StackedBarChart::new(data),
                            false,
                            cx,
                        )
                    })
                    .child(chart_container(
                        "Bar Chart - Rounded corners",
                        BarChart::new(self.monthly_devices.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.desktop)
                            .label(|d| d.desktop.to_string())
                            .corner_radii(px(8.)),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart - Bottom aligned",
                        BarChart::new(self.monthly_devices.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.desktop)
                            .label(|d| d.desktop.to_string()),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart - Top aligned",
                        BarChart::new(self.monthly_devices.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.desktop)
                            .label(|d| d.desktop.to_string())
                            .alignment(BarAlignment::Top),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart - Left aligned",
                        BarChart::new(self.monthly_devices.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.desktop)
                            .label(|d| d.desktop.to_string())
                            .alignment(BarAlignment::Left),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart - Right aligned",
                        BarChart::new(self.monthly_devices.clone())
                            .band(|d| d.month.clone())
                            .value(|d| d.desktop)
                            .label(|d| d.desktop.to_string())
                            .alignment(BarAlignment::Right),
                        false,
                        cx,
                    ))
                    .child({
                        let c = cx.theme().chart_1;
                        chart_container(
                            "Bar Chart - Gradient (Bottom)",
                            BarChart::new(self.monthly_devices.clone())
                                .band(|d| d.month.clone())
                                .value(|d| d.desktop)
                                .label(|d| d.desktop.to_string())
                                .fill_gradient(move |_, chart_range, chart_to_bar| {
                                    [
                                        linear_color_stop(
                                            c.opacity(0.3),
                                            chart_to_bar(*chart_range.start()),
                                        ),
                                        linear_color_stop(c, chart_to_bar(*chart_range.end())),
                                    ]
                                }),
                            false,
                            cx,
                        )
                    })
                    .child({
                        let c = cx.theme().chart_1;
                        chart_container(
                            "Bar Chart - Gradient (Top)",
                            BarChart::new(self.monthly_devices.clone())
                                .band(|d| d.month.clone())
                                .value(|d| d.desktop)
                                .label(|d| d.desktop.to_string())
                                .alignment(BarAlignment::Top)
                                .fill_gradient(move |_, chart_range, chart_to_bar| {
                                    [
                                        linear_color_stop(
                                            c.opacity(0.3),
                                            chart_to_bar(*chart_range.start()),
                                        ),
                                        linear_color_stop(c, chart_to_bar(*chart_range.end())),
                                    ]
                                }),
                            false,
                            cx,
                        )
                    })
                    .child({
                        let c = cx.theme().chart_1;
                        chart_container(
                            "Bar Chart - Gradient (Left)",
                            BarChart::new(self.monthly_devices.clone())
                                .band(|d| d.month.clone())
                                .value(|d| d.desktop)
                                .label(|d| d.desktop.to_string())
                                .alignment(BarAlignment::Left)
                                .fill_gradient(move |_, chart_range, chart_to_bar| {
                                    [
                                        linear_color_stop(
                                            c.opacity(0.3),
                                            chart_to_bar(*chart_range.start()),
                                        ),
                                        linear_color_stop(c, chart_to_bar(*chart_range.end())),
                                    ]
                                }),
                            false,
                            cx,
                        )
                    })
                    .child({
                        let c = cx.theme().chart_1;
                        chart_container(
                            "Bar Chart - Gradient (Right)",
                            BarChart::new(self.monthly_devices.clone())
                                .band(|d| d.month.clone())
                                .value(|d| d.desktop)
                                .label(|d| d.desktop.to_string())
                                .alignment(BarAlignment::Right)
                                .fill_gradient(move |_, chart_range, chart_to_bar| {
                                    [
                                        linear_color_stop(
                                            c.opacity(0.3),
                                            chart_to_bar(*chart_range.start()),
                                        ),
                                        linear_color_stop(c, chart_to_bar(*chart_range.end())),
                                    ]
                                }),
                            false,
                            cx,
                        )
                    })
                    .child({
                        let c = cx.theme().chart_1;
                        chart_container(
                            "Bar Chart - Gradient (Per-bar)",
                            BarChart::new(self.monthly_devices.clone())
                                .band(|d| d.month.clone())
                                .value(|d| d.desktop)
                                .label(|d| d.desktop.to_string())
                                .fill_gradient(move |_, _, _| {
                                    [
                                        linear_color_stop(c.opacity(0.3), 0.),
                                        linear_color_stop(c, 1.),
                                    ]
                                }),
                            false,
                            cx,
                        )
                    })
                    .child({
                        let c1 = cx.theme().chart_1;
                        let c2 = cx.theme().chart_5;
                        chart_container(
                            "Bar Chart - Gradient (Diagonal, across bars)",
                            BarChart::new(self.monthly_devices.clone())
                                .band(|d| d.month.clone())
                                .value(|d| d.desktop)
                                .label(|d| d.desktop.to_string())
                                .fill(move |_, bar, chart, _| {
                                    // Project the bar's corners onto the chart's
                                    // bottom-left → top-right diagonal so each bar
                                    // shows the slice of a chart-wide diagonal
                                    // gradient corresponding to its own footprint.
                                    let w = chart.size.width.max(f32::EPSILON);
                                    let h = chart.size.height.max(f32::EPSILON);
                                    let denom = w * w + h * h;
                                    let project = |x: f32, y: f32| -> f32 {
                                        (x * w + (h - y) * h) / denom
                                    };
                                    let lo = project(bar.origin.x, bar.origin.y + bar.size.height);
                                    let hi = project(bar.origin.x + bar.size.width, bar.origin.y);
                                    let lerp = |t: f32| Hsla {
                                        h: c1.h + (c2.h - c1.h) * t,
                                        s: c1.s + (c2.s - c1.s) * t,
                                        l: c1.l + (c2.l - c1.l) * t,
                                        a: c1.a + (c2.a - c1.a) * t,
                                    };
                                    linear_gradient(
                                        45.,
                                        linear_color_stop(lerp(lo), 0.),
                                        linear_color_stop(lerp(hi), 1.),
                                    )
                                }),
                            false,
                            cx,
                        )
                    }),
            )
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_4()
                    .child(chart_container(
                        "Line Chart",
                        LineChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Line Chart - Linear",
                        LineChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .linear(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Line Chart - Step After",
                        LineChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .step_after(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Line Chart - Dots",
                        LineChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .dot()
                            .stroke(cx.theme().chart_5),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_4()
                    .child(chart_container(
                        "Area Chart",
                        AreaChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Area Chart - Linear",
                        AreaChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .linear(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Area Chart - Step After",
                        AreaChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .step_after(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Area Chart - Linear Gradient",
                        AreaChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .fill(linear_gradient(
                                0.,
                                linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
                                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
                            )),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_4()
                    .child(chart_container(
                        "Candlestick Chart",
                        CandlestickChart::new(self.stock_prices.clone())
                            .x(|d| d.date.clone())
                            .open(|d| d.open)
                            .high(|d| d.high)
                            .low(|d| d.low)
                            .close(|d| d.close),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Candlestick Chart - Narrow",
                        CandlestickChart::new(self.stock_prices.clone())
                            .x(|d| d.date.clone())
                            .open(|d| d.open)
                            .high(|d| d.high)
                            .low(|d| d.low)
                            .close(|d| d.close)
                            .body_width_ratio(0.5),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Candlestick Chart - Wide",
                        CandlestickChart::new(self.stock_prices.clone())
                            .x(|d| d.date.clone())
                            .open(|d| d.open)
                            .high(|d| d.high)
                            .low(|d| d.low)
                            .close(|d| d.close)
                            .body_width_ratio(1.0),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Candlestick Chart - Tick Margin",
                        CandlestickChart::new(self.stock_prices.clone())
                            .x(|d| d.date.clone())
                            .open(|d| d.open)
                            .high(|d| d.high)
                            .low(|d| d.low)
                            .close(|d| d.close)
                            .tick_margin(2),
                        false,
                        cx,
                    )),
            )
    }
}
