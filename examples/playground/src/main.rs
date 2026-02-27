use std::path::PathBuf;

use gpui::{
    AppContext, Bounds, Render, SharedString, Styled, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, point, px, size,
};
use gpui_component::{Root, Theme, ThemeRegistry};
use gpui_platform::application;

struct Example;

impl Render for Example {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div().size_full()
    }
}

fn main() {
    let app = application();

    app.run(move |cx| {
        gpui_component::init(cx);

        let theme_name = SharedString::from("Molokai Dark");

        if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("../../themes"), cx, move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
                Theme::global_mut(cx).apply_config(&theme);
            }
        }) {
            eprintln!("Failed to watch themes directory: {}", err);
        }

        let bounds = Bounds::centered(None, size(px(1000.), px(600.)), cx);

        cx.spawn(async move |cx| {
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::Normal,
                titlebar: Some(TitlebarOptions {
                    title: Some("Example".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(9.0), px(9.0))),
                }),
                ..Default::default()
            };
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| Example);

                cx.new(|cx| Root::new(view, window, cx).window_border_size(px(10.)))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
