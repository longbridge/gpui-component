//! MongoDB 树形视图

use std::collections::{HashMap, HashSet};

use gpui::{
    AnyElement, App, AppContext, AsyncApp, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    SharedString, Styled, Subscription, UniformListScrollHandle, Window, div,
    prelude::FluentBuilder, px, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, WindowExt as _,
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    notification::Notification,
    spinner::Spinner,
    v_flex,
};
use one_core::gpui_tokio::Tokio;
use one_core::storage::{ActiveConnections, StoredConnection};
use rust_i18n::t;
use tracing::{info, warn};

use crate::{GlobalMongoState, MongoManager, MongoNode, MongoNodeType};

/// 树形视图事件
#[derive(Clone, Debug)]
pub enum MongoTreeViewEvent {
    /// 集合选中
    CollectionSelected { node_id: String },
    /// 集合在新标签页打开
    CollectionOpenInTab { node_id: String },
    /// 连接建立
    ConnectionEstablished { node_id: String },
}

#[derive(Clone)]
struct FlatEntry {
    node_id: String,
    depth: usize,
}

/// MongoDB 树形视图
pub struct MongoTreeView {
    nodes: HashMap<String, MongoNode>,
    flat_entries: Vec<FlatEntry>,
    expanded_nodes: HashSet<String>,
    selected_node: Option<String>,
    loading_nodes: HashSet<String>,
    error_nodes: HashMap<String, String>,
    search_query: String,
    search_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
    connected_nodes: HashSet<String>,
    stored_connections: HashMap<String, StoredConnection>,
    connection_order: Vec<String>,
    scroll_handle: UniformListScrollHandle,
    focus_handle: FocusHandle,
}

