//! Concrete chart bindings backed by immutable script data snapshots.
//!
//! `gpui_component::plot::Plot` is deliberately not registered: it is a Rust
//! painting trait implemented by concrete chart elements, not a constructible
//! component surface.

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDataValue, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, RenderOnce, Styled as _},
    gpui_component::chart::{AreaChart, BarChart, LineChart, PieChart, RadarChart},
};
use std::sync::Arc;

#[derive(Clone)]
struct Payload(ComponentArgument);

#[derive(Clone)]
struct Row {
    label: String,
    value: f64,
}

#[derive(Clone, Copy)]
enum Op {
    Grid(bool),
    Dot,
    Natural,
    Linear,
    StepAfter,
    TickMargin(usize),
    Axis(bool),
    ValueAxis(bool),
    InnerRadius(f32),
    PadAngle(f32),
    Labels(bool),
    GridLevels(usize),
}

fn field<'a>(
    fields: &'a [(String, ComponentDataValue)],
    name: &str,
) -> Option<&'a ComponentDataValue> {
    fields
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn rows(
    callback: &gpui_shell::ComponentDataCallback,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> anyhow::Result<Vec<Row>> {
    let snapshot = callback.snapshot_rows_with(&[], window, cx)?;
    (0..snapshot.len())
        .map(|index| {
            let ComponentDataValue::Object(fields) = snapshot.row(index)? else {
                anyhow::bail!("chart row {index} must be a plain object")
            };
            let Some(ComponentDataValue::String(label)) = field(fields, "label") else {
                anyhow::bail!("chart row {index} must have string field `label`")
            };
            let Some(ComponentDataValue::Number(value)) = field(fields, "value") else {
                anyhow::bail!("chart row {index} must have finite number field `value`")
            };
            Ok(Row {
                label: label.clone(),
                value: *value,
            })
        })
        .collect()
}

fn wrap(
    mut request: MaterializeRequest<'_>,
    chart: impl gpui::IntoElement + 'static,
) -> anyhow::Result<gpui::AnyElement> {
    anyhow::ensure!(request.children_len() == 0, "charts do not accept children");
    anyhow::ensure!(
        request.take_children()?.is_empty(),
        "charts do not accept ordinary children"
    );
    let mut wrapper = gpui::div().size_full().child(chart);
    wrapper.style().refine(&request.take_style());
    Ok(wrapper.into_any_element())
}

#[derive(gpui::IntoElement)]
struct ChartHost {
    kind: &'static str,
    callback: gpui_shell::ComponentDataCallback,
    ops: Vec<Op>,
}
impl RenderOnce for ChartHost {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let data = match rows(&self.callback, window, cx) {
            Ok(data) => data,
            Err(error) => {
                #[cfg(test)]
                test_probe::error(error.to_string());
                return gpui::div()
                    .child(format!("Failed to build {} data: {error:#}", self.kind))
                    .into_any_element();
            }
        };
        match self.kind {
            "BarChart" => {
                let mut chart = BarChart::new(data)
                    .band(|row| row.label.clone())
                    .value(|row| row.value)
                    .label(|row| row.value.to_string());
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(v) => chart.grid(*v),
                        Op::TickMargin(v) => chart.tick_margin(*v),
                        Op::Axis(v) => chart.label_axis(*v),
                        Op::ValueAxis(v) => chart.value_axis(*v),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "LineChart" => {
                let mut chart = LineChart::new(data)
                    .x(|row| row.label.clone())
                    .y(|row| row.value);
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(v) => chart.grid(*v),
                        Op::Dot => chart.dot(),
                        Op::Natural => chart.natural(),
                        Op::Linear => chart.linear(),
                        Op::StepAfter => chart.step_after(),
                        Op::TickMargin(v) => chart.tick_margin(*v),
                        Op::Axis(v) => chart.x_axis(*v),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "AreaChart" => {
                let mut chart = AreaChart::new(data)
                    .x(|row| row.label.clone())
                    .y(|row| row.value);
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(v) => chart.grid(*v),
                        Op::Natural => chart.natural(),
                        Op::Linear => chart.linear(),
                        Op::StepAfter => chart.step_after(),
                        Op::TickMargin(v) => chart.tick_margin(*v),
                        Op::Axis(v) => chart.x_axis(*v),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "PieChart" => {
                let mut chart = PieChart::new(data).value(|row| row.value as f32);
                for op in &self.ops {
                    chart = match op {
                        Op::InnerRadius(v) => chart.inner_radius(*v),
                        Op::PadAngle(v) => chart.pad_angle(*v),
                        Op::Labels(true) => chart.label(|row| row.label.clone().into()),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "RadarChart" => {
                let mut chart = RadarChart::new(data)
                    .value(|row| row.value)
                    .label(|row| row.label.clone());
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(v) => chart.grid(*v),
                        Op::Dot => chart.dot(),
                        Op::GridLevels(v) => chart.grid_levels(*v),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            _ => unreachable!(),
        }
    }
}

struct ChartMaterializer(&'static str);
impl ComponentMaterializer for ChartMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("{} received an incompatible payload", self.0))?
            .0
            .clone();
        let callback = request.resolve_data_callback(&argument)?;
        let ops = request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<Op>().copied())
            .collect();
        wrap(
            request,
            ChartHost {
                kind: self.0,
                callback,
                ops,
            },
        )
    }
}

fn bool_method(name: &'static str, op: fn(bool) -> Op) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |args| match args {
            [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(op(*value))),
            _ => Err(format!("{name} expects boolean")),
        },
    )
}
fn positive_usize_method(name: &'static str, op: fn(usize) -> Op) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Number)],
        move |args| match args {
            [ComponentArgument::Number(value)]
                if value.is_finite()
                    && *value >= 1.
                    && value.fract() == 0.
                    && *value <= usize::MAX as f64 =>
            {
                Ok(ComponentPayload::new(op(*value as usize)))
            }
            _ => Err(format!("{name} expects a positive integer")),
        },
    )
}
fn number_method(name: &'static str, op: fn(f32) -> Op) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Number)],
        move |args| match args {
            [ComponentArgument::Number(value)]
                if value.is_finite() && *value >= 0. && *value <= f32::MAX as f64 =>
            {
                Ok(ComponentPayload::new(op(*value as f32)))
            }
            _ => Err(format!("{name} expects a non-negative finite number")),
        },
    )
}
fn flag(name: &'static str, op: Op) -> MethodDescriptor {
    MethodDescriptor::new(name, vec![], move |_| Ok(ComponentPayload::new(op)))
}

