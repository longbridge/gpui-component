//! `gpui-shell` — runs a script application directory.
//!
//! ```text
//! cargo run -p gpui-shell -- examples/js_checklist
//! ```

use std::path::PathBuf;

use gpui::{
    AnyView, AppContext as _, Bounds, Context, IntoElement, Render, TitlebarOptions, Window,
    WindowBounds, WindowOptions, px, size,
};
use gpui_shell::{ScriptView, ShellRoot, ShellRuntime};

fn main() {
    let Some(directory) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: gpui-shell <app-directory>");
        std::process::exit(2);
    };

    gpui_platform::application().run(move |cx| {
        gpui_shell::init(cx);

        let runtime = match ShellRuntime::new() {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("failed to start the script runtime: {error}");
                std::process::exit(1);
            }
        };
        runtime.set_global(cx);

        let loaded = runtime
            .load_app(&directory)
            .and_then(|view_type| runtime.instantiate(&view_type));

        let title = directory
            .file_name()
            .map(|name| format!("{} — gpui-shell", name.to_string_lossy()))
            .unwrap_or_else(|| "gpui-shell".to_owned());
        let bounds = Bounds::centered(None, size(px(880.), px(720.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // A window with no title is unidentifiable in a switcher or a tiling
            // layout, which is how this one first reached a user.
            titlebar: Some(TitlebarOptions {
                title: Some(title.clone().into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let content: AnyView = match loaded {
                Ok(object) => cx.new(|_| ScriptView::new(runtime.clone(), object)).into(),
                Err(error) => {
                    eprintln!("{error}");
                    cx.new(|_| LoadFailure(error.to_string())).into()
                }
            };
            cx.new(|cx| ShellRoot::new(content, window, cx))
        })
        .expect("failed to open window");
    });
}

/// What the window shows when the application could not be loaded.
///
/// A failed load still opens a window: the error belongs on screen, not only in
/// a terminal the user may not be watching.
struct LoadFailure(String);

impl Render for LoadFailure {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_shell::runtime::failure_surface(
            "This application could not be loaded",
            &self.0,
            "gpui-shell <directory> expects main.js in that directory, \
             default-exporting a class that extends View.",
            window,
            cx,
        )
    }
}
