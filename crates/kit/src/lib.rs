//! GPUI Kit: one dependency for building desktop applications with GPUI.
//!
//! GPUI itself is published as a family of `gpui-pre-*` crates that move
//! together. This crate depends on the matching set for you and re-exports
//! each layer under a short name, so an application lists `gpui-kit` alone:
//!
//! | Path          | Crate                   | Feature       |
//! | ------------- | ----------------------- | ------------- |
//! | [`gpui`]      | `gpui`                  | always        |
//! | [`platform`]  | `gpui_platform`         | always        |
//! | [`base`]      | `gpui-base`             | `base` (on)      |
//! | [`component`] | `gpui-component`        | `component` (on) |
//! | [`assets`]    | `gpui-kit-assets` | `assets` (on)    |
//! | [`shell`]     | `gpui-shell`            | `shell` (on)     |
//! | [`webview`]   | `gpui-wry`              | `webview`        |
//!
//! [`prelude`] also brings the crate names themselves into scope, so code
//! written against `gpui::…` (including the `actions!` and `#[derive(Action)]`
//! macros) works unchanged:
//!
//! ```no_run
//! use gpui_kit::prelude::*;
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

pub use gpui;
pub use gpui_platform as platform;
#[cfg(target_family = "wasm")]
pub use gpui_web as web;

#[cfg(feature = "base")]
pub use gpui_base as base;
/// The styled component library, with `gpui_kit::init` initializing it.
///
/// ```no_run
/// use gpui_kit::component::button::*;
/// use gpui_kit::prelude::*;
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
pub use gpui_component as component;
#[cfg(feature = "assets")]
pub use gpui_kit_assets as assets;
#[cfg(feature = "shell")]
pub use gpui_shell as shell;
#[cfg(feature = "webview")]
pub use gpui_wry as webview;

pub use gpui_platform::application;

/// Initializes every enabled layer. Call it once, before using anything else.
///
/// With the `component` feature (on by default) this is
/// `gpui_component::init`, which also initializes `gpui-base`; with only
/// `base` it is `gpui_base::init`. The
/// `shell` runtime has its own [`shell::init`](gpui_shell::init) and
/// [`shell::init_with_components`](gpui_shell::init_with_components), which
/// the host calls when it registers its component catalog.
pub fn init(cx: &mut gpui::App) {
    #[cfg(feature = "component")]
    gpui_component::init(cx);
    #[cfg(all(feature = "base", not(feature = "component")))]
    gpui_base::init(cx);
    #[cfg(not(any(feature = "component", feature = "base")))]
    let _ = cx;
}

/// Everything an application file usually needs, plus the underlying crate
/// names so existing `gpui::…` and `gpui_component::…` paths keep working.
///
/// A few names exist in both `gpui` and `gpui_component` (`Size`, `Edges`,
/// and the color helpers such as `red`); qualify those at the use site, as
/// with `use gpui::*; use gpui_component::*;` today.
#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use gpui;
    pub use gpui::*;
    pub use gpui_platform;

    #[cfg(feature = "component")]
    pub use gpui_component;
    #[cfg(feature = "component")]
    pub use gpui_component::*;

    #[cfg(feature = "base")]
    pub use gpui_base;
    #[cfg(all(feature = "base", not(feature = "component")))]
    pub use gpui_base::*;

    #[cfg(feature = "assets")]
    pub use gpui_kit_assets;
    #[cfg(feature = "shell")]
    pub use gpui_shell;
    #[cfg(feature = "webview")]
    pub use gpui_wry;
}
