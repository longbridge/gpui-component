//! ChatEngine - 共享业务逻辑引擎
//!
//! 封装 AI 聊天面板的通用业务逻辑，包括：
//! - 消息管理（添加、更新、流式处理）
//! - 会话管理（创建、加载、持久化）
//! - Provider/Model 状态
//! - 滚动和加载控制

use gpui::ScrollHandle;
use rust_i18n::t;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

use crate::ai_chat::components::ModelSettings;
use crate::ai_chat::panel::CodeBlockActionRegistry;
use crate::ai_chat::services::{SessionService, extract_session_name};
use crate::ai_chat::types::{
    ChatMessageUIGeneric, ChatRole, MessageExtension, NoExtension,
};
use crate::llm::chat_history::ChatSession;
use crate::llm::ProviderConfig;
use crate::llm::storage::ProviderRepository;
use crate::storage::StorageManager;
use crate::storage::traits::Repository;

/// 共享业务逻辑引擎
///
/// 作为字段被各面板持有，封装所有共享业务逻辑。
/// 通过泛型参数 E 支持不同的消息扩展类型。
pub struct ChatEngine<E: MessageExtension + Default = NoExtension> {
    /// 消息列表
    pub messages: Vec<ChatMessageUIGeneric<E>>,

    /// 当前会话 ID
    pub session_id: Option<i64>,
    /// 是否为新会话（等待第一条消息更新名称）
    pub is_new_session: bool,
    /// 历史会话列表
    pub history_sessions: Vec<ChatSession>,

    /// 当前 Provider ID
    pub provider_id: Option<String>,
    /// 当前选中的模型
    pub selected_model: Option<String>,
    /// Provider 配置列表
    pub provider_configs: Vec<ProviderConfig>,

    /// 是否正在加载/流式中
    pub is_loading: bool,
    /// 取消令牌
    pub cancel_token: Option<CancellationToken>,

    /// 模型设置
    pub model_settings: ModelSettings,

    /// 滚动句柄
    pub scroll_handle: ScrollHandle,
    /// 是否启用自动滚动
    pub auto_scroll_enabled: bool,

    /// 会话服务
    pub session_service: SessionService,

    /// 代码块操作注册表
    pub code_block_actions: CodeBlockActionRegistry,
}

impl<E: MessageExtension + Default> ChatEngine<E> {
    /// 创建引擎
    pub fn new(storage_manager: StorageManager) -> Self {
        Self {
            messages: Vec::new(),
            session_id: None,
            is_new_session: false,
            history_sessions: Vec::new(),
            provider_id: None,
            selected_model: None,
            provider_configs: Vec::new(),
            is_loading: false,
            cancel_token: None,
            model_settings: ModelSettings::default(),
            scroll_handle: ScrollHandle::new(),
            auto_scroll_enabled: true,
            session_service: SessionService::new(storage_manager),
            code_block_actions: CodeBlockActionRegistry::new(),
        }
    }

    // ========================================================================
    // 会话管理
    // ========================================================================

    /// 确保会话存在，如果不存在则创建新会话
    ///
    /// 返回会话 ID。如果创建失败则返回 None。
    pub fn ensure_session_id(&mut self, provider_id: &str, default_name: &str) -> Option<i64> {
        if let Some(id) = self.session_id {
            return Some(id);
        }

        match self.session_service.ensure_session(None, provider_id, default_name) {
            Ok(id) => {
                self.session_id = Some(id);
                self.is_new_session = true;
                Some(id)
            }
            Err(e) => {
                warn!(
                    "{}",
                    t!("AiChat.session_create_failed", error = e).to_string()
                );
                None
            }
        }
    }

    /// 持久化用户消息，并在新会话时更新标题
    pub fn persist_user_message(&mut self, session_id: i64, content: &str) {
        let _ = self.session_service.add_user_message(session_id, content.to_string());

        if self.is_new_session {
            self.is_new_session = false;
            let session_name = extract_session_name(content);
            let _ = self.session_service.update_session_name(session_id, session_name);
        }
    }

