use crate::ui::{main_window::TodoMainView, CloseWindow, Quit};
use gpui::*;
use gpui_component::TitleBar;
use story::Assets;

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
    let app = Application::new().with_assets(Assets);
    app.run(move |cx| {
        AppState::init(cx);
        cx.on_action(|_: &Quit, cx: &mut App| {
            println!("Quit action received, quitting the application.");
            cx.quit();
        });
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

        crate::ui::create_todo_window_options(
            options,
            move |window, cx| TodoMainView::view(window, cx),
            cx,
        );
    });
}
