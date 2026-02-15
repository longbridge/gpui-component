//! Redis 树形视图事件处理

use gpui::{px, App, AppContext, Context, Entity, EventEmitter, ParentElement, Styled, Subscription, Window};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::{notification::Notification, WindowExt};
use one_core::gpui_tokio::Tokio;

use crate::create_key_dialog::CreateKeyDialog;
use crate::key_value_view::KeyValueView;
use crate::redis_cli_view::RedisCliView;
use crate::redis_tree_view::{RedisTreeView, RedisTreeViewEvent};
use crate::{GlobalRedisState, RedisKeyType, RedisNode, RedisNodeType};
use one_core::tab_container::{TabContainer, TabItem};

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
                    RedisTreeViewEvent::OpenCli { connection_id, db_index } => {
                        Self::handle_open_cli(connection_id.clone(), *db_index, tab_container.clone(), window, cx);
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
        tab_container: Entity<TabContainer>,
        window: &mut Window,
        cx: &mut App,
    ) {
        // 生成唯一的标签页 ID
        let tab_id = format!("redis-cli-{}-db{}", connection_id, db_index);

        // 使用 activate_or_add_tab_lazy 方法：如果标签页存在则激活，否则创建新标签页
        tab_container.update(cx, |container, cx| {
            container.activate_or_add_tab_lazy(
                tab_id,
                |window, cx| {
                    let cli_view = cx.new(|cx| {
                        RedisCliView::new(connection_id.clone(), db_index, window, cx)
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
                    let key_value_view = cx.new(|cx| KeyValueView::new(window, cx));
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
                Self::load_keys(
                    node.connection_id.clone(),
                    *db_index,
                    node.id.clone(),
                    "*".to_string(),
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
                        .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
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
                    let name = format!("db{} ({})", db.index, db.keys);
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
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        cx: &mut App,
    ) {
        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
            let keys_with_types = match Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let global_state = global_state.clone();
                async move {
                    let conn = global_state
                        .get_connection(&connection_id)
                        .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                    let guard = conn.read().await;

                    // 切换到目标数据库
                    guard.select(db_index).await.map_err(|e| anyhow::anyhow!("{}", e))?;

                    // 使用 SCAN 扫描键（最多 1000 个）
                    let mut all_keys = Vec::new();
                    let mut cursor = 0u64;
                    let max_keys = 1000;

                    loop {
                        let result = guard
                            .scan(cursor, &pattern, 100)
                            .await
                            .map_err(|e| anyhow::anyhow!("{}", e))?;

                        all_keys.extend(result.keys);

                        if result.finished || all_keys.len() >= max_keys {
                            break;
                        }
                        cursor = result.cursor;
                    }

                    all_keys.truncate(max_keys);

                    // 批量获取每个键的类型
                    let mut keys_with_types = Vec::with_capacity(all_keys.len());
                    for key in all_keys {
                        let key_type = guard.key_type(&key).await.unwrap_or(RedisKeyType::None);
                        keys_with_types.push((key, key_type));
                    }

                    Ok::<Vec<(String, RedisKeyType)>, anyhow::Error>(keys_with_types)
                }
            })
            .await {
                Ok(keys) => keys,
                Err(_) => return,
            };

            // 构建键节点
            let key_nodes: Vec<RedisNode> = keys_with_types
                .into_iter()
                .map(|(key, key_type)| {
                    let key_node_id = format!("{}:db{}:{}", connection_id, db_index, key);
                    RedisNode::new(
                        key_node_id,
                        key.clone(),
                        RedisNodeType::Key(key_type),
                        connection_id.clone(),
                        db_index,
                    )
                    .with_full_key(key)
                })
                .collect();

            _ = cx.update(|cx| {
                tree_view.update(cx, |tree, cx| {
                    tree.set_node_children(&node_id, key_nodes, cx);
                });
            });
        })
        .detach();
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

        Self::load_keys(
            node.connection_id.clone(),
            db_index,
            node.id.clone(),
            server_pattern,
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
                .title("确认删除")
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(format!("确定要删除键 \"{}\" 吗？", key))
                        .child("此操作不可恢复。"),
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
                                .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
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
                                            Self::show_error(window, format!("删除键失败: {}", e), cx);
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
                .title("添加新键")
                .w(px(500.))
                .child(create_key_dialog.clone())
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("确认")
                        .cancel_text("取消"),
                )
                .on_ok(move |_, window, cx: &mut App| {
                    let form_data = create_key_dialog_for_ok.read(cx).form_data(cx);
                    let key = form_data.key;
                    if key.is_empty() {
                        window.push_notification(
                            Notification::error("键名不能为空").autohide(true),
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
                        .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
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
                            if !value.is_empty() {
                                guard.rpush(&key, &[value.as_str()]).await
                                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                            } else {
                                // 创建空列表需要先 push 再 pop
                                guard.rpush(&key, &[""]).await
                                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                                guard.execute_command(&format!("LPOP {}", key)).await
                                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                            }
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
                            return Err(anyhow::anyhow!("不支持的键类型"));
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
                                    Notification::success("键创建成功").autohide(true),
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
                                    Notification::error(format!("创建键失败: {}", e)).autohide(true),
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
                .title("确认关闭连接")
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(format!("确定要关闭连接 \"{}\" 吗？", conn_name))
                        .child("这将断开 Redis 连接并清理相关资源。"),
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
                                            Self::show_error(window, format!("关闭连接失败: {}", e), cx);
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
