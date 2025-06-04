use crate::ui::assets::Assets;
use crate::ui::components::titlebar::TitleBar;
use crate::ui::{main_window::TodoMainWindow, AppExt, CloseWindow, Quit};
use gpui::*;

/// 应用程序状态，管理全局状态
pub struct AppState {
    /// 不可见面板的列表
    pub invisible_panels: Entity<Vec<SharedString>>,
}
// 实现全局状态特征
impl Global for AppState {}

impl AppState {
    /// 初始化应用程序状态
    fn init(cx: &mut App) {
        gpui_component::init(cx);
        let state = Self {
            invisible_panels: cx.new(|_| Vec::new()),
        };
        cx.set_global::<AppState>(state);
    }

    /// 获取全局应用程序状态的不可变引用
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// 获取全局应用程序状态的可变引用
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

pub fn run() {
    const WIDTH: f32 = 400.0;
    const HEIGHT: f32 = WIDTH * 2.2;
    let app = Application::new().with_assets(Assets);
    app.run(move |cx| {
        AppState::init(cx);
        cx.on_action(|_: &Quit, cx: &mut App| {
            println!("Quit action received, quitting the application.");
            cx.quit();
        });
        cx.activate(true);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let window_size = size(px(WIDTH), px(HEIGHT));
        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: None,

            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };
        cx.create_todo_window(options, move |window, cx| TodoMainWindow::view(window, cx));
    });
}
