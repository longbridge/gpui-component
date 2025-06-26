use gpui::App;

mod menu_item;

pub mod context_menu;
pub mod popup_menu;

pub fn init(cx: &mut App) {
    popup_menu::init(cx);
}
