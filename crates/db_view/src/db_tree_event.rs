use crate::database_objects_tab::{DatabaseObjectsBatchAction, DatabaseObjectsEvent};
use crate::db_tree_view::SqlDumpMode;
use crate::{
    database_objects_tab::DatabaseObjectsPanel,
    database_view_plugin::DatabaseViewPluginRegistry,
    db_tree_view::{DbTreeView, DbTreeViewEvent},
    sql_editor_view::SqlEditorTab,
    table_designer_tab::{TableDesigner, TableDesignerConfig},
};
use db::{DbNode, DbNodeType, GlobalDbState, SqlResult};
use gpui::{
    App, AppContext, AsyncApp, Context, Entity, ParentElement, PathPromptOptions, Styled,
    Subscription, Window, div, px,
};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::{WindowExt, h_flex, notification::Notification, v_flex};
use one_core::{
    popup_window::{PopupWindowOptions, open_popup_window},
    tab_container::{TabContainer, TabItem},
};
use rust_i18n::t;
use std::collections::HashSet;
use tracing::log::{error, warn};
use uuid::Uuid;
use one_core::gpui_tokio::Tokio;

// Event handler for database tree view events
pub struct DatabaseEventHandler {
    _tree_subscription: Subscription,
    _objects_subscription: Subscription,
}

impl DatabaseEventHandler {
    /// 显示错误通知
    fn show_error(window: &mut Window, message: impl Into<String>, cx: &mut App) {
        window.push_notification(Notification::error(message.into()).autohide(true), cx);
    }

    /// 在异步上下文中显示错误通知
    fn show_error_async(cx: &mut App, message: impl Into<String>) {
        if let Some(window) = cx.active_window() {
            _ = window.update(cx, |_, window, cx| {
                window.push_notification(Notification::error(message.into()).autohide(true), cx);
            });
        }
    }

    /// 在异步上下文中显示成功通知
    fn show_success_async(cx: &mut App, message: impl Into<String>) {
        if let Some(window) = cx.active_window() {
            _ = window.update(cx, |_, window, cx| {
                window.push_notification(Notification::success(message.into()).autohide(true), cx);
            });
        }
    }
}

