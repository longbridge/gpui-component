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
///
/// The catalog also carries this function, so a host holding only the frozen
/// registry — the shipped command, for one — gets the same startup through
/// [`gpui_shell::init_with_components`].
pub fn init(cx: &mut gpui_shell::gpui::App) {
    gpui_component::init(cx);
    gpui_shell::init(cx);
}

/// Builds and freezes the currently registered component catalog owned by this adapter.
pub fn components() -> Result<gpui_shell::FrozenComponentRegistry, gpui_shell::RegistryError> {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )?
    .with_initializer(gpui_component::init)
    .with_window_opener(open_window_with_root);
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

/// Opens the window with `gpui_component::Root` as its root view.
///
/// Every `gpui-component` overlay — dialog, alert dialog, sheet, notification —
/// finds its host with `window.root::<Root>()` and panics when the window is
/// rooted at anything else. The runtime installs its own `ShellRoot` and cannot
/// name `Root`, so the catalog that needs one supplies it here. `ShellRoot`
/// keeps rendering inside it, and its own overlays with it.
fn open_window_with_root(
    cx: &mut gpui_shell::gpui::App,
    options: gpui_shell::gpui::WindowOptions,
    build: &mut dyn FnMut(
        &mut gpui_shell::gpui::Window,
        &mut gpui_shell::gpui::App,
    ) -> gpui_shell::gpui::AnyView,
) -> gpui_shell::anyhow::Result<gpui_shell::gpui::AnyWindowHandle> {
    use gpui_shell::gpui::AppContext as _;

    let handle = cx.open_window(options, |window, cx| {
        let inner = build(window, cx);
        cx.new(|cx| gpui_component::Root::new(inner, window, cx))
    })?;
    Ok(handle.into())
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

    /// Every `gpui-component` overlay locates its host with
    /// `window.root::<Root>()` and panics when the window is rooted at anything
    /// else. The runtime roots its window at `ShellRoot`, so unless the catalog
    /// supplies the `Root` itself, every dialog, alert dialog, sheet and
    /// notification panics the moment it opens.
    ///
    /// The other tests in this crate build their own `Root`, which is why none
    /// of them noticed.
    #[gpui::test]
    fn the_catalog_opens_a_window_its_overlays_can_find(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;

        let components = crate::components().unwrap();
        let open = components
            .window_opener()
            .expect("a catalog whose overlays require a window root must open the window");

        cx.update(crate::init);
        let handle = cx
            .update(|cx| {
                let options = gpui::WindowOptions {
                    show: false,
                    ..Default::default()
                };
                open(cx, options, &mut |_window, cx| cx.new(|_| Blank).into())
            })
            .expect("the catalog must open a window");

        let found = handle
            .update(cx, |_, window, _| {
                window.root::<gpui_component::Root>().is_some()
            })
            .expect("the window must be live");
        assert!(
            found,
            "the window is not rooted at gpui_component::Root, so every overlay would panic"
        );
    }

    struct Blank;

    impl gpui::Render for Blank {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    #[gpui::test]
    fn init_installs_the_component_catalog_globals(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);

        cx.read(|cx| assert!(cx.has_global::<gpui_component::Theme>()));
    }

    /// The shipped command builds a runtime from [`crate::components`] alone
    /// and never calls [`crate::init`], so the catalog has to carry its own
    /// startup. Without it the binary starts and then panics looking for a
    /// theme on the first render, which no test of `init` would notice.
    #[gpui::test]
    fn the_frozen_catalog_carries_its_own_startup(cx: &mut gpui::TestAppContext) {
        let components = crate::components().unwrap();
        assert!(components.initializer().is_some());

        cx.update(|cx| gpui_shell::init_with_components(cx, &components));

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
