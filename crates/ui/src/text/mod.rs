mod element;
mod html;
mod html5minify;
mod inline_text;
mod markdown;
mod text_view;
mod utils;

use gpui::App;
pub use text_view::*;

pub(crate) fn init(cx: &mut App) {
    text_view::init(cx);
}