fn descriptor(name: &'static str, methods: Vec<MethodDescriptor>) -> ComponentDescriptor {
    ComponentDescriptor {
        name,
        constructors: vec![ConstructorDescriptor::new(
            name,
            vec![ArgumentDescriptor::new(
                "rows",
                ArgumentSchema::Callback(
                    "(cx: Context) => readonly { label: string; value: number }[]",
                ),
            )],
            |args| match args {
                [argument @ ComponentArgument::Callback(_)] => {
                    Ok(ComponentPayload::new(Payload(argument.clone())))
                }
                _ => Err("chart expects a row snapshot callback".into()),
            },
        )],
        methods,
        typescript: TypeScriptDescriptor::new(
            "A concrete native chart backed by one immutable plain-data snapshot per materialization. Rows require { label, value }; style applies to its full-size host.",
        ),
        materializer: Arc::new(ChartMaterializer(name)),
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(descriptor(
        "BarChart",
        vec![
            bool_method("grid", Op::Grid),
            bool_method("labelAxis", Op::Axis),
            bool_method("valueAxis", Op::ValueAxis),
            positive_usize_method("tickMargin", Op::TickMargin),
        ],
    ))?;
    registry.register(descriptor(
        "LineChart",
        vec![
            bool_method("grid", Op::Grid),
            bool_method("xAxis", Op::Axis),
            positive_usize_method("tickMargin", Op::TickMargin),
            flag("dot", Op::Dot),
            flag("natural", Op::Natural),
            flag("linear", Op::Linear),
            flag("stepAfter", Op::StepAfter),
        ],
    ))?;
    registry.register(descriptor(
        "AreaChart",
        vec![
            bool_method("grid", Op::Grid),
            bool_method("xAxis", Op::Axis),
            positive_usize_method("tickMargin", Op::TickMargin),
            flag("natural", Op::Natural),
            flag("linear", Op::Linear),
            flag("stepAfter", Op::StepAfter),
        ],
    ))?;
    registry.register(descriptor(
        "PieChart",
        vec![
            number_method("innerRadius", Op::InnerRadius),
            number_method("padAngle", Op::PadAngle),
            bool_method("labels", Op::Labels),
        ],
    ))?;
    registry.register(descriptor(
        "RadarChart",
        vec![
            bool_method("grid", Op::Grid),
            positive_usize_method("gridLevels", Op::GridLevels),
            flag("dot", Op::Dot),
        ],
    ))?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::RefCell;
    thread_local! { static ERROR: RefCell<Option<String>> = const { RefCell::new(None) }; }
    pub(super) fn error(error: String) {
        ERROR.with(|slot| *slot.borrow_mut() = Some(error));
    }
    pub(crate) fn take_error() -> Option<String> {
        ERROR.with(|slot| slot.borrow_mut().take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_contains_only_concrete_constructible_charts() {
        let mut registry =
            ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        assert_eq!(
            registry
                .freeze()
                .unwrap()
                .descriptors()
                .map(|d| d.name)
                .collect::<Vec<_>>(),
            [
                "BarChart",
                "LineChart",
                "AreaChart",
                "PieChart",
                "RadarChart"
            ]
        );
    }
}
