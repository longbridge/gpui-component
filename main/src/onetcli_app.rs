
use gpui::{
    App, AppContext, Context, Entity, IntoElement, KeyBinding, ParentElement, Render, Styled, Task,
    Window, div, px,
};
use gpui_component::dock::{ClosePanel, ToggleZoom};
use gpui_component::{ActiveTheme, Root};
use one_core::tab_container::{
    TabContainer, TabContainerEvent, TabContainerState, TabContentRegistry, TabItem,
};
use one_core::tab_persistence::{load_tabs, save_tab_state, schedule_save};
use one_core::llm::manager::GlobalProviderState;
use reqwest_client::ReqwestClient;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::home_tab::HomePage;

pub fn init(cx: &mut App) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(
                    "gpui_component=trace"
                        .parse()
                        .expect("固定的日志指令解析不应失败"),
                )
                .add_directive("db=debug".parse().expect("固定的日志指令解析不应失败"))
                .add_directive("one-ui=debug".parse().expect("固定的日志指令解析不应失败"))
                .add_directive("one-core=debug".parse().expect("固定的日志指令解析不应失败"))
                .add_directive(
                    "terminal=debug"
                        .parse()
                        .expect("固定的日志指令解析不应失败"),
                )
                .add_directive(
                    "sftp_view=debug"
                        .parse()
                        .expect("固定的日志指令解析不应失败"),
                )
                .add_directive("main=debug".parse()
                    .expect("固定的日志指令解析不应失败"))
                .add_directive(
                    "terminal_view=debug"
                        .parse()
                        .expect("固定的日志指令解析不应失败"),
                ),
        )
        .init();
    let http_client =
        std::sync::Arc::new(ReqwestClient::user_agent("one-hub").expect("HTTP 客户端初始化失败"));
    cx.set_http_client(http_client);
    gpui_component::init(cx);
    one_core::init(cx);
    one_ui::init(cx);
    db_view::chatdb::agents::init(cx);
    crate::auth::init(cx);
    crate::license::init(cx);
    {
        let auth_service = crate::auth::get_auth_service(cx);
        let global_provider_state = cx.global::<GlobalProviderState>().clone();
        global_provider_state.set_cloud_client(auth_service.cloud_client());
    }
    db::init_cache(cx);
    // 启动后台磁盘缓存清理任务
    if let Some(cache) = cx.try_global::<db::GlobalNodeCache>() {
        cache.start_cleanup_task(cx);
    }
    terminal_view::init(cx);
    redis_view::init(cx);
    mongodb_view::init(cx);
    cx.bind_keys(vec![
        KeyBinding::new("shift-escape", ToggleZoom, None),
        KeyBinding::new("ctrl-w", ClosePanel, None),
    ]);

    let registry = TabContentRegistry::new();
    cx.set_global(registry);

    cx.activate(true);
}

pub struct OnetCliApp {
    tab_container: Entity<TabContainer>,
    last_layout_state: Option<TabContainerState>,
    _save_layout_task: Option<Task<()>>,
}

impl OnetCliApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tab_container = cx.new(|cx| {
            let mut container = TabContainer::new(window, cx)
                .with_tab_bar_colors(
                    Some(gpui::rgb(0x2b2b2b).into()),
                    Some(gpui::rgb(0x1e1e1e).into()),
                )
                .with_tab_item_colors(
                    Some(gpui::rgb(0x555555).into()),
                    Some(gpui::rgb(0x3a3a3a).into()),
                )
                .with_inactive_tab_bg_color(Some(gpui::rgb(0x3a3a3a).into()))
                .with_tab_content_colors(Some(gpui::white()), Some(gpui::rgb(0xaaaaaa).into()));

            #[cfg(target_os = "macos")]
            {
                container = container
                    .with_left_padding(px(80.0))
                    .with_top_padding(px(4.0))
            }

            #[cfg(not(target_os = "macos"))]
            {
                container = container.with_window_controls(true)
            }

            container
        });

        let registry = cx.global::<TabContentRegistry>().clone();

        match load_tabs(&tab_container, &registry, window, cx) {
            Ok(_) => {
                tracing::info!("Tab layout loaded successfully");
            }
            Err(err) => {
                tracing::error!("Failed to load tab layout: {:?}", err);
            }
        }

        let has_tabs = !tab_container.read(cx).tabs().is_empty();
        if !has_tabs {
            let tab_container_clone = tab_container.clone();
            tab_container.update(cx, |tc, cx| {
                let home_page = cx.new(|cx| HomePage::new(tab_container_clone, window, cx));
                let home_tab = TabItem::new("home", "app", home_page);
                tc.add_and_activate_tab_with_focus(home_tab, window, cx);
            });
        }

        cx.subscribe_in(
            &tab_container,
            window,
            |this, _tc, ev: &TabContainerEvent, _window, cx| {
                if matches!(ev, TabContainerEvent::LayoutChanged) {
                    this.save_layout(cx);
                }
            },
        )
        .detach();

        cx.on_app_quit({
            let tab_container = tab_container.clone();
            move |_, cx| {
                let state = tab_container.read(cx).dump(cx);
                cx.background_executor().spawn(async move {
                    if let Err(err) = save_tab_state(&state) {
                        tracing::error!("Failed to save tab state on quit: {:?}", err);
                    }
                })
            }
        })
        .detach();

        Self {
            tab_container,
            last_layout_state: None,
            _save_layout_task: None,
        }
    }

    fn save_layout(&mut self, cx: &mut App) {
        self._save_layout_task = Some(schedule_save(
            self.tab_container.clone(),
            &mut self.last_layout_state,
            cx,
        ));
    }
}

impl Render for OnetCliApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .relative()
            .bg(cx.theme().background)
            .child(div().size_full().child(self.tab_container.clone()))
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}
