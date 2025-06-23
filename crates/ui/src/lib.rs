mod colors;
mod event;
mod icon;
#[cfg(any(feature = "inspector", debug_assertions))]
mod inspector;
mod kbd;
mod menu;
mod root;
mod styled;
mod svg_img;
mod time;
mod title_bar;
mod virtual_list;
mod window_border;

pub(crate) mod actions;

pub mod accordion;
pub mod alert;
pub mod animation;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod chart;
pub mod checkbox;
pub mod clipboard;
pub mod color_picker;
pub mod description_list;
pub mod divider;
pub mod dock;
pub mod drawer;
pub mod dropdown;
pub mod form;
pub mod highlighter;
pub mod history;
pub mod indicator;
pub mod input;
pub mod label;
pub mod link;
pub mod list;
pub mod modal;
pub mod notification;
pub mod plot;
pub mod popover;
pub mod progress;
pub mod radio;
pub mod resizable;
pub mod scroll;
pub mod sidebar;
pub mod skeleton;
pub mod slider;
pub mod switch;
pub mod tab;
pub mod table;
pub mod tag;
pub mod text;
pub mod theme;
pub mod tooltip;

#[cfg(feature = "webview")]
pub mod webview;

use gpui::{App, SharedString};
// re-export
#[cfg(feature = "webview")]
pub use wry;

pub use crate::Disableable;
pub use event::InteractiveElementExt;
#[cfg(any(feature = "inspector", debug_assertions))]
pub use inspector::*;
pub use menu::{context_menu, popup_menu};
pub use root::{ContextModal, Root};
pub use styled::*;
pub use time::*;
pub use title_bar::*;
pub use virtual_list::{h_virtual_list, v_virtual_list, VirtualList};
pub use window_border::{window_border, window_paddings, WindowBorder};

pub use colors::*;
pub use icon::*;
pub use kbd::*;
pub use svg_img::*;
pub use theme::*;

use std::ops::Deref;

rust_i18n::i18n!("locales", fallback = "en");

/// Initialize the components.
///
/// You must initialize the components at your application's entry point.
pub fn init(cx: &mut App) {
    theme::init(cx);
    #[cfg(any(feature = "inspector", debug_assertions))]
    inspector::init(cx);

    root::init(cx);
    highlighter::init(cx);
    date_picker::init(cx);
    color_picker::init(cx);
    dock::init(cx);
    drawer::init(cx);
    dropdown::init(cx);
    input::init(cx);
    list::init(cx);
    modal::init(cx);
    popover::init(cx);
    menu::init(cx);
    table::init(cx);
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
pub fn measure(name: impl Into<SharedString>, f: impl FnOnce()) {
    measure_if(name, true, f);
}

pub struct Measure {
    name: SharedString,
    start: std::time::Instant,
}

impl Measure {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            start: std::time::Instant::now(),
        }
    }

    pub fn end(self) {
        let duration = self.start.elapsed();
        tracing::trace!("{} in {:?}", self.name, duration);
    }
}
