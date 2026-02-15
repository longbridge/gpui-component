use std::ops::Deref;

use crate::database_objects_tab::DatabaseObjectsPanel;
use crate::sidebar::{DatabaseSidebar, DatabaseSidebarEvent, TOOLBAR_WIDTH, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH};
use crate::db_tree_event::DatabaseEventHandler;
use crate::db_tree_view::DbTreeView;
use crate::sql_editor_view::SqlEditorTab;
use one_ui::resize_handle::{resize_handle, HandlePlacement, ResizePanel};
use db::GlobalDbState;
use gpui::{
    AnyElement, App, AppContext, AsyncApp, Axis, Bounds, Context, Element, Entity,
    EventEmitter, FocusHandle, Focusable, FontWeight, Hsla, InteractiveElement, IntoElement,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, SharedString,
    Style, Styled, Task, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, h_flex, v_flex,
};
use one_core::ai_chat::{CodeBlockAction, LanguageMatcher};
use one_core::storage::{ActiveConnections, Workspace};
use one_core::{
    storage::StoredConnection,
    tab_container::{TabContainer, TabContent, TabContentEvent, TabItem},
};
use uuid::Uuid;

const PANEL_MIN_SIZE: Pixels = px(100.0);
const TREE_PANEL_DEFAULT_SIZE: Pixels = px(250.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizingPanel {
    TreePanel,
    Sidebar,
}

pub struct DatabaseTabView {
    connections: Vec<StoredConnection>,
    tab_container: Entity<TabContainer>,
    db_tree_view: Entity<DbTreeView>,
    status_msg: Entity<String>,
    is_connected: Entity<bool>,
    _event_handler: Entity<DatabaseEventHandler>,
    workspace: Option<Workspace>,
    focus_handle: FocusHandle,
    sidebar: Entity<DatabaseSidebar>,
    _subscriptions: Vec<gpui::Subscription>,
    tree_panel_size: Pixels,
    sidebar_panel_size: Pixels,
    resizing: Option<ResizingPanel>,
    bounds: Bounds<Pixels>,
}

impl DatabaseTabView {
    pub fn new_with_active_conn(
        workspace: Option<Workspace>,
        connections: Vec<StoredConnection>,
        active_conn_id: Option<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let db_tree_view = cx.new(|cx| DbTreeView::new(&connections, window, cx));

        let tab_container = cx.new(|cx| TabContainer::new(window, cx));

        let objects_panel = cx.new(|cx| DatabaseObjectsPanel::new(workspace.clone(), window, cx));

        tab_container.update(cx, |container, cx| {
            let panel = objects_panel.clone();
            let tab = TabItem::new("objects-panel", "database", panel);
            container.add_and_activate_tab_with_focus(tab, window, cx);
        });

        let status_msg = cx.new(|_| "Ready".to_string());
        let is_connected = cx.new(|_| true);

        let event_handler = cx.new(|cx| {
            DatabaseEventHandler::new(
                &db_tree_view,
                tab_container.clone(),
                objects_panel.clone(),
                window,
                cx,
            )
        });

        let sidebar = cx.new(|cx| DatabaseSidebar::new(window, cx));

        // 注册 SQL 代码块操作
        Self::register_sql_code_block_actions(
            &sidebar,
            tab_container.clone(),
            &connections,
            cx,
        );

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(&sidebar, |_this, _, event: &DatabaseSidebarEvent, cx| {
            match event {
                DatabaseSidebarEvent::PanelChanged => {
                    cx.notify();
                }
                DatabaseSidebarEvent::AskAi => {
                    cx.notify();
                }
            }
        }));

        let mut global_state = cx.global::<GlobalDbState>().clone();

        let connections_clone = connections.clone();
        let clone_db_tree_view = db_tree_view.clone();
        cx.spawn(async move |_handle, cx: &mut AsyncApp| {
            for conn in &connections_clone {
                if let Ok(db_config) = conn.to_db_connection() {
                    let _ = global_state.register_connection(db_config);
                }
            }
            if let Some(id) = active_conn_id {
                _ = clone_db_tree_view.update(cx, |tree_view, cx| {
                    tree_view.active_connection(id.to_string(), cx);
                });
            }
        })
        .detach();

        Self {
            connections: connections.clone(),
            tab_container,
            db_tree_view,
            status_msg,
            is_connected,
            _event_handler: event_handler,
            workspace,
            focus_handle: cx.focus_handle(),
            sidebar,
            _subscriptions: subscriptions,
            tree_panel_size: TREE_PANEL_DEFAULT_SIZE,
            sidebar_panel_size: SIDEBAR_DEFAULT_WIDTH,
            resizing: None,
            bounds: Bounds::default(),
        }
    }

    pub fn new(
        workspace: Option<Workspace>,
        connection: StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let active_conn_id = connection.id;
        Self::new_with_active_conn(workspace, vec![connection], active_conn_id, window, cx)
    }

    pub fn ask_ai(&mut self, message: String, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.ask_ai(message, cx);
        });
        cx.notify();
    }

    /// 注册 SQL 代码块操作
    fn register_sql_code_block_actions(
        sidebar: &Entity<DatabaseSidebar>,
        tab_container: Entity<TabContainer>,
        connections: &[StoredConnection],
        cx: &mut App,
    ) {
        // 获取第一个连接的信息用于创建新编辑器
        let first_conn = connections.first().cloned();

        // 操作1：插入到当前编辑器
        let tab_container_for_insert = tab_container.clone();
        if let Some(insert_action) = CodeBlockAction::new("sql-insert-to-editor")
            .icon(IconName::Edit)
            .label("插入编辑器")
            .matcher(LanguageMatcher::sql())
            .on_click(move |code, _lang, window, cx| {
                // 获取当前激活的 tab
                if let Some(active_tab) = tab_container_for_insert.read(cx).active_tab() {
                    // 检查是否是 SQL 编辑器
                    if active_tab.content().content_key(cx) == "SqlEditor" {
                        if let Ok(sql_editor) = active_tab.content().view().downcast::<SqlEditorTab>() {
                            sql_editor.update(cx, |editor, cx| {
                                editor.set_sql(code, window, cx);
                            });
                        }
                    }
                }
            })
            .build()
        {
            sidebar.update(cx, |s, cx| {
                s.register_code_block_action(insert_action, cx);
            });
        }

        // 操作2：打开新编辑器
        let tab_container_for_new = tab_container.clone();
        if let Some(new_editor_action) = CodeBlockAction::new("sql-open-new-editor")
            .icon(IconName::Query)
            .label("打开新编辑器")
            .matcher(LanguageMatcher::sql())
            .on_click(move |code, _lang, window, cx| {
                let Some(conn) = first_conn.as_ref() else {
                    return;
                };
                let Ok(db_config) = conn.to_db_connection() else {
                    return;
                };

                let connection_id = conn.id.map(|id| id.to_string()).unwrap_or_default();
                let database_type = db_config.database_type;
                let tab_id = format!("query-ai-{}", Uuid::new_v4());
                let tab_id_clone = tab_id.clone();
                let conn_id_clone = connection_id.clone();
                let code_clone = code.clone();

                tab_container_for_new.update(cx, |container, cx| {
                    container.activate_or_add_tab_lazy(
                        tab_id.clone(),
                        move |window, cx| {
                            let sql_editor = cx.new(|cx| {
                                let editor = SqlEditorTab::new_with_config(
                                    "AI Query",
                                    connection_id.clone(),
                                    database_type,
                                    None,
                                    None,
                                    None,
                                    window,
                                    cx,
                                );
                                editor.set_sql(code_clone.clone(), window, cx);
                                editor
                            });
                            TabItem::new(tab_id_clone.clone(), conn_id_clone.clone(), sql_editor)
                        },
                        window,
                        cx,
                    );
                });
            })
            .build()
        {
            sidebar.update(cx, |s, cx| {
                s.register_code_block_action(new_editor_action, cx);
            });
        }
    }

    fn render_tree_resize_handle(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();

        resize_handle::<ResizePanel, ResizePanel>("tree-resize-handle", Axis::Horizontal)
            .placement(HandlePlacement::Left)
            .on_drag(ResizePanel, move |info, _, _, cx| {
                cx.stop_propagation();
                view.update(cx, |view, cx| {
                    view.resizing = Some(ResizingPanel::TreePanel);
                    cx.notify();
                });
                cx.new(|_| info.deref().clone())
            })
    }

    fn render_sidebar_resize_handle(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();

        resize_handle::<ResizePanel, ResizePanel>("sidebar-resize-handle", Axis::Horizontal)
            .placement(HandlePlacement::Right)
            .on_drag(ResizePanel, move |info, _, _, cx| {
                cx.stop_propagation();
                view.update(cx, |view, cx| {
                    view.resizing = Some(ResizingPanel::Sidebar);
                    cx.notify();
                });
                cx.new(|_| info.deref().clone())
            })
    }

    fn resize(&mut self, mouse_position: Point<Pixels>, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(resizing) = self.resizing else {
            return;
        };

        let available_width = self.bounds.size.width;

        match resizing {
            ResizingPanel::TreePanel => {
                let new_size = mouse_position.x - self.bounds.left();
                let sidebar_visible = self.sidebar.read(cx).is_panel_visible();
                let sidebar_width = if sidebar_visible { self.sidebar_panel_size } else { TOOLBAR_WIDTH };
                let max_size = (available_width - PANEL_MIN_SIZE - sidebar_width).max(PANEL_MIN_SIZE);
                self.tree_panel_size = new_size.clamp(PANEL_MIN_SIZE, max_size);
            }
            ResizingPanel::Sidebar => {
                let new_size = self.bounds.right() - mouse_position.x;
                let max_size = (available_width - self.tree_panel_size - PANEL_MIN_SIZE).max(SIDEBAR_MIN_WIDTH);
                self.sidebar_panel_size = new_size.clamp(SIDEBAR_MIN_WIDTH, max_size.min(SIDEBAR_MAX_WIDTH));
            }
        }

        cx.notify();
    }

    fn done_resizing(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.resizing = None;
        cx.notify();
    }

    fn render_connection_status(&self, cx: &App) -> AnyElement {
        let status_text = self.status_msg.read(cx).clone();
        let is_error = status_text.contains("Failed") || status_text.contains("failed");

        let first_conn = self.connections.first();
        let conn_name = first_conn
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let (conn_host, conn_port, conn_username, conn_database) = first_conn
            .and_then(|c| c.to_db_connection().ok())
            .map(|p| (p.host, p.port, p.username, p.database))
            .unwrap_or_default();

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_6()
            .child(
                div()
                    .w(px(64.0))
                    .h(px(64.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(48.0))
                            .h(px(48.0))
                            .rounded(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(!is_error, |this| {
                                this.border_4()
                                    .border_color(cx.theme().accent)
                                    .text_2xl()
                                    .text_color(cx.theme().accent)
                                    .child("⟳")
                            })
                            .when(is_error, |this| {
                                this.bg(Hsla::red())
                                    .text_color(gpui::white())
                                    .text_2xl()
                                    .child("✕")
                            }),
                    ),
            )
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(format!("Database Connection: {}", conn_name)),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_4()
                    .bg(cx.theme().muted)
                    .rounded(px(8.0))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Host:"))
                            .child(conn_host),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Port:"))
                            .child(format!("{}", conn_port)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Username:"))
                            .child(conn_username),
                    )
                    .when_some(conn_database, |this, db| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .child(div().font_weight(FontWeight::SEMIBOLD).child("Database:"))
                                .child(db),
                        )
                    }),
            )
            .child(
                div()
                    .text_lg()
                    .when(!is_error, |this| this.text_color(cx.theme().accent))
                    .when(is_error, |this| this.text_color(Hsla::red()))
                    .child(status_text),
            )
            .into_any_element()
    }
}

