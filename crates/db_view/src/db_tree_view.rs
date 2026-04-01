// 1. 标准库导入
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

// 2. 外部 crate 导入（按字母顺序）
use gpui::{
    AnyElement, App, AppContext, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ListSizingBehavior, MouseButton, ParentElement, Render,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Subscription, Task,
    UniformListScrollHandle, Window, div, prelude::FluentBuilder, px, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable, Sizable, Size as ComponentSize,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    clipboard::Clipboard,
    h_flex,
    input::{Input, InputEvent, InputState},
    list::{List, ListDelegate, ListState},
    menu::{ContextMenuExt, PopupMenuItem},
    popover::Popover,
    scroll::Scrollbar,
    spinner::Spinner,
    tooltip::Tooltip,
    v_flex,
};
use rust_i18n::t;
use tracing::log::{error, info, trace, warn};

// 3. 当前 crate 导入（按模块分组）
use crate::database_view_plugin::DatabaseViewPluginRegistry;
use db::{DbNode, DbNodeType, GlobalDbState};
use gpui_component::label::Label;
use gpui_component::menu::PopupMenu;
use one_core::storage::DatabaseType;
use one_core::utils::debouncer::Debouncer;
use one_core::{
    connection_notifier::{ConnectionDataEvent, GlobalConnectionNotifier, get_notifier},
    gpui_tokio::Tokio,
    storage::{ActiveConnections, GlobalStorageState, StoredConnection},
};

// ============================================================================
// SQL 导出模式
// ============================================================================

/// SQL 导出模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDumpMode {
    /// 仅导出结构
    StructureOnly,
    /// 仅导出数据
    DataOnly,
    /// 导出结构和数据
    StructureAndData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefreshMetadataScope {
    None,
    Connection,
    Database(String),
}

fn resolve_refresh_metadata_scope(node: &DbNode) -> RefreshMetadataScope {
    match node.node_type {
        DbNodeType::Connection => RefreshMetadataScope::Connection,
        _ => node
            .get_database_name()
            .map(RefreshMetadataScope::Database)
            .unwrap_or(RefreshMetadataScope::None),
    }
}

fn sync_selected_databases_for_connection(
    selected_databases: &mut HashMap<String, Option<HashSet<String>>>,
    connection: &StoredConnection,
) {
    let connection_id = connection.id.unwrap_or(0).to_string();
    let selected = connection
        .get_selected_databases()
        .map(|selected_dbs| selected_dbs.into_iter().collect());
    selected_databases.insert(connection_id, selected);
}

// ============================================================================
// FlatDbEntry - 扁平化的树条目（用于 uniform_list 渲染）
// ============================================================================

#[derive(Clone)]
struct FlatDbEntry {
    node_id: String,
    depth: usize,
}

// ============================================================================
// DatabaseListItem - 数据库筛选列表项
// ============================================================================

#[derive(IntoElement)]
pub struct DatabaseListItem {
    db_id: String,
    db_name: String,
    is_selected: bool,
    selected: bool,
    view: Entity<DbTreeView>,
    connection_id: String,
}

impl DatabaseListItem {
    pub fn new(
        db_id: String,
        db_name: String,
        is_selected: bool,
        selected: bool,
        view: Entity<DbTreeView>,
        connection_id: String,
    ) -> Self {
        Self {
            db_id,
            db_name,
            is_selected,
            selected,
            view,
            connection_id,
        }
    }
}

impl Selectable for DatabaseListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for DatabaseListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view_item = self.view.clone();
        let conn_item = self.connection_id.clone();
        let db_name_item = self.db_name.clone();
        let db_name_display = self.db_name.clone();
        let is_selected = self.is_selected;

        h_flex()
            .id(SharedString::from(format!("db-item-{}", self.db_id)))
            .w_full()
            .px_3()
            .py_2()
            .gap_2()
            .items_center()
            .cursor_pointer()
            .rounded(px(4.0))
            .when(self.selected, |el| el.bg(cx.theme().list_active))
            .when(!self.selected, |el| {
                el.hover(|style| style.bg(cx.theme().list_hover))
            })
            .on_click(move |_, _, cx| {
                view_item.update(cx, |this, cx| {
                    this.toggle_database_selection(&conn_item, &db_name_item, cx);
                });
            })
            .child(
                Checkbox::new(SharedString::from(format!("db-check-{}", self.db_id)))
                    .checked(is_selected),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(db_name_display),
            )
    }
}

// ============================================================================
// DatabaseListDelegate - 数据库筛选列表代理
// ============================================================================

pub struct DatabaseListDelegate {
    view: Entity<DbTreeView>,
    connection_id: String,
    databases: Vec<(String, String)>,
    filtered_databases: Vec<(String, String)>,
    selected_index: Option<IndexPath>,
}

impl DatabaseListDelegate {
    pub fn new(
        view: Entity<DbTreeView>,
        connection_id: String,
        databases: Vec<(String, String)>,
    ) -> Self {
        let filtered_databases = databases.clone();
        Self {
            view,
            connection_id,
            databases,
            filtered_databases,
            selected_index: None,
        }
    }
}

impl ListDelegate for DatabaseListDelegate {
    type Item = DatabaseListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        if query.is_empty() {
            self.filtered_databases = self.databases.clone();
        } else {
            let query_lower = query.to_lowercase();
            self.filtered_databases = self
                .databases
                .iter()
                .filter(|(_, name)| name.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();
        }
        cx.notify();
        Task::ready(())
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_databases.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let (db_id, db_name) = self.filtered_databases.get(ix.row)?.clone();
        let is_selected = self
            .view
            .read(cx)
            .is_database_selected(&self.connection_id, &db_name);
        let selected = Some(ix) == self.selected_index;

        Some(DatabaseListItem::new(
            db_id,
            db_name,
            is_selected,
            selected,
            self.view.clone(),
            self.connection_id.clone(),
        ))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }
}

// ============================================================================
// DbTreeView Events
// ============================================================================

/// 数据库树视图事件
#[derive(Debug, Clone)]
pub enum DbTreeViewEvent {
    /// 打开表数据标签页
    OpenTableData { node_id: String },
    /// 打开视图数据标签页
    OpenViewData { node_id: String },
    /// 设计表（新建或编辑）
    DesignTable { node_id: String },
    /// 为指定数据库创建新查询
    CreateNewQuery { node_id: String },
    /// 打开命名查询
    OpenNamedQuery { node_id: String },
    /// 重命名查询
    RenameQuery { node_id: String },
    /// 删除查询
    DeleteQuery { node_id: String },
    /// 节点被选中（用于更新 objects panel）
    NodeSelected { node_id: String },
    /// 导入数据
    ImportData { node_id: String },
    /// 导出数据
    ExportData { node_id: String },
    /// 关闭连接
    CloseConnection { node_id: String },
    /// 删除连接
    DeleteConnection { node_id: String },
    /// 新建数据库
    CreateDatabase { node_id: String },
    /// 编辑数据库
    EditDatabase { node_id: String },
    /// 关闭数据库
    CloseDatabase { node_id: String },
    /// 删除数据库
    DeleteDatabase { node_id: String },
    /// 新建模式(Schema)
    CreateSchema { node_id: String },
    /// 删除模式(Schema)
    DeleteSchema { node_id: String },
    /// 删除表
    DeleteTable { node_id: String },
    /// 重命名表
    RenameTable { node_id: String },
    /// 复制表
    CopyTable { node_id: String },
    /// 清空表
    TruncateTable { node_id: String },
    /// 删除视图
    DeleteView { node_id: String },
    /// 运行SQL文件
    RunSqlFile { node_id: String },
    /// 转储SQL文件（导出结构和/或数据）
    DumpSqlFile { node_id: String, mode: SqlDumpMode },
}

/// 根据节点类型获取图标（公共函数，可被其他模块复用）
pub fn get_icon_for_node_type(node_type: &DbNodeType, _theme: &gpui_component::Theme) -> Icon {
    match node_type {
        DbNodeType::Connection => IconName::MySQLLineColor
            .color()
            .with_size(ComponentSize::Large),
        DbNodeType::Schema => IconName::Schema.color(),
        DbNodeType::Database => Icon::from(IconName::Database)
            .color()
            .with_size(ComponentSize::Size(px(20.))),
        DbNodeType::Table => Icon::from(IconName::Table).color(),
        DbNodeType::View => Icon::from(IconName::View).color(),
        DbNodeType::Function => Icon::from(IconName::Function).color(),
        DbNodeType::Procedure => Icon::from(IconName::Procedure).color(),
        DbNodeType::Column => Icon::from(IconName::Column).color(),
        DbNodeType::Index => Icon::from(IconName::Index).color(),
        DbNodeType::Trigger => Icon::from(IconName::Trigger).color(),
        DbNodeType::Sequence => Icon::from(IconName::Sequence).color(),
        DbNodeType::NamedQuery => Icon::from(IconName::Query).color(),
        _ => IconName::File.color(),
    }
}

// ============================================================================
// DbTreeView - 数据库连接树视图（支持懒加载）
// ============================================================================

