//! Typed low-level Plot shapes, native scales and axis configuration.
use crate::registration::named_value;
use gpui_kit::component::{
    ActiveTheme,
    plot::{
        AxisLabelSide, AxisText, Curve, Grid, PlotAxis as NativePlotAxis,
        scale::{Scale, ScaleBand, ScaleLinear, ScaleOrdinal, ScalePoint},
        shape::{Arc, Area, Bar, BarAlignment, Line, Pie},
    },
};
use gpui_kit::{AnyElement, App, Bounds, Hsla, IntoElement, Pixels, Window, point, px};
use nsh_core::ui::{
    Either, Event, JsHost, JsIn, JsObject, JsOwned, JsValue, PropEnum, PropError, Props, TsTypes,
};
use nsh_platform_gpui::TypedNativeComponent;

#[derive(Clone, Copy, PartialEq, PropEnum)]
enum PlotColorName {
    Primary,
    Foreground,
    Muted,
    Accent,
    Success,
    Danger,
    Chart1,
    Chart2,
    Chart3,
    Chart4,
    Chart5,
}
named_value! { struct PlotColor(Either<PlotColorName, u32>); }
type PlotValue = Either<f64, String>;
fn key(value: &PlotValue) -> String {
    match value {
        Either::Left(value) => value.to_string(),
        Either::Right(value) => value.clone(),
    }
}

