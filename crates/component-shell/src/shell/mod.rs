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
    collections_remaining::register(registry)?;
    command_remaining::register(registry)?;
    overlays::register(registry)?;
    retained_forms::register(registry)?;
    layout_remaining::register(registry)?;
    media_remaining::register(registry)?;
    scroll_remaining::register(registry)?;
    settings_remaining::register(registry)?;
    structured::register(registry)?;
    remaining::register(registry)?;
    basic_remaining::register(registry)?;
    Ok(())
}

mod basic_remaining;
mod collections_remaining;
mod command_remaining;
mod compound;
mod controls;
mod display;
mod layout_remaining;
mod media_remaining;
mod overlays;
mod remaining;
mod retained_forms;
mod scroll_remaining;
mod separator;
mod settings_remaining;
mod skeleton;
mod spinner;
mod structured;
mod typed_compound;
