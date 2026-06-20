//! A menu rendered natively by the operating system.
//!
//! Unlike [`crate::menu::PopupMenu`], which is drawn by GPUI and therefore
//! clipped to the window bounds, [`NativeMenu`] is rendered by the OS. It can
//! extend beyond the window — useful for small windows where a GPUI-drawn popup
//! menu would otherwise be cut off.
//!
//! Items carry a GPUI [`Action`], dispatched via [`Window::dispatch_action`]
//! when selected — the same mechanism the application menu bar and key bindings
//! use. A [`NativeMenu`] can therefore be built directly from GPUI
//! [`gpui::MenuItem`]s (see [`NativeMenu::from_menu_items`] /
//! [`From<gpui::Menu>`]).
//!
//! ```ignore
//! use gpui_component::native_menu::NativeMenu;
//!
//! NativeMenu::new()
//!     .menu("Copy", Box::new(Copy))
//!     .menu("Paste", Box::new(Paste))
//!     .separator()
//!     .menu("Delete", Box::new(Delete))
//!     .show(position, window, cx);
//! ```

use crate::Icon;

#[cfg(any(target_os = "macos", target_os = "windows"))]
use gpui::AssetSource;
use gpui::{Action, App, Pixels, Point, SharedString, Window};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::path::Path;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

// Drawn-menu fallback (used on platforms without an OS-native popup, e.g. Linux).
// Compiled on all platforms because `Root` holds the overlay entity.
mod fallback;
pub(crate) use fallback::FallbackMenuOverlay;

enum NativeMenuItem {
    Separator,
    Item {
        label: SharedString,
        disabled: bool,
        checked: bool,
        /// Icon shown next to the label.
        icon: Option<Box<NativeMenuIcon>>,
        /// Action dispatched when the item is selected.
        action: Option<Box<dyn Action>>,
    },
    Submenu {
        label: SharedString,
        disabled: bool,
        items: Vec<NativeMenuItem>,
    },
}

