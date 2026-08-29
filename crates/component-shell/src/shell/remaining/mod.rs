//! Remaining stateless component bindings that fit the shell's typed-child seam.

mod icon;
mod sidebar;
mod typed;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    icon::register(registry)?;
    sidebar::register(registry)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::{COMPONENT_REGISTRY_API_VERSION, ComponentRegistry};

    #[test]
    fn remaining_catalog_registers_real_icon_and_sidebar_exports() {
        let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        assert_eq!(
            frozen
                .descriptors()
                .map(|descriptor| descriptor.name)
                .collect::<Vec<_>>(),
            [
                "Icon",
                "SidebarMenuItem",
                "SidebarMenu",
                "SidebarHeader",
                "SidebarFooter",
                "Sidebar",
                "SidebarToggleButton",
            ]
        );
    }
}
