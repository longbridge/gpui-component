//! Redis 树形视图

use std::collections::{HashMap, HashSet};

use gpui::{
    AnyElement, App, AppContext, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
    UniformListScrollHandle, uniform_list,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, Size, h_flex, v_flex,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenuItem},
    popover::Popover,
    spinner::Spinner,
};
use one_core::gpui_tokio::Tokio;
use rust_i18n::t;
use one_core::storage::{ActiveConnections, StoredConnection};
use tracing::{info, warn};

use crate::{GlobalRedisState, RedisKeyType, RedisManager, RedisNode, RedisNodeType};

/// 树形视图事件
#[derive(Clone, Debug)]
pub enum RedisTreeViewEvent {
    /// 节点选中
    NodeSelected { node_id: String },
    /// 键选中
    KeySelected { node_id: String },
    /// 搜索键（服务端查询）
    SearchKeys { node_id: String, pattern: String },
    /// 在新标签页中打开键
    OpenKeyInNewTab { node_id: String },
    /// 刷新键列表
    RefreshKeys { node_id: String },
    /// 加载更多键
    LoadMoreKeys { node_id: String },
    /// 删除键
    DeleteKey { node_id: String },
    /// 创建键
    CreateKey { node_id: String },
    /// 关闭连接
    CloseConnection { node_id: String },
    /// 连接已建立
    ConnectionEstablished { node_id: String },
    /// 打开 CLI
    OpenCli { connection_id: String, db_index: u8, stored_connection: StoredConnection },
}

/// 扁平化的树条目
#[derive(Clone)]
struct FlatEntry {
    node_id: String,
    depth: usize,
}

/// Redis 树形视图
pub struct RedisTreeView {
    /// 节点映射
    nodes: HashMap<String, RedisNode>,
    /// 扁平化的条目列表
    flat_entries: Vec<FlatEntry>,
    /// 展开的节点 ID
    expanded_nodes: HashSet<String>,
    /// 选中的节点 ID
    selected_node: Option<String>,
    /// 搜索状态
    search_state: Entity<InputState>,
    /// 搜索关键词
    search_keyword: String,
    /// 搜索请求序号（node_id -> token）
    search_tokens: HashMap<String, u64>,
    /// 数据库总键数（db_node_id -> total）
    db_total_key_counts: HashMap<String, i64>,
    /// SCAN 游标（db_node_id -> cursor）
    scan_cursors: HashMap<String, u64>,
    /// SCAN 模式（db_node_id -> pattern）
    scan_patterns: HashMap<String, String>,
    /// 滚动句柄
    scroll_handle: UniformListScrollHandle,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 是否正在加载（全局）
    is_loading: bool,
    /// 正在加载的节点集合
    loading_nodes: HashSet<String>,
    /// 加载失败的节点及错误信息
    error_nodes: HashMap<String, String>,
    /// 已连接的节点集合
    connected_nodes: HashSet<String>,
    /// 存储的连接配置（node_id -> StoredConnection）
    stored_connections: HashMap<String, StoredConnection>,
}

