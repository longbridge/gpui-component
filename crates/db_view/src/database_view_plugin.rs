use std::{collections::HashMap, sync::Arc};

use db::DbNodeType;
use gpui::{App, Entity, Global, Window};
use gpui_component::IconName;
use one_core::storage::DatabaseType;

use crate::clickhouse::clickhouse_view_plugin::ClickHouseDatabaseViewPlugin;
use crate::common::db_connection_form::DbConnectionForm;
use crate::common::{DatabaseEditorView, SchemaEditorView};
use crate::database_objects_tab::DatabaseObjectsEvent;
use crate::db_tree_view::DbTreeViewEvent;
use crate::duckdb::duckdb_view_plugin::DuckDbDatabaseViewPlugin;
use crate::mssql::mssql_view_plugin::MsSqlDatabaseViewPlugin;
use crate::mysql::mysql_view_plugin::MySqlDatabaseViewPlugin;
use crate::oracle::oracle_view_plugin::OracleDatabaseViewPlugin;
use crate::postgresql::postgresql_view_plugin::PostgreSqlDatabaseViewPlugin;
use crate::sqlite::sqlite_view_plugin::SqliteDatabaseViewPlugin;

/// 工具栏按钮类型
#[derive(Debug, Clone)]
pub enum ToolbarButtonType {
    /// 针对当前选中的节点（如刷新、新建）
    CurrentNode,
    /// 针对表格中选中的行（如删除、编辑）
    SelectedRow,
}

/// 工具栏按钮配置
#[derive(Clone)]
pub struct ToolbarButton {
    pub id: &'static str,
    pub icon: IconName,
    pub tooltip: String,
    pub button_type: ToolbarButtonType,
    pub event_fn: fn(db::DbNode) -> DatabaseObjectsEvent,
}

impl ToolbarButton {
    pub fn current_node(
        id: &'static str,
        icon: IconName,
        tooltip: impl Into<String>,
        event_fn: fn(db::DbNode) -> DatabaseObjectsEvent,
    ) -> Self {
        Self {
            id,
            icon,
            tooltip: tooltip.into(),
            button_type: ToolbarButtonType::CurrentNode,
            event_fn,
        }
    }

    pub fn selected_row(
        id: &'static str,
        icon: IconName,
        tooltip: impl Into<String>,
        event_fn: fn(db::DbNode) -> DatabaseObjectsEvent,
    ) -> Self {
        Self {
            id,
            icon,
            tooltip: tooltip.into(),
            button_type: ToolbarButtonType::SelectedRow,
            event_fn,
        }
    }
}

/// 上下文菜单项定义
#[derive(Debug, Clone)]
pub enum ContextMenuItem {
    /// 普通菜单项
    Item {
        label: String,
        event: ContextMenuEvent,
        /// 是否需要连接处于激活状态才可用
        requires_active: bool,
    },
    /// 分隔符
    Separator,
    /// 子菜单
    Submenu {
        label: String,
        items: Vec<ContextMenuItem>,
        /// 是否需要连接处于激活状态才可用
        requires_active: bool,
    },
}

/// 上下文菜单事件
#[derive(Debug, Clone)]
pub enum ContextMenuEvent {
    /// 直接触发的树视图事件
    TreeEvent(DbTreeViewEvent),
    /// 自定义处理器（暂不实现，预留扩展）
    Custom(String),
}

impl ContextMenuItem {
    /// 创建普通菜单项（默认需要连接激活）
    pub fn item(label: impl Into<String>, event: impl Into<DbTreeViewEvent>) -> Self {
        Self::Item {
            label: label.into(),
            event: ContextMenuEvent::TreeEvent(event.into()),
            requires_active: true,
        }
    }

    /// 创建不需要连接激活的菜单项（如删除连接）
    pub fn always_enabled_item(
        label: impl Into<String>,
        event: impl Into<DbTreeViewEvent>,
    ) -> Self {
        Self::Item {
            label: label.into(),
            event: ContextMenuEvent::TreeEvent(event.into()),
            requires_active: false,
        }
    }

    /// 创建分隔符
    pub fn separator() -> Self {
        Self::Separator
    }

    /// 创建子菜单（默认需要连接激活）
    pub fn submenu(label: impl Into<String>, items: Vec<ContextMenuItem>) -> Self {
        Self::Submenu {
            label: label.into(),
            items,
            requires_active: true,
        }
    }
}

/// 表设计器 UI 配置能力
#[derive(Clone, Debug)]
pub struct TableDesignerCapabilities {
    /// 是否支持存储引擎选择（MySQL: InnoDB/MyISAM）
    pub supports_engine: bool,
    /// 是否支持字符集选择
    pub supports_charset: bool,
    /// 是否支持排序规则选择
    pub supports_collation: bool,
    /// 是否支持自增起始值设置
    pub supports_auto_increment: bool,
    /// 是否支持表空间（PostgreSQL）
    pub supports_tablespace: bool,
}

