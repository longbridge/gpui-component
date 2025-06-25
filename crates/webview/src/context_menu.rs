use gpui::{Action, App, Entity, Window};
use gpui_component::popup_menu::PopupMenu;
use rust_i18n::t;
use schemars::JsonSchema;
use serde::Deserialize;
use wef::{ContextMenuParams, Frame, LogicalUnit, Point};

use crate::WebView;

#[derive(Action, Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = webview)]
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
        if params.type_.contains(TypeFlags::SELECTION) {
            popmenu = popmenu.menu(
                t!("WebView.ContextMenu.Copy"),
                Box::new(ContextMenuAction::Copy),
            );
        }

        if params.type_.contains(TypeFlags::LINK) {
            popmenu = popmenu.menu(
                t!("WebView.ContextMenu.CopyLinkAddress"),
                Box::new(ContextMenuAction::CopyLinkAddress),
            );
        } else if params.type_.contains(TypeFlags::EDITABLE) {
            popmenu = popmenu
                .menu_with_disabled(
                    t!("WebView.ContextMenu.Undo"),
                    Box::new(ContextMenuAction::Undo),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_UNDO),
                )
                .menu_with_disabled(
                    t!("WebView.ContextMenu.Redo"),
                    Box::new(ContextMenuAction::Redo),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_REDO),
                )
                .separator()
                .menu_with_disabled(
                    t!("WebView.ContextMenu.Cut"),
                    Box::new(ContextMenuAction::Cut),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_CUT),
                )
                .menu_with_disabled(
                    t!("WebView.ContextMenu.Copy"),
                    Box::new(ContextMenuAction::Copy),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_COPY),
                )
                .menu_with_disabled(
                    t!("WebView.ContextMenu.Paste"),
                    Box::new(ContextMenuAction::Paste),
                    !params.edit_state_flags.contains(EditStateFlags::CAN_PASTE),
                )
                .menu_with_disabled(
                    t!("WebView.ContextMenu.ParseAsPlainText"),
                    Box::new(ContextMenuAction::ParseAsPlainText),
                    !params
                        .edit_state_flags
                        .contains(EditStateFlags::CAN_EDIT_RICHLY),
                )
                .menu_with_disabled(
                    t!("WebView.ContextMenu.SelectAll"),
                    Box::new(ContextMenuAction::SelectAll),
                    !params
                        .edit_state_flags
                        .contains(EditStateFlags::CAN_SELECT_ALL),
                );
        } else if params.type_.contains(TypeFlags::PAGE) {
            popmenu = popmenu
                .menu_with_disabled(
                    t!("WebView.ContextMenu.Back"),
                    Box::new(ContextMenuAction::GoBack),
                    !webview.browser().can_back(),
                )
                .menu_with_disabled(
                    t!("WebView.ContextMenu.Forward"),
                    Box::new(ContextMenuAction::GoForward),
                    !webview.browser().can_forward(),
                )
                .menu(
                    t!("WebView.ContextMenu.Reload"),
                    Box::new(ContextMenuAction::Reload),
                )
        }

        cx.notify();
        popmenu
    })
}