pub struct DbTreeView {
    focus_handle: FocusHandle,
    // 扁平化的树条目
    flat_entries: Vec<FlatDbEntry>,
    scroll_handle: UniformListScrollHandle,
    selected_ix: Option<usize>,
    selected_node_id: Option<String>,
    // 存储 DbNode 映射 (ID -> DbNode)，用于懒加载
    db_nodes: HashMap<String, DbNode>,
    // 已经懒加载过子节点的集合
    loaded_children: HashSet<String>,
    // 正在加载的节点集合（用于显示加载状态）
    loading_nodes: HashSet<String>,
    // 加载失败的节点集合（用于显示错误状态）
    error_nodes: HashMap<String, String>,
    // 已展开的节点（用于在重建树时保持展开状态）
    expanded_nodes: HashSet<String>,
    // 当前连接名称或者工作区名称
    connection_name: Option<String>,
    // 工作区ID
    workspace_id: Option<i64>,
    // 搜索输入框状态
    search_input: Entity<InputState>,
    // 搜索关键字
    search_query: String,
    // 搜索防抖序列号
    search_seq: u64,
    search_debouncer: Arc<Debouncer>,
    // 数据库筛选：连接ID -> 选中的数据库ID集合（None 表示全选）
    selected_databases: HashMap<String, Option<HashSet<String>>>,
    // 数据库筛选搜索词：连接ID -> 搜索词
    db_filter_search: HashMap<String, String>,
    // 数据库筛选列表状态：连接ID -> ListState
    db_filter_list_states: HashMap<String, Entity<ListState<DatabaseListDelegate>>>,
    // 当前 Tab 跟踪的连接 ID 列表
    tracked_connection_ids: Vec<i64>,

    _subscriptions: Vec<Subscription>,
}

impl DbTreeView {
    /// 将 ContextMenuItem 渲染到 PopupMenu
    fn render_context_menu_items(
        menu: PopupMenu,
        items: Vec<crate::database_view_plugin::ContextMenuItem>,
        is_active: bool,
        view: &Entity<Self>,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        use crate::database_view_plugin::{ContextMenuEvent, ContextMenuItem};

        let mut result_menu = menu;

        for item in items {
            match item {
                ContextMenuItem::Item {
                    label,
                    event,
                    requires_active,
                } => {
                    let disabled = requires_active && !is_active;
                    match event {
                        ContextMenuEvent::TreeEvent(tree_event) => {
                            let view_clone = view.clone();
                            let item = PopupMenuItem::new(label).disabled(disabled).on_click(
                                window.listener_for(&view_clone, move |_this, _, _, cx| {
                                    cx.emit(tree_event.clone());
                                }),
                            );
                            result_menu = result_menu.item(item);
                        }
                        ContextMenuEvent::Custom(_) => {
                            // 暂不支持，预留扩展
                        }
                    }
                }
                ContextMenuItem::Separator => {
                    result_menu = result_menu.separator();
                }
                ContextMenuItem::Submenu {
                    label,
                    items: sub_items,
                    requires_active,
                } => {
                    let disabled = requires_active && !is_active;
                    let view_submenu = view.clone();
                    let submenu_entity =
                        PopupMenu::build(window, cx, move |submenu, window, cx| {
                            Self::render_context_menu_items(
                                submenu,
                                sub_items.clone(),
                                is_active,
                                &view_submenu,
                                window,
                                cx,
                            )
                        });
                    result_menu = result_menu
                        .item(PopupMenuItem::submenu(label, submenu_entity).disabled(disabled));
                }
            }
        }

        result_menu
    }

