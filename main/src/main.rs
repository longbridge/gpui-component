rust_i18n::i18n!("locales", fallback = "en");

mod auth;

mod home;
mod home_tab;
mod license;
mod onehub_app;
mod setting_tab;
mod settings;
mod update;
mod user_avatar;
mod encourage;

use crate::onehub_app::OneHubApp;
use db::GlobalDbState;
use db_view::database_view_plugin::DatabaseViewPluginRegistry;
use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;

fn main() {
    if update::handle_update_command() {
        return;
    }

    // 加载 .env 文件（开发环境）
    // 优先级：.env.local > .env
    let _ = dotenvy::from_filename(".env.local");
    let _ = dotenvy::dotenv();

    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        onehub_app::init(cx);
        setting_tab::init_settings(cx);
        // Initialize global database state
        let db_state = GlobalDbState::new();
        // Start cleanup task
        db_state.start_cleanup_task(cx);
        cx.set_global(db_state);

        // Initialize Ask AI notifier
        db_view::init_ask_ai_notifier(cx);

        // Initialize database view plugin registry
        let view_registry = DatabaseViewPluginRegistry::new();
        cx.set_global(view_registry);
        let mut window_size = size(px(1600.0), px(1200.0));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            #[cfg(not(target_os = "linux"))]
            titlebar: Some(gpui_component::TitleBar::title_bar_options()),
            window_min_size: Some(Size {
                width: px(640.),
                height: px(480.),
            }),
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            kind: WindowKind::Normal,
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(options, |window, cx| {
                update::schedule_update_check(window, cx);
                let view = cx.new(|cx| OneHubApp::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
