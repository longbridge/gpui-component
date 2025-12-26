use gpui::{App, SharedString};
use std::ops::Deref;

mod element_ext;
mod event;
mod geometry;
mod global_state;
mod icon;
mod index_path;
#[cfg(any(feature = "inspector", debug_assertions))]
mod inspector;
mod root;
mod styled;
mod time;
mod title_bar;
mod virtual_list;
mod window_border;
mod window_ext;

pub(crate) mod actions;

pub mod accordion;
pub mod alert;
pub mod animation;
pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod chart;
pub mod checkbox;
pub mod clipboard;
pub mod collapsible;
pub mod color_picker;
pub mod description_list;
pub mod dialog;
pub mod divider;
pub mod dock;
pub mod form;
pub mod group_box;
pub mod highlighter;
pub mod history;
pub mod input;
pub mod kbd;
pub mod label;
pub mod link;
pub mod list;
pub mod menu;
pub mod notification;
pub mod pagination;
pub mod plot;
pub mod popover;
pub mod progress;
pub mod radio;
pub mod resizable;
pub mod scroll;
pub mod select;
pub mod setting;
pub mod sheet;
pub mod sidebar;
pub mod skeleton;
pub mod slider;
pub mod spinner;
pub mod stepper;
pub mod switch;
pub mod tab;
pub mod table;
pub mod tag;
pub mod text;
pub mod theme;
pub mod tooltip;
pub mod tree;

pub use crate::Disableable;
pub use element_ext::ElementExt;
pub use event::InteractiveElementExt;
pub use geometry::*;
pub use icon::*;
pub use index_path::IndexPath;
pub use input::{Rope, RopeExt, RopeLines};
#[cfg(any(feature = "inspector", debug_assertions))]
pub use inspector::*;
pub use root::Root;
pub use styled::*;
pub use theme::*;
pub use time::{calendar, date_picker};
pub use title_bar::*;
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
pub use window_border::{WindowBorder, window_border, window_paddings};
pub use window_ext::WindowExt;

rust_i18n::i18n!("locales", fallback = "en");

/// Initialize the components.
///
/// You must initialize the components at your application's entry point.
pub fn init(cx: &mut App) {
    theme::init(cx);
    global_state::init(cx);
    #[cfg(any(feature = "inspector", debug_assertions))]
    inspector::init(cx);
    root::init(cx);
    color_picker::init(cx);
    date_picker::init(cx);
    dock::init(cx);
    sheet::init(cx);
    select::init(cx);
    input::init(cx);
    list::init(cx);
    dialog::init(cx);
    popover::init(cx);
    menu::init(cx);
    table::init(cx);
    text::init(cx);
    tree::init(cx);
}

#[inline]
pub fn locale() -> impl Deref<Target = str> {
    rust_i18n::locale()
}

#[inline]
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale)
}

#[inline]
pub(crate) fn measure_enable() -> bool {
    std::env::var("ZED_MEASUREMENTS").is_ok() || std::env::var("GPUI_MEASUREMENTS").is_ok()
}

/// Measures the execution time of a function and logs it if `if_` is true.
///
/// And need env `GPUI_MEASUREMENTS=1`
#[inline]
#[track_caller]
pub fn measure_if(name: impl Into<SharedString>, if_: bool, f: impl FnOnce()) {
    if if_ && measure_enable() {
        let measure = Measure::new(name);
        f();
        measure.end();
    } else {
        f();
    }
}

/// Measures the execution time.
#[inline]
#[track_caller]
pub fn measure(name: impl Into<SharedString>, f: impl FnOnce()) {
    measure_if(name, true, f);
}

pub struct Measure {
    name: SharedString,
    start: std::time::Instant,
}

impl Measure {
    #[track_caller]
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            start: std::time::Instant::now(),
        }
    }

    #[track_caller]
    pub fn end(self) {
        let duration = self.start.elapsed();
        tracing::trace!("{} in {:?}", self.name, duration);
    }
}