    /// 持久化助手消息
    pub fn persist_assistant_message(&self, session_id: i64, content: String) {
        let _ = self.session_service.add_assistant_message(session_id, content);
    }

    /// 开始新会话
    pub fn start_new_session(&mut self) {
        self.session_id = None;
        self.is_new_session = false;
        self.messages.clear();
    }

    // ========================================================================
    // 消息管理
    // ========================================================================

    /// 添加用户消息到 messages
    pub fn push_user_message(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessageUIGeneric::user(content));
    }

    /// 添加流式助手消息占位符，返回消息 ID
    pub fn push_streaming_assistant(&mut self) -> String {
        let msg_id = Uuid::new_v4().to_string();
        self.messages.push(
            ChatMessageUIGeneric::streaming_assistant()
                .with_id(msg_id.clone()),
        );
        msg_id
    }

    /// 更新流式消息内容
    pub fn update_streaming_content(&mut self, msg_id: &str, content: String) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.content = content;
        }
    }

    /// 完成流式消息
    pub fn finalize_streaming(&mut self, msg_id: &str, content: String) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.is_streaming = false;
            msg.content = content;
            msg.extension.on_finalize_streaming();
        }
    }

    /// 设置消息为错误状态
    pub fn set_message_error(&mut self, msg_id: &str, error: String) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.is_streaming = false;
            msg.content = error;
        }
    }

    /// 添加状态消息，返回消息 ID
    pub fn push_status(&mut self, title: impl Into<String>, is_done: bool) -> String {
        let msg_id = Uuid::new_v4().to_string();
        self.messages.push(
            ChatMessageUIGeneric::status(title, is_done)
                .with_id(msg_id.clone()),
        );
        msg_id
    }

    /// 添加完整助手消息
    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessageUIGeneric::assistant(content));
    }

    // ========================================================================
    // 取消操作
    // ========================================================================

    /// 取消当前操作
    pub fn cancel_current_operation(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }

        self.is_loading = false;

        // 更新最后一条流式消息为取消状态
        if let Some(msg) = self.messages.iter_mut().rev().find(|m| m.is_streaming) {
            msg.is_streaming = false;
            if msg.content.is_empty() {
                msg.content = t!("AiChat.operation_cancelled").to_string();
            } else {
                msg.content
                    .push_str(&t!("AiChat.operation_cancelled_markdown").to_string());
            }
        }
    }

    /// 是否可以取消
    pub fn can_cancel(&self) -> bool {
        self.is_loading && self.cancel_token.is_some()
    }

    // ========================================================================
    // 滚动控制
    // ========================================================================

    /// 自动滚动到底部
    pub fn scroll_to_bottom(&self) {
        if self.auto_scroll_enabled {
            self.scroll_handle.scroll_to_bottom();
        }
    }

    // ========================================================================
    // Provider 配置加载
    // ========================================================================

    /// 同步加载 provider 配置列表
    pub fn load_provider_configs_sync(storage: &StorageManager) -> Vec<ProviderConfig> {
        let repo = match storage.get::<ProviderRepository>() {
            Some(r) => r,
            None => return Vec::new(),
        };
        match repo.list() {
            Ok(all) => all.into_iter().filter(|p| p.enabled).collect(),
            Err(_) => Vec::new(),
        }
    }

    // ========================================================================
    // 历史会话辅助
    // ========================================================================

    /// 将历史会话消息转换为 ChatMessageUI
    pub fn messages_from_history(
        messages: &[crate::llm::chat_history::ChatMessage],
    ) -> Vec<ChatMessageUIGeneric<E>> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => ChatRole::User,
                    "assistant" => ChatRole::Assistant,
                    "system" => ChatRole::System,
                    _ => ChatRole::User,
                };
                ChatMessageUIGeneric::from_history(msg.id.to_string(), role, msg.content.clone())
            })
            .collect()
    }
}
