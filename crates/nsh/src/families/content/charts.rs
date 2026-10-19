//! NSH declarations and native GPUI charts share typed, transactional data.
use gpui_kit::component::chart::{
    AreaChart, BarChart, CandlestickChart, LineChart, PieChart, RadarChart, SankeyChart,
};
use gpui_kit::{
    AnyElement, App, IntoElement, ObjectFit, SharedString, Styled, StyledImage, Window, div, img,
    prelude::FluentBuilder,
};
use nsh_core::ui::{Children, Either, Event, JsObject, PropEnum, PropError, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct ChartDatum {
    label: Option<String>,
    x: Option<String>,
    value: Option<f64>,
    y: Option<f64>,
}
impl ChartDatum {
    fn label(&self) -> SharedString {
        self.label
            .as_ref()
            .or(self.x.as_ref())
            .cloned()
            .unwrap_or_default()
            .into()
    }
    fn number(&self) -> f64 {
        self.value.or(self.y).unwrap_or_default()
    }
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct CandlestickDatum {
    label: Option<String>,
    x: Option<String>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}
impl CandlestickDatum {
    fn label(&self) -> SharedString {
        self.label
            .as_ref()
            .or(self.x.as_ref())
            .cloned()
            .unwrap_or_default()
            .into()
    }
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SankeyChartLink {
    source: usize,
    target: usize,
    value: f64,
}
#[derive(Clone, Copy, PartialEq, PropEnum, Default)]
pub(crate) enum ChartCurve {
    #[default]
    Natural,
    Linear,
    StepAfter,
}
#[derive(Clone, Copy, PartialEq, PropEnum, Default)]
pub(crate) enum ChartBarAlignment {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}
#[derive(Clone, Copy, PartialEq, PropEnum, Default)]
pub(crate) enum ImageFit {
    #[default]
    Contain,
    Cover,
    Fill,
    None,
}
macro_rules! chart_props {
 ($name:ident,$datum:ty,{$($fields:tt)*})=>{
 #[derive(Clone,PartialEq,Props)]
 pub(crate) struct $name {children:Children,#[prop(default)] data:Vec<$datum>,#[prop(default=true)] interactive:bool,#[prop(default=true)] appear:bool,$($fields)*}
 };
}
macro_rules! line_props {
 ($name:ident,{$($extra:tt)*})=>{chart_props!($name,ChartDatum,{
 #[prop(default=true)] grid:bool,#[prop(default=true)] x_axis:bool,#[prop(default=true)] y_axis:bool,#[prop(default)] custom_tooltip:bool,#[prop(default)] curve:ChartCurve,appear_key:Option<Either<String,f64>>,y_domain:Option<[f64;2]>,reference_line:Option<f64>,#[prop(default=1)] tick_margin:usize,#[prop(default)] grid_dashed:bool,$($extra)*
 });};
}
line_props!(LineChartProps,{#[prop(default)] dot:bool,});
line_props!(AreaChartProps, {});
chart_props!(BarChartProps,ChartDatum,{
 #[prop(default=true)] grid:bool,#[prop(default=true)] x_axis:bool,#[prop(default=true)] y_axis:bool,#[prop(default)] custom_tooltip:bool,#[prop(default)] grid_dashed:bool,#[prop(default)] alignment:ChartBarAlignment,#[prop(default=0.1)] padding_inner:f64,#[prop(default=0.1)] padding_outer:f64,
});
chart_props!(PieChartProps,ChartDatum,{
 #[prop(default)] inner_radius:f64,#[prop(default=0.8)] outer_radius:f64,#[prop(default)] pad_angle:f64,
});
chart_props!(RadarChartProps,ChartDatum,{#[prop(default=true)] grid:bool,#[prop(default)] custom_tooltip:bool,});
chart_props!(CandlestickChartProps,CandlestickDatum,{#[prop(default=true)] grid:bool,#[prop(default)] custom_tooltip:bool,});
chart_props!(SankeyChartProps,ChartDatum,{#[prop(default)] links:Vec<SankeyChartLink>,});
#[derive(Clone, PartialEq, Props)]
pub(crate) struct ImageProps {
    #[prop(default)]
    src: String,
    #[prop(default)]
    fit: ImageFit,
}
#[derive(Clone, Event)]
pub(crate) enum ChartEvent {
    Hover(ChartDatum),
    // Typed validation reports errors through the native host using this declaration.
    #[allow(dead_code)]
    Error(String),
}
#[derive(Clone, Event)]
pub(crate) enum CandlestickChartEvent {
    Hover(CandlestickDatum),
    // Typed validation reports errors through the native host using this declaration.
    #[allow(dead_code)]
    Error(String),
}
#[derive(Clone, Event)]
pub(crate) enum StaticChartEvent {
    // Typed validation reports errors through the native host using this declaration.
    #[allow(dead_code)]
    Error(String),
}
fn finite(value: f64) -> bool {
    value.is_finite() && value.abs() <= f32::MAX as f64
}
fn validate_data(data: &[ChartDatum]) -> Result<(), PropError> {
    if data
        .iter()
        .any(|datum| !datum.value.or(datum.y).is_some_and(finite))
    {
        Err(PropError(
            "Chart data requires a finite value or y within the native numeric range".into(),
        ))
    } else {
        Ok(())
    }
}
fn validate_line(
    data: &[ChartDatum],
    domain: Option<[f64; 2]>,
    reference: Option<f64>,
) -> Result<(), PropError> {
    validate_data(data)?;
    if domain.is_some_and(|[min, max]| !finite(min) || !finite(max) || min >= max)
        || reference.is_some_and(|value| !finite(value))
    {
        return Err(PropError(
            "Chart domain must be finite and increasing; referenceLine must be finite".into(),
        ));
    }
    Ok(())
}
fn appear_key(value: &Option<Either<String, f64>>) -> Option<String> {
    value.as_ref().map(|value| match value {
        Either::Left(value) => format!("string:{value}"),
        Either::Right(value) => format!("number:{value}"),
    })
}
macro_rules! tooltip {
    ($chart:expr,$owner:ident,$p:ident,$event:ty) => {
        $chart.when($p.custom_tooltip, |chart| {
            let context = $owner.context.clone();
            let hovered = $owner.hovered.clone();
            chart.tooltip_content(move |datum, _, _| {
                if hovered.borrow().as_ref() != Some(datum) {
                    *hovered.borrow_mut() = Some(datum.clone());
                    context.emit(<$event>::Hover(datum.clone()));
                }
                crate::bridge::slot(&context, "tooltip").unwrap_or_else(|| div().into_any_element())
            })
        })
    };
}
macro_rules! chart_control {
    ($name:ident,$props:ty,$datum:ty,$event:ty,$owner:ident,$p:ident,$render:expr,$validate:expr) => {
        pub(crate) struct $name {
            context: TypedComponentContext<$event>,
            hovered: Rc<RefCell<Option<$datum>>>,
        }
        impl $name {
            fn new(context: TypedComponentContext<$event>) -> Self {
                Self {
                    context,
                    hovered: Rc::default(),
                }
            }
        }
        impl TypedNativeComponent for $name {
            type Props = $props;
            type Event = $event;
            fn validate_props(&self, $p: &$props) -> Result<(), PropError> {
                $validate
            }
            fn render(&mut self, $p: &$props, _: &mut Window, _: &mut App) -> Option<AnyElement> {
                let $owner = self;
                let _ = (&$owner.context, &$owner.hovered);
                Some(($render).into_any_element())
            }
        }
    };
}
chart_control!(
    NativeLineChart,
    LineChartProps,
    ChartDatum,
    ChartEvent,
    owner,
    p,
    tooltip!(LineChart::new(p.data.clone()), owner, p, ChartEvent)
        .id("chart")
        .x(ChartDatum::label)
        .y(ChartDatum::number)
        .interactive(p.interactive)
        .appear(p.appear)
        .grid(p.grid)
        .x_axis(p.x_axis)
        .y_axis(p.y_axis)
        .appear_key(appear_key(&p.appear_key))
        .map(|chart| match p.curve {
            ChartCurve::Natural => chart.natural(),
            ChartCurve::Linear => chart.linear(),
            ChartCurve::StepAfter => chart.step_after(),
        })
        .when_some(p.y_domain, |chart, [min, max]| chart.y_domain(min, max))
        .when_some(p.reference_line, |chart, value| chart.reference_line(value))
        .tick_margin(p.tick_margin)
        .grid_dashed(p.grid_dashed)
        .when(p.dot, |chart| chart.dot()),
    validate_line(&p.data, p.y_domain, p.reference_line)
);
chart_control!(
    NativeAreaChart,
    AreaChartProps,
    ChartDatum,
    ChartEvent,
    owner,
    p,
    tooltip!(AreaChart::new(p.data.clone()), owner, p, ChartEvent)
        .id("chart")
        .x(ChartDatum::label)
        .y(ChartDatum::number)
        .interactive(p.interactive)
        .appear(p.appear)
        .grid(p.grid)
        .x_axis(p.x_axis)
        .y_axis(p.y_axis)
        .appear_key(appear_key(&p.appear_key))
        .map(|chart| match p.curve {
            ChartCurve::Natural => chart.natural(),
            ChartCurve::Linear => chart.linear(),
            ChartCurve::StepAfter => chart.step_after(),
        })
        .when_some(p.y_domain, |chart, [min, max]| chart.y_domain(min, max))
        .when_some(p.reference_line, |chart, value| chart.reference_line(value))
        .tick_margin(p.tick_margin)
        .grid_dashed(p.grid_dashed),
    validate_line(&p.data, p.y_domain, p.reference_line)
);
chart_control!(
    NativeBarChart,
    BarChartProps,
    ChartDatum,
    ChartEvent,
    owner,
    p,
    tooltip!(BarChart::new(p.data.clone()), owner, p, ChartEvent)
        .id("chart")
        .band(ChartDatum::label)
        .value(ChartDatum::number)
        .interactive(p.interactive)
        .appear(p.appear)
        .grid(p.grid)
        .label_axis(p.x_axis)
        .value_axis(p.y_axis)
        .alignment(match p.alignment {
            ChartBarAlignment::Bottom => gpui_kit::component::plot::shape::BarAlignment::Bottom,
            ChartBarAlignment::Top => gpui_kit::component::plot::shape::BarAlignment::Top,
            ChartBarAlignment::Left => gpui_kit::component::plot::shape::BarAlignment::Left,
            ChartBarAlignment::Right => gpui_kit::component::plot::shape::BarAlignment::Right,
        })
        .padding_inner(p.padding_inner as f32)
        .padding_outer(p.padding_outer as f32)
        .grid_dashed(p.grid_dashed),
    {
        validate_data(&p.data)?;
        if !finite(p.padding_inner)
            || !finite(p.padding_outer)
            || p.padding_inner < 0.
            || p.padding_outer < 0.
        {
            Err(PropError(
                "Bar chart padding must be nonnegative and finite".into(),
            ))
        } else {
            Ok(())
        }
    }
);
chart_control!(
    NativePieChart,
    PieChartProps,
    ChartDatum,
    StaticChartEvent,
    owner,
    p,
    PieChart::new(p.data.clone())
        .id("chart")
        .value(|datum| datum.number() as f32)
        .label(ChartDatum::label)
        .inner_radius(p.inner_radius as f32)
        .outer_radius(p.outer_radius as f32)
        .pad_angle(p.pad_angle as f32)
        .interactive(p.interactive)
        .appear(p.appear),
    {
        validate_data(&p.data)?;
        if !(0. ..=1.).contains(&p.inner_radius)
            || !(0. ..=1.).contains(&p.outer_radius)
            || !finite(p.pad_angle)
        {
            Err(PropError(
                "Pie radii must be within [0,1] and padAngle finite".into(),
            ))
        } else {
            Ok(())
        }
    }
);
chart_control!(
    NativeRadarChart,
    RadarChartProps,
    ChartDatum,
    ChartEvent,
    owner,
    p,
    tooltip!(RadarChart::new(p.data.clone()), owner, p, ChartEvent)
        .id("chart")
        .value(ChartDatum::number)
        .label(ChartDatum::label)
        .interactive(p.interactive)
        .appear(p.appear)
        .grid(p.grid),
    validate_data(&p.data)
);
chart_control!(
    NativeCandlestickChart,
    CandlestickChartProps,
    CandlestickDatum,
    CandlestickChartEvent,
    owner,
    p,
    tooltip!(
        CandlestickChart::new(p.data.clone()),
        owner,
        p,
        CandlestickChartEvent
    )
    .id("chart")
    .x(CandlestickDatum::label)
    .open(|datum| datum.open)
    .high(|datum| datum.high)
    .low(|datum| datum.low)
    .close(|datum| datum.close)
    .interactive(p.interactive)
    .appear(p.appear)
    .grid(p.grid),
    if p.data
        .iter()
        .any(|datum| [datum.open, datum.high, datum.low, datum.close]
            .iter()
            .any(|value| !finite(*value)))
    {
        Err(PropError(
            "Candlestick OHLC values must be finite and within native numeric range".into(),
        ))
    } else {
        Ok(())
    }
);
chart_control!(
    NativeSankeyChart,
    SankeyChartProps,
    ChartDatum,
    StaticChartEvent,
    owner,
    p,
    SankeyChart::new(
        p.data.clone(),
        p.links
            .iter()
            .map(|link| gpui_kit::component::plot::shape::SankeyLink::new(
                link.source,
                link.target,
                link.value
            ))
    )
    .id("chart")
    .node_label(ChartDatum::label)
    .interactive(p.interactive)
    .appear(p.appear),
    if p.links.iter().any(|link| link.value < 0.
        || !finite(link.value)
        || link.source >= p.data.len()
        || link.target >= p.data.len())
    {
        Err(PropError(
            "Sankey links require valid node indices and finite nonnegative values".into(),
        ))
    } else {
        Ok(())
    }
);
pub(crate) struct NativeImage;
impl TypedNativeComponent for NativeImage {
    type Props = ImageProps;
    type Event = nsh_core::ui::NoEvent;
    fn render(&mut self, p: &ImageProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            img(p.src.clone())
                .size_full()
                .object_fit(match p.fit {
                    ImageFit::Contain => ObjectFit::Contain,
                    ImageFit::Cover => ObjectFit::Cover,
                    ImageFit::Fill => ObjectFit::Fill,
                    ImageFit::None => ObjectFit::None,
                })
                .into_any_element(),
        )
    }
}
pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeLineChart>(
        "gpui-component.LineChart",
        "LineChart",
        NativeLineChart::new,
    );
    registry.register::<NativeAreaChart>(
        "gpui-component.AreaChart",
        "AreaChart",
        NativeAreaChart::new,
    );
    registry.register::<NativeBarChart>("gpui-component.BarChart", "BarChart", NativeBarChart::new);
    registry.register::<NativePieChart>("gpui-component.PieChart", "PieChart", NativePieChart::new);
    registry.register::<NativeRadarChart>(
        "gpui-component.RadarChart",
        "RadarChart",
        NativeRadarChart::new,
    );
    registry.register::<NativeCandlestickChart>(
        "gpui-component.CandlestickChart",
        "CandlestickChart",
        NativeCandlestickChart::new,
    );
    registry.register::<NativeSankeyChart>(
        "gpui-component.SankeyChart",
        "SankeyChart",
        NativeSankeyChart::new,
    );
    registry.register::<NativeImage>("gpui-component.Image", "Image", |_| NativeImage);
}
