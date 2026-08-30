//! JavaScript component bindings for [`gpui_shell`].
//!
//! This crate is the only place concrete `gpui-component` knowledge belongs.
//! The dependency edge runs one way: this crate uses both `gpui-shell` and
//! `gpui-component`, and the runtime depends on neither this crate nor the
//! component library, so it stays usable without a component catalog.

mod shell;

/// Initializes the component catalog and the shell runtime it registers into.
///
/// Must be called once at application startup, before any script runs. This is
/// the entry point for a host that renders this catalog; [`gpui_shell::init`]
/// alone installs the base layer without any concrete components.
pub fn init(cx: &mut gpui_shell::gpui::App) {
    gpui_component::init(cx);
    gpui_shell::init(cx);
}

/// Builds and freezes the currently registered component catalog owned by this adapter.
pub fn components() -> Result<gpui_shell::FrozenComponentRegistry, gpui_shell::RegistryError> {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )?;
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
    fn the_runtime_does_not_depend_on_the_component_library() {
        let manifest = include_str!("../../shell/Cargo.toml");
        let dependencies = manifest
            .split_once("[dependencies]")
            .expect("dependencies table")
            .1
            .split_once("[dev-dependencies]")
            .expect("dev-dependencies table")
            .0;

        assert!(
            !dependencies.contains("gpui-component"),
            "`gpui-shell` must stay free of the concrete component catalog; \
             the adapter depends on both, not the runtime on one"
        );
    }

    #[gpui::test]
    fn init_installs_the_component_catalog_globals(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);

        cx.read(|cx| assert!(cx.has_global::<gpui_component::Theme>()));
    }

    #[test]
    fn register_exposes_the_first_leaf_component_batch_in_stable_order() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();

        crate::register(&mut registry).unwrap();

        let frozen = registry.freeze().unwrap();
        let descriptors = frozen.descriptors().collect::<Vec<_>>();
        assert_eq!(
            descriptors
                .iter()
                .take(3)
                .map(|descriptor| descriptor.name())
                .collect::<Vec<_>>(),
            ["Spinner", "Separator", "Skeleton"]
        );
        assert_eq!(
            descriptors
                .iter()
                .take(3)
                .flat_map(|descriptor| {
                    descriptor
                        .constructors()
                        .iter()
                        .map(|constructor| constructor.export())
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
        let undocumented = descriptors
            .iter()
            .filter(|descriptor| descriptor.documentation().is_none())
            .map(|descriptor| descriptor.name())
            .collect::<Vec<_>>();
        assert!(
            undocumented.is_empty(),
            "every registered component needs documentation: {undocumented:?}"
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
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        crate::register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        let spinner = frozen
            .descriptors()
            .find(|item| item.name() == "Spinner")
            .unwrap();
        assert_eq!(
            spinner
                .methods()
                .iter()
                .map(|method| (method.name(), method.arguments()))
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
            .find(|item| item.name() == "Separator")
            .unwrap();
        assert_eq!(
            separator
                .methods()
                .iter()
                .map(|method| method.name())
                .collect::<Vec<_>>(),
            ["label", "color", "dashed"]
        );

        let skeleton = frozen
            .descriptors()
            .find(|item| item.name() == "Skeleton")
            .unwrap();
        assert_eq!(
            skeleton
                .methods()
                .iter()
                .map(|method| method.name())
                .collect::<Vec<_>>(),
            ["secondary"]
        );
        let undocumented = frozen
            .descriptors()
            .flat_map(|descriptor| {
                descriptor
                    .methods()
                    .iter()
                    .filter(|method| method.documentation().is_none())
                    .map(move |method| format!("{}.{}", descriptor.name(), method.name()))
            })
            .collect::<Vec<_>>();
        assert!(
            undocumented.is_empty(),
            "every generated TypeScript method needs documentation: {undocumented:?}"
        );
    }

    #[test]
    fn runtime_typings_include_leaf_exports_and_methods() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
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
