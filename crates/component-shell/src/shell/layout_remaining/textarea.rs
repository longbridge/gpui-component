use std::sync::Arc;

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, TypeScriptDescriptor,
    anyhow,
    gpui::{self, AppContext as _, Entity, IntoElement as _, Refineable as _, Styled as _},
    gpui_component::input::{Textarea, TextareaState},
};

#[derive(Clone)]
enum Op {
    Appearance(bool),
    Bordered(bool),
    Readonly(bool),
    AriaLabel(String),
}

fn bool_method(name: &'static str, make: fn(bool) -> Op) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |args| match args {
            [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(make(*value))),
            _ => Err(format!("Textarea.{name} expects one boolean")),
        },
    )
    .documented("Sets the corresponding native textarea presentation or editing policy.")
}

struct Materializer;
fn require_leaf(children: usize) -> anyhow::Result<()> {
    anyhow::ensure!(children == 0, "Textarea does not accept children");
    Ok(())
}
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<ComponentArgument>()
            .ok_or_else(|| anyhow::anyhow!("Textarea received an incompatible payload"))?;
        let state = request.with_state::<Entity<TextareaState>, _>(argument, Clone::clone)?;
        let mut textarea = Textarea::new(&state).disabled(request.disabled());
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>())
        {
            textarea = match op {
                Op::Appearance(value) => textarea.appearance(*value),
                Op::Bordered(value) => textarea.bordered(*value),
                Op::Readonly(value) => textarea.readonly(*value),
                Op::AriaLabel(value) => textarea.aria_label(value.clone()),
            };
        }
        require_leaf(request.children_len())?;
        textarea.style().refine(&request.take_style());
        Ok(textarea.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textarea_is_an_exact_leaf() {
        assert!(require_leaf(0).is_ok());
        assert_eq!(
            require_leaf(1).unwrap_err().to_string(),
            "Textarea does not accept children"
        );
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new("TextareaState", "TextareaState", vec![], |_, window, cx| {
            Ok(Box::new(cx.new(|cx| TextareaState::new(window, cx))))
        })
        .documented("Retained multi-line editing state owned by a mounted view."),
    )?;
    registry.register(ComponentDescriptor {
        name: "Textarea",
        constructors: vec![ConstructorDescriptor::new("Textarea", vec![ArgumentDescriptor::new("state", ArgumentSchema::Entity("TextareaState"))], |args| match args {
            [argument @ ComponentArgument::Entity { .. }] => Ok(ComponentPayload::new(argument.clone())),
            _ => Err("Textarea expects one TextareaState entity".into()),
        })],
        methods: vec![
            MethodDescriptor::new("disabled", vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)], |_| Ok(ComponentPayload::new(()))).documented("Sets the common disabled state."),
            bool_method("appearance", Op::Appearance), bool_method("bordered", Op::Bordered), bool_method("readonly", Op::Readonly),
            MethodDescriptor::new("ariaLabel", vec![ArgumentDescriptor::new("label", ArgumentSchema::String)], |args| match args {
                [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(Op::AriaLabel(value.clone()))),
                _ => Err("Textarea.ariaLabel expects non-empty text".into()),
            }).documented("Sets the accessibility label."),
        ],
        typescript: TypeScriptDescriptor::new("A retained native multi-line text editor. Shell style and common disabled state are honored; children are rejected."),
        materializer: Arc::new(Materializer),
    })?;
    Ok(())
}
