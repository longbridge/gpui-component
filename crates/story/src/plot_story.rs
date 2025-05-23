use gpui::{
    div, linear_color_stop, linear_gradient, prelude::FluentBuilder, px, rgb, rgba, App,
    AppContext, Bounds, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Pixels, Render, Styled, Window,
};
use gpui_component::{
    divider::Divider,
    dock::PanelControl,
    h_flex,
    plot::{
        scale::{Scale, ScaleBand, ScaleLinear, ScalePoint},
        shape::{Arc, Area, Bar, Line, Pie},
        Grid, IntoPlot, Plot, StrokeStyle,
    },
    v_flex, ActiveTheme, StyledExt,
};

#[derive(Clone)]
struct DataItem {
    month: &'static str,
    desktop: f64,
    color: u32,
}

const CHART_DATA: [DataItem; 6] = [
    DataItem {
        month: "January",
        desktop: 186.,
        color: 0x2a9d90,
    },
    DataItem {
        month: "February",
        desktop: 305.,
        color: 0xe76e50,
    },
    DataItem {
        month: "March",
        desktop: 237.,
        color: 0x274754,
    },
    DataItem {
        month: "April",
        desktop: 73.,
        color: 0xe8c468,
    },
    DataItem {
        month: "May",
        desktop: 209.,
        color: 0xf4a462,
    },
    DataItem {
        month: "June",
        desktop: 214.,
        color: 0x2563eb,
    },
];

fn grid(bounds: &Bounds<Pixels>, window: &mut Window) {
    let height = bounds.size.height.to_f64();
    Grid::new()
        .y((0..=3).map(|i| height * i as f64 / 4.0).collect())
        .solid(rgb(0xf0f0f0))
        .paint(&bounds, window);
}

#[derive(IntoPlot)]
struct AreaChart {
    stroke_style: StrokeStyle,
    linear_gradient: bool,
}

impl AreaChart {
    pub fn new() -> Self {
        Self {
            stroke_style: Default::default(),
            linear_gradient: false,
        }
    }

    pub fn linear_gradient(mut self) -> Self {
        self.linear_gradient = true;
        self
    }

    pub fn linear(mut self) -> Self {
        self.stroke_style = StrokeStyle::Linear;
        self
    }
}

impl Plot for AreaChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let width = bounds.size.width.to_f64();
        let height = bounds.size.height.to_f64();

        let x = ScalePoint::new(
            CHART_DATA.iter().map(|v| v.month).collect(),
            vec![0., width],
        );

        let y_max = CHART_DATA
            .iter()
            .map(|v| v.desktop)
            .reduce(f64::max)
            .unwrap_or(0.);
        let y = ScaleLinear::new([0., y_max].to_vec(), vec![0., height]);

        let color = if self.linear_gradient {
            linear_gradient(
                0.,
                linear_color_stop(rgba(0x2563eb66), 1.),
                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
            )
        } else {
            rgba(0x2563eb66).into()
        };

        grid(&bounds, window);
        Area::new()
            .data(&CHART_DATA)
            .x(move |d| x.tick(&d.month))
            .y0(height)
            .y1(move |d| y.tick(&d.desktop))
            .fill(color)
            .stroke(rgb(0x2563eb))
            .stroke_style(self.stroke_style)
            .paint(&bounds, window);
    }
}

#[derive(IntoPlot)]
struct BarChart;

impl Plot for BarChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let width = bounds.size.width.to_f64();
        let height = bounds.size.height.to_f64();

        let x = ScaleBand::new(
            CHART_DATA.iter().map(|v| v.month).collect(),
            vec![0., width],
        )
        .padding_inner(0.4)
        .padding_outer(0.2);

        let y_max = CHART_DATA
            .iter()
            .map(|v| v.desktop)
            .reduce(f64::max)
            .unwrap_or(0.);
        let y = ScaleLinear::new([0., y_max].to_vec(), vec![0., height]);

        grid(&bounds, window);
        Bar::new()
            .data(&CHART_DATA)
            .band_width(x.band_width())
            .x(move |d| x.tick(&d.month))
            .y0(height)
            .y1(move |d| y.tick(&d.desktop))
            .fill(move |_| rgb(0x2563eb))
            .paint(&bounds, window, cx);
    }
}

