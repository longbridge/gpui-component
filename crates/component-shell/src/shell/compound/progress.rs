use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor, anyhow,
    gpui::{self, IntoElement as _, ParentElement as _, Refineable as _, Styled as _},
    gpui_component::{Sizable as _, Size, progress::Progress},
};
use std::sync::Arc;

use super::common::{finite_f32, nonempty_id};

#[derive(Clone)]
struct ProgressPayload(String);
#[derive(Clone)]
enum ProgressOp {
    Value(f32),
    Loading(bool),
    Label(String),
    Size(Size),
}
struct ProgressMaterializer;
impl ProgressMaterializer {
    fn component<'a>(
        payload: &ComponentPayload,
        ops: impl IntoIterator<Item = &'a ProgressOp>,
    ) -> anyhow::Result<Progress> {
        let id = &payload
            .downcast_ref::<ProgressPayload>()
            .ok_or_else(|| anyhow::anyhow!("Progress received an incompatible payload"))?
            .0;
        Ok(ops
            .into_iter()
            .fold(Progress::new(id.clone()), |progress, op| match op {
                ProgressOp::Value(value) => progress.value(*value),
                ProgressOp::Loading(value) => progress.loading(*value),
                ProgressOp::Label(value) => progress.accessibility_label(value.clone()),
                ProgressOp::Size(value) => progress.with_size(*value),
            }))
    }
}
impl ComponentMaterializer for ProgressMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        anyhow::ensure!(
            request.children_len() == 0,
            "Progress does not accept children"
        );
        let ops = request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<ProgressOp>());
        let mut wrapper = gpui::div().child(Self::component(request.payload(), ops)?);
        wrapper.style().refine(&request.take_style());
        Ok(wrapper.into_any_element())
    }
}
fn unary(
    name: &'static str,
    schema: ArgumentSchema,
    documentation: &'static str,
    factory: fn(&ComponentArgument) -> Result<ProgressOp, String>,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, schema)],
        move |args| match args {
            [arg] => factory(arg).map(ComponentPayload::new),
            _ => Err(format!("Progress.{name}({name}) expects one argument")),
        },
    )
    .documented(documentation)
}
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor {
        name: "Progress",
        constructors: vec![ConstructorDescriptor::new(
            "Progress",
            vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
            |args| match args {
                [ComponentArgument::String(id)] => nonempty_id(id, "Progress")
                    .map(ProgressPayload)
                    .map(ComponentPayload::new),
                _ => Err("Progress(id) expects a string id".into()),
            },
        )],
        methods: vec![
            unary(
                "value",
                ArgumentSchema::Number,
                "Sets percentage progress; the component clamps it to 0–100.",
                |a| match a {
                    ComponentArgument::Number(v) => {
                        finite_f32(*v, "Progress.value(value)").map(ProgressOp::Value)
                    }
                    _ => Err(
                        "Progress.value(value) expects a finite number representable as f32".into(),
                    ),
                },
            ),
            unary(
                "loading",
                ArgumentSchema::Boolean,
                "Enables indeterminate loading animation.",
                |a| match a {
                    ComponentArgument::Boolean(v) => Ok(ProgressOp::Loading(*v)),
                    _ => Err("Progress.loading(loading) expects a boolean".into()),
                },
            ),
            unary(
                "accessibilityLabel",
                ArgumentSchema::String,
                "Sets the accessible name.",
                |a| match a {
                    ComponentArgument::String(v) => Ok(ProgressOp::Label(v.clone())),
                    _ => Err("Progress.accessibilityLabel(label) expects a string".into()),
                },
            ),
            unary(
                "size",
                ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                "Sets the semantic size.",
                |a| match a {
                    ComponentArgument::Enum(v) => match v.as_str() {
                        "xsmall" => Ok(ProgressOp::Size(Size::XSmall)),
                        "small" => Ok(ProgressOp::Size(Size::Small)),
                        "medium" => Ok(ProgressOp::Size(Size::Medium)),
                        "large" => Ok(ProgressOp::Size(Size::Large)),
                        _ => Err(format!("unsupported Progress size `{v}`")),
                    },
                    _ => Err("Progress.size(size) expects a size literal".into()),
                },
            ),
        ],
        typescript: TypeScriptDescriptor::new(
            "A linear determinate or indeterminate progress indicator.",
        ),
        materializer: Arc::new(ProgressMaterializer),
    })?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_progress_accepts_value() {
        drop(
            ProgressMaterializer::component(
                &ComponentPayload::new(ProgressPayload("p".into())),
                [&ProgressOp::Value(42.)],
            )
            .unwrap()
            .into_any_element(),
        );
    }
    #[test]
    fn invalid_payload_fails() {
        assert!(ProgressMaterializer::component(&ComponentPayload::new(()), []).is_err());
    }
    #[test]
    fn value_rejects_f64_values_outside_the_f32_range() {
        assert!(finite_f32((f32::MAX as f64) * 2.0, "Progress.value(value)").is_err());
        assert!(finite_f32(f64::NEG_INFINITY, "Progress.value(value)").is_err());
    }
    #[test]
    fn id_rejects_empty_and_whitespace_only_values() {
        assert!(nonempty_id("", "Progress").is_err());
        assert!(nonempty_id("   ", "Progress").is_err());
    }
}