/// A horizontal coordinate and a numeric vertical coordinate.
#[derive(Clone, PartialEq)]
struct PlotPoint {
    x: PlotValue,
    y: f64,
}
impl JsValue for PlotPoint {
    fn ts_type(types: &mut TsTypes) -> String {
        types.declare("PlotPoint", |types| {
            format!(
                "export type PlotPoint = [{}, number];",
                PlotValue::ts_type(types)
            )
        })
    }
    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        if value.array_len()? != Some(2) {
            return Err(value.expected("a two-coordinate plot point"));
        }
        let x = value.index(0)?;
        let y = value.index(1)?;
        Ok(Self {
            x: PlotValue::from_js(x.get())?,
            y: f64::from_js(y.get())?,
        })
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        let x = self.x.to_js(host)?;
        let y = self.y.to_js(host)?;
        Ok(JsOwned::new(
            host,
            host.new_array(vec![x.into_raw(), y.into_raw()])?,
        ))
    }
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum PlotLinearType {
    #[default]
    Linear,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum PlotCategoryType {
    Point,
    Band,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotLinearScale {
    #[prop(rename = "type", default)]
    scale_type: PlotLinearType,
    domain: Vec<f64>,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotCategoryScale {
    #[prop(rename = "type")]
    scale_type: PlotCategoryType,
    domain: Vec<PlotValue>,
    padding_inner: Option<f32>,
    padding_outer: Option<f32>,
}
named_value! { struct PlotScale(Either<PlotCategoryScale, PlotLinearScale>); }
#[derive(Clone, Default, PartialEq, JsObject)]
struct PlotScales {
    x: Option<PlotScale>,
    y: Option<PlotScale>,
}
#[derive(Clone, Copy, Default, PartialEq, JsObject)]
struct PlotMargins {
    left: Option<f32>,
    top: Option<f32>,
    right: Option<f32>,
    bottom: Option<f32>,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotAxisLabel {
    label: String,
    value: PlotValue,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum PlotAxisSide {
    Start,
    End,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotAxis {
    #[prop(default)]
    labels: Vec<PlotAxisLabel>,
    font_size: Option<f32>,
    position: Option<f32>,
    side: Option<PlotAxisSide>,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotAxes {
    x: Option<PlotAxis>,
    y: Option<PlotAxis>,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotGrid {
    #[prop(default)]
    x: Vec<PlotValue>,
    #[prop(default)]
    y: Vec<PlotValue>,
    dashes: Option<Vec<f32>>,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotPalette {
    domain: Vec<String>,
    range: Vec<PlotColor>,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum PlotCartesianType {
    Line,
    Area,
    Bar,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum PlotCurve {
    Natural,
    Linear,
    StepAfter,
}
impl PlotCurve {
    fn native(self) -> Curve {
        match self {
            Self::Natural => Curve::Natural,
            Self::Linear => Curve::Linear,
            Self::StepAfter => Curve::StepAfter,
        }
    }
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotCartesianSeries {
    #[prop(rename = "type")]
    series_type: PlotCartesianType,
    points: Vec<PlotPoint>,
    id: Option<String>,
    color: Option<PlotColor>,
    curve: Option<PlotCurve>,
    baseline: Option<f64>,
    opacity: Option<f32>,
    stroke_width: Option<f32>,
    band_width: Option<f32>,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum PlotPieType {
    Pie,
}
#[derive(Clone, PartialEq, JsObject)]
struct PlotPieSeries {
    #[prop(rename = "type")]
    series_type: PlotPieType,
    values: Vec<f64>,
    id: Option<String>,
    color: Option<PlotColor>,
    inner_radius: Option<f32>,
    outer_radius: Option<f32>,
    pad_angle: Option<f32>,
    start_angle: Option<f32>,
    end_angle: Option<f32>,
    colors: Option<Vec<PlotColor>>,
}
named_value! { struct PlotSeries(Either<PlotCartesianSeries, PlotPieSeries>); }
/// Native shapes and scales. Domains default to [0, 1]; lengths use logical pixels.
#[derive(Clone, PartialEq, Props)]
pub(crate) struct PlotProps {
    series: Vec<PlotSeries>,
    scales: Option<PlotScales>,
    margins: Option<PlotMargins>,
    axes: Option<PlotAxes>,
    grid: Option<PlotGrid>,
    colors: Option<PlotPalette>,
}
#[derive(Event)]
#[allow(dead_code)] // Native typed decoding reports this event on invalid values.
pub(crate) enum PlotEvent {
    Error(String),
}
struct PlotControl;
impl TypedNativeComponent for PlotControl {
    type Props = PlotProps;
    type Event = PlotEvent;
    fn render(&mut self, props: &PlotProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            NativePlot {
                props: props.clone(),
            }
            .into_any_element(),
        )
    }
}
pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<PlotControl>("gpui-component.Plot", "Plot", |_| PlotControl);
}
#[derive(gpui_kit::component::plot::IntoPlot)]
struct NativePlot {
    props: PlotProps,
}
#[derive(Clone)]
enum AxisScale {
    Linear(ScaleLinear<f64>),
    Point(ScalePoint<String>),
    Band(ScaleBand<String>),
}
impl AxisScale {
    fn new(config: Option<&PlotScale>, range: [f32; 2]) -> Self {
        match config.map(PlotScale::value) {
            Some(Either::Left(config)) => match config.scale_type {
                PlotCategoryType::Point => {
                    Self::Point(ScalePoint::new(config.domain.iter().map(key), range))
                }
                PlotCategoryType::Band => Self::Band(
                    ScaleBand::new(config.domain.iter().map(key), range)
                        .padding_inner(config.padding_inner.unwrap_or(0.1))
                        .padding_outer(config.padding_outer.unwrap_or(0.1)),
                ),
            },
            Some(Either::Right(config)) => {
                Self::Linear(ScaleLinear::new(config.domain.iter().copied(), range))
            }
            None => Self::Linear(ScaleLinear::new([0., 1.], range)),
        }
    }
    fn tick(&self, value: &PlotValue) -> Option<f32> {
        match self {
            Self::Linear(scale) => match value {
                Either::Left(value) => scale.tick(value),
                _ => None,
            },
            Self::Point(scale) => scale.tick(&key(value)),
            Self::Band(scale) => scale.tick(&key(value)),
        }
    }
    fn number_tick(&self, value: f64) -> Option<f32> {
        self.tick(&Either::Left(value))
    }
    fn band_width(&self) -> Option<f32> {
        match self {
            Self::Band(scale) => Some(scale.band_width()),
            _ => None,
        }
    }
}
fn color(value: Option<&PlotColor>, cx: &App) -> Hsla {
    let theme = cx.theme();
    match value.map(PlotColor::value) {
        Some(Either::Right(value)) => gpui_kit::rgb(*value).into(),
        Some(Either::Left(value)) => match value {
            PlotColorName::Primary => theme.primary,
            PlotColorName::Foreground => theme.foreground,
            PlotColorName::Muted => theme.muted_foreground,
            PlotColorName::Accent => theme.accent,
            PlotColorName::Success => theme.success,
            PlotColorName::Danger => theme.danger,
            PlotColorName::Chart1 => theme.chart_1,
            PlotColorName::Chart2 => theme.chart_2,
            PlotColorName::Chart3 => theme.chart_3,
            PlotColorName::Chart4 => theme.chart_4,
            PlotColorName::Chart5 => theme.chart_5,
        },
        None => theme.primary,
    }
}
impl gpui_kit::component::plot::Plot for NativePlot {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let margins = self.props.margins.unwrap_or_default();
        let left = margins.left.unwrap_or(0.).max(0.);
        let top = margins.top.unwrap_or(0.).max(0.);
        let width = (f32::from(bounds.size.width) - left - margins.right.unwrap_or(0.)).max(0.);
        let height = (f32::from(bounds.size.height) - top - margins.bottom.unwrap_or(0.)).max(0.);
        let bounds = Bounds {
            origin: bounds.origin + point(px(left), px(top)),
            size: gpui_kit::size(px(width), px(height)),
        };
        let scales = self.props.scales.as_ref();
        let x = AxisScale::new(scales.and_then(|scales| scales.x.as_ref()), [0., width]);
        let y = AxisScale::new(scales.and_then(|scales| scales.y.as_ref()), [height, 0.]);
        if let Some(grid) = &self.props.grid {
            let xs = grid.x.iter().filter_map(|value| x.tick(value));
            let ys = grid.y.iter().filter_map(|value| y.tick(value));
            let grid_element = Grid::new().x(xs).y(ys).stroke(cx.theme().border);
            let grid_element = if let Some(dashes) = &grid.dashes {
                grid_element.dash_array(&dashes.iter().map(|value| px(*value)).collect::<Vec<_>>())
            } else {
                grid_element
            };
            grid_element.paint(&bounds, window);
        }
        let palette = ScaleOrdinal::new(
            self.props
                .colors
                .as_ref()
                .map(|colors| colors.domain.clone())
                .unwrap_or_default(),
            self.props
                .colors
                .as_ref()
                .map(|colors| {
                    colors
                        .range
                        .iter()
                        .map(|value| color(Some(value), cx))
                        .collect()
                })
                .unwrap_or_default(),
        );
        for series in &self.props.series {
            match series.value() {
                Either::Left(series) => {
                    let stroke = series
                        .id
                        .as_ref()
                        .and_then(|id| palette.map(id))
                        .unwrap_or_else(|| color(series.color.as_ref(), cx));
                    let x_scale = x.clone();
                    let y_scale = y.clone();
                    let curve = series.curve.unwrap_or(PlotCurve::Natural).native();
                    let baseline = series
                        .baseline
                        .and_then(|value| y.number_tick(value))
                        .unwrap_or(height);
                    match series.series_type {
                        PlotCartesianType::Area => Area::new()
                            .data(series.points.clone())
                            .x(move |point| x_scale.tick(&point.x))
                            .y0(baseline)
                            .y1(move |point| y_scale.number_tick(point.y))
                            .fill(stroke.opacity(series.opacity.unwrap_or(0.25)))
                            .stroke(stroke)
                            .curve(curve)
                            .paint(&bounds, window),
                        PlotCartesianType::Bar => Bar::new()
                            .data(series.points.clone())
                            .cross(move |point| x_scale.tick(&point.x))
                            .value(move |point| y_scale.number_tick(point.y))
                            .base(move |_| baseline)
                            .band_width(x.band_width().unwrap_or(series.band_width.unwrap_or(12.)))
                            .alignment(BarAlignment::Bottom)
                            .fill(move |_, _, _| stroke)
                            .paint(&bounds, window, cx),
                        PlotCartesianType::Line => Line::new()
                            .data(series.points.clone())
                            .x(move |point| x_scale.tick(&point.x))
                            .y(move |point| y_scale.number_tick(point.y))
                            .stroke(stroke)
                            .stroke_width(px(series.stroke_width.unwrap_or(1.)))
                            .curve(curve)
                            .paint(&bounds, window),
                    }
                }
                Either::Right(series) => {
                    let stroke = series
                        .id
                        .as_ref()
                        .and_then(|id| palette.map(id))
                        .unwrap_or_else(|| color(series.color.as_ref(), cx));
                    let arcs = Pie::new()
                        .value(|value: &f64| Some(*value as f32))
                        .start_angle(series.start_angle.unwrap_or(0.))
                        .end_angle(series.end_angle.unwrap_or(std::f32::consts::TAU))
                        .pad_angle(series.pad_angle.unwrap_or(0.))
                        .arcs(&series.values);
                    let arc = Arc::new()
                        .inner_radius(series.inner_radius.unwrap_or(0.))
                        .outer_radius(series.outer_radius.unwrap_or(width.min(height) / 2.));
                    for (index, datum) in arcs.iter().enumerate() {
                        let fill = series
                            .colors
                            .as_ref()
                            .and_then(|colors| colors.get(index))
                            .map(|value| color(Some(value), cx))
                            .unwrap_or_else(|| {
                                let color = if series.color.is_none() {
                                    cx.theme().chart_1
                                } else {
                                    stroke
                                };
                                color.opacity((1. - index as f32 * 0.12).max(0.15))
                            });
                        arc.paint(datum, fill, &bounds, window);
                    }
                }
            }
        }
        if let Some(axes) = &self.props.axes {
            for (is_x, scale, position, config) in [
                (true, &x, height, axes.x.as_ref()),
                (false, &y, 0., axes.y.as_ref()),
            ] {
                let Some(config) = config else { continue };
                let labels = config
                    .labels
                    .iter()
                    .filter_map(|label| {
                        Some(
                            AxisText::new(
                                label.label.clone(),
                                px(scale.tick(&label.value)?),
                                cx.theme().muted_foreground,
                            )
                            .font_size(px(config.font_size.unwrap_or(10.))),
                        )
                    })
                    .collect::<Vec<_>>();
                let side = if config.side == Some(PlotAxisSide::Start) {
                    AxisLabelSide::Start
                } else {
                    AxisLabelSide::End
                };
                let axis = NativePlotAxis::new().stroke(cx.theme().border);
                let axis = if is_x {
                    axis.x(px(config.position.unwrap_or(position)))
                        .x_axis(true)
                        .x_label(labels)
                        .x_label_side(side)
                } else {
                    axis.y(px(config.position.unwrap_or(position)))
                        .y_axis(true)
                        .y_label(labels)
                        .y_label_side(side)
                };
                axis.paint(&bounds, window, cx);
            }
        }
    }
}
