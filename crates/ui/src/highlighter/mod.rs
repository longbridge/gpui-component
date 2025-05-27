mod highlight;
mod languages;

pub use highlight::*;
pub use languages::*;

use gpui::App;

pub fn init(cx: &mut App) {
    highlight::init(cx);
}
