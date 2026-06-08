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

use gpui::{Action, App, Pixels, Point, SharedString, Window};

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
        self.menu_with(label, false, false, Some(action))
    }

    /// Append an item, controlling its `disabled` state.
    pub fn menu_with_disabled(
        self,
        label: impl Into<SharedString>,
        disabled: bool,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with(label, disabled, false, Some(action))
    }

    /// Append an item, controlling its `checked` state (a check mark is shown).
    pub fn menu_with_check(
        self,
        label: impl Into<SharedString>,
        checked: bool,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with(label, false, checked, Some(action))
    }

    fn menu_with(
        mut self,
        label: impl Into<SharedString>,
        disabled: bool,
        checked: bool,
        action: Option<Box<dyn Action>>,
    ) -> Self {
        self.items.push(NativeMenuItem::Item {
            label: label.into(),
            disabled,
            checked,
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
        macos::show(self.items, position, window, cx);
        #[cfg(target_os = "windows")]
        windows::show(self.items, position, window, cx);
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        fallback::show(self.items, position, window, cx);
    }
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
