//! JavaScript component bindings for [`gpui_shell`].
//!
//! This crate is the only place concrete `gpui-component` registrations belong.
//! It deliberately depends on `gpui-shell`; the runtime stays usable without
//! the complete component catalog.

mod shell;

/// Builds and freezes the currently registered component catalog owned by this adapter.
pub fn components() -> Result<gpui_shell::FrozenComponentRegistry, gpui_shell::RegistryError> {
    let mut registry =
        gpui_shell::ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION)?;
    register(&mut registry)?;
    registry.freeze()
}

/// Creates the application's default shell runtime with this component catalog.
pub fn new_runtime(
    cx: &mut gpui_shell::gpui::App,
) -> gpui_shell::anyhow::Result<std::rc::Rc<gpui_shell::ShellRuntime>> {
    gpui_shell::ShellRuntime::new_with_components(cx, components()?)
}

/// Creates an isolated shell runtime with this component catalog.
pub fn new_isolated_runtime() -> gpui_shell::anyhow::Result<std::rc::Rc<gpui_shell::ShellRuntime>> {
    gpui_shell::ShellRuntime::new_isolated_with_components(components()?)
}

/// Writes declarations for the currently registered adapter catalog.
pub fn write_type_declarations(
    root: impl AsRef<std::path::Path>,
) -> gpui_shell::anyhow::Result<Vec<std::path::PathBuf>> {
    Ok(gpui_shell::write_type_declarations_with_components(
        root.as_ref(),
        &components()?,
    )?)
}

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
    fn normal_dependency_surface_is_only_gpui_shell() {
        let manifest = include_str!("../Cargo.toml");
        let dependencies = manifest
            .split_once("[dependencies]")
            .expect("dependencies table")
            .1
            .split_once("[dev-dependencies]")
            .expect("dev-dependencies table")
            .0;
        let names = dependencies
            .lines()
            .filter_map(|line| line.split_once('=').map(|(name, _)| name.trim()))
            .map(|name| name.strip_suffix(".workspace").unwrap_or(name))
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();

        assert_eq!(names, ["gpui-shell"]);
    }

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
                .take(3)
                .map(|descriptor| descriptor.name)
                .collect::<Vec<_>>(),
            ["Spinner", "Separator", "Skeleton"]
        );
        assert_eq!(
            descriptors
                .iter()
                .take(3)
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
    fn adapter_runtime_owns_the_registered_component_catalog() {
        let runtime = crate::new_isolated_runtime().unwrap();

        let declarations = runtime.type_declarations();
        assert!(declarations.contains("export const Spinner: { new(): SpinnerElement };"));
        assert!(declarations.contains("export const Skeleton: { new(): SkeletonElement };"));
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
        let undocumented = frozen
            .descriptors()
            .flat_map(|descriptor| {
                descriptor
                    .methods
                    .iter()
                    .filter(|method| method.documentation.is_none())
                    .map(move |method| format!("{}.{}", descriptor.name, method.name))
            })
            .collect::<Vec<_>>();
        assert!(
            undocumented.is_empty(),
            "every generated TypeScript method needs documentation: {undocumented:?}"
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