impl Focusable for DatabaseTabView {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.tab_container.focus_handle(cx)
    }
}

impl EventEmitter<TabContentEvent> for DatabaseTabView {}

impl TabContent for DatabaseTabView {
    fn content_key(&self) -> &'static str {
        "Database"
    }

    fn title(&self, _cx: &App) -> SharedString {
        if let Some(workspace) = &self.workspace {
            workspace.name.clone().into()
        } else {
            self.connections
                .first()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Database".to_string())
                .into()
        }
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        if self.workspace.is_some() {
            Some(IconName::AppsColor.color().with_size(Size::Medium))
        } else {
            let db_connection = self.connections.first().map(|c| c.to_db_connection());
            match db_connection {
                None => Some(IconName::Database.color()),
                Some(result) => match result {
                    Ok(conn) => Some(conn.database_type.as_node_icon().with_size(Size::Medium)),
                    Err(_) => Some(IconName::Database.color().with_size(Size::Medium)),
                },
            }
        }
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
    }

    fn on_activate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_active(true, cx);
        });
    }

    fn on_deactivate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_active(false, cx);
        });
    }

    fn try_close(
        &mut self,
        _tab_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        let tab_container = self.tab_container.clone();
        let connections = self.connections.clone();

        let tabs_info: Vec<_> = tab_container
            .read(cx)
            .tabs()
            .iter()
            .map(|t| (t.id().to_string(), t.content().clone()))
            .collect();

        let tasks: Vec<_> = tabs_info
            .iter()
            .map(|(id, content)| content.try_close(id, window, cx))
            .collect();

        cx.spawn(async move |_handle, cx: &mut AsyncApp| {
            for task in tasks {
                if !task.await {
                    return false;
                }
            }

            let _ = cx.update(|cx| {
                let global_state = cx.global_mut::<ActiveConnections>();
                for conn in &connections {
                    if let Some(id) = conn.id {
                        global_state.remove(id);
                    }
                }
            });

            let global_state = cx.update(|cx| cx.global::<GlobalDbState>().clone());
            if let Ok(global_state) = global_state {
                let connection_ids: Vec<String> = connections
                    .iter()
                    .filter_map(|conn| conn.id.map(|id| id.to_string()))
                    .collect();

                for connection_id in connection_ids {
                    if let Err(e) = global_state.disconnect_all(cx, connection_id.clone()).await {
                        tracing::warn!("Failed to disconnect connection {}: {}", connection_id, e);
                    }
                }
            }

            true
        })
    }
}

