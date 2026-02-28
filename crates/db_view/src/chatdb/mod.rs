pub mod agents;
pub mod ai_input;
pub mod chat_markdown;
pub mod chat_panel;
pub mod chat_sql_block;
pub mod chat_sql_result;
pub mod components;
pub mod db_connection_selector;
pub mod query_workflow;
pub mod services;
pub mod sql_query_detector;

// 重导出常用类型
pub use components::{ChatMessageUI, ChatRole, MESSAGE_RENDER_LIMIT, MessageVariant, SqlExtension};
// 从核心库重导出模型设置组件和会话服务
pub use one_core::ai_chat::components::{ModelSettings, ModelSettingsEvent, ModelSettingsPanel};
pub use one_core::ai_chat::services::{SessionError, SessionService, extract_session_name};
pub use services::SqlService;
