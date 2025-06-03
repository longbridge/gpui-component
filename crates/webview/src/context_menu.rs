use gpui::{App, Entity, Window, impl_actions};
use gpui_component::popup_menu::PopupMenu;
use schemars::JsonSchema;
use serde::Deserialize;
use wef::{ContextMenuParams, Frame, LogicalUnit, Point};

use crate::WebView;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) enum ContextMenuAction {
    CopyLinkAddress,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    ParseAsPlainText,
    SelectAll,
    GoBack,
    GoForward,
    Reload,
}

impl_actions!(webview, [ContextMenuAction]);

pub(crate) struct ContextMenuInfo {
    pub(crate) crood: Point<LogicalUnit<i32>>,
    pub(crate) frame: Frame,
    pub(crate) menu: Entity<PopupMenu>,
    pub(crate) link_url: Option<String>,
}

pub(crate) fn build_context_menu(
    webview: &WebView,
    params: &ContextMenuParams,
    window: &mut Window,
    cx: &mut App,
) -> Entity<PopupMenu> {
    use wef::{ContextMenuEditStateFlags as EditStateFlags, ContextMenuTypeFlags as TypeFlags};

    PopupMenu::build(window, cx, |mut popmenu, _window, cx| {
        if params.type_.contains(TypeFlags::LINK) {
            popmenu = popmenu.menu(
                "Copy link address",
                Box::new(ContextMenuAction::CopyLinkAddress),
            );
        } else if params.type_.contains(TypeFlags::EDITABLE) {
            popmenu = popmenu
                .menu_with_disabled(
                    "Undo",
                    Box::new(ContextMenuAction::Undo),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_UNDO),
                )
                .menu_with_disabled(
                    "Redo",
                    Box::new(ContextMenuAction::Redo),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_REDO),
                )
                .separator()
                .menu_with_disabled(
                    "Cut",
                    Box::new(ContextMenuAction::Cut),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_CUT),
                )
                .menu_with_disabled(
                    "Copy",
                    Box::new(ContextMenuAction::Copy),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_COPY),
                )
                .menu_with_disabled(
                    "Paste",
                    Box::new(ContextMenuAction::Paste),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_PASTE),
                )
                .menu_with_disabled(
                    "Parse as plain text",
                    Box::new(ContextMenuAction::ParseAsPlainText),
                    !params
                        .edit_state_flags
                        .contains(EditStateFlags::CAN_EDIT_RICHLY),
                )
                .menu_with_disabled(
                    "Select all",
                    Box::new(ContextMenuAction::SelectAll),
                    !params
                        .edit_state_flags
                        .contains(EditStateFlags::CAN_SELECT_ALL),
                );
        } else if params.type_.contains(TypeFlags::PAGE) {
            popmenu = popmenu
                .menu_with_disabled(
                    "Back",
                    Box::new(ContextMenuAction::GoBack),
                    !webview.browser().can_back(),
                )
                .menu_with_disabled(
                    "Forward",
                    Box::new(ContextMenuAction::GoForward),
                    !webview.browser().can_forward(),
                )
                .menu("Reload", Box::new(ContextMenuAction::Reload))
        }

        cx.notify();
        popmenu
    })
}
