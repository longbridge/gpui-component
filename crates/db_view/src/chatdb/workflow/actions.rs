//! WorkflowAction - 工作流动作定义
//!
//! 定义工作流状态转换的动作

use super::{QueryContext, TableBrief, WorkflowState};

// ============================================================================
// 工作流动作
// ============================================================================

/// 工作流动作 - 指导状态转换
#[derive(Clone, Debug)]
pub enum WorkflowAction {
    /// 继续下一阶段
    Continue(WorkflowState),

    /// 需要 AI 选表
    NeedAiSelectTables {
        /// 选表 Prompt
        prompt: String,
        /// 候选表列表
        tables: Vec<TableBrief>,
        /// 警告信息（如表数量超过阈值）
        warning: Option<String>,
    },

    /// 准备好生成 SQL
    ReadyToGenerate {
        /// 查询上下文
        context: QueryContext,
    },

    /// 需要用户手动 @表
    RequireUserMention {
        /// 提示消息
        message: String,
        /// 表数量
        table_count: usize,
    },

    /// 进度更新
    Progress {
        /// 当前阶段
        stage: String,
        /// 已完成数量
        completed: usize,
        /// 总数量
        total: usize,
    },

    /// 错误
    Error(String),

    /// 已取消
    Cancelled,
}

impl WorkflowAction {
    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::ReadyToGenerate { .. }
                | Self::RequireUserMention { .. }
                | Self::Error(_)
                | Self::Cancelled
        )
    }

    /// 是否为错误
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_) | Self::Cancelled)
    }

    /// 获取错误消息
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg),
            Self::Cancelled => Some("操作已取消"),
            _ => None,
        }
    }

    /// 创建进度更新动作
    pub fn progress(stage: impl Into<String>, completed: usize, total: usize) -> Self {
        Self::Progress {
            stage: stage.into(),
            completed,
            total,
        }
    }
}
