//! AI Chat Services - 共用服务层
//!
//! 提供 AI 聊天面板共用的服务功能：
//! - SessionService: 会话持久化服务

use crate::llm::chat_history::{ChatMessage, ChatSession, MessageRepository, SessionRepository};
use crate::storage::StorageManager;
use crate::storage::traits::Repository;
use rust_i18n::t;

// ============================================================================
// 错误类型
// ============================================================================

/// SessionService 错误类型
#[derive(Debug, Clone)]
pub enum SessionError {
    /// 仓库不可用
    RepositoryNotAvailable,
    /// 会话未找到
    SessionNotFound,
    /// 存储错误
    StorageError(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::RepositoryNotAvailable => {
                write!(f, "{}", t!("AiChat.session_repo_unavailable"))
            }
            SessionError::SessionNotFound => write!(f, "{}", t!("AiChat.session_not_found")),
            SessionError::StorageError(msg) => {
                write!(f, "{}", t!("AiChat.session_storage_error", error = msg))
            }
        }
    }
}

impl std::error::Error for SessionError {}

// ============================================================================
// SessionService
// ============================================================================

/// 会话持久化服务
#[derive(Clone)]
pub struct SessionService {
    storage_manager: StorageManager,
}

impl SessionService {
    /// 创建新的 SessionService
    pub fn new(storage_manager: StorageManager) -> Self {
        Self { storage_manager }
    }

    /// 创建新会话
    pub fn create_session(&self, name: String, provider_id: String) -> Result<i64, SessionError> {
        let session_repo = self
            .storage_manager
            .get::<SessionRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        let mut session = ChatSession::new(name, provider_id);
        session_repo
            .insert(&mut session)
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 获取会话
    pub fn get_session(&self, session_id: i64) -> Result<Option<ChatSession>, SessionError> {
        let session_repo = self
            .storage_manager
            .get::<SessionRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        session_repo
            .get(session_id)
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Result<Vec<ChatSession>, SessionError> {
        let session_repo = self
            .storage_manager
            .get::<SessionRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        session_repo
            .list()
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 删除会话
    pub fn delete_session(&self, session_id: i64) -> Result<(), SessionError> {
        let session_repo = self
            .storage_manager
            .get::<SessionRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        session_repo
            .delete(session_id)
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 更新会话名称
    pub fn update_session_name(&self, session_id: i64, name: String) -> Result<(), SessionError> {
        let session_repo = self
            .storage_manager
            .get::<SessionRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        let mut session = session_repo
            .get(session_id)
            .map_err(|e| SessionError::StorageError(e.to_string()))?
            .ok_or(SessionError::SessionNotFound)?;

        session.name = name;
        session_repo
            .update(&session)
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 添加用户消息
    pub fn add_user_message(&self, session_id: i64, content: String) -> Result<i64, SessionError> {
        let message_repo = self
            .storage_manager
            .get::<MessageRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        let mut message = ChatMessage::user(session_id, content);
        message_repo
            .insert(&mut message)
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 添加助手消息
    pub fn add_assistant_message(
        &self,
        session_id: i64,
        content: String,
    ) -> Result<i64, SessionError> {
        let message_repo = self
            .storage_manager
            .get::<MessageRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        let mut message = ChatMessage::assistant(session_id, content);
        message_repo
            .insert(&mut message)
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 获取会话的所有消息
    pub fn get_messages(&self, session_id: i64) -> Result<Vec<ChatMessage>, SessionError> {
        let message_repo = self
            .storage_manager
            .get::<MessageRepository>()
            .ok_or(SessionError::RepositoryNotAvailable)?;

        message_repo
            .list_by_session(session_id)
            .map_err(|e| SessionError::StorageError(e.to_string()))
    }

    /// 确保会话存在，不存在则创建
    pub fn ensure_session(
        &self,
        session_id: Option<i64>,
        provider_id: &str,
        default_name: &str,
    ) -> Result<i64, SessionError> {
        if let Some(id) = session_id {
            // 验证会话存在
            if self.get_session(id)?.is_some() {
                return Ok(id);
            }
        }

        // 创建新会话
        self.create_session(default_name.to_string(), provider_id.to_string())
    }

    /// 获取存储管理器的引用
    pub fn storage_manager(&self) -> &StorageManager {
        &self.storage_manager
    }
}

// ============================================================================
// 会话标题工具函数
// ============================================================================

/// 从消息内容提取会话名称（取前 20 个字符）
pub fn extract_session_name(content: &str) -> String {
    let clean_content = content.trim();
    if clean_content.chars().count() <= 20 {
        clean_content.to_string()
    } else {
        format!("{}...", clean_content.chars().take(17).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_name_short() {
        assert_eq!(extract_session_name("Hello"), "Hello");
        assert_eq!(extract_session_name("  Hello  "), "Hello");
    }

    #[test]
    fn test_extract_session_name_long() {
        let long_text = "这是一个非常长的会话标题，需要被截断";
        let result = extract_session_name(long_text);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 20);
    }
}