impl Render for DatabaseTabView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connected_flag = *self.is_connected.read(cx);
        let view = cx.entity().clone();
        let sidebar_visible = self.sidebar.read(cx).is_panel_visible();
        let sidebar_panel_size = self.sidebar_panel_size;

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .when(!is_connected_flag, |el: gpui::Div| {
                el.child(self.render_connection_status(cx))
            })
            .when(is_connected_flag, |el: gpui::Div| {
                let border_color = cx.theme().border;
                let tree_panel_size = self.tree_panel_size;

                el.child(
                    h_flex()
                        .size_full()
                        .child(
                            div()
                                .relative()
                                .h_full()
                                .w(tree_panel_size)
                                .flex_shrink_0()
                                .border_r_1()
                                .border_color(border_color)
                                .child(self.db_tree_view.clone())
                                .child(self.render_tree_resize_handle(window, cx))
                        )
                        .child(
                            div()
                                .flex_1()
                                .h_full()
                                .min_w_0()
                                .child(self.tab_container.clone())
                        )
                        .when(sidebar_visible, |this| {
                            this.child(
                                div()
                                    .relative()
                                    .h_full()
                                    .w(sidebar_panel_size)
                                    .flex_shrink_0()
                                    .child(self.render_sidebar_resize_handle(window, cx))
                                    .child(self.sidebar.clone())
                            )
                        })
                        .when(!sidebar_visible, |this| {
                            this.child(self.sidebar.clone())
                        })
                        .child(ResizeEventHandler { view })
                )
            })
    }
}

struct ResizeEventHandler {
    view: Entity<DatabaseTabView>,
}

impl IntoElement for ResizeEventHandler {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ResizeEventHandler {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let bounds = window.bounds();
        self.view.update(cx, |view, _| {
            view.bounds = Bounds {
                origin: Point::default(),
                size: bounds.size,
            };
        });
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.on_mouse_event({
            let view = self.view.clone();
            let resizing = view.read(cx).resizing;
            move |e: &MouseMoveEvent, phase, window, cx| {
                if resizing.is_none() {
                    return;
                }
                if !phase.bubble() {
                    return;
                }
                view.update(cx, |view, cx| view.resize(e.position, window, cx));
            }
        });

        window.on_mouse_event({
            let view = self.view.clone();
            move |_: &MouseUpEvent, phase, window, cx| {
                if phase.bubble() {
                    view.update(cx, |view, cx| view.done_resizing(window, cx));
                }
            }
        });
    }
}
