use gpui::App;

mod menu_item;

pub mod context_menu;
pub mod popup_menu;

pub(crate) fn init(cx: &mut App) {
    popup_menu::init(cx);
}
