//! Small remaining surfaces whose native contracts fit the current shell.
//!
//! See `DEFERRED.md` beside this file for visually related catalog entries
//! that cannot cross the current host boundary without bypassing ownership or
//! capability rules.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    text::register(registry)?;
    dropdown_button::register(registry)?;
    Ok(())
}

mod dropdown_button;
mod text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_only_the_two_closed_renderable_surfaces() {
        let mut registry =
            ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        assert_eq!(
            frozen
                .descriptors()
                .map(|descriptor| descriptor.name)
                .collect::<Vec<_>>(),
            ["Text", "DropdownButton"]
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
