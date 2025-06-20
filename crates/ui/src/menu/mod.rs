use gpui::App;

pub mod context_menu;
pub mod popup_menu;

pub(crate) fn init(cx: &mut App) {
    popup_menu::init(cx);
}
