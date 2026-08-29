//! Compound components that can be represented without retaining typed child state.

mod avatar;
mod collapsible;
mod common;
mod pagination;
mod progress;
mod radio;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    avatar::register(registry)?;
    collapsible::register(registry)?;
    pagination::register(registry)?;
    progress::register(registry)?;
    radio::register(registry)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use gpui_shell::{ArgumentSchema, COMPONENT_REGISTRY_API_VERSION};

    use super::*;

    #[test]
    fn registers_only_the_honestly_materializable_compound_batch() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        assert_eq!(
            frozen
                .descriptors()
                .map(|descriptor| descriptor.name)
                .collect::<Vec<_>>(),
            ["Avatar", "Collapsible", "Pagination", "Progress", "Radio"]
        );
        assert!(
            frozen
                .descriptors()
                .flat_map(|descriptor| descriptor.methods.iter())
                .all(|method| method.documentation.is_some())
        );
    }

    #[test]
    fn numeric_and_controlled_arguments_have_closed_schemas() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let pagination = frozen
            .descriptors()
            .find(|descriptor| descriptor.name == "Pagination")
            .unwrap();

        assert_eq!(
            pagination.methods[0].arguments[0].schema,
            ArgumentSchema::Number
        );
        let radio = frozen
            .descriptors()
            .find(|descriptor| descriptor.name == "Radio")
            .unwrap();
        assert_eq!(
            radio.methods[2].arguments[0].schema,
            ArgumentSchema::Boolean
        );
    }
}
