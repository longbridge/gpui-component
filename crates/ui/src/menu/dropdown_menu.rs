use gpui::{Context, Corner, InteractiveElement, IntoElement, SharedString, Styled, Window};

use crate::{button::Button, menu::PopupMenu, popover::Popover, Selectable};

/// A dropdown menu trait for buttons and other interactive elements
pub trait DropdownMenu: Styled + Selectable + InteractiveElement + IntoElement + 'static {
    /// Create a dropdown menu with the given items, anchored to the TopLeft corner
    fn dropdown_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Popover {
        self.dropdown_menu_with_anchor(Corner::TopLeft, f)
    }

    /// Create a dropdown menu with the given items, anchored to the given corner
    fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Corner>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Popover {
        let style = self.style().clone();
        let id = self.interactivity().element_id.clone();

        Popover::new(SharedString::from(format!("dropdown-menu:{:?}", id)))
            .appearance(false)
            .trigger(self)
            .trigger_style(style)
            .anchor(anchor.into())
            .content(move |_, window, cx| {
                PopupMenu::build(window, cx, |menu, window, cx| f(menu, window, cx))
            })
    }
}

impl DropdownMenu for Button {}
