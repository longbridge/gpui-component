mod format;
mod inline;
mod node;
mod text_view;
mod utils;

use gpui::App;
pub use text_view::*;

pub(crate) fn init(cx: &mut App) {
    text_view::init(cx);
}
