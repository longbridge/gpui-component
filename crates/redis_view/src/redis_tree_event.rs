//! Redis 树形视图事件处理

use crate::RedisManager;
use crate::create_key_dialog::CreateKeyDialog;
use crate::key_value_view::KeyValueView;
use crate::redis_cli_view::RedisCliView;
use crate::redis_tree_view::{RedisTreeView, RedisTreeViewEvent};
use crate::{GlobalRedisState, RedisKeyType, RedisNode, RedisNodeType};
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, ParentElement, Styled, Subscription, Window, px,
};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::{WindowExt, notification::Notification};
use one_core::gpui_tokio::Tokio;
use one_core::tab_container::{TabContainer, TabItem};
use rust_i18n::t;

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
                            Self::handle_node_selected(
                                node,
                                key_value_view,
                                global_state.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::KeySelected { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_key_selected(
                                node,
                                key_value_view,
                                global_state.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::SearchKeys { node_id, pattern } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_search_keys(node, pattern.clone(), tree_view.clone(), cx);
                        }
                    }
                    RedisTreeViewEvent::OpenKeyInNewTab { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_open_key_in_new_tab(
                                node,
                                tab_container.clone(),
                                global_state.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::DeleteKey { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_delete_key(
                                node,
                                tree_view.clone(),
                                global_state.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::CreateKey { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_create_key(
                                node,
                                tree_view.clone(),
                                global_state.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::CloseConnection { node_id } => {
                        if let Some(node) = get_node(node_id, cx) {
                            Self::handle_close_connection(
                                node,
                                tree_view.clone(),
                                global_state.clone(),
                                window,
                                cx,
                            );
                        }
                    }
                    RedisTreeViewEvent::ConnectionEstablished { .. } => {
                        // 连接建立事件，不需要特殊处理
                    }
                    RedisTreeViewEvent::OpenCli {
                        connection_id,
                        db_index,
                        stored_connection,
                    } => {
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
                async move {
                    global_state
                        .create_connection(config)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
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
                                                RedisCliView::new(
                                                    unique_id.clone(),
                                                    db_index,
                                                    true,
                                                    window,
                                                    cx,
                                                )
                                            });
                                            TabItem::new(
                                                format!(
                                                    "redis-cli-{}-db{}",
                                                    connection_id, db_index
                                                ),
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
                        view.load_key(
                            node.connection_id.clone(),
                            node.db_index,
                            full_key.clone(),
                            cx,
                        );
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
                view.load_key(
                    node.connection_id.clone(),
                    node.db_index,
                    full_key.clone(),
                    cx,
                );
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
        let tab_id = format!(
            "key-{}-db{}-{}",
            node.connection_id, node.db_index, full_key
        );
        let connection_id = node.connection_id.clone();
        let db_index = node.db_index;

        // 使用 activate_or_add_tab_lazy 方法：如果标签页存在则激活，否则创建新标签页
        tab_container.update(cx, |container, cx| {
            container.activate_or_add_tab_lazy(
                tab_id,
                |window, cx| {
                    // 创建新的 KeyValueView 并加载数据
                    let key_value_view =
                        cx.new(|cx| KeyValueView::new_with_closeable(true, window, cx));
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

    /// 处理搜索键事件（服务端查询）
    fn handle_search_keys(
        node: RedisNode,
        pattern: String,
        tree_view: Entity<RedisTreeView>,
        cx: &mut App,
    ) {
        tree_view.update(cx, |tree, cx| {
            tree.search_keys(node, pattern, cx);
        });
    }

    /// 处理删除键事件
    fn handle_delete_key(
        node: RedisNode,
        tree_view: Entity<RedisTreeView>,
        global_state: GlobalRedisState,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_component::{WindowExt, v_flex};

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
                        .child(t!("RedisTree.confirm_delete_key", key = key).to_string())
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
                            let conn = state.get_connection(&conn_id).ok_or_else(|| {
                                anyhow::anyhow!(t!("RedisTree.connection_missing"))
                            })?;
                            let guard = conn.read().await;
                            guard
                                .del(&[key.as_str()])
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))
                        }
                    });

                    cx.spawn(async move |cx: &mut gpui::AsyncApp| match task.await {
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
                                            t!("RedisTree.delete_key_failed", error = e)
                                                .to_string(),
                                            cx,
                                        );
                                    });
                                }
                            });
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
                    guard
                        .select(db_index)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;

                    // 根据类型创建键
                    match key_type {
                        RedisKeyType::String => {
                            guard
                                .set(&key, &value, ttl)
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                        }
                        RedisKeyType::List => {
                            let push_value = if value.is_empty() { "" } else { value.as_str() };
                            guard
                                .rpush(&key, &[push_value])
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard
                                        .expire(&key, seconds)
                                        .await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        RedisKeyType::Set => {
                            if !value.is_empty() {
                                guard
                                    .sadd(&key, &[value.as_str()])
                                    .await
                                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                            }
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard
                                        .expire(&key, seconds)
                                        .await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        RedisKeyType::ZSet => {
                            // 使用用户输入的分数
                            if !value.is_empty() {
                                guard
                                    .zadd(&key, &[(zset_score, value.as_str())])
                                    .await
                                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                            }
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard
                                        .expire(&key, seconds)
                                        .await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        RedisKeyType::Hash => {
                            // 使用用户输入的字段名和值
                            let field = if hash_field.is_empty() {
                                "field".to_string()
                            } else {
                                hash_field
                            };
                            guard
                                .hset(&key, &field, &value)
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))?;
                            if let Some(seconds) = ttl {
                                if seconds > 0 {
                                    guard
                                        .expire(&key, seconds)
                                        .await
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                }
                            }
                        }
                        _ => {
                            return Err(anyhow::anyhow!(t!("RedisTree.unsupported_key_type")));
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
                            tree.refresh_keys(node_id.clone(), cx);
                        });

                        if let Some(window) = cx.active_window() {
                            _ = window.update(cx, |_, window, cx| {
                                window.push_notification(
                                    Notification::success(t!("RedisTree.key_created").to_string())
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
        use gpui_component::{WindowExt, v_flex};

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
                        .child(t!("RedisTree.confirm_disconnect", name = conn_name).to_string())
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
                            state
                                .remove_connection(&conn_id)
                                .await
                                .map_err(|e| anyhow::anyhow!("{}", e))
                        }
                    });

                    cx.spawn(async move |cx: &mut gpui::AsyncApp| match task.await {
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
                                            t!("RedisTree.disconnect_failed", error = e)
                                                .to_string(),
                                            cx,
                                        );
                                    });
                                }
                            });
                        }
                    })
                    .detach();
                    true
                })
        });
    }
}

impl EventEmitter<()> for RedisEventHandler {}
