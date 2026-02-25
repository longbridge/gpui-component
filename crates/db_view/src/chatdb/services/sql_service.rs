//! SqlService - SQL 执行服务
//!
//! 管理 SQL 语句的执行

use db::{GlobalDbState, SqlResult};
use gpui::AsyncApp;
use one_core::storage::DatabaseType;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use rust_i18n::t;

// ============================================================================
// 错误类型
// ============================================================================

/// SqlService 错误类型
#[derive(Debug, Clone)]
pub enum SqlError {
    /// 执行错误
    ExecutionError(String),
    /// 请求已取消
    Cancelled,
    /// 连接未找到
    ConnectionNotFound,
}

impl std::fmt::Display for SqlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlError::ExecutionError(msg) => {
                write!(f, "{}", t!("SqlService.execution_error", message = msg))
            }
            SqlError::Cancelled => write!(f, "{}", t!("SqlService.cancelled")),
            SqlError::ConnectionNotFound => write!(f, "{}", t!("SqlService.connection_not_found")),
        }
    }
}

impl std::error::Error for SqlError {}

// ============================================================================
// 执行结果
// ============================================================================

/// SQL 执行结果
#[derive(Debug, Clone)]
pub struct SqlExecutionResult {
    /// SQL 语句
    pub sql: String,
    /// 执行结果列表
    pub results: Vec<SqlResult>,
    /// 数据库类型
    pub database_type: DatabaseType,
    /// 执行耗时（毫秒）
    pub elapsed_ms: u64,
}

// ============================================================================
// SqlService
// ============================================================================

/// SQL 执行服务
pub struct SqlService {
    global_db_state: Arc<GlobalDbState>,
}

impl SqlService {
    /// 创建新的 SqlService
    pub fn new(global_db_state: Arc<GlobalDbState>) -> Self {
        Self { global_db_state }
    }

    /// 从 GlobalDbState 引用创建
    pub fn from_ref(global_db_state: &GlobalDbState) -> Self {
        Self {
            global_db_state: Arc::new(global_db_state.clone()),
        }
    }

    /// 执行 SQL 脚本（可取消）
    pub async fn execute_script(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        sql: String,
        database: Option<String>,
        schema: Option<String>,
        cancel_token: Option<CancellationToken>,
    ) -> Result<SqlExecutionResult, SqlError> {
        let start = std::time::Instant::now();
        let global_state = self.global_db_state.clone();
        let sql_clone = sql.clone();

        // 获取数据库类型
        let database_type = global_state
            .get_config(&connection_id)
            .map(|c| c.database_type)
            .unwrap_or(DatabaseType::MySQL);

        let execute_future = async {
            global_state
                .execute_script(cx, connection_id, sql_clone, database, schema, None)
                .await
                .map_err(|e| SqlError::ExecutionError(e.to_string()))
        };

        let results = if let Some(token) = cancel_token {
            tokio::select! {
                result = execute_future => result?,
                _ = token.cancelled() => return Err(SqlError::Cancelled),
            }
        } else {
            execute_future.await?
        };

        let elapsed_ms = start.elapsed().as_millis() as u64;

        Ok(SqlExecutionResult {
            sql,
            results,
            database_type,
            elapsed_ms,
        })
    }

    /// 判断 SQL 是否为查询语句
    pub fn is_query_statement(&self, connection_id: &str, sql: &str) -> bool {
        crate::chatdb::sql_query_detector::is_query_statement_for_connection(
            &self.global_db_state,
            connection_id,
            sql,
        )
    }

    /// 获取数据库类型
    pub fn get_database_type(&self, connection_id: &str) -> DatabaseType {
        self.global_db_state
            .get_config(connection_id)
            .map(|c| c.database_type)
            .unwrap_or(DatabaseType::MySQL)
    }

    /// 获取 GlobalDbState 的引用
    pub fn global_db_state(&self) -> &GlobalDbState {
        &self.global_db_state
    }
}

impl Clone for SqlService {
    fn clone(&self) -> Self {
        Self {
            global_db_state: self.global_db_state.clone(),
        }
    }
}
