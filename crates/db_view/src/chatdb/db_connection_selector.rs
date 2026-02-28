//! 数据库连接/数据库/架构 选择器（懒加载）

use db::GlobalDbState;
use gpui::prelude::FluentBuilder;
use gpui::{
    App, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    div, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size,
    button::{Button, ButtonVariants},
    h_flex,
    popover::Popover,
    spinner::Spinner,
    v_flex,
};
use one_core::storage::traits::Repository;
use one_core::storage::{ConnectionRepository, ConnectionType, DatabaseType, GlobalStorageState};
use rust_i18n::t;

// ========================================================================
// 数据类型
// ========================================================================

#[derive(Clone, Debug)]
pub struct ConnectionItem {
    pub id: String,
    pub name: String,
    pub database_type: DatabaseType,
}

impl ConnectionItem {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        database_type: DatabaseType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            database_type,
        }
    }
}

// ========================================================================
// 事件定义
// ========================================================================

#[derive(Clone, Debug)]
pub enum DbConnectionSelectorEvent {
    SelectionChanged {
        connection: Option<ConnectionItem>,
        database: Option<String>,
        schema: Option<String>,
        supports_schema: bool,
        uses_schema_as_database: bool,
    },
}

// ========================================================================
// 组件定义
// ========================================================================

pub struct DbConnectionSelector {
    focus_handle: FocusHandle,

    storage_manager: one_core::storage::StorageManager,

    connections: Vec<ConnectionItem>,
    databases: Vec<String>,
    schemas: Vec<String>,

    selected_connection: Option<ConnectionItem>,
    selected_database: Option<String>,
    selected_schema: Option<String>,

    supports_schema: bool,
    uses_schema_as_database: bool,
    popover_open: bool,

    connections_loaded: bool,
    loading_connections: bool,
    loading_databases: bool,
    loading_schemas: bool,
}

#[derive(Clone)]
struct DbConnectionSelectorSnapshot {
    connections: Vec<ConnectionItem>,
    databases: Vec<String>,
    schemas: Vec<String>,
    selected_connection: Option<ConnectionItem>,
    selected_database: Option<String>,
    selected_schema: Option<String>,
    supports_schema: bool,
    uses_schema_as_database: bool,
    loading_connections: bool,
    loading_databases: bool,
    loading_schemas: bool,
}

impl DbConnectionSelector {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let storage_manager = cx.global::<GlobalStorageState>().storage.clone();

