use crate::sql_editor::SqlEditor;
use crate::sql_result_tab::SqlResultTabContainer;
use db::{DbManager, GlobalDbState, format_sql};
use gpui::prelude::*;
use gpui::{
    App, AppContext, AsyncApp, Axis, Bounds, ClickEvent, Context, Element, Entity, EventEmitter,
    FocusHandle, Focusable, IntoElement, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point, Render, SharedString, Styled, Task, WeakEntity, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{InputContextMenuItem, InputEvent};
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, Size, WindowExt, h_flex, v_flex,
};
use one_core::storage::DatabaseType;
use one_core::storage::manager::get_queries_dir;
use one_core::tab_container::{TabContainer, TabContent, TabContentEvent};
use one_core::utils::auto_save_config::AutoSaveConfig;
use one_ui::resize_handle::{HandlePlacement, ResizePanel, resize_handle};
use rust_i18n::t;
use smol::Timer;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tracing::log::error;

const PANEL_MIN_SIZE: Pixels = px(100.0);
const RESULT_PANEL_DEFAULT_SIZE: Pixels = px(400.0);

// Events emitted by SqlEditorTabContent
#[derive(Debug, Clone)]
pub enum SqlEditorEvent {
    /// Query was saved successfully
    QuerySaved {
        connection_id: String,
        database: Option<String>,
    },
}

pub struct SqlEditorTab {
    title: SharedString,
    editor: Entity<SqlEditor>,
    connection_id: String,
    database_type: DatabaseType,
    sql_result_tab_container: Entity<SqlResultTabContainer>,
    database_select: Entity<SelectState<SearchableVec<String>>>,
    schema_select: Entity<SelectState<SearchableVec<String>>>,
    supports_schema: bool,
    uses_schema_as_database: bool,
    focus_handle: FocusHandle,
    file_path: PathBuf,
    _save_task: Option<Task<()>>,
    result_panel_size: Pixels,
    resizing: bool,
    bounds: Bounds<Pixels>,
    /// 自动保存序列号，用于防抖
    auto_save_seq: Arc<AtomicU64>,
    /// 是否有未保存的修改
    is_dirty: Arc<AtomicBool>,
}

impl SqlEditorTab {
    pub fn new_with_config(
        title: impl Into<SharedString>,
        connection_id: impl Into<String>,
        database_type: DatabaseType,
        file_path: Option<PathBuf>,
        initial_database: Option<String>,
        initial_schema: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| SqlEditor::new(window, cx));
        let focus_handle = cx.focus_handle();
        let database_select =
            cx.new(|cx| SelectState::new(SearchableVec::new(vec![]), None, window, cx));
        let schema_select =
            cx.new(|cx| SelectState::new(SearchableVec::new(vec![]), None, window, cx));

        let global_state = cx.global::<GlobalDbState>().clone();
        let supports_schema = global_state.supports_schema(&database_type);
        let uses_schema_as_database = global_state.uses_schema_as_database(&database_type);
        let connection_id_str = connection_id.into();

        let should_load_file = file_path.is_some();
        let resolved_file_path = file_path.unwrap_or_else(|| {
            Self::generate_new_file_path(
                &database_type,
                &connection_id_str,
                initial_database.as_deref().unwrap_or("default"),
            )
        });

        let auto_save_seq = Arc::new(AtomicU64::new(0));
        let is_dirty = Arc::new(AtomicBool::new(false));

        let instance = Self {
            title: title.into(),
            editor: editor.clone(),
            connection_id: connection_id_str,
            database_type,
            sql_result_tab_container: cx.new(|cx| SqlResultTabContainer::new(window, cx)),
            database_select: database_select.clone(),
            schema_select: schema_select.clone(),
            supports_schema,
            uses_schema_as_database,
            focus_handle,
            file_path: resolved_file_path.clone(),
            _save_task: None,
            result_panel_size: RESULT_PANEL_DEFAULT_SIZE,
            resizing: false,
            bounds: Bounds::default(),
            auto_save_seq: auto_save_seq.clone(),
            is_dirty: is_dirty.clone(),
        };

        instance.configure_editor_context_menu(cx);
        instance.bind_select_event(cx);
        instance.bind_auto_save(auto_save_seq, is_dirty, window, cx);
        instance.load_databases_async(
            initial_database,
            initial_schema,
            resolved_file_path,
            should_load_file,
            cx,
            window,
        );