struct NativeMenuIcon {
    icon: Icon,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    source: Option<NativeMenuIconSource>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
enum NativeMenuIconSource {
    File(SharedString),
    Bytes(Vec<u8>),
}

impl NativeMenuIcon {
    fn new(icon: Icon) -> Self {
        Self {
            icon,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            source: None,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    fn path_ref(&self) -> &SharedString {
        self.icon.path_ref()
    }

    fn into_icon(self) -> Icon {
        self.icon
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn source(&self) -> Option<&NativeMenuIconSource> {
        self.source.as_ref()
    }
}

/// A menu rendered by the operating system.
///
/// Build it with the [`NativeMenu::menu`] / [`NativeMenu::separator`] builders,
/// then call [`NativeMenu::show`] to display it at a position.
#[derive(Default)]
pub struct NativeMenu {
    items: Vec<NativeMenuItem>,
}

impl NativeMenu {
    /// Create an empty native menu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a clickable item that dispatches `action` when selected.
    pub fn menu(self, label: impl Into<SharedString>, action: Box<dyn Action>) -> Self {
        self.menu_with(label, false, false, None, Some(action))
    }

    /// Append an item, controlling its `disabled` state.
    pub fn menu_with_disabled(
        self,
        label: impl Into<SharedString>,
        disabled: bool,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with(label, disabled, false, None, Some(action))
    }

    /// Append an item, controlling its `checked` state (a check mark is shown).
    pub fn menu_with_check(
        self,
        label: impl Into<SharedString>,
        checked: bool,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with(label, false, checked, None, Some(action))
    }

    /// Append an item showing `icon` next to its label.
    ///
    /// Native platform menus render file-backed icons from their filesystem path
    /// and asset-backed icons from memory. [`crate::IconName`] works across all backends.
    /// - **macOS**: loaded into an `NSImage` as a template image, so it tints with the item
    /// text and assigned to the item ([`NSMenuItem::image`]).
    /// - **Windows**: loaded into an `HBITMAP` and set as the item's
    /// content bitmap (`MENUITEMINFOW::hbmpItem`), shown beside the label. SVG files are
    /// rasterized, with `resvg`; other formats (PNG, JPEG, BMP, ...) are decoded by GDI+.
    /// **Other platforms** (fallback): rendered as the menu item's [`crate::Icon`].
    ///
    /// Note: this is the menu item's *content* icon, not its state/check-mark indicator.
    pub fn menu_with_icon(
        self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with(label, false, false, Some(icon.into()), Some(action))
    }

    /// Append an item showing `icon` next to its label, controlling its `disabled` state.
    ///
    /// Same icon behavior as [`Self::menu_with_icon`]. Use this when an item
    /// carries an icon but should be greyed out.
    pub fn menu_with_icon_disabled(
        self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        disabled: bool,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with(label, disabled, false, Some(icon.into()), Some(action))
    }

    /// Add Menu Item with Icon and disabled state.
    ///
    /// Alias for [`Self::menu_with_icon_disabled`], matching [`crate::menu::PopupMenu`].
    pub fn menu_with_icon_and_disabled(
        self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        action: Box<dyn Action>,
        disabled: bool,
    ) -> Self {
        self.menu_with_icon_disabled(label, icon, disabled, action)
    }

    fn menu_with(
        mut self,
        label: impl Into<SharedString>,
        disabled: bool,
        checked: bool,
        icon: Option<Icon>,
        action: Option<Box<dyn Action>>,
    ) -> Self {
        self.items.push(NativeMenuItem::Item {
            label: label.into(),
            disabled,
            checked,
            icon: icon.map(NativeMenuIcon::new).map(Box::new),
            action,
        });
        self
    }

    /// Append a separator line.
    pub fn separator(mut self) -> Self {
        self.items.push(NativeMenuItem::Separator);
        self
    }

    /// Append a submenu built from another [`NativeMenu`].
    pub fn submenu(mut self, label: impl Into<SharedString>, submenu: NativeMenu) -> Self {
        self.items.push(NativeMenuItem::Submenu {
            label: label.into(),
            disabled: false,
            items: submenu.items,
        });
        self
    }

    /// Whether the menu has no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Pop up the menu at `position` (window coordinates, in logical pixels).
    ///
    /// The menu is shown without blocking the caller: the OS tracking loop runs
    /// off GPUI's call stack, so GPUI is not borrowed while it is open. When an
    /// item is selected, its action is dispatched via [`Window::dispatch_action`].
    pub fn show(self, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
        if self.items.is_empty() {
            return;
        }

        #[cfg(target_os = "macos")]
        {
            let mut items = self.items;
            resolve_platform_icons(&mut items, cx.asset_source().as_ref());
            macos::show(items, position, window, cx);
        }
        #[cfg(target_os = "windows")]
        {
            let mut items = self.items;
            resolve_platform_icons(&mut items, cx.asset_source().as_ref());
            windows::show(items, position, window, cx);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        fallback::show(self.items, position, window, cx);
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn resolve_platform_icons(items: &mut [NativeMenuItem], asset_source: &dyn AssetSource) {
    for item in items {
        match item {
            NativeMenuItem::Separator => {}
            NativeMenuItem::Item {
                icon: icon_slot, ..
            } => {
                let Some(icon) = icon_slot.as_mut() else {
                    continue;
                };
                let source = resolve_icon_source(icon.path_ref(), asset_source);
                if source.is_some() {
                    icon.source = source;
                } else {
                    *icon_slot = None;
                }
            }
            NativeMenuItem::Submenu { items, .. } => resolve_platform_icons(items, asset_source),
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn resolve_icon_source(
    path: &SharedString,
    asset_source: &dyn AssetSource,
) -> Option<NativeMenuIconSource> {
    if path.is_empty() {
        return None;
    }

    if Path::new(path.as_ref()).is_file() {
        return Some(NativeMenuIconSource::File(path.clone()));
    }

    let bytes = asset_source.load(path.as_ref()).ok().flatten()?;
    Some(NativeMenuIconSource::Bytes(bytes.into_owned()))
}

/// Reuse an existing GPUI menu definition as a native menu.
///
/// `Action`s, separators, submenus, `checked`, and `disabled` are mapped over;
/// system menus (e.g. macOS Services) have no native popup equivalent and are
/// skipped.
impl From<gpui::Menu> for NativeMenu {
    fn from(menu: gpui::Menu) -> Self {
        let mut native = Self::new();
        for item in menu.items {
            match item {
                gpui::MenuItem::Separator => native.items.push(NativeMenuItem::Separator),
                gpui::MenuItem::Action {
                    name,
                    action,
                    checked,
                    disabled,
                    ..
                } => native.items.push(NativeMenuItem::Item {
                    label: name,
                    disabled,
                    checked,
                    icon: None,
                    action: Some(action),
                }),
                gpui::MenuItem::Submenu(submenu) => native.items.push(NativeMenuItem::Submenu {
                    label: submenu.name.clone(),
                    disabled: submenu.disabled,
                    items: Self::from(submenu).items,
                }),
                gpui::MenuItem::SystemMenu(_) => {}
            }
        }
        native
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IconName;
    use serde::Deserialize;

    #[derive(Action, Clone, PartialEq, Deserialize)]
    #[action(namespace = native_menu_tests, no_json)]
    struct TestAction;

    #[test]
    fn test_native_menu_builder_accepts_icon() {
        let menu =
            NativeMenu::new().menu_with_icon("Github", IconName::Github, Box::new(TestAction));

        assert_eq!(menu.items.len(), 1);
        let NativeMenuItem::Item {
            label,
            disabled,
            checked,
            icon: Some(icon),
            action: Some(_),
        } = &menu.items[0]
        else {
            panic!("expected an actionable item with an icon");
        };

        assert_eq!(label, "Github");
        assert!(!disabled);
        assert!(!checked);
        assert!(icon.path_ref().ends_with("github.svg"));
    }

    #[test]
    fn test_native_menu_builder_accepts_icon_and_disabled_alias() {
        let menu = NativeMenu::new().menu_with_icon_and_disabled(
            "Inbox",
            IconName::Inbox,
            Box::new(TestAction),
            true,
        );

        assert_eq!(menu.items.len(), 1);
        let NativeMenuItem::Item {
            label,
            disabled,
            checked,
            icon: Some(icon),
            action: Some(_),
        } = &menu.items[0]
        else {
            panic!("expected a disabled actionable item with an icon");
        };

        assert_eq!(label, "Inbox");
        assert!(disabled);
        assert!(!checked);
        assert!(icon.path_ref().ends_with("inbox.svg"));
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn test_native_menu_icon_asset_resolves_to_bytes() {
        let icon = Icon::new(IconName::Github);
        let source = resolve_icon_source(icon.path_ref(), &gpui_component_assets::Assets)
            .expect("icon asset should resolve");

        let NativeMenuIconSource::Bytes(bytes) = source else {
            panic!("expected asset-backed icon to resolve to bytes");
        };
        assert!(!bytes.is_empty());
    }
}
