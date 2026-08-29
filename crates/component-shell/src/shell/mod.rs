//! Component-family registrations.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    spinner::register(registry)?;
    separator::register(registry)?;
    skeleton::register(registry)?;
    Ok(())
}

mod separator;
mod skeleton;
mod spinner;