        instance
    }

    fn configure_editor_context_menu(&self, cx: &mut Context<Self>) {
        let view = cx.entity().clone();
        self.editor.update(cx, |editor, cx| {
            editor.set_mouse_context_menu_items(
                vec![
                    InputContextMenuItem::on_click(t!("Query.run_selected").to_string(), {
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.handle_run_selected_query(window, cx);
                            });
                        }
                    })
                    .icon(IconName::ArrowRight),
                ],
                cx,
            );
        });
    }

    pub fn new_with_file_path(
        file_path: PathBuf,
        title: impl Into<SharedString>,
        connection_id: impl Into<String>,
        database_type: DatabaseType,
        initial_database: Option<String>,
        initial_schema: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_with_config(
            title,
            connection_id,
            database_type,
            Some(file_path),
            initial_database,
            initial_schema,
            window,
            cx,
        )
    }

    fn generate_new_file_path(
        database_type: &DatabaseType,
        connection_id: &str,
        database: &str,
    ) -> PathBuf {
        let queries_dir = get_queries_dir().unwrap_or_else(|_| PathBuf::from("."));
        let dir_path = queries_dir
            .join(database_type.as_str())
            .join(connection_id)
            .join(database);

        let mut next_number = 1;
        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                let prefix = t!("Query.query_editor_prefix");
                if name.starts_with(&*prefix) && name.ends_with(".sql") {
                    if let Some(num_str) = name
                        .strip_prefix(&*prefix)
                        .and_then(|s| s.strip_suffix(".sql"))
                    {
                        if let Ok(num) = num_str.parse::<u32>() {
                            next_number = next_number.max(num + 1);
                        }
                    }
                }
            }
        }

        let file_name = format!("{} {}.sql", t!("Query.query_editor_prefix"), next_number);
        dir_path.join(file_name)
    }

    pub fn get_file_path(&self) -> &PathBuf {
        &self.file_path
    }

    fn bind_select_event(&self, cx: &mut Context<Self>) {
        let this = self.clone();
        cx.subscribe(&self.database_select, move |_this, _select, event, cx| {
            let global_state = cx.global::<GlobalDbState>().clone();
            if let SelectEvent::Confirm(Some(db_name)) = event {
                let db = db_name.clone();
                let instance = this.clone();
                cx.spawn(async move |_handle, cx| {
                    // Load schemas if supported
                    if instance.supports_schema {
                        instance
                            .load_schemas_for_db(global_state.clone(), &db, None, cx)
                            .await;
                    }
                    instance.update_schema_for_db(global_state, &db, cx).await;
                })
                .detach();
            }
        })
        .detach();

        // Bind schema select event
        let this_for_schema = self.clone();
        cx.subscribe(&self.schema_select, move |_this, _select, event, cx| {
            let global_state = cx.global::<GlobalDbState>().clone();
            if let SelectEvent::Confirm(Some(_schema_name)) = event {
                let instance = this_for_schema.clone();
                // Get current database
                let database = instance.database_select.read(cx).selected_value().cloned();
                if let Some(db) = database {
                    cx.spawn(async move |_handle, cx| {
                        instance.update_schema_for_db(global_state, &db, cx).await;
                    })
                    .detach();
                }
            }
        })
        .detach();
    }

    /// 绑定自动保存功能
    /// 监听编辑器内容变化，当内容变化时启动防抖计时器进行自动保存
    fn bind_auto_save(
        &self,
        auto_save_seq: Arc<AtomicU64>,
        is_dirty: Arc<AtomicBool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor_input = self.editor.read(cx).input();
        let file_path = self.file_path.clone();
        let editor_entity = self.editor.clone();

        cx.subscribe_in(
            &editor_input,
            window,
            move |_this, _input, event: &InputEvent, _window, cx| {
                if let InputEvent::Change = event {
                    // 标记为已修改
                    is_dirty.store(true, Ordering::Relaxed);

                    // 检查自动保存是否启用
                    let auto_save_config = cx.try_global::<AutoSaveConfig>();
                    let (enabled, interval_ms) = match auto_save_config {
                        Some(config) => (config.is_enabled(), config.interval_ms()),
                        None => (true, 5000), // 默认值：启用，5秒间隔
                    };

                    if !enabled {
                        return;
                    }

                    // 增加序列号以取消之前的保存任务
                    let my_seq = auto_save_seq.fetch_add(1, Ordering::SeqCst) + 1;
                    let seq_clone = auto_save_seq.clone();
                    let dirty_clone = is_dirty.clone();
                    let file_path_clone = file_path.clone();
                    let editor_clone = editor_entity.clone();

                    // 启动防抖定时保存
                    cx.spawn(async move |_handle, cx| {
                        // 等待指定间隔
                        Timer::after(Duration::from_millis(interval_ms)).await;

                        // 检查是否被更新的请求取代
                        if seq_clone.load(Ordering::SeqCst) != my_seq {
                            return;
                        }

                        // 检查是否有未保存的修改
                        if !dirty_clone.load(Ordering::Relaxed) {
                            return;
                        }

                        // 执行保存
                        let _ = cx.update(|cx| {
                            let sql = editor_clone.read(cx).get_text(cx);
                            if sql.trim().is_empty() {
                                return;
                            }

                            // 创建目录
                            if let Some(parent) = file_path_clone.parent() {
                                if let Err(e) = std::fs::create_dir_all(parent) {
                                    error!(
                                        "{}",
                                        t!(
                                            "SqlEditorView.create_dir_failed",
                                            path = format!("{:?}", parent),
                                            error = e
                                        )
                                        .to_string()
                                    );
                                    return;
                                }
                            }

                            // 写入文件
                            if let Err(e) = std::fs::write(&file_path_clone, &sql) {
                                error!(
                                    "{}",
                                    t!(
                                        "SqlEditorView.auto_save_failed",
                                        path = format!("{:?}", file_path_clone),
                                        error = e
                                    )
                                    .to_string()
                                );
                            } else {
                                // 保存成功，清除脏标记
                                dirty_clone.store(false, Ordering::Relaxed);
                            }
                        });
                    })
                    .detach();
                }
            },
        )
        .detach();
    }

    /// Load schemas for a database
    async fn load_schemas_for_db(
        &self,
        global_state: GlobalDbState,
        database: &str,
        initial_schema: Option<String>,
        cx: &mut AsyncApp,
    ) {
        let connection_id = self.connection_id.clone();
        let schema_select = self.schema_select.clone();
        let db = database.to_string();

        let schemas = match global_state
            .list_schemas(cx, connection_id.clone(), db.clone())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to load schemas for {}: {}", db, e);
                return;
            }
        };

        let _ = cx.update(|cx| {
            if let Some(window_id) = cx.active_window() {
                let _ = cx.update_window(window_id, |_entity, window, cx| {
                    schema_select.update(cx, |state, cx| {
                        if schemas.is_empty() {
                            let items = SearchableVec::new(vec![
                                t!("Common.no_available", item = &t!("Schema.schema")).to_string(),
                            ]);
                            state.set_items(items, window, cx);
                            state.set_selected_index(None, window, cx);
                        } else {
                            let items = SearchableVec::new(schemas.clone());
                            state.set_items(items, window, cx);

                            if let Some(schema_name) = initial_schema.as_ref() {
                                if let Some(index) = schemas.iter().position(|s| s == schema_name) {
                                    state.set_selected_index(
                                        Some(IndexPath::new(index)),
                                        window,
                                        cx,
                                    );
                                } else if !schemas.is_empty() {
                                    state.set_selected_index(Some(IndexPath::new(0)), window, cx);
                                }
                            } else if !schemas.is_empty() {
                                state.set_selected_index(Some(IndexPath::new(0)), window, cx);
                            }
                        }
                    });
                });
            }
        });
    }

    pub fn set_sql(&self, sql: String, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |e, cx| e.set_value(sql, window, cx));
    }

    /// Load databases into the select dropdown
    fn load_databases_async(
        &self,
        init_db: Option<String>,
        init_schema: Option<String>,
        file_path: PathBuf,
        should_load_file: bool,
        cx: &mut Context<Self>,
        window: &mut Window,
    ) {
        let _ = window;
        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = self.connection_id.clone();
        let database_select = self.database_select.clone();
        let editor = self.editor.clone();
        let initial_database = init_db.clone();
        let instance = self.clone();
        let uses_schema_as_database = self.uses_schema_as_database;

        cx.spawn(async move |_handle, cx: &mut AsyncApp| {
            let databases = if uses_schema_as_database {
                match global_state
                    .list_schemas(cx, connection_id.clone(), String::new())
                    .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        error!("Failed to load schemas for {}: {}", connection_id, e);
                        Self::notify_async(cx, format!("Failed to load schemas: {}", e));
                        return;
                    }
                }
            } else {
                match global_state.list_databases(cx, connection_id.clone()).await {
                    Ok(result) => result,
                    Err(e) => {
                        error!("Failed to load databases for {}: {}", connection_id, e);
                        Self::notify_async(cx, format!("Failed to load databases: {}", e));
                        return;
                    }
                }
            };

            let sql_content = if should_load_file && file_path.exists() {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => Some(content),
                    Err(e) => {
                        error!("Failed to read SQL file {:?}: {}", file_path, e);
                        None
                    }
                }
            } else {
                None
            };

            let resolved_database = initial_database.clone();
            let selected_name = resolved_database
                .clone()
                .or_else(|| databases.first().cloned());

            cx.update(|cx: &mut App| {
                if let Some(window_id) = cx.active_window() {
                    cx.update_window(window_id, |_entity, window, cx| {
                        database_select.update(cx, |state, cx| {
                            if databases.is_empty() {
                                let items = SearchableVec::new(vec![
                                    t!("Common.no_available", item = &t!("Database.database"))
                                        .to_string(),
                                ]);
                                state.set_items(items, window, cx);
                                state.set_selected_index(None, window, cx);
                            } else {
                                let items = SearchableVec::new(databases.clone());
                                state.set_items(items, window, cx);
                                if let Some(name) = selected_name.as_ref() {
                                    if let Some(index) = databases.iter().position(|d| d == name) {
                                        state.set_selected_index(
                                            Some(IndexPath::new(index)),
                                            window,
                                            cx,
                                        );
                                    }
                                }
                            }
                        });
                        if let Some(sql) = sql_content {
                            editor.update(cx, |e, cx| {
                                e.set_value(sql.clone(), window, cx);
                            });
                        }
                    })
                } else {
                    Err(anyhow::anyhow!("No active window"))
                }
            })
            .ok();

            if let Some(ref db) = resolved_database {
                if instance.supports_schema {
                    instance
                        .load_schemas_for_db(global_state.clone(), db, init_schema, cx)
                        .await;
                }
                instance.update_schema_for_db(global_state, db, cx).await;
            }
        })
        .detach();
    }

    /// Update SQL editor schema with tables and columns from current database
    pub async fn update_schema_for_db(
        &self,
        global_state: GlobalDbState,
        database: &str,
        cx: &mut AsyncApp,
    ) {
        use crate::sql_editor::SqlSchema;

        let connection_id = self.connection_id.clone();
        let editor = self.editor.clone();

        // For Oracle (uses_schema_as_database), the database parameter is actually the schema name
        let (db, selected_schema) = if self.uses_schema_as_database {
            (String::new(), Some(database.to_string()))
        } else if self.supports_schema {
            let schema = self
                .schema_select
                .read_with(cx, |state, _cx| state.selected_value().cloned());
            (database.to_string(), schema)
        } else {
            (database.to_string(), None)
        };

        let tables = match global_state
            .list_tables(
                cx,
                connection_id.clone(),
                db.clone(),
                selected_schema.clone(),
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to get tables: {}", e);
                return;
            }
        };

        // Get database-specific completion info
        let db_completion_info = match global_state.get_completion_info(cx, connection_id.clone()) {
            Ok(info) => info,
            Err(e) => {
                eprintln!("Failed to get completion info: {}", e);
                return;
            }
        };

        let mut schema = SqlSchema::default();

        // Add tables to schema
        let table_items: Vec<(String, String)> = tables
            .iter()
            .map(|t| {
                let description = if let Some(comment) = &t.comment {
                    format!("Table: {} - {}", t.name, comment)
                } else {
                    format!("Table: {}", t.name)
                };
                (t.name.clone(), description)
            })
            .collect();
        schema = schema.with_tables(table_items);

        // Load columns for each table
        for table in &tables {
            if let Ok(columns) = global_state
                .list_columns(
                    cx,
                    connection_id.clone(),
                    db.clone(),
                    selected_schema.clone(),
                    table.name.clone(),
                )
                .await
            {
                let column_items: Vec<(String, String, String)> = columns
                    .iter()
                    .map(|c| {
                        (
                            c.name.clone(),
                            c.data_type.clone(),
                            c.comment.as_ref().unwrap_or(&String::new()).clone(),
                        )
                    })
                    .collect();
                schema = schema.with_table_columns_typed(&table.name, column_items);
            }
        }

        // Update editor with schema and database-specific completion info
        _ = editor.update(cx, |e, cx| {
            e.set_db_completion_info(db_completion_info, schema, cx);
        });
    }

    fn get_sql_text(&self, cx: &App) -> String {
        self.editor.read(cx).get_text(cx)
    }

    fn execute_sql_text(&mut self, sql: String, window: &mut Window, cx: &mut Context<Self>) {
        let connection_id = self.connection_id.clone();
        let sql_result_tab_container = self.sql_result_tab_container.clone();

        let selected_value = self.database_select.read(cx).selected_value().cloned();

        // For non-Oracle databases, database selection is required
        if !self.uses_schema_as_database && selected_value.is_none() {
            window.push_notification(t!("Query.please_select_database").to_string(), cx);
            return;
        }

        // For Oracle (uses_schema_as_database), the database_select contains schema values
        let (current_database_value, current_schema_value) = if self.uses_schema_as_database {
            (None, selected_value)
        } else {
            let schema = if self.supports_schema {
                self.schema_select.read(cx).selected_value().cloned()
            } else {
                None
            };
            (selected_value, schema)
        };

        if sql.trim().is_empty() {
            window.push_notification(t!("Query.please_enter_query").to_string(), cx);
            return;
        }

        sql_result_tab_container.update(cx, |container, cx| {
            container.handle_run_query(
                sql,
                connection_id,
                current_database_value,
                current_schema_value,
                window,
                cx,
            );
        })
    }

    fn notify_async(cx: &mut AsyncApp, message: String) {
        let _ = cx.update(|cx| {
            if let Some(window_id) = cx.active_window() {
                let notification = message.clone();
                cx.update_window(window_id, move |_entity, window, cx| {
                    window.push_notification(notification.clone(), cx);
                })
            } else {
                Err(anyhow::anyhow!("No active window"))
            }
        });
    }

    fn handle_run_query(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let selected_text = self.editor.read(cx).get_selected_text(cx);
        let sql = if selected_text.trim().is_empty() {
            self.get_sql_text(cx)
        } else {
            selected_text
        };
        self.execute_sql_text(sql, window, cx);
    }

    fn handle_run_selected_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected_text = self.editor.read(cx).get_selected_text(cx);
        if selected_text.trim().is_empty() {
            window.push_notification(t!("Query.please_select_sql_to_run").to_string(), cx);
            return;
        }
        self.execute_sql_text(selected_text, window, cx);
    }

    fn handle_format_query(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.get_sql_text(cx);
        if text.trim().is_empty() {
            window.push_notification(t!("Query.no_sql_to_format").to_string(), cx);
            return;
        }
        let window_option = cx.active_window().clone();
        cx.spawn(async move |entity: WeakEntity<Self>, cx: &mut AsyncApp| {
            entity
                .update(cx, |this, cx| {
                    let formatted = format_sql(&text);
                    if let Some(window_id) = window_option {
                        cx.update_window(window_id, move |_entity, window, cx| {
                            this.editor
                                .update(cx, |s, cx| s.set_value(formatted, window, cx));
                        })
                        .ok();
                    }
                })
                .ok()
        })
        .detach();
    }

    pub fn save_query(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.save_to_file(cx);
    }

    fn save_to_file(&self, cx: &App) {
        let sql = self.get_sql_text(cx);
        let file_path = self.file_path.clone();

        if let Some(parent) = file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("Failed to create directory {:?}: {}", parent, e);
                return;
            }
        }

        if let Err(e) = std::fs::write(&file_path, sql) {
            error!("Failed to save SQL file {:?}: {}", file_path, e);
        }
    }

    pub fn save_and_close(
        &mut self,
        tab_container: Entity<TabContainer>,
        tab_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save_to_file(cx);
        tab_container.update(cx, |container, cx| {
            container.force_close_tab_by_id(&tab_id, cx);
        });
        cx.emit(SqlEditorEvent::QuerySaved {
            connection_id: self.connection_id.clone(),
            database: self.database_select.read(cx).selected_value().cloned(),
        });
    }

    fn handle_save_query(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let sql = self.get_sql_text(cx);
        if sql.trim().is_empty() {
            window.push_notification(t!("Query.query_content_empty").to_string(), cx);
            return;
        }

        self.save_to_file(cx);
        window.push_notification(t!("Query.query_saved").to_string(), cx);
        cx.emit(SqlEditorEvent::QuerySaved {
            connection_id: self.connection_id.clone(),
            database: self.database_select.read(cx).selected_value().cloned(),
        });
    }

    fn handle_show_results(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sql_result_tab_container.update(cx, |container, cx| {
            container.show(cx);
        });
    }

    fn render_resize_handle(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();

        resize_handle::<ResizePanel, ResizePanel>("result-resize-handle", Axis::Vertical)
            .placement(HandlePlacement::Left)
            .on_drag(ResizePanel, move |info, _, _, cx| {
                cx.stop_propagation();
                view.update(cx, |view, cx| {
                    view.resizing = true;
                    cx.notify();
                });
                cx.new(|_| info.deref().clone())
            })
    }

    fn resize(
        &mut self,
        mouse_position: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.resizing {
            return;
        }

        let available_height = self.bounds.size.height;
        let new_size = self.bounds.bottom() - mouse_position.y;
        let max_size = (available_height - PANEL_MIN_SIZE).max(PANEL_MIN_SIZE);
        self.result_panel_size = new_size.clamp(PANEL_MIN_SIZE, max_size);

        cx.notify();
    }

    fn done_resizing(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.resizing = false;
        cx.notify();
    }

    fn render_has_results(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let result_panel_size = self.result_panel_size;
        let border_color = cx.theme().border;

        v_flex()
            .size_full()
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sql_editor(cx)),
            )
            .child(
                div()
                    .relative()
                    .h(result_panel_size)
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(border_color)
                    .child(self.sql_result_tab_container.clone())
                    .child(self.render_resize_handle(window, cx)),
            )
    }

    fn handle_explain_sql(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let selected_text = self.editor.read(cx).get_selected_text(cx);
        let sql = if selected_text.trim().is_empty() {
            self.get_sql_text(cx)
        } else {
            selected_text
        };

        if sql.trim().is_empty() {
            window.push_notification(t!("Query.please_enter_query").to_string(), cx);
            return;
        }

        let selected_value = self.database_select.read(cx).selected_value().cloned();

        // For non-Oracle databases, database selection is required
        if !self.uses_schema_as_database && selected_value.is_none() {
            window.push_notification(t!("Query.please_select_database").to_string(), cx);
            return;
        }

        // For Oracle (uses_schema_as_database), the database_select contains schema values
        let (current_database_value, current_schema_value) = if self.uses_schema_as_database {
            (None, selected_value)
        } else {
            let schema = if self.supports_schema {
                self.schema_select.read(cx).selected_value().cloned()
            } else {
                None
            };
            (selected_value, schema)
        };

        let Ok(plugin) = DbManager::default().get_plugin(&self.database_type) else {
            window.push_notification("未找到当前数据库插件".to_string(), cx);
            return;
        };

        let Some(explain_sql) = plugin.build_explain_sql(&sql) else {
            window.push_notification("EXPLAIN 仅支持查询语句".to_string(), cx);
            return;
        };

        let connection_id = self.connection_id.clone();
        let sql_result_tab_container = self.sql_result_tab_container.clone();

        sql_result_tab_container.update(cx, |container, cx| {
            container.handle_run_query(
                explain_sql,
                connection_id,
                current_database_value,
                current_schema_value,
                window,
                cx,
            );
        })
    }

    fn render_sql_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.clone();
        let database_select = self.database_select.clone();
        let schema_select = self.schema_select.clone();
        let supports_schema = self.supports_schema;
        let uses_schema_as_database = self.uses_schema_as_database;

        // Check if there are any results and if the panel is visible
        let has_results = self.sql_result_tab_container.read(cx).has_results(cx);
        let results_visible = self.sql_result_tab_container.read(cx).is_visible(cx);
        let is_query_executing = self.sql_result_tab_container.read(cx).is_executing(cx);

        // Check if there is selected text in the editor
        let has_selection = !self.editor.read(cx).get_selected_text(cx).trim().is_empty();

        v_flex()
            .size_full()
            .gap_2()
            .child(
                // Toolbar
                h_flex()
                    .gap_2()
                    .p_2()
                    .bg(cx.theme().muted)
                    .rounded_md()
                    .items_center()
                    .w_full()
                    .when(!uses_schema_as_database, |this| {
                        this.child(
                            // Database selector (for non-Oracle databases)
                            Select::new(&database_select)
                                .with_size(Size::Small)
                                .placeholder(t!("Query.select_database"))
                                .w(px(200.)),
                        )
                    })
                    .when(uses_schema_as_database, |this| {
                        this.child(
                            // Schema selector for Oracle (using database_select entity)
                            Select::new(&database_select)
                                .with_size(Size::Small)
                                .placeholder(t!("Query.select_schema"))
                                .w(px(200.)),
                        )
                    })
                    .when(supports_schema, |this| {
                        this.child(
                            // Schema selector for PostgreSQL
                            Select::new(&schema_select)
                                .with_size(Size::Small)
                                .placeholder(t!("Query.select_schema"))
                                .w(px(150.)),
                        )
                    })
                    .child(
                        Button::new("run-query")
                            .with_size(Size::Small)
                            .primary()
                            .loading(is_query_executing)
                            .label(if is_query_executing {
                                t!("Query.running")
                            } else if has_selection {
                                t!("Query.run_selected")
                            } else {
                                t!("Query.run")
                            })
                            .icon(IconName::ArrowRight)
                            .on_click(cx.listener(Self::handle_run_query)),
                    )
                    .child(
                        Button::new("explain-sql")
                            .with_size(Size::Small)
                            .ghost()
                            .disabled(is_query_executing)
                            .label(t!("Query.explain"))
                            .on_click(cx.listener(Self::handle_explain_sql)),
                    )
                    .child(
                        Button::new("format-query")
                            .with_size(Size::Small)
                            .ghost()
                            .label(t!("Query.format"))
                            .icon(IconName::Star)
                            .on_click(cx.listener(Self::handle_format_query)),
                    )
                    .child(
                        Button::new("save-query")
                            .with_size(Size::Small)
                            .ghost()
                            .label(t!("Query.save"))
                            .icon(IconName::Plus)
                            .on_click(cx.listener(Self::handle_save_query)),
                    ),
            )
            .child(
                // Editor
                v_flex().p_1().flex_1().child(editor.clone()).when(
                    has_results && !results_visible,
                    |this| {
                        this.child(
                            h_flex().w_full().justify_end().child(
                                Button::new("show-results")
                                    .with_size(Size::Small)
                                    .ghost()
                                    .tooltip(t!("Query.show_results"))
                                    .icon(IconName::ArrowUp)
                                    .on_click(cx.listener(Self::handle_show_results)),
                            ),
                        )
                    },
                ),
            )
    }
}

