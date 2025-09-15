mod code_action_menu;
mod completion_menu;
mod diagnostic_popover;

pub(crate) use code_action_menu::*;
pub(crate) use completion_menu::*;
pub(crate) use diagnostic_popover::*;
use gpui::{App, Entity, IntoElement};

pub(crate) enum ContextMenu {
    Completion(Entity<CompletionMenu>),
    CodeAction(Entity<CodeActionMenu>),
}

impl ContextMenu {
    pub(crate) fn is_open(&self, cx: &App) -> bool {
        match self {
            ContextMenu::Completion(menu) => menu.read(cx).is_open(),
            ContextMenu::CodeAction(menu) => menu.read(cx).is_open(),
        }
    }

    pub(crate) fn render(&self) -> impl IntoElement {
        match self {
            ContextMenu::Completion(menu) => menu.clone().into_any_element(),
            ContextMenu::CodeAction(menu) => menu.clone().into_any_element(),
        }
    }
}
