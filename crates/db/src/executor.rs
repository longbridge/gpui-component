use crate::types::FieldType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SQL 脚本来源
#[derive(Clone, Debug)]
pub enum SqlSource {
    /// 直接的 SQL 脚本字符串
    Script(String),
    /// SQL 文件路径
    File(PathBuf),
}

impl SqlSource {
    pub fn file_size(&self) -> Option<u64> {
        match self {
            SqlSource::Script(s) => Some(s.len() as u64),
            SqlSource::File(path) => std::fs::metadata(path).ok().map(|m| m.len()),
        }
    }

    pub fn is_file(&self) -> bool {
        matches!(self, SqlSource::File(_))
    }
}

/// Execution options for SQL script
#[derive(Debug, Clone)]
pub struct ExecOptions {
    /// Whether to stop execution when encountering an error
    pub stop_on_error: bool,
    /// Whether to wrap the entire script in a transaction
    pub transactional: bool,
    /// Maximum number of rows to return for query results
    pub max_rows: Option<usize>,
    /// 是否启用流式执行（逐条解析执行，适合大文件/大脚本）
    /// 默认 false，会先解析所有语句再执行
    pub streaming: bool,
}

impl Default for ExecOptions {
    fn default() -> Self {
        Self {
            stop_on_error: true,
            transactional: false,
            max_rows: Some(1000),
            streaming: false,
        }
    }
}

/// Result of a single SQL statement execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SqlResult {
    /// Query result (SELECT, SHOW, etc.)
    Query(QueryResult),
    /// Execution result (INSERT, UPDATE, DELETE, DDL, etc.)
    Exec(ExecResult),
    /// Error result
    Error(SqlErrorInfo),
}

impl SqlResult {
    pub fn is_error(&self) -> bool {
        matches!(self, SqlResult::Error(_))
    }
}

/// Column metadata for query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryColumnMeta {
    /// Column name
    pub name: String,
    /// Original database type (e.g., "VARCHAR(255)", "INT")
    pub db_type: String,
    /// Abstract field type for UI rendering
    pub field_type: FieldType,
    /// Whether the column is nullable
    pub nullable: bool,
}

impl QueryColumnMeta {
    pub fn new(name: impl Into<String>, db_type: impl Into<String>) -> Self {
        let db_type_str = db_type.into();
        let field_type = FieldType::from_db_type(&db_type_str);
        Self {
            name: name.into(),
            db_type: db_type_str,
            field_type,
            nullable: true,
        }
    }

    pub fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }
}

/// Query result with data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Original SQL statement
    pub sql: String,
    /// Column names
    pub columns: Vec<String>,
    /// Column metadata with type information
    pub column_meta: Vec<QueryColumnMeta>,
    /// Row data (each row is a vector of optional strings)
    pub rows: Vec<Vec<Option<String>>>,
    /// Execution time in milliseconds
    pub elapsed_ms: u128,
}

/// Execution result for non-query statements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    /// Original SQL statement
    pub sql: String,
    /// Number of rows affected
    pub rows_affected: u64,
    /// Execution time in milliseconds
    pub elapsed_ms: u128,
    /// Optional message
    pub message: Option<String>,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlErrorInfo {
    /// Original SQL statement
    pub sql: String,
    /// Error message
    pub message: String,
}

pub fn format_message(sql: &str, rows_affected: u64) -> String {
    let trimmed = sql.trim().to_uppercase();

    if trimmed.starts_with("INSERT") {
        format!("Inserted {} row(s)", rows_affected)
    } else if trimmed.starts_with("UPDATE") {
        format!("Updated {} row(s)", rows_affected)
    } else if trimmed.starts_with("DELETE") {
        format!("Deleted {} row(s)", rows_affected)
    } else if trimmed.starts_with("REPLACE") {
        format!("Replaced {} row(s)", rows_affected)
    } else if trimmed.starts_with("CREATE") {
        "Object created successfully".to_string()
    } else if trimmed.starts_with("ALTER") {
        "Object altered successfully".to_string()
    } else if trimmed.starts_with("DROP") {
        "Object dropped successfully".to_string()
    } else if trimmed.starts_with("TRUNCATE") {
        "Table truncated successfully".to_string()
    } else if trimmed.starts_with("RENAME") {
        "Object renamed successfully".to_string()
    } else if trimmed.starts_with("USE") {
        "Database changed successfully".to_string()
    } else if trimmed.starts_with("SET") {
        "Variable set successfully".to_string()
    } else if trimmed.starts_with("BEGIN") || trimmed.starts_with("START TRANSACTION") {
        "Transaction started".to_string()
    } else if trimmed.starts_with("COMMIT") {
        "Transaction committed".to_string()
    } else if trimmed.starts_with("ROLLBACK") {
        "Transaction rolled back".to_string()
    } else {
        format!(
            "Query executed successfully, {} row(s) affected",
            rows_affected
        )
    }
}

/// Statement type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementType {
    /// Query statement (SELECT, SHOW, etc.)
    Query,
    /// Data manipulation (INSERT, UPDATE, DELETE)
    Dml,
    /// Data definition (CREATE, ALTER, DROP)
    Ddl,
    /// Transaction control (BEGIN, COMMIT, ROLLBACK)
    Transaction,
    /// Database commands (USE, SET)
    Command,
    /// Other execution statements
    Exec,
}
