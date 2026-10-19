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
const INFRASTRUCTURE: &str = "Infrastructure the registered controls use, not a control.";
const SIZES_CONTENT: &str = "Resizable panels size the app's own content: a native component is a leaf that cannot hold NSH children.";
const PAGES: &str = "A navigation stack of GPUI views: NSH apps use NSH's own navigation, whose pages are NSH content.";
const SCROLLS_CONTENT: &str = "A scrollbar drives another element's scrolling through its handle: NSH's scroll views draw their own indicator.";
const POPUP: &str = "The anchored overlay part the registered pickers and selects compose; it has no use on its own.";

pub(super) const INVENTORY: &[(&str, Coverage)] = &[
    ("a11y_disabled", Deferred(INFRASTRUCTURE)),
    ("accordion", Controls(&["Accordion"])),
    ("actions", Deferred(INFRASTRUCTURE)),
    ("alert_dialog", Controls(&["AlertDialog"])),
    ("animation", Deferred(INFRASTRUCTURE)),
    ("async_util", Deferred(INFRASTRUCTURE)),
    ("auto_scroll", Deferred(INFRASTRUCTURE)),
    ("avatar", Controls(&["Avatar"])),
    ("button", Controls(&["Button"])),
    ("calendar", Controls(&["Calendar"])),
    ("checkbox", Controls(&["Checkbox"])),
    ("collapsible", Controls(&["Collapsible"])),
    ("color_picker", Controls(&["ColorPicker"])),
    ("combobox", Controls(&["Combobox"])),
    ("component_traits", Deferred(INFRASTRUCTURE)),
    ("date_picker", Controls(&["DatePicker"])),
    ("dialog", Controls(&["Dialog"])),
    ("dock", Deferred(CONTAINER)),
    ("element_ext", Deferred(INFRASTRUCTURE)),
    ("event", Deferred(INFRASTRUCTURE)),
    ("focus_trap", Deferred(INFRASTRUCTURE)),
    ("geometry", Deferred(INFRASTRUCTURE)),
    ("global_state", Deferred(INFRASTRUCTURE)),
    ("history", Deferred(INFRASTRUCTURE)),
    ("hover_card", Controls(&["HoverCard"])),
    ("index_path", Deferred(INFRASTRUCTURE)),
    ("input", Controls(&["Input", "Textarea"])),
    ("link", Controls(&["Link"])),
    ("list_settings", Deferred(INFRASTRUCTURE)),
    ("macos_accessibility", Deferred(INFRASTRUCTURE)),
    ("measure", Deferred(INFRASTRUCTURE)),
    ("motion", Deferred(INFRASTRUCTURE)),
    ("nav_stack", Deferred(PAGES)),
    ("number_input", Controls(&["NumberInput"])),
    ("observe", Deferred(INFRASTRUCTURE)),
    ("otp_input", Controls(&["OtpInput"])),
    ("overscroll", Deferred(INFRASTRUCTURE)),
    ("pagination", Controls(&["Pagination"])),
    ("plot", Deferred(INFRASTRUCTURE)),
    ("popover", Controls(&["Popover"])),
    ("popup", Deferred(POPUP)),
    ("positioner", Deferred(INFRASTRUCTURE)),
    ("progress", Controls(&["Progress"])),
    ("questionnaire", Deferred(CONTAINER)),
    ("radio", Controls(&["Radio"])),
    ("radio_group", Controls(&["RadioGroup"])),
    ("reduce_motion", Deferred(INFRASTRUCTURE)),
    ("resizable", Deferred(SIZES_CONTENT)),
    ("roving_focus", Deferred(INFRASTRUCTURE)),
    ("root", Deferred(INFRASTRUCTURE)),
    ("scroll_bounce", Deferred(INFRASTRUCTURE)),
    ("scrollable_mask", Deferred(INFRASTRUCTURE)),
    ("scrollbar", Deferred(SCROLLS_CONTENT)),
    ("select", Controls(&["Select"])),
    ("selectable_text", Controls(&["SelectableText"])),
    ("sheet", Controls(&["Sheet"])),
    ("slider", Controls(&["Slider"])),
    ("state_style", Deferred(INFRASTRUCTURE)),
    ("styled", Deferred(INFRASTRUCTURE)),
    ("switch", Controls(&["Switch"])),
    ("table", Controls(&["Table"])),
    ("tabs", Controls(&["Tabs"])),
    ("text", Controls(&["TextView"])),
    ("text_boundary", Deferred(INFRASTRUCTURE)),
    ("text_selection", Deferred(INFRASTRUCTURE)),
    ("theme", Deferred(INFRASTRUCTURE)),
    ("theme_tokens", Deferred(INFRASTRUCTURE)),
    ("time_field", Controls(&["TimeField"])),
    ("toast", Controls(&["ToastStack"])),
    ("toggle", Controls(&["Toggle"])),
    ("toggle_group", Controls(&["ToggleGroup"])),
    ("toolbar", Controls(&["Toolbar"])),
    ("tooltip", Controls(&["Tooltip"])),
    ("touch_selection", Deferred(INFRASTRUCTURE)),
    ("tree", Controls(&["Tree"])),
    ("undo_history", Deferred(INFRASTRUCTURE)),
    ("virtual_list", Controls(&["List"])),
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