        Self {
            focus_handle,
            storage_manager,
            connections: Vec::new(),
            databases: Vec::new(),
            schemas: Vec::new(),
            selected_connection: None,
            selected_database: None,
            selected_schema: None,
            supports_schema: false,
            uses_schema_as_database: false,
            popover_open: false,
            connections_loaded: false,
            loading_connections: false,
            loading_databases: false,
            loading_schemas: false,
        }
    }

    pub fn get_connection_info(&self) -> Option<(String, Option<String>, Option<String>)> {
        let connection = self.selected_connection.as_ref()?;
        if self.uses_schema_as_database {
            return Some((connection.id.clone(), None, self.selected_schema.clone()));
        }
        Some((
            connection.id.clone(),
            self.selected_database.clone(),
            self.selected_schema.clone(),
        ))
    }

    pub fn supports_schema(&self) -> bool {
        self.supports_schema
    }

    pub fn uses_schema_as_database(&self) -> bool {
        self.uses_schema_as_database
    }

    fn snapshot(&self) -> DbConnectionSelectorSnapshot {
        DbConnectionSelectorSnapshot {
            connections: self.connections.clone(),
            databases: self.databases.clone(),
            schemas: self.schemas.clone(),
            selected_connection: self.selected_connection.clone(),
            selected_database: self.selected_database.clone(),
            selected_schema: self.selected_schema.clone(),
            supports_schema: self.supports_schema,
            uses_schema_as_database: self.uses_schema_as_database,
            loading_connections: self.loading_connections,
            loading_databases: self.loading_databases,
            loading_schemas: self.loading_schemas,
        }
    }

    fn emit_selection(&self, cx: &mut Context<Self>) {
        cx.emit(DbConnectionSelectorEvent::SelectionChanged {
            connection: self.selected_connection.clone(),
            database: self.selected_database.clone(),
            schema: self.selected_schema.clone(),
            supports_schema: self.supports_schema,
            uses_schema_as_database: self.uses_schema_as_database,
        });
    }

    fn close_popover(&mut self, cx: &mut Context<Self>) {
        if self.popover_open {
            self.popover_open = false;
            cx.notify();
        }
    }

    fn ensure_connections_loaded(&mut self, cx: &mut Context<Self>) {
        if self.connections_loaded || self.loading_connections {
            return;
        }

        self.loading_connections = true;

        let repo = match self.storage_manager.get::<ConnectionRepository>() {
            Some(repo) => repo,
            None => {
                self.loading_connections = false;
                return;
            }
        };

        let all_connections = match repo.list() {
            Ok(connections) => connections,
            Err(_) => {
                self.loading_connections = false;
                return;
            }
        };

        self.connections = all_connections
            .into_iter()
            .filter(|c| c.connection_type == ConnectionType::Database)
            .filter_map(|c| {
                let config = c.to_db_connection().ok()?;
                Some(ConnectionItem::new(
                    c.id?.to_string(),
                    c.name,
                    config.database_type,
                ))
            })
            .collect();

        self.connections_loaded = true;
        self.loading_connections = false;
        cx.notify();
    }

    fn handle_connection_selected(&mut self, connection: ConnectionItem, cx: &mut Context<Self>) {
        // 如果选择的是同一个连接，不做任何处理
        if self.selected_connection.as_ref().map(|c| &c.id) == Some(&connection.id) {
            return;
        }

        self.selected_connection = Some(connection.clone());
        self.selected_database = None;
        self.selected_schema = None;
        self.databases.clear();
        self.schemas.clear();

        // 重置加载状态，确保新连接能正常加载
        self.loading_databases = false;
        self.loading_schemas = false;

        let global_db_state = cx.global::<GlobalDbState>().clone();
        self.supports_schema = global_db_state.supports_schema(&connection.database_type);
        self.uses_schema_as_database =
            global_db_state.uses_schema_as_database(&connection.database_type);

        self.register_connection(connection.id.clone(), cx);
        self.emit_selection(cx);
        self.load_databases(connection.id.clone(), cx);
        cx.notify();
    }

    fn register_connection(&mut self, connection_id: String, cx: &mut Context<Self>) {
        let repo = match self.storage_manager.get::<ConnectionRepository>() {
            Some(repo) => repo,
            None => return,
        };
        let connection_id_i64: i64 = match connection_id.parse() {
            Ok(id) => id,
            Err(_) => return,
        };
        let stored_conn = match repo.get(connection_id_i64) {
            Ok(Some(conn)) => conn,
            _ => return,
        };
        if let Ok(db_config) = stored_conn.to_db_connection() {
            let mut global_db_state = cx.global::<GlobalDbState>().clone();
            global_db_state.register_connection(db_config);
        }
    }

    fn load_databases(&mut self, connection_id: String, cx: &mut Context<Self>) {
        if self.loading_databases {
            return;
        }
        self.loading_databases = true;
        cx.notify();

        let global_state = cx.global::<GlobalDbState>().clone();
        let uses_schema_as_database = self.uses_schema_as_database;
        let expected_connection_id = connection_id.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = if uses_schema_as_database {
                global_state
                    .list_schemas(cx, connection_id.clone(), String::new())
                    .await
            } else {
                global_state.list_databases(cx, connection_id.clone()).await
            };

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    entity.update(cx, |selector, cx| {
                        // 检查连接是否已切换，避免旧连接的结果覆盖新连接
                        let current_conn_id = selector.selected_connection.as_ref().map(|c| &c.id);
                        if current_conn_id != Some(&expected_connection_id) {
                            return;
                        }

                        selector.loading_databases = false;
                        match result {
                            Ok(list) => {
                                selector.databases = list;
                            }
                            Err(_) => {
                                selector.databases.clear();
                            }
                        }
                        cx.notify();
                    });
                });
            }
        })
        .detach();
    }

    fn handle_database_selected(&mut self, database: String, cx: &mut Context<Self>) {
        if self.uses_schema_as_database {
            self.selected_database = None;
            self.selected_schema = Some(database);
            self.emit_selection(cx);
            self.close_popover(cx);
            cx.notify();
            return;
        }

        self.selected_database = Some(database.clone());
        self.selected_schema = None;
        self.schemas.clear();
        self.emit_selection(cx);

        if self.supports_schema {
            self.load_schemas(database, cx);
        } else {
            self.close_popover(cx);
        }
        cx.notify();
    }

    fn load_schemas(&mut self, database: String, cx: &mut Context<Self>) {
        if self.loading_schemas {
            return;
        }
        let Some(conn) = &self.selected_connection else {
            return;
        };

        self.loading_schemas = true;
        cx.notify();

        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = conn.id.clone();
        let expected_connection_id = connection_id.clone();
        let expected_database = database.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = global_state.list_schemas(cx, connection_id, database).await;

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    entity.update(cx, |selector, cx| {
                        // 检查连接和数据库是否已切换
                        let current_conn_id = selector.selected_connection.as_ref().map(|c| &c.id);
                        let current_database = selector.selected_database.as_ref();
                        if current_conn_id != Some(&expected_connection_id)
                            || current_database != Some(&expected_database)
                        {
                            return;
                        }

                        selector.loading_schemas = false;
                        match result {
                            Ok(list) => selector.schemas = list,
                            Err(_) => selector.schemas.clear(),
                        }
                        cx.notify();
                    });
                });
            }
        })
        .detach();
    }

    fn handle_schema_selected(&mut self, schema: String, cx: &mut Context<Self>) {
        self.selected_schema = Some(schema);
        self.emit_selection(cx);
        self.close_popover(cx);
        cx.notify();
    }

    fn selection_label(&self) -> String {
        let Some(connection) = &self.selected_connection else {
            return t!("ChatDbSelector.select_source").to_string();
        };

        let mut parts = vec![connection.name.clone()];
        if self.uses_schema_as_database {
            match &self.selected_schema {
                Some(schema) => parts.push(schema.clone()),
                None => parts.push(t!("ChatDbSelector.select_schema").to_string()),
            }
            return parts.join(" / ");
        }

        match &self.selected_database {
            Some(database) => parts.push(database.clone()),
            None => parts.push(t!("ChatDbSelector.select_database").to_string()),
        }

        if self.supports_schema {
            match &self.selected_schema {
                Some(schema) => parts.push(schema.clone()),
                None => parts.push(t!("ChatDbSelector.select_schema").to_string()),
            }
        }

        parts.join(" / ")
    }

    fn render_list_item(
        id: impl Into<gpui::ElementId>,
        label: String,
        selected: bool,
        colors: SelectorColors,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let corner_radius = px(6.0);

        h_flex()
            .id(id)
            .w_full()
            .relative()
            .items_center()
            .justify_start()
            .px_3()
            .py_2()
            .text_sm()
            .text_color(colors.foreground)
            .cursor_pointer()
            .rounded(corner_radius)
            .when(!selected, |this| {
                this.hover(|style| style.bg(colors.list_hover))
            })
            .on_click(on_click)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(label),
            )
            .map(|this| {
                if selected {
                    this.bg(colors.list_active).child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .left_0()
                            .rounded(corner_radius)
                            .border_1()
                            .border_color(colors.list_active_border),
                    )
                } else {
                    this
                }
            })
    }

    fn render_column(
        column_id: &str,
        title: String,
        items: Vec<AnyColumnItem>,
        is_loading: bool,
        width: f32,
        colors: SelectorColors,
    ) -> impl IntoElement {
        v_flex()
            .w(px(width))
            .min_w(px(width))
            .h(px(260.0))
            .overflow_hidden()
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .text_color(colors.muted_foreground)
                    .child(title),
            )
            .child(
                div()
                    .id(SharedString::from(format!("db-selector-{}", column_id)))
                    .flex_1()
                    .overflow_x_hidden()
                    .overflow_y_scroll()
                    .when(is_loading, |this| {
                        this.child(
                            div()
                                .w_full()
                                .items_center()
                                .justify_center()
                                .py_4()
                                .child(Spinner::new().with_size(Size::Small)),
                        )
                    })
                    .when(!is_loading && items.is_empty(), |this| {
                        this.child(
                            div()
                                .w_full()
                                .py_4()
                                .text_sm()
                                .text_color(colors.muted_foreground)
                                .child(t!("ChatDbSelector.no_data")),
                        )
                    })
                    .children(items.into_iter().map(|item| item.element)),
            )
    }

    fn render_popover_content(
        snapshot: DbConnectionSelectorSnapshot,
        view: Entity<Self>,
        colors: SelectorColors,
    ) -> impl IntoElement {
        let DbConnectionSelectorSnapshot {
            connections,
            databases,
            schemas,
            selected_connection,
            selected_database,
            selected_schema,
            supports_schema,
            uses_schema_as_database,
            loading_connections,
            loading_databases,
            loading_schemas,
        } = snapshot;

        let connection_items = connections
            .into_iter()
            .map(|conn| {
                let selected = selected_connection
                    .as_ref()
                    .map(|c| c.id == conn.id)
                    .unwrap_or(false);
                let label = conn.name.clone();
                let view = view.clone();
                let conn_clone = conn.clone();
                AnyColumnItem::new(Self::render_list_item(
                    SharedString::from(format!("conn-item-{}", conn.id)),
                    label,
                    selected,
                    colors,
                    move |_, _window, cx| {
                        view.update(cx, |selector, cx| {
                            selector.handle_connection_selected(conn_clone.clone(), cx);
                        });
                    },
                ))
            })
            .collect::<Vec<_>>();

        let database_items = databases
            .into_iter()
            .map(|db| {
                let selected = if uses_schema_as_database {
                    selected_schema.as_ref().map(|s| s == &db).unwrap_or(false)
                } else {
                    selected_database
                        .as_ref()
                        .map(|s| s == &db)
                        .unwrap_or(false)
                };
                let view = view.clone();
                let db_clone = db.clone();
                AnyColumnItem::new(Self::render_list_item(
                    SharedString::from(format!("db-item-{}", db)),
                    db,
                    selected,
                    colors,
                    move |_, _window, cx| {
                        view.update(cx, |selector, cx| {
                            selector.handle_database_selected(db_clone.clone(), cx);
                        });
                    },
                ))
            })
            .collect::<Vec<_>>();

        let schema_items = schemas
            .into_iter()
            .map(|schema| {
                let selected = selected_schema
                    .as_ref()
                    .map(|s| s == &schema)
                    .unwrap_or(false);
                let view = view.clone();
                let schema_clone = schema.clone();
                AnyColumnItem::new(Self::render_list_item(
                    SharedString::from(format!("schema-item-{}", schema)),
                    schema,
                    selected,
                    colors,
                    move |_, _window, cx| {
                        view.update(cx, |selector, cx| {
                            selector.handle_schema_selected(schema_clone.clone(), cx);
                        });
                    },
                ))
            })
            .collect::<Vec<_>>();

        let database_title = if uses_schema_as_database {
            t!("ChatDbSelector.schema_title").to_string()
        } else {
            t!("ChatDbSelector.database_title").to_string()
        };

        h_flex()
            .gap_0()
            .child(Self::render_column(
                "connections",
                t!("ChatDbSelector.connection_title").to_string(),
                connection_items,
                loading_connections,
                200.0,
                colors,
            ))
            .child(Self::render_column(
                "databases",
                database_title,
                database_items,
                loading_databases,
                200.0,
                colors,
            ))
            .when(supports_schema && !uses_schema_as_database, |this| {
                this.child(Self::render_column(
                    "schemas",
                    t!("ChatDbSelector.schema_title").to_string(),
                    schema_items,
                    loading_schemas,
                    200.0,
                    colors,
                ))
            })
    }

    fn render_trigger(&self, view: Entity<Self>, _cx: &mut Context<Self>) -> impl IntoElement {
        let view_for_open = view.clone();
        let label = self.selection_label();

        Popover::new("db-connection-selector")
            .open(self.popover_open)
            .on_open_change(move |open, _window, cx| {
                view_for_open.update(cx, |selector, cx| {
                    selector.popover_open = *open;
                    if *open {
                        selector.ensure_connections_loaded(cx);
                    }
                    cx.notify();
                });
            })
            .trigger(
                Button::new("db-connection-selector-trigger")
                    .ghost()
                    .with_size(Size::Small)
                    .icon(IconName::Database.color())
                    .label(label),
            )
            .content({
                let view_for_content = view.clone();
                move |_state, _window, cx| {
                    let snapshot = view_for_content.read(cx).snapshot();
                    let colors = SelectorColors {
                        border: cx.theme().border,
                        foreground: cx.theme().foreground,
                        muted_foreground: cx.theme().muted_foreground,
                        list_active: cx.theme().list_active,
                        list_active_border: cx.theme().list_active_border,
                        list_hover: cx.theme().list_hover,
                    };
                    Self::render_popover_content(snapshot, view_for_content.clone(), colors)
                }
            })
            .max_w(px(640.0))
            .into_any_element()
    }
}

impl EventEmitter<DbConnectionSelectorEvent> for DbConnectionSelector {}

impl Focusable for DbConnectionSelector {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DbConnectionSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        self.render_trigger(view, cx)
    }
}

struct AnyColumnItem {
    element: gpui::AnyElement,
}

impl AnyColumnItem {
    fn new(element: impl IntoElement) -> Self {
        Self {
            element: element.into_any_element(),
        }
    }
}

#[derive(Clone, Copy)]
struct SelectorColors {
    border: Hsla,
    foreground: Hsla,
    muted_foreground: Hsla,
    list_active: Hsla,
    list_active_border: Hsla,
    list_hover: Hsla,
}
