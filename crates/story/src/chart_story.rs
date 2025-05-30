use gpui::{
    div, linear_color_stop, linear_gradient, prelude::FluentBuilder, px, rgb, rgba, App,
    AppContext, Context, Entity, FocusHandle, Focusable, Hsla, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};
use gpui_component::{
    chart::{AreaChart, BarChart, ChartDelegate, LineChart, PieChart, PieChartDelegate},
    divider::Divider,
    dock::PanelControl,
    h_flex, v_flex, ActiveTheme, StyledExt,
};

use crate::fixtures::{DataItem, DataItem2};

use super::fixtures::{CHART_DATA, CHART_DATA_2};

impl ChartDelegate<&'static str, f64> for DataItem {
    fn x(&self) -> &'static str {
        self.month
    }

    fn y(&self) -> f64 {
        self.desktop
    }
}

impl PieChartDelegate for DataItem {
    fn value(&self) -> Option<f64> {
        Some(self.desktop)
    }

    fn color(&self) -> impl Into<Hsla> {
        rgb(self.color)
    }
}

impl ChartDelegate<&'static str, f64> for DataItem2 {
    fn x(&self) -> &'static str {
        self.date
    }

    fn y(&self) -> f64 {
        self.desktop
    }

    fn label(&self) -> impl Into<SharedString> {
        self.date
    }
}

pub struct PlotStory {
    focus_handle: FocusHandle,
}

impl PlotStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for PlotStory {
    fn title() -> &'static str {
        "Chart"
    }

    fn description() -> &'static str {
        "A low-level approach to data analysis and visualization."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for PlotStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn chart_container(
    title: &str,
    chart: impl IntoElement,
    center: bool,
    cx: &mut Context<PlotStory>,
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

impl Render for PlotStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_y_4()
            .bg(cx.theme().background)
            .child(
                div().h(px(400.)).child(chart_container(
                    "Area Chart - Stacked",
                    AreaChart::new(CHART_DATA_2.to_vec())
                        .fill(linear_gradient(
                            0.,
                            linear_color_stop(rgba(0x2563eb66), 1.),
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
                    .h(px(400.))
                    .child(chart_container(
                        "Area Chart",
                        AreaChart::new(CHART_DATA.to_vec()),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Area Chart - Linear",
                        AreaChart::new(CHART_DATA.to_vec()).linear(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Area Chart - Linear Gradient",
                        AreaChart::new(CHART_DATA.to_vec()).fill(linear_gradient(
                            0.,
                            linear_color_stop(rgba(0x2563eb66), 1.),
                            linear_color_stop(cx.theme().background.opacity(0.3), 0.),
                        )),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal().my_6())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
                    .child(chart_container(
                        "Line Chart",
                        LineChart::new(CHART_DATA.to_vec()),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Line Chart - Linear",
                        LineChart::new(CHART_DATA.to_vec()).linear(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Line Chart - Dots",
                        LineChart::new(CHART_DATA.to_vec()).point(),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal().my_6())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
                    .child(chart_container(
                        "Bar Chart",
                        BarChart::new(CHART_DATA.to_vec()),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart",
                        BarChart::new(CHART_DATA.to_vec()),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Bar Chart",
                        BarChart::new(CHART_DATA.to_vec()),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal().my_6())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(450.))
                    .child(chart_container(
                        "Pie Chart",
                        PieChart::new(CHART_DATA.to_vec()).outer_radius(100.),
                        true,
                        cx,
                    ))
                    .child(chart_container(
                        "Pie Chart - Donut",
                        PieChart::new(CHART_DATA.to_vec())
                            .outer_radius(100.)
                            .inner_radius(60.),
                        true,
                        cx,
                    ))
                    .child(chart_container(
                        "Pie Chart - Pad Angle",
                        PieChart::new(CHART_DATA.to_vec())
                            .outer_radius(100.)
                            .inner_radius(60.)
                            .pad_angle(4. / 100.),
                        true,
                        cx,
                    )),
            )
    }
}
