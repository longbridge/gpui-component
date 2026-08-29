//! JavaScript component bindings for [`gpui_shell`].
//!
//! This crate is the only place concrete `gpui-component` registrations belong.
//! It deliberately depends on `gpui-shell`; the runtime stays usable without
//! the complete component catalog.

mod shell;

/// Registers the `gpui-component` JavaScript bindings provided by this crate.
pub fn register(
    registry: &mut gpui_shell::ComponentRegistry,
) -> Result<(), gpui_shell::RegistryError> {
    shell::register(registry)
}

#[cfg(test)]
mod tests {
    use gpui_shell::{ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentRegistry};

    #[test]
    fn shell_reexports_the_adapter_component_and_rendering_dependencies() {
        fn accepts_component(_: gpui_shell::gpui_component::Size) {}
        fn accepts_element(_: gpui_shell::gpui::AnyElement) {}

        accepts_component(gpui_shell::gpui_component::Size::Medium);
        accepts_element(gpui_shell::gpui::IntoElement::into_any_element(
            gpui_shell::gpui::div(),
        ));
    }

    #[test]
    fn register_exposes_the_first_leaf_component_batch_in_stable_order() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();

        crate::register(&mut registry).unwrap();

        let frozen = registry.freeze().unwrap();
        let descriptors = frozen.descriptors().collect::<Vec<_>>();
        assert_eq!(
            descriptors
                .iter()
                .map(|descriptor| descriptor.name)
                .collect::<Vec<_>>(),
            ["Spinner", "Separator", "Skeleton"]
        );
        assert_eq!(
            descriptors
                .iter()
                .flat_map(|descriptor| {
                    descriptor
                        .constructors
                        .iter()
                        .map(|constructor| constructor.export)
                })
                .collect::<Vec<_>>(),
            [
                "Spinner",
                "Separator",
                "VerticalSeparator",
                "DashedSeparator",
                "VerticalDashedSeparator",
                "Skeleton",
            ]
        );
        assert!(
            descriptors
                .iter()
                .all(|descriptor| descriptor.typescript.documentation.is_some())
        );
    }

    #[test]
    fn leaf_descriptors_publish_only_closed_honest_method_schemas() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        crate::register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        let spinner = frozen
            .descriptors()
            .find(|item| item.name == "Spinner")
            .unwrap();
        assert_eq!(
            spinner
                .methods
                .iter()
                .map(|method| (method.name, method.arguments.as_slice()))
                .collect::<Vec<_>>(),
            [
                (
                    "size",
                    [gpui_shell::ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )]
                    .as_slice(),
                ),
                (
                    "icon",
                    [gpui_shell::ArgumentDescriptor::new(
                        "icon",
                        ArgumentSchema::Enum(&["loader", "loaderCircle"]),
                    )]
                    .as_slice(),
                ),
                (
                    "color",
                    [gpui_shell::ArgumentDescriptor::new(
                        "color",
                        ArgumentSchema::String
                    )]
                    .as_slice(),
                ),
                (
                    "ease",
                    [gpui_shell::ArgumentDescriptor::new(
                        "ease",
                        ArgumentSchema::Enum(&["linear", "easeInOut", "easeOutQuint"]),
                    )]
                    .as_slice(),
                ),
            ]
        );

        let separator = frozen
            .descriptors()
            .find(|item| item.name == "Separator")
            .unwrap();
        assert_eq!(
            separator
                .methods
                .iter()
                .map(|method| method.name)
                .collect::<Vec<_>>(),
            ["label", "color", "dashed"]
        );

        let skeleton = frozen
            .descriptors()
            .find(|item| item.name == "Skeleton")
            .unwrap();
        assert_eq!(
            skeleton
                .methods
                .iter()
                .map(|method| method.name)
                .collect::<Vec<_>>(),
            ["secondary"]
        );
        assert!(
            frozen
                .descriptors()
                .flat_map(|descriptor| &descriptor.methods)
                .all(|method| method.documentation.is_some()),
            "every generated TypeScript method needs documentation"
        );
    }

    #[test]
    fn runtime_typings_include_leaf_exports_and_methods() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        crate::register(&mut registry).unwrap();
        let runtime =
            gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap())
                .unwrap();

        let declarations = runtime.type_declarations();
        for expected in [
            "export const Spinner: { new(): SpinnerElement };",
            "size(size: \"xsmall\" | \"small\" | \"medium\" | \"large\"): SpinnerElement;",
            "export const VerticalDashedSeparator: { new(): SeparatorElement };",
            "label(label: string): SeparatorElement;",
            "secondary(): SkeletonElement;",
        ] {
            assert!(declarations.contains(expected), "missing `{expected}`");
        }
    }
}
