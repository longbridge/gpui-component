use gpui::App;

mod app_menu_bar;
mod menu_item;

pub mod context_menu;
pub mod popup_menu;

pub use app_menu_bar::AppMenuBar;

pub(crate) fn init(cx: &mut App) {
    app_menu_bar::init(cx);
    popup_menu::init(cx);
}