impl Render for SqlEditorTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_results = self.sql_result_tab_container.read(cx).has_results(cx);
        let results_visible = self.sql_result_tab_container.read(cx).is_visible(cx);
        let view = cx.entity().clone();

        let mut div = v_flex().size_full();
        if has_results && results_visible {
            div = div
                .child(self.render_has_results(window, cx))
                .child(ResizeEventHandler { view });
        } else {
            div = div.child(self.render_sql_editor(cx));
        }
        div
    }
}

// Make it Clone so we can use it in closures
impl Clone for SqlEditorTab {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            editor: self.editor.clone(),
            connection_id: self.connection_id.clone(),
            database_type: self.database_type,
            sql_result_tab_container: self.sql_result_tab_container.clone(),
            database_select: self.database_select.clone(),
            schema_select: self.schema_select.clone(),
            supports_schema: self.supports_schema,
            uses_schema_as_database: self.uses_schema_as_database,
            focus_handle: self.focus_handle.clone(),
            file_path: self.file_path.clone(),
            _save_task: None,
            result_panel_size: self.result_panel_size,
            resizing: false,
            bounds: self.bounds,
            auto_save_seq: self.auto_save_seq.clone(),
            is_dirty: self.is_dirty.clone(),
        }
    }
}

impl Focusable for SqlEditorTab {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<SqlEditorEvent> for SqlEditorTab {}

impl EventEmitter<TabContentEvent> for SqlEditorTab {}

impl TabContent for SqlEditorTab {
    fn content_key(&self) -> &'static str {
        "SqlEditor"
    }

