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
use gpui::{Image, ImageFormat};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::{path::Path, sync::Arc};

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
        icon: Option<Box<Icon>>,
        /// Action dispatched when the item is selected.
        action: Option<Box<dyn Action>>,
    },
    Submenu {
        label: SharedString,
        disabled: bool,
        items: Vec<NativeMenuItem>,
    },
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
            icon: icon.map(Box::new),
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
            macos::show(self.items, cx.asset_source().clone(), position, window, cx);
        }
        #[cfg(target_os = "windows")]
        {
            windows::show(self.items, cx.asset_source().clone(), position, window, cx);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        fallback::show(self.items, position, window, cx);
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn resolve_icon_image(
    path: &SharedString,
    asset_source: &dyn AssetSource,
) -> Option<Arc<Image>> {
    if path.is_empty() {
        return None;
    }

    let bytes = if Path::new(path.as_ref()).is_file() {
        std::fs::read(path.as_ref()).ok()?
    } else {
        asset_source
            .load(path.as_ref())
            .ok()
            .flatten()?
            .into_owned()
    };
    let format = image_format(path.as_ref(), &bytes)?;
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn image_format(path: &str, bytes: &[u8]) -> Option<ImageFormat> {
    if let Some(extension) = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        let format = match extension.to_ascii_lowercase().as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "webp" => ImageFormat::Webp,
            "gif" => ImageFormat::Gif,
            "svg" => ImageFormat::Svg,
            "bmp" => ImageFormat::Bmp,
            "tif" | "tiff" => ImageFormat::Tiff,
            "ico" => ImageFormat::Ico,
            "pbm" | "pgm" | "ppm" | "pnm" => ImageFormat::Pnm,
            _ => return None,
        };
        return Some(format);
    }

    image_format_from_bytes(bytes)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn image_format_from_bytes(bytes: &[u8]) -> Option<ImageFormat> {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some(ImageFormat::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(ImageFormat::Gif)
    } else if bytes.starts_with(b"BM") {
        Some(ImageFormat::Bmp)
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some(ImageFormat::Webp)
    } else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        Some(ImageFormat::Tiff)
    } else if bytes.starts_with(b"\0\0\x01\0") || bytes.starts_with(b"\0\0\x02\0") {
        Some(ImageFormat::Ico)
    } else if is_svg_bytes(bytes) {
        Some(ImageFormat::Svg)
    } else if matches!(
        bytes.get(0..2),
        Some(b"P1" | b"P2" | b"P3" | b"P4" | b"P5" | b"P6")
    ) {
        Some(ImageFormat::Pnm)
    } else {
        None
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn is_svg_bytes(bytes: &[u8]) -> bool {
    let text = match std::str::from_utf8(&bytes[..bytes.len().min(256)]) {
        Ok(text) => text.trim_start(),
        Err(_) => return false,
    };
    text.starts_with("<svg") || text.starts_with("<?xml")
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
        let image = resolve_icon_image(icon.path_ref(), &gpui_component_assets::Assets)
            .expect("icon asset should resolve");

        assert_eq!(image.format, ImageFormat::Svg);
        assert!(!image.bytes.is_empty());
    }
}
