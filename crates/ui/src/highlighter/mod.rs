mod diagnostics;
mod highlighter;
mod languages;
mod registry;

pub use diagnostics::*;
pub use highlighter::*;
pub use languages::*;
pub use registry::*;

use gpui::App;

pub fn init(cx: &mut App) {
    registry::init(cx);
}