#[derive(IntoPlot)]
struct LineChart {
    stroke_style: StrokeStyle,
    point: bool,
}

impl LineChart {
    pub fn new() -> Self {
        Self {
            stroke_style: Default::default(),
            point: false,
        }
    }

    pub fn linear(mut self) -> Self {
        self.stroke_style = StrokeStyle::Linear;
        self
    }

    pub fn point(mut self) -> Self {
        self.point = true;
        self
    }
}

impl Plot for LineChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, _: &mut App) {
        let width = bounds.size.width.to_f64();
        let height = bounds.size.height.to_f64();

        let x = ScalePoint::new(
            CHART_DATA.iter().map(|v| v.month).collect(),
            vec![0., width],
        );

        let y_max = CHART_DATA
            .iter()
            .map(|v| v.desktop)
            .reduce(f64::max)
            .unwrap_or(0.);
        let y = ScaleLinear::new([0., y_max].to_vec(), vec![0., height]);

        let mut line = Line::new()
            .data(&CHART_DATA)
            .x(move |d| x.tick(&d.month))
            .y(move |d| y.tick(&d.desktop))
            .stroke(rgb(0x2563eb))
            .stroke_style(self.stroke_style)
            .stroke_width(2.);

        if self.point {
            line = line.point().point_size(8.).point_fill_color(rgb(0x2563eb));
        }

        grid(&bounds, window);
        line.paint(&bounds, window);
    }
}

#[derive(IntoPlot)]
struct PieChart {
    donut: bool,
    pad_angle: bool,
}

impl PieChart {
    pub fn new() -> Self {
        Self {
            donut: false,
            pad_angle: false,
        }
    }

    pub fn donut(mut self) -> Self {
        self.donut = true;
        self
    }

    pub fn pad_angle(mut self) -> Self {
        self.pad_angle = true;
        self
    }
}

impl Plot for PieChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, _: &mut App) {
        let radius = bounds.size.height.to_f64() * 0.4;
        let inner_radius = if self.donut { radius * 0.8 } else { 0. };
        let arc = Arc::new().inner_radius(inner_radius).outer_radius(radius);
        let mut pie = Pie::<DataItem>::new().value(|d| Some(d.desktop));
        if self.pad_angle {
            pie = pie.pad_angle(4. / radius);
        }
        let arcs = pie.arcs(&CHART_DATA);

        for a in &arcs {
            arc.paint(a, rgb(a.data.color), &bounds, window);
        }
    }
}

pub struct PlotStory {
    focus_handle: gpui::FocusHandle,
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
        "Plot"
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
                .child("January-June 2024"),
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
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
                    .child(chart_container("Area Chart", AreaChart::new(), false, cx))
                    .child(chart_container(
                        "Area Chart - Linear",
                        AreaChart::new().linear(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Area Chart - Linear Gradient",
                        AreaChart::new().linear_gradient(),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal().my_6())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
                    .child(chart_container("Line Chart", LineChart::new(), false, cx))
                    .child(chart_container(
                        "Line Chart - Linear",
                        LineChart::new().linear(),
                        false,
                        cx,
                    ))
                    .child(chart_container(
                        "Line Chart - Dots",
                        LineChart::new().point(),
                        false,
                        cx,
                    )),
            )
            .child(Divider::horizontal().my_6())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(400.))
                    .child(chart_container("Bar Chart", BarChart, false, cx))
                    .child(chart_container("Bar Chart", BarChart, false, cx))
                    .child(chart_container("Bar Chart", BarChart, false, cx)),
            )
            .child(Divider::horizontal().my_6())
            .child(
                h_flex()
                    .gap_x_8()
                    .h(px(450.))
                    .child(chart_container("Pie Chart", PieChart::new(), true, cx))
                    .child(chart_container(
                        "Pie Chart - Donut",
                        PieChart::new().donut(),
                        true,
                        cx,
                    ))
                    .child(chart_container(
                        "Pie Chart - Pad Angle",
                        PieChart::new().donut().pad_angle(),
                        true,
                        cx,
                    )),
            )
    }
}
