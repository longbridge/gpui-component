//! GPUI Kit: one dependency for building desktop applications with GPUI.
//!
//! GPUI itself is published as a family of `gpui-pre-*` crates that move
//! together. This crate depends on the matching set for you, so an
//! application lists `gpui-kit` alone. `use gpui_kit::*;` is GPUI, and each
//! layer is reachable by name:
//!
//! | Path            | Crate             | Feature          |
//! | --------------- | ----------------- | ---------------- |
//! | `gpui_kit::*`   | `gpui`            | always           |
//! | [`platform`]    | `gpui_platform`   | always           |
//! | [`base`]        | `gpui-base`       | always           |
//! | [`component`]   | `gpui-component`  | `component` (on) |
//! | [`assets`]      | `gpui-kit-assets` | `assets` (on)    |
//! | [`shell`]       | `gpui-shell`      | `shell` (on)     |
//!
//! [`application`] opens the platform and [`init`] initializes the enabled
//! layers:
//!
//! ```no_run
//! use gpui_kit::*;
//!
//! actions!(hello, [Quit]);
//!
//! struct Hello;
//!
//! impl Render for Hello {
//!     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
//!         div().child("Hello, World!")
//!     }
//! }
//!
//! fn main() {
//!     gpui_kit::application().run(|cx| {
//!         gpui_kit::init(cx);
//!         cx.spawn(async move |cx| {
//!             cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Hello))
//!                 .expect("failed to open window");
//!         })
//!         .detach();
//!     });
//! }
//! ```
//!
//! See [`component`] for the same program with the styled component library.

// Everything in GPUI itself, so `use gpui_kit::*;` is enough to get started.
// With the `test-support` feature the glob also carries GPUI's `test`
// attribute, so a test module imports explicitly (or adds
// `use core::prelude::v1::test;`) to keep the built-in `#[test]`.
pub use ::gpui::*;

// The crate name, so code that keeps `gpui::…` paths still resolves after
// `use gpui_kit::*;`. `gpui_kit::*` is the documented way.
#[doc(hidden)]
pub use ::gpui;

pub use ::gpui_base as base;
pub use ::gpui_platform as platform;
#[cfg(target_family = "wasm")]
pub use ::gpui_web as web;

/// The styled component library.
///
/// ```no_run
/// use gpui_kit::component::button::*;
/// use gpui_kit::component::Root;
/// use gpui_kit::*;
///
/// struct Hello;
///
/// impl Render for Hello {
///     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
///         div().child(Button::new("ok").primary().label("Let's Go!"))
///     }
/// }
///
/// fn main() {
///     gpui_kit::application().run(|cx| {
///         gpui_kit::init(cx);
///         cx.spawn(async move |cx| {
///             cx.open_window(WindowOptions::default(), |window, cx| {
///                 let view = cx.new(|_| Hello);
///                 cx.new(|cx| Root::new(view, window, cx))
///             })
///             .expect("failed to open window");
///         })
///         .detach();
///     });
/// }
/// ```
#[cfg(feature = "component")]
pub use ::gpui_component as component;
#[cfg(feature = "assets")]
pub use ::gpui_kit_assets as assets;
#[cfg(feature = "shell")]
pub use ::gpui_shell as shell;

pub use ::gpui_platform::application;

/// Initializes every enabled layer. Call it once, before using anything else.
///
/// With the `component` feature (on by default) this is
/// `gpui_component::init`, which also initializes `gpui-base`; otherwise it
/// is `gpui_base::init`. The `shell` runtime has its own
/// [`shell::init`](gpui_shell::init) and
/// [`shell::init_with_components`](gpui_shell::init_with_components), which
/// the host calls when it registers its component catalog.
pub fn init(cx: &mut App) {
    #[cfg(feature = "component")]
    gpui_component::init(cx);
    #[cfg(not(feature = "component"))]
    gpui_base::init(cx);
}
