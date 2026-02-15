//! WorkflowState - 统一的工作流状态机
//!
//! 支持取消和重试功能

use one_core::storage::DatabaseType;
use tokio_util::sync::CancellationToken;

use super::{QueryContext, TableBrief};

// ============================================================================
// 重试上下文
// ============================================================================

/// 重试上下文 - 保存重试所需的信息
#[derive(Clone, Debug)]
pub struct RetryContext {
    /// 用户问题
    pub user_question: String,
    /// 提及的表名
    pub mentioned_tables: Vec<String>,
    /// 连接 ID
    pub connection_id: String,
    /// 数据库名
    pub database: Option<String>,
    /// Schema 名
    pub schema: Option<String>,
    /// 数据库类型
    pub database_type: DatabaseType,
}

// ============================================================================
// 工作流状态
// ============================================================================

/// 统一的工作流状态
#[derive(Clone, Debug)]
pub enum WorkflowState {
    /// 空闲状态
    Idle,

    /// 分析中（获取表列表）
    Analyzing {
        /// 用户问题
        user_question: String,
        /// 提及的表名
        mentioned_tables: Vec<String>,
        /// 取消令牌
        cancel_token: CancellationToken,
    },

    /// AI 选表中
    SelectingTables {
        /// 用户问题
        user_question: String,
        /// 候选表列表
        tables: Vec<TableBrief>,
        /// 取消令牌
        cancel_token: CancellationToken,
    },

    /// 获取元数据中
    FetchingMetadata {
        /// 用户问题
        user_question: String,
        /// 选中的表名
        selected_tables: Vec<String>,
        /// 是否用户主动 @表
        is_user_mentioned: bool,
        /// 取消令牌
        cancel_token: CancellationToken,
        /// 进度：(已完成, 总数)
        progress: (usize, usize),
    },

    /// 生成 SQL 中
    GeneratingSql {
        /// 查询上下文
        context: QueryContext,
        /// 取消令牌
        cancel_token: CancellationToken,
    },

    /// 等待用户输入（需要 @表）
    WaitingForInput {
        /// 提示消息
        message: String,
        /// 表数量
        table_count: usize,
    },

    /// 错误状态（可重试）
    Error {
        /// 错误消息
        message: String,
        /// 重试上下文（如果可重试）
        retry_context: Option<RetryContext>,
    },
}

impl WorkflowState {
    /// 获取当前状态的取消令牌（如果可取消）
    pub fn cancel_token(&self) -> Option<&CancellationToken> {
        match self {
            Self::Analyzing { cancel_token, .. }
            | Self::SelectingTables { cancel_token, .. }
            | Self::FetchingMetadata { cancel_token, .. }
            | Self::GeneratingSql { cancel_token, .. } => Some(cancel_token),
            _ => None,
        }
    }

    /// 是否可取消
    pub fn is_cancellable(&self) -> bool {
        self.cancel_token().is_some()
    }

    /// 是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Error {
                retry_context: Some(_),
                ..
            }
        )
    }

    /// 获取重试上下文
    pub fn retry_context(&self) -> Option<&RetryContext> {
        match self {
            Self::Error { retry_context, .. } => retry_context.as_ref(),
            _ => None,
        }
    }

    /// 是否正在加载
    pub fn is_loading(&self) -> bool {
        matches!(
            self,
            Self::Analyzing { .. }
                | Self::SelectingTables { .. }
                | Self::FetchingMetadata { .. }
                | Self::GeneratingSql { .. }
        )
    }

    /// 获取状态显示文本
    pub fn display_text(&self) -> &'static str {
        match self {
            Self::Idle => "就绪",
            Self::Analyzing { .. } => "分析查询意图...",
            Self::SelectingTables { .. } => "AI 选择相关表...",
            Self::FetchingMetadata { .. } => "获取表结构信息...",
            Self::GeneratingSql { .. } => "生成 SQL...",
            Self::WaitingForInput { .. } => "等待输入",
            Self::Error { .. } => "发生错误",
        }
    }

    /// 获取进度（如果有）
    pub fn progress(&self) -> Option<(usize, usize)> {
        match self {
            Self::FetchingMetadata { progress, .. } => Some(*progress),
            _ => None,
        }
    }

    /// 取消当前操作
    pub fn cancel(&self) {
        if let Some(token) = self.cancel_token() {
            token.cancel();
        }
    }

    /// 创建分析状态
    pub fn analyzing(user_question: String, mentioned_tables: Vec<String>) -> Self {
        Self::Analyzing {
            user_question,
            mentioned_tables,
            cancel_token: CancellationToken::new(),
        }
    }

    /// 创建选表状态
    pub fn selecting_tables(user_question: String, tables: Vec<TableBrief>) -> Self {
        Self::SelectingTables {
            user_question,
            tables,
            cancel_token: CancellationToken::new(),
        }
    }

    /// 创建获取元数据状态
    pub fn fetching_metadata(
        user_question: String,
        selected_tables: Vec<String>,
        is_user_mentioned: bool,
    ) -> Self {
        let total = selected_tables.len();
        Self::FetchingMetadata {
            user_question,
            selected_tables,
            is_user_mentioned,
            cancel_token: CancellationToken::new(),
            progress: (0, total),
        }
    }

    /// 创建生成 SQL 状态
    pub fn generating_sql(context: QueryContext) -> Self {
        Self::GeneratingSql {
            context,
            cancel_token: CancellationToken::new(),
        }
    }

    /// 创建错误状态（可重试）
    pub fn error_with_retry(message: String, retry_context: RetryContext) -> Self {
        Self::Error {
            message,
            retry_context: Some(retry_context),
        }
    }

    /// 创建错误状态（不可重试）
    pub fn error(message: String) -> Self {
        Self::Error {
            message,
            retry_context: None,
        }
    }
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self::Idle
    }
}
