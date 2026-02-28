use crate::executor::{ExecOptions, SqlResult, SqlSource};
use crate::DatabasePlugin;
use async_trait::async_trait;
use one_core::storage::DbConnectionConfig;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("connection error: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("query error: {message}")]
    Query {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("transaction error: {message}")]
    Transaction {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("not connected to database")]
    NotConnected,

    #[error("operation not supported: {0}")]
    NotSupported(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl DbError {
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            source: None,
        }
    }

    pub fn connection_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let message = format!("{}: {}", message.into(), source);
        Self::Connection {
            message,
            source: Some(Box::new(source)),
        }
    }

    pub fn query(message: impl Into<String>) -> Self {
        Self::Query {
            message: message.into(),
            source: None,
        }
    }

    pub fn query_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let message = format!("{}: {}", message.into(), source);
        Self::Query {
            message,
            source: Some(Box::new(source)),
        }
    }

    pub fn transaction(message: impl Into<String>) -> Self {
        Self::Transaction {
            message: message.into(),
            source: None,
        }
    }

    pub fn transaction_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let message = format!("{}: {}", message.into(), source);
        Self::Transaction {
            message,
            source: Some(Box::new(source)),
        }
    }
}

/// 流式执行进度信息
#[derive(Clone, Debug)]
pub struct StreamingProgress {
    pub current: usize,
    pub total: usize,
    pub result: SqlResult,
    /// 已读取字节数（文件流式模式）
    pub bytes_read: u64,
    /// 文件总大小（文件流式模式，0 表示非文件模式）
    pub file_size: u64,
}

impl StreamingProgress {
    pub fn new(current: usize, total: usize, result: SqlResult) -> Self {
        Self {
            current,
            total,
            result,
            bytes_read: 0,
            file_size: 0,
        }
    }

    pub fn with_file_progress(
        current: usize,
        result: SqlResult,
        bytes_read: u64,
        file_size: u64,
    ) -> Self {
        Self {
            current,
            total: 0,
            result,
            bytes_read,
            file_size,
        }
    }

    /// 计算进度百分比
    /// 文件模式使用字节比例，脚本模式使用语句比例
    pub fn progress_percent(&self) -> f32 {
        if self.file_size > 0 {
            (self.bytes_read as f64 / self.file_size as f64 * 100.0) as f32
        } else if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as f32
        } else {
            0.0
        }
    }
}

#[async_trait]
pub trait DbConnection: Sync + Send {
    fn config(&self) -> &DbConnectionConfig;

    /// Update database field in config (used when connection's actual database changes)
    fn set_config_database(&mut self, database: Option<String>);

    /// Whether this database type supports switching database within a connection
    fn supports_database_switch(&self) -> bool {
        true
    }

    async fn connect(&mut self) -> Result<(), DbError>;
    async fn disconnect(&mut self) -> Result<(), DbError>;
    async fn execute(
        &self,
        plugin: &dyn DatabasePlugin,
        script: &str,
        options: ExecOptions,
    ) -> Result<Vec<SqlResult>, DbError>;
    async fn query(&self, query: &str) -> Result<SqlResult, DbError>;

    async fn ping(&self) -> Result<(), DbError> {
        self.query("SELECT 1").await.map(|_| ())
    }

    /// Get current database/schema name from the connection
    async fn current_database(&self) -> Result<Option<String>, DbError>;

    /// Switch to a different database
    async fn switch_database(&self, database: &str) -> Result<(), DbError>;

    /// Switch to a different schema within the current database
    /// For PostgreSQL: SET search_path TO schema
    /// For Oracle: Uses switch_database instead (schema = user)
    /// Other databases: No-op (use schema table in SQL)
    async fn switch_schema(&self, _schema: &str) -> Result<(), DbError> {
        Ok(())
    }

    async fn execute_streaming(
        &self,
        plugin: &dyn DatabasePlugin,
        source: SqlSource,
        options: ExecOptions,
        sender: mpsc::Sender<StreamingProgress>,
    ) -> Result<(), DbError>;
}
