//! Standalone NSH hosts using Kit's window root and component catalog.
//!
//! Install the two runners under `NSH_RUNTIME/gpui` to use the regular
//! `nsh run --target gpui` and `nsh package --target gpui` commands.

use std::{cell::Cell, process::ExitCode, rc::Rc};

use crate::nsh::runner;
use crate::nsh::{App, AppOptions, AppSource, AppView, Appearance};
use gpui_kit::{AppContext, Focusable};

/// Runs one NSH application with Kit assets, initialization, root and catalog.
///
/// The provided catalog is used unchanged, so hosts can extend
/// [`crate::components`] before passing it in `options.components`.
/// Scripted backend testing is not available through this entry point;
/// use the adapter's headless verification harness instead.
pub fn run(source: AppSource, options: AppOptions) -> ExitCode {
    if options.script.is_some() {
        eprintln!("NSH: backend --script is not supported by the Kit root host");
        return ExitCode::FAILURE;
    }
    let ok = Rc::new(Cell::new(true));
    let result = ok.clone();
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            match options.appearance {
                Appearance::Light => gpui_kit::component::Theme::change(
                    gpui_kit::component::ThemeMode::Light,
                    None,
                    cx,
                ),
                Appearance::Dark => gpui_kit::component::Theme::change(
                    gpui_kit::component::ThemeMode::Dark,
                    None,
                    cx,
                ),
                _ => gpui_kit::component::Theme::change(cx.window_appearance(), None, cx),
            }
            let window_options = AppView::window_options(&options, cx);
            let error_status = ok.clone();
            let window = gpui_kit::open_window(window_options, cx, move |window, cx| {
                let view = cx.new(|cx| AppView::new(options, window, cx));
                view.read(cx).own_window();
                cx.observe(&view, |view, cx| {
                    let dark = view.read(cx).is_dark();
                    if gpui_kit::component::Theme::global(cx).mode.is_dark() != dark {
                        gpui_kit::component::Theme::change(
                            if dark {
                                gpui_kit::component::ThemeMode::Dark
                            } else {
                                gpui_kit::component::ThemeMode::Light
                            },
                            None,
                            cx,
                        );
                    }
                })
                .detach();
                #[cfg(feature = "dev")]
                let development = matches!(&source, AppSource::DevServer(_));
                #[cfg(not(feature = "dev"))]
                let development = false;
                match source {
                    AppSource::Bundle(bundle) => view.read(cx).load(App::Js(bundle)),
                    AppSource::Rust(root) => view.read(cx).load(App::Rust(root)),
                    #[cfg(feature = "dev")]
                    AppSource::DevServer(url) => {
                        AppView::connect_dev_server(&view, url, cx);
                    }
                }
                // DevSession owns the runtime error callback for its live error
                // overlay. The release runner records fatal errors for its exit.
                if !development {
                    view.read(cx).on_error(move |error| {
                        eprintln!("NSH: {error}");
                        error_status.set(false);
                    });
                }
                window.focus(&view.read(cx).focus_handle(cx), cx);
                view
            });
            if let Err(error) = window {
                eprintln!("NSH: cannot open a window: {error}");
                ok.set(false);
                cx.quit();
                return;
            }
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
    if result.get() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Finds NSH resources relative to an installed release executable.
pub use crate::nsh::runner::resources;

/// Runs an installed package using NSH's standard resource layout.
pub fn packaged() -> ExitCode {
    runner::init_logging();
    crate::nsh::set_ui_sets(vec![nsh_ui_mobile::registry(), crate::ui_set()]);
    let options = AppOptions {
        components: crate::components(),
        ..Default::default()
    };
    match runner::package_source(options) {
        Ok((source, options)) => run(source, options),
        Err(error) => {
            eprintln!("NSH: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the development host with NSH's dev-server argument protocol.
#[cfg(feature = "dev")]
pub fn development(arguments: impl IntoIterator<Item = String>) -> ExitCode {
    crate::nsh::dev::init_logging();
    crate::nsh::set_ui_sets(vec![nsh_ui_mobile::registry(), crate::ui_set()]);
    let options = AppOptions {
        components: crate::components(),
        ..Default::default()
    };
    match runner::development_source(arguments, options) {
        Ok((source, options)) => run(source, options),
        Err(error) => {
            eprintln!("NSH: {error}");
            ExitCode::FAILURE
        }
    }
}
