use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor, anyhow, gpui,
    gpui_component::{Sizable as _, Size, radio::Radio},
};
use std::sync::Arc;

use super::common::nonempty_id;
#[derive(Clone)]
struct RadioPayload(String);
#[derive(Clone)]
enum RadioOp {
    Label(String),
    A11y(String),
    Checked(bool),
    TabStop(bool),
    Size(Size),
}
struct RadioMaterializer;
impl ComponentMaterializer for RadioMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = &request
            .payload()
            .downcast_ref::<RadioPayload>()
            .ok_or_else(|| anyhow::anyhow!("Radio received an incompatible payload"))?
            .0;
        let mut radio = Radio::new(id.clone())
            .disabled(request.disabled())
            .checked(request.selected());
        for op in request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<RadioOp>())
        {
            radio = match op {
                RadioOp::Label(v) => radio.label(v.clone()),
                RadioOp::A11y(v) => radio.accessibility_label(v.clone()),
                RadioOp::Checked(v) => radio.checked(*v),
                RadioOp::TabStop(v) => radio.tab_stop(*v),
                RadioOp::Size(v) => radio.with_size(*v),
            }
        }
        request.finish(radio)
    }
}
fn method(
    name: &'static str,
    schema: ArgumentSchema,
    doc: &'static str,
    f: fn(&ComponentArgument) -> Result<RadioOp, String>,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, schema)],
        move |a| match a {
            [v] => f(v).map(ComponentPayload::new),
            _ => Err(format!("Radio.{name}({name}) expects one argument")),
        },
    )
    .documented(doc)
}
pub(super) fn register(r: &mut ComponentRegistry) -> Result<(), RegistryError> {
    r.register(ComponentDescriptor {
        name: "Radio",
        constructors: vec![ConstructorDescriptor::new(
            "Radio",
            vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
            |a| match a {
                [ComponentArgument::String(id)] => nonempty_id(id, "Radio")
                    .map(RadioPayload)
                    .map(ComponentPayload::new),
                _ => Err("Radio(id) expects a string id".into()),
            },
        )],
        methods: vec![
            method(
                "label",
                ArgumentSchema::String,
                "Sets the visible label.",
                |v| match v {
                    ComponentArgument::String(x) => Ok(RadioOp::Label(x.clone())),
                    _ => Err("Radio.label(label) expects a string".into()),
                },
            ),
            method(
                "accessibilityLabel",
                ArgumentSchema::String,
                "Overrides the announced name.",
                |v| match v {
                    ComponentArgument::String(x) => Ok(RadioOp::A11y(x.clone())),
                    _ => Err("Radio.accessibilityLabel(label) expects a string".into()),
                },
            ),
            method(
                "checked",
                ArgumentSchema::Boolean,
                "Controls checked state.",
                |v| match v {
                    ComponentArgument::Boolean(x) => Ok(RadioOp::Checked(*x)),
                    _ => Err("Radio.checked(checked) expects a boolean".into()),
                },
            ),
            method(
                "tabStop",
                ArgumentSchema::Boolean,
                "Controls keyboard tab-stop participation.",
                |v| match v {
                    ComponentArgument::Boolean(x) => Ok(RadioOp::TabStop(*x)),
                    _ => Err("Radio.tabStop(tabStop) expects a boolean".into()),
                },
            ),
            method(
                "size",
                ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                "Sets semantic size.",
                |v| match v {
                    ComponentArgument::Enum(x) => match x.as_str() {
                        "xsmall" => Ok(RadioOp::Size(Size::XSmall)),
                        "small" => Ok(RadioOp::Size(Size::Small)),
                        "medium" => Ok(RadioOp::Size(Size::Medium)),
                        "large" => Ok(RadioOp::Size(Size::Large)),
                        _ => Err(format!("unsupported Radio size `{x}`")),
                    },
                    _ => Err("Radio.size(size) expects a size literal".into()),
                },
            ),
        ],
        typescript: TypeScriptDescriptor::new(
            "A controlled radio control; selected and disabled common behavior is supported.",
        ),
        materializer: Arc::new(RadioMaterializer),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_rejects_empty_and_whitespace_only_values() {
        assert!(nonempty_id("", "Radio").is_err());
        assert!(nonempty_id(" \t ", "Radio").is_err());
        assert_eq!(nonempty_id("choice-a", "Radio").unwrap(), "choice-a");
    }
}
