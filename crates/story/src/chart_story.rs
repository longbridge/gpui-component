use gpui::{
    div, linear_color_stop, linear_gradient, prelude::FluentBuilder, px, App, AppContext, Context,
    Entity, FocusHandle, Focusable, Hsla, IntoElement, ParentElement, Render, SharedString, Styled,
    Window,
};
use gpui_component::{
    chart::{AreaChart, BarChart, LineChart, PieChart},
    divider::Divider,
    dock::PanelControl,
    h_flex, v_flex, ActiveTheme, StyledExt,
};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
struct MonthlyDevice {
    pub month: SharedString,
    pub desktop: f64,
    pub color: Hsla,
}

#[derive(Clone, Deserialize)]
struct DailyDevice {
    pub date: SharedString,
    pub desktop: f64,
    pub mobile: f64,
}

pub struct ChartStory {
    focus_handle: FocusHandle,
    daily_devices: Vec<DailyDevice>,
    monthly_devices: Vec<MonthlyDevice>,
}

impl ChartStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let daily_devices =
            serde_json::from_str::<Vec<DailyDevice>>(include_str!("fixtures/daily-devices.json"))
                .unwrap();
        let monthly_devices = serde_json::from_str::<Vec<MonthlyDevice>>(include_str!(
            "fixtures/monthly-devices.json"
        ))
        .unwrap();

        Self {
            daily_devices,
            monthly_devices,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for ChartStory {
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
        .h_full()
        .border_1()
        .border_color(cx.theme().border)
        .rounded_lg()
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
        v_flex()
            .size_full()
            .gap_y_4()
            .bg(cx.theme().background)
            .child(
                div().h(px(400.)).child(chart_container(
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
                    .gap_x_8()
                    .h(px(450.))
                    .child(chart_container(
                        "Pie Chart",
                        PieChart::new(self.monthly_devices.clone())
                            .value(|d| d.desktop)
                            .outer_radius(100.)
                            .color(|d| d.color),
                        true,
                        cx,
                    ))
                    .child(chart_container(
                        "Pie Chart - Donut",
                        PieChart::new(self.monthly_devices.clone())
                            .value(|d| d.desktop)
                            .outer_radius(100.)
                            .inner_radius(60.)
                            .color(|d| d.color),
                        true,
                        cx,
                    ))
                    .child(chart_container(
                        "Pie Chart - Pad Angle",
                        PieChart::new(self.monthly_devices.clone())
                            .value(|d| d.desktop)
                            .outer_radius(100.)
                            .inner_radius(60.)
                            .pad_angle(4. / 100.)
                            .color(|d| d.color),
                        true,
                        cx,
                    )),
            )
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
                    .child(chart_container(
                        "Bar Chart",
                        BarChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart - Mixed",
                        BarChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .fill(|d| d.color),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart - Label",
                        BarChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .label(|d| d.desktop.to_string()),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
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
                        "Line Chart - Dots",
                        LineChart::new(self.monthly_devices.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.desktop)
                            .dot(),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
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
    }
}