impl MongoTreeView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = UniformListScrollHandle::new();
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoTree.search_connections").to_string())
                .clean_on_escape()
        });
        let search_sub = cx.subscribe(&search_input, |this, input, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let query = input.read(cx).text().to_string();
                this.update_search_query(query, cx);
            }
        });

        Self {
            nodes: HashMap::new(),
            flat_entries: Vec::new(),
            expanded_nodes: HashSet::new(),
            selected_node: None,
            loading_nodes: HashSet::new(),
            error_nodes: HashMap::new(),
            search_query: String::new(),
            search_input,
            _subscriptions: vec![search_sub],
            connected_nodes: HashSet::new(),
            stored_connections: HashMap::new(),
            connection_order: Vec::new(),
            scroll_handle,
            focus_handle,
        }
    }

    pub fn new_with_connections(
        connections: &[StoredConnection],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self::new(window, cx);
        for connection in connections {
            this.add_stored_connection(connection.clone(), cx);
        }
        this
    }

    pub fn add_stored_connection(&mut self, connection: StoredConnection, cx: &mut Context<Self>) {
        let connection_id = connection.id.map(|id| id.to_string()).unwrap_or_else(|| {
            format!(
                "temp-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
        });

        let node = MongoNode::new_connection(connection_id.clone(), connection.name.clone());
        self.nodes.insert(connection_id.clone(), node);
        self.stored_connections
            .insert(connection_id.clone(), connection);
        if !self.connection_order.contains(&connection_id) {
            self.connection_order.push(connection_id);
        }
        self.rebuild_flat_entries();
        cx.notify();
    }

    pub fn get_node(&self, node_id: &str) -> Option<&MongoNode> {
        self.nodes.get(node_id)
    }

    pub fn is_connected(&self, node_id: &str) -> bool {
        self.connected_nodes.contains(node_id)
    }

    pub fn active_connection(&mut self, connection_id: String, cx: &mut Context<Self>) {
        if !self.nodes.contains_key(&connection_id) {
            return;
        }

        self.selected_node = Some(connection_id.clone());
        self.expand_node(&connection_id);
        self.connect_node(connection_id, cx);
    }

    pub fn connect_node(&mut self, node_id: String, cx: &mut Context<Self>) {
        if self.connected_nodes.contains(&node_id) || self.loading_nodes.contains(&node_id) {
            return;
        }

        let Some(connection) = self.stored_connections.get(&node_id).cloned() else {
            warn!(
                "{}",
                t!("MongoTree.connection_config_not_found", node_id = node_id).to_string()
            );
            return;
        };

        info!(
            "{}",
            t!("MongoTree.connecting_mongo", name = connection.name).to_string()
        );

        self.loading_nodes.insert(node_id.clone());
        self.error_nodes.remove(&node_id);
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();
        let connection_id = connection.id;

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let config = match MongoManager::config_from_stored(&connection) {
                Ok(config) => config,
                Err(error) => {
                    let error_message = error.to_string();
                    _ = this.update(cx, |view, cx| {
                        view.loading_nodes.remove(&node_id);
                        view.error_nodes.insert(node_id.clone(), error_message);
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
                Ok(_) => {
                    _ = this.update(cx, |view, cx| {
                        view.loading_nodes.remove(&node_id);
                        view.connected_nodes.insert(node_id.clone());
                        view.expand_node(&node_id);
                        if let Some(connection_numeric_id) = connection_id {
                            cx.global_mut::<ActiveConnections>()
                                .add(connection_numeric_id);
                        }
                        cx.emit(MongoTreeViewEvent::ConnectionEstablished {
                            node_id: node_id.clone(),
                        });
                        cx.notify();
                        view.load_databases(node_id, cx);
                    });
                }
                Err(error) => {
                    let error_message = error.to_string();
                    _ = this.update(cx, |view, cx| {
                        view.loading_nodes.remove(&node_id);
                        view.error_nodes.insert(node_id.clone(), error_message);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn load_databases(&mut self, connection_id: String, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let database_names = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let global_state = global_state.clone();
                async move {
                    let connection = global_state
                        .get_connection(&connection_id)
                        .ok_or_else(|| anyhow::anyhow!(t!("MongoTree.connection_missing")))?;
                    let guard = connection.read().await;
                    guard
                        .list_databases()
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            match database_names {
                Ok(names) => {
                    _ = this.update(cx, |view, cx| {
                        let children: Vec<MongoNode> = names
                            .into_iter()
                            .map(|database_name| {
                                let node_id = format!("{}:db:{}", connection_id, database_name);
                                MongoNode::new_database(
                                    node_id,
                                    database_name.clone(),
                                    connection_id.clone(),
                                    database_name,
                                )
                            })
                            .collect();
                        view.set_node_children(&connection_id, children, cx);
                    });
                }
                Err(error) => {
                    let error_message = error.to_string();
                    _ = this.update(cx, |view, cx| {
                        view.error_nodes.insert(connection_id, error_message);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn load_collections(&mut self, database_node: MongoNode, cx: &mut Context<Self>) {
        let global_state = cx.global::<GlobalMongoState>().clone();
        let connection_id = database_node.connection_id.clone();
        let database_name = database_node.database_name.clone().unwrap_or_default();
        let node_id = database_node.id.clone();

        if database_name.is_empty() {
            return;
        }

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let collection_names = Tokio::spawn_result(cx, {
                let connection_id = connection_id.clone();
                let database_name = database_name.clone();
                let global_state = global_state.clone();
                async move {
                    let connection = global_state
                        .get_connection(&connection_id)
                        .ok_or_else(|| anyhow::anyhow!(t!("MongoTree.connection_missing")))?;
                    let guard = connection.read().await;
                    guard
                        .list_collections(&database_name)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            })
            .await;

            match collection_names {
                Ok(names) => {
                    _ = this.update(cx, |view, cx| {
                        let children: Vec<MongoNode> = names
                            .into_iter()
                            .map(|collection_name| {
                                let node_id = format!(
                                    "{}:db:{}:col:{}",
                                    connection_id, database_name, collection_name
                                );
                                MongoNode::new_collection(
                                    node_id,
                                    collection_name.clone(),
                                    connection_id.clone(),
                                    database_name.clone(),
                                    collection_name,
                                )
                            })
                            .collect();
                        view.set_node_children(&node_id, children, cx);
                    });
                }
                Err(error) => {
                    let error_message = error.to_string();
                    _ = this.update(cx, |view, cx| {
                        view.error_nodes.insert(node_id, error_message);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn set_node_children(
        &mut self,
        node_id: &str,
        children: Vec<MongoNode>,
        cx: &mut Context<Self>,
    ) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.set_children(children.clone());
        }

        for child in &children {
            self.nodes.insert(child.id.clone(), child.clone());
        }

        self.rebuild_flat_entries();
        cx.notify();
    }

    fn update_search_query(&mut self, query: String, cx: &mut Context<Self>) {
        if self.search_query == query {
            return;
        }

        self.search_query = query;
        self.rebuild_flat_entries();
        cx.notify();
    }

    fn expand_node(&mut self, node_id: &str) {
        self.expanded_nodes.insert(node_id.to_string());
        self.rebuild_flat_entries();
    }

    fn toggle_expand(&mut self, node_id: &str) {
        if self.expanded_nodes.contains(node_id) {
            self.expanded_nodes.remove(node_id);
        } else {
            self.expanded_nodes.insert(node_id.to_string());
        }
        self.rebuild_flat_entries();
    }

    fn toggle_node(&mut self, node_id: &str, cx: &mut Context<Self>) {
        if self.expanded_nodes.contains(node_id) {
            self.expanded_nodes.remove(node_id);
            self.rebuild_flat_entries();
            cx.notify();
            return;
        }

        let Some(node) = self.nodes.get(node_id).cloned() else {
            return;
        };

        match node.node_type {
            MongoNodeType::Connection => {
                if !self.connected_nodes.contains(node_id) {
                    self.connect_node(node_id.to_string(), cx);
                    return;
                }
                if !node.children_loaded {
                    self.load_databases(node_id.to_string(), cx);
                }
            }
            MongoNodeType::Database => {
                if !self.connected_nodes.contains(&node.connection_id) {
                    self.connect_node(node.connection_id.clone(), cx);
                    return;
                }
                if !node.children_loaded {
                    self.load_collections(node, cx);
                }
            }
            MongoNodeType::Collection => {}
        }

        self.expanded_nodes.insert(node_id.to_string());
        self.rebuild_flat_entries();
        cx.notify();
    }

    fn should_show_arrow(&self, node_id: &str) -> bool {
        let Some(node) = self.nodes.get(node_id) else {
            return false;
        };

        match node.node_type {
            MongoNodeType::Connection => {
                if !self.connected_nodes.contains(node_id) {
                    return false;
                }
                !node.children_loaded || !node.children.is_empty()
            }
            MongoNodeType::Database => !node.children_loaded || !node.children.is_empty(),
            MongoNodeType::Collection => false,
        }
    }

    fn rebuild_flat_entries(&mut self) {
        self.flat_entries.clear();

        let query = self.search_query.trim().to_lowercase();
        let is_filtering = !query.is_empty();
        let connection_ids = self.connection_order.clone();
        for node_id in connection_ids {
            if self.nodes.contains_key(&node_id) {
                if is_filtering {
                    let entries = self.build_filtered_entries(&node_id, 0, &query);
                    self.flat_entries.extend(entries);
                } else {
                    self.append_flat_entries(&node_id, 0);
                }
            }
        }
    }

    fn append_flat_entries(&mut self, node_id: &str, depth: usize) {
        self.flat_entries.push(FlatEntry {
            node_id: node_id.to_string(),
            depth,
        });

        if !self.expanded_nodes.contains(node_id) {
            return;
        }

        let children = self
            .nodes
            .get(node_id)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        for child in children {
            self.append_flat_entries(&child.id, depth + 1);
        }
    }

    fn build_filtered_entries(&self, node_id: &str, depth: usize, query: &str) -> Vec<FlatEntry> {
        let Some(node) = self.nodes.get(node_id) else {
            return Vec::new();
        };

        let mut matches = node.name.to_string().to_lowercase().contains(query);
        let mut child_entries = Vec::new();

        for child in &node.children {
            let entries = self.build_filtered_entries(&child.id, depth + 1, query);
            if !entries.is_empty() {
                matches = true;
                child_entries.extend(entries);
            }
        }

        if matches {
            let mut entries = Vec::with_capacity(1 + child_entries.len());
            entries.push(FlatEntry {
                node_id: node_id.to_string(),
                depth,
            });
            entries.extend(child_entries);
            entries
        } else {
            Vec::new()
        }
    }

    fn handle_click(&mut self, node_id: String, cx: &mut Context<Self>) {
        self.selected_node = Some(node_id.clone());

        let Some(node) = self.nodes.get(&node_id).cloned() else {
            cx.notify();
            return;
        };

        match node.node_type {
            MongoNodeType::Connection => {
                if !self.connected_nodes.contains(&node_id) {
                    self.connect_node(node_id, cx);
                } else {
                    self.toggle_expand(&node.id);
                    cx.notify();
                }
            }
            MongoNodeType::Database => {
                if !self.connected_nodes.contains(&node.connection_id) {
                    self.connect_node(node.connection_id.clone(), cx);
                    return;
                }

                let should_load = !node.children_loaded;
                self.toggle_expand(&node.id);
                cx.notify();
                if should_load {
                    self.load_collections(node, cx);
                }
            }
            MongoNodeType::Collection => {
                cx.emit(MongoTreeViewEvent::CollectionSelected { node_id });
            }
        }
    }

    fn handle_double_click(&mut self, node_id: &str, cx: &mut Context<Self>) {
        self.selected_node = Some(node_id.to_string());
        let Some(node) = self.nodes.get(node_id) else {
            cx.notify();
            return;
        };

        if node.node_type == MongoNodeType::Collection {
            cx.emit(MongoTreeViewEvent::CollectionOpenInTab {
                node_id: node_id.to_string(),
            });
        }
        cx.notify();
    }

    fn refresh_connection(&mut self, node_id: &str, cx: &mut Context<Self>) {
        if !self.connected_nodes.contains(node_id) {
            self.connect_node(node_id.to_string(), cx);
            return;
        }

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.children.clear();
            node.children_loaded = false;
        }

        self.expand_node(node_id);
        self.load_databases(node_id.to_string(), cx);
    }

    fn refresh_database_by_id(&mut self, node_id: &str, cx: &mut Context<Self>) {
        let Some(node) = self.nodes.get(node_id).cloned() else {
            return;
        };
        if node.node_type != MongoNodeType::Database {
            return;
        }

        if let Some(db_node) = self.nodes.get_mut(node_id) {
            db_node.children.clear();
            db_node.children_loaded = false;
        }

        self.expand_node(node_id);
        self.load_collections(node, cx);
    }

    fn open_create_collection_dialog(
        &mut self,
        node_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node) = self.nodes.get(&node_id).cloned() else {
            return;
        };
        if node.node_type != MongoNodeType::Database {
            return;
        }
        let Some(database_name) = node.database_name.clone() else {
            return;
        };

        let collection_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoTree.collection_name_placeholder").to_string())
                .clean_on_escape()
        });
        collection_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        let view = cx.entity().clone();
        let connection_id = node.connection_id.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = collection_input.clone();
            let view_for_ok = view.clone();
            let connection_id_for_ok = connection_id.clone();
            let database_name_for_ok = database_name.clone();

            dialog
                .title(t!("MongoTree.create_collection_title").to_string())
                .w(px(420.0))
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(t!("MongoTree.collection_name_label")))
                        .child(Input::new(&collection_input).w_full()),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.create").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let name = input_for_ok.read(cx).text().to_string();
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        window.push_notification(
                            Notification::warning(
                                t!("MongoTree.collection_name_required").to_string(),
                            )
                            .autohide(true),
                            cx,
                        );
                        return false;
                    }
                    view_for_ok.update(cx, |view, cx| {
                        view.submit_create_collection(
                            connection_id_for_ok.clone(),
                            database_name_for_ok.clone(),
                            name,
                            cx,
                        );
                    });
                    window.close_dialog(cx);
                    false
                })
        });
    }

    fn open_create_database_dialog(
        &mut self,
        connection_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let database_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoTree.database_name_placeholder").to_string())
                .clean_on_escape()
        });
        let collection_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoTree.initial_collection_placeholder").to_string())
                .clean_on_escape()
        });

        database_input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        let view = cx.entity().clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let db_input_for_ok = database_input.clone();
            let collection_input_for_ok = collection_input.clone();
            let view_for_ok = view.clone();
            let connection_id_for_ok = connection_id.clone();

            dialog
                .title(t!("MongoTree.create_database_title").to_string())
                .w(px(460.0))
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_sm().child(t!("MongoTree.database_name_label")))
                        .child(Input::new(&database_input).w_full())
                        .child(
                            div()
                                .text_sm()
                                .child(t!("MongoTree.initial_collection_label")),
                        )
                        .child(Input::new(&collection_input).w_full()),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.create").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let database_name = db_input_for_ok.read(cx).text().to_string();
                    let database_name = database_name.trim().to_string();
                    let collection_name = collection_input_for_ok.read(cx).text().to_string();
                    let collection_name = collection_name.trim().to_string();
                    if database_name.is_empty() || collection_name.is_empty() {
                        window.push_notification(
                            Notification::warning(
                                t!("MongoTree.database_and_collection_required").to_string(),
                            )
                            .autohide(true),
                            cx,
                        );
                        return false;
                    }
                    view_for_ok.update(cx, |view, cx| {
                        view.submit_create_database(
                            connection_id_for_ok.clone(),
                            database_name,
                            collection_name,
                            cx,
                        );
                    });
                    window.close_dialog(cx);
                    false
                })
        });
    }

    fn open_drop_database_dialog(
        &mut self,
        node_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node) = self.nodes.get(&node_id).cloned() else {
            return;
        };
        if node.node_type != MongoNodeType::Database {
            return;
        }
        let Some(database_name) = node.database_name.clone() else {
            return;
        };

        let view = cx.entity().clone();
        let connection_id = node.connection_id.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let view_for_ok = view.clone();
            let connection_id_for_ok = connection_id.clone();
            let database_name_for_ok = database_name.clone();

            dialog
                .title(t!("MongoTree.delete_database_title").to_string())
                .confirm()
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            t!("MongoTree.confirm_delete_database", name = database_name)
                                .to_string(),
                        )
                        .child(t!("Common.irreversible").to_string()),
                )
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.delete").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    view_for_ok.update(cx, |view, cx| {
                        view.submit_drop_database(
                            connection_id_for_ok.clone(),
                            database_name_for_ok.clone(),
                            cx,
                        );
                    });
                    window.close_dialog(cx);
                    false
                })
        });
    }

    fn open_connection_info_dialog(
        &mut self,
        connection_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(stored) = self.stored_connections.get(&connection_id).cloned() else {
            Self::notify_error(t!("MongoTree.connection_info_not_found").as_ref(), cx);
            return;
        };
        let config = match MongoManager::config_from_stored(&stored) {
            Ok(config) => config,
            Err(error) => {
                Self::notify_error(
                    &t!("MongoTree.read_connection_info_failed", error = error).to_string(),
                    cx,
                );
                return;
            }
        };
        let params = match stored.to_mongodb_params() {
            Ok(params) => params,
            Err(error) => {
                Self::notify_error(
                    &t!("MongoTree.parse_connection_params_failed", error = error).to_string(),
                    cx,
                );
                return;
            }
        };
        let params_json =
            serde_json::to_string_pretty(&params).unwrap_or_else(|_| "{}".to_string());

        let connection_name = stored.name.clone();
        let connection_string = config.connection_string.clone();
        let connection_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).multi_line(true).auto_grow(2, 4);
            state.set_value(connection_string.clone(), window, cx);
            state
        });
        let params_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(10)
                .soft_wrap(true);
            state.set_value(params_json.clone(), window, cx);
            state
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let connection_input = connection_input.clone();
            let params_input = params_input.clone();
            let connection_string = connection_string.clone();

            dialog
                .title(t!("MongoTree.connection_info_title").to_string())
                .w(px(720.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            div().text_sm().child(
                                t!("MongoTree.connection_name_display", name = connection_name)
                                    .to_string(),
                            ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("MongoTree.connection_string_label")),
                                )
                                .child(Input::new(&connection_input).w_full().disabled(true)),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(t!("MongoTree.connection_params_label")),
                                )
                                .child(Input::new(&params_input).w_full().disabled(true)),
                        ),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("MongoTree.copy_connection_string").to_string())
                        .cancel_text(t!("Common.close").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(connection_string.clone()));
                    window.push_notification(
                        Notification::success(t!("MongoTree.connection_string_copied").to_string())
                            .autohide(true),
                        cx,
                    );
                    window.close_dialog(cx);
                    false
                })
        });
    }

    fn submit_create_collection(
        &mut self,
        connection_id: String,
        database_name: String,
        collection_name: String,
        cx: &mut Context<Self>,
    ) {
        let global_state = cx.global::<GlobalMongoState>().clone();
        let database_node_id = format!("{}:db:{}", connection_id, database_name);

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state
                    .get_connection(&connection_id)
                    .ok_or_else(|| anyhow::anyhow!(t!("MongoTree.connection_missing")))?;
                let guard = connection.read().await;
                guard
                    .create_collection(&database_name, &collection_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| match result {
                Ok(_) => {
                    view.refresh_database_by_id(&database_node_id, cx);
                    Self::notify_success(t!("MongoTree.collection_created").as_ref(), cx);
                }
                Err(error) => {
                    Self::notify_error(
                        &t!("MongoTree.create_collection_failed", error = error).to_string(),
                        cx,
                    );
                }
            });
        })
        .detach();
    }

    fn submit_create_database(
        &mut self,
        connection_id: String,
        database_name: String,
        collection_name: String,
        cx: &mut Context<Self>,
    ) {
        let global_state = cx.global::<GlobalMongoState>().clone();
        let connection_id_for_task = connection_id.clone();
        let connection_id_for_update = connection_id.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state
                    .get_connection(&connection_id_for_task)
                    .ok_or_else(|| anyhow::anyhow!(t!("MongoTree.connection_missing")))?;
                let guard = connection.read().await;
                guard
                    .create_collection(&database_name, &collection_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| match result {
                Ok(_) => {
                    view.refresh_connection(&connection_id_for_update, cx);
                    Self::notify_success(t!("MongoTree.database_created").as_ref(), cx);
                }
                Err(error) => {
                    Self::notify_error(
                        &t!("MongoTree.create_database_failed", error = error).to_string(),
                        cx,
                    );
                }
            });
        })
        .detach();
    }

    fn submit_drop_database(
        &mut self,
        connection_id: String,
        database_name: String,
        cx: &mut Context<Self>,
    ) {
        let global_state = cx.global::<GlobalMongoState>().clone();
        let connection_id_for_task = connection_id.clone();
        let connection_id_for_update = connection_id.clone();
        let database_name_for_task = database_name.clone();
        let database_name_for_update = database_name.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state
                    .get_connection(&connection_id_for_task)
                    .ok_or_else(|| anyhow::anyhow!(t!("MongoTree.connection_missing")))?;
                let guard = connection.read().await;
                guard
                    .drop_database(&database_name_for_task)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| match result {
                Ok(_) => {
                    view.refresh_connection(&connection_id_for_update, cx);
                    let database_node_id = format!(
                        "{}:db:{}",
                        connection_id_for_update, database_name_for_update
                    );
                    if view.selected_node.as_deref() == Some(database_node_id.as_str()) {
                        view.selected_node = None;
                    }
                    Self::notify_success(t!("MongoTree.database_deleted").as_ref(), cx);
                }
                Err(error) => {
                    Self::notify_error(
                        &t!("MongoTree.delete_database_failed", error = error).to_string(),
                        cx,
                    );
                }
            });
        })
        .detach();
    }

    fn disconnect_connection(&mut self, connection_id: String, cx: &mut Context<Self>) {
        if !self.connected_nodes.contains(&connection_id) {
            return;
        }
        let global_state = cx.global::<GlobalMongoState>().clone();
        let connection_id_for_task = connection_id.clone();
        let connection_id_for_update = connection_id.clone();
        self.loading_nodes.insert(connection_id.clone());
        cx.notify();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                global_state
                    .remove_connection(&connection_id_for_task)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.loading_nodes.remove(&connection_id_for_update);
                match result {
                    Ok(_) => {
                        view.clear_connection_state(&connection_id_for_update, cx);
                        Self::notify_success(t!("MongoTree.connection_disconnected").as_ref(), cx);
                    }
                    Err(error) => {
                        Self::notify_error(
                            &t!("MongoTree.disconnect_failed", error = error).to_string(),
                            cx,
                        );
                    }
                }
            });
        })
        .detach();
    }

    fn clear_connection_state(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let child_ids: Vec<String> = self
            .nodes
            .get(connection_id)
            .map(|node| node.children.iter().map(|child| child.id.clone()).collect())
            .unwrap_or_default();

        for child_id in child_ids {
            self.remove_node_recursive(&child_id);
        }

        if let Some(node) = self.nodes.get_mut(connection_id) {
            node.children.clear();
            node.children_loaded = false;
        }

        self.connected_nodes.remove(connection_id);
        self.expanded_nodes.remove(connection_id);
        self.error_nodes.remove(connection_id);
        self.loading_nodes.remove(connection_id);
        if let Some(selected_id) = self.selected_node.clone() {
            if selected_id == connection_id
                || selected_id.starts_with(&format!("{}:", connection_id))
            {
                self.selected_node = None;
            }
        }
        if let Ok(conn_id) = connection_id.parse::<i64>() {
            cx.global_mut::<ActiveConnections>().remove(conn_id);
        }

        self.rebuild_flat_entries();
        cx.notify();
    }

    fn remove_node_recursive(&mut self, node_id: &str) {
        let child_ids: Vec<String> = self
            .nodes
            .get(node_id)
            .map(|node| node.children.iter().map(|child| child.id.clone()).collect())
            .unwrap_or_default();

        for child_id in child_ids {
            self.remove_node_recursive(&child_id);
        }

        self.nodes.remove(node_id);
        self.expanded_nodes.remove(node_id);
        self.loading_nodes.remove(node_id);
        self.error_nodes.remove(node_id);
    }

    fn notify_error(message: &str, cx: &mut Context<Self>) {
        if let Some(window) = cx.active_window() {
            let message = message.to_string();
            let _ = window.update(cx, |_, window, cx| {
                window.push_notification(Notification::error(message).autohide(true), cx);
            });
        }
    }

    fn notify_success(message: &str, cx: &mut Context<Self>) {
        if let Some(window) = cx.active_window() {
            let message = message.to_string();
            let _ = window.update(cx, |_, window, cx| {
                window.push_notification(Notification::success(message).autohide(true), cx);
            });
        }
    }

    fn notify_info(message: &str, cx: &mut Context<Self>) {
        if let Some(window) = cx.active_window() {
            let message = message.to_string();
            let _ = window.update(cx, |_, window, cx| {
                window.push_notification(Notification::info(message).autohide(true), cx);
            });
        }
    }

    fn open_collection(&mut self, node_id: &str, cx: &mut Context<Self>) {
        if let Some(node) = self.nodes.get(node_id) {
            if node.node_type == MongoNodeType::Collection {
                cx.emit(MongoTreeViewEvent::CollectionSelected {
                    node_id: node_id.to_string(),
                });
            }
        }
    }

    fn open_collection_in_tab(&mut self, node_id: &str, cx: &mut Context<Self>) {
        if let Some(node) = self.nodes.get(node_id) {
            if node.node_type == MongoNodeType::Collection {
                cx.emit(MongoTreeViewEvent::CollectionOpenInTab {
                    node_id: node_id.to_string(),
                });
            }
        }
    }

    fn render_item(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entry = match self.flat_entries.get(index).cloned() {
            Some(entry) => entry,
            None => return div().into_any_element(),
        };

        let node_id = entry.node_id.clone();
        let node = match self.nodes.get(&node_id).cloned() {
            Some(node) => node,
            None => return div().into_any_element(),
        };

        let is_selected = self.selected_node.as_deref() == Some(node_id.as_str());
        let is_loading = self.loading_nodes.contains(&node_id);
        let error_message = self.error_nodes.get(&node_id).cloned();
        let is_expanded = self.expanded_nodes.contains(&node_id);
        let show_arrow = self.should_show_arrow(&node_id);

        let icon = match node.node_type {
            MongoNodeType::Connection => IconName::MongoDB,
            MongoNodeType::Database => IconName::Database,
            MongoNodeType::Collection => IconName::Table,
        }
        .color();

        let indent = px((entry.depth as f32) * 12.0);
        let click_node_id = node_id.clone();
        let view_for_click = cx.entity().clone();
        let view_for_double_click = cx.entity().clone();
        let node_id_for_double_click = node_id.clone();
        let view_for_context = cx.entity().clone();
        let node_id_for_context = node_id.clone();
        let view_for_arrow = cx.entity().clone();
        let node_id_for_arrow = node_id.clone();

        h_flex()
            .id(SharedString::from(format!("mongo-node-{}", node_id)))
            .gap_2()
            .items_center()
            .w_full()
            .min_h(px(28.0))
            .px_2()
            .py_1()
            .cursor_pointer()
            .when(is_selected, |this| this.bg(cx.theme().list_active))
            .when(!is_selected, |this| this.text_color(cx.theme().foreground))
            .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                if event.click_count == 2 {
                    view_for_double_click.update(cx, |view, cx| {
                        view.handle_double_click(&node_id_for_double_click, cx);
                    });
                } else {
                    view_for_click.update(cx, |view, cx| {
                        view.handle_click(click_node_id.clone(), cx);
                    });
                }
            })
            .child(div().w(indent))
            .child(
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(show_arrow, |this| {
                        this.cursor_pointer()
                            .on_mouse_down(MouseButton::Left, {
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
            .child(Icon::new(icon).with_size(Size::Small))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(node.name),
            )
            .when(is_loading, |this| {
                this.child(Spinner::new().with_size(Size::Small))
            })
            .when_some(error_message, |this, error| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(error))
            })
            .context_menu({
                move |menu, window, cx| {
                    Self::build_context_menu(
                        menu,
                        &view_for_context,
                        &node_id_for_context,
                        window,
                        cx,
                    )
                }
            })
            .into_any_element()
    }

    fn build_context_menu(
        mut menu: PopupMenu,
        view: &Entity<Self>,
        node_id: &str,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let Some(node) = view.read(cx).nodes.get(node_id).cloned() else {
            return menu;
        };

        match node.node_type {
            MongoNodeType::Connection => {
                let is_connected = view.read(cx).connected_nodes.contains(node_id);
                let view_for_action = view.clone();
                let node_id_for_action = node_id.to_string();
                if !is_connected {
                    menu = menu.item(
                        PopupMenuItem::new(t!("MongoTree.menu_connect").to_string()).on_click(
                            window.listener_for(&view_for_action, move |view, _, _, cx| {
                                view.connect_node(node_id_for_action.clone(), cx);
                            }),
                        ),
                    );
                } else {
                    menu = menu.item(
                        PopupMenuItem::new(t!("MongoTree.menu_refresh_databases").to_string())
                            .on_click(window.listener_for(
                                &view_for_action,
                                move |view, _, _, cx| {
                                    view.refresh_connection(&node_id_for_action, cx);
                                },
                            )),
                    );
                }
            }
            MongoNodeType::Database => {
                let connection_id = node.connection_id.clone();
                let view_for_create_collection = view.clone();
                let node_id_for_create_collection = node_id.to_string();
                let view_for_create_database = view.clone();
                let connection_id_for_create_database = connection_id.clone();
                let view_for_drop_database = view.clone();
                let node_id_for_drop_database = node_id.to_string();
                let view_for_shell = view.clone();
                let view_for_metrics = view.clone();
                let view_for_connection_info = view.clone();
                let connection_id_for_connection_info = connection_id.clone();
                let view_for_refresh = view.clone();
                let connection_id_for_refresh = connection_id.clone();
                let view_for_disconnect = view.clone();
                let connection_id_for_disconnect = connection_id.clone();

                menu = menu
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_create_collection").to_string())
                            .on_click(window.listener_for(
                                &view_for_create_collection,
                                move |view, _, window, cx| {
                                    view.open_create_collection_dialog(
                                        node_id_for_create_collection.clone(),
                                        window,
                                        cx,
                                    );
                                },
                            )),
                    )
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_create_database").to_string())
                            .on_click(window.listener_for(
                                &view_for_create_database,
                                move |view, _, window, cx| {
                                    view.open_create_database_dialog(
                                        connection_id_for_create_database.clone(),
                                        window,
                                        cx,
                                    );
                                },
                            )),
                    )
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_delete_database").to_string())
                            .on_click(window.listener_for(
                                &view_for_drop_database,
                                move |view, _, window, cx| {
                                    view.open_drop_database_dialog(
                                        node_id_for_drop_database.clone(),
                                        window,
                                        cx,
                                    );
                                },
                            )),
                    )
                    .separator()
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_open_shell").to_string()).on_click(
                            window.listener_for(&view_for_shell, move |_, _, _, cx| {
                                Self::notify_info(
                                    t!("MongoTree.shell_not_implemented").as_ref(),
                                    cx,
                                );
                            }),
                        ),
                    )
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_show_metrics").to_string()).on_click(
                            window.listener_for(&view_for_metrics, move |_, _, _, cx| {
                                Self::notify_info(
                                    t!("MongoTree.metrics_not_implemented").as_ref(),
                                    cx,
                                );
                            }),
                        ),
                    )
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_show_connection_info").to_string())
                            .on_click(window.listener_for(
                                &view_for_connection_info,
                                move |view, _, window, cx| {
                                    view.open_connection_info_dialog(
                                        connection_id_for_connection_info.clone(),
                                        window,
                                        cx,
                                    );
                                },
                            )),
                    )
                    .separator()
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_refresh_databases").to_string())
                            .on_click(window.listener_for(
                                &view_for_refresh,
                                move |view, _, _, cx| {
                                    view.refresh_connection(&connection_id_for_refresh, cx);
                                },
                            )),
                    )
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_disconnect").to_string()).on_click(
                            window.listener_for(&view_for_disconnect, move |view, _, _, cx| {
                                view.disconnect_connection(
                                    connection_id_for_disconnect.clone(),
                                    cx,
                                );
                            }),
                        ),
                    );
            }
            MongoNodeType::Collection => {
                let view_for_action = view.clone();
                let node_id_for_action = node_id.to_string();
                let view_for_tab = view.clone();
                let node_id_for_tab = node_id.to_string();
                menu = menu
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_open_collection").to_string())
                            .on_click(window.listener_for(
                                &view_for_action,
                                move |view, _, _, cx| {
                                    view.open_collection(&node_id_for_action, cx);
                                },
                            )),
                    )
                    .item(
                        PopupMenuItem::new(t!("MongoTree.menu_open_in_new_tab").to_string())
                            .on_click(window.listener_for(&view_for_tab, move |view, _, _, cx| {
                                view.open_collection_in_tab(&node_id_for_tab, cx);
                            })),
                    );
            }
        }

        menu
    }
}

