use std::sync::Arc;

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, RegistryError, TypeScriptDescriptor, anyhow,
    gpui::{self, ParentElement as _, div},
    gpui_component::text::Text,
};

#[derive(Clone)]
struct TextPayload(String);

struct TextMaterializer;

fn text_payload(arguments: &[ComponentArgument]) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::String(value)] => Ok(ComponentPayload::new(TextPayload(value.clone()))),
        _ => Err("Text(value) expects one string".into()),
    }
}

impl ComponentMaterializer for TextMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let value = request
            .payload()
            .downcast_ref::<TextPayload>()
            .ok_or_else(|| anyhow::anyhow!("Text received an incompatible payload"))?
            .0
            .clone();
        anyhow::ensure!(
            request.children_len() == 0,
            "Text does not accept children; pass its content to Text(value)"
        );
        request.finish(div().child(Text::from(value)))
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor {
        name: "Text",
        constructors: vec![ConstructorDescriptor::new(
            "Text",
            vec![ArgumentDescriptor::new("value", ArgumentSchema::String)],
            text_payload,
        )],
        methods: vec![],
        typescript: TypeScriptDescriptor::new(
            "Plain gpui-component Text content in a styleable shell wrapper. Text accepts no children.",
        ),
        materializer: Arc::new(TextMaterializer),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_constructor_is_closed_and_preserves_content() {
        let mut registry =
            ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let constructor = &frozen.descriptors().next().unwrap().constructors[0];
        assert_eq!(constructor.arguments[0].schema, ArgumentSchema::String);
        let payload = text_payload(&[ComponentArgument::String("hello".into())]).unwrap();
        assert_eq!(payload.downcast_ref::<TextPayload>().unwrap().0, "hello");
    }
}