    pub fn new(
        connections: &Vec<StoredConnection>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let mut db_nodes = HashMap::new();
        let mut workspace_id = None;
        let mut unselected_databases_map = HashMap::new();
        let mut tracked_connection_ids = Vec::new();

        if connections.is_empty() {
            let node = DbNode::new(
                "root",
                t!("Connection.no_database_connected").to_string(),
                DbNodeType::Connection,
                "".to_string(),
                DatabaseType::MySQL,
            );
            db_nodes.insert("root".to_string(), node.clone());
        } else {
            for conn in connections {
                workspace_id = conn.workspace_id;
                let id = conn.id.unwrap_or(0).to_string();

                if let Some(conn_id) = conn.id {
                    tracked_connection_ids.push(conn_id);
                }

                let conn_config = match conn.to_db_connection() {
                    Ok(config) => config,
                    Err(e) => {
                        tracing::error!(
                            "{}",
                            t!(
                                "DbTreeView.parse_connection_config_failed",
                                id = id,
                                error = e
                            )
                            .to_string()
                        );
                        continue;
                    }
                };

                sync_selected_databases_for_connection(&mut unselected_databases_map, conn);

                let node = DbNode::new(
                    id.clone(),
                    conn_config.name.to_string(),
                    DbNodeType::Connection,
                    id.clone(),
                    conn_config.database_type,
                );
                db_nodes.insert(id, node.clone());
            }
        }

        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Common.search").to_string())
                .clean_on_escape()
        });
        let search_debouncer = Arc::new(Debouncer::new(Duration::from_millis(250)));

        let search_sub = cx.subscribe_in(
            &search_input,
            window,
            |this: &mut Self,
             input: &Entity<InputState>,
             event: &InputEvent,
             _window,
             cx: &mut Context<Self>| {
                if let InputEvent::Change = event {
                    let query = input.read(cx).text().to_string();

                    this.search_seq += 1;
                    let current_seq = this.search_seq;
                    let debouncer = Arc::clone(&this.search_debouncer);
                    let query_for_task = query.clone();

                    cx.spawn(async move |view, cx| {
                        if debouncer.debounce(cx).await {
                            _ = view.update(cx, |this, cx| {
                                if this.search_seq == current_seq {
                                    this.search_query = query_for_task.clone();
                                    this.rebuild_flat_entries(cx);
                                }
                            });
                        }
                    })
                    .detach();
                }
            },
        );

        // 订阅全局连接数据变更事件
        let mut subscriptions = vec![search_sub];
        if let Some(notifier) = cx.try_global::<GlobalConnectionNotifier>().cloned() {
            let notifier_sub = cx.subscribe(&notifier.0, Self::handle_connection_data_event);
            subscriptions.push(notifier_sub);
        }

        // 构建初始的扁平化条目
        let flat_entries = Self::build_initial_flat_entries(&db_nodes);

        Self {
            focus_handle,
            flat_entries,
            scroll_handle: UniformListScrollHandle::new(),
            selected_ix: None,
            selected_node_id: None,
            db_nodes,
            loaded_children: HashSet::new(),
            loading_nodes: HashSet::new(),
            error_nodes: HashMap::new(),
            expanded_nodes: HashSet::new(),
            connection_name: None,
            workspace_id,
            search_input,
            search_query: String::new(),
            search_seq: 0,
            search_debouncer,
            selected_databases: unselected_databases_map,
            db_filter_search: HashMap::new(),
            db_filter_list_states: HashMap::new(),
            tracked_connection_ids,
            _subscriptions: subscriptions,
        }
    }

    /// 构建初始的扁平化条目
    fn build_initial_flat_entries(db_nodes: &HashMap<String, DbNode>) -> Vec<FlatDbEntry> {
        let mut root_nodes: Vec<&DbNode> = db_nodes
            .values()
            .filter(|n| n.parent_context.is_none())
            .collect();
        root_nodes.sort();

        root_nodes
            .iter()
            .map(|n| FlatDbEntry {
                node_id: n.id.clone(),
                depth: 0,
            })
            .collect()
    }

    /// 处理全局连接数据变更事件
    fn handle_connection_data_event(
        &mut self,
        _notifier: Entity<one_core::connection_notifier::ConnectionDataNotifier>,
        event: &ConnectionDataEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            ConnectionDataEvent::ConnectionDeleted { connection_id } => {
                if self.tracked_connection_ids.contains(connection_id) {
                    self.remove_connection(&connection_id.to_string(), cx);
                }
            }
            ConnectionDataEvent::ConnectionUpdated { connection } => {
                if let Some(conn_id) = connection.id {
                    if self.tracked_connection_ids.contains(&conn_id) {
                        self.update_connection_info(connection, cx);
                    }
                }
            }
            ConnectionDataEvent::ConnectionCreated { connection } => {
                if connection.workspace_id == self.workspace_id && self.workspace_id.is_some() {
                    self.add_connection(connection, cx);
                }
            }
            ConnectionDataEvent::WorkspaceDeleted { workspace_id } => {
                if self.workspace_id == Some(*workspace_id) {
                    info!(
                        "Workspace {} deleted, tree view may need to be closed",
                        workspace_id
                    );
                }
            }
            ConnectionDataEvent::WorkspaceUpdated { workspace_id } => {
                if self.workspace_id == Some(*workspace_id) {
                    info!("Workspace {} updated", workspace_id);
                }
            }
            ConnectionDataEvent::WorkspaceCreated { .. } => {}
            ConnectionDataEvent::SchemaChanged {
                connection_id,
                database,
                schema: _,
            } => {
                // DDL 变更后自动刷新受影响连接的树节点
                if self
                    .tracked_connection_ids
                    .contains(&connection_id.parse::<i64>().unwrap_or(-1))
                {
                    // 找到该连接下对应数据库节点并刷新
                    let target_node_id = self.find_database_node(connection_id, database);
                    if let Some(node_id) = target_node_id {
                        info!(
                            "Auto-refreshing tree after DDL: connection={}, database={}",
                            connection_id, database
                        );
                        self.refresh_tree(node_id, cx);
                    } else {
                        // 找不到具体数据库节点，刷新整个连接
                        info!(
                            "Auto-refreshing connection tree after DDL: connection={}",
                            connection_id
                        );
                        self.refresh_tree(connection_id.clone(), cx);
                    }
                }
            }
        }
    }

    /// 移除连接节点
    pub fn remove_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        info!("Removing connection from DbTreeView: {}", connection_id);

        self.clear_node_descendants(connection_id);
        self.db_nodes.remove(connection_id);
        self.clear_node_all_state(connection_id);
        self.selected_databases.remove(connection_id);
        self.db_filter_list_states.remove(connection_id);

        if let Ok(conn_id) = connection_id.parse::<i64>() {
            self.tracked_connection_ids.retain(|&id| id != conn_id);
        }

        if self.selected_node_id.as_ref() == Some(&connection_id.to_string()) {
            self.selected_node_id = None;
            self.selected_ix = None;
        }

        self.rebuild_tree(cx);

        let conn_id = connection_id.to_string();
        let mut global_db_state = cx.global_mut::<GlobalDbState>().clone();
        cx.spawn(async move |_, cx| {
            let result = global_db_state
                .unregister_connection(cx, conn_id.clone())
                .await;
            match result {
                Ok(_) => {
                    info!("Unregistered connection {}", conn_id);
                }
                Err(err) => {
                    error!("Failed to unregister connection {}: {}", conn_id, err)
                }
            };
        })
        .detach();
    }

    /// 更新连接信息（名称等）
    fn update_connection_info(&mut self, connection: &StoredConnection, cx: &mut Context<Self>) {
        if let Ok(config) = connection.to_db_connection() {
            let id = connection.id.unwrap_or(0).to_string();
            info!("Updating connection info: {}", id);
            sync_selected_databases_for_connection(&mut self.selected_databases, connection);

            if let Some(node) = self.db_nodes.get_mut(&id) {
                node.name = config.name.to_string();
                let mut global_db_state = cx.global_mut::<GlobalDbState>().clone();
                let conn_id = id.clone();
                if let Some(exist_config) = global_db_state.get_config(&id) {
                    if exist_config.is_change(&config) {
                        cx.spawn(async move |_, cx| {
                            let result = global_db_state.update_connection(cx, config).await;
                            match result {
                                Ok(_) => {
                                    info!("Updating connection {}", conn_id);
                                }
                                Err(err) => {
                                    error!("Failed to Updating connection {}: {}", conn_id, err)
                                }
                            };
                        })
                        .detach();
                    }
                }
            }
            self.rebuild_tree(cx);
        }
    }

    /// 添加新连接节点
    pub fn add_connection(&mut self, connection: &StoredConnection, cx: &mut Context<Self>) {
        if let Ok(config) = connection.to_db_connection() {
            let id = connection.id.unwrap_or(0).to_string();

            // 检查是否已存在
            if self.db_nodes.contains_key(&id) {
                return;
            }

            // 添加到跟踪列表
            if let Some(conn_id) = connection.id {
                if !self.tracked_connection_ids.contains(&conn_id) {
                    self.tracked_connection_ids.push(conn_id);
                }
            }

            sync_selected_databases_for_connection(&mut self.selected_databases, connection);

            let node = DbNode::new(
                id.clone(),
                config.name.to_string(),
                DbNodeType::Connection,
                id.clone(),
                config.database_type,
            );
            let global_db_state = cx.global_mut::<GlobalDbState>();
            global_db_state.register_connection(config);
            self.db_nodes.insert(id, node);
            self.rebuild_tree(cx);
        }
    }

    /// 折叠所有节点
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.expanded_nodes.clear();
        self.rebuild_flat_entries(cx);
    }

    /// 设置连接名称
    pub fn set_connection_name(&mut self, name: String) {
        self.connection_name = Some(name);
    }

    /// 获取连接下的所有数据库或Schema节点（Oracle连接下是Schema）
    pub fn get_databases_for_connection(&self, connection_id: &str) -> Vec<(String, String)> {
        let mut databases = Vec::new();
        if let Some(conn_node) = self.db_nodes.get(connection_id) {
            for child in &conn_node.children {
                // Database 节点或者是 Connection 直接子节点的 Schema（Oracle）
                if child.node_type == DbNodeType::Database || child.node_type == DbNodeType::Schema
                {
                    databases.push((child.id.clone(), child.name.clone()));
                }
            }
        }
        databases
    }

    /// 检查节点是否应该被数据库筛选过滤
    /// 只有 Database 节点和 Connection 直接子节点的 Schema（Oracle）才需要筛选
    fn should_filter_by_database_selection(&self, node: &DbNode, connection_id: &str) -> bool {
        if node.node_type == DbNodeType::Database {
            return true;
        }
        if node.node_type == DbNodeType::Schema {
            // 只有当 Schema 是 Connection 的直接子节点时才筛选（Oracle 的情况）
            if let Some(parent_context) = &node.parent_context {
                return parent_context == connection_id;
            }
        }
        false
    }

    /// 获取选中的数据库数量
    pub fn get_selected_database_count(&self, connection_id: &str) -> (usize, usize) {
        let databases = self.get_databases_for_connection(connection_id);
        let total = databases.len();

        match self.selected_databases.get(connection_id) {
            None => (total, total),
            Some(None) => (total, total),
            Some(Some(selected)) => {
                let count = databases
                    .iter()
                    .filter(|(_, name)| selected.contains(name))
                    .count();
                (count, total)
            }
        }
    }

    /// 切换数据库选中状态
    pub fn toggle_database_selection(
        &mut self,
        connection_id: &str,
        database_name: &str,
        cx: &mut Context<Self>,
    ) {
        let databases = self.get_databases_for_connection(connection_id);
        let all_db_names: HashSet<String> =
            databases.iter().map(|(_, name)| name.clone()).collect();

        let selected = self
            .selected_databases
            .entry(connection_id.to_string())
            .or_insert(None);

        match selected {
            None => {
                let mut new_selected = all_db_names.clone();
                new_selected.remove(database_name);
                *selected = Some(new_selected);
            }
            Some(set) => {
                if set.contains(database_name) {
                    set.remove(database_name);
                } else {
                    set.insert(database_name.to_string());
                }
                if set.len() == all_db_names.len() {
                    *selected = None;
                }
            }
        }

        self.rebuild_tree(cx);
        self.save_database_filter(connection_id, cx);
    }

    /// 全选数据库
    pub fn select_all_databases(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        self.selected_databases
            .insert(connection_id.to_string(), None);
        self.rebuild_tree(cx);
        self.save_database_filter(connection_id, cx);
    }

    /// 清除筛选（取消全选）
    pub fn deselect_all_databases(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        self.selected_databases
            .insert(connection_id.to_string(), Some(HashSet::new()));
        self.rebuild_tree(cx);
        self.save_database_filter(connection_id, cx);
    }

    /// 将新数据库添加到已选择列表（用于新建数据库后自动选中）
    pub fn add_database_to_selection(
        &mut self,
        connection_id: &str,
        database_name: &str,
        cx: &mut Context<Self>,
    ) {
        if database_name.is_empty() {
            return;
        }

        match self.selected_databases.get_mut(connection_id) {
            None => {
                // 全选状态，不需要添加
            }
            Some(Some(set)) => {
                set.insert(database_name.to_string());
                self.save_database_filter(connection_id, cx);
            }
            Some(None) => {
                // 全选状态，不需要添加
            }
        }
    }

    /// 确保数据库节点存在于树中并展开
    pub fn ensure_database_node_expanded(
        &mut self,
        connection_id: &str,
        database_name: &str,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if database_name.is_empty() {
            return None;
        }

        // 确认连接存在
        if !self.db_nodes.contains_key(connection_id) {
            return None;
        }

        let mut node_id = self.find_database_node(connection_id, database_name);
        if node_id.is_none() {
            self.add_database_to_selection(connection_id, database_name, cx);
            self.add_database_node(connection_id, database_name, cx);
            node_id = self.find_database_node(connection_id, database_name);
        } else {
            // 确保筛选状态包含该数据库
            self.add_database_to_selection(connection_id, database_name, cx);
        }

        let Some(node_id) = node_id else {
            return None;
        };

        self.expanded_nodes.insert(connection_id.to_string());
        self.expanded_nodes.insert(node_id.clone());
        // 同步更新选中节点，确保 lazy_load_children 完成后能正确触发 NodeSelected 事件
        self.selected_node_id = Some(node_id.clone());

        self.lazy_load_children(connection_id.to_string(), cx);
        self.lazy_load_children(node_id.clone(), cx);
        self.rebuild_tree(cx);

        Some(node_id)
    }

    /// 保存数据库筛选状态到存储
    fn save_database_filter(&self, connection_id: &str, cx: &mut Context<Self>) {
        let selected_dbs: Option<Vec<String>> = match self.selected_databases.get(connection_id) {
            None => None,       // 全选
            Some(None) => None, // 全选
            Some(Some(selected_set)) => {
                // 保存已选中的数据库列表
                Some(selected_set.iter().cloned().collect())
            }
        };

        let Some(storage_state) = cx.try_global::<GlobalStorageState>() else {
            warn!(
                "GlobalStorageState 不存在，无法持久化连接 {} 的数据库筛选状态",
                connection_id
            );
            return;
        };
        let connection_id_str = connection_id.to_string();
        let storage = storage_state.storage.clone();

        cx.spawn(async move |_view, cx| {
            use one_core::gpui_tokio::Tokio;
            use one_core::storage::ConnectionRepository;
            use one_core::storage::traits::Repository;

            let conn_id: i64 = match connection_id_str.parse() {
                Ok(id) => id,
                Err(_) => return Ok::<(), anyhow::Error>(()),
            };

            let result = Tokio::spawn_result(cx, async move {
                if let Some(repo) = storage.get::<ConnectionRepository>() {
                    if let Ok(Some(mut conn)) = repo.get(conn_id) {
                        conn.set_selected_databases(selected_dbs);
                        repo.update(&conn)?;
                        return Ok(Some(conn));
                    }
                }
                Ok(None)
            })
            .await;

            if let Ok(Some(connection)) = result {
                let _ = cx.update(|cx| {
                    if let Some(notifier) = get_notifier(cx) {
                        notifier.update(cx, |_, cx| {
                            cx.emit(ConnectionDataEvent::ConnectionUpdated { connection });
                        });
                    }
                });
            }
            Ok(())
        })
        .detach();
    }

    /// 检查数据库是否被选中
    pub fn is_database_selected(&self, connection_id: &str, database_name: &str) -> bool {
        match self.selected_databases.get(connection_id) {
            None => true,
            Some(None) => true,
            Some(Some(set)) => set.contains(database_name),
        }
    }

    /// 检查是否全选
    pub fn is_all_selected(&self, connection_id: &str) -> bool {
        match self.selected_databases.get(connection_id) {
            None => true,
            Some(None) => true,
            Some(Some(set)) => {
                let databases = self.get_databases_for_connection(connection_id);
                let all_names: HashSet<&String> = databases.iter().map(|(_, name)| name).collect();
                set.len() == all_names.len() && set.iter().all(|name| all_names.contains(name))
            }
        }
    }

    /// 设置数据库筛选搜索词
    pub fn set_db_filter_search(
        &mut self,
        connection_id: &str,
        query: String,
        cx: &mut Context<Self>,
    ) {
        self.db_filter_search
            .insert(connection_id.to_string(), query);
        cx.notify();
    }

    /// 获取数据库筛选搜索词
    pub fn get_db_filter_search(&self, connection_id: &str) -> String {
        self.db_filter_search
            .get(connection_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取过滤后的数据库列表
    pub fn get_filtered_databases(&self, connection_id: &str) -> Vec<(String, String)> {
        let databases = self.get_databases_for_connection(connection_id);
        let search_query = self.get_db_filter_search(connection_id).to_lowercase();

        if search_query.is_empty() {
            databases
        } else {
            databases
                .into_iter()
                .filter(|(_, name)| name.to_lowercase().contains(&search_query))
                .collect()
        }
    }

    /// 刷新指定节点及其子节点
    ///
    /// 这个方法会：
    /// 1. 清除节点的子节点缓存
    /// 2. 递归清除所有后代节点
    /// 3. 重新加载子节点
    /// 4. 如果节点已展开，保持展开状态
    pub fn refresh_tree(&mut self, node_id: String, cx: &mut Context<Self>) {
        let refresh_node_id = self
            .db_nodes
            .get(&node_id)
            .and_then(|node| match node.node_type {
                DbNodeType::View | DbNodeType::Function => node.parent_context.clone(),
                _ => None,
            })
            .unwrap_or_else(|| node_id.clone());

        info!("Refreshing node in DbTreeView: {}", refresh_node_id);

        let should_reload_children = self.expanded_nodes.contains(&refresh_node_id);
        let refresh_node = self.db_nodes.get(&refresh_node_id).cloned();
        self.clear_node_descendants(&refresh_node_id);
        self.clear_node_loading_state(&refresh_node_id);
        self.reset_node_children(&refresh_node_id);
        self.rebuild_tree(cx);

        let Some(refresh_node) = refresh_node else {
            return;
        };

        let refresh_scope = resolve_refresh_metadata_scope(&refresh_node);
        let connection_id = refresh_node.connection_id.clone();
        let global_state = cx.global::<GlobalDbState>().clone();
        let cache_ctx = global_state
            .get_config(&connection_id)
            .map(|config| db::CacheContext::from_config(&config));
        let cache = cx.try_global::<db::GlobalNodeCache>().cloned();
        let refresh_node_id_for_task = refresh_node_id.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            if let Some(cache) = cache {
                let cache_ctx = cache_ctx.clone();
                let connection_id = connection_id.clone();
                let refresh_scope = refresh_scope.clone();
                let refresh_node_id = refresh_node_id_for_task.clone();

                if let Err(err) = Tokio::spawn(cx, async move {
                    if let Some(cache_ctx) = cache_ctx.as_ref() {
                        cache
                            .invalidate_node_recursive(cache_ctx, &refresh_node_id)
                            .await;
                    }

                    match refresh_scope {
                        RefreshMetadataScope::None => {}
                        RefreshMetadataScope::Connection => {
                            cache.invalidate_connection_metadata(&connection_id).await;
                        }
                        RefreshMetadataScope::Database(database) => {
                            cache.invalidate_database(&connection_id, &database).await;
                        }
                    }
                })
                .await
                {
                    error!("刷新数据库树缓存时 Tokio 任务失败: {}", err);
                }
            }

            _ = this.update(cx, |this, cx| {
                if should_reload_children {
                    this.lazy_load_children(refresh_node_id_for_task.clone(), cx);
                } else {
                    this.rebuild_tree(cx);
                }
            });
        })
        .detach();
    }

    /// 清理节点的加载/错误状态（不包含展开状态）
    fn clear_node_loading_state(&mut self, node_id: &str) {
        self.loaded_children.remove(node_id);
        self.loading_nodes.remove(node_id);
        self.error_nodes.remove(node_id);
    }

    /// 清理节点的所有状态（包含展开状态）
    fn clear_node_all_state(&mut self, node_id: &str) {
        self.clear_node_loading_state(node_id);
        self.expanded_nodes.remove(node_id);
    }

    /// 重置节点的子节点状态
    fn reset_node_children(&mut self, node_id: &str) {
        if let Some(node) = self.db_nodes.get_mut(node_id) {
            node.children.clear();
            node.children_loaded = false;
        }
    }

    /// 从父节点的子节点列表中移除指定节点
    fn remove_from_parent(&mut self, node_id: &str) {
        if let Some(node) = self.db_nodes.get(node_id).cloned() {
            if let Some(parent_context) = &node.parent_context {
                if let Some(parent_node) = self.db_nodes.get_mut(parent_context) {
                    parent_node.children.retain(|child| child.id != node_id);
                }
            }
        }
    }

    /// 完整移除一个节点（清理后代、从父节点移除、清理状态、从db_nodes移除）
    fn remove_node_complete(&mut self, node_id: &str) {
        self.clear_node_descendants(node_id);
        self.remove_from_parent(node_id);
        self.db_nodes.remove(node_id);
        self.clear_node_all_state(node_id);
    }

    /// 递归清除节点所有后代的展开状态
    ///
    /// 用于关闭连接/数据库时彻底清理展开状态，避免重新连接后自动展开旧节点。
    fn clear_expanded_descendants(&mut self, node_id: &str) {
        let child_ids: Vec<String> = if let Some(node) = self.db_nodes.get(node_id) {
            node.children.iter().map(|c| c.id.clone()).collect()
        } else {
            return;
        };

        for child_id in child_ids {
            self.clear_expanded_descendants(&child_id);
            self.expanded_nodes.remove(&child_id);
        }
    }

    /// 递归清除节点的所有后代
    ///
    /// 注意：此方法不会清除展开状态(expanded_nodes)，因为展开状态是用户的UI状态，
    /// 应该独立于节点数据。如果节点被删除，展开状态自然不会生效；
    /// 如果节点仍然存在（刷新后重新加载），展开状态应该被保留。
    fn clear_node_descendants(&mut self, node_id: &str) {
        let child_ids: Vec<String> = if let Some(node) = self.db_nodes.get(node_id) {
            node.children.iter().map(|c| c.id.clone()).collect()
        } else {
            return;
        };

        for child_id in child_ids {
            self.clear_node_descendants(&child_id);
            self.db_nodes.remove(&child_id);
            self.clear_node_loading_state(&child_id);
        }
    }

    /// 懒加载节点的子节点
    fn lazy_load_children(&mut self, node_id: String, cx: &mut Context<Self>) {
        // 如果已经加载过或正在加载，跳过
        if self.loaded_children.contains(&node_id) || self.loading_nodes.contains(&node_id) {
            return;
        }

        // 获取节点信息
        let node = match self.db_nodes.get(&node_id) {
            Some(n) => n.clone(),
            None => {
                error!(
                    "DbTreeView lazy_load_children: node not found in db_nodes: {}",
                    node_id
                );
                return;
            }
        };

        info!(
            "DbTreeView lazy_load_children: attempting to load children for: {} (type: {:?})",
            node_id, node.node_type
        );

        // 标记为正在加载
        self.loading_nodes.insert(node_id.clone());
        cx.notify();

        let Some(global_state) = cx.try_global::<GlobalDbState>() else {
            warn!(
                "GlobalDbState 不存在，无法加载连接 {} 的子节点",
                &node.connection_id
            );
            return;
        };
        let global_state = global_state.clone();
        let clone_node_id = node_id.clone();
        let connection_id = node.connection_id.clone();
        let node_type = node.node_type.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            // 使用 DatabasePlugin 的方法加载子节点，添加超时机制
            let children_result = global_state.load_node_children(cx, connection_id.clone(), node.clone()).await;

            this.update(cx, |this: &mut Self, cx| {
                // 移除加载状态
                this.loading_nodes.remove(&clone_node_id);

                match children_result {
                    Ok(children) => {
                        info!("DbTreeView lazy_load_children: loaded {} children for node: {}", children.len(), clone_node_id);
                        // 标记为已加载，清除错误状态
                        this.loaded_children.insert(clone_node_id.clone());
                        this.error_nodes.remove(&clone_node_id);

                        // 如果是 Connection 节点，设置为活跃状态
                        if node_type == DbNodeType::Connection {
                            if let Ok(conn_id) = connection_id.parse::<i64>() {
                                cx.global_mut::<ActiveConnections>().add(conn_id);
                            }
                        }

                        // 更新节点的子节点
                        if let Some(parent_node) = this.db_nodes.get_mut(&clone_node_id) {
                            parent_node.children = children.clone();
                            parent_node.children_loaded = true;
                            // 子节点加载完成后，如果该节点是当前选中节点，重新触发选中事件以刷新对象页签
                            if this.selected_node_id.as_deref() == Some(&clone_node_id) {
                                cx.emit(DbTreeViewEvent::NodeSelected{node_id: clone_node_id.clone()})
                            }
                        }

                        // 递归地将所有子节点及其后代添加到 db_nodes
                        fn insert_nodes_recursive(
                            db_nodes: &mut HashMap<String, DbNode>,
                            node: &DbNode,
                        ) {
                            db_nodes.insert(node.id.clone(), node.clone());
                            for child in &node.children {
                                insert_nodes_recursive(db_nodes, child);
                            }
                        }

                        for child in &children {
                            trace!("DbTreeView lazy_load_children: adding child: {} (type: {:?})", child.id, child.node_type);
                            insert_nodes_recursive(&mut this.db_nodes, child);
                        }

                        // 检查子节点是否在 expanded_nodes 中，如果是，递归加载它们的子节点
                        // 这确保刷新后已展开的节点能恢复其子节点
                        let children_to_expand: Vec<String> = children
                            .iter()
                            .filter(|child| this.expanded_nodes.contains(&child.id))
                            .map(|child| child.id.clone())
                            .collect();

                        // 重建树结构
                        this.rebuild_tree(cx);

                        // 触发已展开子节点的懒加载
                        for child_id in children_to_expand {
                            this.lazy_load_children(child_id, cx);
                        }
                    }
                    Err(e) => {
                        error!("DbTreeView lazy_load_children: failed to execute load_node_children for {}: {}", clone_node_id, e);
                        this.expanded_nodes.remove(&clone_node_id);
                        // 记录错误状态
                        this.error_nodes.insert(clone_node_id.clone(), e.to_string());
                        this.rebuild_tree(cx);
                    }
                }
            }).ok();
        }).detach();
    }

    /// 重建扁平化的树条目
    pub fn rebuild_flat_entries(&mut self, cx: &mut Context<Self>) {
        self.flat_entries.clear();
        self.selected_ix = None;

        // 获取根节点并排序
        let mut root_nodes: Vec<DbNode> = self
            .db_nodes
            .values()
            .filter(|n| n.parent_context.is_none())
            .cloned()
            .collect();
        root_nodes.sort();

        let search_query = self.search_query.to_lowercase();

        // 递归添加条目
        for node in root_nodes {
            self.add_flat_entry_recursive(&node.id, 0, &search_query, None);
        }

        cx.notify();
    }

    /// 递归添加扁平化条目
    fn add_flat_entry_recursive(
        &mut self,
        node_id: &str,
        depth: usize,
        query: &str,
        current_connection_id: Option<&str>,
    ) -> bool {
        let node = match self.db_nodes.get(node_id) {
            Some(n) => n.clone(),
            None => return false,
        };

        // 确定当前所属的连接ID
        let conn_id = if node.node_type == DbNodeType::Connection {
            Some(node.id.as_str())
        } else {
            current_connection_id
        };

        // 如果是数据库或Schema节点（Oracle），检查是否被选中
        if let Some(cid) = conn_id {
            if self.should_filter_by_database_selection(&node, cid) {
                let is_selected = match self.selected_databases.get(cid) {
                    None => true,
                    Some(None) => true,
                    Some(Some(set)) => set.contains(&node.name),
                };
                if !is_selected {
                    return false;
                }
            }
        }

        // 检查当前节点是否匹配搜索
        let self_matches = query.is_empty() || node.name.to_lowercase().contains(query);

        // 检查子节点是否有匹配的
        let mut has_matching_children = false;
        let mut matching_children: Vec<String> = Vec::new();

        if node.children_loaded && !node.children.is_empty() {
            for child in &node.children {
                let child_id = child.id.clone();
                // 递归检查子节点（但先不添加，只是检查是否匹配）
                if self.check_node_matches(&child_id, query, conn_id) {
                    has_matching_children = true;
                    matching_children.push(child_id);
                }
            }
        }

        // 如果当前节点不匹配且没有匹配的子节点，跳过
        if !self_matches && !has_matching_children {
            return false;
        }

        // 添加当前节点
        self.flat_entries.push(FlatDbEntry {
            node_id: node_id.to_string(),
            depth,
        });

        // 如果展开或者搜索匹配到子节点，添加子节点
        let should_show_children = if !query.is_empty() {
            has_matching_children
        } else {
            self.expanded_nodes.contains(node_id)
        };

        if should_show_children {
            // 需要克隆 children 以避免借用冲突
            let children: Vec<String> = node.children.iter().map(|c| c.id.clone()).collect();
            for child_id in children {
                self.add_flat_entry_recursive(&child_id, depth + 1, query, conn_id);
            }
        }

        true
    }

    /// 检查节点是否匹配搜索条件（不添加条目，仅检查）
    fn check_node_matches(
        &self,
        node_id: &str,
        query: &str,
        current_connection_id: Option<&str>,
    ) -> bool {
        let node = match self.db_nodes.get(node_id) {
            Some(n) => n,
            None => return false,
        };

        // 确定当前所属的连接ID
        let conn_id = if node.node_type == DbNodeType::Connection {
            Some(node.id.as_str())
        } else {
            current_connection_id
        };

        // 如果是数据库或Schema节点（Oracle），检查是否被选中
        if let Some(cid) = conn_id {
            if self.should_filter_by_database_selection(node, cid) {
                let is_selected = match self.selected_databases.get(cid) {
                    None => true,
                    Some(None) => true,
                    Some(Some(set)) => set.contains(&node.name),
                };
                if !is_selected {
                    return false;
                }
            }
        }

        // 检查当前节点是否匹配
        let self_matches = query.is_empty() || node.name.to_lowercase().contains(query);
        if self_matches {
            return true;
        }

        // 检查子节点
        if node.children_loaded {
            for child in &node.children {
                if self.check_node_matches(&child.id, query, conn_id) {
                    return true;
                }
            }
        }

        false
    }

    /// 检查节点是否需要显示展开箭头（有子节点或可能有子节点）
    fn node_has_children(&self, node_id: &str) -> bool {
        if let Some(node) = self.db_nodes.get(node_id) {
            // 如果已有子节点，显示箭头
            if !node.children.is_empty() {
                return true;
            }
            // Connection、Database、Schema 未激活时不显示箭头，只能通过双击激活
            if matches!(
                node.node_type,
                DbNodeType::Connection | DbNodeType::Database | DbNodeType::Schema
            ) {
                return false;
            }
            // 其他类型：未加载但可能有子节点时显示箭头
            if !node.children_loaded {
                return matches!(
                    node.node_type,
                    DbNodeType::Table
                        | DbNodeType::TablesFolder
                        | DbNodeType::ViewsFolder
                        | DbNodeType::ColumnsFolder
                        | DbNodeType::IndexesFolder
                        | DbNodeType::FunctionsFolder
                        | DbNodeType::ProceduresFolder
                        | DbNodeType::TriggersFolder
                        | DbNodeType::SequencesFolder
                        | DbNodeType::QueriesFolder
                        | DbNodeType::ForeignKeysFolder
                        | DbNodeType::ChecksFolder
                );
            }
        }
        false
    }

    /// 保持向后兼容的方法别名
    pub fn rebuild_tree(&mut self, cx: &mut Context<Self>) {
        self.rebuild_flat_entries(cx);
    }

    fn render_database_filter_popover(
        view: &Entity<Self>,
        connection_id: &str,
        list_state: &Entity<ListState<DatabaseListDelegate>>,
        cx: &mut App,
    ) -> AnyElement {
        let view_clone = view.clone();
        let conn_id = connection_id.to_string();
        let is_all_selected = view.read(cx).is_all_selected(&conn_id);

        v_flex()
            .w(px(280.0))
            .max_h(px(400.0))
            .gap_2()
            .p_2()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .px_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child({
                                let view_select = view_clone.clone();
                                let conn_select = conn_id.clone();
                                Checkbox::new("select-all")
                                    .checked(is_all_selected)
                                    .on_click(move |_, _, cx| {
                                        view_select.update(cx, |this, cx| {
                                            if this.is_all_selected(&conn_select) {
                                                this.deselect_all_databases(&conn_select, cx);
                                            } else {
                                                this.select_all_databases(&conn_select, cx);
                                            }
                                        });
                                    })
                            })
                            .child(div().text_sm().child(t!("Common.select_all").to_string())),
                    )
                    .child({
                        let view_clear = view_clone.clone();
                        let conn_clear = conn_id.clone();
                        Button::new("clear-filter")
                            .ghost()
                            .small()
                            .label(t!("Common.clear_filter"))
                            .on_click(move |_, _, cx| {
                                view_clear.update(cx, |this, cx| {
                                    this.deselect_all_databases(&conn_clear, cx);
                                });
                            })
                    }),
            )
            .child(div().border_t_1().border_color(cx.theme().border))
            .child(
                List::new(list_state)
                    .w_full()
                    .max_h(px(320.0))
                    .p(px(8.))
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius),
            )
            .into_any_element()
    }

    pub fn active_connection(&mut self, active_conn_id: String, cx: &mut Context<Self>) {
        self.selected_node_id = Some(active_conn_id.clone());
        self.expanded_nodes.insert(active_conn_id.clone());
        self.lazy_load_children(active_conn_id, cx);
        self.rebuild_tree(cx);
    }

    /// 根据节点类型获取图标
    fn get_icon_for_node(
        &self,
        node_id: &str,
        _is_expanded: bool,
        _cx: &mut Context<Self>,
    ) -> Icon {
        let node = self.db_nodes.get(node_id);
        match node.map(|n| &n.node_type) {
            Some(DbNodeType::Connection) => {
                if let Some(n) = node {
                    n.database_type.as_node_icon()
                } else {
                    IconName::Database.color().with_size(ComponentSize::Large)
                }
            }
            Some(DbNodeType::Database) => Icon::from(IconName::Database)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::Schema) => Icon::from(IconName::Schema)
                .color()
                .with_size(ComponentSize::Size(px(20.))),

            Some(DbNodeType::TablesFolder) => Icon::from(IconName::FolderTables)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::ViewsFolder) => Icon::from(IconName::FolderViews)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::FunctionsFolder) => Icon::from(IconName::FolderFunctions)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::ProceduresFolder) => Icon::from(IconName::FolderProcedures)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::TriggersFolder) => Icon::from(IconName::FolderTriggers)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::ForeignKeysFolder) => Icon::from(IconName::FolderForeignKeys)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::ChecksFolder) => Icon::from(IconName::FolderCheckConstraints)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::QueriesFolder) => Icon::from(IconName::FolderQueries)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::ColumnsFolder) => Icon::from(IconName::FolderColumns)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::IndexesFolder) => Icon::from(IconName::FolderIndexes)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::SequencesFolder) => Icon::from(IconName::FolderSequences)
                .color()
                .with_size(ComponentSize::Size(px(20.))),

            Some(DbNodeType::Table) => Icon::from(IconName::Table)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::View) => Icon::from(IconName::View)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::Function) => Icon::from(IconName::Function)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::Procedure) => Icon::from(IconName::Procedure)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::Column) => {
                let is_primary_key = node
                    .map(|n| n.metadata.get("is_primary_key"))
                    .flatten()
                    .map(|v| v == "true")
                    .unwrap_or(false);
                if is_primary_key {
                    Icon::from(IconName::PrimaryKey)
                        .color()
                        .with_size(ComponentSize::Size(px(20.)))
                } else {
                    Icon::from(IconName::Column)
                        .color()
                        .with_size(ComponentSize::Size(px(20.)))
                }
            }
            Some(DbNodeType::Index) => Icon::from(IconName::Index)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::ForeignKey) => Icon::from(IconName::GoldKey)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::Trigger) => Icon::from(IconName::Trigger)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::Sequence) => Icon::from(IconName::Sequence)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::Check) => Icon::from(IconName::CheckConstraint)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            Some(DbNodeType::NamedQuery) => Icon::from(IconName::Query)
                .color()
                .with_size(ComponentSize::Size(px(20.))),
            _ => Icon::from(IconName::Loader).with_size(ComponentSize::Size(px(14.))),
        }
    }

    fn handle_item_double_click(&mut self, node_id: &str, cx: &mut Context<Self>) {
        // 如果节点有错误，双击重试连接
        if self.error_nodes.contains_key(node_id) {
            self.error_nodes.remove(node_id);
            self.lazy_load_children(node_id.to_string(), cx);
            return;
        }

        // 根据节点类型执行不同的操作
        if let Some(node) = self.db_nodes.get(node_id).cloned() {
            let database = node.get_database_name().unwrap_or_default();
            match node.node_type {
                DbNodeType::Table => {
                    // 查找所属数据库

                    info!(
                        "DbTreeView: opening table data tab: {}.{}",
                        database, node.name
                    );
                    cx.emit(DbTreeViewEvent::OpenTableData {
                        node_id: node.id.clone(),
                    });
                }
                DbNodeType::View => {
                    info!(
                        "DbTreeView: opening view data tab: {}.{}",
                        database, node.name
                    );
                    cx.emit(DbTreeViewEvent::OpenViewData {
                        node_id: node.id.clone(),
                    });
                }
                DbNodeType::NamedQuery => {
                    // 打开命名查询
                    info!("DbTreeView: opening named query: {}", node.name);
                    cx.emit(DbTreeViewEvent::OpenNamedQuery {
                        node_id: node.id.clone(),
                    });
                }
                DbNodeType::Connection
                | DbNodeType::Database
                | DbNodeType::Schema
                | DbNodeType::ColumnsFolder
                | DbNodeType::IndexesFolder
                | DbNodeType::FunctionsFolder
                | DbNodeType::ProceduresFolder
                | DbNodeType::TriggersFolder
                | DbNodeType::QueriesFolder
                | DbNodeType::TablesFolder
                | DbNodeType::ViewsFolder => {
                    let is_expanded = self.expanded_nodes.contains(node_id);

                    // 切换展开状态
                    if is_expanded {
                        self.expanded_nodes.remove(node_id);
                    } else {
                        self.expanded_nodes.insert(node_id.to_string());
                    }

                    // 如果是展开操作，加载子节点（如果尚未加载）
                    if !is_expanded {
                        self.lazy_load_children(node_id.to_string(), cx);
                    }
                    // 无论展开还是折叠，都需要重建树以更新展开状态
                    self.rebuild_tree(cx);
                }
                _ => {
                    // 其他类型的节点暂不处理双击
                }
            }
            cx.emit(DbTreeViewEvent::NodeSelected {
                node_id: node_id.to_string(),
            })
        }
        cx.notify();
    }

    fn handle_item_click(&mut self, node_id: &str, ix: usize, cx: &mut Context<Self>) {
        self.selected_node_id = Some(node_id.to_string());
        self.selected_ix = Some(ix);
        // 发出节点选择事件
        cx.emit(DbTreeViewEvent::NodeSelected {
            node_id: node_id.to_string(),
        });
        cx.notify();
    }

    /// 获取节点信息（公开方法）
    pub fn get_node(&self, node_id: &str) -> Option<&DbNode> {
        self.db_nodes.get(node_id)
    }

    /// 查找指定连接下的数据库节点 ID
    fn find_database_node(&self, connection_id: &str, database_name: &str) -> Option<String> {
        if let Some(conn_node) = self.db_nodes.get(connection_id) {
            for child in &conn_node.children {
                if child.node_type == DbNodeType::Database && child.name == database_name {
                    return Some(child.id.clone());
                }
            }
        }
        None
    }

    /// 关闭连接并清理相关状态
    pub fn close_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        info!("Closing connection in DbTreeView: {}", connection_id);

        let global_state = cx.global::<GlobalDbState>().clone();
        if let Some(config) = global_state.get_config(connection_id) {
            let cache_ctx = db::CacheContext::from_config(&config);
            let conn_id = connection_id.to_string();
            if let Some(cache) = cx.try_global::<db::GlobalNodeCache>().cloned() {
                Tokio::spawn(cx, async move {
                    cache.clear_connection_cache(&cache_ctx).await;
                    cache.invalidate_connection_metadata(&conn_id).await;
                    Ok::<_, anyhow::Error>(())
                })
                .detach();
            }
        }

        if let Ok(conn_id) = connection_id.parse::<i64>() {
            cx.global_mut::<ActiveConnections>().remove(conn_id);
        }

        self.clear_expanded_descendants(connection_id);
        self.clear_node_descendants(connection_id);
        self.reset_node_children(connection_id);
        self.clear_node_all_state(connection_id);
        self.rebuild_tree(cx);
    }

    /// 关闭数据库并清理相关状态
    pub fn close_database(&mut self, database_node_id: &str, cx: &mut Context<Self>) {
        info!("Closing database in DbTreeView: {}", database_node_id);

        self.clear_expanded_descendants(database_node_id);
        self.clear_node_descendants(database_node_id);
        self.reset_node_children(database_node_id);
        self.clear_node_all_state(database_node_id);
        self.rebuild_tree(cx);
    }

    /// 添加数据库节点（用于新建数据库后直接更新树，避免刷新整个连接）
    pub fn add_database_node(
        &mut self,
        connection_id: &str,
        database_name: &str,
        cx: &mut Context<Self>,
    ) {
        info!(
            "Adding database node: {} to connection: {}",
            database_name, connection_id
        );

        // 获取连接节点信息
        let (database_type, connection_id_str) = match self.db_nodes.get(connection_id) {
            Some(conn_node) => (conn_node.database_type, conn_node.connection_id.clone()),
            None => {
                error!("Connection node not found: {}", connection_id);
                return;
            }
        };

        // 创建新的数据库节点
        let db_node_id = format!("{}:{}", connection_id, database_name);
        let db_node = DbNode::new(
            db_node_id.clone(),
            database_name.to_string(),
            DbNodeType::Database,
            connection_id_str,
            database_type,
        )
        .with_parent_context(connection_id.to_string());

        // 添加到 db_nodes
        self.db_nodes.insert(db_node_id.clone(), db_node.clone());

        // 添加到连接节点的子节点列表
        if let Some(conn_node) = self.db_nodes.get_mut(connection_id) {
            conn_node.children.push(db_node);
            conn_node.children.sort();
        }

        // 重建树以反映变化
        self.rebuild_tree(cx);
    }

    /// 移除数据库节点（用于删除数据库后直接更新树，避免刷新整个连接）
    pub fn remove_database_node(
        &mut self,
        connection_id: &str,
        database_name: &str,
        cx: &mut Context<Self>,
    ) {
        info!(
            "Removing database node: {} from connection: {}",
            database_name, connection_id
        );

        let db_node_id = format!("{}:{}", connection_id, database_name);

        self.clear_node_descendants(&db_node_id);
        self.db_nodes.remove(&db_node_id);
        self.clear_node_all_state(&db_node_id);

        if let Some(conn_node) = self.db_nodes.get_mut(connection_id) {
            conn_node.children.retain(|child| child.id != db_node_id);
        }

        self.rebuild_tree(cx);
    }

    /// 添加 Schema 节点（用于新建 Schema 后直接更新树）
    pub fn add_schema_node(
        &mut self,
        connection_id: &str,
        database_name: &str,
        schema_name: &str,
        cx: &mut Context<Self>,
    ) {
        info!(
            "Adding schema node: {} to database: {}",
            schema_name, database_name
        );

        let db_node_id = format!("{}:{}", connection_id, database_name);

        // 获取数据库节点信息
        let database_type = match self.db_nodes.get(&db_node_id) {
            Some(db_node) => db_node.database_type,
            None => {
                error!("Database node not found: {}", db_node_id);
                return;
            }
        };

        // 创建新的 Schema 节点
        let schema_node_id = format!("{}:{}:{}", connection_id, database_name, schema_name);
        let schema_node = DbNode::new(
            schema_node_id.clone(),
            schema_name.to_string(),
            DbNodeType::Schema,
            connection_id.to_string(),
            database_type,
        )
        .with_parent_context(db_node_id.clone());

        // 添加到 db_nodes
        self.db_nodes
            .insert(schema_node_id.clone(), schema_node.clone());

        // 添加到数据库节点的子节点列表
        if let Some(db_node) = self.db_nodes.get_mut(&db_node_id) {
            db_node.children.push(schema_node);
            db_node.children.sort();
        }

        // 重建树以反映变化
        self.rebuild_tree(cx);
    }

    /// 移除 Schema 节点（用于删除 Schema 后直接更新树）
    pub fn remove_schema_node(
        &mut self,
        connection_id: &str,
        database_name: &str,
        schema_name: &str,
        cx: &mut Context<Self>,
    ) {
        info!(
            "Removing schema node: {} from database: {}",
            schema_name, database_name
        );

        let db_node_id = format!("{}:{}", connection_id, database_name);
        let schema_node_id = format!("{}:{}:{}", connection_id, database_name, schema_name);

        self.clear_node_descendants(&schema_node_id);
        self.db_nodes.remove(&schema_node_id);
        self.clear_node_all_state(&schema_node_id);

        if let Some(db_node) = self.db_nodes.get_mut(&db_node_id) {
            db_node.children.retain(|child| child.id != schema_node_id);
        }

        self.rebuild_tree(cx);
    }

    /// 移除表节点（用于删除表后直接更新树）
    pub fn remove_table_node(&mut self, node_id: &str, cx: &mut Context<Self>) {
        info!("Removing table node: {}", node_id);
        self.remove_node_complete(node_id);
        self.rebuild_tree(cx);
    }

    /// 移除视图节点（用于删除视图后直接更新树）
    pub fn remove_view_node(&mut self, node_id: &str, cx: &mut Context<Self>) {
        info!("Removing view node: {}", node_id);
        self.remove_node_complete(node_id);
        self.rebuild_tree(cx);
    }

    /// 获取当前选中的数据库名称
    pub fn get_selected_database(&self) -> Option<String> {
        if let Some(id) = self.selected_node_id.clone() {
            // 从选中的节点ID中提取数据库名
            if let Some(node) = self.db_nodes.get(&id) {
                return node.get_database_name();
            }
        }
        None
    }
}

