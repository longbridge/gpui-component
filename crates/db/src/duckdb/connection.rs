use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use duckdb::{types::ValueRef, Connection};
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tracing::{debug, error, info};

use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::{format_message, truncate_str, DatabasePlugin};
use one_core::storage::DbConnectionConfig;

pub struct DuckDbConnection {
    config: DbConnectionConfig,
    connection: Arc<Mutex<Option<Connection>>>,
}

impl DuckDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            connection: Arc::new(Mutex::new(None)),
        }
    }

    fn extract_value(value: ValueRef<'_>, decl_type: Option<&str>) -> Option<String> {
        match value {
            ValueRef::Null => None,
            ValueRef::Boolean(v) => Some(v.to_string()),
            ValueRef::TinyInt(i) => Some(i.to_string()),
            ValueRef::SmallInt(i) => Some(i.to_string()),
            ValueRef::Int(i) => {
                let type_upper = decl_type.map(|t| t.to_uppercase());
                let is_datetime = type_upper.as_ref().is_some_and(|t| {
                    t.contains("DATE") || t.contains("TIME") || t.contains("TIMESTAMP")
                });

                if is_datetime {
                    if let Some(dt) = chrono::DateTime::from_timestamp(i as i64, 0) {
                        return Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                    }
                }

                Some(i.to_string())
            }
            ValueRef::BigInt(i) => Some(i.to_string()),
            ValueRef::HugeInt(i) => Some(i.to_string()),
            ValueRef::UTinyInt(i) => Some(i.to_string()),
            ValueRef::USmallInt(i) => Some(i.to_string()),
            ValueRef::UInt(i) => Some(i.to_string()),
            ValueRef::UBigInt(i) => Some(i.to_string()),
            ValueRef::Float(f) => Some(f.to_string()),
            ValueRef::Double(f) => Some(f.to_string()),
            ValueRef::Decimal(d) => Some(d.to_string()),
            ValueRef::Text(t) => String::from_utf8(t.to_vec()).ok(),
            ValueRef::Blob(b) => {
                if let Ok(s) = String::from_utf8(b.to_vec()) {
                    Some(s)
                } else {
                    Some(format!("0x{}", hex::encode(b)))
                }
            }
            other => Some(format!("{other:?}")),
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

    fn is_query_sql(sql: &str) -> bool {
        let normalized = sql.trim_start().to_ascii_uppercase();
        normalized.starts_with("SELECT")
            || normalized.starts_with("WITH")
            || normalized.starts_with("PRAGMA")
            || normalized.starts_with("SHOW")
            || normalized.starts_with("DESCRIBE")
            || normalized.starts_with("EXPLAIN")
    }

    fn execute_statement(conn: &Connection, sql: &str, start: Instant) -> SqlResult {
        let sql_preview = if sql.len() > 200 {
            format!("{}...", truncate_str(sql, 200))
        } else {
            sql.to_string()
        };

        match conn.prepare(sql) {
            Ok(mut stmt) => {
                if !Self::is_query_sql(sql) {
                    match conn.execute(sql, []) {
                        Ok(rows_affected) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!(
                                "[DuckDB] Execute completed: {} rows affected, {}ms",
                                rows_affected, elapsed_ms
                            );
                            Self::build_exec_result(
                                sql.to_string(),
                                rows_affected as u64,
                                elapsed_ms,
                            )
                        }
                        Err(e) => {
                            error!("[DuckDB] Execute failed: {}, SQL: {}", e, sql_preview);
                            SqlResult::Error(SqlErrorInfo {
                                sql: sql.to_string(),
                                message: e.to_string(),
                            })
                        }
                    }
                } else {
                    let rows_result: Result<
                        (Vec<String>, Vec<Option<String>>, Vec<Vec<Option<String>>>),
                        duckdb::Error,
                    > = stmt.query([]).and_then(|mut rows| {
                        let stmt_ref = rows
                            .as_ref()
                            .expect("DuckDB rows should retain statement metadata");
                        let column_count = stmt_ref.column_count();
                        let columns = stmt_ref.column_names();
                        let column_types: Vec<Option<String>> = (0..column_count)
                            .map(|idx| Some(format!("{:?}", stmt_ref.column_type(idx))))
                            .collect();
                        let mut data_rows = Vec::new();
                        while let Some(row) = rows.next()? {
                            let row_data: Vec<Option<String>> = (0..column_count)
                                .map(|i| {
                                    let decl_type = column_types.get(i).and_then(|t| t.as_deref());
                                    row.get_ref(i)
                                        .ok()
                                        .and_then(|v| Self::extract_value(v, decl_type))
                                })
                                .collect();
                            data_rows.push(row_data);
                        }
                        Ok((columns, column_types, data_rows))
                    });

                    match rows_result {
                        Ok((columns, column_types, data_rows)) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!(
                                "[DuckDB] Query completed: {} rows, {} columns, {}ms",
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
                            error!("[DuckDB] Query failed: {}, SQL: {}", e, sql_preview);
                            SqlResult::Error(SqlErrorInfo {
                                sql: sql.to_string(),
                                message: e.to_string(),
                            })
                        }
                    }
                }
            }
            Err(e) => {
                error!("[DuckDB] Prepare failed: {}, SQL: {}", e, sql_preview);
                SqlResult::Error(SqlErrorInfo {
                    sql: sql.to_string(),
                    message: e.to_string(),
                })
            }
        }
    }
}

