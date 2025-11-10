use std::rc::Rc;

use gpui::{
    Context, Corner, DismissEvent, ElementId, Focusable, InteractiveElement, IntoElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window,
};

use crate::{button::Button, menu::PopupMenu, popover::Popover, Selectable};

/// A dropdown menu trait for buttons and other interactive elements
pub trait DropdownMenu: Styled + Selectable + InteractiveElement + IntoElement + 'static {
    /// Create a dropdown menu with the given items, anchored to the TopLeft corner
    fn dropdown_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        self.dropdown_menu_with_anchor(Corner::TopLeft, f)
    }

    /// Create a dropdown menu with the given items, anchored to the given corner
    fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Corner>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        let style = self.style().clone();
        let id = self.interactivity().element_id.clone();

        DropdownMenuPopover::new(id.unwrap_or(0.into()), anchor, self, f).trigger_style(style)
    }
}

impl DropdownMenu for Button {}

#[derive(IntoElement)]
pub struct DropdownMenuPopover<T: Selectable + IntoElement + 'static> {
    id: ElementId,
    style: StyleRefinement,
    anchor: Corner,
    trigger: T,
    builder: Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>,
}

impl<T> DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn new(
        id: ElementId,
        anchor: impl Into<Corner>,
        trigger: T,
        builder: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        Self {
            id: SharedString::from(format!("dropdown-menu:{:?}", id)).into(),
            style: StyleRefinement::default(),
            anchor: anchor.into(),
            trigger,
            builder: Rc::new(builder),
        }
    }

    /// Set the anchor corner for the dropdown menu popover.
    pub fn anchor(mut self, anchor: impl Into<Corner>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the style refinement for the dropdown menu trigger.
    fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.style = style;
        self
    }
}

impl<T> RenderOnce for DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let builder = self.builder.clone();
        let state = window.use_keyed_state(self.id, cx, |window, cx| {
            builder(PopupMenu::new(cx), window, cx)
        });

        Popover::new(SharedString::from(state.entity_id().to_string()))
            .appearance(false)
            .trigger(self.trigger)
            .trigger_style(self.style)
            .anchor(self.anchor)
            .track_focus(&state.focus_handle(cx))
            .content(move |window, cx| {
                let popover_state = cx.entity();

                // Listen for dismiss events from the PopupMenu to close the popover.
                window
                    .subscribe(&state, cx, move |_, _: &DismissEvent, window, cx| {
                        popover_state.update(cx, |state, cx| {
                            state.dismiss(window, cx);
                        });
                    })
                    .detach();

                state.clone()
            })
    }
}