impl DatabaseEventHandler {
    pub(crate) fn new(
        db_tree_view: &Entity<DbTreeView>,
        tab_container: Entity<TabContainer>,
        objects_panel: Entity<DatabaseObjectsPanel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tab_container_clone = tab_container.clone();
        let objects_panel_clone = objects_panel.clone();
        let global_state = cx.global::<GlobalDbState>().clone();
        let tree_view_clone = db_tree_view.clone();

        let tree_subscription = cx.subscribe_in(
            db_tree_view,
            window,
            move |_handler, _tree, event, window, cx| {
                let global_state = global_state.clone();
                let tab_container = tab_container_clone.clone();
                let objects_panel = objects_panel_clone.clone();
                let tree_view = tree_view_clone.clone();

                let get_node = |node_id: &str, cx: &mut Context<Self>| -> Option<DbNode> {
                    let node = tree_view.read(cx).get_node(node_id).cloned();
                    if node.is_none() {
                        warn!("not found node {}", node_id);
                    }
                    node
                };

                match event {
                    DbTreeViewEvent::NodeSelected { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_node_selected(node, global_state, objects_panel, cx);
                        }
                    }
                    DbTreeViewEvent::CreateNewQuery { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_create_new_query(node, tab_container, window, cx);
                        }
                    }
                    DbTreeViewEvent::OpenTableData { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_open_table_data(node, tab_container, window, cx);
                        }
                    }
                    DbTreeViewEvent::OpenViewData { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_open_view_data(node, tab_container, window, cx);
                        }
                    }
                    DbTreeViewEvent::DesignTable { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_design_table(
                                node,
                                tab_container,
                                tree_view.clone(),
                                Some(objects_panel.clone()),
                                global_state.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::ImportData { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_import_data(node, window, cx);
                        }
                    }
                    DbTreeViewEvent::ExportData { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_export_data(node, global_state, window, cx);
                        }
                    }
                    DbTreeViewEvent::CloseConnection { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_close_connection(
                                node,
                                global_state,
                                tree_view.clone(),
                                tab_container.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::DeleteConnection { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_delete_connection(
                                node,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::CreateDatabase { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_create_database(
                                node,
                                global_state,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::EditDatabase { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_edit_database(
                                node,
                                global_state,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::CloseDatabase { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_close_database(
                                node,
                                global_state,
                                tree_view.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::DeleteDatabase { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_delete_database(
                                node,
                                global_state,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::CreateSchema { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_create_schema(
                                node,
                                global_state,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::DeleteSchema { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_delete_schema(
                                node,
                                global_state,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::DeleteTable { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_delete_table(
                                node,
                                global_state,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::RenameTable { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_rename_table(
                                node,
                                global_state,
                                tree_view.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::TruncateTable { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_truncate_table(
                                node,
                                global_state,
                                tree_view.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::DeleteView { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_delete_view(
                                node,
                                global_state,
                                tree_view.clone(),
                                None,
                                window,
                                cx,
                            );
                        }
                    }
                    DbTreeViewEvent::OpenNamedQuery { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_open_named_query(node, tab_container, window, cx);
                        }
                    }
                    DbTreeViewEvent::RenameQuery { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_rename_query(node, tree_view, global_state, window, cx);
                        }
                    }
                    DbTreeViewEvent::DeleteQuery { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_delete_query(node, tree_view.clone(), None, window, cx);
                        }
                    }
                    DbTreeViewEvent::RunSqlFile { node_id } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_run_sql_file(node, global_state, window, cx);
                        }
                    }
                    DbTreeViewEvent::DumpSqlFile { node_id, mode } => {
                        if let Some(node) = get_node(&node_id, cx) {
                            Self::handle_dump_sql_file(node, *mode, global_state, window, cx);
                        }
                    }
                }
            },
        );

        let tab_container_for_objects = tab_container.clone();
        let global_state_for_objects = cx.global::<GlobalDbState>().clone();
        let tree_view_for_objects = db_tree_view.clone();

        let database_objects = objects_panel.read(cx).database_objects().clone();
        let objects_subscription = cx.subscribe_in(
            &database_objects,
            window,
            move |_handler, _db_objects, event, window, cx| {
                let tab_container = tab_container_for_objects.clone();
                let global_state = global_state_for_objects.clone();
                let tree_view = tree_view_for_objects.clone();

                match event {
                    DatabaseObjectsEvent::Refresh { node } => {
                        Self::handle_node_selected(
                            node.clone(),
                            global_state,
                            objects_panel.clone(),
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::CreateDatabase { node } => {
                        Self::handle_create_database(
                            node.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::EditDatabase { node } => {
                        Self::handle_edit_database(
                            node.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::DeleteDatabase { node } => {
                        Self::handle_delete_database(
                            node.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::DeleteConnection { node } => {
                        Self::handle_delete_connection(
                            node.clone(),
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::CloseConnection { node } => {
                        Self::handle_close_connection(
                            node.clone(),
                            global_state,
                            tree_view,
                            tab_container.clone(),
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::OpenTableData { node } => {
                        Self::handle_open_table_data(node.clone(), tab_container, window, cx);
                    }
                    DatabaseObjectsEvent::DesignTable { node } => {
                        Self::handle_design_table(
                            node.clone(),
                            tab_container,
                            tree_view.clone(),
                            Some(objects_panel.clone()),
                            global_state.clone(),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::DeleteTable { node } => {
                        Self::handle_delete_table(
                            node.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::OpenViewData { node } => {
                        Self::handle_open_view_data(node.clone(), tab_container, window, cx);
                    }
                    DatabaseObjectsEvent::DeleteView { node } => {
                        Self::handle_delete_view(
                            node.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::CreateNewQuery { node } => {
                        Self::handle_create_new_query(node.clone(), tab_container, window, cx);
                    }
                    DatabaseObjectsEvent::OpenNamedQuery { node } => {
                        Self::handle_open_named_query(node.clone(), tab_container, window, cx);
                    }
                    DatabaseObjectsEvent::RenameQuery { node } => {
                        Self::handle_rename_query(
                            node.clone(),
                            tree_view,
                            global_state,
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::DeleteQuery { node } => {
                        Self::handle_delete_query(
                            node.clone(),
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::DeleteSchema { node } => {
                        Self::handle_delete_schema(
                            node.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::CreateSchema { node } => {
                        Self::handle_create_schema(
                            node.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                    DatabaseObjectsEvent::Batch { action, nodes } => {
                        Self::handle_batch_action(
                            action.clone(),
                            nodes.clone(),
                            global_state,
                            tree_view,
                            Some(objects_panel.clone()),
                            window,
                            cx,
                        );
                    }
                }
            },
        );

        Self {
            _tree_subscription: tree_subscription,
            _objects_subscription: objects_subscription,
        }
    }

    /// 处理节点选中事件
    fn handle_node_selected(
        node: DbNode,
        global_state: GlobalDbState,
        objects_panel: Entity<DatabaseObjectsPanel>,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let config = global_state.get_config(&connection_id);
        if let Some(config) = config {
            objects_panel.update(cx, |panel, cx| {
                panel.handle_node_selected(node, config, cx);
            });
        } else {
            error!("handle_node_selected error: config is None")
        }
    }

    /// 处理创建新查询事件
    fn handle_create_new_query(
        node: DbNode,
        tab_container: Entity<TabContainer>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let database = node.get_database_name();
        let schema = node.get_schema_name();
        let database_type = node.database_type;
        let title = format!(
            "{} - Query",
            database.as_deref().unwrap_or("New Query")
        );

        let tab_id = format!(
            "query-{}-{}",
            database.as_deref().unwrap_or("new"),
            Uuid::new_v4()
        );
        let tab_id_clone = tab_id.clone();
        let conn_id_clone = connection_id.clone();

        tab_container.update(cx, |container, cx| {
            container.activate_or_add_tab_lazy(
                tab_id.clone(),
                move |window, cx| {
                    let sql_editor = cx.new(|cx| {
                        SqlEditorTab::new_with_config(
                            title.clone(),
                            connection_id.clone(),
                            database_type,
                            None,
                            database.clone(),
                            schema.clone(),
                            window,
                            cx,
                        )
                    });
                    TabItem::new(tab_id_clone.clone(), conn_id_clone.clone(), sql_editor)
                },
                window,
                cx,
            );
        });
    }

    /// 处理打开表数据事件
    fn handle_open_table_data(
        node: DbNode,
        tab_container: Entity<TabContainer>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use crate::table_data_tab::TableDataTabContent;
        use tracing::info;

        let connection_id = node.connection_id.clone();
        let table = node.name.clone();

        info!(
            "handle_open_table_data: connection_id={}, table={}",
            connection_id, table
        );

        let Some(database) = node.get_database_name() else {
            Self::show_error(window, t!("Common.error_info").to_string(), cx);
            return;
        };
        let schema = node.get_schema_name();

        let tab_id = format!(
            "table-data-{}.{}.{}",
            database,
            schema.as_deref().unwrap_or(""),
            table
        );

        let database_type = node.database_type;
        tab_container.update(cx, |container, cx| {
            let tab_id_clone = tab_id.clone();
            let database_clone = database.clone();
            let schema_clone = schema.clone();
            let table_clone = table.clone();
            let config_id_clone = connection_id.clone();
            container.activate_or_add_tab_lazy(
                tab_id_clone.clone(),
                move |window, cx| {
                    let table_data = cx.new(|cx| {
                        TableDataTabContent::new(
                            database_clone.clone(),
                            schema_clone.clone(),
                            table_clone.clone(),
                            config_id_clone.clone(),
                            database_type,
                            true,
                            window,
                            cx,
                        )
                    });
                    TabItem::new(tab_id_clone.clone(), connection_id.clone(), table_data)
                },
                window,
                cx,
            );
        });
    }

    /// 处理打开视图数据事件
    fn handle_open_view_data(
        node: DbNode,
        tab_container: Entity<TabContainer>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use crate::table_data_tab::TableDataTabContent;

        let connection_id = node.connection_id.clone();
        let view = node.name.clone();

        let Some(database) = node.get_database_name() else {
            Self::show_error(window, t!("Common.error_info").to_string(), cx);
            return;
        };
        let schema = node.get_schema_name();

        let tab_id = format!(
            "view-data-{}.{}.{}",
            database,
            schema.as_deref().unwrap_or(""),
            view
        );

        let database_type = node.database_type;
        tab_container.update(cx, |container, cx| {
            let tab_id_clone = tab_id.clone();
            let database_clone = database.clone();
            let schema_clone = schema.clone();
            let view_clone = view.clone();
            let config_id_clone = connection_id.clone();
            container.activate_or_add_tab_lazy(
                tab_id_clone.clone(),
                move |window, cx| {
                    let view_data = cx.new(|cx| {
                        TableDataTabContent::new(
                            database_clone.clone(),
                            schema_clone.clone(),
                            view_clone.clone(),
                            config_id_clone.clone(),
                            database_type,
                            false,
                            window,
                            cx,
                        )
                    });
                    TabItem::new(tab_id_clone.clone(), connection_id.clone(), view_data)
                },
                window,
                cx,
            );
        });
    }

    /// 处理设计表事件（新建或编辑表结构）
    fn handle_design_table(
        node: DbNode,
        tab_container: Entity<TabContainer>,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        global_state: GlobalDbState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let database_type = node.database_type;
        let clone_conn_id = connection_id.clone();
        let (database_name, schema_name, table_name) = match node.node_type {
            DbNodeType::Schema => {
                let database = node.get_database_name().unwrap_or_default();
                let schema = node.get_schema_name();
                (database, schema, None)
            }
            DbNodeType::TablesFolder => {
                let database = node.get_database_name().unwrap_or_default();
                let schema = node.get_schema_name();
                (database, schema, None)
            }
            DbNodeType::Table => {
                let database = node.get_database_name().unwrap_or_default();
                let schema = node.get_schema_name();
                (database, schema, Some(node.name.clone()))
            }
            _ => return,
        };

        let tab_id = if let Some(ref table) = table_name {
            format!("table-designer-{}-{}", database_name, table)
        } else {
            format!("table-designer-{}-new-{}", database_name, Uuid::new_v4())
        };

        let tab_id_for_config = tab_id.clone();
        tab_container.update(cx, |container, cx| {
            container.activate_or_add_tab_lazy(
                tab_id.clone(),
                |window, cx| {
                    let mut config =
                        TableDesignerConfig::new(connection_id, database_name, database_type)
                            .with_tab_id(tab_id_for_config.clone());
                    if let Some(schema) = schema_name {
                        config = config.with_schema_name(schema);
                    }
                    let mut tab_title = t!("Table.new_table").to_string();
                    if let Some(table) = table_name {
                        tab_title = t!("DbTreeEvent.design_table_title", table = table).to_string();
                        config = config.with_table_name(table);
                    }
                    let designer = cx.new(|cx| TableDesigner::new(tab_title, config, window, cx));

                    let tree_view_clone = tree_view.clone();
                    let objects_panel_clone = objects_panel.clone();
                    let global_state_clone = global_state.clone();
                    cx.subscribe(
                        &designer,
                        move |_, _, event: &crate::table_designer_tab::TableDesignerEvent, cx| {
                            match event {
                                crate::table_designer_tab::TableDesignerEvent::Saved {
                                    connection_id,
                                    database_name,
                                    schema_name,
                                    ..
                                } => {
                                    let connection_id = connection_id.clone();
                                    let database_name = database_name.clone();
                                    let schema_name = schema_name.clone();
                                    let tree_view = tree_view_clone.clone();
                                    let objects_panel = objects_panel_clone.clone();
                                    let global_state = global_state_clone.clone();

                                    cx.defer(move |cx| {
                                        let node_id = if let Some(schema) = schema_name {
                                            format!(
                                                "{}:{}:{}",
                                                connection_id, database_name, schema
                                            )
                                        } else {
                                            format!("{}:{}", connection_id, database_name)
                                        };
                                        tree_view.update(cx, |tree, cx| {
                                            tree.refresh_tree(node_id, cx);
                                        });
                                        if let Some(panel) = objects_panel {
                                            let state_for_refresh = global_state.clone();
                                            panel.update(cx, |panel, cx| {
                                                panel.refresh(state_for_refresh, cx);
                                            });
                                        }
                                    });
                                }
                            }
                        },
                    )
                    .detach();

                    TabItem::new(tab_id, clone_conn_id.clone(), designer)
                },
                window,
                cx,
            );
        });
    }

    /// 处理导入数据事件
    fn handle_import_data(node: DbNode, window: &mut Window, cx: &mut App) {
        // 表节点：使用表导入视图（支持 TXT/CSV/JSON）
        use crate::import_export::table_import_view::TableImportView;
        let connection_id = node.connection_id.clone();
        let db = node.get_database_name().unwrap_or_default();
        let schema = node.get_schema_name();
        let table_name = node.name.clone();

        let import_view = TableImportView::new(
            connection_id.clone(),
            db.clone(),
            schema,
            Some(table_name.clone()),
            window,
            cx,
        );

        open_popup_window(
            PopupWindowOptions::new(t!("Table.import_data_to_table").to_string())
                .size(900.0, 600.0),
            move |_window, _cx| import_view.clone(),
            cx,
        );
    }

    /// 处理导出数据事件
    fn handle_export_data(
        node: DbNode,
        global_state: GlobalDbState,
        window: &mut Window,
        cx: &mut App,
    ) {
        use crate::import_export::table_export_view::DataExportView;

        let connection_id = node.connection_id.clone();
        let database = node.get_database_name().unwrap_or_default();
        let schema = node.get_schema_name();
        let table_name = node.name.clone();
        let schema_clone = schema.clone();
        let export_view = cx.new(|cx| {
            DataExportView::new(
                connection_id,
                database.clone(),
                schema_clone,
                table_name.clone(),
                window,
                cx,
            )
        });
        let clone_connection_id = node.connection_id.clone();
        let clone_global_db_state = global_state.clone();
        let clone_export_view = export_view.clone();
        let clone_table_name = table_name.clone();
        let clone_database = database.clone();
        cx.spawn(async move |cx: &mut AsyncApp| {
            let column_info = clone_global_db_state
                .list_columns(
                    cx,
                    clone_connection_id,
                    clone_database,
                    schema.into(),
                    clone_table_name,
                )
                .await;
            match column_info {
                Ok(columns) => {
                    cx.update(|cx| {
                        clone_export_view.update(cx, |this, cx| {
                            this.update_column_list(columns, cx);
                        });
                    });
                }
                Err(err) => {
                    error!("{}", err);
                }
            };
        })
        .detach();

        open_popup_window(
            PopupWindowOptions::new(t!("ImportExport.export_table").to_string())
                .size(800.0, 600.0),
            move |_window, _cx| export_view.clone(),
            cx,
        );
    }

    /// 处理关闭连接事件
    fn handle_close_connection(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        tab_container: Entity<TabContainer>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::WindowExt;

        let connection_id = node.connection_id.clone();
        let connection_name = node.name.clone();

        let close_task = tab_container.update(cx, |container, cx| {
            container.close_tabs_by_tab_from(&connection_id, window, cx)
        });

        let tree_clone = tree_view.clone();
        let global_state_clone = global_state.clone();

        cx.spawn(async move |cx: &mut AsyncApp| {
            let all_closed = close_task.await;
            if !all_closed {
                return;
            }

            let _ = cx.update(|cx| {
                if let Some(window_id) = cx.active_window() {
                    let _ = cx.update_window(window_id, |_, window, cx| {
                        let tree = tree_clone.clone();
                        let global_state = global_state_clone.clone();
                        let panel = objects_panel.clone();
                        let conn_id = connection_id.clone();
                        let conn_name = connection_name.clone();

                        window.open_dialog(cx, move |dialog, _window, _cx| {
                            let conn_id = conn_id.clone();
                            let conn_name = conn_name.clone();
                            let tree = tree.clone();
                            let global_state = global_state.clone();
                            let panel = panel.clone();

                            dialog
                                .overlay(false)
                                .title(t!("DbTreeEvent.confirm_close_connection_title").to_string())
                                .confirm()
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            t!(
                                                "DbTreeEvent.confirm_close_connection_message",
                                                name = conn_name
                                            )
                                            .to_string(),
                                        )
                                        .child(
                                            t!("DbTreeEvent.confirm_close_connection_desc")
                                                .to_string(),
                                        ),
                                )
                                .on_ok(move |_, _window, cx| {
                                    let conn_id = conn_id.clone();
                                    let tree = tree.clone();
                                    let global_state = global_state.clone();
                                    let panel = panel.clone();

                                    cx.spawn(async move |cx: &mut AsyncApp| {
                                        let result =
                                            global_state.disconnect_all(cx, conn_id.clone()).await;
                                        match result {
                                            Ok(_) => {
                                                let state_for_refresh = global_state.clone();
                                                let _ = cx.update(|cx| {
                                                    tree.update(cx, |tree_view, cx| {
                                                        tree_view.close_connection(&conn_id, cx);
                                                    });
                                                    if let Some(panel) = panel {
                                                        panel.update(cx, |panel, cx| {
                                                            panel.refresh(state_for_refresh, cx);
                                                        });
                                                    }
                                                    Self::show_success_async(
                                                        cx,
                                                        t!("DbTreeEvent.close_connection_success")
                                                            .to_string(),
                                                    );
                                                });
                                            }
                                            Err(e) => {
                                                let _ = cx.update(|cx| {
                                                    Self::show_error_async(
                                                        cx,
                                                        t!(
                                                            "DbTreeEvent.close_connection_failed",
                                                            error = e
                                                        )
                                                        .to_string(),
                                                    );
                                                });
                                            }
                                        }
                                    })
                                    .detach();
                                    true
                                })
                        });
                    });
                }
            });
        })
        .detach();
    }

    /// 处理删除连接事件
    fn handle_delete_connection(
        node: DbNode,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use one_core::storage::traits::Repository;
        use one_core::storage::{ConnectionRepository, GlobalStorageState};

        let connection_id = node.connection_id.clone();
        let connection_name = node.name.clone();
        let storage_manager = cx.global::<GlobalStorageState>().storage.clone();
        let global_state = cx.global::<GlobalDbState>().clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let conn_name = connection_name.clone();
            let storage = storage_manager.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let state = global_state.clone();

            dialog
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .overlay(false)
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_delete_connection_message",
                                name = conn_name
                            )
                            .to_string(),
                        )
                        .child(t!("Common.irreversible").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let conn_id = conn_id.clone();
                    let storage = storage.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let state = state.clone();
                    cx.spawn(async move |cx| match conn_id.parse::<i64>() {
                        Ok(id) => {
                            if let Some(conn_repo) = storage.get::<ConnectionRepository>() {
                                match conn_repo.delete(id) {
                                    Ok(_) => {
                                        let state_for_refresh = state.clone();
                                        let _ = cx.update(|cx| {
                                            tree.update(cx, |tree, cx| {
                                                tree.refresh_tree(conn_id.clone(), cx);
                                            });
                                            if let Some(panel) = panel {
                                                panel.update(cx, |panel, cx| {
                                                    panel.refresh(state_for_refresh, cx);
                                                });
                                            }
                                            Self::show_success_async(
                                                cx,
                                                t!("DbTreeEvent.delete_connection_success")
                                                    .to_string(),
                                            );
                                        });
                                    }
                                    Err(e) => {
                                        let _ = cx.update(|cx| {
                                            Self::show_error_async(
                                                cx,
                                                t!(
                                                    "DbTreeEvent.delete_connection_failed",
                                                    error = e
                                                )
                                                .to_string(),
                                            );
                                        });
                                    }
                                }
                            } else {
                                let _ = cx.update(|cx| {
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.delete_connection_repo_missing")
                                            .to_string(),
                                    );
                                });
                            }
                        }
                        Err(e) => {
                            let _ = cx.update(|cx| {
                                Self::show_error_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.delete_connection_invalid_id",
                                        error = e
                                    )
                                    .to_string(),
                                );
                            });
                        }
                    })
                    .detach();
                    true
                })
        });
    }

    /// 处理新建数据库事件
    fn handle_create_database(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::WindowExt;

        let connection_id = node.connection_id.clone();
        let database_type = node.database_type;

        let plugin_registry = cx.global::<DatabaseViewPluginRegistry>();
        let editor_view = if let Some(plugin) = plugin_registry.get(&database_type) {
            plugin.create_database_editor_view(connection_id.clone(), window, cx)
        } else {
            Self::show_error(
                window,
                t!("DbTreeEvent.unsupported_database_type", db_type = format!("{:?}", database_type))
                    .to_string(),
                cx,
            );
            return;
        };

        let global_state_clone = global_state.clone();
        let connection_id_clone = connection_id.clone();
        let tree_view_clone = tree_view.clone();

        let editor_view_for_ok = editor_view.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let editor_view_ok = editor_view_for_ok.clone();
            let connection_id_for_ok = connection_id_clone.clone();
            let global_state_for_ok = global_state_clone.clone();
            let tree_view_for_ok = tree_view_clone.clone();
            let panel = objects_panel.clone();

            dialog
                .title(t!("DbTreeEvent.create_database_title").to_string())
                .overlay(false)
                .child(editor_view.clone())
                .width(px(700.0))
                .button_props(
                    DialogButtonProps::default().ok_text(t!("Common.create").to_string()),
                )
                .footer(|ok, cancel, window, cx| vec![cancel(window, cx), ok(window, cx)])
                .on_ok(move |_, _window, cx| {
                    let sql = editor_view_ok.read(cx).get_sql(cx);
                    let database_name = editor_view_ok.read(cx).get_database_name(cx);
                    if sql.trim().is_empty() {
                        editor_view_ok.update(cx, |view, cx| {
                            view.set_save_error(t!("DbTreeEvent.sql_empty").to_string(), cx);
                        });
                        return false;
                    }

                    let connection_id = connection_id_for_ok.clone();
                    let global_state = global_state_for_ok.clone();
                    let tree_view = tree_view_for_ok.clone();
                    let editor_view = editor_view_ok.clone();
                    let window_id = cx.active_window();
                    let panel = panel.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let Some(window_id) = window_id else { return };
                        let result = global_state
                            .execute_single(cx, connection_id.clone(), sql, None, None)
                            .await;

                        match result {
                            Ok(sql_result) => match sql_result {
                                SqlResult::Query(_) => {}
                                SqlResult::Exec(_) => {
                                    let state_for_refresh = global_state.clone();
                                    let _ = cx.update_window(window_id, |_entity, window, cx| {
                                        window.close_dialog(cx);
                                        tree_view.update(cx, |tree, cx| {
                                            tree.add_database_to_selection(
                                                &connection_id,
                                                &database_name,
                                                cx,
                                            );
                                            tree.add_database_node(
                                                &connection_id,
                                                &database_name,
                                                cx,
                                            );
                                        });
                                        if let Some(panel) = panel {
                                            panel.update(cx, |panel, cx| {
                                                panel.refresh(state_for_refresh, cx);
                                            });
                                        }
                                        window.push_notification(
                                            Notification::success(
                                                t!("DbTreeEvent.create_database_success")
                                                    .to_string(),
                                            )
                                            .autohide(true),
                                            cx,
                                        );
                                    });
                                }
                                SqlResult::Error(err) => {
                                    let _ = editor_view.update(cx, |view, cx| {
                                        view.set_save_error(
                                            t!(
                                                "DbTreeEvent.create_database_failed",
                                                error = err.message
                                            )
                                            .to_string(),
                                            cx,
                                        );
                                    });
                                }
                            },
                            Err(e) => {
                                let _ = editor_view.update(cx, |view, cx| {
                                    view.set_save_error(
                                        t!("DbTreeEvent.create_database_failed", error = e)
                                            .to_string(),
                                        cx,
                                    );
                                });
                            }
                        }
                    })
                    .detach();

                    false
                })
                .on_cancel(|_, _window, _cx| true)
        });
    }

    /// 处理编辑数据库事件
    fn handle_edit_database(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::WindowExt;

        let connection_id = node.connection_id.clone();
        let database_name = node.name.clone();
        let database_type = node.database_type;

        let plugin_registry = cx.global::<DatabaseViewPluginRegistry>();
        let editor_view = if let Some(plugin) = plugin_registry.get(&database_type) {
            plugin.create_database_editor_view_for_edit(
                connection_id.clone(),
                database_name.clone(),
                window,
                cx,
            )
        } else {
            Self::show_error(
                window,
                t!("DbTreeEvent.unsupported_database_type", db_type = format!("{:?}", database_type))
                    .to_string(),
                cx,
            );
            return;
        };

        let global_state_clone = global_state.clone();
        let connection_id_clone = connection_id.clone();
        let tree_view_clone = tree_view.clone();

        let editor_view_for_ok = editor_view.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let editor_view_ok = editor_view_for_ok.clone();
            let connection_id_for_ok = connection_id_clone.clone();
            let global_state_for_ok = global_state_clone.clone();
            let tree_view_for_ok = tree_view_clone.clone();
            let panel = objects_panel.clone();

            dialog
                .title(
                    t!("DbTreeEvent.edit_database_title", name = database_name).to_string(),
                )
                .child(editor_view.clone())
                .overlay(false)
                .width(px(700.0))
                .button_props(
                    DialogButtonProps::default().ok_text(t!("Common.save").to_string()),
                )
                .footer(|ok, cancel, window, cx| vec![cancel(window, cx), ok(window, cx)])
                .on_ok(move |_, _window, cx| {
                    let sql = editor_view_ok.read(cx).get_sql(cx);
                    if sql.trim().is_empty() {
                        editor_view_ok.update(cx, |view, cx| {
                            view.set_save_error(t!("DbTreeEvent.sql_empty").to_string(), cx);
                        });
                        return false;
                    }

                    let connection_id = connection_id_for_ok.clone();
                    let global_state = global_state_for_ok.clone();
                    let tree_view = tree_view_for_ok.clone();
                    let editor_view = editor_view_ok.clone();
                    let window_id = cx.active_window();
                    let panel = panel.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let Some(window_id) = window_id else { return };
                        let result = global_state
                            .execute_single(cx, connection_id.clone(), sql, None, None)
                            .await;

                        match result {
                            Ok(sql_result) => match sql_result {
                                SqlResult::Query(_) => {}
                                SqlResult::Exec(_) => {
                                    let state_for_refresh = global_state.clone();
                                    let _ = cx.update_window(window_id, |_entity, window, cx| {
                                        window.close_dialog(cx);
                                        tree_view.update(cx, |tree, cx| {
                                            tree.refresh_tree(connection_id.clone(), cx);
                                        });
                                        if let Some(panel) = panel {
                                            panel.update(cx, |panel, cx| {
                                                panel.refresh(state_for_refresh, cx);
                                            });
                                        }
                                        window.push_notification(
                                            Notification::success(
                                                t!("DbTreeEvent.edit_database_success")
                                                    .to_string(),
                                            )
                                            .autohide(true),
                                            cx,
                                        );
                                    });
                                }
                                SqlResult::Error(err) => {
                                    let _ = editor_view.update(cx, |view, cx| {
                                        view.set_save_error(
                                            t!(
                                                "DbTreeEvent.edit_database_failed",
                                                error = err.message
                                            )
                                            .to_string(),
                                            cx,
                                        );
                                    });
                                }
                            },
                            Err(e) => {
                                let _ = editor_view.update(cx, |view, cx| {
                                    view.set_save_error(
                                        t!("DbTreeEvent.edit_database_failed", error = e)
                                            .to_string(),
                                        cx,
                                    );
                                });
                            }
                        }
                    })
                    .detach();

                    false
                })
                .on_cancel(|_, _window, _cx| true)
        });
    }

    /// 处理关闭数据库事件
    fn handle_close_database(
        node: DbNode,
        _global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::WindowExt;

        let connection_id = node.connection_id.clone();
        let database_name = node.name.clone();

        let tree_clone = tree_view.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let db_name = database_name.clone();
            let tree = tree_clone.clone();

            dialog
                .title(t!("Database.close_database").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(format!(
                            "{} \"{}\" {}?",
                            t!("Common.confirm").to_string(),
                            db_name,
                            t!("Database.close_database").to_string()
                        ))
                        .child(t!("Database.close_hint").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let db_name = db_name.clone();
                    let db_name_log = db_name.clone();
                    let tree = tree.clone();
                    let db_node_id = format!("{}:{}", conn_id, db_name);

                    tree.update(cx, |tree_view, cx| {
                        tree_view.close_database(&db_node_id, cx);
                    });
                    Self::show_success_async(
                        cx,
                        t!("DbTreeEvent.close_database_success", name = db_name_log).to_string(),
                    );
                    true
                })
        });
    }

    /// 处理删除数据库事件
    fn handle_delete_database(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let database_name = node.name.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let db_name = database_name.clone();
            let state = global_state.clone();
            let db_name_display = database_name.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_delete_database_message",
                                name = db_name_display
                            )
                            .to_string(),
                        )
                        .child(t!("DbTreeEvent.delete_database_desc").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let conn_id = conn_id.clone();
                    let db_name = db_name.clone();
                    let db_name_log = db_name.clone();
                    let db_name_for_remove = db_name.clone();
                    let tree = tree.clone();
                    let state = state.clone();
                    let panel = panel.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let result = state
                            .drop_database(cx, conn_id.clone(), db_name.clone())
                            .await;
                        match result {
                            Ok(_) => {
                                let state_for_refresh = state.clone();
                                let _ = cx.update(|cx| {
                                    tree.update(cx, |tree, cx| {
                                        tree.remove_database_node(
                                            &conn_id,
                                            &db_name_for_remove,
                                            cx,
                                        );
                                    });
                                    if let Some(panel) = panel {
                                        panel.update(cx, |panel, cx| {
                                            panel.refresh(state_for_refresh, cx);
                                        });
                                    }
                                    Self::show_success_async(
                                        cx,
                                        t!(
                                            "DbTreeEvent.delete_database_success",
                                            name = db_name_log
                                        )
                                        .to_string(),
                                    );
                                });
                            }
                            Err(e) => {
                                let _ = cx.update(|cx| {
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.delete_database_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    true
                })
        });
    }

    /// 处理新建模式事件
    fn handle_create_schema(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::WindowExt;

        let connection_id = node.connection_id.clone();
        let database_name = node.name.clone();
        let database_type = node.database_type;

        let plugin_registry = cx.global::<DatabaseViewPluginRegistry>();
        let editor_view = if let Some(plugin) = plugin_registry.get(&database_type) {
            if let Some(view) = plugin.create_schema_editor_view(
                connection_id.clone(),
                database_name.clone(),
                window,
                cx,
            ) {
                view
            } else {
                Self::show_error(
                    window,
                    t!(
                        "DbTreeEvent.create_schema_unsupported",
                        db_type = format!("{:?}", database_type)
                    )
                    .to_string(),
                    cx,
                );
                return;
            }
        } else {
            Self::show_error(
                window,
                t!("DbTreeEvent.unsupported_database_type", db_type = format!("{:?}", database_type))
                    .to_string(),
                cx,
            );
            return;
        };

        let global_state_clone = global_state.clone();
        let connection_id_clone = connection_id.clone();
        let tree_view_clone = tree_view.clone();
        let database_name_clone = database_name.clone();

        let editor_view_for_ok = editor_view.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let editor_view_ok = editor_view_for_ok.clone();
            let connection_id_for_ok = connection_id_clone.clone();
            let global_state_for_ok = global_state_clone.clone();
            let tree_view_for_ok = tree_view_clone.clone();
            let database_for_ok = database_name_clone.clone();
            let panel = objects_panel.clone();

            dialog
                .overlay(false)
                .title(
                    t!("DbTreeEvent.create_schema_title", name = database_name).to_string(),
                )
                .child(editor_view.clone())
                .width(px(600.0))
                .button_props(
                    DialogButtonProps::default().ok_text(t!("Common.create").to_string()),
                )
                .footer(|ok, cancel, window, cx| vec![cancel(window, cx), ok(window, cx)])
                .on_ok(move |_, _window, cx| {
                    let sql = editor_view_ok.read(cx).get_sql(cx);
                    if sql.trim().is_empty() {
                        editor_view_ok.update(cx, |view, cx| {
                            view.set_save_error(t!("DbTreeEvent.sql_empty").to_string(), cx);
                        });
                        return false;
                    }

                    let connection_id = connection_id_for_ok.clone();
                    let global_state = global_state_for_ok.clone();
                    let tree_view = tree_view_for_ok.clone();
                    let database = database_for_ok.clone();
                    let editor_view = editor_view_ok.clone();
                    let window_id = cx.active_window();
                    let panel = panel.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let Some(window_id) = window_id else { return };
                        let result = global_state
                            .execute_single(
                                cx,
                                connection_id.clone(),
                                sql,
                                Some(database.clone()),
                                None,
                            )
                            .await;

                        match result {
                            Ok(sql_result) => match sql_result {
                                SqlResult::Query(_) => {}
                                SqlResult::Exec(_) => {
                                    let state_for_refresh = global_state.clone();
                                    let db_node_id = format!("{}:{}", connection_id, database);
                                    let _ = cx.update_window(window_id, |_entity, window, cx| {
                                        window.close_dialog(cx);
                                        tree_view.update(cx, |tree, cx| {
                                            tree.refresh_tree(db_node_id, cx);
                                        });
                                        if let Some(panel) = panel {
                                            panel.update(cx, |panel, cx| {
                                                panel.refresh(state_for_refresh, cx);
                                            });
                                        }
                                        window.push_notification(
                                            Notification::success(
                                                t!("DbTreeEvent.create_schema_success")
                                                    .to_string(),
                                            )
                                            .autohide(true),
                                            cx,
                                        );
                                    });
                                }
                                SqlResult::Error(err) => {
                                    let _ = editor_view.update(cx, |view, cx| {
                                        view.set_save_error(
                                            t!(
                                                "DbTreeEvent.create_schema_failed",
                                                error = err.message
                                            )
                                            .to_string(),
                                            cx,
                                        );
                                    });
                                }
                            },
                            Err(e) => {
                                let _ = editor_view.update(cx, |view, cx| {
                                    view.set_save_error(
                                        t!("DbTreeEvent.create_schema_failed", error = e)
                                            .to_string(),
                                        cx,
                                    );
                                });
                            }
                        }
                    })
                    .detach();

                    false
                })
                .on_cancel(|_, _window, _cx| true)
        });
    }

    /// 处理删除模式事件
    fn handle_delete_schema(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let schema_name = node.name.clone();
        let metadata = node.metadata.clone();
        let database_type = node.database_type;

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let schema = schema_name.clone();
            let meta = metadata.clone();
            let state = global_state.clone();
            let schema_display = schema_name.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_delete_schema_message",
                                name = schema_display
                            )
                            .to_string(),
                        )
                        .child(t!("DbTreeEvent.delete_schema_desc").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let conn_id = conn_id.clone();
                    let schema = schema.clone();
                    let meta = meta.clone();
                    let state = state.clone();
                    let schema_log = schema.clone();
                    let schema_for_remove = schema.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let database = meta
                        .get("database")
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let database_for_remove = database.clone();

                    let sql = state
                        .get_plugin(&database_type)
                        .map(|p| p.build_drop_schema_sql(&schema))
                        .unwrap_or_else(|_| format!("DROP SCHEMA \"{}\"", schema));

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let result = state
                            .execute_single(cx, conn_id.clone(), sql, Some(database.clone()), None)
                            .await;

                        match result {
                            Ok(sql_result) => match sql_result {
                                SqlResult::Query(_) => {}
                                SqlResult::Exec(_) => {
                                    let state_for_refresh = state.clone();
                                    let _ = cx.update(|cx| {
                                        tree.update(cx, |tree, cx| {
                                            tree.remove_schema_node(
                                                &conn_id,
                                                &database_for_remove,
                                                &schema_for_remove,
                                                cx,
                                            );
                                        });
                                        if let Some(panel) = panel {
                                            panel.update(cx, |panel, cx| {
                                                panel.refresh(state_for_refresh, cx);
                                            });
                                        }
                                        Self::show_success_async(
                                            cx,
                                            t!(
                                                "DbTreeEvent.delete_schema_success",
                                                name = schema_log
                                            )
                                            .to_string(),
                                        );
                                    });
                                }
                                SqlResult::Error(err) => {
                                    let _ = cx.update(|cx| {
                                        Self::show_error_async(
                                            cx,
                                            t!(
                                                "DbTreeEvent.delete_schema_failed",
                                                error = err.message
                                            )
                                            .to_string(),
                                        );
                                    });
                                }
                            },
                            Err(e) => {
                                let _ = cx.update(|cx| {
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.delete_schema_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    true
                })
        });
    }

    fn handle_batch_action(
        action: DatabaseObjectsBatchAction,
        nodes: Vec<DbNode>,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if nodes.is_empty() {
            return;
        }

        match action {
            DatabaseObjectsBatchAction::DeleteConnection => {
                Self::handle_batch_delete_connections(nodes, tree_view, objects_panel, window, cx);
            }
            DatabaseObjectsBatchAction::DeleteDatabase => {
                Self::handle_batch_delete_databases(
                    nodes,
                    global_state,
                    tree_view,
                    objects_panel,
                    window,
                    cx,
                );
            }
            DatabaseObjectsBatchAction::DeleteSchema => {
                Self::handle_batch_delete_schemas(
                    nodes,
                    global_state,
                    tree_view,
                    objects_panel,
                    window,
                    cx,
                );
            }
            DatabaseObjectsBatchAction::DeleteTable => {
                Self::handle_batch_delete_tables(
                    nodes,
                    global_state,
                    tree_view,
                    objects_panel,
                    window,
                    cx,
                );
            }
            DatabaseObjectsBatchAction::DeleteView => {
                Self::handle_batch_delete_views(
                    nodes,
                    global_state,
                    tree_view,
                    objects_panel,
                    window,
                    cx,
                );
            }
            DatabaseObjectsBatchAction::DeleteQuery => {
                Self::handle_batch_delete_queries(
                    nodes,
                    tree_view,
                    objects_panel,
                    window,
                    cx,
                );
            }
        }
    }

    fn handle_batch_delete_connections(
        nodes: Vec<DbNode>,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use one_core::storage::traits::Repository;
        use one_core::storage::{ConnectionRepository, GlobalStorageState};

        let storage_manager = cx.global::<GlobalStorageState>().storage.clone();
        let global_state = cx.global::<GlobalDbState>().clone();
        let delete_count = nodes.len();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let storage = storage_manager.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let state = global_state.clone();
            let nodes = nodes.clone();

            dialog
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .overlay(false)
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_batch_delete_connections",
                                count = delete_count
                            )
                            .to_string(),
                        )
                        .child(t!("Common.irreversible").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let storage = storage.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let state = state.clone();
                    let nodes = nodes.clone();

                    cx.spawn(async move |cx| {
                        let mut removed_ids: Vec<String> = Vec::new();
                        let mut errors: Vec<String> = Vec::new();

                        for node in nodes {
                            match node.connection_id.parse::<i64>() {
                                Ok(id) => {
                                    if let Some(conn_repo) = storage.get::<ConnectionRepository>() {
                                        match conn_repo.delete(id) {
                                            Ok(_) => removed_ids.push(node.connection_id.clone()),
                                            Err(error) => errors.push(format!(
                                                "{}: {}",
                                                node.name, error
                                            )),
                                        }
                                    } else {
                                        errors.push(
                                            t!(
                                                "DbTreeEvent.delete_connection_repo_missing_named",
                                                name = node.name
                                            )
                                            .to_string(),
                                        );
                                    }
                                }
                                Err(error) => errors.push(
                                    t!(
                                        "DbTreeEvent.delete_connection_invalid_id_named",
                                        name = node.name,
                                        error = error
                                    )
                                    .to_string(),
                                ),
                            }
                        }

                        let state_for_refresh = state.clone();
                        let _ = cx.update(|cx| {
                            if !removed_ids.is_empty() {
                                tree.update(cx, |tree, cx| {
                                    for conn_id in &removed_ids {
                                        tree.refresh_tree(conn_id.clone(), cx);
                                    }
                                });
                                if let Some(panel) = panel {
                                    panel.update(cx, |panel, cx| {
                                        panel.refresh(state_for_refresh, cx);
                                    });
                                }
                            }

                            if errors.is_empty() {
                                Self::show_success_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_connections_success",
                                        count = removed_ids.len()
                                    )
                                    .to_string(),
                                );
                            } else {
                                Self::show_error_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_connections_failed",
                                        errors = errors.join("; ")
                                    )
                                    .to_string(),
                                );
                            }
                        });
                    })
                    .detach();
                    true
                })
        });
    }

    fn handle_batch_delete_databases(
        nodes: Vec<DbNode>,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let delete_count = nodes.len();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let state = global_state.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let nodes = nodes.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_batch_delete_databases",
                                count = delete_count
                            )
                            .to_string(),
                        )
                        .child(t!("DbTreeEvent.delete_database_desc").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let state = state.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let nodes = nodes.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let mut removed: Vec<(String, String)> = Vec::new();
                        let mut errors: Vec<String> = Vec::new();

                        for node in nodes {
                            let connection_id = node.connection_id.clone();
                            let database_name = node.name.clone();
                            let result = state
                                .drop_database(cx, connection_id.clone(), database_name.clone())
                                .await;
                            match result {
                                Ok(_) => removed.push((connection_id, database_name)),
                                Err(error) => {
                                    errors.push(format!("{}: {}", database_name, error));
                                }
                            }
                        }

                        let state_for_refresh = state.clone();
                        let _ = cx.update(|cx| {
                            if !removed.is_empty() {
                                tree.update(cx, |tree, cx| {
                                    for (conn_id, db_name) in &removed {
                                        tree.remove_database_node(conn_id, db_name, cx);
                                    }
                                });
                                if let Some(panel) = panel {
                                    panel.update(cx, |panel, cx| {
                                        panel.refresh(state_for_refresh, cx);
                                    });
                                }
                            }

                            if errors.is_empty() {
                                Self::show_success_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_databases_success",
                                        count = removed.len()
                                    )
                                    .to_string(),
                                );
                            } else {
                                Self::show_error_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_databases_failed",
                                        errors = errors.join("; ")
                                    )
                                    .to_string(),
                                );
                            }
                        });
                    })
                    .detach();
                    true
                })
        });
    }

    fn handle_batch_delete_schemas(
        nodes: Vec<DbNode>,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let delete_count = nodes.len();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let state = global_state.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let nodes = nodes.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_batch_delete_schemas",
                                count = delete_count
                            )
                            .to_string(),
                        )
                        .child(t!("DbTreeEvent.delete_schema_desc").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let state = state.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let nodes = nodes.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let mut removed: Vec<(String, String, String)> = Vec::new();
                        let mut errors: Vec<String> = Vec::new();

                        for node in nodes {
                            let connection_id = node.connection_id.clone();
                            let schema_name = node.name.clone();
                            let database_type = node.database_type;
                            let database = node
                                .metadata
                                .get("database")
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let sql = state
                                .get_plugin(&database_type)
                                .map(|p| p.build_drop_schema_sql(&schema_name))
                                .unwrap_or_else(|_| format!("DROP SCHEMA \"{}\"", schema_name));

                            let result = state
                                .execute_single(
                                    cx,
                                    connection_id.clone(),
                                    sql,
                                    Some(database.clone()),
                                    None,
                                )
                                .await;

                            match result {
                                Ok(sql_result) => match sql_result {
                                    SqlResult::Query(_) => {}
                                    SqlResult::Exec(_) => {
                                        removed.push((connection_id, database, schema_name));
                                    }
                                    SqlResult::Error(err) => {
                                        errors.push(format!("{}: {}", schema_name, err.message));
                                    }
                                },
                                Err(error) => {
                                    errors.push(format!("{}: {}", schema_name, error));
                                }
                            }
                        }

                        let state_for_refresh = state.clone();
                        let _ = cx.update(|cx| {
                            if !removed.is_empty() {
                                tree.update(cx, |tree, cx| {
                                    for (conn_id, database, schema) in &removed {
                                        tree.remove_schema_node(conn_id, database, schema, cx);
                                    }
                                });
                                if let Some(panel) = panel {
                                    panel.update(cx, |panel, cx| {
                                        panel.refresh(state_for_refresh, cx);
                                    });
                                }
                            }

                            if errors.is_empty() {
                                Self::show_success_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_schemas_success",
                                        count = removed.len()
                                    )
                                    .to_string(),
                                );
                            } else {
                                Self::show_error_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_schemas_failed",
                                        errors = errors.join("; ")
                                    )
                                    .to_string(),
                                );
                            }
                        });
                    })
                    .detach();
                    true
                })
        });
    }

    fn handle_batch_delete_tables(
        nodes: Vec<DbNode>,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let delete_count = nodes.len();
        let window_id = cx.active_window();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let state = global_state.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let nodes = nodes.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_batch_delete_tables",
                                count = delete_count
                            )
                            .to_string(),
                        )
                        .child(t!("DbTreeEvent.delete_table_desc").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let state = state.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let nodes = nodes.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let mut removed: Vec<String> = Vec::new();
                        let mut errors: Vec<String> = Vec::new();

                        for node in nodes {
                            let Some(table_name) =
                                node.get_table_name().or_else(|| Some(node.name.clone()))
                            else {
                                continue;
                            };
                            let database_name = node.get_database_name();
                            let schema_name = node.get_schema_name();
                            let (database, schema) = if database_name.is_none()
                                && schema_name.is_some()
                            {
                                (schema_name.clone().unwrap_or_default(), None)
                            } else {
                                (database_name.unwrap_or_default(), schema_name)
                            };

                            let task = state
                                .drop_table(
                                    &mut cx.clone(),
                                    node.connection_id.clone(),
                                    database,
                                    schema,
                                    table_name.clone(),
                                )
                                .await;

                            match task {
                                Ok(_) => removed.push(node.id.clone()),
                                Err(error) => errors.push(format!("{}: {}", table_name, error)),
                            }
                        }

                        let state_for_refresh = state.clone();
                        let _ = cx.update(|cx| {
                            if let Some(window_id) = window_id {
                                cx.update_window(window_id, |_entity, window, cx| {
                                    window.close_dialog(cx);
                                })
                                .ok();
                            }

                            if !removed.is_empty() {
                                tree.update(cx, |tree, cx| {
                                    for node_id in &removed {
                                        tree.remove_table_node(node_id, cx);
                                    }
                                });
                                if let Some(panel) = panel {
                                    panel.update(cx, |panel, cx| {
                                        panel.refresh(state_for_refresh, cx);
                                    });
                                }
                            }

                            if errors.is_empty() {
                                Self::show_success_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_tables_success",
                                        count = removed.len()
                                    )
                                    .to_string(),
                                );
                            } else {
                                Self::show_error_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_tables_failed",
                                        errors = errors.join("; ")
                                    )
                                    .to_string(),
                                );
                            }
                        });
                    })
                    .detach();
                    false
                })
        });
    }

    fn handle_batch_delete_views(
        nodes: Vec<DbNode>,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let delete_count = nodes.len();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let state = global_state.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let nodes = nodes.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_batch_delete_views",
                                count = delete_count
                            )
                            .to_string(),
                        )
                        .child(t!("Common.irreversible").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let state = state.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let nodes = nodes.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let mut removed: Vec<String> = Vec::new();
                        let mut errors: Vec<String> = Vec::new();

                        for node in nodes {
                            let database = node
                                .metadata
                                .get("database")
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let result = state
                                .drop_view(
                                    cx,
                                    node.connection_id.clone(),
                                    database,
                                    node.name.clone(),
                                )
                                .await;
                            match result {
                                Ok(_) => removed.push(node.id.clone()),
                                Err(error) => {
                                    errors.push(format!("{}: {}", node.name, error));
                                }
                            }
                        }

                        let state_for_refresh = state.clone();
                        let _ = cx.update(|cx| {
                            if !removed.is_empty() {
                                tree.update(cx, |tree, cx| {
                                    for node_id in &removed {
                                        tree.remove_view_node(node_id, cx);
                                    }
                                });
                                if let Some(panel) = panel {
                                    panel.update(cx, |panel, cx| {
                                        panel.refresh(state_for_refresh, cx);
                                    });
                                }
                            }

                            if errors.is_empty() {
                                Self::show_success_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_views_success",
                                        count = removed.len()
                                    )
                                    .to_string(),
                                );
                            } else {
                                Self::show_error_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_views_failed",
                                        errors = errors.join("; ")
                                    )
                                    .to_string(),
                                );
                            }
                        });
                    })
                    .detach();
                    true
                })
        });
    }

    fn handle_batch_delete_queries(
        nodes: Vec<DbNode>,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use one_core::gpui_tokio::Tokio;

        let delete_count = nodes.len();
        let global_state = cx.global::<GlobalDbState>().clone();
        let window_id = cx.active_window();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let state = global_state.clone();
            let nodes = nodes.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_batch_delete_queries",
                                count = delete_count
                            )
                            .to_string(),
                        )
                        .child(t!("Common.irreversible").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let state = state.clone();
                    let nodes = nodes.clone();

                    let nodes_for_delete = nodes.clone();
                    let task = Tokio::spawn_result(cx, async move {
                        let mut errors: Vec<String> = Vec::new();
                        for node in &nodes_for_delete {
                            let Some(path_str) = node.metadata.get("file_path") else {
                                continue;
                            };
                            let file_path = std::path::PathBuf::from(path_str);
                            if let Err(error) = tokio::fs::remove_file(file_path).await {
                                errors.push(format!("{}: {}", node.name, error));
                            }
                        }
                        Ok(errors)
                    });

                    cx.spawn(async move |cx| {
                        let errors = task.await.unwrap_or_else(|error| {
                            vec![
                                t!("DbTreeEvent.delete_query_failed", error = error).to_string()
                            ]
                        });

                        let mut parent_ids: HashSet<String> = HashSet::new();
                        for node in &nodes {
                            if let Some(parent_id) = node.parent_context.clone() {
                                parent_ids.insert(parent_id);
                            }
                        }

                        if let Some(window_id) = window_id {
                            _ = cx.update_window(window_id, |_entity, window, cx| {
                                if !parent_ids.is_empty() {
                                    tree.update(cx, |tree, cx| {
                                        for parent_id in &parent_ids {
                                            tree.refresh_tree(parent_id.clone(), cx);
                                        }
                                });
                            }
                            if let Some(panel) = panel {
                                panel.update(cx, |panel, cx| {
                                    panel.refresh(state, cx);
                                });
                            }
                            window.close_dialog(cx);

                            if errors.is_empty() {
                                Self::show_success_async(
                                    cx,
                                    t!(
                                        "DbTreeEvent.batch_delete_queries_success",
                                        count = delete_count
                                    )
                                    .to_string(),
                                );
                                } else {
                                    Self::show_error_async(
                                        cx,
                                        t!(
                                            "DbTreeEvent.batch_delete_queries_failed",
                                            errors = errors.join("; ")
                                        )
                                        .to_string(),
                                    );
                                }
                            });
                        }
                    })
                    .detach();
                    false
                })
        });
    }

    /// 处理删除表事件
    fn handle_delete_table(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let table_node_id = node.id.clone();
        let database_name = node.get_database_name();
        let schema_name = node.get_schema_name();
        let table_name = node.get_table_name();
        let window_id = cx.active_window();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let tbl_name = table_name.clone();
            let tbl_node_id = table_node_id.clone();
            let state = global_state.clone();
            let tbl_name_display = table_name.as_ref().map(|s| s.as_str()).unwrap_or("");
            let tree = tree_view.clone();
            let panel = objects_panel.clone();
            let db_name = database_name.clone();
            let sch_name = schema_name.clone();

            dialog
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .overlay(false)
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_delete_table_message",
                                name = tbl_name_display
                            )
                            .to_string(),
                        )
                        .child(t!("DbTreeEvent.delete_table_desc").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let conn_id = conn_id.clone();
                    let tbl_name = tbl_name.clone();
                    let tbl_node_id = tbl_node_id.clone();
                    let state = state.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();
                    let db_name = db_name.clone();
                    let sch_name = sch_name.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let Some(window_id) = window_id else { return };
                        let Some(tbl_name_value) = tbl_name else { return };

                        // For Oracle: use schema as database, for others: use database
                        let (database, schema) = if db_name.is_none() && sch_name.is_some() {
                            // Oracle case: no database, only schema
                            (sch_name.clone().unwrap_or_default(), None)
                        } else {
                            // Other databases: database with optional schema
                            (db_name.unwrap_or_default(), sch_name)
                        };

                        let task = state
                            .drop_table(&mut cx.clone(), conn_id.clone(), database, schema, tbl_name_value.clone())
                            .await;

                        match task {
                            Ok(_) => {
                                let state_for_refresh = state.clone();
                                let _ = cx.update_window(window_id, |_entity, window, cx| {
                                    window.close_dialog(cx);
                                    tree.update(cx, |tree, cx| {
                                        tree.remove_table_node(&tbl_node_id, cx);
                                    });
                                    if let Some(panel) = panel {
                                        panel.update(cx, |panel, cx| {
                                            panel.refresh(state_for_refresh, cx);
                                        });
                                    }
                                    Self::show_success_async(
                                        cx,
                                        t!(
                                            "DbTreeEvent.delete_table_success",
                                            name = tbl_name_value
                                        )
                                        .to_string(),
                                    );
                                });
                            }
                            Err(e) => {
                                let _ = cx.update_window(window_id, |_entity, window, cx| {
                                    window.close_dialog(cx);
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.delete_table_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    false
                })
        });
    }

    /// 处理重命名表事件
    fn handle_rename_table(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::{
            WindowExt,
            input::{Input, InputState},
        };

        let connection_id = node.connection_id.clone();
        let old_table_name = node.name.clone();
        let metadata = node.metadata.clone();

        // 创建输入框状态
        let input_state = cx.new(|cx| {
            let mut state =
                InputState::new(window, cx).placeholder(t!("DbTreeEvent.rename_table_placeholder").to_string());
            state.set_value(old_table_name.clone(), window, cx);
            state
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let old_name = old_table_name.clone();
            let meta = metadata.clone();
            let state = global_state.clone();
            let input = input_state.clone();
            let tree = tree_view.clone();

            dialog
                .overlay(false)
                .title(t!("DbTreeEvent.rename_table_title").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_4()
                        .p_4()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(80.))
                                        .child(t!("DbTreeEvent.rename_table_old_label").to_string()),
                                )
                                .child(div().flex_1().child(old_name.clone())),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(80.))
                                        .child(t!("DbTreeEvent.rename_table_new_label").to_string()),
                                )
                                .child(div().flex_1().child(Input::new(&input))),
                        ),
                )
                .on_ok(move |_, _, cx| {
                    let new_name = input.read(cx).text().to_string().trim().to_string();
                    if new_name.is_empty() || new_name == old_name {
                        return false; // 不关闭对话框
                    }

                    let conn_id = conn_id.clone();
                    let old_name = old_name.clone();
                    let meta = meta.clone();
                    let state = state.clone();
                    let tree = tree.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let old_name_log = old_name.clone();
                        let new_name_log = new_name.clone();
                        let database = meta
                            .get("database")
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let db_node_id = format!("{}:{}", conn_id, database);

                        let task = state
                            .rename_table(
                                cx,
                                conn_id.clone(),
                                database,
                                old_name.clone(),
                                new_name.clone(),
                            )
                            .await;
                        match task {
                            Ok(_) => {
                                let _ = cx.update(|cx| {
                                    // 刷新数据库节点以显示新表名
                                    tree.update(cx, |tree, cx| {
                                        tree.refresh_tree(db_node_id, cx);
                                    });
                                    Self::show_success_async(
                                        cx,
                                        t!(
                                            "DbTreeEvent.rename_table_success",
                                            old = old_name_log,
                                            new = new_name_log
                                        )
                                        .to_string(),
                                    );
                                });
                            }
                            Err(e) => {
                                let _ = cx.update(|cx| {
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.rename_table_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    true
                })
        });
    }

    /// 处理清空表事件
    fn handle_truncate_table(
        node: DbNode,
        global_state: GlobalDbState,
        _tree_view: Entity<DbTreeView>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let table_name = node.name.clone();
        let metadata = node.metadata.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let tbl_name = table_name.clone();
            let meta = metadata.clone();
            let state = global_state.clone();
            let tbl_name_display = table_name.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_clear").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_truncate_table_message",
                                name = tbl_name_display
                            )
                            .to_string(),
                        )
                        .child(t!("DbTreeEvent.truncate_table_desc").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let conn_id = conn_id.clone();
                    let tbl_name = tbl_name.clone();
                    let meta = meta.clone();
                    let state = state.clone();
                    let tbl_name_log = tbl_name.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let database = meta
                            .get("database")
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let task = state
                            .truncate_table(cx, conn_id.clone(), database, tbl_name.clone())
                            .await;

                        match task {
                            Ok(_) => {
                                let _ = cx.update(|cx| {
                                    Self::show_success_async(
                                        cx,
                                        t!(
                                            "DbTreeEvent.truncate_table_success",
                                            name = tbl_name_log
                                        )
                                        .to_string(),
                                    );
                                });
                            }
                            Err(e) => {
                                let _ = cx.update(|cx| {
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.truncate_table_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    true
                })
        });
    }

    /// 处理删除视图事件
    fn handle_delete_view(
        node: DbNode,
        global_state: GlobalDbState,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let connection_id = node.connection_id.clone();
        let view_name = node.name.clone();
        let view_node_id = node.id.clone();
        let metadata = node.metadata.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let v_name = view_name.clone();
            let v_node_id = view_node_id.clone();
            let meta = metadata.clone();
            let state = global_state.clone();
            let v_name_display = view_name.clone();
            let tree = tree_view.clone();
            let panel = objects_panel.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_delete_view_message",
                                name = v_name_display
                            )
                            .to_string(),
                        )
                        .child(t!("Common.irreversible").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let conn_id = conn_id.clone();
                    let v_name = v_name.clone();
                    let v_node_id = v_node_id.clone();
                    let meta = meta.clone();
                    let state = state.clone();
                    let v_name_log = v_name.clone();
                    let tree = tree.clone();
                    let panel = panel.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let database = meta
                            .get("database")
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let result = state
                            .drop_view(cx, conn_id.clone(), database, v_name.clone())
                            .await;

                        match result {
                            Ok(_) => {
                                let state_for_refresh = state.clone();
                                let _ = cx.update(|cx| {
                                    tree.update(cx, |tree, cx| {
                                        tree.remove_view_node(&v_node_id, cx);
                                    });
                                    if let Some(panel) = panel {
                                        panel.update(cx, |panel, cx| {
                                            panel.refresh(state_for_refresh, cx);
                                        });
                                    }
                                    Self::show_success_async(
                                        cx,
                                        t!(
                                            "DbTreeEvent.delete_view_success",
                                            name = v_name_log
                                        )
                                        .to_string(),
                                    );
                                });
                            }
                            Err(e) => {
                                let _ = cx.update(|cx| {
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.delete_view_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    true
                })
        });
    }

    /// 处理打开命名查询事件
    fn handle_open_named_query(
        node: DbNode,
        tab_container: Entity<TabContainer>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let file_path = node
            .metadata
            .get("file_path")
            .map(|p| std::path::PathBuf::from(p));

        if let Some(path) = file_path {
            let connection_id = node.connection_id.clone();
            let query_name = node.name.clone();
            let database_type = node.database_type;
            let database = node.get_database_name();
            let schema = node.get_schema_name();
            let tab_id = format!("query-{}", query_name);

            let tab_id_clone = tab_id.clone();
            let conn_id_clone = connection_id.clone();

            tab_container.update(cx, |container, cx| {
                container.activate_or_add_tab_lazy(
                    tab_id.clone(),
                    move |window, cx| {
                        let sql_editor = cx.new(|cx| {
                            SqlEditorTab::new_with_file_path(
                                path.clone(),
                                query_name.clone(),
                                connection_id.clone(),
                                database_type,
                                database.clone(),
                                schema.clone(),
                                window,
                                cx,
                            )
                        });
                        TabItem::new(tab_id_clone.clone(), conn_id_clone.clone(), sql_editor)
                    },
                    window,
                    cx,
                );
            });
        }
    }

    /// 处理重命名查询事件
    fn handle_rename_query(
        node: DbNode,
        db_tree: Entity<DbTreeView>,
        _global_state: GlobalDbState,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::{
            WindowExt,
            input::{Input, InputState},
        };

        let Some(path_str) = node.metadata.get("file_path") else {
            return;
        };

        let old_path = std::path::PathBuf::from(path_str);
        let old_name = node.name.clone();
        let parent_id = node.parent_context.clone();
        let window_id = cx.active_window().expect("No active window");

        let input_state = cx.new(|cx| {
            let mut state =
                InputState::new(window, cx).placeholder(t!("DbTreeEvent.rename_query_placeholder").to_string());
            state.set_value(old_name.clone(), window, cx);
            state
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input = input_state.clone();
            let db_tree = db_tree.clone();
            let old_path = old_path.clone();
            let old_name = old_name.clone();
            let parent_id = parent_id.clone();

            dialog
                .overlay(false)
                .title(t!("DbTreeEvent.rename_query_title").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_4()
                        .p_4()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(80.))
                                        .child(t!("DbTreeEvent.rename_query_old_label").to_string()),
                                )
                                .child(div().flex_1().child(old_name.clone())),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(80.))
                                        .child(t!("DbTreeEvent.rename_query_new_label").to_string()),
                                )
                                .child(div().flex_1().child(Input::new(&input))),
                        ),
                )
                .on_ok(move |_, _, cx| {
                    let new_name = input.read(cx).text().to_string().trim().to_string();
                    if new_name.is_empty() || new_name == old_name {
                        return false;
                    }

                    let Some(parent_dir) = old_path.parent() else {
                        return false;
                    };
                    let new_path = parent_dir.join(format!("{}.sql", new_name));

                    let db_tree = db_tree.clone();
                    let parent_id = parent_id.clone();
                    let old_path = old_path.clone();


                    let task = Tokio::spawn_result(cx, async move {
                        tokio::fs::rename(old_path, new_path).await?;
                        Ok(())
                    });

                    cx.spawn(async move |cx| {
                        let result = task.await;
                        match result {
                            Ok(_) => {
                                _ = cx.update_window(window_id, |_entity, window, cx| {
                                    if let Some(parent_id) = parent_id {
                                        db_tree.update(cx, |db, cx| {
                                            db.refresh_tree(parent_id, cx);
                                        });
                                    }
                                    window.close_dialog(cx);
                                    Self::show_success_async(
                                        cx,
                                        t!("DbTreeEvent.rename_query_success").to_string(),
                                    );
                                });
                            }
                            Err(e) => {
                                error!("Failed to rename query file: {}", e);
                                _ = cx.update_window(window_id, |_entity, window, cx| {
                                    window.close_dialog(cx);
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.rename_query_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    false
                })
        });
    }

    /// 处理删除查询事件
    fn handle_delete_query(
        node: DbNode,
        tree_view: Entity<DbTreeView>,
        objects_panel: Option<Entity<DatabaseObjectsPanel>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(path_str) = node.metadata.get("file_path") else {
            return;
        };

        let file_path = std::path::PathBuf::from(path_str);
        let query_name = node.name.clone();
        let parent_id = node.parent_context.clone();
        let global_state = cx.global::<GlobalDbState>().clone();
        let window_id = cx.active_window().expect("No active window");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let tree = tree_view.clone();
            let parent_id = parent_id.clone();
            let panel = objects_panel.clone();
            let state = global_state.clone();
            let file_path = file_path.clone();

            dialog
                .overlay(false)
                .title(t!("Common.confirm_delete").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!(
                                "DbTreeEvent.confirm_delete_query_message",
                                name = query_name
                            )
                            .to_string(),
                        )
                        .child(t!("Common.irreversible").to_string()),
                )
                .on_ok(move |_, _, cx| {
                    let tree = tree.clone();
                    let parent_id = parent_id.clone();
                    let panel = panel.clone();
                    let state = state.clone();
                    let file_path = file_path.clone();
                    let task = Tokio::spawn_result(cx, async move {
                        tokio::fs::remove_file(file_path).await?;
                        Ok(())
                    });

                    cx.spawn(async move |cx| {
                        let result = task.await;

                        match result {
                            Ok(_) => {
                                _ = cx.update_window(window_id, |_entity, window, cx| {
                                    if let Some(parent_id) = parent_id {
                                        tree.update(cx, |tree, cx| {
                                            tree.refresh_tree(parent_id, cx);
                                        });
                                    }
                                    if let Some(panel) = panel {
                                        panel.update(cx, |panel, cx| {
                                            panel.refresh(state, cx);
                                        });
                                    }
                                    window.close_dialog(cx);
                                    Self::show_success_async(
                                        cx,
                                        t!("DbTreeEvent.delete_query_success").to_string(),
                                    );
                                });
                            }
                            Err(e) => {
                                error!("Failed to delete query file: {}", e);
                                _ = cx.update_window(window_id, |_entity, window, cx| {
                                    window.close_dialog(cx);
                                    Self::show_error_async(
                                        cx,
                                        t!("DbTreeEvent.delete_query_failed", error = e)
                                            .to_string(),
                                    );
                                });
                            }
                        }
                    })
                    .detach();
                    false
                })
        });
    }

    /// 处理运行SQL文件事件
    fn handle_run_sql_file(
        node: DbNode,
        _global_state: GlobalDbState,
        _window: &mut Window,
        cx: &mut App,
    ) {
        use crate::import_export::sql_run_view::SqlRunView;

        let connection_id = node.connection_id.clone();
        let database = if node.node_type == DbNodeType::Database {
            Some(node.name.clone())
        } else {
            None
        };

        let schema = if node.node_type == DbNodeType::Schema {
            Some(node.name.clone())
        } else {
            None
        };

        open_popup_window(
            PopupWindowOptions::new(t!("ImportExport.run_sql_file").to_string())
                .size(800.0, 520.0),
            move |window, cx| SqlRunView::new(connection_id, database, schema, window, cx),
            cx,
        );
    }

    /// 处理转储SQL文件事件
    fn handle_dump_sql_file(
        node: DbNode,
        mode: SqlDumpMode,
        global_state: GlobalDbState,
        _window: &mut Window,
        cx: &mut App,
    ) {
        use crate::import_export::sql_dump_view::SqlDumpView;
        use std::path::PathBuf;

        let connection_id = node.connection_id.clone();
        let (database, table) = if node.node_type == DbNodeType::Table {
            let db = node.metadata.get("database").cloned().unwrap_or_default();
            (db, Some(node.name.clone()))
        } else {
            (node.name.clone(), None)
        };

        let window_id = cx.active_window();

        let future = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            multiple: false,
            directories: true,
            prompt: Some(t!("ImportExport.select_export_directory").into()),
        });

        cx.spawn(async move |cx: &mut AsyncApp| {
            let output_path: PathBuf = match future.await {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.first() {
                        path.clone()
                    } else {
                        return;
                    }
                }
                _ => return,
            };

            let config = global_state.get_config(&connection_id);

            match (config, window_id) {
                (Some(config), Some(window_id)) => {
                    let config_id = config.id.clone();
                    let server_info = config.server_info();
                    let database = database.clone();
                    let table = table.clone();

                    cx.update_window(window_id, |_entity, _window, cx| {
                        open_popup_window(
                            PopupWindowOptions::new(t!("ImportExport.dump_sql_file").to_string())
                                .size(800.0, 510.0),
                            move |window, cx| {
                                SqlDumpView::new(
                                    config_id,
                                    server_info,
                                    database,
                                    None, // schema
                                    table,
                                    output_path,
                                    mode,
                                    window,
                                    cx,
                                )
                            },
                            cx,
                        );
                    })
                    .ok();
                }

                _ => {
                    if let Some(window_id) = window_id {
                        cx.update_window(window_id, |_entity, window, cx| {
                            Self::show_error(
                                window,
                                t!(
                                    "DbTreeEvent.dump_sql_config_missing",
                                    id = connection_id
                                )
                                .to_string(),
                                cx,
                            );
                        })
                        .ok();
                    }
                }
            }
        })
        .detach();
    }
}
