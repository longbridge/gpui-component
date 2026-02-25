rust_i18n::i18n!("locales", fallback = "en");

pub mod clickhouse;
pub mod common;
pub mod connection_form_window;
pub mod database_objects_tab;
pub mod database_tab;
pub mod database_view_plugin;
mod db_tree_event;
pub mod db_tree_view;
mod import_export;
pub mod mssql;
pub mod mysql;
pub mod oracle;
pub mod postgresql;
mod sidebar;
pub mod sql_editor;
pub(crate) mod sql_inline_completion;
#[cfg(test)]
mod sql_editor_completion_tests;
pub mod sql_editor_view;
pub mod sql_result_tab;
pub mod sqlite;
mod table_data;
pub mod table_data_tab;
pub mod table_designer_tab;
pub mod chatdb;

pub use one_core::ai_chat::ask_ai::{emit_ask_ai_event, init_ask_ai_notifier, AskAiButton};
pub use common::DatabaseFormEvent;
