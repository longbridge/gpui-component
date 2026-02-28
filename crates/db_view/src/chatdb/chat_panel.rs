//! Chat SQL Content - AI SQL 助手面板，支持 AI 对话和 SQL 执行
//!
//! 使用 AIInput 组件实现双模式（Agent/SQL）的智能输入
//! 支持 @表名 语法和智能工作流
//!
//! 所有 AI 请求统一通过 AgentDispatcher::dispatch() 路由：
//! - 有数据库连接 → SqlWorkflowAgent（自动处理选表/元数据/SQL生成）
//! - 无数据库连接 → GeneralChatAgent

use crate::chatdb::agents::{CAP_DB_METADATA, DatabaseMetadataProvider};
use crate::chatdb::ai_input::{AIInput, AIInputEvent};
use crate::chatdb::chat_markdown::SqlCodeBlock;
use crate::chatdb::chat_sql_block::SqlBlockResultState;
use crate::chatdb::chat_sql_result::ChatSqlResultView;
use crate::chatdb::components::{
    ChatMessageUI, ChatRole, MESSAGE_RENDER_LIMIT, MESSAGE_RENDER_STEP, MessageVariant,
    SqlBlockCacheExt,
};
use db::{GlobalDbState, is_query_statement_fallback};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px,
};
use gpui_component::button::ButtonVariants;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, WindowExt as _,
    button::Button,
    clipboard::Clipboard,
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputState},
    list::{List, ListState},
    scroll::Scrollbar,
    text::TextView,
    v_flex,
};
use one_core::agent::registry::AgentRegistry;
use one_core::agent::{AgentContext, AgentDispatcher, AgentEvent, SessionAffinity};
use one_core::cloud_sync::GlobalCloudUser;
use one_core::gpui_tokio::Tokio;
use one_core::llm::{
    Message, ProviderConfig, Role,
    chat_history::{ChatSession, MessageRepository},
    manager::GlobalProviderState,
    storage::ProviderRepository,
};
use one_core::storage::{DatabaseType, GlobalStorageState, traits::Repository};
use one_core::tab_container::{TabContent, TabContentEvent};
use rust_i18n::t;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
// 从核心库导入
use crate::chatdb::sql_query_detector::is_query_statement_for_connection;
use crate::sql_result_tab::SqlResultTabContainer;
use one_core::ai_chat::components::{
    ModelSettings, ProviderItem, SessionData, SessionListConfig, SessionListDelegate,
    SessionListHost,
};
use one_core::ai_chat::services::{SessionService, extract_session_name};

// ============================================================================
// 事件定义
// ============================================================================

#[derive(Clone, Debug)]
pub enum ChatPanelEvent {
    Close,
}

// ============================================================================
// ChatPanel 组件
// ============================================================================

pub struct ChatPanel {
    focus_handle: FocusHandle,

    // 输入组件
    ai_input: Entity<AIInput>,
    _input_subscription: Subscription,

    // 服务层
    session_service: SessionService,
    session_affinity: SessionAffinity,

    // 状态
    provider_id: Option<String>,
    selected_model: Option<String>,
    is_loading: bool,
    session_id: Option<i64>,

    // 消息和结果
    messages: Vec<ChatMessageUI>,
    chat_history: Vec<Message>,
    sql_result_views: HashMap<String, Entity<ChatSqlResultView>>,
    sql_block_results: HashMap<String, HashMap<usize, SqlBlockResultState>>,
    scroll_handle: ScrollHandle,
    history_sessions: Vec<ChatSession>,
    session_list: Option<Entity<ListState<SessionListDelegate<ChatPanel>>>>,
    storage_manager: one_core::storage::StorageManager,
    latest_ai_message_id: Option<String>,

    render_limit: usize,
    /// 用户是否在底部区域（用于判断是否可以裁剪渲染窗口）
    is_at_bottom: bool,
    /// 是否为新会话（用于在发送第一条消息时更新会话名称）
    is_new_session: bool,
    is_logged_in: bool,

    // 模型设置
    model_settings: ModelSettings,

    // 取消和重试
    cancel_token: Option<CancellationToken>,
    /// 上一次用户输入（用于简化重试）
    last_user_input: Option<String>,
}

