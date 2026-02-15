use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use rusqlite::{types::ValueRef, Connection, OpenFlags};
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tracing::{debug, error, info};

use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::{format_message, truncate_str, DatabasePlugin};
use one_core::storage::DbConnectionConfig;

pub struct SqliteDbConnection {
    config: DbConnectionConfig,
    connection: Arc<Mutex<Option<Connection>>>,
}

impl SqliteDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            connection: Arc::new(Mutex::new(None)),
        }
    }

    fn extract_value(value: ValueRef<'_>, decl_type: Option<&str>) -> Option<String> {
        match value {
            ValueRef::Null => None,
            ValueRef::Integer(i) => {
                let type_upper = decl_type.map(|t| t.to_uppercase());
                let is_datetime = type_upper.as_ref().map_or(false, |t| {
                    t.contains("DATE") || t.contains("TIME") || t.contains("TIMESTAMP")
                });

                if is_datetime {
                    let is_millis = i > 1_000_000_000_000;
                    if is_millis {
                        if let Some(dt) = chrono::DateTime::from_timestamp_millis(i) {
                            return Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                        }
                    } else {
                        if let Some(dt) = chrono::DateTime::from_timestamp(i, 0) {
                            return Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                        }
                    }
                }
                Some(i.to_string())
            }
            ValueRef::Real(f) => Some(f.to_string()),
            ValueRef::Text(t) => String::from_utf8(t.to_vec()).ok(),
            ValueRef::Blob(b) => {
                if let Ok(s) = String::from_utf8(b.to_vec()) {
                    Some(s)
                } else {
                    Some(format!("0x{}", hex::encode(b)))
                }
            }
        }
    }

    fn build_query_result(
        columns: Vec<String>,
        column_types: Vec<Option<String>>,
        rows: Vec<Vec<Option<String>>>,
        sql: String,
        elapsed_ms: u128,
    ) -> SqlResult {
        let column_meta: Vec<QueryColumnMeta> = columns
            .iter()
            .zip(column_types.iter())
            .map(|(name, decl_type)| {
                QueryColumnMeta::new(
                    name.clone(),
                    decl_type.clone().unwrap_or_else(|| "TEXT".to_string()),
                )
            })
            .collect();

        SqlResult::Query(QueryResult {
            sql,
            columns,
            column_meta,
            rows,
            elapsed_ms,
        })
    }

    fn build_exec_result(sql: String, rows_affected: u64, elapsed_ms: u128) -> SqlResult {
        let message = format_message(&sql, rows_affected);
        SqlResult::Exec(ExecResult {
            sql,
            rows_affected,
            elapsed_ms,
            message: Some(message),
        })
    }

    fn execute_statement(conn: &Connection, sql: &str, start: Instant) -> SqlResult {
        let sql_preview = if sql.len() > 200 {
            format!("{}...", truncate_str(sql, 200))
        } else {
            sql.to_string()
        };

        match conn.prepare(sql) {
            Ok(mut stmt) => {
                let column_count = stmt.column_count();

                if column_count == 0 {
                    match conn.execute(sql, []) {
                        Ok(rows_affected) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!(
                                "[SQLite] Execute completed: {} rows affected, {}ms",
                                rows_affected, elapsed_ms
                            );
                            Self::build_exec_result(
                                sql.to_string(),
                                rows_affected as u64,
                                elapsed_ms,
                            )
                        }
                        Err(e) => {
                            error!("[SQLite] Execute failed: {}, SQL: {}", e, sql_preview);
                            SqlResult::Error(SqlErrorInfo {
                                sql: sql.to_string(),
                                message: e.to_string(),
                            })
                        }
                    }
                } else {
                    let stmt_columns = stmt.columns();
                    let columns: Vec<String> =
                        stmt_columns.iter().map(|c| c.name().to_string()).collect();

                    let column_types: Vec<Option<String>> = stmt_columns
                        .iter()
                        .map(|c| c.decl_type().map(|s| s.to_string()))
                        .collect();

                    let rows_result: Result<Vec<Vec<Option<String>>>, rusqlite::Error> =
                        stmt.query([]).and_then(|mut rows| {
                            let mut data_rows = Vec::new();
                            while let Some(row) = rows.next()? {
                                let row_data: Vec<Option<String>> = (0..column_count)
                                    .map(|i| {
                                        let decl_type =
                                            column_types.get(i).and_then(|t| t.as_deref());
                                        row.get_ref(i)
                                            .ok()
                                            .and_then(|v| Self::extract_value(v, decl_type))
                                    })
                                    .collect();
                                data_rows.push(row_data);
                            }
                            Ok(data_rows)
                        });

                    match rows_result {
                        Ok(data_rows) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!(
                                "[SQLite] Query completed: {} rows, {} columns, {}ms",
                                data_rows.len(),
                                columns.len(),
                                elapsed_ms
                            );
                            Self::build_query_result(
                                columns,
                                column_types,
                                data_rows,
                                sql.to_string(),
                                elapsed_ms,
                            )
                        }
                        Err(e) => {
                            error!("[SQLite] Query failed: {}, SQL: {}", e, sql_preview);
                            SqlResult::Error(SqlErrorInfo {
                                sql: sql.to_string(),
                                message: e.to_string(),
                            })
                        }
                    }
                }
            }
            Err(e) => {
                error!("[SQLite] Prepare failed: {}, SQL: {}", e, sql_preview);
                SqlResult::Error(SqlErrorInfo {
                    sql: sql.to_string(),
                    message: e.to_string(),
                })
            }
        }
    }
}

