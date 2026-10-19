//! Native Shell (NSH) in GPUI Kit apps.
//!
//! - [`NshView`] embeds an NSH app (TSX, or a Rust root) in a GPUI Kit
//!   window, in GPUI Kit's theme.
//! - NSH apps use GPUI Base's controls as native components ([`base`]):
//!   the behavior is GPUI Base's own, the presentation the app's, and the
//!   TypeScript declarations come from the same Rust [`Catalog`] that
//!   registers them.
//!
//! NSH draws with GPUI Fast, so this crate turns on GPUI Kit's `gpui-fast`
//! feature: the app and NSH share one GPUI.

#![cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]

pub mod base;
mod catalog;
#[cfg(test)]
mod testing;
mod theme;
mod view;

pub use catalog::{Catalog, Control, Events, PropType, Props, ROOT_STATES, StateStyles};
pub use nsh_platform_gpui::{self as platform, App as NshApp, AppOptions, AssetBase, Bundle, Root};
pub use view::NshView;