impl ChatPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // 创建 AIInput 组件
        let ai_input = cx.new(|cx| AIInput::new(window, cx));

        // 订阅 AIInput 事件
        let input_subscription =
            cx.subscribe_in(&ai_input, window, |this, _input, event, window, cx| {
                match event {
                    AIInputEvent::Submit { content } => {
                        // 只有内容有效且未在 loading 时才设置 loading 状态
                        if !content.trim().is_empty() && !this.is_loading {
                            this.ai_input.update(cx, |input, cx| {
                                input.set_loading(true, window, cx);
                            });
                        }
                        this.send_message(content.clone(), cx);
                    }
                    AIInputEvent::ProviderChanged { provider_id } => {
                        this.provider_id = Some(provider_id.clone());
                    }
                    AIInputEvent::ModelChanged { model } => {
                        this.selected_model = Some(model.clone());
                    }
                    AIInputEvent::ExecuteSql {
                        sql,
                        connection_id,
                        database,
                        schema,
                    } => {
                        this.execute_sql(
                            sql.clone(),
                            connection_id.clone(),
                            database.clone(),
                            schema.clone(),
                            window,
                            cx,
                        );
                    }
                    AIInputEvent::ModeChanged { .. } => {
                        cx.notify();
                    }
                    AIInputEvent::SettingsChanged { settings } => {
                        this.model_settings = settings.clone();
                        cx.notify();
                    }
                    AIInputEvent::Cancel => {
                        this.cancel_current_operation(window, cx);
                    }
                }
            });

        let global_state = cx.global::<GlobalStorageState>();
        let storage_manager = global_state.storage.clone();
        let session_service = SessionService::new(storage_manager.clone());

        let mut instance = Self {
            focus_handle,
            ai_input,
            _input_subscription: input_subscription,
            session_service,
            session_affinity: SessionAffinity::new(),
            provider_id: None,
            selected_model: None,
            is_loading: false,
            session_id: None,
            messages: Vec::new(),
            chat_history: Vec::new(),
            sql_result_views: HashMap::new(),
            sql_block_results: HashMap::new(),
            scroll_handle: ScrollHandle::new(),
            history_sessions: Vec::new(),
            session_list: None,
            storage_manager,
            latest_ai_message_id: None,
            render_limit: MESSAGE_RENDER_LIMIT,
            is_at_bottom: true,
            is_new_session: false,
            is_logged_in: GlobalCloudUser::is_logged_in(cx),
            model_settings: ModelSettings::default(),
            cancel_token: None,
            last_user_input: None,
        };

        instance.load_providers(window, cx);
        instance.load_history_sessions(cx);
        instance
    }

    // ========================================================================
    // Provider 加载
    // ========================================================================

    fn load_providers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let is_logged_in = GlobalCloudUser::is_logged_in(cx);
        self.is_logged_in = is_logged_in;

        let repo = match self.storage_manager.get::<ProviderRepository>() {
            Some(repo) => repo,
            None => {
                tracing::error!("ProviderRepository not found");
                return;
            }
        };

        let mut providers = match repo.list() {
            Ok(all_providers) => all_providers
                .into_iter()
                .filter(|p| p.enabled)
                .collect::<Vec<_>>(),
            Err(e) => {
                tracing::error!("Failed to load providers: {}", e);
                Vec::new()
            }
        };

        if is_logged_in {
            if let Ok(provider) = repo.ensure_onetcli_provider() {
                if !providers.iter().any(|p| p.id == provider.id) {
                    providers.insert(0, provider);
                }
            }
        } else {
            providers.retain(|p| !p.is_builtin());
        }

        let items: Vec<ProviderItem> = providers.iter().map(ProviderItem::from_config).collect();
        if items.is_empty() {
            self.provider_id = None;
            self.selected_model = None;
        }
        // update_providers 会同步选择默认 provider 和模型，
        // 然后通过 AIInputEvent::ProviderChanged/ModelChanged 事件回调更新 chat_panel 的状态
        self.ai_input.update(cx, |input, cx| {
            input.update_providers(items, window, cx);
        });
    }

    // ========================================================================
    // 历史会话
    // ========================================================================

    pub fn start_new_session(&mut self, cx: &mut Context<Self>) {
        self.session_id = None;
        self.messages.clear();
        self.chat_history.clear();
        self.sql_result_views.clear();
        self.sql_block_results.clear();
        self.latest_ai_message_id = None;
        self.render_limit = MESSAGE_RENDER_LIMIT;
        self.session_affinity.reset();
        cx.notify();
    }

    fn update_session_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sessions_data: Vec<SessionData> = self
            .history_sessions
            .iter()
            .map(|s| SessionData::new(s.id, s.name.clone(), s.updated_at))
            .collect();
        let panel = cx.entity();

        if let Some(session_list) = &self.session_list {
            session_list.update(cx, |state, _| {
                let delegate = state.delegate_mut();
                delegate.update_sessions(sessions_data);
            });
        } else {
            self.session_list = Some(cx.new(|cx| {
                ListState::new(
                    SessionListDelegate::new(panel, sessions_data, SessionListConfig::default()),
                    window,
                    cx,
                )
                .searchable(true)
            }));
        }
    }

    fn load_history_sessions(&mut self, cx: &mut Context<Self>) {
        let session_service = self.session_service.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let sessions = match session_service.list_sessions() {
                Ok(s) => s,
                Err(_) => return,
            };

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    if let Some(window_id) = cx.active_window() {
                        let _ = cx.update_window(window_id, |_, window, cx| {
                            entity.update(cx, |this, cx| {
                                this.history_sessions = sessions;
                                this.update_session_list(window, cx);
                                cx.notify();
                            });
                        });
                    }
                });
            }
        })
        .detach();
    }

    fn delete_session(&mut self, session_id: i64, cx: &mut Context<Self>) {
        let storage_manager = self.storage_manager.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let delete_ok = {
                let message_repo = match storage_manager.get::<MessageRepository>() {
                    Some(r) => r,
                    None => return,
                };
                let _ = message_repo.delete_by_session(session_id);

                let session_service = SessionService::new(storage_manager);
                session_service.delete_session(session_id).is_ok()
            };

            if delete_ok {
                if let Some(entity) = this.upgrade() {
                    let _ = cx.update(|cx| {
                        entity.update(cx, |this, cx| {
                            if this.session_id == Some(session_id) {
                                this.session_id = None;
                                this.messages.clear();
                                this.chat_history.clear();
                            }
                            this.history_sessions.retain(|s| s.id != session_id);
                            let sessions_data: Vec<SessionData> = this
                                .history_sessions
                                .iter()
                                .map(|s| SessionData::new(s.id, s.name.clone(), s.updated_at))
                                .collect();
                            if let Some(session_list) = &this.session_list {
                                session_list.update(cx, |state, _| {
                                    state.delegate_mut().update_sessions(sessions_data);
                                });
                            }
                            cx.notify();
                        });
                    });
                }
            }
        })
        .detach();
    }

    fn start_rename_session(
        &mut self,
        session_id: i64,
        current_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(&current_name)
                .placeholder(t!("ChatPanel.session_name_placeholder").to_string())
        });

        let panel_entity = cx.entity();
        let input_for_dialog = input_state.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = input_for_dialog.clone();
            let panel_for_ok = panel_entity.clone();

            dialog
                .title(t!("ChatPanel.rename_session_title").to_string())
                .w(px(360.0))
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.save").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, _window, cx| {
                    let new_name = input_for_ok.read(cx).value().to_string();
                    if !new_name.trim().is_empty() {
                        panel_for_ok.update(cx, |this, cx| {
                            this.rename_session(session_id, new_name, cx);
                        });
                    }
                    true
                })
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .child(t!("ChatPanel.rename_session_prompt").to_string()),
                        )
                        .child(Input::new(&input_for_dialog).w_full()),
                )
        });
    }

    fn rename_session(&mut self, session_id: i64, new_name: String, cx: &mut Context<Self>) {
        let session_service = self.session_service.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let renamed = session_service
                .update_session_name(session_id, new_name)
                .is_ok();

            if renamed {
                if let Some(entity) = this.upgrade() {
                    let _ = cx.update(|cx| {
                        entity.update(cx, |this, cx| {
                            this.load_history_sessions(cx);
                        });
                    });
                }
            }
        })
        .detach();
    }

    fn load_session(&mut self, session_id: i64, cx: &mut Context<Self>) {
        let session_service = self.session_service.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let messages = match session_service.get_messages(session_id) {
                Ok(m) => m,
                Err(_) => return,
            };

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    entity.update(cx, |this, cx| {
                        this.session_id = Some(session_id);
                        this.messages = messages
                            .iter()
                            .map(|msg| {
                                let role = match msg.role.as_str() {
                                    "assistant" => ChatRole::Assistant,
                                    _ => ChatRole::User,
                                };
                                ChatMessageUI::from_history(
                                    msg.id.to_string(),
                                    role,
                                    msg.content.clone(),
                                )
                            })
                            .collect();
                        this.chat_history = messages
                            .iter()
                            .map(|msg| {
                                let role = match msg.role.as_str() {
                                    "assistant" => Role::Assistant,
                                    "system" => Role::System,
                                    _ => Role::User,
                                };
                                Message::text(role, msg.content.clone())
                            })
                            .collect();
                        this.sql_result_views.clear();
                        this.sql_block_results.clear();
                        this.latest_ai_message_id = None;
                        this.render_limit = MESSAGE_RENDER_LIMIT;
                        cx.notify();
                    });
                });
            }
        })
        .detach();
    }

    // ========================================================================
    // 消息发送与工作流
    // ========================================================================

    fn persist_assistant_message(&self, session_id: i64, content: String) {
        let _ = self
            .session_service
            .add_assistant_message(session_id, content);
    }

    fn send_message(&mut self, content: String, cx: &mut Context<Self>) {
        if content.trim().is_empty() || self.is_loading {
            return;
        }

        // 添加用户消息
        self.messages.push(ChatMessageUI::user(content.clone()));
        self.chat_history
            .push(Message::text(Role::User, content.clone()));
        self.last_user_input = Some(content.clone());
        self.scroll_to_bottom_and_mark();
        cx.notify();

        // 统一走 send_to_ai — AgentDispatcher 自动路由：
        //   有 database_metadata capability → SqlWorkflowAgent
        //   无 → GeneralChatAgent
        self.send_to_ai(content, cx);
    }

    /// 构建 ProviderConfig（两条路径共用）
    fn build_provider_config(&self, provider_id: i64, _cx: &mut Context<Self>) -> ProviderConfig {
        let base = self
            .storage_manager
            .get::<ProviderRepository>()
            .and_then(|repo| repo.get(provider_id).ok().flatten())
            .unwrap_or_default();
        ProviderConfig {
            model: self.selected_model.clone().unwrap_or(base.model),
            max_tokens: Some(self.model_settings.max_tokens as i32),
            temperature: Some(self.model_settings.temperature),
            ..base
        }
    }

    /// 取消当前操作
    pub fn cancel_current_operation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 取消正在进行的请求
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }

        // 重置状态
        self.is_loading = false;

        // 更新输入组件状态
        self.ai_input.update(cx, |input, cx| {
            input.set_loading(false, window, cx);
        });

        // 移除进行中的状态消息
        self.messages
            .retain(|m| !matches!(m.variant, MessageVariant::Status { is_done: false, .. }));

        // 添加取消消息
        self.messages.push(ChatMessageUI::status(
            t!("ChatPanel.action_cancelled").to_string(),
            true,
        ));

        cx.notify();
    }

    /// 重试上一次操作
    pub fn retry_last_operation(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(last_input) = self.last_user_input.clone() else {
            return;
        };

        // 移除错误消息
        self.messages.retain(|m| {
            !matches!(
                &m.variant,
                MessageVariant::Status { title, .. }
                    if title.contains(t!("ChatPanel.error_keyword").as_ref())
                        || title.contains(t!("ChatPanel.failed_keyword").as_ref())
            )
        });

        // 重新发送
        self.send_to_ai(last_input, cx);
    }

    /// 是否可以取消
    fn can_cancel(&self) -> bool {
        self.is_loading && self.cancel_token.is_some()
    }

    /// 是否可以重试
    fn can_retry(&self) -> bool {
        self.last_user_input.is_some() && !self.is_loading
    }

    /// 统一 AI 发送入口 — 通过 AgentDispatcher 路由到合适的 Agent
    fn send_to_ai(&mut self, content: String, cx: &mut Context<Self>) {
        let Some(provider_id_str) = self.provider_id.clone() else {
            self.messages.push(ChatMessageUI::assistant(
                t!("ChatPanel.select_ai_model_first").to_string(),
            ));
            cx.notify();
            return;
        };

        let provider_id: i64 = match provider_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                self.messages.push(ChatMessageUI::assistant(
                    t!("ChatPanel.invalid_model_id").to_string(),
                ));
                cx.notify();
                return;
            }
        };

        // 创建助手消息占位符
        let assistant_msg_id = Uuid::new_v4().to_string();
        self.latest_ai_message_id = Some(assistant_msg_id.clone());
        self.messages
            .push(ChatMessageUI::streaming_assistant().with_id(assistant_msg_id.clone()));

        self.is_loading = true;
        self.scroll_to_bottom_and_mark();
        cx.notify();

        let global_provider_state = cx.global::<GlobalProviderState>().clone();
        let session_service = self.session_service.clone();
        let provider_config = self.build_provider_config(provider_id, cx);
        let storage_manager = self.storage_manager.clone();

        // 根据设置限制历史记录数量
        let history_count = self.model_settings.history_count;
        let history: Vec<Message> = if history_count > 0 && !self.chat_history.is_empty() {
            let history_start = self.chat_history.len().saturating_sub(history_count);
            self.chat_history
                .iter()
                .skip(history_start)
                .cloned()
                .collect()
        } else {
            self.chat_history.clone()
        };

        let ai_input = self.ai_input.clone();
        let session_id = self.session_id;
        let provider_id_str_clone = provider_id_str.clone();
        let message_content = content.clone();

        // 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());

        // 获取 AgentRegistry 快照
        let registry = cx.global::<AgentRegistry>();
        let agents: Vec<_> = registry
            .sorted_ids()
            .iter()
            .filter_map(|id| registry.get(id).cloned())
            .collect();

        let mut affinity = self.session_affinity.clone();

        // 注入数据库元数据 capability（如果有数据库连接）
        let db_metadata =
            self.ai_input
                .read(cx)
                .get_connection_info()
                .and_then(|(conn_id, database, schema)| {
                    let db_state = cx.global::<GlobalDbState>().clone();
                    let db_type = db_state
                        .get_config(&conn_id)
                        .map(|c| c.database_type)
                        .unwrap_or(DatabaseType::MySQL);
                    let database_str = database.unwrap_or_default();
                    if database_str.is_empty() {
                        return None;
                    }
                    Some(DatabaseMetadataProvider::new(
                        db_state,
                        conn_id,
                        database_str,
                        schema,
                        db_type,
                    ))
                });

        // 获取 Tokio runtime handle（AgentDispatcher::dispatch 内部使用 tokio::spawn）
        let tokio_handle = Tokio::handle(cx);

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            // 进入 Tokio runtime 上下文
            let _guard = tokio_handle.enter();

            // 确保 session 存在
            let is_new_session = session_id.is_none();
            let session_db_id = match session_service.ensure_session(
                session_id,
                &provider_id_str_clone,
                t!("ChatPanel.sql_session_name").as_ref(),
            ) {
                Ok(id) => {
                    if session_id.is_none() {
                        if let Some(entity) = this.upgrade() {
                            let _ = cx.update(|cx| {
                                entity.update(cx, |content, cx| {
                                    content.session_id = Some(id);
                                    content.is_new_session = true;
                                    cx.notify();
                                });
                            });
                        }
                    }
                    id
                }
                Err(e) => {
                    let error_msg =
                        t!("ChatPanel.session_error", error = format!("{}", e)).to_string();
                    if let Some(entity) = this.upgrade() {
                        let _ = cx.update(|cx| {
                            entity.update(cx, |content, cx| {
                                if let Some(msg) = content
                                    .messages
                                    .iter_mut()
                                    .find(|m| m.id == assistant_msg_id)
                                {
                                    msg.is_streaming = false;
                                    msg.content = error_msg;
                                }
                                content.is_loading = false;
                                cx.notify();
                            });
                        });
                    }
                    return;
                }
            };

            let _ = session_service.add_user_message(session_db_id, message_content.clone());

            // 如果是新会话的第一条消息，更新会话名称
            if is_new_session {
                let session_name = extract_session_name(&message_content);
                if session_service
                    .update_session_name(session_db_id, session_name)
                    .is_ok()
                {
                    if let Some(entity) = this.upgrade() {
                        let _ = cx.update(|cx| {
                            entity.update(cx, |content, cx| {
                                content.is_new_session = false;
                                content.load_history_sessions(cx);
                            });
                        });
                    }
                }
            }

            // 构建 AgentContext（注入 DB capability）
            let mut ctx_agent = AgentContext::new(
                message_content,
                history,
                provider_config,
                global_provider_state,
                storage_manager,
                cancel_token,
            );

            if let Some(db_meta) = db_metadata {
                ctx_agent.set_capability(CAP_DB_METADATA, db_meta);
            }

            // 使用 AgentDispatcher 进行路由和执行
            let mut local_registry = AgentRegistry::new();
            for agent in agents {
                local_registry.register_arc(agent);
            }
            let mut rx = AgentDispatcher::dispatch(ctx_agent, &local_registry, &mut affinity).await;

            // 回写亲和性状态
            if let Some(entity) = this.upgrade() {
                let affinity_clone = affinity.clone();
                let _ = cx.update(|cx| {
                    entity.update(cx, |this, _cx| {
                        this.session_affinity = affinity_clone;
                    });
                });
            }

            // 处理 Agent 事件
            let mut full_content = String::new();
            while let Some(event) = rx.recv().await {
                match event {
                    AgentEvent::Progress(stage) => {
                        // 更新状态消息（工作流进度）
                        if let Some(entity) = this.upgrade() {
                            let stage_clone = stage.clone();
                            let msg_id = assistant_msg_id.clone();
                            let _ = cx.update(|cx| {
                                entity.update(cx, |panel, cx| {
                                    if let Some(msg) =
                                        panel.messages.iter_mut().find(|m| m.id == msg_id)
                                    {
                                        msg.variant = MessageVariant::Status {
                                            title: stage_clone,
                                            is_done: false,
                                        };
                                    }
                                    panel.scroll_to_bottom_and_mark();
                                    cx.notify();
                                });
                            });
                        }
                    }
                    AgentEvent::TextDelta(delta) => {
                        full_content.push_str(&delta);
                        if let Some(entity) = this.upgrade() {
                            let content_clone = full_content.clone();
                            let msg_id = assistant_msg_id.clone();
                            let _ = cx.update(|cx| {
                                entity.update(cx, |panel, cx| {
                                    if let Some(msg) =
                                        panel.messages.iter_mut().find(|m| m.id == msg_id)
                                    {
                                        // 如果消息还是 Status 模式，切换为 Text 模式
                                        if matches!(msg.variant, MessageVariant::Status { .. }) {
                                            msg.variant = MessageVariant::Text;
                                            msg.is_streaming = true;
                                        }
                                        msg.content = content_clone;
                                    }
                                    panel.scroll_to_bottom_and_mark();
                                    cx.notify();
                                });
                            });
                        }
                    }
                    AgentEvent::Completed(result) => {
                        if let Some(entity) = this.upgrade() {
                            let final_content = if full_content.is_empty() {
                                result.content
                            } else {
                                full_content.clone()
                            };
                            let msg_id = assistant_msg_id.clone();
                            let _ = cx.update(|cx| {
                                if let Some(window_id) = cx.active_window() {
                                    let _ = cx.update_window(window_id, |_, window, cx| {
                                        entity.update(cx, |content, cx| {
                                            if let Some(msg) =
                                                content.messages.iter_mut().find(|m| m.id == msg_id)
                                            {
                                                msg.is_streaming = false;
                                                msg.variant = MessageVariant::Text;
                                                msg.content = final_content.clone();
                                            }
                                            content.chat_history.push(Message::text(
                                                Role::Assistant,
                                                final_content.clone(),
                                            ));
                                            content.is_loading = false;
                                            content.cancel_token = None;
                                            ai_input.update(cx, |input, cx| {
                                                input.set_loading(false, window, cx);
                                            });
                                            content.persist_assistant_message(
                                                session_db_id,
                                                final_content.clone(),
                                            );
                                            content.load_history_sessions(cx);
                                            content.auto_execute_query_blocks(
                                                &msg_id,
                                                &final_content,
                                                window,
                                                cx,
                                            );
                                            content.scroll_to_bottom_and_mark();
                                            cx.notify();
                                        });
                                    });
                                }
                            });
                        }
                        break;
                    }
                    AgentEvent::Error(message) => {
                        if let Some(entity) = this.upgrade() {
                            let msg_id = assistant_msg_id.clone();
                            let _ = cx.update(|cx| {
                                if let Some(window_id) = cx.active_window() {
                                    let _ = cx.update_window(window_id, |_, window, cx| {
                                        entity.update(cx, |content, cx| {
                                            if let Some(msg) =
                                                content.messages.iter_mut().find(|m| m.id == msg_id)
                                            {
                                                msg.is_streaming = false;
                                                msg.variant = MessageVariant::Text;
                                                msg.content = t!(
                                                    "ChatPanel.error_message",
                                                    message = message
                                                )
                                                .to_string();
                                            }
                                            content.is_loading = false;
                                            content.cancel_token = None;
                                            ai_input.update(cx, |input, cx| {
                                                input.set_loading(false, window, cx);
                                            });
                                            cx.notify();
                                        });
                                    });
                                }
                            });
                        }
                        break;
                    }
                    AgentEvent::Cancelled => {
                        if let Some(entity) = this.upgrade() {
                            let msg_id = assistant_msg_id.clone();
                            let _ = cx.update(|cx| {
                                if let Some(window_id) = cx.active_window() {
                                    let _ = cx.update_window(window_id, |_, window, cx| {
                                        entity.update(cx, |content, cx| {
                                            if let Some(msg) =
                                                content.messages.iter_mut().find(|m| m.id == msg_id)
                                            {
                                                msg.is_streaming = false;
                                                msg.variant = MessageVariant::Text;
                                                msg.content =
                                                    t!("ChatPanel.action_cancelled").to_string();
                                            }
                                            content.is_loading = false;
                                            content.cancel_token = None;
                                            ai_input.update(cx, |input, cx| {
                                                input.set_loading(false, window, cx);
                                            });
                                            cx.notify();
                                        });
                                    });
                                }
                            });
                        }
                        break;
                    }
                }
            }
        })
        .detach();
    }

    // ========================================================================
    // SQL 执行
    // ========================================================================

    fn execute_sql(
        &mut self,
        sql: String,
        connection_id: String,
        database: Option<String>,
        schema: Option<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 添加用户消息显示 SQL
        let sql_display = format!("```sql\n{}\n```", sql.trim());
        self.messages.push(ChatMessageUI::user(sql_display));

        // 添加状态消息
        let result_msg_id = Uuid::new_v4().to_string();
        self.messages.push(
            ChatMessageUI::status(t!("ChatPanel.executing").to_string(), false)
                .with_id(result_msg_id.clone()),
        );

        self.scroll_to_bottom_and_mark();
        cx.notify();

        let global_db_state = cx.global::<GlobalDbState>().clone();
        let db_type = global_db_state
            .get_config(&connection_id)
            .map(|c| c.database_type)
            .unwrap_or(DatabaseType::MySQL);

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let results = global_db_state
                .execute_script(
                    cx,
                    connection_id.clone(),
                    sql,
                    database.clone(),
                    schema,
                    None,
                )
                .await;

            if let Some(entity) = this.upgrade() {
                let _ = cx.update(|cx| {
                    if let Some(window_id) = cx.active_window() {
                        let _ = cx.update_window(window_id, |_, window, cx| {
                            entity.update(cx, |content, cx| {
                                if let Some(msg) =
                                    content.messages.iter_mut().find(|m| m.id == result_msg_id)
                                {
                                    match results {
                                        Ok(sql_results) => {
                                            // 用户主动执行的 SQL 结果默认展开
                                            let result_view = cx.new(|cx| {
                                                ChatSqlResultView::new(
                                                    sql_results,
                                                    &connection_id,
                                                    database.clone(),
                                                    db_type,
                                                    false, // 新创建的结果视图默认展开
                                                    window,
                                                    cx,
                                                )
                                            });

                                            content
                                                .sql_result_views
                                                .insert(result_msg_id.clone(), result_view);
                                            msg.variant = MessageVariant::SqlResult;
                                            msg.is_streaming = false;

                                            // 折叠历史 SQL 结果，保持最新 3 条展开
                                            content.collapse_old_sql_results(3, cx);
                                        }
                                        Err(e) => {
                                            msg.content =
                                                t!("ChatPanel.execution_error", error = e)
                                                    .to_string();
                                            msg.variant = MessageVariant::Text;
                                            msg.is_streaming = false;
                                        }
                                    }
                                }
                                content.scroll_to_bottom_and_mark();
                                cx.notify();
                            });
                        });
                    }
                });
            }
        })
        .detach();
    }

    // ========================================================================
    // SQL 代码块执行与结果
    // ========================================================================

    /// 折叠历史 SQL 结果视图，保持最新 N 条展开
    fn collapse_old_sql_results(&mut self, keep_recent: usize, cx: &mut Context<Self>) {
        let total = self.sql_result_views.len();
        if total <= keep_recent {
            return;
        }

        // 按消息在 messages 中的位置排序，找到需要折叠的视图
        let msg_ids: Vec<String> = self
            .messages
            .iter()
            .filter(|m| matches!(m.variant, MessageVariant::SqlResult))
            .map(|m| m.id.clone())
            .collect();

        let to_collapse_count = msg_ids.len().saturating_sub(keep_recent);
        for msg_id in msg_ids.iter().take(to_collapse_count) {
            if let Some(view) = self.sql_result_views.get(msg_id) {
                view.update(cx, |v, cx| {
                    if !v.is_collapsed() {
                        v.set_collapsed(true);
                        cx.notify();
                    }
                });
            }
        }
    }

    /// 折叠历史 SQL 代码块结果，保持最新 N 条展开
    ///
    /// 参数：
    /// - `keep_recent`: 保持展开的最近结果数量
    /// - `current_msg_id`: 当前正在执行的消息ID（可选，用于确保当前执行的不被折叠）
    /// - `current_block_key`: 当前正在执行的代码块key（可选）
    fn collapse_old_sql_block_results(
        &mut self,
        keep_recent: usize,
        current_msg_id: Option<&str>,
        current_block_key: Option<usize>,
    ) {
        // 收集所有已执行且展开的 SQL 代码块结果，按消息位置排序
        let msg_positions: HashMap<String, usize> = self
            .messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| (msg.id.clone(), idx))
            .collect();

        // 收集所有已执行的代码块（消息ID、代码块key、消息位置）
        let mut executed_blocks: Vec<(String, usize, usize)> = self
            .sql_block_results
            .iter()
            .flat_map(|(msg_id, blocks)| {
                let msg_pos = msg_positions.get(msg_id).copied().unwrap_or(usize::MAX);
                blocks.iter().filter_map(move |(key, state)| {
                    // 只处理已执行过的代码块
                    if state.last_run_sql.is_some() {
                        Some((msg_id.clone(), *key, msg_pos))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // 按消息位置和代码块key排序（最老的在前）
        executed_blocks.sort_by(
            |(_, a_key, a_pos), (_, b_key, b_pos)| match a_pos.cmp(b_pos) {
                std::cmp::Ordering::Equal => a_key.cmp(b_key),
                other => other,
            },
        );

        let total = executed_blocks.len();
        if total <= keep_recent {
            return;
        }

        // 折叠最老的，保留最新的 keep_recent 个
        let to_collapse_count = total.saturating_sub(keep_recent);
        for (msg_id, key, _) in executed_blocks.iter().take(to_collapse_count) {
            // 跳过当前正在执行的代码块
            if current_msg_id == Some(msg_id.as_str()) && current_block_key == Some(*key) {
                continue;
            }

            if let Some(blocks) = self.sql_block_results.get_mut(msg_id) {
                if let Some(state) = blocks.get_mut(key) {
                    if !state.collapsed {
                        state.collapsed = true;
                    }
                }
            }
        }
    }

    /// 尝试裁剪渲染限制（仅当用户在底部时）
    fn maybe_trim_render_limit(&mut self) {
        if self.is_at_bottom && self.render_limit > MESSAGE_RENDER_LIMIT {
            self.render_limit = MESSAGE_RENDER_LIMIT;
        }
    }

    /// 滚动到底部并标记状态
    fn scroll_to_bottom_and_mark(&mut self) {
        self.scroll_handle.scroll_to_bottom();
        self.is_at_bottom = true;
        self.maybe_trim_render_limit();
    }

    fn auto_execute_query_blocks(
        &mut self,
        message_id: &str,
        _content: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.latest_ai_message_id.as_deref() != Some(message_id) {
            return;
        }
        let connection_info = self.ai_input.read(cx).get_connection_info();
        let global_state = cx.global::<GlobalDbState>().clone();

        // 使用消息的缓存方法获取 SQL 代码块
        let highlight_theme = cx.theme().highlight_theme.clone();
        let blocks = self
            .messages
            .iter_mut()
            .find(|m| m.id == message_id)
            .map(|msg| msg.get_sql_blocks(highlight_theme.as_ref()))
            .unwrap_or_default();

        for block in blocks {
            if !block.is_sql {
                continue;
            }
            self.ensure_sql_block_state(message_id, &block, window, cx);
            let is_query = match &connection_info {
                Some((connection_id, _, _)) => {
                    is_query_statement_for_connection(&global_state, connection_id, &block.code)
                }
                None => is_query_statement_fallback(&block.code),
            };
            if is_query {
                self.execute_sql_block(message_id, &block, window, cx, false);
            }
        }
    }

    fn ensure_sql_block_state(
        &mut self,
        message_id: &str,
        block: &SqlCodeBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entry = self
            .sql_block_results
            .entry(message_id.to_string())
            .or_default();
        let needs_reset = entry
            .get(&block.key)
            .map_or(true, |state| state.sql != block.code);
        if needs_reset {
            let container = cx.new(|cx| SqlResultTabContainer::new(window, cx));
            entry.insert(
                block.key,
                SqlBlockResultState::new(block.code.clone(), container),
            );
        }
    }

    fn execute_sql_block(
        &mut self,
        message_id: &str,
        block: &SqlCodeBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
        force: bool,
    ) {
        self.ensure_sql_block_state(message_id, block, window, cx);

        let Some((connection_id, database, schema)) = self.ai_input.read(cx).get_connection_info()
        else {
            if let Some(state) = self
                .sql_block_results
                .get_mut(message_id)
                .and_then(|map| map.get_mut(&block.key))
            {
                state.set_error(t!("ChatPanel.select_database_or_schema_first").to_string());
            }
            cx.notify();
            return;
        };

        let Some(state) = self
            .sql_block_results
            .get_mut(message_id)
            .and_then(|map| map.get_mut(&block.key))
        else {
            return;
        };

        if !force && !state.should_run(&block.code) {
            return;
        }

        state.clear_error();
        state.mark_run(block.code.clone());
        // 确保当前代码块展开
        state.collapsed = false;

        let container = state.container.clone();
        let sql = block.code.clone();
        container.update(cx, |container, cx| {
            container.handle_run_query(sql, connection_id, database, schema, window, cx);
            container.show(cx);
        });

        // 折叠旧的SQL代码块结果，保持最新3条展开
        self.collapse_old_sql_block_results(3, Some(message_id), Some(block.key));
        cx.notify();
    }

    // ========================================================================
    // 渲染
    // ========================================================================

    fn render_messages(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = cx.entity().clone();
        let total = self.messages.len();
        let hidden_count = total.saturating_sub(self.render_limit);
        let can_collapse = total > MESSAGE_RENDER_LIMIT && self.render_limit > MESSAGE_RENDER_LIMIT;

        div()
            .id("chat-messages-list")
            .flex_1()
            .min_h_0()
            .w_full()
            .relative()
            .child(
                div()
                    .id("chat-messages-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .p_4()
                    .pb_8()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .when(hidden_count > 0 || can_collapse, |this| {
                                this.child(
                                    h_flex().w_full().justify_center().gap_2().child(
                                        h_flex()
                                            .gap_2()
                                            .when(hidden_count > 0, |this| {
                                                this.child(
                                                    Button::new("chat-load-more")
                                                        .ghost()
                                                        .label(
                                                            t!(
                                                                "ChatMessageList.load_more",
                                                                hidden_count = hidden_count
                                                            )
                                                            .to_string(),
                                                        )
                                                        .on_click(cx.listener(
                                                            |this, _event, _window, cx| {
                                                                let total = this.messages.len();
                                                                this.render_limit = (this
                                                                    .render_limit
                                                                    + MESSAGE_RENDER_STEP)
                                                                    .min(total);
                                                                // 用户加载更多时，标记不在底部，防止自动裁剪
                                                                this.is_at_bottom = false;
                                                                cx.notify();
                                                            },
                                                        )),
                                                )
                                            })
                                            .when(can_collapse, |this| {
                                                this.child(
                                                    Button::new("chat-collapse-history")
                                                        .ghost()
                                                        .label(
                                                            t!("ChatMessageList.collapse_history")
                                                                .to_string(),
                                                        )
                                                        .on_click(cx.listener(
                                                            |this, _event, _window, cx| {
                                                                this.render_limit =
                                                                    MESSAGE_RENDER_LIMIT;
                                                                this.scroll_to_bottom_and_mark();
                                                                cx.notify();
                                                            },
                                                        )),
                                                )
                                            }),
                                    ),
                                )
                            })
                            .children(
                                self.messages
                                    .iter()
                                    .skip(hidden_count)
                                    .map(|msg| self.render_message(msg, &panel, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(px(16.0))
                    .child(Scrollbar::vertical(&self.scroll_handle)),
            )
    }

    fn render_history_sidebar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let border = cx.theme().border;
        let muted = cx.theme().muted;

        if self.session_list.is_none() {
            self.update_session_list(window, cx);
        }

        let session_list = self.session_list.clone();

        v_flex()
            .w(px(260.0))
            .h_full()
            .min_h_0()
            .flex_shrink_0()
            .border_r_1()
            .border_color(border)
            .bg(muted)
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(t!("ChatSession.history_title")),
                    )
                    .child(
                        Button::new("sql-new-session")
                            .icon(IconName::Plus)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.start_new_session(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .when_some(session_list, |this, list| {
                        this.child(
                            List::new(&list)
                                .w_full()
                                .h_full()
                                .border_1()
                                .border_color(border)
                                .rounded(cx.theme().radius),
                        )
                    }),
            )
    }

    fn render_message(
        &self,
        msg: &ChatMessageUI,
        panel: &Entity<Self>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match msg.role {
            ChatRole::User => div()
                .w_full()
                .px_3()
                .py_2()
                .bg(cx.theme().accent)
                .text_color(cx.theme().accent_foreground)
                .rounded_lg()
                .child(
                    TextView::markdown(
                        SharedString::from(format!("user-msg-{}", msg.id)),
                        msg.content.clone(),
                    )
                    .selectable(true),
                )
                .into_any_element(),
            ChatRole::Assistant => match &msg.variant {
                MessageVariant::Status { title, is_done } => {
                    self.render_status_message(&msg.id, title, *is_done, panel, cx)
                }
                MessageVariant::Text => self.render_assistant_message(msg, panel, cx),
                MessageVariant::SqlResult => self.render_sql_result(&msg.id, cx),
            },
            ChatRole::System => {
                // 系统消息渲染为居中的灰色文本
                h_flex()
                    .w_full()
                    .justify_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(msg.content.clone()),
                    )
                    .into_any_element()
            }
        }
    }

    fn render_status_message(
        &self,
        id: &str,
        title: &str,
        is_done: bool,
        panel: &Entity<Self>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let icon = if is_done {
            IconName::Check
        } else {
            IconName::Loader
        };
        let can_cancel = !is_done && self.can_cancel();
        let can_retry = is_done
            && self.can_retry()
            && (title.contains(t!("ChatPanel.error_keyword").as_ref())
                || title.contains(t!("ChatPanel.failed_keyword").as_ref()));

        h_flex()
            .id(SharedString::from(id.to_string()))
            .w_full()
            .items_center()
            .justify_between()
            .gap_2()
            .py_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Icon::new(icon)
                            .with_size(Size::Small)
                            .text_color(if is_done {
                                cx.theme().success
                            } else {
                                cx.theme().muted_foreground
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(title.to_string()),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .when(can_cancel, {
                        let panel = panel.clone();
                        move |this| {
                            this.child(
                                Button::new("cancel-operation")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Close)
                                    .label(t!("Common.cancel").to_string())
                                    .on_click({
                                        let panel = panel.clone();
                                        move |_, window, cx| {
                                            panel.update(cx, |content, cx| {
                                                content.cancel_current_operation(window, cx);
                                            });
                                        }
                                    }),
                            )
                        }
                    })
                    .when(can_retry, {
                        let panel = panel.clone();
                        move |this| {
                            this.child(
                                Button::new("retry-operation")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Refresh)
                                    .label(t!("ChatPanel.retry").to_string())
                                    .on_click({
                                        let panel = panel.clone();
                                        move |_, window, cx| {
                                            panel.update(cx, |content, cx| {
                                                content.retry_last_operation(window, cx);
                                            });
                                        }
                                    }),
                            )
                        }
                    }),
            )
            .into_any_element()
    }

    fn render_assistant_message(
        &self,
        msg: &ChatMessageUI,
        panel: &Entity<Self>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if msg.is_streaming && msg.content.is_empty() {
            return div()
                .w_full()
                .py_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("ChatPanel.thinking")),
                )
                .into_any_element();
        }

        let panel_for_actions = panel.clone();
        let panel_for_render = panel.clone();
        let message_id = msg.id.clone();

        div()
            .w_full()
            .child(
                div().w_full().p_3().child(
                    TextView::markdown(
                        SharedString::from(format!("ai-sql-msg-{}", msg.id)),
                        msg.content.clone(),
                    )
                    .selectable(true)
                    .code_block_actions({
                        let message_id = message_id.clone();
                        move |code_block, _window, _cx| {
                            let block = SqlCodeBlock::from_code_block(code_block, 0);
                            let is_sql = block.is_sql;
                            let code = code_block.code();

                            h_flex()
                                .gap_1()
                                .child(Clipboard::new("copy").value(code.clone()))
                                .when(is_sql, {
                                    let panel = panel_for_actions.clone();
                                    let message_id = message_id.clone();
                                    let block_for_action = block.clone();
                                    move |this| {
                                        this.child(
                                            Button::new("run-sql")
                                                .icon(IconName::SquareTerminal)
                                                .ghost()
                                                .xsmall()
                                                .label(t!("ChatSqlBlock.run").to_string())
                                                .on_click({
                                                    let panel = panel.clone();
                                                    let message_id = message_id.clone();
                                                    let block_for_action = block_for_action.clone();
                                                    move |_, window, cx| {
                                                        panel.update(cx, |p, cx| {
                                                            p.execute_sql_block(
                                                                &message_id,
                                                                &block_for_action,
                                                                window,
                                                                cx,
                                                                true,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                    }
                                })
                                .into_any_element()
                        }
                    })
                    .code_block_renderer({
                        let message_id = message_id.clone();
                        let panel_for_collapse = panel_for_render.clone();
                        move |code_block, options, default_element, _window, cx| {
                            let block = SqlCodeBlock::from_code_block(code_block, options.index);
                            if !block.is_sql {
                                return default_element;
                            }

                            let content = panel_for_collapse.read(cx);
                            content.render_sql_block_container(
                                &message_id,
                                &block,
                                default_element,
                                &panel_for_collapse,
                                cx,
                            )
                        }
                    }),
                ),
            )
            .into_any_element()
    }

    fn render_sql_block_container(
        &self,
        message_id: &str,
        block: &SqlCodeBlock,
        default_element: AnyElement,
        panel: &Entity<Self>,
        cx: &App,
    ) -> AnyElement {
        let result_state = self
            .sql_block_results
            .get(message_id)
            .and_then(|map| map.get(&block.key));

        let (error, container, has_visible_result, collapsed, summary) = result_state
            .map(|state| {
                (
                    state.error.clone(),
                    Some(state.container.clone()),
                    state.has_visible_result(cx),
                    state.collapsed,
                    state.get_summary(cx),
                )
            })
            .unwrap_or((None, None, false, false, None));

        let error_element = error.map(|error| {
            div()
                .text_sm()
                .text_color(cx.theme().danger)
                .child(error)
                .into_any_element()
        });

        // 结果区域头部（折叠按钮 + 摘要）
        let result_header = if has_visible_result {
            let icon = if collapsed {
                IconName::ChevronRight
            } else {
                IconName::ChevronDown
            };

            let panel_for_toggle = panel.clone();
            let message_id_for_toggle = message_id.to_string();
            let block_key = block.key;

            Some(
                h_flex()
                    .id(SharedString::from(format!(
                        "sql-result-toggle-{}-{}",
                        message_id, block_key
                    )))
                    .w_full()
                    .items_center()
                    .gap_2()
                    .py_1()
                    .cursor_pointer()
                    .on_click(move |_, _window, cx| {
                        panel_for_toggle.update(cx, |p, cx| {
                            if let Some(state) = p
                                .sql_block_results
                                .get_mut(&message_id_for_toggle)
                                .and_then(|map| map.get_mut(&block_key))
                            {
                                state.toggle_collapsed();
                                cx.notify();
                            }
                        });
                    })
                    .child(
                        Icon::new(icon)
                            .with_size(Size::Small)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(if collapsed {
                                summary.unwrap_or_else(|| t!("ChatPanel.view_result").to_string())
                            } else {
                                t!("ChatPanel.collapse_result").to_string()
                            }),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        // 结果容器（仅在展开时显示）
        let result_element = if has_visible_result && !collapsed {
            container.map(|container| {
                div()
                    .w_full()
                    .h(px(280.0))
                    .child(container)
                    .into_any_element()
            })
        } else {
            None
        };

        v_flex()
            .w_full()
            .gap_2()
            .child(default_element)
            .when_some(error_element, |this, error| this.child(error))
            .when_some(result_header, |this, header| this.child(header))
            .when_some(result_element, |this, result| this.child(result))
            .into_any_element()
    }

    fn render_sql_result(&self, msg_id: &str, _cx: &mut Context<Self>) -> AnyElement {
        if let Some(result_view) = self.sql_result_views.get(msg_id) {
            div().w_full().child(result_view.clone()).into_any_element()
        } else {
            div()
                .w_full()
                .text_sm()
                .child(t!("ChatMessageList.loading"))
                .into_any_element()
        }
    }

    fn render_input(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w_full().px_2().py_2().child(self.ai_input.clone())
    }
}

impl EventEmitter<ChatPanelEvent> for ChatPanel {}
impl EventEmitter<TabContentEvent> for ChatPanel {}

impl Focusable for ChatPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl SessionListHost for ChatPanel {
    fn on_session_select(&mut self, session_id: i64, cx: &mut Context<Self>) {
        self.load_session(session_id, cx);
    }

    fn on_session_edit(
        &mut self,
        session_id: i64,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_rename_session(session_id, name, window, cx);
    }

    fn on_session_delete(&mut self, session_id: i64, cx: &mut Context<Self>) {
        self.delete_session(session_id, cx);
    }

    fn is_current_session(&self, session_id: i64) -> bool {
        self.session_id == Some(session_id)
    }
}

impl Render for ChatPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_logged_in = GlobalCloudUser::is_logged_in(cx);
        if is_logged_in != self.is_logged_in {
            self.is_logged_in = is_logged_in;
            self.load_providers(window, cx);
        }

        div().size_full().bg(cx.theme().background).child(
            h_flex()
                .size_full()
                .child(self.render_history_sidebar(window, cx))
                .child(
                    div().flex_1().h_full().min_w_0().child(
                        v_flex()
                            .size_full()
                            .child(self.render_messages(cx))
                            .child(self.render_input(cx)),
                    ),
                ),
        )
    }
}

impl TabContent for ChatPanel {
    fn content_key(&self) -> &'static str {
        "SQL-Chat"
    }

    fn title(&self, _cx: &App) -> SharedString {
        SharedString::from(t!("ChatPanel.title").to_string())
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(IconName::Database.color().with_size(Size::Medium))
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
    }
}
