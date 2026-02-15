//! ChatDB 工作流层
//!
//! 管理智能 SQL 生成的多阶段工作流：
//! - WorkflowState: 统一的工作流状态机（支持取消/重试）
//! - WorkflowAction: 工作流动作定义
//! - WorkflowExecutor: 工作流执行器（并行元数据获取）

mod state;
mod actions;
mod executor;

pub use state::*;
pub use actions::*;
pub use executor::*;

// 重导出旧模块中仍需使用的类型
pub use crate::chatdb::query_workflow::{
    parse_user_input, build_table_selection_prompt, parse_table_selection_response,
    ParsedInput, TableBrief, TableMeta, ColumnMeta, QueryContext,
    TABLE_COUNT_THRESHOLD,
};
