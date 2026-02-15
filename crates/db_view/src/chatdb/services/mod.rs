//! ChatDB 服务层
//!
//! 将业务逻辑从 UI 组件中分离，提供独立的服务抽象：
//! - SqlService: SQL 执行服务
//!
//! 注意: SessionService 已移至 one_core::ai_chat::services
//! 注意: ChatService 已被 Agent 框架取代

mod sql_service;

pub use sql_service::*;
