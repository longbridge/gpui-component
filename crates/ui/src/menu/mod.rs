use gpui::App;

mod menu_bar;
mod menu_item;

pub mod context_menu;
pub mod popup_menu;

pub use menu_bar::{MenuBar, MenuBarMenu};

pub(crate) fn init(cx: &mut App) {
    popup_menu::init(cx);
}
