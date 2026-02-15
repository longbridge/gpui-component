//! AI Chat 共享类型定义
//!
//! 此模块包含 AI 聊天面板的核心类型，可被不同的面板实现复用。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;
use crate::llm::{Message, MessageBlock, Role};

// ============================================================================
// 常量
// ============================================================================

/// 消息渲染限制
pub const MESSAGE_RENDER_LIMIT: usize = 60;
/// 消息渲染步进
pub const MESSAGE_RENDER_STEP: usize = 40;

// ============================================================================
// 聊天角色
// ============================================================================

/// 聊天角色
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatRole {
    /// 用户
    User,
    /// AI 助手
    Assistant,
    /// 系统
    System,
}

// ============================================================================
// 消息变体
// ============================================================================

/// 消息变体类型
#[derive(Clone, Debug, PartialEq)]
pub enum MessageVariant {
    /// 普通文本
    Text,
    /// SQL 结果（用于显示查询结果）
    SqlResult,
    /// 状态消息（用于显示处理进度）
    Status {
        /// 状态标题
        title: String,
        /// 是否已完成
        is_done: bool,
    },
}

// ============================================================================
// 聊天消息
// ============================================================================

/// UI 聊天消息结构
///
/// 包含消息的所有 UI 相关状态，如流式状态、是否展开等。
#[derive(Clone, Debug)]
pub struct ChatMessageUI {
    /// 消息唯一标识
    pub id: String,
    /// 消息角色
    pub role: ChatRole,
    /// 消息内容
    pub content: String,
    /// 消息变体
    pub variant: MessageVariant,
    /// 是否正在流式输出
    pub is_streaming: bool,
    /// 是否展开（用于可折叠的消息）
    pub is_expanded: bool,
    /// 内容缓存（用于避免重复解析）
    cached_content_hash: Option<u64>,
}

impl ChatMessageUI {
    /// 创建用户消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::User,
            content: content.into(),
            variant: MessageVariant::Text,
            is_streaming: false,
            is_expanded: true,
            cached_content_hash: None,
        }
    }

    /// 创建助手消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: content.into(),
            variant: MessageVariant::Text,
            is_streaming: false,
            is_expanded: true,
            cached_content_hash: None,
        }
    }

    /// 创建系统消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::System,
            content: content.into(),
            variant: MessageVariant::Text,
            is_streaming: false,
            is_expanded: true,
            cached_content_hash: None,
        }
    }

    /// 创建状态消息
    pub fn status(title: impl Into<String>, is_done: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: String::new(),
            variant: MessageVariant::Status {
                title: title.into(),
                is_done,
            },
            is_streaming: !is_done,
            is_expanded: !is_done,
            cached_content_hash: None,
        }
    }

    /// 创建流式助手消息
    pub fn streaming_assistant() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: String::new(),
            variant: MessageVariant::Text,
            is_streaming: true,
            is_expanded: true,
            cached_content_hash: None,
        }
    }

    /// 从历史消息创建
    pub fn from_history(id: impl Into<String>, role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role,
            content: content.into(),
            variant: MessageVariant::Text,
            is_streaming: false,
            is_expanded: true,
            cached_content_hash: None,
        }
    }

    /// 设置 ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// 设置变体
    pub fn with_variant(mut self, variant: MessageVariant) -> Self {
        self.variant = variant;
        self
    }

    /// 设置流式状态
    pub fn with_streaming(mut self, is_streaming: bool) -> Self {
        self.is_streaming = is_streaming;
        self
    }

    /// 设置内容
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self.cached_content_hash = None;
        self
    }

    /// 计算内容哈希
    pub fn content_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        hasher.finish()
    }

    /// 检查缓存是否有效
    pub fn is_cache_valid(&self) -> bool {
        self.cached_content_hash
            .map(|hash| hash == self.content_hash())
            .unwrap_or(false)
    }

    /// 更新缓存哈希
    pub fn update_cache(&mut self) {
        self.cached_content_hash = Some(self.content_hash());
    }

    /// 流式消息结束时调用
    pub fn finalize_streaming(&mut self) {
        self.is_streaming = false;
        self.cached_content_hash = None;
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.cached_content_hash = None;
    }

    /// 转换为 LLM 消息
    pub fn to_llm_message(&self) -> Message {
        let role = match self.role {
            ChatRole::User => Role::User,
            ChatRole::Assistant => Role::Assistant,
            ChatRole::System => Role::System,
        };
        Message::text(role, &self.content)
    }

    /// 从 LLM 消息创建
    pub fn from_llm_message(llm_msg: &Message) -> Self {
        let role = match llm_msg.role {
            Role::User => ChatRole::User,
            Role::Assistant => ChatRole::Assistant,
            Role::System => ChatRole::System,
            Role::Tool => ChatRole::Assistant,
        };

        let content = llm_msg
            .content
            .iter()
            .filter_map(|block| {
                if let MessageBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            variant: MessageVariant::Text,
            is_streaming: false,
            is_expanded: true,
            cached_content_hash: None,
        }
    }
}

// ============================================================================
// Provider 选择项
// ============================================================================

/// Provider 配置项（用于 UI 选择）
#[derive(Clone, Debug)]
pub struct ProviderSelectItem {
    /// Provider ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 默认模型
    pub model: String,
    /// Provider 类型名称
    pub provider_type: String,
    /// 可用模型列表
    pub models: Vec<String>,
    /// 是否为默认 provider
    pub is_default: bool,
}

impl ProviderSelectItem {
    /// 创建新的 Provider 选择项
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        model: impl Into<String>,
        provider_type: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: model.into(),
            provider_type: provider_type.into(),
            models: Vec::new(),
            is_default: false,
        }
    }

    /// 设置可用模型列表
    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }

    /// 设置是否为默认
    pub fn with_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    /// 获取显示名称
    pub fn display_name(&self) -> String {
        format!("{} - {} ({})", self.provider_type, self.model, self.name)
    }
}

/// 模型选择项
#[derive(Clone, Debug)]
pub struct ModelSelectItem {
    /// 模型 ID
    pub id: String,
}

impl ModelSelectItem {
    /// 创建新的模型选择项
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}
