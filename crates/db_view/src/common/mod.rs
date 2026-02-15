mod database_editor_view;
pub mod db_connection_form;
mod schema_editor_view;

pub use database_editor_view::DatabaseEditorView;
pub use schema_editor_view::SchemaEditorView;

use db::plugin::DatabaseOperationRequest;

/// 数据库表单通用事件
/// 所有数据库类型的表单都应该发出这些事件
pub enum DatabaseFormEvent {
    FormChanged(DatabaseOperationRequest),
}

/// Schema 编辑器请求
#[derive(Clone, Debug)]
pub struct SchemaOperationRequest {
    pub schema_name: String,
    pub comment: Option<String>,
}

/// Schema 表单事件
pub enum SchemaFormEvent {
    FormChanged(SchemaOperationRequest),
}
