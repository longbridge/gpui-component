//! A GPUI Kit window embedding an NSH app (`app/App.tsx`) that uses GPUI
//! Base's controls.
//!
//! ```sh
//! make example-nsh
//! ```

use std::path::Path;

use gpui_kit::{
    component::{ActiveTheme as _, Sizable as _, Theme, ThemeMode, button::Button, h_flex, v_flex},
    *,
};
use gpui_nsh::{AppOptions, AssetBase, Bundle, NshApp, NshView};

struct Example {
    app: Entity<NshView>,
}

impl Example {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("build");
        let options = AppOptions {
            asset_base: Some(AssetBase::Directory(build.join("assets"))),
            ..AppOptions::default()
        };
        let app = cx.new(|cx| NshView::new(options, window, cx));
        // SAFETY: the bundle is this example's own `nsh build` output.
        match unsafe { Bundle::read_trusted(&build.join("bundle.nshc")) } {
            Ok(bundle) => app.read(cx).load(NshApp::Js(bundle), cx),
            Err(err) => {
                eprintln!("Run `make example-nsh` from the repository root: {err}")
            }
        }
        cx.new(|_| Self { app })
    }

    fn toggle_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = if cx.theme().mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        Theme::change(mode, Some(window), cx);
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = cx.theme().mode.is_dark();
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().flex_1().text_sm().child("NSH app in GPUI Kit"))
                    .child(
                        Button::new("mode")
                            .small()
                            .outline()
                            .label(if dark { "Light" } else { "Dark" })
                            .on_click(
                                cx.listener(|this, _, window, cx| this.toggle_mode(window, cx)),
                            ),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.app.clone()))
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            let options = WindowOptions {
                // NSH animates at full rate while the window is unfocused.
                inactive_frame_interval: None,
                ..WindowOptions::default()
            };
            gpui_kit::open_window(options, cx, Example::new).expect("Failed to open window");
        });
}
