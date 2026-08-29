//! Component-family registrations.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    spinner::register(registry)?;
    separator::register(registry)?;
    skeleton::register(registry)?;
    controls::register(registry)?;
    display::register(registry)?;
    compound::register(registry)?;
    typed_compound::register(registry)?;
    Ok(())
}

mod compound;
mod controls;
mod display;
mod separator;
mod skeleton;
mod spinner;
mod typed_compound;