impl RedisTreeView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("RedisTree.search_placeholder").to_string())
        });

        cx.subscribe(&search_state, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.search_keyword = this.search_state.read(cx).text().to_string();
                this.rebuild_flat_entries();
                this.update_local_search_counts();
                cx.notify();
            }

            if matches!(event, InputEvent::PressEnter { .. }) {
                this.search_keyword = this.search_state.read(cx).text().to_string();
                if let Some(node_id) = this.get_selected_refreshable_node() {
                    if let Some(node) = this.nodes.get(&node_id) {
                        if matches!(node.node_type, RedisNodeType::Database(_)) {
                            let pattern = this.search_keyword.trim().to_string();
                            if pattern.is_empty() {
                                this.reset_db_key_count(&node_id);
                                cx.emit(RedisTreeViewEvent::RefreshKeys { node_id });
                            } else {
                                cx.emit(RedisTreeViewEvent::SearchKeys { node_id, pattern });
                            }
                        }
                    }
                }
                cx.notify();
            }
        })
        .detach();

        Self {
            nodes: HashMap::new(),
            flat_entries: Vec::new(),
            expanded_nodes: HashSet::new(),
            selected_node: None,
            search_state,
            search_keyword: String::new(),
            search_tokens: HashMap::new(),
            db_total_key_counts: HashMap::new(),
            scan_cursors: HashMap::new(),
            scan_patterns: HashMap::new(),
            scroll_handle: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            is_loading: false,
            loading_nodes: HashSet::new(),
            error_nodes: HashMap::new(),
            connected_nodes: HashSet::new(),
            stored_connections: HashMap::new(),
        }
    }

    /// 从连接列表创建树视图
    pub fn new_with_connections(
        connections: &[StoredConnection],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self::new(window, cx);

        for conn in connections {
            this.add_stored_connection(conn.clone(), cx);
        }

        this
    }

    /// 添加存储的连接（未连接状态）
    pub fn add_stored_connection(&mut self, connection: StoredConnection, cx: &mut Context<Self>) {
        let conn_id = connection
            .id
            .map(|id| id.to_string())
            .unwrap_or_else(|| format!("temp-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()));

        let node = RedisNode::new(
            conn_id.clone(),
            connection.name.clone(),
            RedisNodeType::Connection,
            conn_id.clone(),
            0,
        );

        self.nodes.insert(conn_id.clone(), node);
        self.stored_connections.insert(conn_id, connection);
        self.rebuild_flat_entries();
        cx.notify();
    }

    /// 获取节点
    pub fn get_node(&self, node_id: &str) -> Option<&RedisNode> {
        self.nodes.get(node_id)
    }

    /// 获取存储的连接配置
    pub fn get_stored_connection(&self, node_id: &str) -> Option<&StoredConnection> {
        self.stored_connections.get(node_id)
    }

    /// 检查节点是否已连接
    pub fn is_connected(&self, node_id: &str) -> bool {
        self.connected_nodes.contains(node_id)
    }

    /// 检查节点是否正在加载
    pub fn is_loading_node(&self, node_id: &str) -> bool {
        self.loading_nodes.contains(node_id)
    }

    /// 获取节点错误信息
    pub fn get_error(&self, node_id: &str) -> Option<&String> {
        self.error_nodes.get(node_id)
    }

    /// 激活连接并自动连接
    pub fn active_connection(&mut self, connection_id: String, cx: &mut Context<Self>) {
        if !self.nodes.contains_key(&connection_id) {
            return;
        }

        self.selected_node = Some(connection_id.clone());
        self.expand_node(&connection_id, cx);
        self.connect_node(connection_id, cx);
    }

    /// 连接到 Redis 节点
    pub fn connect_node(&mut self, node_id: String, cx: &mut Context<Self>) {
        // 如果已经连接或正在加载，跳过
        if self.connected_nodes.contains(&node_id) || self.loading_nodes.contains(&node_id) {
            return;
        }

        let Some(connection) = self.stored_connections.get(&node_id).cloned() else {
            warn!(
                "{}",
                t!("RedisTree.connection_config_missing", node_id = node_id).to_string()
            );
            return;
        };

        info!(
            "{}",
            t!("RedisTree.connecting", name = connection.name).to_string()
        );

        // 标记为正在加载
        self.loading_nodes.insert(node_id.clone());
        self.error_nodes.remove(&node_id);
        cx.notify();

        let global_state = cx.global::<GlobalRedisState>().clone();
        let connection_id = connection.id;

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let config = match RedisManager::config_from_stored(&connection) {
                Ok(config) => config,
                Err(e) => {
                    let error_msg = e.to_string();
                    _ = this.update(cx, |view, cx| {
                        view.loading_nodes.remove(&node_id);
                        view.error_nodes.insert(node_id.clone(), error_msg);
                        cx.notify();
                    });
                    return;
                }
            };

            let connect_result = Tokio::spawn_result(cx, {
                let config = config.clone();
                let global_state = global_state.clone();
                async move {
                    global_state
                        .create_connection(config)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            match connect_result {
                Ok(_conn_id) => {
                    _ = this.update(cx, |view, cx| {
                        view.loading_nodes.remove(&node_id);
                        view.connected_nodes.insert(node_id.clone());
                        if let Some(conn_id) = connection_id {
                            cx.global_mut::<ActiveConnections>().add(conn_id);
                        }
                        cx.emit(RedisTreeViewEvent::ConnectionEstablished {
                            node_id: node_id.clone(),
                        });
                        // 自动加载数据库列表
                        view.load_databases(node_id, cx);
                    });
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    _ = this.update(cx, |view, cx| {
                        view.loading_nodes.remove(&node_id);
                        view.error_nodes.insert(node_id.clone(), error_msg);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// 加载数据库列表
    fn load_databases(&mut self, connection_id: String, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalRedisState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let databases = match Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let global_state = global_state.clone();
                async move {
                        let conn = global_state
                            .get_connection(&connection_id)
                            .ok_or_else(|| anyhow::anyhow!(t!("RedisTree.connection_missing")))?;
                    let guard = conn.read().await;
                    guard.get_databases_info().await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await {
                Ok(dbs) => dbs,
                Err(_) => return,
            };

            let db_nodes: Vec<RedisNode> = databases
                .into_iter()
                .map(|db| {
                    let node_id = format!("{}:db{}", connection_id, db.index);
                    let name = format!("db{}", db.index);
                    RedisNode::new(
                        node_id,
                        name,
                        RedisNodeType::Database(db.index),
                        connection_id.clone(),
                        db.index,
                    )
                    .with_key_count(db.keys)
                })
                .collect();

            _ = this.update(cx, |view, cx| {
                view.set_node_children(&connection_id, db_nodes, cx);
                view.expand_node(&connection_id, cx);
            });
        })
        .detach();
    }

    /// 添加连接节点（已连接状态）
    pub fn add_connection(&mut self, node: RedisNode, cx: &mut Context<Self>) {
        let node_id = node.id.clone();
        self.nodes.insert(node.id.clone(), node);
        self.connected_nodes.insert(node_id);
        self.rebuild_flat_entries();
        cx.notify();
    }

    /// 刷新节点
    pub fn refresh_node(&mut self, _node_id: &str, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// 清除节点的子节点加载状态，强制下次展开时重新加载
    pub fn clear_node_loaded(&mut self, node_id: &str, cx: &mut Context<Self>) {
        let child_ids: Vec<String> = self.nodes
            .get(node_id)
            .map(|node| {
                node.children
                    .iter()
                    .map(|child| child.id.clone())
                    .collect()
            })
            .unwrap_or_default();

        for child_id in child_ids {
            self.remove_node_recursive(&child_id);
        }

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.children_loaded = false;
            node.children.clear();
        }
        cx.notify();
    }

    /// 移除节点
    pub fn remove_node(&mut self, node_id: &str, cx: &mut Context<Self>) {
        let parent_id = self
            .nodes
            .get(node_id)
            .and_then(|node| Self::parent_node_id(node));

        if let Some(parent_id) = parent_id {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.children.retain(|child| child.id != node_id);
            }
        }

        self.remove_node_recursive(node_id);
        self.rebuild_flat_entries();
        cx.notify();
    }

    /// 关闭连接（断开但保留节点）
    pub fn disconnect_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        // 清除子节点
        let child_ids: Vec<String> = self.nodes
            .get(connection_id)
            .map(|node| {
                node.children
                    .iter()
                    .map(|child| child.id.clone())
                    .collect()
            })
            .unwrap_or_default();

        for child_id in child_ids {
            self.remove_node_recursive(&child_id);
        }

        if let Some(node) = self.nodes.get_mut(connection_id) {
            node.children.clear();
            node.children_loaded = false;
        }

        // 移除连接状态
        self.connected_nodes.remove(connection_id);
        self.expanded_nodes.remove(connection_id);
        self.error_nodes.remove(connection_id);
        if let Ok(conn_id) = connection_id.parse::<i64>() {
            cx.global_mut::<ActiveConnections>().remove(conn_id);
        }

        self.rebuild_flat_entries();
        cx.notify();
    }

    /// 完全移除连接（从树中删除）
    pub fn close_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let connection_nodes: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, node)| {
                node.connection_id == connection_id
                    && matches!(node.node_type, RedisNodeType::Connection)
            })
            .map(|(id, _)| id.clone())
            .collect();

        let nodes_to_remove = if connection_nodes.is_empty() {
            self.nodes
                .iter()
                .filter(|(_, node)| node.connection_id == connection_id)
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            connection_nodes
        };

        for node_id in nodes_to_remove {
            self.remove_node_recursive(&node_id);
            self.stored_connections.remove(&node_id);
            self.connected_nodes.remove(&node_id);
            self.error_nodes.remove(&node_id);
        }
        if let Ok(conn_id) = connection_id.parse::<i64>() {
            cx.global_mut::<ActiveConnections>().remove(conn_id);
        }

        self.rebuild_flat_entries();
        cx.notify();
    }

    fn remove_node_recursive(&mut self, node_id: &str) {
        let Some(node) = self.nodes.remove(node_id) else {
            return;
        };

        self.search_tokens.remove(node_id);
        self.scan_cursors.remove(node_id);
        self.scan_patterns.remove(node_id);
        if self.selected_node.as_ref() == Some(&node.id) {
            self.selected_node = None;
        }

        self.expanded_nodes.remove(&node.id);
        self.loading_nodes.remove(&node.id);
        self.error_nodes.remove(&node.id);

        for child in node.children {
            self.remove_node_recursive(&child.id);
        }
    }

    fn parent_node_id(node: &RedisNode) -> Option<String> {
        match node.node_type {
            RedisNodeType::Connection => None,
            RedisNodeType::Database(_) => Some(node.connection_id.clone()),
            RedisNodeType::Key(_) | RedisNodeType::Namespace => {
                Some(format!("{}:db{}", node.connection_id, node.db_index))
            }
            RedisNodeType::LoadMore => {
                Some(format!("{}:db{}", node.connection_id, node.db_index))
            }
        }
    }

    fn load_more_node_id(node_id: &str) -> String {
        format!("{}:load_more", node_id)
    }

    fn remove_load_more_child(children: &mut Vec<RedisNode>) {
        children.retain(|child| !matches!(child.node_type, RedisNodeType::LoadMore));
    }

    fn build_load_more_node(&self, node_id: &str, connection_id: &str, db_index: u8) -> RedisNode {
        RedisNode::new(
            Self::load_more_node_id(node_id),
            t!("RedisTree.load_more").to_string(),
            RedisNodeType::LoadMore,
            connection_id.to_string(),
            db_index,
        )
    }

    /// 更新 SCAN 状态
    pub fn set_scan_state(&mut self, node_id: &str, pattern: String, cursor: u64) {
        self.scan_patterns.insert(node_id.to_string(), pattern);
        self.scan_cursors.insert(node_id.to_string(), cursor);
    }

    /// 获取 SCAN 状态
    pub fn get_scan_state(&self, node_id: &str) -> Option<(String, u64)> {
        let pattern = self.scan_patterns.get(node_id)?.clone();
        let cursor = *self.scan_cursors.get(node_id).unwrap_or(&0);
        Some((pattern, cursor))
    }

    /// 应用键分页结果
    pub fn apply_key_page(
        &mut self,
        node_id: &str,
        key_nodes: Vec<RedisNode>,
        pattern: String,
        next_cursor: u64,
        append: bool,
        cx: &mut Context<Self>,
    ) {
        self.set_scan_state(node_id, pattern, next_cursor);
        if append {
            self.merge_node_children(node_id, key_nodes, cx);
        } else {
            self.set_node_children(node_id, key_nodes, cx);
        }

        // 搜索时展示匹配数量
        if !self.search_keyword.trim().is_empty() {
            if let Some(node) = self.nodes.get_mut(node_id) {
                let count = node
                    .children
                    .iter()
                    .filter(|child| matches!(child.node_type, RedisNodeType::Key(_)))
                    .count() as i64;
                node.key_count = Some(count);
            }
        }
        if self.search_keyword.trim().is_empty() {
            self.reset_db_key_count(node_id);
        }

        if next_cursor != 0 {
            if let Some(node) = self.nodes.get(node_id) {
                let load_more = self.build_load_more_node(node_id, &node.connection_id, node.db_index);
                self.append_node_children(node_id, vec![load_more], cx);
            }
        }
    }

    fn merge_node_children(
        &mut self,
        node_id: &str,
        new_children: Vec<RedisNode>,
        cx: &mut Context<Self>,
    ) {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return;
        };

        let mut existing = node.children.clone();
        Self::remove_load_more_child(&mut existing);

        fn merge_into(existing: &mut Vec<RedisNode>, incoming: RedisNode) {
            match incoming.node_type {
                RedisNodeType::Namespace => {
                    if let Some(target) = existing.iter_mut().find(|n| {
                        matches!(n.node_type, RedisNodeType::Namespace) && n.name == incoming.name
                    }) {
                        let mut merged_children = target.children.clone();
                        for child in incoming.children {
                            merge_into(&mut merged_children, child);
                        }
                        target.children = merged_children;
                        target.children_loaded = true;
                    } else {
                        existing.push(incoming);
                    }
                }
                RedisNodeType::Key(_) => {
                    let incoming_id = incoming.id.clone();
                    if !existing.iter().any(|n| n.id == incoming_id) {
                        existing.push(incoming);
                    }
                }
                RedisNodeType::LoadMore | RedisNodeType::Database(_) | RedisNodeType::Connection => {
                    // 不应出现在键子节点中，忽略
                }
            }
        }

        for child in new_children {
            merge_into(&mut existing, child);
        }

        node.children = existing;
        node.children_loaded = true;
        let children_snapshot = node.children.clone();
        let _ = node;

        for child in &children_snapshot {
            self.insert_node_recursive(child);
        }

        self.rebuild_flat_entries();
        cx.notify();
    }

    fn update_local_search_counts(&mut self) {
        let Some(keyword) = self.local_filter_keyword() else {
            for (node_id, total) in self.db_total_key_counts.iter() {
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.key_count = Some(*total);
                }
            }
            return;
        };

        let mut counts: HashMap<String, i64> = HashMap::new();
        for node in self.nodes.values() {
            if matches!(node.node_type, RedisNodeType::Key(_)) {
                if node.name.to_lowercase().contains(&keyword) {
                    let db_node_id = format!("{}:db{}", node.connection_id, node.db_index);
                    *counts.entry(db_node_id).or_insert(0) += 1;
                }
            }
        }

        for (node_id, total) in self.db_total_key_counts.iter() {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.key_count = Some(*counts.get(node_id).unwrap_or(&0));
            } else {
                let _ = total;
            }
        }
    }

    fn reset_db_key_count(&mut self, node_id: &str) {
        if let Some(total) = self.db_total_key_counts.get(node_id).copied() {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.key_count = Some(total);
            }
        }
    }

    /// 递增搜索令牌，用于丢弃过期的搜索结果
    pub fn bump_search_token(&mut self, node_id: &str) -> u64 {
        let next = self.search_tokens.get(node_id).copied().unwrap_or(0) + 1;
        self.search_tokens.insert(node_id.to_string(), next);
        next
    }

    /// 判断搜索令牌是否仍然有效
    pub fn is_search_token_current(&self, node_id: &str, token: u64) -> bool {
        self.search_tokens.get(node_id).copied().unwrap_or(0) == token
    }

    /// 设置节点加载状态
    pub fn set_node_loading(&mut self, node_id: &str, loading: bool, cx: &mut Context<Self>) {
        if loading {
            self.loading_nodes.insert(node_id.to_string());
        } else {
            self.loading_nodes.remove(node_id);
        }
        cx.notify();
    }

    /// 获取当前搜索令牌
    pub fn current_search_token(&self, node_id: &str) -> u64 {
        self.search_tokens.get(node_id).copied().unwrap_or(0)
    }

    /// 设置节点子项
    pub fn set_node_children(
        &mut self,
        node_id: &str,
        children: Vec<RedisNode>,
        cx: &mut Context<Self>,
    ) {
        let old_child_ids: Vec<String> = match self.nodes.get(node_id) {
            Some(node) => node
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect(),
            None => return,
        };

        for child_id in old_child_ids {
            self.remove_node_recursive(&child_id);
        }

        for child in &children {
            self.insert_node_recursive(child);
        }

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.children = children;
            node.children_loaded = true;
        }

        self.rebuild_flat_entries();
        cx.notify();
    }

    /// 追加节点子项（用于加载更多）
    pub fn append_node_children(
        &mut self,
        node_id: &str,
        children: Vec<RedisNode>,
        cx: &mut Context<Self>,
    ) {
        let Some(mut next_children) = self
            .nodes
            .get(node_id)
            .map(|node| node.children.clone())
        else {
            return;
        };

        Self::remove_load_more_child(&mut next_children);

        for child in &children {
            self.insert_node_recursive(child);
            next_children.push(child.clone());
        }

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.children = next_children;
            node.children_loaded = true;
        }

        self.rebuild_flat_entries();
        cx.notify();
    }

    fn insert_node_recursive(&mut self, node: &RedisNode) {
        if let RedisNodeType::Database(_) = node.node_type {
            if let Some(count) = node.key_count {
                self.db_total_key_counts.insert(node.id.clone(), count);
            }
        }

        for child in &node.children {
            self.insert_node_recursive(child);
        }

        self.nodes.insert(node.id.clone(), node.clone());
    }

    /// 展开节点
    pub fn expand_node(&mut self, node_id: &str, cx: &mut Context<Self>) {
        self.expanded_nodes.insert(node_id.to_string());
        self.rebuild_flat_entries();
        cx.notify();
    }

    /// 折叠节点
    fn collapse_node(&mut self, node_id: &str, cx: &mut Context<Self>) {
        self.expanded_nodes.remove(node_id);
        self.rebuild_flat_entries();
        cx.notify();
    }

    /// 切换节点展开状态
    fn toggle_node(&mut self, node_id: &str, cx: &mut Context<Self>) {
        if self.expanded_nodes.contains(node_id) {
            self.collapse_node(node_id, cx);
        } else {
            // 检查是否需要加载子节点
            if let Some(node) = self.nodes.get(node_id) {
                if node.is_expandable() && !node.children_loaded {
                    cx.emit(RedisTreeViewEvent::RefreshKeys {
                        node_id: node_id.to_string(),
                    });
                }
            }
            self.expand_node(node_id, cx);
        }
    }

    /// 处理双击事件
    fn handle_double_click(&mut self, node_id: &str, cx: &mut Context<Self>) {
        // 如果有错误，双击重试
        if self.error_nodes.contains_key(node_id) {
            self.error_nodes.remove(node_id);
            self.connect_node(node_id.to_string(), cx);
            return;
        }

        let Some(node) = self.nodes.get(node_id).cloned() else {
            return;
        };

        match node.node_type {
            RedisNodeType::Connection => {
                if !self.connected_nodes.contains(node_id) {
                    // 未连接，触发连接
                    self.connect_node(node_id.to_string(), cx);
                } else {
                    // 已连接，切换展开状态
                    self.toggle_node(node_id, cx);
                }
            }
            RedisNodeType::Database(_) | RedisNodeType::Namespace => {
                // 切换展开状态
                self.toggle_node(node_id, cx);
            }
            RedisNodeType::Key(_) => {
                // 键节点：发出选中事件
                cx.emit(RedisTreeViewEvent::KeySelected {
                    node_id: node_id.to_string(),
                });
            }
            RedisNodeType::LoadMore => {}
        }
    }

    /// 重建扁平化条目列表
    fn rebuild_flat_entries(&mut self) {
        self.flat_entries.clear();
        let mut match_cache: HashMap<String, bool> = HashMap::new();

        // 获取根节点（连接节点）
        let root_nodes: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, node)| matches!(node.node_type, RedisNodeType::Connection))
            .map(|(id, _)| id.clone())
            .collect();

        for root_id in root_nodes {
            self.add_node_entries(&root_id, 0, &mut match_cache);
        }
    }

    /// 递归添加节点条目
    fn add_node_entries(&mut self, node_id: &str, depth: usize, match_cache: &mut HashMap<String, bool>) {
        let filter_keyword = self.local_filter_keyword();
        let (_is_expandable, matches, child_ids, is_load_more) = match self.nodes.get(node_id) {
            Some(node) => {
                let is_load_more = matches!(node.node_type, RedisNodeType::LoadMore);
                let matches = filter_keyword
                    .as_ref()
                    .map(|keyword| node.name.to_lowercase().contains(keyword))
                    .unwrap_or(true);
                let child_ids = node
                    .children
                    .iter()
                    .map(|child| child.id.clone())
                    .collect::<Vec<_>>();
                (node.is_expandable(), matches, child_ids, is_load_more)
            }
            None => return,
        };

        // 搜索过滤
        if filter_keyword.is_some() {
            if !matches && !is_load_more && !self.node_has_matching_descendant(node_id, match_cache) {
                return;
            }
        }

        self.flat_entries.push(FlatEntry {
            node_id: node_id.to_string(),
            depth,
        });

        // 如果节点展开，添加子节点
        if self.expanded_nodes.contains(node_id) {
            for child_id in child_ids {
                self.add_node_entries(&child_id, depth + 1, match_cache);
            }
        }
    }

    fn node_has_matching_descendant(
        &self,
        node_id: &str,
        match_cache: &mut HashMap<String, bool>,
    ) -> bool {
        if let Some(cached) = match_cache.get(node_id) {
            return *cached;
        }

        let Some(node) = self.nodes.get(node_id) else {
            match_cache.insert(node_id.to_string(), false);
            return false;
        };

        let Some(keyword) = self.local_filter_keyword() else {
            match_cache.insert(node_id.to_string(), true);
            return true;
        };

        for child in &node.children {
            if child.name.to_lowercase().contains(&keyword) {
                match_cache.insert(node_id.to_string(), true);
                return true;
            }
            if self.node_has_matching_descendant(&child.id, match_cache) {
                match_cache.insert(node_id.to_string(), true);
                return true;
            }
        }

        match_cache.insert(node_id.to_string(), false);
        false
    }

    fn local_filter_keyword(&self) -> Option<String> {
        let keyword = self.search_keyword.trim();
        if keyword.is_empty() {
            return None;
        }
        if keyword.contains('*') || keyword.contains('?') || keyword.contains('[') {
            return None;
        }
        Some(keyword.to_lowercase())
    }

    fn get_node_icon(&self, node_type: &RedisNodeType) -> IconName {
        match node_type {
            RedisNodeType::Connection => IconName::Redis,
            RedisNodeType::Database(_) => IconName::Database,
            RedisNodeType::Namespace => IconName::FolderOpen1,
            RedisNodeType::Key(_) => IconName::Key,
            RedisNodeType::LoadMore => IconName::Ellipsis,
        }
    }

    /// 获取键类型的徽章文字和颜色
    fn get_key_type_badge(&self, key_type: &RedisKeyType) -> (&'static str, gpui::Hsla) {
        match key_type {
            RedisKeyType::String => ("S", gpui::hsla(0.33, 0.7, 0.45, 1.0)),  // 绿色
            RedisKeyType::List => ("L", gpui::hsla(0.08, 0.8, 0.55, 1.0)),    // 橙色
            RedisKeyType::Set => ("E", gpui::hsla(0.55, 0.7, 0.50, 1.0)),     // 青色
            RedisKeyType::ZSet => ("Z", gpui::hsla(0.75, 0.6, 0.55, 1.0)),    // 紫色
            RedisKeyType::Hash => ("H", gpui::hsla(0.0, 0.7, 0.55, 1.0)),     // 红色
            RedisKeyType::Stream => ("X", gpui::hsla(0.58, 0.6, 0.50, 1.0)),  // 蓝色
            RedisKeyType::None => ("?", gpui::hsla(0.0, 0.0, 0.50, 1.0)),     // 灰色
        }
    }

    /// 检查连接节点是否应该显示展开箭头
    fn should_show_arrow(&self, node_id: &str) -> bool {
        let Some(node) = self.nodes.get(node_id) else {
            return false;
        };

        match node.node_type {
            RedisNodeType::Connection => {
                // 未连接的连接节点不显示箭头
                self.connected_nodes.contains(node_id) && !node.children.is_empty()
            }
            RedisNodeType::Database(_) | RedisNodeType::Namespace => {
                node.children_loaded && !node.children.is_empty()
                    || !node.children_loaded
            }
            RedisNodeType::Key(_) | RedisNodeType::LoadMore => false,
        }
    }

    /// 获取当前选中的数据库信息
    pub fn get_selected_db_info(&self) -> Option<(u8, i64, i64)> {
        // 返回 (db_index, loaded_keys, total_keys)
        if let Some(ref selected) = self.selected_node {
            if let Some(node) = self.nodes.get(selected) {
                let db_index = node.db_index;
                // 统计当前数据库下的键数量
                let loaded_keys = self.nodes.values()
                    .filter(|n| {
                        n.db_index == db_index &&
                        n.connection_id == node.connection_id &&
                        matches!(n.node_type, RedisNodeType::Key(_))
                    })
                    .count() as i64;
                let total_keys = self.nodes.values()
                    .find(|n| {
                        n.connection_id == node.connection_id &&
                        matches!(n.node_type, RedisNodeType::Database(idx) if idx == db_index)
                    })
                    .and_then(|n| n.key_count)
                    .unwrap_or(loaded_keys);
                return Some((db_index, loaded_keys, total_keys));
            }
        }
        None
    }

    /// 获取当前选中的可刷新节点 ID（数据库或连接节点）
    fn get_selected_refreshable_node(&self) -> Option<String> {
        let selected = self.selected_node.as_ref()?;
        let node = self.nodes.get(selected)?;

        // 只有已连接的节点才能刷新
        if !self.connected_nodes.contains(&node.connection_id) {
            return None;
        }

        match &node.node_type {
            RedisNodeType::Database(_) | RedisNodeType::Connection => {
                Some(node.id.clone())
            }
            RedisNodeType::Key(_) | RedisNodeType::Namespace => {
                // 如果选中的是键，返回其所在数据库节点
                let db_node_id = format!("{}:db{}", node.connection_id, node.db_index);
                if self.nodes.contains_key(&db_node_id) {
                    Some(db_node_id)
                } else {
                    None
                }
            }
            RedisNodeType::LoadMore => None,
        }
    }

    /// 获取当前选中的数据库节点信息（用于新建键）
    fn get_selected_db_context(&self) -> Option<(String, u8)> {
        let selected = self.selected_node.as_ref()?;
        let node = self.nodes.get(selected)?;

        // 只有已连接的节点才能新建键
        if !self.connected_nodes.contains(&node.connection_id) {
            return None;
        }

        match &node.node_type {
            RedisNodeType::Database(db_index) => {
                Some((node.connection_id.clone(), *db_index))
            }
            RedisNodeType::Key(_) | RedisNodeType::Namespace => {
                Some((node.connection_id.clone(), node.db_index))
            }
            RedisNodeType::Connection | RedisNodeType::LoadMore => None,
        }
    }

    fn render_toolbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let _view_for_add = cx.entity().clone();
        let can_refresh = self.get_selected_refreshable_node().is_some();
        let _can_add = self.get_selected_db_context().is_some();
        let view_for_search = cx.entity().clone();

        h_flex()
            .w_full()
            .p_1()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&self.search_state)),
            )
            .child(
                Button::new("search")
                    .icon(IconName::Search)
                    .ghost()
                    .xsmall()
                    .tooltip(t!("RedisTree.search_help").to_string())
                    .on_click(move |_, _, cx| {
                        view_for_search.update(cx, |this, cx| {
                            this.search_keyword = this.search_state.read(cx).text().to_string();
                            if let Some(node_id) = this.get_selected_refreshable_node() {
                                if let Some(node) = this.nodes.get(&node_id) {
                                    if matches!(node.node_type, RedisNodeType::Database(_)) {
                                        let pattern = this.search_keyword.trim().to_string();
                                        if pattern.is_empty() {
                                            this.reset_db_key_count(&node_id);
                                            cx.emit(RedisTreeViewEvent::RefreshKeys { node_id });
                                        } else {
                                            cx.emit(RedisTreeViewEvent::SearchKeys { node_id, pattern });
                                        }
                                    }
                                }
                            }
                            cx.notify();
                        });
                    }),
            )
            .child(
                Button::new("refresh")
                    .icon(IconName::Refresh)
                    .ghost()
                    .xsmall()
                    .disabled(!can_refresh)
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| {
                            if let Some(node_id) = this.get_selected_refreshable_node() {
                                // 先清除该节点的子节点加载状态，强制重新加载
                                if let Some(node) = this.nodes.get_mut(&node_id) {
                                    node.children_loaded = false;
                                }
                                cx.emit(RedisTreeViewEvent::RefreshKeys { node_id });
                            }
                        });
                    }),
            )
    }

    fn render_item(&self, ix: usize, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(entry) = self.flat_entries.get(ix) else {
            return div().into_any_element();    
        };

        let Some(node) = self.nodes.get(&entry.node_id) else {
            return div().into_any_element();
        };

        let node_id = entry.node_id.clone();
        let is_selected = self.selected_node.as_ref() == Some(&node_id);
        let is_expanded = self.expanded_nodes.contains(&node_id);
        let is_connection = matches!(node.node_type, RedisNodeType::Connection);
        let is_connected = self.connected_nodes.contains(&node_id);
        let is_load_more = matches!(node.node_type, RedisNodeType::LoadMore);
        let is_loading = if is_load_more {
            let db_node_id = format!("{}:db{}", node.connection_id, node.db_index);
            self.loading_nodes.contains(&db_node_id)
        } else {
            self.loading_nodes.contains(&node_id)
        };
        let error_msg = self.error_nodes.get(&node_id).cloned();
        let is_key = matches!(node.node_type, RedisNodeType::Key(_));
        let key_type_badge = if let RedisNodeType::Key(key_type) = &node.node_type {
            Some(self.get_key_type_badge(key_type))
        } else {
            None
        };
        let depth = entry.depth;
        let icon = self.get_node_icon(&node.node_type).color();
        let name = node.name.clone();
        let key_count = node.key_count;
        let show_arrow = self.should_show_arrow(&node_id);
        let node_connection_id = node.connection_id.clone();
        let node_db_index = node.db_index;
        let view = cx.entity().clone();
        let view_for_delete = cx.entity().clone();
        let view_for_context = cx.entity().clone();
        let view_for_dbl = cx.entity().clone();
        let node_id_for_delete = node_id.clone();
        let node_id_for_context = node_id.clone();
        let node_id_for_dbl = node_id.clone();

        let view_for_arrow = cx.entity().clone();
        let node_id_for_arrow = entry.node_id.clone();

        h_flex()
            .id(SharedString::from(format!("redis-node-{}", ix)))
            .group("tree-item")
            .w_full()
            .h(px(28.0))
            .pl(px(8.0 + depth as f32 * 16.0))
            .pr(px(4.0))
            .gap_1()
            .items_center()
            .cursor_pointer()
            .rounded(px(4.0))
            .when(is_selected, |this| this.bg(cx.theme().list_active))
            .when(!is_selected, |this| {
                this.hover(|style| style.bg(cx.theme().list_hover))
            })
            // 单击选中，双击展开/连接
            .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                if is_load_more {
                    view_for_dbl.update(cx, |_view, cx| {
                        cx.emit(RedisTreeViewEvent::LoadMoreKeys {
                            node_id: format!("{}:db{}", node_connection_id, node_db_index),
                        });
                    });
                    return;
                }
                if event.click_count == 2 {
                    view_for_dbl.update(cx, |view, cx| {
                        view.handle_double_click(&node_id_for_dbl, cx);
                    });
                } else {
                    view.update(cx, |view, cx| {
                        view.selected_node = Some(node_id.clone());
                        cx.notify();

                        // 单击只选中，不展开
                        cx.emit(RedisTreeViewEvent::NodeSelected {
                            node_id: node_id.clone(),
                        });
                    });
                }
            })
            // 展开/折叠箭头
            .child(
                div()
                    .id(SharedString::from(format!("arrow-{}", ix)))
                    .w(px(16.0))
                    .h(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(show_arrow, |this| {
                        this
                            .cursor_pointer()
                            .on_click({
                                let view_for_arrow = view_for_arrow.clone();
                                let node_id_for_arrow = node_id_for_arrow.clone();
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    view_for_arrow.update(cx, |view, cx| {
                                        view.toggle_node(&node_id_for_arrow, cx);
                                    });
                                }
                            })
                            .child(
                                Icon::new(if is_expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .with_size(Size::XSmall)
                                .text_color(cx.theme().muted_foreground),
                            )
                    }),
            )
            // 类型徽章（仅对 Key 节点显示）
            .when_some(key_type_badge, |this, (badge_text, badge_color)| {
                this.child(
                    div()
                        .w(px(18.0))
                        .h(px(18.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.0))
                        .bg(badge_color)
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(gpui::white())
                        .child(badge_text),
                )
            })
            // 图标（非 Key 节点显示）
            .when(!is_key, |this| {
                this.child(
                    Icon::new(icon)
                        .with_size(Size::Small)
                        .when(is_connection && !is_connected && error_msg.is_none(), |icon| {
                            icon.text_color(cx.theme().muted_foreground)
                        })
                        .when(is_connection && is_connected, |icon| {
                            icon.text_color(cx.theme().success)
                        })
                        .when(!is_connection, |icon| {
                            icon.text_color(cx.theme().muted_foreground)
                        }),
                )
            })
            // 名称
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .truncate()
                    .when(is_connection && !is_connected && error_msg.is_none(), |el| {
                        el.text_color(cx.theme().muted_foreground)
                    })
                    .child(name),
            )
            // 加载中指示器
            .when(is_loading, |this| {
                this.child(
                    Spinner::new()
                        .with_size(Size::XSmall)
                        .color(cx.theme().muted_foreground),
                )
            })
            // 错误指示器
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
                                .p_2()
                                .max_w(px(300.0))
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
                                                        .with_size(Size::Small)
                                                        .text_color(cx.theme().warning),
                                                )
                                                .child(t!("RedisTree.connection_error").to_string()),
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
                                .into_any_element()
                        }),
                )
            })
            // 键数量（对于可展开节点）
            .when_some(key_count, |this: gpui::Stateful<gpui::Div>, count: i64| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("({})", count)),
                )
            })
            // 删除按钮（仅对 Key 节点，悬停显示）
            .when(is_key, |this| {
                this.child(
                    div()
                        .id(SharedString::from(format!("delete-btn-{}", ix)))
                        .w(px(20.0))
                        .h(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.0))
                        .invisible()
                        .group_hover("tree-item", |style| style.visible())
                        .hover(|style| style.bg(cx.theme().danger.opacity(0.1)))
                        .cursor_pointer()
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            view_for_delete.update(cx, |_view, cx| {
                                cx.emit(RedisTreeViewEvent::DeleteKey {
                                    node_id: node_id_for_delete.clone(),
                                });
                            });
                        })
                        .child(
                            Icon::new(IconName::Remove)
                                .with_size(Size::XSmall)
                                .text_color(cx.theme().danger),
                        ),
                )
            })
            // 右键菜单
            .context_menu({
                let view_for_context = view_for_context.clone();
                let node_id_for_context = node_id_for_context.clone();
                move |menu, window, cx| {
                    // 右键即选中
                    view_for_context.update(cx, |view, cx| {
                        view.selected_node = Some(node_id_for_context.clone());
                        cx.notify();
                    });
                    let is_connected = view_for_context.read(cx).connected_nodes.contains(&node_id_for_context);
                    let node = view_for_context.read(cx).nodes.get(&node_id_for_context).cloned();

                    if let Some(node) = node {
                        match &node.node_type {
                            RedisNodeType::Key(_) => {
                                // 键节点：显示"在新标签页中打开"菜单项
                                let view_for_open = view_for_context.clone();
                                let node_id_for_open = node_id_for_context.clone();
                                menu.item(
                                    PopupMenuItem::new(
                                        t!("RedisTree.menu_open_in_new_tab").to_string(),
                                    )
                                        .on_click(window.listener_for(&view_for_open, move |_view, _, _, cx| {
                                            cx.emit(RedisTreeViewEvent::OpenKeyInNewTab {
                                                node_id: node_id_for_open.clone(),
                                            });
                                        }))
                                )
                            }
                            RedisNodeType::Connection => {
                                if is_connected {
                                    // 已连接：打开 CLI、切换数据库、新建 Key、刷新、断开连接
                                    let view_for_cli = view_for_context.clone();
                                    let view_for_create = view_for_context.clone();
                                    let view_for_refresh = view_for_context.clone();
                                    let view_for_disconnect = view_for_context.clone();
                                    let connection_id_for_cli = node.connection_id.clone();
                                    let stored_for_cli = view_for_context
                                        .read(cx)
                                        .stored_connections
                                        .get(&node_id_for_context)
                                        .cloned();
                                    let node_id_for_create = node_id_for_context.clone();
                                    let node_id_for_refresh = node_id_for_context.clone();
                                    let node_id_for_disconnect = node_id_for_context.clone();
                                    menu.item(
                                        PopupMenuItem::new(
                                            t!("RedisTree.menu_open_cli").to_string(),
                                        )
                                            .on_click(window.listener_for(&view_for_cli, move |_view, _, _, cx| {
                                                if let Some(stored_connection) = stored_for_cli.clone() {
                                                    cx.emit(RedisTreeViewEvent::OpenCli {
                                                        connection_id: connection_id_for_cli.clone(),
                                                        db_index: 0,
                                                        stored_connection,
                                                    });
                                                }
                                            }))
                                    )
                                    .separator()
                                    .item(
                                        PopupMenuItem::new(
                                            t!("RedisTree.menu_create_key").to_string(),
                                        )
                                            .on_click(window.listener_for(&view_for_create, move |_view, _, _, cx| {
                                                cx.emit(RedisTreeViewEvent::CreateKey {
                                                    node_id: node_id_for_create.clone(),
                                                });
                                            }))
                                    )
                                    .separator()
                                    .item(
                                        PopupMenuItem::new(
                                            t!("Common.refresh").to_string(),
                                        )
                                            .on_click(window.listener_for(&view_for_refresh, move |view, _, _, cx| {
                                                if let Some(node) = view.nodes.get_mut(&node_id_for_refresh) {
                                                    node.children_loaded = false;
                                                }
                                                cx.emit(RedisTreeViewEvent::RefreshKeys {
                                                    node_id: node_id_for_refresh.clone(),
                                                });
                                            }))
                                    )
                                    .separator()
                                    .item(
                                        PopupMenuItem::new(
                                            t!("RedisTree.menu_disconnect").to_string(),
                                        )
                                            .on_click(window.listener_for(&view_for_disconnect, move |view, _, _, cx| {
                                                view.disconnect_connection(&node_id_for_disconnect, cx);
                                                cx.emit(RedisTreeViewEvent::CloseConnection {
                                                    node_id: node_id_for_disconnect.clone(),
                                                });
                                            }))
                                    )
                                } else {
                                    // 未连接：连接
                                    let view_for_connect = view_for_context.clone();
                                    let node_id_for_connect = node_id_for_context.clone();
                                    menu.item(
                                        PopupMenuItem::new(
                                            t!("RedisTree.menu_connect").to_string(),
                                        )
                                            .on_click(window.listener_for(&view_for_connect, move |view, _, _, cx| {
                                                view.connect_node(node_id_for_connect.clone(), cx);
                                            }))
                                    )
                                }
                            }
                            RedisNodeType::Database(_db_idx) => {
                                // 数据库节点：打开 CLI、新建 Key、刷新
                                let view_for_cli = view_for_context.clone();
                                let view_for_create = view_for_context.clone();
                                let view_for_refresh = view_for_context.clone();
                                let connection_id_for_cli = node.connection_id.clone();
                                let node_id_for_cli = node_id_for_context.clone();
                                let node_id_for_create = node_id_for_context.clone();
                                let node_id_for_refresh = node_id_for_context.clone();
                                let stored_for_cli = view_for_context
                                    .read(cx)
                                    .stored_connections
                                    .get(&node.connection_id)
                                    .cloned();
                                menu.item(
                                    PopupMenuItem::new(
                                        t!("RedisTree.menu_open_cli").to_string(),
                                    )
                                        .on_click(window.listener_for(&view_for_cli, move |view, _, _, cx| {
                                            if let Some(node) = view.nodes.get(&node_id_for_cli) {
                                                if let RedisNodeType::Database(db_index_for_cli) = node.node_type {
                                                    if let Some(stored_connection) = stored_for_cli.clone() {
                                                        cx.emit(RedisTreeViewEvent::OpenCli {
                                                            connection_id: connection_id_for_cli.clone(),
                                                            db_index: db_index_for_cli,
                                                            stored_connection,
                                                        });
                                                    }
                                                }
                                            }
                                        }))
                                )
                                .separator()
                                .item(
                                    PopupMenuItem::new(
                                        t!("RedisTree.menu_create_key").to_string(),
                                    )
                                        .on_click(window.listener_for(&view_for_create, move |_view, _, _, cx| {
                                            cx.emit(RedisTreeViewEvent::CreateKey {
                                                node_id: node_id_for_create.clone(),
                                            });
                                        }))
                                )
                                .separator()
                                .item(
                                    PopupMenuItem::new(
                                        t!("Common.refresh").to_string(),
                                    )
                                        .on_click(window.listener_for(&view_for_refresh, move |view, _, _, cx| {
                                            if let Some(node) = view.nodes.get_mut(&node_id_for_refresh) {
                                                node.children_loaded = false;
                                            }
                                            cx.emit(RedisTreeViewEvent::RefreshKeys {
                                                node_id: node_id_for_refresh.clone(),
                                            });
                                        }))
                                )
                            }
                            RedisNodeType::Namespace => {
                                // 命名空间节点：刷新
                                let view_for_refresh = view_for_context.clone();
                                let node_id_for_refresh = node_id_for_context.clone();
                                menu.item(
                                    PopupMenuItem::new(
                                        t!("Common.refresh").to_string(),
                                    )
                                        .on_click(window.listener_for(&view_for_refresh, move |view, _, _, cx| {
                                            // 刷新命名空间所在的数据库
                                            if let Some(node) = view.nodes.get(&node_id_for_refresh) {
                                                let db_node_id = format!("{}:db{}", node.connection_id, node.db_index);
                                                if let Some(db_node) = view.nodes.get_mut(&db_node_id) {
                                                    db_node.children_loaded = false;
                                                }
                                                cx.emit(RedisTreeViewEvent::RefreshKeys {
                                                    node_id: db_node_id,
                                                });
                                            }
                                        }))
                                )
                            }
                            RedisNodeType::LoadMore => menu,
                        }
                    } else {
                        menu
                    }
                }
            })
            .into_any_element()
    }
}

impl EventEmitter<RedisTreeViewEvent> for RedisTreeView {}

impl Focusable for RedisTreeView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RedisTreeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entry_count = self.flat_entries.len();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(window, cx))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(self.is_loading, |this| {
                        this.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Spinner::new()),
                        )
                    })
                    .when(!self.is_loading && entry_count == 0, |this| {
                        this.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("RedisTree.no_data").to_string()),
                        )
                    })
                    .when(!self.is_loading && entry_count > 0, |this| {
                        this.child(
                            uniform_list(
                                "redis-tree-list",
                                entry_count,
                                cx.processor(
                                    move |this: &mut Self, visible_range: std::ops::Range<usize>, window, cx| {
                                        visible_range
                                            .map(|ix| this.render_item(ix, window, cx))
                                            .collect()
                                    },
                                ),
                            )
                            .size_full()
                            .track_scroll(&self.scroll_handle),
                        )
                    }),
            )
    }
}
