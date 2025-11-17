mod field;
mod group;
mod page;

pub use field::*;
use gpui::SharedString;
pub use group::*;
pub use page::*;

pub struct Settings {
    pages: Vec<SettingSection>,
}

/// A section in the settings, containing multiple pages.
pub struct SettingSection {
    title: SharedString,
    pages: Vec<SettingPage>,
}
