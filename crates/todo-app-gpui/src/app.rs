use crate::models::mcp_config::McpProviderManager;
use crate::models::profile_config::ProfileManager;
use crate::models::provider_config::LlmProviderManager;
use crate::models::todo_item::TodoManager;
use crate::ui::assets::Assets;

use crate::ui::components::container::Container;
use crate::ui::components::titlebar::TitleBar;
use crate::ui::components::ViewKit;
use crate::ui::views::introduction::Introduction;
use crate::ui::views::llm_provider::LlmProvider;
use crate::ui::views::mcp_provider::McpProvider;
use crate::ui::views::profile::Profile;
use crate::ui::views::settings::Settings;
use crate::ui::{main_window::TodoMainWindow, AppExt};
use gpui::*;
use gpui_component::dock::{register_panel, PanelControl, PanelInfo};
use serde::{Deserialize, Serialize};
actions!(story, [Quit, Open, CloseWindow, ToggleSearch]);

/// 故事状态，用于序列化和反序列化故事信息
#[derive(Debug, Serialize, Deserialize)]
pub struct ViewKitState {
    pub story_klass: SharedString, // 故事类名
}

impl ViewKitState {
    /// 转换为 JSON 值
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "story_klass": self.story_klass,
        })
    }

    /// 从 JSON 值创建
    pub fn from_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }

    /// 转换为故事元组
    pub fn to_viewkit(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> (
        &'static str,
        &'static str,
        bool,
        Option<PanelControl>,
        AnyView,
        fn(AnyView, bool, &mut Window, &mut App),
    ) {
        // 宏定义：简化故事创建代码
        macro_rules! story {
            ($klass:tt) => {
                (
                    $klass::title(),
                    $klass::description(),
                    $klass::closable(),
                    $klass::zoomable(),
                    $klass::view(window, cx).into(),
                    $klass::on_active_any,
                )
            };
        }

        // 根据故事类名匹配对应的故事
        match self.story_klass.to_string().as_str() {
            "Introduction" => story!(Introduction),
            "LlmProvider" => story!(LlmProvider),
            _ => {
                unreachable!("Invalid story klass: {}", self.story_klass)
            }
        }
    }
}

/// 应用程序状态，管理全局状态
pub struct AppState {
    /// 不可见面板的列表
    pub invisible_panels: Entity<Vec<SharedString>>,
    pub profile_manager: ProfileManager,
    pub llm_provider: LlmProviderManager,
    pub mcp_provider: McpProviderManager,
    pub todo_manager: TodoManager,
}

/// 面板名称常量
const PANEL_NAME: &str = "Container";

// 实现全局状态特征
impl Global for AppState {}

impl AppState {
    /// 初始化应用程序状态
    fn init(cx: &mut App) {
        let state = Self {
            invisible_panels: cx.new(|_| Vec::new()),
            profile_manager: ProfileManager::load(),
            llm_provider: LlmProviderManager::load(),
            mcp_provider: McpProviderManager::load(),
            todo_manager: TodoManager::create_fake_data(),
        };
        cx.set_global::<AppState>(state);
    }

    /// 获取全局应用程序状态的不可变引用
    pub fn state(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// 获取全局应用程序状态的可变引用
    pub fn state_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}
actions!(input_story, [Tab, TabPrev]);

const CONTEXT: &str = "InputStory";

pub fn run() {
    const WIDTH: f32 = 400.0;
    const HEIGHT: f32 = WIDTH * 2.2;
    let app = Application::new().with_assets(Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        AppState::init(cx);
        Profile::init(cx);
        LlmProvider::init(cx);
        McpProvider::init(cx);
        Settings::init(cx);

        cx.on_action(|_: &Quit, cx: &mut App| {
            println!("Quit action received, quitting the application.");
            cx.quit();
        });

        // 注册面板
        register_panel(cx, PANEL_NAME, |_, _, info, window, cx| {
            let story_state = match info {
                PanelInfo::Panel(value) => ViewKitState::from_value(value.clone()),
                _ => {
                    unreachable!("Invalid PanelInfo: {:?}", info)
                }
            };

            let view = cx.new(|cx| {
                let (title, description, closable, zoomable, story, on_active) =
                    story_state.to_viewkit(window, cx);
                let mut container = Container::new(window, cx)
                    .story(story, story_state.story_klass)
                    .on_active(on_active);

                // 监听焦点变化
                cx.on_focus_in(
                    &container.focus_handle,
                    window,
                    |this: &mut Container, _, _| {
                        println!("Container focus in: {}", this.name);
                    },
                )
                .detach();

                container.name = title.into();
                container.description = description.into();
                container.closable = closable;
                container.zoomable = zoomable;
                container
            });
            Box::new(view)
        });

        // 设置应用程序菜单
        use gpui_component::input::{Copy, Cut, Paste, Redo, Undo};
        cx.set_menus(vec![
            Menu {
                name: "GPUI App".into(),
                items: vec![MenuItem::action("Quit", Quit)],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::os_action("Undo", Undo, gpui::OsAction::Undo),
                    MenuItem::os_action("Redo", Redo, gpui::OsAction::Redo),
                    MenuItem::separator(),
                    MenuItem::os_action("Cut", Cut, gpui::OsAction::Cut),
                    MenuItem::os_action("Copy", Copy, gpui::OsAction::Copy),
                    MenuItem::os_action("Paste", Paste, gpui::OsAction::Paste),
                ],
            },
            Menu {
                name: "Window".into(),
                items: vec![],
            },
        ]);

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
        cx.activate(true);
    });
}