#[async_trait]
impl DbConnection for SqliteDbConnection {
    fn config(&self) -> &DbConnectionConfig {
        &self.config
    }

    fn set_config_database(&mut self, database: Option<String>) {
        self.config.database = database;
    }

    fn supports_database_switch(&self) -> bool {
        false
    }

    async fn connect(&mut self) -> Result<(), DbError> {
        let config = self.config.clone();

        let database_path = if !config.host.is_empty() {
            config.host.clone()
        } else {
            config
                .database
                .clone()
                .ok_or_else(|| DbError::connection("database path is required for SQLite"))?
        };

        info!("[SQLite] Connecting to {}", database_path);

        let conn = spawn_blocking(move || {
            Connection::open_with_flags(
                &database_path,
                OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
            )
        })
        .await
        .map_err(|e| {
            error!("[SQLite] Task join error: {}", e);
            DbError::Internal(format!("task join error: {}", e))
        })?
        .map_err(|e| {
            error!("[SQLite] Connection failed: {}", e);
            DbError::connection_with_source("failed to connect", e)
        })?;

        debug!("[SQLite] Setting pragmas...");
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(|e| {
            error!("[SQLite] Failed to set pragmas: {}", e);
            DbError::connection_with_source("failed to set pragmas", e)
        })?;

        {
            let mut guard = self
                .connection
                .lock()
                .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
            *guard = Some(conn);
        }

        info!("[SQLite] Connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DbError> {
        debug!("[SQLite] Disconnecting...");
        let conn_opt = {
            let mut guard = self
                .connection
                .lock()
                .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
            guard.take()
        };

        if let Some(conn) = conn_opt {
            spawn_blocking(move || drop(conn)).await.map_err(|e| {
                error!("[SQLite] Disconnect failed: {}", e);
                DbError::Internal(format!("task join error: {}", e))
            })?;
        }

        info!("[SQLite] Disconnected");
        Ok(())
    }

    async fn execute(
        &self,
        plugin: &dyn DatabasePlugin,
        script: &str,
        options: ExecOptions,
    ) -> Result<Vec<SqlResult>, DbError> {
        debug!(
            "[SQLite] execute() called, stop_on_error={}",
            options.stop_on_error
        );
        let parser = plugin
            .create_parser(SqlSource::Script(script.to_string()))
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;
        let statements: Vec<String> = parser
            .filter_map(|r| r.ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        debug!("[SQLite] Split into {} statement(s)", statements.len());
        let mut results = Vec::new();

        for (idx, sql) in statements.iter().enumerate() {
            let sql = sql.trim();
            if sql.is_empty() {
                continue;
            }

            debug!(
                "[SQLite] Executing statement {}/{}",
                idx + 1,
                statements.len()
            );
            let start = Instant::now();
            let sql_owned = sql.to_string();
            let connection = Arc::clone(&self.connection);

            let result = spawn_blocking(move || {
                let guard = connection
                    .lock()
                    .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                let conn = guard.as_ref().ok_or(DbError::NotConnected)?;

                Ok(Self::execute_statement(conn, &sql_owned, start))
            })
            .await
            .map_err(|e| {
                error!("[SQLite] Task join error: {}", e);
                DbError::Internal(format!("task join error: {}", e))
            })??;

            let is_error = result.is_error();
            if is_error {
                debug!(
                    "[SQLite] Statement {}/{} returned error",
                    idx + 1,
                    statements.len()
                );
            }
            results.push(result);

            if is_error && options.stop_on_error {
                debug!("[SQLite] Stopping execution due to error (stop_on_error=true)");
                break;
            }
        }

        debug!(
            "[SQLite] execute() completed with {} result(s)",
            results.len()
        );
        Ok(results)
    }

    async fn query(&self, query: &str) -> Result<SqlResult, DbError> {
        debug!("[SQLite] query() called");
        let start = Instant::now();
        let query_owned = query.to_string();
        let connection = Arc::clone(&self.connection);

        let result = spawn_blocking(move || {
            let guard = connection
                .lock()
                .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
            let conn = guard.as_ref().ok_or(DbError::NotConnected)?;

            Ok(Self::execute_statement(conn, &query_owned, start))
        })
        .await
        .map_err(|e| {
            error!("[SQLite] Query task join error: {}", e);
            DbError::Internal(format!("task join error: {}", e))
        })??;

        Ok(result)
    }

    async fn current_database(&self) -> Result<Option<String>, DbError> {
        Ok(self.config.database.clone())
    }

    async fn switch_database(&self, _database: &str) -> Result<(), DbError> {
        Err(DbError::NotSupported(
            "SQLite does not support switching databases. Each database is a separate file connection.".to_string()
        ))
    }

    async fn execute_streaming(
        &self,
        plugin: &dyn DatabasePlugin,
        source: SqlSource,
        options: ExecOptions,
        sender: mpsc::Sender<StreamingProgress>,
    ) -> Result<(), DbError> {
        debug!(
            "[SQLite] execute_streaming() called, transactional={}, streaming={}",
            options.transactional, options.streaming
        );

        let total_size = source.file_size().unwrap_or(0);
        let is_file_source = source.is_file();

        let mut parser = plugin
            .create_parser(source)
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;

        if options.streaming || is_file_source {
            let mut current = 0usize;

            if options.transactional {
                debug!("[SQLite] Starting transaction for streaming...");
                {
                    let connection = Arc::clone(&self.connection);
                    spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        conn.execute("BEGIN", []).map_err(|e| {
                            DbError::transaction_with_source("failed to begin transaction", e)
                        })?;
                        Ok::<_, DbError>(())
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;
                }

                let mut has_error = false;

                while let Some(stmt_result) = parser.next() {
                    let bytes_read = parser.bytes_read();
                    let sql = match stmt_result {
                        Ok(s) if !s.trim().is_empty() => s,
                        Ok(_) => continue,
                        Err(e) => {
                            let progress = StreamingProgress::with_file_progress(
                                current,
                                SqlResult::Error(SqlErrorInfo {
                                    sql: String::new(),
                                    message: format!("Parse error: {}", e),
                                }),
                                bytes_read,
                                total_size,
                            );
                            let _ = sender.send(progress).await;
                            has_error = true;
                            break;
                        }
                    };

                    current += 1;
                    debug!("[SQLite] Streaming TX statement {}", current);
                    let start = Instant::now();
                    let sql_owned = sql.clone();
                    let connection = Arc::clone(&self.connection);

                    let result = spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        Ok(Self::execute_statement(conn, &sql_owned, start))
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;

                    let is_error = result.is_error();
                    if is_error {
                        has_error = true;
                    }

                    let progress = StreamingProgress::with_file_progress(
                        current, result, bytes_read, total_size,
                    );
                    if sender.send(progress).await.is_err() {
                        break;
                    }

                    if is_error {
                        break;
                    }
                }

                {
                    let connection = Arc::clone(&self.connection);
                    let command = if has_error { "ROLLBACK" } else { "COMMIT" };
                    spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        conn.execute(command, []).map_err(|e| {
                            DbError::transaction_with_source(
                                format!("failed to {}", command.to_lowercase()),
                                e,
                            )
                        })?;
                        Ok::<_, DbError>(())
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;
                }
            } else {
                while let Some(stmt_result) = parser.next() {
                    let bytes_read = parser.bytes_read();
                    let sql = match stmt_result {
                        Ok(s) if !s.trim().is_empty() => s,
                        Ok(_) => continue,
                        Err(e) => {
                            let progress = StreamingProgress::with_file_progress(
                                current,
                                SqlResult::Error(SqlErrorInfo {
                                    sql: String::new(),
                                    message: format!("Parse error: {}", e),
                                }),
                                bytes_read,
                                total_size,
                            );
                            let _ = sender.send(progress).await;
                            if options.stop_on_error {
                                break;
                            }
                            continue;
                        }
                    };

                    current += 1;
                    debug!("[SQLite] Streaming statement {}", current);
                    let start = Instant::now();
                    let sql_owned = sql.clone();
                    let connection = Arc::clone(&self.connection);

                    let result = spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        Ok(Self::execute_statement(conn, &sql_owned, start))
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;

                    let is_error = result.is_error();
                    let progress = StreamingProgress::with_file_progress(
                        current, result, bytes_read, total_size,
                    );
                    if sender.send(progress).await.is_err() {
                        break;
                    }

                    if is_error && options.stop_on_error {
                        break;
                    }
                }
            }
        } else {
            let statements: Vec<String> = parser
                .filter_map(|r| r.ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let total = statements.len();
            debug!("[SQLite] Streaming {} statement(s)", total);

            if options.transactional {
                debug!("[SQLite] Starting transaction for streaming...");
                {
                    let connection = Arc::clone(&self.connection);
                    spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        conn.execute("BEGIN", []).map_err(|e| {
                            DbError::transaction_with_source("failed to begin transaction", e)
                        })?;
                        Ok::<_, DbError>(())
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;
                }

                let mut has_error = false;

                for (index, sql) in statements.into_iter().enumerate() {
                    let current = index + 1;
                    debug!("[SQLite] Streaming TX statement {}/{}", current, total);
                    let start = Instant::now();
                    let sql_owned = sql.clone();
                    let connection = Arc::clone(&self.connection);

                    let result = spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        Ok(Self::execute_statement(conn, &sql_owned, start))
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;

                    let is_error = result.is_error();
                    if is_error {
                        has_error = true;
                    }

                    let progress = StreamingProgress::new(current, total, result);
                    if sender.send(progress).await.is_err() {
                        break;
                    }

                    if is_error {
                        break;
                    }
                }

                {
                    let connection = Arc::clone(&self.connection);
                    let command = if has_error { "ROLLBACK" } else { "COMMIT" };
                    spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        conn.execute(command, []).map_err(|e| {
                            DbError::transaction_with_source(
                                format!("failed to {}", command.to_lowercase()),
                                e,
                            )
                        })?;
                        Ok::<_, DbError>(())
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;
                }
            } else {
                for (index, sql) in statements.into_iter().enumerate() {
                    let current = index + 1;
                    debug!("[SQLite] Streaming statement {}/{}", current, total);
                    let start = Instant::now();
                    let sql_owned = sql.clone();
                    let connection = Arc::clone(&self.connection);

                    let result = spawn_blocking(move || {
                        let guard = connection
                            .lock()
                            .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
                        let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                        Ok(Self::execute_statement(conn, &sql_owned, start))
                    })
                    .await
                    .map_err(|e| DbError::Internal(format!("task join error: {}", e)))??;

                    let is_error = result.is_error();
                    let progress = StreamingProgress::new(current, total, result);
                    if sender.send(progress).await.is_err() {
                        break;
                    }

                    if is_error && options.stop_on_error {
                        break;
                    }
                }
            }
        }

        debug!("[SQLite] execute_streaming() completed");
        Ok(())
    }
}