impl Render for DbTreeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries_len = self.flat_entries.len();

        v_flex()
            .id("db-tree-view")
            .size_full()
            .bg(cx.theme().sidebar)
            .child({
                let view_for_collapse = cx.entity();
                h_flex()
                    .w_full()
                    .p_1()
                    .gap_1()
                    .border_t_1()
                    .border_color(cx.theme().sidebar_border)
                    .bg(cx.theme().sidebar)
                    .child(
                        div().flex_1().child(
                            Input::new(&self.search_input)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .cleanable(true)
                                .small()
                                .w_full(),
                        ),
                    )
                    .child(
                        Button::new("collapse-all")
                            .icon(IconName::ChevronsUpDown)
                            .ghost()
                            .small()
                            .tooltip(t!("Common.collapse_all"))
                            .on_click(move |_, _, cx| {
                                view_for_collapse.update(cx, |this, cx| {
                                    this.collapse_all(cx);
                                });
                            }),
                    )
            })
            .child(
                // 树形视图
                v_flex()
                    .flex_1()
                    .w_full()
                    .bg(cx.theme().sidebar)
                    .child(
                        div()
                            .id("tree-scroll")
                            .flex_1()
                            .overflow_scroll()
                            .p_2()
                            .map(|this| {
                                if entries_len == 0 && !self.search_query.is_empty() {
                                    // 搜索无结果时显示空状态
                                    this.child(
                                        v_flex()
                                            .size_full()
                                            .items_center()
                                            .justify_center()
                                            .gap_3()
                                            .child(
                                                Icon::new(IconName::Search)
                                                    .with_size(ComponentSize::Large)
                                                    .text_color(cx.theme().muted_foreground),
                                            )
                                            .child(
                                                div()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(t!("Common.not_found").to_string()),
                                            ),
                                    )
                                } else {
                                    this.child(self.render_tree_list(window, cx))
                                }
                            }),
                    )
                    .child(Scrollbar::vertical(&self.scroll_handle)),
            )
    }
}

