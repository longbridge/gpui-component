use std::collections::HashSet;

use gpui::{App, ElementId, FocusHandle, Global, OwnedMenu};

/// Application-wide state shared by Base behaviors.
pub struct GlobalState {
    app_menus: Vec<OwnedMenu>,
    deferred_popovers: HashSet<ElementId>,
    suppress_text_selection: bool,
}

impl Global for GlobalState {}

impl GlobalState {
    fn new() -> Self {
        Self {
            app_menus: Vec::new(),
            deferred_popovers: HashSet::new(),
            suppress_text_selection: false,
        }
    }

    /// Ensures that the Base global exists.
    #[doc(hidden)]
    pub fn init(cx: &mut App) {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::new());
        }
    }

    /// Suppresses window-level text selection for the current mouse down.
    ///
    /// Controls that own a press or drag interaction use this so the same
    /// pointer event does not also start application text selection.
    pub fn suppress_text_selection(cx: &mut App) {
        Self::global_mut(cx).suppress_text_selection = true;
    }

    /// Clears the current mouse-down text-selection suppression.
    #[doc(hidden)]
    pub fn reset_text_selection_suppression(cx: &mut App) {
        Self::global_mut(cx).suppress_text_selection = false;
    }

    /// Returns whether the current mouse down suppresses text selection.
    #[doc(hidden)]
    pub fn is_text_selection_suppressed(cx: &App) -> bool {
        Self::global(cx).suppress_text_selection
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Returns the application menus.
    pub fn app_menus(&self) -> &[OwnedMenu] {
        &self.app_menus
    }

    /// Replaces the application menus.
    pub fn set_app_menus(&mut self, menus: Vec<OwnedMenu>) {
        self.app_menus = menus;
    }

    /// Returns whether any deferred popup currently owns an open interaction
    /// context.
    pub fn is_in_deferred_context(cx: &App) -> bool {
        !Self::global(cx).deferred_popovers.is_empty()
    }

    /// Registers an open deferred popup by its focus identity.
    pub fn register_deferred_popover(focus_handle: &FocusHandle, cx: &mut App) {
        Self::global_mut(cx)
            .deferred_popovers
            .insert(format!("{focus_handle:?}").into());
    }

    /// Removes a deferred popup from the open interaction context.
    pub fn unregister_deferred_popover(focus_handle: &FocusHandle, cx: &mut App) {
        let id: ElementId = format!("{focus_handle:?}").into();
        Self::global_mut(cx).deferred_popovers.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn initialization_is_idempotent_and_suppression_can_be_reset(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            GlobalState::init(cx);
            GlobalState::suppress_text_selection(cx);
            GlobalState::init(cx);
            assert!(GlobalState::is_text_selection_suppressed(cx));

            GlobalState::reset_text_selection_suppression(cx);
            assert!(!GlobalState::is_text_selection_suppressed(cx));

            let focus_handle = cx.focus_handle();
            assert!(!GlobalState::is_in_deferred_context(cx));
            GlobalState::register_deferred_popover(&focus_handle, cx);
            assert!(GlobalState::is_in_deferred_context(cx));
            GlobalState::unregister_deferred_popover(&focus_handle, cx);
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }
}
