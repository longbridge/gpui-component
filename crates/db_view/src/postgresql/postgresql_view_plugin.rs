use crate::common::db_connection_form::{DbConnectionForm, DbFormConfig};
use crate::common::{DatabaseEditorView, SchemaEditorView};
use crate::database_objects_tab::DatabaseObjectsEvent;
use crate::database_view_plugin::{
    ContextMenuItem, DatabaseViewPlugin, TableDesignerCapabilities, ToolbarButton,
};
use crate::db_tree_view::{DbTreeViewEvent, SqlDumpMode};
use crate::postgresql::database_form::PostgreSqlDatabaseForm;
use crate::postgresql::schema_form::PostgreSqlSchemaForm;
use db::DbNodeType;
use gpui::{App, AppContext, Entity, Window};
use gpui_component::IconName;
use one_core::storage::DatabaseType;

pub struct PostgreSqlDatabaseViewPlugin;

impl PostgreSqlDatabaseViewPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl DatabaseViewPlugin for PostgreSqlDatabaseViewPlugin {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::PostgreSQL
    }

    fn create_connection_form(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<DbConnectionForm> {
        cx.new(|cx| DbConnectionForm::new(DbFormConfig::postgres(), window, cx))
    }

    fn create_database_editor_view(
        &self,
        _connection_id: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<DatabaseEditorView> {
        cx.new(|cx| {
            let form = cx.new(|cx| PostgreSqlDatabaseForm::new(window, cx));
            DatabaseEditorView::new(form, DatabaseType::PostgreSQL, false, window, cx)
        })
    }

    fn create_database_editor_view_for_edit(
        &self,
        _connection_id: String,
        database_name: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<DatabaseEditorView> {
        cx.new(|cx| {
            let form =
                cx.new(|cx| PostgreSqlDatabaseForm::new_for_edit(&database_name, window, cx));
            DatabaseEditorView::new(form, DatabaseType::PostgreSQL, true, window, cx)
        })
    }

    fn create_schema_editor_view(
        &self,
        _connection_id: String,
        _database_name: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Entity<SchemaEditorView>> {
        Some(cx.new(|cx| {
            let form = cx.new(|cx| PostgreSqlSchemaForm::new(window, cx));
            SchemaEditorView::new(form, DatabaseType::PostgreSQL, window, cx)
        }))
    }

    fn get_table_designer_capabilities(&self) -> TableDesignerCapabilities {
        TableDesignerCapabilities {
            supports_engine: false,
            supports_charset: true,
            supports_collation: true,
            supports_auto_increment: false,
            supports_tablespace: true,
        }
    }

    fn get_engines(&self) -> Vec<String> {
        vec![]
    }

    fn build_context_menu(&self, node_id: &str, node_type: DbNodeType) -> Vec<ContextMenuItem> {
        match node_type {
            DbNodeType::Connection => {
                vec![
                    ContextMenuItem::item(
                        "运行SQL文件",
                        DbTreeViewEvent::RunSqlFile {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "关闭连接",
                        DbTreeViewEvent::CloseConnection {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "删除连接",
                        DbTreeViewEvent::DeleteConnection {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "新建数据库",
                        DbTreeViewEvent::CreateDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                ]
            }
            DbNodeType::Database => {
                vec![
                    ContextMenuItem::item(
                        "新建查询",
                        DbTreeViewEvent::CreateNewQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "运行SQL文件",
                        DbTreeViewEvent::RunSqlFile {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::submenu(
                        "转储SQL文件",
                        vec![
                            ContextMenuItem::item(
                                "导出结构",
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                "导出数据",
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::DataOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                "导出结构和数据",
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureAndData,
                                },
                            ),
                        ],
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "编辑数据库",
                        DbTreeViewEvent::EditDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "新建模式",
                        DbTreeViewEvent::CreateSchema {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "关闭数据库",
                        DbTreeViewEvent::CloseDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "删除数据库",
                        DbTreeViewEvent::DeleteDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                ]
            }
            DbNodeType::Schema => {
                vec![
                    ContextMenuItem::item(
                        "新建查询",
                        DbTreeViewEvent::CreateNewQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "运行SQL文件",
                        DbTreeViewEvent::RunSqlFile {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "新建表",
                        DbTreeViewEvent::DesignTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "删除模式",
                        DbTreeViewEvent::DeleteSchema {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                ]
            }
            DbNodeType::Table => {
                vec![
                    ContextMenuItem::item(
                        "查看表数据",
                        DbTreeViewEvent::OpenTableData {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "设计表",
                        DbTreeViewEvent::DesignTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "重命名表",
                        DbTreeViewEvent::RenameTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "清空表",
                        DbTreeViewEvent::TruncateTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "删除表",
                        DbTreeViewEvent::DeleteTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::submenu(
                        "转储SQL文件",
                        vec![
                            ContextMenuItem::item(
                                "导出结构",
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                "导出数据",
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::DataOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                "导出结构和数据",
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureAndData,
                                },
                            ),
                        ],
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "导入数据",
                        DbTreeViewEvent::ImportData {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "导出表",
                        DbTreeViewEvent::ExportData {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                ]
            }
            DbNodeType::View => {
                vec![
                    ContextMenuItem::item(
                        "查看视图数据",
                        DbTreeViewEvent::OpenViewData {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "删除视图",
                        DbTreeViewEvent::DeleteView {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                ]
            }
            DbNodeType::TablesFolder => {
                vec![
                    ContextMenuItem::item(
                        "新建表",
                        DbTreeViewEvent::DesignTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                ]
            }
            DbNodeType::QueriesFolder => {
                vec![
                    ContextMenuItem::item(
                        "新建查询",
                        DbTreeViewEvent::CreateNewQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                ]
            }
            DbNodeType::NamedQuery => {
                vec![
                    ContextMenuItem::item(
                        "打开查询",
                        DbTreeViewEvent::OpenNamedQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        "重命名查询",
                        DbTreeViewEvent::RenameQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        "删除查询",
                        DbTreeViewEvent::DeleteQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                ]
            }
            _ => vec![],
        }
    }

    fn build_toolbar_buttons(
        &self,
        node_type: DbNodeType,
        data_node_type: DbNodeType,
    ) -> Vec<ToolbarButton> {
        match node_type {
            DbNodeType::Connection => {
                if data_node_type == DbNodeType::Connection {
                    vec![
                        ToolbarButton::selected_row(
                            "delete-connection",
                            IconName::Minus,
                            "删除连接",
                            |node| DatabaseObjectsEvent::DeleteConnection { node },
                        ),
                        ToolbarButton::selected_row(
                            "close-connection",
                            IconName::CircleX,
                            "关闭连接",
                            |node| DatabaseObjectsEvent::CloseConnection { node },
                        ),
                    ]
                } else if data_node_type == DbNodeType::Schema {
                    vec![
                        ToolbarButton::current_node(
                            "create-schema",
                            IconName::Plus,
                            "新建模式",
                            |node| DatabaseObjectsEvent::CreateSchema { node },
                        ),
                        ToolbarButton::selected_row(
                            "delete-schema",
                            IconName::Minus,
                            "删除模式",
                            |node| DatabaseObjectsEvent::DeleteSchema { node },
                        ),
                    ]
                } else {
                    vec![
                        ToolbarButton::current_node(
                            "create-database",
                            IconName::Plus,
                            "新建数据库",
                            |node| DatabaseObjectsEvent::CreateDatabase { node },
                        ),
                        ToolbarButton::selected_row(
                            "edit-database",
                            IconName::Edit,
                            "编辑数据库",
                            |node| DatabaseObjectsEvent::EditDatabase { node },
                        ),
                        ToolbarButton::selected_row(
                            "delete-database",
                            IconName::Minus,
                            "删除数据库",
                            |node| DatabaseObjectsEvent::DeleteDatabase { node },
                        ),
                    ]
                }
            }
            DbNodeType::Database => {
                if data_node_type == DbNodeType::Schema {
                    vec![ToolbarButton::selected_row(
                        "delete-schema",
                        IconName::Minus,
                        "删除模式",
                        |node| DatabaseObjectsEvent::DeleteSchema { node },
                    )]
                } else {
                    vec![
                        ToolbarButton::current_node(
                            "create-table",
                            IconName::Plus,
                            "新建表",
                            |node| DatabaseObjectsEvent::DesignTable { node },
                        ),
                        ToolbarButton::selected_row(
                            "open-table",
                            IconName::Eye,
                            "查看表数据",
                            |node| DatabaseObjectsEvent::OpenTableData { node },
                        ),
                        ToolbarButton::selected_row(
                            "design-table",
                            IconName::Edit,
                            "设计表",
                            |node| DatabaseObjectsEvent::DesignTable { node },
                        ),
                        ToolbarButton::selected_row(
                            "drop-table",
                            IconName::Minus,
                            "删除表",
                            |node| DatabaseObjectsEvent::DeleteTable { node },
                        ),
                    ]
                }
            }
            DbNodeType::Schema | DbNodeType::TablesFolder => {
                vec![
                    ToolbarButton::current_node(
                        "create-table",
                        IconName::Plus,
                        "新建表",
                        |node| DatabaseObjectsEvent::DesignTable { node },
                    ),
                    ToolbarButton::selected_row(
                        "open-table",
                        IconName::Eye,
                        "查看表数据",
                        |node| DatabaseObjectsEvent::OpenTableData { node },
                    ),
                    ToolbarButton::selected_row(
                        "design-table",
                        IconName::Edit,
                        "设计表",
                        |node| DatabaseObjectsEvent::DesignTable { node },
                    ),
                    ToolbarButton::selected_row(
                        "drop-table",
                        IconName::Minus,
                        "删除表",
                        |node| DatabaseObjectsEvent::DeleteTable { node },
                    ),
                ]
            }
            DbNodeType::Table => {
                vec![
                    ToolbarButton::current_node(
                        "open-table",
                        IconName::Eye,
                        "查看表数据",
                        |node| DatabaseObjectsEvent::OpenTableData { node },
                    ),
                    ToolbarButton::current_node(
                        "design-table",
                        IconName::Edit,
                        "设计表",
                        |node| DatabaseObjectsEvent::DesignTable { node },
                    ),
                    ToolbarButton::current_node(
                        "drop-table",
                        IconName::Minus,
                        "删除表",
                        |node| DatabaseObjectsEvent::DeleteTable { node },
                    ),
                ]
            }
            DbNodeType::ViewsFolder => {
                vec![
                    ToolbarButton::selected_row(
                        "open-view",
                        IconName::Eye,
                        "查看视图数据",
                        |node| DatabaseObjectsEvent::OpenViewData { node },
                    ),
                    ToolbarButton::selected_row(
                        "drop-view",
                        IconName::Minus,
                        "删除视图",
                        |node| DatabaseObjectsEvent::DeleteView { node },
                    ),
                ]
            }
            DbNodeType::View => {
                vec![
                    ToolbarButton::current_node(
                        "open-view",
                        IconName::Eye,
                        "查看视图数据",
                        |node| DatabaseObjectsEvent::OpenViewData { node },
                    ),
                    ToolbarButton::current_node(
                        "drop-view",
                        IconName::Minus,
                        "删除视图",
                        |node| DatabaseObjectsEvent::DeleteView { node },
                    ),
                ]
            }
            DbNodeType::QueriesFolder | DbNodeType::NamedQuery => {
                vec![
                    ToolbarButton::current_node(
                        "create-query",
                        IconName::Plus,
                        "新建查询",
                        |node| DatabaseObjectsEvent::CreateNewQuery { node },
                    ),
                    ToolbarButton::selected_row(
                        "open-query",
                        IconName::Eye,
                        "打开查询",
                        |node| DatabaseObjectsEvent::OpenNamedQuery { node },
                    ),
                    ToolbarButton::selected_row(
                        "rename-query",
                        IconName::Edit,
                        "重命名查询",
                        |node| DatabaseObjectsEvent::RenameQuery { node },
                    ),
                    ToolbarButton::selected_row(
                        "delete-query",
                        IconName::Minus,
                        "删除查询",
                        |node| DatabaseObjectsEvent::DeleteQuery { node },
                    ),
                ]
            }
            _ => vec![],
        }
    }
}
