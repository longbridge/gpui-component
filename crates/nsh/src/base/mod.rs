//! GPUI Base's controls as NSH native components (`gpui-base.<Name>`).
//!
//! GPUI Base owns each control's behavior: pointer and keyboard
//! interaction, focus, accessibility, and its state machine. It installs no
//! presentation, so the NSH app supplies it: the box's own NSH styles, and
//! class strings for the element and its parts (`rootClass`, `thumbClass`, ...;
//! see [`crate::Props::styled_by`]). Each entry only maps props to the
//! control's builder and its callbacks to events.
//!
//! `inventory.rs` lists every GPUI Base module as registered or deferred, and
//! its test keeps the list complete.

use crate::{Catalog, Control};

mod check;
mod display;
mod group;
mod input;
#[cfg(test)]
mod inventory;
mod number;
mod pagination;
mod picker;
mod press;
mod select;
mod slider;
mod text;
mod time;

/// The prefix the controls are registered under (`gpui-base.Button`).
pub const PREFIX: &str = "gpui-base";

fn controls() -> impl Iterator<Item = Control> {
    [
        press::controls(),
        check::controls(),
        group::controls(),
        display::controls(),
        slider::controls(),
        input::controls(),
        number::controls(),
        time::controls(),
        picker::controls(),
        select::controls(),
        pagination::controls(),
        text::controls(),
    ]
    .into_iter()
    .flatten()
}

/// GPUI Base's controls.
pub fn catalog() -> Catalog {
    controls().fold(Catalog::new(PREFIX), Catalog::control)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `types/gpui-base.ts` is what the catalog generates. Run with
    /// `UPDATE_TYPES=1` to write it again.
    #[test]
    fn test_typescript_is_current() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/types/gpui-base.ts");
        let generated = catalog().typescript();
        if std::env::var_os("UPDATE_TYPES").is_some() {
            std::fs::write(path, &generated).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(path).unwrap_or_default(),
            generated,
            "types/gpui-base.ts is stale: run `UPDATE_TYPES=1 cargo test -p gpui-nsh`"
        );
    }

    #[test]
    fn test_control_names_are_unique() {
        let mut names: Vec<_> = controls().map(|control| control.name()).collect();
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "two controls share a name");
    }
}
