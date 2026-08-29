//! Structured, stateless component bindings.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    description_list::register(registry)?;
    form::register(registry)?;
    table::register(registry)?;
    Ok(())
}

mod common;
mod description_list;
mod form;
mod table;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_structured_components_in_dependency_order() {
        let mut registry =
            ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        assert_eq!(
            frozen
                .descriptors()
                .map(|descriptor| descriptor.name)
                .collect::<Vec<_>>(),
            [
                "DescriptionItem",
                "DescriptionList",
                "Field",
                "Form",
                "TableHeader",
                "TableBody",
                "TableFooter",
                "TableRow",
                "TableHead",
                "TableCell",
                "TableCaption",
                "Table",
            ]
        );
        assert!(frozen.descriptors().all(|descriptor| {
            descriptor.typescript.documentation.is_some()
                && descriptor
                    .methods
                    .iter()
                    .all(|method| method.documentation.is_some())
        }));
    }
}
