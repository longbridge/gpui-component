use std::sync::Arc;

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, TypeScriptDescriptor,
    anyhow,
    gpui::{self, AppContext as _, Entity, IntoElement as _, Refineable as _, Styled as _},
    gpui_component::input::{Editor, EditorState},
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
            _ => Err(format!("Editor.{name} expects one boolean")),
        },
    )
}

fn require_leaf(children: usize) -> anyhow::Result<()> {
    anyhow::ensure!(children == 0, "Editor does not accept children");
    Ok(())
}

struct Materializer;
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<ComponentArgument>()
            .ok_or_else(|| anyhow::anyhow!("Editor received an incompatible payload"))?;
        let state = request.with_state::<Entity<EditorState>, _>(argument, Clone::clone)?;
        let mut editor = Editor::new(&state).disabled(request.disabled());
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>())
        {
            editor = match op {
                Op::Appearance(value) => editor.appearance(*value),
                Op::Bordered(value) => editor.bordered(*value),
                Op::Readonly(value) => editor.readonly(*value),
                Op::AriaLabel(value) => editor.aria_label(value.clone()),
            };
        }
        require_leaf(request.children_len())?;
        editor.style().refine(&request.take_style());
        Ok(editor.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "EditorState",
            "EditorState",
            vec![ArgumentDescriptor::new(
                "initialValue",
                ArgumentSchema::String,
            )],
            |args, window, cx| match args {
                [ComponentArgument::String(value)] => Ok(Box::new(
                    cx.new(|cx| EditorState::new(window, cx).default_value(value.clone())),
                ) as _),
                _ => Err("EditorState expects one initial string".into()),
            },
        )
        .documented("Retained source-editor state initialized with a local text value."),
    )?;
    registry.register(ComponentDescriptor {
        name: "Editor",
        constructors: vec![ConstructorDescriptor::new(
            "Editor",
            vec![ArgumentDescriptor::new("state", ArgumentSchema::Entity("EditorState"))],
            |args| match args {
                [argument @ ComponentArgument::Entity { .. }] => Ok(ComponentPayload::new(argument.clone())),
                _ => Err("Editor expects one EditorState entity".into()),
            },
        )],
        methods: vec![
            MethodDescriptor::new("disabled", vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)], |_| Ok(ComponentPayload::new(()))).documented("Disables the editor."),
            bool_method("appearance", Op::Appearance).documented("Controls the editor appearance."),
            bool_method("bordered", Op::Bordered).documented("Controls the editor border."),
            bool_method("readonly", Op::Readonly).documented("Controls read-only mode."),
            MethodDescriptor::new(
                "ariaLabel",
                vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                |args| match args {
                    [ComponentArgument::String(value)] if !value.trim().is_empty() => Ok(ComponentPayload::new(Op::AriaLabel(value.clone()))),
                    _ => Err("Editor.ariaLabel expects non-empty text".into()),
                },
            ).documented("Sets the editor accessibility label."),
        ],
        typescript: TypeScriptDescriptor::new(
            "A retained native source editor. Shell style and common disabled state are honored; children are rejected. Language providers and custom context menus are not exposed.",
        ),
        materializer: Arc::new(Materializer),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_is_an_exact_leaf() {
        assert!(require_leaf(0).is_ok());
        assert_eq!(
            require_leaf(1).unwrap_err().to_string(),
            "Editor does not accept children"
        );
    }
}
