use crate::ui::main_window::TodoMainView;
use gpui::*;
use gpui_component::TitleBar;
use story::Assets;

pub fn run() {
    let app = Application::new().with_assets(Assets);
    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);
        let window_size = size(px(400.0), px(600.0));
        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(gpui::Size {
                width: px(400.),
                height: px(600.),
            }),

            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        story::create_new_window_options(
            "xTodo",
            options,
            move |window, cx| TodoMainView::view(window, cx),
            cx,
        );
    });
}
