//! ChatDB Agent 模块
//!
//! 将 SQL 工作流接入 Agent 框架：
//! - SqlWorkflowAgent: SQL 查询生成 Agent
//! - DatabaseMetadataProvider: 数据库元数据访问能力

pub mod db_metadata;
pub mod sql_workflow;

pub use db_metadata::{CAP_DB_METADATA, DatabaseMetadataProvider};
pub use sql_workflow::SqlWorkflowAgent;

use gpui::{App, BorrowAppContext};
use one_core::agent::registry::AgentRegistry;

/// Register SqlWorkflowAgent into the global AgentRegistry.
pub fn init(cx: &mut App) {
    cx.update_global::<AgentRegistry, _>(|registry: &mut AgentRegistry, _| {
        registry.register(SqlWorkflowAgent);
    });
}
