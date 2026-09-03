//! Every path `gpui-kit` promises resolves, under every feature set.
//!
//! These are compile-time checks: each `use` or type alias fails to build if
//! a re-export goes missing or moves, which is the regression that matters
//! for an umbrella crate.

#![allow(unused_imports, dead_code)]

/// `use gpui_kit::*;` alone is GPUI, so a file needs nothing else for it.
mod glob_is_gpui {
    use gpui_kit::*;

    type Element = Div;
    type Window_ = Window;
    type App_ = App;
    type Geometry = Size<Pixels>;

    actions!(exports, [Quit]);

    fn build() -> impl IntoElement {
        div().child("hello").bg(red())
    }

    fn startup(cx: &mut App) {
        gpui_kit::init(cx);
        let _ = gpui_kit::application;
    }
}

/// `gpui_kit::gpui` is hidden but kept, for code that keeps `gpui::…` paths.
mod gpui_by_name {
    use gpui_kit::*;

    type Through = gpui::Window;
    type Spelled = gpui_kit::gpui::Window;
}

/// The layers that are always present.
mod always {
    type Base = gpui_kit::base::Button;
    const _APPLICATION: fn() -> gpui_kit::Application = gpui_kit::application;
    const _PLATFORM_APPLICATION: fn() -> gpui_kit::Application = gpui_kit::platform::application;
    const _INIT: fn(&mut gpui_kit::App) = gpui_kit::init;
}

#[cfg(feature = "component")]
mod component {
    use gpui_kit::component::button::*;
    use gpui_kit::component::{ActiveTheme, Root, Size};
    use gpui_kit::*;

    type Sizing = Size;

    fn theme(cx: &App) -> Hsla {
        cx.theme().background
    }

    fn button() -> Button {
        Button::new("ok").primary()
    }

    const _INIT: fn(&mut App) = gpui_kit::component::init;
}

#[cfg(feature = "assets")]
mod assets {
    type Assets = gpui_kit::assets::Assets;
}

#[cfg(feature = "shell")]
mod shell {
    const _INIT: fn(&mut gpui_kit::App) = gpui_kit::shell::init;
}
