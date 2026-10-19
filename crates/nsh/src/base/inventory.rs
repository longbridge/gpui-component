//! Every GPUI Base module, as the controls registered from it or the
//! reason it is not. The test keeps this list in step with GPUI Base's
//! modules and the catalog.

/// What NSH apps get from a GPUI Base module.
#[derive(Debug, Clone, Copy)]
pub(super) enum Coverage {
    /// Registered as these controls.
    Controls(&'static [&'static str]),
    /// Not registered, for this reason.
    Deferred(&'static str),
}

use Coverage::*;

const CONTAINER: &str = "A container of the app's own content: NSH lays out content itself, and a native component is a leaf that cannot hold NSH children.";
const OVERLAY: &str = "An overlay or a window-level surface: the host window owns overlays, and NSH apps use NSH's own modals and alerts.";
const INFRASTRUCTURE: &str = "Infrastructure the registered controls use, not a control.";
const LIST: &str =
    "A virtualized data view: NSH apps use NSH's own lists, which lay out NSH content.";

pub(super) const INVENTORY: &[(&str, Coverage)] = &[
    ("a11y_disabled", Deferred(INFRASTRUCTURE)),
    ("accordion", Deferred(CONTAINER)),
    ("actions", Deferred(INFRASTRUCTURE)),
    ("alert_dialog", Deferred(OVERLAY)),
    ("animation", Deferred(INFRASTRUCTURE)),
    ("async_util", Deferred(INFRASTRUCTURE)),
    ("auto_scroll", Deferred(INFRASTRUCTURE)),
    ("avatar", Controls(&["Avatar"])),
    ("button", Controls(&["Button"])),
    ("calendar", Controls(&["Calendar"])),
    ("checkbox", Controls(&["Checkbox"])),
    ("collapsible", Deferred(CONTAINER)),
    ("color_picker", Controls(&["ColorPicker"])),
    ("combobox", Controls(&["Combobox"])),
    ("component_traits", Deferred(INFRASTRUCTURE)),
    ("date_picker", Controls(&["DatePicker"])),
    ("dialog", Deferred(OVERLAY)),
    ("dock", Deferred(CONTAINER)),
    ("element_ext", Deferred(INFRASTRUCTURE)),
    ("event", Deferred(INFRASTRUCTURE)),
    ("focus_trap", Deferred(INFRASTRUCTURE)),
    ("geometry", Deferred(INFRASTRUCTURE)),
    ("global_state", Deferred(INFRASTRUCTURE)),
    ("history", Deferred(INFRASTRUCTURE)),
    ("hover_card", Deferred(OVERLAY)),
    ("index_path", Deferred(INFRASTRUCTURE)),
    ("input", Controls(&["Input", "Textarea"])),
    ("link", Controls(&["Link"])),
    ("list_settings", Deferred(INFRASTRUCTURE)),
    ("macos_accessibility", Deferred(INFRASTRUCTURE)),
    ("measure", Deferred(INFRASTRUCTURE)),
    ("motion", Deferred(INFRASTRUCTURE)),
    ("nav_stack", Deferred(CONTAINER)),
    ("number_input", Controls(&["NumberInput"])),
    ("observe", Deferred(INFRASTRUCTURE)),
    ("otp_input", Controls(&["OtpInput"])),
    ("overscroll", Deferred(INFRASTRUCTURE)),
    ("pagination", Controls(&["Pagination"])),
    ("plot", Deferred(INFRASTRUCTURE)),
    ("popover", Deferred(OVERLAY)),
    ("popup", Deferred(OVERLAY)),
    ("positioner", Deferred(INFRASTRUCTURE)),
    ("progress", Controls(&["Progress"])),
    ("questionnaire", Deferred(CONTAINER)),
    ("radio", Controls(&["Radio"])),
    ("radio_group", Controls(&["RadioGroup"])),
    ("reduce_motion", Deferred(INFRASTRUCTURE)),
    ("resizable", Deferred(CONTAINER)),
    ("root", Deferred(OVERLAY)),
    ("scroll_bounce", Deferred(INFRASTRUCTURE)),
    ("scrollable_mask", Deferred(INFRASTRUCTURE)),
    ("scrollbar", Deferred(INFRASTRUCTURE)),
    ("select", Controls(&["Select"])),
    ("selectable_text", Controls(&["SelectableText"])),
    ("sheet", Deferred(OVERLAY)),
    ("slider", Controls(&["Slider"])),
    ("state_style", Deferred(INFRASTRUCTURE)),
    ("styled", Deferred(INFRASTRUCTURE)),
    ("switch", Controls(&["Switch"])),
    ("table", Deferred(LIST)),
    ("tabs", Controls(&["Tabs"])),
    ("text", Controls(&["TextView"])),
    ("text_boundary", Deferred(INFRASTRUCTURE)),
    ("text_selection", Deferred(INFRASTRUCTURE)),
    ("theme", Deferred(INFRASTRUCTURE)),
    ("theme_tokens", Deferred(INFRASTRUCTURE)),
    ("time_field", Controls(&["TimeField"])),
    ("toast", Deferred(OVERLAY)),
    ("toggle", Controls(&["Toggle"])),
    ("toggle_group", Controls(&["ToggleGroup"])),
    ("toolbar", Deferred(CONTAINER)),
    ("tooltip", Deferred(OVERLAY)),
    ("touch_selection", Deferred(INFRASTRUCTURE)),
    ("tree", Deferred(LIST)),
    ("undo_history", Deferred(INFRASTRUCTURE)),
    ("virtual_list", Deferred(LIST)),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The modules `crates/base/src/lib.rs` declares.
    fn base_modules() -> Vec<String> {
        include_str!("../../../base/src/lib.rs")
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let rest = line
                    .strip_prefix("pub mod ")
                    .or_else(|| line.strip_prefix("mod "))?;
                let name = rest.strip_suffix(';')?;
                (name != "test_support").then(|| name.to_owned())
            })
            .collect()
    }

    #[test]
    fn test_inventory_lists_every_base_module() {
        let listed: Vec<&str> = INVENTORY.iter().map(|(module, _)| *module).collect();
        let modules = base_modules();
        for module in &modules {
            assert!(
                listed.contains(&module.as_str()),
                "`{module}` is not in the inventory"
            );
        }
        for module in &listed {
            assert!(
                modules.iter().any(|m| m == module),
                "`{module}` is not a GPUI Base module"
            );
        }
        let mut sorted = listed.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), listed.len(), "a module is listed twice");
    }

    #[test]
    fn test_deferred_modules_give_a_reason() {
        for (module, coverage) in INVENTORY {
            if let Deferred(reason) = coverage {
                assert!(reason.ends_with('.'), "`{module}`: give a full sentence");
            }
        }
    }

    #[test]
    fn test_inventory_matches_catalog() {
        let catalog = super::super::catalog();
        let registered: Vec<&str> = catalog.controls().map(|control| control.name()).collect();
        let mut listed = Vec::new();
        for (module, coverage) in INVENTORY {
            if let Controls(names) = coverage {
                for name in *names {
                    assert!(
                        registered.contains(name),
                        "`{module}`: `{name}` is not registered"
                    );
                    listed.push(*name);
                }
            }
        }
        for name in registered {
            assert!(
                listed.contains(&name),
                "`{name}` is registered but not in the inventory"
            );
        }
    }
}