#[async_trait]
impl DbConnection for DuckDbConnection {
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
                .ok_or_else(|| DbError::connection("database path is required for DuckDB"))?
        };

        info!("[DuckDB] Connecting to {}", database_path);

        let conn = spawn_blocking(move || Connection::open(database_path))
            .await
            .map_err(|e| {
                error!("[DuckDB] Task join error: {}", e);
                DbError::Internal(format!("task join error: {}", e))
            })?
            .map_err(|e| {
                error!("[DuckDB] Connection failed: {}", e);
                DbError::connection_with_source("failed to connect", e)
            })?;

        {
            let mut guard = self
                .connection
                .lock()
                .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
            *guard = Some(conn);
        }

        info!("[DuckDB] Connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DbError> {
        let conn_opt = {
            let mut guard = self
                .connection
                .lock()
                .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
            guard.take()
        };

        if let Some(conn) = conn_opt {
            spawn_blocking(move || drop(conn)).await.map_err(|e| {
                error!("[DuckDB] Disconnect failed: {}", e);
                DbError::Internal(format!("task join error: {}", e))
            })?;
        }

        Ok(())
    }

    async fn execute(
        &self,
        plugin: &dyn DatabasePlugin,
        script: &str,
        options: ExecOptions,
    ) -> Result<Vec<SqlResult>, DbError> {
        let parser = plugin
            .create_parser(SqlSource::Script(script.to_string()))
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;
        let statements: Vec<String> = parser
            .filter_map(|r| r.ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut results = Vec::new();

        for sql in statements {
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
            results.push(result);
            if is_error && options.stop_on_error {
                break;
            }
        }

        Ok(results)
    }

    async fn query(&self, query: &str) -> Result<SqlResult, DbError> {
        let start = Instant::now();
        let query_owned = query.to_string();
        let connection = Arc::clone(&self.connection);

        spawn_blocking(move || {
            let guard = connection
                .lock()
                .map_err(|e| DbError::Internal(format!("lock poisoned: {}", e)))?;
            let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
            Ok(Self::execute_statement(conn, &query_owned, start))
        })
        .await
        .map_err(|e| DbError::Internal(format!("task join error: {}", e)))?
    }

    async fn current_database(&self) -> Result<Option<String>, DbError> {
        Ok(Some("main".to_string()))
    }

    async fn switch_database(&self, _database: &str) -> Result<(), DbError> {
        Err(DbError::NotSupported(
            "DuckDB does not support switching databases within one file connection".to_string(),
        ))
    }

    async fn execute_streaming(
        &self,
        plugin: &dyn DatabasePlugin,
        source: SqlSource,
        options: ExecOptions,
        sender: mpsc::Sender<StreamingProgress>,
    ) -> Result<(), DbError> {
        let total_size = source.file_size().unwrap_or(0);
        let is_file_source = source.is_file();
        let mut parser = plugin
            .create_parser(source)
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;

        if options.streaming || is_file_source {
            let mut current = 0usize;

            if options.transactional {
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

                let command = if has_error { "ROLLBACK" } else { "COMMIT" };
                let connection = Arc::clone(&self.connection);
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
            for (idx, sql) in statements.into_iter().enumerate() {
                let result = self.execute(plugin, &sql, options.clone()).await?;
                for item in result {
                    let progress = StreamingProgress::new(idx + 1, total, item);
                    if sender.send(progress).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DuckDbConnection;
    use crate::connection::DbConnection;
    use crate::executor::SqlResult;
    use one_core::storage::{DatabaseType, DbConnectionConfig};

    #[tokio::test]
    async fn test_duckdb_connection_can_query_temp_database() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let db_path = temp_dir.path().join("duckdb-basic-test.duckdb");

        let mut connection = DuckDbConnection::new(DbConnectionConfig {
            id: "duckdb-test".to_string(),
            name: "duckdb-test".to_string(),
            database_type: DatabaseType::DuckDB,
            host: db_path.to_string_lossy().to_string(),
            port: 0,
            workspace_id: None,
            username: String::new(),
            password: String::new(),
            database: None,
            service_name: None,
            sid: None,
            extra_params: Default::default(),
        });

        connection.connect().await.expect("duckdb should connect");

        let result = connection
            .query("select 42 as answer")
            .await
            .expect("query should succeed");

        match result {
            SqlResult::Query(query) => {
                assert_eq!(query.columns, vec!["answer".to_string()]);
                assert_eq!(query.rows, vec![vec![Some("42".to_string())]]);
            }
            other => panic!("expected query result, got {other:?}"),
        }

        connection
            .disconnect()
            .await
            .expect("duckdb should disconnect");
    }
}
