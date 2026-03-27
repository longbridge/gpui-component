use crate::common::db_connection_form::{DbConnectionForm, DbFormConfig};
use crate::common::{DatabaseEditorView, SchemaEditorView};
use crate::database_objects_tab::DatabaseObjectsEvent;
use crate::database_view_plugin::{
    ContextMenuItem, DatabaseViewPlugin, TableDesignerCapabilities, ToolbarButton,
};
use crate::db_tree_view::{DbTreeViewEvent, SqlDumpMode};
use crate::mssql::database_form::MsSqlDatabaseForm;
use crate::mssql::schema_form::MsSqlSchemaForm;
use db::DbNodeType;
use gpui::{App, AppContext, Entity, Window};
use gpui_component::IconName;
use one_core::storage::DatabaseType;
use rust_i18n::t;

pub struct MsSqlDatabaseViewPlugin;

impl MsSqlDatabaseViewPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl DatabaseViewPlugin for MsSqlDatabaseViewPlugin {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::MSSQL
    }

    fn create_connection_form(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<DbConnectionForm> {
        cx.new(|cx| DbConnectionForm::new(DbFormConfig::mssql(), window, cx))
    }

    fn create_database_editor_view(
        &self,
        _connection_id: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<DatabaseEditorView> {
        cx.new(|cx| {
            let form = cx.new(|cx| MsSqlDatabaseForm::new(window, cx));
            DatabaseEditorView::new(form, DatabaseType::MSSQL, false, window, cx)
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
            let form = cx.new(|cx| MsSqlDatabaseForm::new_for_edit(&database_name, window, cx));
            DatabaseEditorView::new(form, DatabaseType::MSSQL, true, window, cx)
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
            let form = cx.new(|cx| MsSqlSchemaForm::new(window, cx));
            SchemaEditorView::new(form, DatabaseType::MSSQL, window, cx)
        }))
    }

    fn get_table_designer_capabilities(&self) -> TableDesignerCapabilities {
        TableDesignerCapabilities {
            supports_engine: false,
            supports_charset: false,
            supports_collation: true,
            supports_auto_increment: false,
            supports_tablespace: false,
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
                        t!("ImportExport.run_sql_file").to_string(),
                        DbTreeViewEvent::RunSqlFile {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::always_enabled_item(
                        t!("Connection.close_connection").to_string(),
                        DbTreeViewEvent::CloseConnection {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::always_enabled_item(
                        t!("Connection.delete_connection").to_string(),
                        DbTreeViewEvent::DeleteConnection {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("Database.new_database").to_string(),
                        DbTreeViewEvent::CreateDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                ]
            }
            DbNodeType::Database => {
                vec![
                    ContextMenuItem::item(
                        t!("Table.new_table").to_string(),
                        DbTreeViewEvent::DesignTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Query.new_query").to_string(),
                        DbTreeViewEvent::CreateNewQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("ImportExport.run_sql_file").to_string(),
                        DbTreeViewEvent::RunSqlFile {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::submenu(
                        t!("ImportExport.dump_sql_file").to_string(),
                        vec![
                            ContextMenuItem::item(
                                t!("ImportExport.export_structure").to_string(),
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                t!("ImportExport.export_data").to_string(),
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::DataOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                t!("ImportExport.export_structure_and_data").to_string(),
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureAndData,
                                },
                            ),
                        ],
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("Database.edit_database").to_string(),
                        DbTreeViewEvent::EditDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Database.new_schema").to_string(),
                        DbTreeViewEvent::CreateSchema {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::always_enabled_item(
                        t!("Database.close_database").to_string(),
                        DbTreeViewEvent::CloseDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::always_enabled_item(
                        t!("Database.delete_database").to_string(),
                        DbTreeViewEvent::DeleteDatabase {
                            node_id: node_id.to_string(),
                        },
                    ),
                ]
            }
            DbNodeType::Schema => {
                vec![
                    ContextMenuItem::item(
                        t!("Query.new_query").to_string(),
                        DbTreeViewEvent::CreateNewQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("Table.new_table").to_string(),
                        DbTreeViewEvent::DesignTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Database.delete_schema").to_string(),
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
                        t!("Table.view_data").to_string(),
                        DbTreeViewEvent::OpenTableData {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Table.design_table").to_string(),
                        DbTreeViewEvent::DesignTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("Table.rename_table").to_string(),
                        DbTreeViewEvent::RenameTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Table.copy_table").to_string(),
                        DbTreeViewEvent::CopyTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Table.truncate_table").to_string(),
                        DbTreeViewEvent::TruncateTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Table.delete_table").to_string(),
                        DbTreeViewEvent::DeleteTable {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::submenu(
                        t!("ImportExport.dump_sql_file").to_string(),
                        vec![
                            ContextMenuItem::item(
                                t!("ImportExport.export_structure").to_string(),
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                t!("ImportExport.export_data").to_string(),
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::DataOnly,
                                },
                            ),
                            ContextMenuItem::item(
                                t!("ImportExport.export_structure_and_data").to_string(),
                                DbTreeViewEvent::DumpSqlFile {
                                    node_id: node_id.to_string(),
                                    mode: SqlDumpMode::StructureAndData,
                                },
                            ),
                        ],
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("ImportExport.import_data").to_string(),
                        DbTreeViewEvent::ImportData {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("ImportExport.export_table").to_string(),
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
                        t!("View.view_data").to_string(),
                        DbTreeViewEvent::OpenViewData {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("View.delete_view").to_string(),
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
                        t!("Table.new_table").to_string(),
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
                        t!("Query.new_query").to_string(),
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
                        t!("Query.open_query").to_string(),
                        DbTreeViewEvent::OpenNamedQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::separator(),
                    ContextMenuItem::item(
                        t!("Query.rename_query").to_string(),
                        DbTreeViewEvent::RenameQuery {
                            node_id: node_id.to_string(),
                        },
                    ),
                    ContextMenuItem::item(
                        t!("Query.delete_query").to_string(),
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
                            t!("Connection.delete_connection").to_string(),
                            |node| DatabaseObjectsEvent::DeleteConnection { node },
                        ),
                        ToolbarButton::selected_row(
                            "close-connection",
                            IconName::CircleX,
                            t!("Connection.close_connection").to_string(),
                            |node| DatabaseObjectsEvent::CloseConnection { node },
                        ),
                    ]
                } else if data_node_type == DbNodeType::Schema {
                    vec![ToolbarButton::selected_row(
                        "delete-schema",
                        IconName::Minus,
                        t!("Database.delete_schema").to_string(),
                        |node| DatabaseObjectsEvent::DeleteSchema { node },
                    )]
                } else {
                    vec![
                        ToolbarButton::current_node(
                            "create-database",
                            IconName::Plus,
                            t!("Database.new_database").to_string(),
                            |node| DatabaseObjectsEvent::CreateDatabase { node },
                        ),
                        ToolbarButton::selected_row(
                            "edit-database",
                            IconName::Edit,
                            t!("Database.edit_database").to_string(),
                            |node| DatabaseObjectsEvent::EditDatabase { node },
                        ),
                        ToolbarButton::selected_row(
                            "delete-database",
                            IconName::Minus,
                            t!("Database.delete_database").to_string(),
                            |node| DatabaseObjectsEvent::DeleteDatabase { node },
                        ),
                    ]
                }
            }
            DbNodeType::Database => {
                if data_node_type == DbNodeType::Schema {
                    vec![
                        ToolbarButton::current_node(
                            "create-schema",
                            IconName::Plus,
                            t!("Database.new_schema").to_string(),
                            |node| DatabaseObjectsEvent::CreateSchema { node },
                        ),
                        ToolbarButton::selected_row(
                            "delete-schema",
                            IconName::Minus,
                            t!("Database.delete_schema").to_string(),
                            |node| DatabaseObjectsEvent::DeleteSchema { node },
                        ),
                    ]
                } else {
                    vec![
                        ToolbarButton::current_node(
                            "create-table",
                            IconName::Plus,
                            t!("Table.new_table").to_string(),
                            |node| DatabaseObjectsEvent::DesignTable { node },
                        ),
                        ToolbarButton::selected_row(
                            "open-table",
                            IconName::Eye,
                            t!("Table.view_data").to_string(),
                            |node| DatabaseObjectsEvent::OpenTableData { node },
                        ),
                        ToolbarButton::selected_row(
                            "design-table",
                            IconName::Edit,
                            t!("Table.design_table").to_string(),
                            |node| DatabaseObjectsEvent::DesignTable { node },
                        ),
                        ToolbarButton::selected_row(
                            "drop-table",
                            IconName::Minus,
                            t!("Table.delete_table").to_string(),
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
                        t!("Table.new_table").to_string(),
                        |node| DatabaseObjectsEvent::DesignTable { node },
                    ),
                    ToolbarButton::selected_row(
                        "open-table",
                        IconName::Eye,
                        t!("Table.view_data").to_string(),
                        |node| DatabaseObjectsEvent::OpenTableData { node },
                    ),
                    ToolbarButton::selected_row(
                        "design-table",
                        IconName::Edit,
                        t!("Table.design_table").to_string(),
                        |node| DatabaseObjectsEvent::DesignTable { node },
                    ),
                    ToolbarButton::selected_row(
                        "drop-table",
                        IconName::Minus,
                        t!("Table.delete_table").to_string(),
                        |node| DatabaseObjectsEvent::DeleteTable { node },
                    ),
                ]
            }
            DbNodeType::Table => {
                vec![
                    ToolbarButton::current_node(
                        "open-table",
                        IconName::Eye,
                        t!("Table.view_data").to_string(),
                        |node| DatabaseObjectsEvent::OpenTableData { node },
                    ),
                    ToolbarButton::current_node(
                        "design-table",
                        IconName::Edit,
                        t!("Table.design_table").to_string(),
                        |node| DatabaseObjectsEvent::DesignTable { node },
                    ),
                    ToolbarButton::current_node(
                        "drop-table",
                        IconName::Minus,
                        t!("Table.delete_table").to_string(),
                        |node| DatabaseObjectsEvent::DeleteTable { node },
                    ),
                ]
            }
            DbNodeType::ViewsFolder => {
                vec![
                    ToolbarButton::selected_row(
                        "open-view",
                        IconName::Eye,
                        t!("View.view_data").to_string(),
                        |node| DatabaseObjectsEvent::OpenViewData { node },
                    ),
                    ToolbarButton::selected_row(
                        "drop-view",
                        IconName::Minus,
                        t!("View.delete_view").to_string(),
                        |node| DatabaseObjectsEvent::DeleteView { node },
                    ),
                ]
            }
            DbNodeType::View => {
                vec![
                    ToolbarButton::current_node(
                        "open-view",
                        IconName::Eye,
                        t!("View.view_data").to_string(),
                        |node| DatabaseObjectsEvent::OpenViewData { node },
                    ),
                    ToolbarButton::current_node(
                        "drop-view",
                        IconName::Minus,
                        t!("View.delete_view").to_string(),
                        |node| DatabaseObjectsEvent::DeleteView { node },
                    ),
                ]
            }
            DbNodeType::QueriesFolder | DbNodeType::NamedQuery => {
                vec![
                    ToolbarButton::current_node(
                        "create-query",
                        IconName::Plus,
                        t!("Query.new_query").to_string(),
                        |node| DatabaseObjectsEvent::CreateNewQuery { node },
                    ),
                    ToolbarButton::selected_row(
                        "open-query",
                        IconName::Eye,
                        t!("Query.open_query").to_string(),
                        |node| DatabaseObjectsEvent::OpenNamedQuery { node },
                    ),
                    ToolbarButton::selected_row(
                        "rename-query",
                        IconName::Edit,
                        t!("Query.rename_query").to_string(),
                        |node| DatabaseObjectsEvent::RenameQuery { node },
                    ),
                    ToolbarButton::selected_row(
                        "delete-query",
                        IconName::Minus,
                        t!("Query.delete_query").to_string(),
                        |node| DatabaseObjectsEvent::DeleteQuery { node },
                    ),
                ]
            }
            _ => vec![],
        }
    }
}