    fn title(&self, _cx: &App) -> SharedString {
        self.title.clone()
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(IconName::Query.color())
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
    }

    fn try_close(
        &mut self,
        _tab_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        self.save_query(window, cx);
        Task::ready(true)
    }
}

struct ResizeEventHandler {
    view: Entity<SqlEditorTab>,
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
        (window.request_layout(gpui::Style::default(), None, cx), ())
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
                if !resizing {
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

#[cfg(test)]
mod tests {
    use db::DbManager;
    use one_core::storage::DatabaseType;

    fn build_explain_sql(database_type: DatabaseType, sql: &str) -> Option<String> {
        let plugin = DbManager::default()
            .get_plugin(&database_type)
            .expect("plugin should exist");
        plugin.build_explain_sql(sql)
    }

    #[test]
    fn test_build_explain_sql_mysql() {
        assert_eq!(
            build_explain_sql(DatabaseType::MySQL, " SELECT * FROM users "),
            Some("EXPLAIN SELECT * FROM users".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_sqlite() {
        assert_eq!(
            build_explain_sql(DatabaseType::SQLite, "select * from users"),
            Some("EXPLAIN QUERY PLAN select * from users".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_duckdb() {
        assert_eq!(
            build_explain_sql(DatabaseType::DuckDB, "select * from users"),
            Some("EXPLAIN select * from users".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_mssql() {
        assert_eq!(
            build_explain_sql(DatabaseType::MSSQL, "select * from users"),
            Some("SET SHOWPLAN_TEXT ON;\nselect * from users\nSET SHOWPLAN_TEXT OFF;".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_oracle() {
        assert_eq!(
            build_explain_sql(DatabaseType::Oracle, "select * from users"),
            Some(
                "EXPLAIN PLAN FOR select * from users;\nSELECT PLAN_TABLE_OUTPUT FROM TABLE(DBMS_XPLAN.DISPLAY())"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_build_explain_sql_mysql_multiple_statements() {
        assert_eq!(
            build_explain_sql(
                DatabaseType::MySQL,
                "select * from users; select * from posts;"
            ),
            Some("EXPLAIN select * from users;\nEXPLAIN select * from posts".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_mysql_preserves_semicolon_in_string() {
        assert_eq!(
            build_explain_sql(DatabaseType::MySQL, "select ';' as semi; select 2 as id;"),
            Some("EXPLAIN select ';' as semi;\nEXPLAIN select 2 as id".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_oracle_multiple_statements() {
        assert_eq!(
            build_explain_sql(DatabaseType::Oracle, "select * from users; select * from posts;"),
            Some(
                "EXPLAIN PLAN FOR select * from users;\nSELECT PLAN_TABLE_OUTPUT FROM TABLE(DBMS_XPLAN.DISPLAY());\nEXPLAIN PLAN FOR select * from posts;\nSELECT PLAN_TABLE_OUTPUT FROM TABLE(DBMS_XPLAN.DISPLAY())"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_build_explain_sql_skips_non_select_statements() {
        assert_eq!(
            build_explain_sql(
                DatabaseType::MySQL,
                "insert into users values (1); select * from users; update users set id = 2;"
            ),
            Some("EXPLAIN select * from users".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_returns_none_for_non_select_only() {
        assert_eq!(
            build_explain_sql(
                DatabaseType::MySQL,
                "insert into users values (1); update users set id = 2;"
            ),
            None
        );
    }

    #[test]
    fn test_build_explain_sql_supports_with_query_via_is_query_statement() {
        assert_eq!(
            build_explain_sql(
                DatabaseType::MySQL,
                "with active_users as (select * from users) select * from active_users"
            ),
            Some(
                "EXPLAIN with active_users as (select * from users) select * from active_users"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_build_explain_sql_keeps_existing_explain_statement() {
        assert_eq!(
            build_explain_sql(DatabaseType::MySQL, "EXPLAIN select * from users"),
            Some("EXPLAIN select * from users".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_keeps_existing_explain_and_wraps_remaining_queries() {
        assert_eq!(
            build_explain_sql(
                DatabaseType::MySQL,
                "EXPLAIN select * from users; select * from posts;"
            ),
            Some("EXPLAIN select * from users;\nEXPLAIN select * from posts".to_string())
        );
    }

    #[test]
    fn test_build_explain_sql_keeps_existing_mssql_showplan_script() {
        assert_eq!(
            build_explain_sql(
                DatabaseType::MSSQL,
                "SET SHOWPLAN_TEXT ON;\nselect * from users\nSET SHOWPLAN_TEXT OFF;"
            ),
            Some("SET SHOWPLAN_TEXT ON;\nselect * from users\nSET SHOWPLAN_TEXT OFF;".to_string())
        );
    }
}
