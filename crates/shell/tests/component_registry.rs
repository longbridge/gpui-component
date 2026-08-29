use std::sync::Arc;

use gpui::{AnyElement, IntoElement as _, div};
use gpui_shell::{
    COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor, ComponentMaterializer, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, RegistryError, TypeScriptDescriptor,
};

struct EmptyMaterializer;

impl ComponentMaterializer for EmptyMaterializer {
    fn materialize(&self, _request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
        Ok(div().into_any_element())
    }
}

fn descriptor(name: &'static str, export: &'static str) -> ComponentDescriptor {
    ComponentDescriptor {
        name,
        constructors: vec![ConstructorDescriptor::new(export)],
        methods: Vec::new(),
        typescript: TypeScriptDescriptor::default(),
        materializer: Arc::new(EmptyMaterializer),
    }
}

#[test]
fn registry_assigns_stable_ids_and_preserves_registration_order() {
    let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();

    let button = registry.register(descriptor("Button", "Button")).unwrap();
    let badge = registry.register(descriptor("Badge", "Badge")).unwrap();
    let frozen = registry.freeze().unwrap();

    assert_eq!(button.as_u32(), 0);
    assert_eq!(badge.as_u32(), 1);
    assert_eq!(
        frozen
            .descriptors()
            .map(|descriptor| descriptor.name)
            .collect::<Vec<_>>(),
        ["Button", "Badge"]
    );
    assert_eq!(frozen.descriptor(button).unwrap().name, "Button");
}

#[test]
fn registry_rejects_duplicate_component_and_export_names() {
    let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
    registry.register(descriptor("Button", "Button")).unwrap();

    assert!(matches!(
        registry.register(descriptor("Button", "AnotherButton")),
        Err(RegistryError::DuplicateComponent(name)) if name == "Button"
    ));
    assert!(matches!(
        registry.register(descriptor("ButtonAlias", "Button")),
        Err(RegistryError::DuplicateExport(name)) if name == "Button"
    ));
}

#[test]
fn registry_rejects_registration_after_freeze() {
    let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
    registry.register(descriptor("Button", "Button")).unwrap();
    let _frozen = registry.freeze().unwrap();

    assert!(matches!(
        registry.register(descriptor("Badge", "Badge")),
        Err(RegistryError::Frozen)
    ));
}

#[test]
fn registry_rejects_an_incompatible_api_version() {
    assert!(matches!(
        ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION + 1),
        Err(RegistryError::IncompatibleApiVersion { expected, actual })
            if expected == COMPONENT_REGISTRY_API_VERSION
                && actual == COMPONENT_REGISTRY_API_VERSION + 1
    ));
}
