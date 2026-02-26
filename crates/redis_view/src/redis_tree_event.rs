//! Redis 树形视图事件处理

use gpui::{px, App, AppContext, Context, Entity, EventEmitter, ParentElement, Styled, Subscription, Window};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::{notification::Notification, WindowExt};
use one_core::gpui_tokio::Tokio;
use rust_i18n::t;

use crate::create_key_dialog::CreateKeyDialog;
use crate::key_value_view::KeyValueView;
use crate::redis_cli_view::RedisCliView;
use crate::redis_tree_view::{RedisTreeView, RedisTreeViewEvent};
use crate::RedisManager;
use crate::{GlobalRedisState, RedisKeyType, RedisNode, RedisNodeType};
use one_core::tab_container::{TabContainer, TabItem};

const SCAN_BATCH_SIZE: usize = 500;
const SCAN_TARGET_KEYS: usize = 500;

/// Redis 事件处理器
pub struct RedisEventHandler {
    _tree_subscription: Subscription,
}

impl RedisEventHandler {
    /// 显示错误通知
    fn show_error(window: &mut Window, message: impl Into<String>, cx: &mut App) {
        window.push_notification(Notification::error(message.into()).autohide(true), cx);
    }

    pub fn new(
        tree_view: &Entity<RedisTreeView>,
        tab_container: Entity<TabContainer>,
        key_value_view: Entity<KeyValueView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tab_container_clone = tab_container.clone();
        let key_value_view_clone = key_value_view.clone();
        let global_state = cx.global::<GlobalRedisState>().clone();
        let tree_view_clone = tree_view.clone();

        let tree_subscription = cx.subscribe_in(
            tree_view,
            window,
            move |_handler, _tree, event, window, cx| {
                let global_state = global_state.clone();
                let tab_container = tab_container_clone.clone();
                let key_value_view = key_value_view_clone.clone();
                let tree_view = tree_view_clone.clone();

                let get_node = |node_id: &str, cx: &mut Context<Self>| -> Option<RedisNode> {
                    tree_view.read(cx).get_node(node_id).cloned()
                };

                match event {
                    RedisTreeViewEvent::NodeSelected { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_node_selected(node, key_value_view, global_state.clone(), window, cx);
                        }
                    }
                    RedisTreeViewEvent::KeySelected { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_key_selected(node, key_value_view, global_state.clone(), window, cx);
                        }
                    }
                    RedisTreeViewEvent::SearchKeys { node_id, pattern } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_search_keys(
                                node,
                                pattern.clone(),
                                tree_view.clone(),
                                global_state.clone(),
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::OpenKeyInNewTab { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_open_key_in_new_tab(node, tab_container.clone(), global_state.clone(), window, cx);
                        }
                    }
                    RedisTreeViewEvent::RefreshKeys { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_refresh_keys(node, tree_view.clone(), global_state.clone(), window, cx);
                        }
                    }
                    RedisTreeViewEvent::LoadMoreKeys { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_load_more_keys(
                                node,
                                tree_view.clone(),
                                global_state.clone(),
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::DeleteKey { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_delete_key(node, tree_view.clone(), global_state.clone(), window, cx);
                        }
                    }
                    RedisTreeViewEvent::CreateKey { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_create_key(node, tree_view.clone(), global_state.clone(), window, cx);
                        }
                    }
                    RedisTreeViewEvent::CloseConnection { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_close_connection(node, tree_view.clone(), global_state.clone(), window, cx);
                        }
                    }
                    RedisTreeViewEvent::ConnectionEstablished { .. } => {
                        // 连接建立事件，不需要特殊处理
                    }
                    RedisTreeViewEvent::OpenCli { connection_id, db_index, stored_connection } => {
                        Self::handle_open_cli(
                            connection_id.clone(),
                            *db_index,
                            stored_connection.clone(),
                            tab_container.clone(),
                            window,
                            cx,
                        );
                    }
                }
            },
        );

        Self {
            _tree_subscription: tree_subscription,
        }
    }

    /// 处理打开 CLI 事件
    fn handle_open_cli(
        connection_id: String,
        db_index: u8,
        stored_connection: one_core::storage::StoredConnection,
        tab_container: Entity<TabContainer>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        // 生成唯一的标签页 ID（仍使用原连接 ID 作为稳定标识）
        let tab_id = format!("redis-cli-{}-db{}", connection_id, db_index);
        let global_state = cx.global::<GlobalRedisState>().clone();
        let tab_container = tab_container.clone();

        // 为 CLI 创建独立连接
        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
            let mut config = match RedisManager::config_from_stored(&stored_connection) {
                Ok(config) => config,
                Err(e) => {
                    let error_message = e.to_string();
                    let _ = cx.update(|cx| {
                        if let Some(window) = cx.active_window() {
                            _ = window.update(cx, |_, window, cx| {
                                Self::show_error(window, error_message.clone(), cx);
                            });
                        }
                    });
                    return;
                }
            };

            let unique_id = format!(
                "cli-{}-{}",
                connection_id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            config.id = unique_id.clone();
            config.name = format!("{} (CLI)", stored_connection.name);

            let create_result = Tokio::spawn_result(cx, {
                let global_state = global_state.clone();
                let config = config.clone();
                async move { global_state.create_connection(config).await.map_err(|e| anyhow::anyhow!("{}", e)) }
            })
            .await;

            match create_result {
                Ok(_cli_conn_id) => {
                    let _ = cx.update(|cx| {
                        if let Some(window) = cx.active_window() {
                            _ = window.update(cx, |_, window, cx| {
                                tab_container.update(cx, |container, cx| {
                                    container.activate_or_add_tab_lazy(
                                        tab_id.clone(),
                                        |window, cx| {
                                            let cli_view = cx.new(|cx| {
                                                RedisCliView::new(unique_id.clone(), db_index, true, window, cx)
                                            });
                                            TabItem::new(
                                                format!("redis-cli-{}-db{}", connection_id, db_index),
                                                format!("CLI (db{})", db_index),
                                                cli_view,
                                            )
                                        },
                                        window,
                                        cx,
                                    );
                                });
                            });
                        }
                    });
                }
                Err(e) => {
                    let _ = cx.update(|cx| {
                        if let Some(window) = cx.active_window() {
                            _ = window.update(cx, |_, window, cx| {
                                Self::show_error(window, e.to_string(), cx);
                            });
                        }
                    });
                }
            }
        })
        .detach();
    }

    /// 处理节点选中事件
    fn handle_node_selected(
        node: RedisNode,
        key_value_view: Entity<KeyValueView>,
        _global_state: GlobalRedisState,
        _window: &mut Window,
        cx: &mut App,
    ) {
        match &node.node_type {
            RedisNodeType::Key(_) => {
                if let Some(full_key) = &node.full_key {
                    key_value_view.update(cx, |view, cx| {
                        view.load_key(node.connection_id.clone(), node.db_index, full_key.clone(), cx);
                    });
                }
            }
            _ => {}
        }
    }

    /// 处理键选中事件
    fn handle_key_selected(
        node: RedisNode,
        key_value_view: Entity<KeyValueView>,
        _global_state: GlobalRedisState,
        _window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(full_key) = &node.full_key {
            key_value_view.update(cx, |view, cx| {
                view.load_key(node.connection_id.clone(), node.db_index, full_key.clone(), cx);
            });
        }
    }

    /// 处理在新标签页中打开键事件
    fn handle_open_key_in_new_tab(
        node: RedisNode,
        tab_container: Entity<TabContainer>,
        _global_state: GlobalRedisState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(full_key) = node.full_key.clone() else {
            return;
        };

        // 生成唯一的标签页 ID
        let tab_id = format!("key-{}-db{}-{}", node.connection_id, node.db_index, full_key);
        let connection_id = node.connection_id.clone();
        let db_index = node.db_index;

        // 使用 activate_or_add_tab_lazy 方法：如果标签页存在则激活，否则创建新标签页
        tab_container.update(cx, |container, cx| {
            container.activate_or_add_tab_lazy(
                tab_id,
                |window, cx| {
                    // 创建新的 KeyValueView 并加载数据
                    let key_value_view = cx.new(|cx| KeyValueView::new_with_closeable(true, window, cx));
                    key_value_view.update(cx, |view, cx| {
                        view.load_key(connection_id.clone(), db_index, full_key.clone(), cx);
                    });
                    TabItem::new(
                        format!("key-{}-db{}-{}", connection_id, db_index, full_key),
                        full_key.clone(),
                        key_value_view,
                    )
                },
                window,
                cx,
            );
        });
    }

    /// 处理刷新键列表事件
    fn handle_refresh_keys(
        node: RedisNode,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        _window: &mut Window,
        cx: &mut App,
    ) {
        match &node.node_type {
            RedisNodeType::Connection => {
                // 加载数据库列表
                Self::load_databases(node.connection_id.clone(), tree_view, global_state, cx);
            }
            RedisNodeType::Database(db_index) => {
                // 加载该数据库的键列表
                let token = tree_view.update(cx, |tree, _| tree.bump_search_token(&node.id));
                Self::load_keys(
                    node.connection_id.clone(),
                    *db_index,
                    node.id.clone(),
                    "*".to_string(),
                    0,
                    false,
                    token,
                    tree_view,
                    global_state,
                    cx,
                );
            }
            _ => {
                // 其他节点类型暂不处理
            }
        }
    }

    /// 加载数据库列表
    fn load_databases(
        connection_id: String,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        cx: &mut App,
    ) {
        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
            let databases = match Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let global_state = global_state.clone();
                async move {
                    let conn = global_state
                        .get_connection(&connection_id)
                        .ok_or_else(|| anyhow::anyhow!(t!("RedisTree.connection_missing")))?;
                    let guard = conn.read().await;
                    guard
                        .get_databases_info()
                        .await
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

            _ = cx.update(|cx| {
                tree_view.update(cx, |tree, cx| {
                    tree.set_node_children(&connection_id, db_nodes, cx);
                });
            });
        })
        .detach();
    }

    /// 加载键列表
    fn load_keys(
        connection_id: String,
        db_index: u8,
        node_id: String,
        pattern: String,
        cursor: u64,
        append: bool,
        token: u64,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        cx: &mut App,
    ) {
        let node_id_for_loading = node_id.clone();
        tree_view.update(cx, |tree, cx| {
            tree.set_node_loading(&node_id_for_loading, true, cx);
        });

        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
            let pattern_for_scan = pattern.clone();
            let (keys, next_cursor) = match Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let global_state = global_state.clone();
                async move {
                    let conn = global_state
                        .get_connection(&connection_id)
                        .ok_or_else(|| anyhow::anyhow!(t!("RedisTree.connection_missing")))?;
                    let guard = conn.read().await;

                    // 使用 SCAN 扫描键（分页）
                    let mut all_keys = Vec::new();
                    let mut current_cursor = cursor;
                    loop {
                        let result = guard
                            .scan_in_db(db_index, current_cursor, &pattern_for_scan, SCAN_BATCH_SIZE)
                            .await
                            .map_err(|e| anyhow::anyhow!("{}", e))?;
                        all_keys.extend(result.keys);
                        current_cursor = if result.finished { 0 } else { result.cursor };
                        if current_cursor == 0 || all_keys.len() >= SCAN_TARGET_KEYS {
                            break;
                        }
                    }
                    let next_cursor = current_cursor;

                    Ok::<(Vec<String>, u64), anyhow::Error>((all_keys, next_cursor))
                }
            })
            .await {
                Ok(result) => result,
                Err(_) => {
                    let _ = cx.update(|cx| {
                        tree_view.update(cx, |tree, cx| {
                            tree.set_node_loading(&node_id, false, cx);
                        });
                    });
                    return;
                }
            };

            // 构建键节点（按 ":" 分割为树），类型异步加载
            let key_nodes = Self::build_namespace_tree(
                &connection_id,
                db_index,
                keys.iter().cloned().map(|key| (key, RedisKeyType::None)).collect(),
            );
            let keys_for_types = keys.clone();
            let connection_id_for_types = connection_id.clone();
            let node_id_for_types = node_id.clone();
            let tree_for_types = tree_view.clone();
            let global_for_types = global_state.clone();

            _ = cx.update(|cx| {
                tree_view.update(cx, |tree, cx| {
                    tree.set_node_loading(&node_id, false, cx);
                    if !tree.is_search_token_current(&node_id, token) {
                        return;
                    }
                    tree.apply_key_page(
                        &node_id,
                        key_nodes,
                        pattern.clone(),
                        next_cursor,
                        append,
                        cx,
                    );
                });
            });

            if !keys_for_types.is_empty() {
                cx.spawn(async move |cx: &mut gpui::AsyncApp| {
                    let types_result = match Tokio::spawn_result(cx, {
                        let global_state = global_for_types.clone();
                        let connection_id = connection_id_for_types.clone();
                        async move {
                            let conn = global_state
                                .get_connection(&connection_id)
                                .ok_or_else(|| anyhow::anyhow!(t!("RedisTree.connection_missing")))?;
                            let guard = conn.read().await;
                            Ok::<Vec<(String, RedisKeyType)>, anyhow::Error>(
                                guard
                                    .key_types_batch_in_db(db_index, &keys_for_types)
                                    .await
                                    .unwrap_or_else(|_| {
                                        keys_for_types
                                            .into_iter()
                                            .map(|key| (key, RedisKeyType::None))
                                            .collect()
                                    }),
                            )
                        }
                    })
                    .await {
                        Ok(types) => types,
                        Err(_) => return,
                    };

                    _ = cx.update(|cx| {
                        tree_for_types.update(cx, |tree, cx| {
                            if !tree.is_search_token_current(&node_id_for_types, token) {
                                return;
                            }
                            tree.update_key_types(&connection_id_for_types, db_index, types_result, cx);
                        });
                    });
                })
                .detach();
            }
        })
        .detach();
    }

    fn build_namespace_tree(
        connection_id: &str,
        db_index: u8,
        keys_with_types: Vec<(String, RedisKeyType)>,
    ) -> Vec<RedisNode> {
        #[derive(Default)]
        struct NsNode {
            name: String,
            children: Vec<NsEntry>,
        }

        enum NsEntry {
            Namespace(NsNode),
            Key { name: String, full_key: String, key_type: RedisKeyType },
        }

        fn insert_entry(entries: &mut Vec<NsEntry>, parts: &[&str], full_key: &str, key_type: RedisKeyType) {
            if parts.is_empty() {
                return;
            }
            if parts.len() == 1 {
                entries.push(NsEntry::Key {
                    name: parts[0].to_string(),
                    full_key: full_key.to_string(),
                    key_type,
                });
                return;
            }

            let name = parts[0];
            let mut index = None;
            for (i, entry) in entries.iter().enumerate() {
                if let NsEntry::Namespace(ns) = entry {
                    if ns.name == name {
                        index = Some(i);
                        break;
                    }
                }
            }

            let idx = if let Some(i) = index {
                i
            } else {
                entries.push(NsEntry::Namespace(NsNode {
                    name: name.to_string(),
                    children: Vec::new(),
                }));
                entries.len() - 1
            };

            if let NsEntry::Namespace(ns) = &mut entries[idx] {
                insert_entry(&mut ns.children, &parts[1..], full_key, key_type);
            }
        }

        fn build_nodes(
            entries: Vec<NsEntry>,
            connection_id: &str,
            db_index: u8,
            path: &str,
        ) -> Vec<RedisNode> {
            let mut nodes = Vec::new();
            for entry in entries {
                match entry {
                    NsEntry::Namespace(ns) => {
                        let next_path = if path.is_empty() {
                            ns.name.clone()
                        } else {
                            format!("{}:{}", path, ns.name)
                        };
                        let node_id = format!("{}:db{}:ns:{}", connection_id, db_index, next_path);
                        let children = build_nodes(ns.children, connection_id, db_index, &next_path);
                        let mut node = RedisNode::new(
                            node_id,
                            ns.name,
                            RedisNodeType::Namespace,
                            connection_id.to_string(),
                            db_index,
                        );
                        node.children = children;
                        node.children_loaded = true;
                        nodes.push(node);
                    }
                    NsEntry::Key { name, full_key, key_type } => {
                        let key_node_id = format!("{}:db{}:{}", connection_id, db_index, full_key);
                        let node = RedisNode::new(
                            key_node_id,
                            name,
                            RedisNodeType::Key(key_type),
                            connection_id.to_string(),
                            db_index,
                        )
                        .with_full_key(full_key);
                        nodes.push(node);
                    }
                }
            }
            nodes
        }

        let mut root_entries: Vec<NsEntry> = Vec::new();
        for (key, key_type) in keys_with_types {
            let parts: Vec<&str> = key.split(':').collect();
            insert_entry(&mut root_entries, &parts, &key, key_type);
        }

        build_nodes(root_entries, connection_id, db_index, "")
    }

    /// 处理搜索键事件（服务端查询）
    fn handle_search_keys(
        node: RedisNode,
        pattern: String,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        cx: &mut App,
    ) {
        let RedisNodeType::Database(db_index) = node.node_type else {
            return;
        };

        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            return;
        }

        let server_pattern = if trimmed.contains('*')
            || trimmed.contains('?')
            || trimmed.contains('[')
        {
            trimmed.to_string()
        } else {
            format!("*{}*", trimmed)
        };

        let token = tree_view.update(cx, |tree, _| tree.bump_search_token(&node.id));
        Self::load_keys(
            node.connection_id.clone(),
            db_index,
            node.id.clone(),
            server_pattern,
            0,
            false,
            token,
            tree_view,
            global_state,
            cx,
        );
    }

    /// 处理加载更多键事件
    fn handle_load_more_keys(
        node: RedisNode,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        cx: &mut App,
    ) {
        let RedisNodeType::Database(db_index) = node.node_type else {
            return;
        };

        let (pattern, cursor, token) = tree_view.read(cx).get_scan_state(&node.id)
            .map(|(pattern, cursor)| {
                let token = tree_view.read(cx).current_search_token(&node.id);
                (pattern, cursor, token)
            })
            .unwrap_or(("*".to_string(), 0, 0));

        if cursor == 0 {
            return;
        }

        Self::load_keys(
            node.connection_id.clone(),
            db_index,
            node.id.clone(),
            pattern,
            cursor,
            true,
            token,
            tree_view,
            global_state,
            cx,
        );
    }

    /// 处理删除键事件
    fn handle_delete_key(
        node: RedisNode,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::{v_flex, WindowExt};

        let Some(full_key) = node.full_key.clone() else {
            return;
        };

        let connection_id = node.connection_id.clone();
        let key_name = full_key.clone();
        let tree = tree_view.clone();
        let node_id = node.id.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let key = key_name.clone();
            let state = global_state.clone();
            let tree = tree.clone();
            let node_id = node_id.clone();

            dialog
                .overlay(false)
                .title(t!("RedisTree.confirm_delete_title").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!("RedisTree.confirm_delete_key", key = key).to_string(),
                        )
                        .child(t!("RedisTree.irreversible").to_string()),
                )
                .on_ok(move |_, _window, cx: &mut App| {
                    let conn_id = conn_id.clone();
                    let key = key.clone();
                    let state = state.clone();
                    let tree = tree.clone();
                    let node_id = node_id.clone();

                    // 使用 Tokio::spawn_result 在 Tokio 运行时中执行删除操作
                    let task = Tokio::spawn_result(cx, {
                        let conn_id = conn_id.clone();
                        let key = key.clone();
                        async move {
                            let conn = state
                                .get_connection(&conn_id)
                                .ok_or_else(|| anyhow::anyhow!(t!("RedisTree.connection_missing")))?;
                            let guard = conn.read().await;
                            guard.del(&[key.as_str()]).await
                                .map_err(|e| anyhow::anyhow!("{}", e))
                        }
                    });

                    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
                        match task.await {
                            Ok(_) => {
                                let _ = cx.update(|cx| {
                                    tree.update(cx, |view, cx| {
                                        view.remove_node(&node_id, cx);
                                    });
                                });
                            }
                            Err(e) => {
                                let _ = cx.update(|cx| {
                                    if let Some(window) = cx.active_window() {
                                        _ = window.update(cx, |_, window, cx| {
                                            Self::show_error(
                                                window,
                                                t!(
                                                    "RedisTree.delete_key_failed",
                                                    error = e
                                                )
                                                .to_string(),
                                                cx,
                                            );
                                        });
                                    }
                                });
                            }
                        }
                    })
                    .detach();
                    true
                })
        });
    }

    /// 处理创建键事件
    fn handle_create_key(
        node: RedisNode,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 获取数据库信息
        let (connection_id, db_index) = match &node.node_type {
            RedisNodeType::Database(idx) => (node.connection_id.clone(), *idx),
            RedisNodeType::Key(_) | RedisNodeType::Namespace => {
                (node.connection_id.clone(), node.db_index)
            }
            _ => return,
        };

        // 创建对话框 UI 状态
        let create_key_dialog = cx.new(|cx| CreateKeyDialog::new(db_index, window, cx));

        let tree = tree_view.clone();
        let state = global_state.clone();

        // 打开对话框
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let create_key_dialog_for_ok = create_key_dialog.clone();
            let conn_id = connection_id.clone();
            let tree_for_ok = tree.clone();
            let state_for_ok = state.clone();

            dialog
                .title(t!("RedisTree.create_key_title").to_string())
                .w(px(500.))
                .child(create_key_dialog.clone())
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.confirm").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx: &mut App| {
                    let form_data = create_key_dialog_for_ok.read(cx).form_data(cx);
                    let key = form_data.key;
                    if key.is_empty() {
                        window.push_notification(
                            Notification::error(t!("RedisTree.key_name_required").to_string())
                                .autohide(true),
                            cx,
                        );
                        return false;
                    }

                    // 异步创建键
                    Self::create_key_async(
                        conn_id.clone(),
                        db_index,
                        key,
                        form_data.key_type,
                        form_data.value,
                        form_data.hash_field,
                        form_data.zset_score,
                        form_data.ttl,
                        tree_for_ok.clone(),
                        state_for_ok.clone(),
                        cx,
                    );

                    window.close_dialog(cx);
                    true
                })
        });
    }

    /// 异步创建键
    fn create_key_async(
        connection_id: String,
        db_index: u8,
        key: String,
        key_type: RedisKeyType,
        value: String,
        hash_field: String,
        zset_score: f64,
        ttl: Option<i64>,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        cx: &mut App,
    ) {
        let node_id = format!("{}:db{}", connection_id, db_index);

        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
            let result = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let key = key.clone();
                let value = value.clone();
                let hash_field = hash_field.clone();
                async move {
                    let conn = global_state
                        .get_connection(&connection_id)
                        .ok_or_else(|| anyhow::anyhow!(t!("RedisTree.connection_missing")))?;
                    let guard = conn.read().await;

                    // 切换到目标数据库
                    guard.select(db_index).await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;

                    // 根据类型创建键
                    match key_type {
                        RedisKeyType::String => {
                            guard.set(&key, &value, ttl).await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                        }
                        RedisKeyType::List => {
                            let push_value = if value.is_empty() { "" } else { value.as_str() };
                            guard.rpush(&key, &[push_value]).await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard.expire(&key, seconds).await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        RedisKeyType::Set => {
                            if !value.is_empty() {
                                guard.sadd(&key, &[value.as_str()]).await
                                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                            }
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard.expire(&key, seconds).await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        RedisKeyType::ZSet => {
                            // 使用用户输入的分数
                            if !value.is_empty() {
                                guard.zadd(&key, &[(zset_score, value.as_str())]).await
                                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                            }
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard.expire(&key, seconds).await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        RedisKeyType::Hash => {
                            // 使用用户输入的字段名和值
                            let field = if hash_field.is_empty() { "field".to_string() } else { hash_field };
                            guard.hset(&key, &field, &value).await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard.expire(&key, seconds).await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                t!("RedisTree.unsupported_key_type")
                            ));
                        }
                    }

                    Ok::<String, anyhow::Error>(key)
                }
            })
            .await;

            match result {
                Ok(_key) => {
                    // 创建成功，刷新键列表
                    let _ = cx.update(|cx| {
                        tree_view.update(cx, |tree, cx| {
                            // 清除数据库节点的子节点加载状态，强制重新加载
                            tree.clear_node_loaded(&node_id, cx);
                            cx.emit(RedisTreeViewEvent::RefreshKeys { node_id: node_id.clone() });
                        });

                        if let Some(window) = cx.active_window() {
                            _ = window.update(cx, |_, window, cx| {
                                window.push_notification(
                                    Notification::success(
                                        t!("RedisTree.key_created").to_string(),
                                    )
                                    .autohide(true),
                                    cx,
                                );
                            });
                        }
                    });
                }
                Err(e) => {
                    let _ = cx.update(|cx| {
                        if let Some(window) = cx.active_window() {
                            _ = window.update(cx, |_, window, cx| {
                                window.push_notification(
                                    Notification::error(
                                        t!("RedisTree.create_key_failed", error = e).to_string(),
                                    )
                                    .autohide(true),
                                    cx,
                                );
                            });
                        }
                    });
                }
            }
        })
        .detach();
    }

    /// 处理关闭连接事件
    fn handle_close_connection(
        node: RedisNode,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::{v_flex, WindowExt};

        let connection_id = node.connection_id.clone();
        let connection_name = node.name.clone();
        let tree = tree_view.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let conn_id = connection_id.clone();
            let conn_name = connection_name.clone();
            let state = global_state.clone();
            let tree = tree.clone();

            dialog
                .overlay(false)
                .title(t!("RedisTree.confirm_disconnect_title").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!("RedisTree.confirm_disconnect", name = conn_name).to_string(),
                        )
                        .child(t!("RedisTree.disconnect_warning").to_string()),
                )
                .on_ok(move |_, _window, cx: &mut App| {
                    let conn_id = conn_id.clone();
                    let state = state.clone();
                    let tree = tree.clone();

                    // 使用 Tokio::spawn_result 在 Tokio 运行时中执行关闭连接操作
                    let task = Tokio::spawn_result(cx, {
                        let conn_id = conn_id.clone();
                        async move {
                            state.remove_connection(&conn_id).await
                                .map_err(|e| anyhow::anyhow!("{}", e))
                        }
                    });

                    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
                        match task.await {
                            Ok(_) => {
                                let _ = cx.update(|cx| {
                                    tree.update(cx, |view, cx| {
                                        view.disconnect_connection(&conn_id, cx);
                                    });
                                });
                            }
                            Err(e) => {
                                let _ = cx.update(|cx| {
                                    if let Some(window) = cx.active_window() {
                                        _ = window.update(cx, |_, window, cx| {
                                            Self::show_error(
                                                window,
                                                t!(
                                                    "RedisTree.disconnect_failed",
                                                    error = e
                                                )
                                                .to_string(),
                                                cx,
                                            );
                                        });
                                    }
                                });
                            }
                        }
                    })
                    .detach();
                    true
                })
        });
    }
}

impl EventEmitter<()> for RedisEventHandler {}