impl DbTreeView {
    fn render_tree_list(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries_len = self.flat_entries.len();
        let selected_ix = self.selected_ix;

        uniform_list(
            "db-tree-entries",
            entries_len,
            cx.processor(
                move |this: &mut Self, visible_range: Range<usize>, window, cx| {
                    let view = cx.entity();
                    let mut items = Vec::with_capacity(visible_range.len());
                    for ix in visible_range {
                        if let Some(entry) = this.flat_entries.get(ix).cloned() {
                            let is_selected = selected_ix == Some(ix);
                            let item = this.render_tree_item(
                                ix,
                                entry,
                                is_selected,
                                view.clone(),
                                window,
                                cx,
                            );
                            items.push(item);
                        }
                    }
                    items
                },
            ),
        )
        .flex_1()
        .size_full()
        .track_scroll(&self.scroll_handle)
        .with_sizing_behavior(ListSizingBehavior::Auto)
    }

    fn render_tree_item(
        &mut self,
        ix: usize,
        entry: FlatDbEntry,
        is_selected: bool,
        view: Entity<Self>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let node_id = entry.node_id.clone();
        let depth = entry.depth;

        // 获取节点信息
        let node = self.db_nodes.get(&node_id).cloned();
        let is_expanded = self.expanded_nodes.contains(&node_id);
        let is_loading = self.loading_nodes.contains(&node_id);
        let error_msg = self.error_nodes.get(&node_id).cloned();
        let has_children = self.node_has_children(&node_id);
        let search_query = self.search_query.clone();

        // 获取图标
        let icon = self.get_icon_for_node(&node_id, is_expanded, cx).color();

        // 获取节点名称
        let label_text = node
            .as_ref()
            .map(|n| {
                if matches!(
                    n.node_type,
                    DbNodeType::TablesFolder
                        | DbNodeType::ViewsFolder
                        | DbNodeType::FunctionsFolder
                        | DbNodeType::ProceduresFolder
                        | DbNodeType::SequencesFolder
                        | DbNodeType::QueriesFolder
                        | DbNodeType::ColumnsFolder
                        | DbNodeType::IndexesFolder
                        | DbNodeType::ForeignKeysFolder
                        | DbNodeType::TriggersFolder
                        | DbNodeType::ChecksFolder
                ) {
                    t!(&n.name).to_string()
                } else {
                    n.name.clone()
                }
            })
            .unwrap_or_default();
        let label_for_tooltip = if let Some(ref error) = error_msg {
            error.clone()
        } else {
            label_text.clone()
        };

        // 获取节点类型相关信息
        let node_type = node.as_ref().map(|n| n.node_type.clone());
        let database_type = node.as_ref().map(|n| n.database_type.clone());
        // 判断是否是分组类型（Folder 类型）
        let is_folder_type = matches!(
            node_type,
            Some(DbNodeType::TablesFolder)
                | Some(DbNodeType::ViewsFolder)
                | Some(DbNodeType::FunctionsFolder)
                | Some(DbNodeType::ProceduresFolder)
                | Some(DbNodeType::TriggersFolder)
                | Some(DbNodeType::QueriesFolder)
                | Some(DbNodeType::ColumnsFolder)
                | Some(DbNodeType::IndexesFolder)
        );

        // 数据库筛选计数
        let db_count = if node_type == Some(DbNodeType::Connection)
            && !matches!(
                database_type,
                Some(DatabaseType::SQLite | DatabaseType::DuckDB)
            ) {
            Some(self.get_selected_database_count(&node_id))
        } else {
            None
        };

        // 获取数据库筛选列表状态
        let db_filter_list = self.db_filter_list_states.get(&node_id).cloned();

        // 样式
        let selection_bg = cx.theme().sidebar_accent;
        let selection_bar_color = cx.theme().blue;
        let selection_text_color = cx.theme().sidebar_accent_foreground;
        let hover_bg = cx.theme().secondary;
        let folder_text_color = cx.theme().muted_foreground;
        let foreground_color = cx.theme().sidebar_foreground;
        let indent = px(8.) + px(16.) * depth as f32;

        // 箭头
        let view_for_arrow = view.clone();
        let node_id_for_arrow = node_id.clone();
        let arrow = if has_children {
            let arrow_icon = if is_expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            };
            div()
                .id(SharedString::from(format!("arrow-{}", ix)))
                .w(px(18.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(arrow_icon)
                        .with_size(ComponentSize::XSmall)
                        .text_color(cx.theme().muted_foreground),
                )
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    view_for_arrow.update(cx, |this, cx| {
                        let is_expanded = this.expanded_nodes.contains(&node_id_for_arrow);
                        if is_expanded {
                            this.expanded_nodes.remove(&node_id_for_arrow);
                        } else {
                            this.expanded_nodes.insert(node_id_for_arrow.clone());
                            this.lazy_load_children(node_id_for_arrow.clone(), cx);
                        }
                        this.rebuild_flat_entries(cx);
                    });
                })
                .into_any_element()
        } else {
            div().w(px(18.)).into_any_element()
        };

        // 克隆用于事件处理
        let view_for_click = view.clone();
        let view_for_double_click = view.clone();
        let view_for_context = view.clone();
        let node_id_for_click = node_id.clone();
        let node_id_for_double_click = node_id.clone();
        let node_id_for_context = node_id.clone();
        let view_for_filter = view.clone();
        let node_id_for_filter = node_id.clone();

        // 构建行
        div()
            .id(SharedString::from(format!("tree-item-{}", ix)))
            .w_full()
            .h(px(26.))
            .relative()
            .flex()
            .items_center()
            .text_sm()
            .text_color(foreground_color)
            .pl(indent)
            // 选中时显示左侧蓝条和背景
            .when(is_selected, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(px(3.))
                        .bg(selection_bar_color),
                )
                .bg(selection_bg)
                .text_color(selection_text_color)
            })
            // hover 背景
            .when(!is_selected, |this| this.hover(|style| style.bg(hover_bg)))
            .pr_2()
            // 点击事件（处理单击和双击）
            .on_click({
                let node_id = node_id_for_click.clone();
                let node_id_for_dbl = node_id_for_double_click.clone();
                move |event, _window, cx| {
                    if event.click_count() == 2 {
                        view_for_double_click.update(cx, |this, cx| {
                            this.handle_item_double_click(&node_id_for_dbl, cx);
                        });
                    } else {
                        view_for_click.update(cx, |this, cx| {
                            this.handle_item_click(&node_id, ix, cx);
                        });
                    }
                }
            })
            // 内容
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .child(arrow)
                    .child(icon)
                    .child(
                        div()
                            .id(SharedString::from(format!("label-{}", ix)))
                            .flex_1()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .when(is_folder_type && !is_selected, |this| {
                                this.text_color(folder_text_color)
                            })
                            .child(
                                Label::new(label_text)
                                    .highlights(search_query)
                                    .into_any_element(),
                            )
                            .tooltip(move |window, cx| {
                                Tooltip::new(label_for_tooltip.clone()).build(window, cx)
                            }),
                    )
                    .when_some(db_count, |this, (selected, total)| {
                        if total > 0 {
                            let view_open = view_for_filter.clone();
                            let node_id_open = node_id_for_filter.clone();

                            this.child(
                                Popover::new(SharedString::from(format!("db-filter-{}", ix)))
                                    .on_open_change(move |open, window, cx| {
                                        if *open {
                                            view_open.update(cx, |this, cx| {
                                                let databases_data = this
                                                    .get_databases_for_connection(&node_id_open);

                                                if let Some(list_state) =
                                                    this.db_filter_list_states.get(&node_id_open)
                                                {
                                                    list_state.update(cx, |state, _| {
                                                        let delegate = state.delegate_mut();
                                                        delegate.databases = databases_data.clone();
                                                        delegate.filtered_databases =
                                                            databases_data;
                                                    });
                                                } else {
                                                    let list_state = cx.new(|cx| {
                                                        ListState::new(
                                                            DatabaseListDelegate::new(
                                                                view_open.clone(),
                                                                node_id_open.clone(),
                                                                databases_data.clone(),
                                                            ),
                                                            window,
                                                            cx,
                                                        )
                                                        .searchable(true)
                                                    });
                                                    this.db_filter_list_states
                                                        .insert(node_id_open.clone(), list_state);
                                                }
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .when_some(db_filter_list.as_ref(), |popover, list| {
                                        popover.track_focus(&list.focus_handle(cx))
                                    })
                                    .trigger(
                                        Button::new(SharedString::from(format!(
                                            "db-filter-trigger-{}",
                                            ix
                                        )))
                                        .ghost()
                                        .small()
                                        .label(format!("{} of {}", selected, total)),
                                    )
                                    .when_some(db_filter_list, |popover, list| {
                                        let view_content = view_for_filter.clone();
                                        let node_id_content = node_id_for_filter.clone();
                                        popover.content(move |_state, _window, cx| {
                                            Self::render_database_filter_popover(
                                                &view_content,
                                                &node_id_content,
                                                &list,
                                                cx,
                                            )
                                        })
                                    }),
                            )
                        } else {
                            this
                        }
                    })
                    .when(is_loading, |this| {
                        this.child(
                            Spinner::new()
                                .with_size(ComponentSize::Small)
                                .color(cx.theme().muted_foreground),
                        )
                    })
                    .when_some(error_msg.clone(), |this, error_text| {
                        let error_for_copy = error_text.clone();
                        this.child(
                            Popover::new(SharedString::from(format!("error-popover-{}", ix)))
                                .trigger(
                                    Button::new(SharedString::from(format!("error-btn-{}", ix)))
                                        .ghost()
                                        .icon(IconName::TriangleAlert)
                                        .xsmall()
                                        .text_color(cx.theme().warning),
                                )
                                .content(move |_state, _window, cx| {
                                    let error_for_copy = error_for_copy.clone();
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    h_flex()
                                                        .items_center()
                                                        .gap_1()
                                                        .child(
                                                            Icon::new(IconName::TriangleAlert)
                                                                .with_size(ComponentSize::Small)
                                                                .text_color(cx.theme().warning),
                                                        )
                                                        .child(t!("Common.error_info").to_string()),
                                                )
                                                .child(
                                                    Clipboard::new(SharedString::from(format!(
                                                        "copy-error-{}",
                                                        ix
                                                    )))
                                                    .value(error_for_copy),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(error_text.clone()),
                                        )
                                })
                                .max_w(px(400.)),
                        )
                    }),
            )
            // 上下文菜单
            .context_menu({
                move |menu, window, cx| {
                    if let Some(node) = view_for_context
                        .read(cx)
                        .db_nodes
                        .get(&node_id_for_context)
                        .cloned()
                    {
                        Self::build_context_menu(
                            menu,
                            &node,
                            &node_id_for_context,
                            &view_for_context,
                            window,
                            cx,
                        )
                    } else {
                        menu
                    }
                }
            })
            .into_any_element()
    }

    fn build_context_menu(
        mut menu: PopupMenu,
        node: &DbNode,
        node_id: &str,
        view: &Entity<Self>,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        // 判断节点是否处于可操作状态：
        // - 连接必须激活
        // - 非 Connection 节点还需要 children_loaded（即节点已展开加载过）
        let conn_active = node
            .connection_id
            .parse::<i64>()
            .ok()
            .map(|conn_id| cx.global::<ActiveConnections>().is_active(conn_id))
            .unwrap_or(false);
        let is_active =
            conn_active && (node.node_type != DbNodeType::Database || node.children_loaded);

        // 尝试从 plugin 获取菜单
        let registry = cx.global::<DatabaseViewPluginRegistry>();
        if let Some(plugin) = registry.get(&node.database_type) {
            let menu_items = plugin.build_context_menu(node_id, node.node_type);

            if !menu_items.is_empty() {
                // 渲染 plugin 提供的菜单，传入连接激活状态
                menu =
                    Self::render_context_menu_items(menu, menu_items, is_active, view, window, cx);
            }
        }

        // 添加通用的刷新菜单项
        let view_ref = view.clone();
        let node_id_for_refresh = node_id.to_string();
        menu.item(
            PopupMenuItem::new(t!("Common.refresh")).on_click(window.listener_for(
                &view_ref,
                move |this, _, _, cx| {
                    this.refresh_tree(node_id_for_refresh.clone(), cx);
                },
            )),
        )
    }
}

impl EventEmitter<DbTreeViewEvent> for DbTreeView {}

impl Focusable for DbTreeView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use one_core::storage::ConnectionType;

    fn build_node(node_type: DbNodeType, name: &str, metadata: &[(&str, &str)]) -> DbNode {
        let metadata = metadata
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();

        DbNode::new(
            format!("node-{name}"),
            name,
            node_type,
            "conn-1".to_string(),
            DatabaseType::MySQL,
        )
        .with_metadata(metadata)
    }

    fn build_connection(id: i64, selected_databases: Option<Vec<&str>>) -> StoredConnection {
        StoredConnection {
            id: Some(id),
            name: "conn".to_string(),
            connection_type: ConnectionType::Database,
            params: "{}".to_string(),
            workspace_id: None,
            selected_databases: selected_databases.map(|dbs| {
                serde_json::to_string(
                    &dbs.into_iter()
                        .map(|db| db.to_string())
                        .collect::<Vec<String>>(),
                )
                .expect("测试数据库列表序列化不应失败")
            }),
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    #[test]
    fn refresh_connection_node_uses_connection_scope() {
        let node = build_node(DbNodeType::Connection, "conn", &[]);
        assert_eq!(
            resolve_refresh_metadata_scope(&node),
            RefreshMetadataScope::Connection
        );
    }

    #[test]
    fn refresh_nodes_with_database_context_use_database_scope() {
        let cases = [
            build_node(DbNodeType::Database, "analytics", &[]),
            build_node(DbNodeType::Schema, "public", &[("database", "analytics")]),
            build_node(
                DbNodeType::Table,
                "users",
                &[("database", "analytics"), ("schema", "public")],
            ),
            build_node(
                DbNodeType::ViewsFolder,
                "views",
                &[("database", "analytics"), ("schema", "public")],
            ),
            build_node(
                DbNodeType::Function,
                "fn_count",
                &[("database", "analytics")],
            ),
        ];

        for node in cases {
            assert_eq!(
                resolve_refresh_metadata_scope(&node),
                RefreshMetadataScope::Database("analytics".to_string())
            );
        }
    }

    #[test]
    fn refresh_nodes_without_database_context_skip_metadata_invalidation() {
        let node = build_node(DbNodeType::QueriesFolder, "queries", &[]);
        assert_eq!(
            resolve_refresh_metadata_scope(&node),
            RefreshMetadataScope::None
        );
    }

    #[test]
    fn sync_selected_databases_from_connection_preserves_saved_filter() {
        let connection = build_connection(7, Some(vec!["analytics", "warehouse"]));
        let mut selected_databases = HashMap::new();

        sync_selected_databases_for_connection(&mut selected_databases, &connection);

        assert_eq!(
            selected_databases.get("7"),
            Some(&Some(HashSet::from([
                "analytics".to_string(),
                "warehouse".to_string()
            ])))
        );
    }

    #[test]
    fn sync_selected_databases_from_connection_restores_all_selected_state() {
        let connection = build_connection(9, None);
        let mut selected_databases =
            HashMap::from([("9".to_string(), Some(HashSet::from(["legacy".to_string()])))]);

        sync_selected_databases_for_connection(&mut selected_databases, &connection);

        assert_eq!(selected_databases.get("9"), Some(&None));
    }
}