impl Default for TableDesignerCapabilities {
    fn default() -> Self {
        Self {
            supports_engine: false,
            supports_charset: false,
            supports_collation: false,
            supports_auto_increment: false,
            supports_tablespace: false,
        }
    }
}

/// 列编辑器 UI 配置能力
#[derive(Clone, Debug)]
pub struct ColumnEditorCapabilities {
    /// 是否支持 unsigned（MySQL 特有）
    pub supports_unsigned: bool,
    /// 是否支持枚举/集合类型值编辑（MySQL ENUM/SET）
    pub supports_enum_values: bool,
    /// 是否在详情面板显示字符集
    pub show_charset_in_detail: bool,
    /// 是否在详情面板显示排序规则
    pub show_collation_in_detail: bool,
}

impl Default for ColumnEditorCapabilities {
    fn default() -> Self {
        Self {
            supports_unsigned: false,
            supports_enum_values: false,
            show_charset_in_detail: false,
            show_collation_in_detail: false,
        }
    }
}

/// 数据库视图插件接口
/// 每种数据库类型实现此 trait 来提供特定的 UI 组件
pub trait DatabaseViewPlugin: Send + Sync {
    fn database_type(&self) -> DatabaseType;

    /// 创建连接表单视图
    fn create_connection_form(&self, window: &mut Window, cx: &mut App)
    -> Entity<DbConnectionForm>;

    /// 创建数据库编辑器视图（用于新建数据库）
    fn create_database_editor_view(
        &self,
        connection_id: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<DatabaseEditorView>;

    /// 创建数据库编辑器视图（用于编辑现有数据库）
    fn create_database_editor_view_for_edit(
        &self,
        connection_id: String,
        database_name: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<DatabaseEditorView>;

    /// 创建模式编辑器视图（用于新建模式）
    fn create_schema_editor_view(
        &self,
        _connection_id: String,
        _database_name: String,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Entity<SchemaEditorView>> {
        None
    }

    /// 获取表设计器 UI 配置能力
    fn get_table_designer_capabilities(&self) -> TableDesignerCapabilities {
        TableDesignerCapabilities::default()
    }

    /// 获取存储引擎列表（用于表设计器下拉框）
    fn get_engines(&self) -> Vec<String> {
        vec![]
    }

    /// 获取列编辑器 UI 配置能力
    fn get_column_editor_capabilities(&self) -> ColumnEditorCapabilities {
        ColumnEditorCapabilities::default()
    }

    /// 为指定节点类型构建上下文菜单
    ///
    /// node_id: 节点 ID，用于构建事件
    /// node_type: 节点类型
    ///
    /// 返回菜单项列表。不同数据库可以为同一节点类型返回不同的菜单。
    fn build_context_menu(&self, node_id: &str, node_type: DbNodeType) -> Vec<ContextMenuItem>;

    /// 为指定节点类型构建工具栏按钮
    ///
    /// node_type: 当前选中的树节点类型
    /// data_node_type: 表格中显示的数据节点类型
    ///
    /// 返回工具栏按钮配置列表。不同数据库可以为同一节点类型返回不同的按钮。
    fn build_toolbar_buttons(
        &self,
        node_type: DbNodeType,
        data_node_type: DbNodeType,
    ) -> Vec<ToolbarButton>;
}

pub type DatabaseViewPluginRef = Arc<dyn DatabaseViewPlugin>;

/// 插件注册表：用 HashMap 实现 O(1) 查找
pub struct DatabaseViewPluginRegistry {
    plugins: HashMap<DatabaseType, DatabaseViewPluginRef>,
}

impl DatabaseViewPluginRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            plugins: HashMap::new(),
        };

        registry.register(MySqlDatabaseViewPlugin::new());
        registry.register(PostgreSqlDatabaseViewPlugin::new());
        registry.register(MsSqlDatabaseViewPlugin::new());
        registry.register(OracleDatabaseViewPlugin::new());
        registry.register(ClickHouseDatabaseViewPlugin::new());
        registry.register(SqliteDatabaseViewPlugin::new());
        registry.register(DuckDbDatabaseViewPlugin::new());

        registry
    }

    pub fn register<P>(&mut self, plugin: P)
    where
        P: DatabaseViewPlugin + 'static,
    {
        let plugin_ref = Arc::new(plugin);
        let db_type = plugin_ref.database_type();
        self.plugins.insert(db_type, plugin_ref);
    }

    pub fn get(&self, db_type: &DatabaseType) -> Option<DatabaseViewPluginRef> {
        self.plugins.get(db_type).cloned()
    }

    pub fn all(&self) -> impl Iterator<Item = DatabaseViewPluginRef> + '_ {
        self.plugins.values().cloned()
    }
}

impl Default for DatabaseViewPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for DatabaseViewPluginRegistry {}
