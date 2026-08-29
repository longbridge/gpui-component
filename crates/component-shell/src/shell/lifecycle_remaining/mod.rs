//! Concrete lifecycle-adjacent surfaces that can be mounted as ordinary elements.

mod tooltip;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tooltip::test_probe;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(crate) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    tooltip::register(registry)
}
