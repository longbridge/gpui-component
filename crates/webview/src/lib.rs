#![doc = include_str!("../README.md")]

mod browser_handler;
mod context_menu;
mod element;
mod frame_view;
mod utils;
mod webview;

pub mod events;

pub use webview::WebView;
pub use wef;

rust_i18n::i18n!("locales", fallback = "en");