impl Focusable for MongoTreeView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<MongoTreeViewEvent> for MongoTreeView {}

impl Render for MongoTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entry_count = self.flat_entries.len();
        let connection_count = self.connection_order.len();
        let is_filtering = !self.search_query.trim().is_empty();
        let empty_message = if is_filtering {
            t!("MongoTree.no_matching_connections").to_string()
        } else {
            t!("MongoTree.no_connections").to_string()
        };

        v_flex()
            .id("mongo-tree-view")
            .size_full()
            .bg(cx.theme().sidebar)
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex().items_center().justify_between().child(
                            h_flex()
                                .items_center()
                                .gap_1()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("MongoTree.connections_header").to_string())
                                .child(format!("({})", connection_count)),
                        ),
                    )
                    .child(
                        Input::new(&self.search_input)
                            .prefix(
                                Icon::new(IconName::Search).text_color(cx.theme().muted_foreground),
                            )
                            .cleanable(true)
                            .small()
                            .w_full(),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .size_full()
                    .when(entry_count == 0, |this| {
                        this.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child(empty_message),
                        )
                    })
                    .when(entry_count > 0, |this| {
                        this.child(
                            uniform_list(
                                "mongo-tree-list",
                                entry_count,
                                cx.processor(
                                    move |this: &mut Self,
                                          visible_range: std::ops::Range<usize>,
                                          window,
                                          cx| {
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
