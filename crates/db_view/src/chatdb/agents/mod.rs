//! ChatDB Agent 模块
//!
//! 将 SQL 工作流接入 Agent 框架：
//! - SqlWorkflowAgent: SQL 查询生成 Agent
//! - ChatBiAgent: 数据分析与图表 Agent
//! - DatabaseMetadataProvider: 数据库元数据访问能力

pub mod chat_bi;
pub mod db_metadata;
pub mod query_workflow;
pub mod sql_workflow;

pub use chat_bi::ChatBiAgent;
pub use db_metadata::{CAP_DB_METADATA, DatabaseMetadataProvider};
pub use sql_workflow::SqlWorkflowAgent;

use gpui::{App, BorrowAppContext};
use one_core::agent::registry::AgentRegistry;

/// Register SqlWorkflowAgent into the global AgentRegistry.
pub fn init(cx: &mut App) {
    cx.update_global::<AgentRegistry, _>(|registry: &mut AgentRegistry, _| {
        registry.register(ChatBiAgent);
        registry.register(SqlWorkflowAgent);
    });
}
