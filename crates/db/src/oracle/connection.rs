use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, Utc};
use one_core::storage::DbConnectionConfig;
use oracle::sql_type::OracleType;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, error, info};

use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::ssh_tunnel::resolve_connection_target;
use crate::{format_message, truncate_str, DatabasePlugin};
use ssh::LocalPortForwardTunnel;

pub struct OracleDbConnection {
    config: DbConnectionConfig,
    conn: Arc<Mutex<Option<oracle::Connection>>>,
    tunnel: Option<LocalPortForwardTunnel>,
}

impl OracleDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            conn: Arc::new(Mutex::new(None)),
            tunnel: None,
        }
    }

    fn build_connect_string(config: &DbConnectionConfig, host: &str, port: u16) -> String {
        if let Some(ref service) = config.service_name {
            format!("//{}:{}/{}", host, port, service)
        } else {
            format!(
                "//{}:{}/{}",
                host,
                port,
                config.sid.clone().unwrap_or_default()
            )
        }
    }

    fn format_binary(bytes: &[u8]) -> String {
        if bytes.is_empty() {
            return String::new();
        }

        let hex: String = bytes.iter().map(|byte| format!("{:02X}", byte)).collect();
        format!("0x{}", hex)
    }

    fn format_naive_date(value: NaiveDate) -> String {
        value.format("%Y-%m-%d").to_string()
    }

    fn format_naive_date_time(value: NaiveDateTime) -> String {
        let micros = value.and_utc().timestamp_subsec_micros();
        if micros == 0 {
            value.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            format!("{}.{:06}", value.format("%Y-%m-%d %H:%M:%S"), micros)
        }
    }

    fn format_offset_date_time(value: DateTime<FixedOffset>) -> String {
        let micros = value.timestamp_subsec_micros();
        if micros == 0 {
            value.format("%Y-%m-%d %H:%M:%S %:z").to_string()
        } else {
            format!(
                "{}.{:06} {}",
                value.format("%Y-%m-%d %H:%M:%S"),
                micros,
                value.format("%:z")
            )
        }
    }

    fn extract_scalar_value(row: &oracle::Row, index: usize) -> Option<String> {
        row.get::<usize, Option<String>>(index)
            .ok()
            .flatten()
            .or_else(|| {
                row.get::<usize, Option<i64>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
            .or_else(|| {
                row.get::<usize, Option<f64>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
            .or_else(|| {
                row.get::<usize, Option<bool>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
            .or_else(|| {
                row.get::<usize, Option<u64>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
    }

    fn extract_value(row: &oracle::Row, index: usize, oracle_type: &OracleType) -> Option<String> {
        match oracle_type {
            OracleType::Date | OracleType::Timestamp(_) => row
                .get::<usize, Option<NaiveDateTime>>(index)
                .ok()
                .flatten()
                .map(Self::format_naive_date_time)
                .or_else(|| {
                    row.get::<usize, Option<NaiveDate>>(index)
                        .ok()
                        .flatten()
                        .map(Self::format_naive_date)
                })
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::TimestampTZ(_) | OracleType::TimestampLTZ(_) => row
                .get::<usize, Option<DateTime<FixedOffset>>>(index)
                .ok()
                .flatten()
                .map(Self::format_offset_date_time)
                .or_else(|| {
                    row.get::<usize, Option<DateTime<Utc>>>(index)
                        .ok()
                        .flatten()
                        .map(|value| Self::format_offset_date_time(value.fixed_offset()))
                })
                .or_else(|| {
                    row.get::<usize, Option<DateTime<Local>>>(index)
                        .ok()
                        .flatten()
                        .map(|value| Self::format_offset_date_time(value.fixed_offset()))
                })
                .or_else(|| {
                    row.get::<usize, Option<NaiveDateTime>>(index)
                        .ok()
                        .flatten()
                        .map(Self::format_naive_date_time)
                })
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::Raw(_) | OracleType::LongRaw | OracleType::BLOB | OracleType::BFILE => row
                .get::<usize, Option<Vec<u8>>>(index)
                .ok()
                .flatten()
                .map(|bytes| Self::format_binary(&bytes))
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::BinaryFloat => row
                .get::<usize, Option<f32>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::BinaryDouble => row
                .get::<usize, Option<f64>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::Boolean => row
                .get::<usize, Option<bool>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::Int64 => row
                .get::<usize, Option<i64>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::UInt64 => row
                .get::<usize, Option<u64>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
                .or_else(|| Self::extract_scalar_value(row, index)),
            OracleType::Number(_, _)
            | OracleType::Float(_)
            | OracleType::Varchar2(_)
            | OracleType::NVarchar2(_)
            | OracleType::Char(_)
            | OracleType::NChar(_)
            | OracleType::Rowid
            | OracleType::CLOB
            | OracleType::NCLOB
            | OracleType::IntervalDS(_, _)
            | OracleType::IntervalYM(_)
            | OracleType::Object(_)
            | OracleType::Long
            | OracleType::Json
            | OracleType::Xml
            | OracleType::RefCursor => Self::extract_scalar_value(row, index),
        }
    }

    fn build_query_result(
        columns: Vec<String>,
        column_meta: Vec<QueryColumnMeta>,
        rows: Vec<Vec<Option<String>>>,
        sql: String,
        elapsed_ms: u128,
    ) -> SqlResult {
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

    fn execute_statement_sync(conn: &oracle::Connection, sql: &str) -> Result<SqlResult, DbError> {
        let start = Instant::now();
        let sql_string = sql.to_string();
        let sql_preview = if sql.len() > 200 {
            format!("{}...", truncate_str(&sql, 200))
        } else {
            sql.to_string()
        };

        match conn.statement(sql).build() {
            Ok(mut stmt) => {
                if stmt.is_query() {
                    match stmt.query(&[]) {
                        Ok(rows) => {
                            let elapsed_ms = start.elapsed().as_millis();

                            let column_info = rows.column_info();
                            let columns: Vec<String> = column_info
                                .iter()
                                .map(|col| col.name().to_string())
                                .collect();

                            let column_meta: Vec<QueryColumnMeta> = column_info
                                .iter()
                                .map(|col| {
                                    QueryColumnMeta::new(
                                        col.name().to_string(),
                                        col.oracle_type().to_string(),
                                    )
                                })
                                .collect();
                            let column_types: Vec<OracleType> = column_info
                                .iter()
                                .map(|col| col.oracle_type().clone())
                                .collect();

                            let mut data_rows = Vec::new();
                            for row_result in rows {
                                match row_result {
                                    Ok(row) => {
                                        let row_data: Vec<Option<String>> = (0..columns.len())
                                            .map(|i| {
                                                column_types
                                                    .get(i)
                                                    .and_then(|oracle_type| {
                                                        Self::extract_value(&row, i, oracle_type)
                                                    })
                                                    .or_else(|| Self::extract_scalar_value(&row, i))
                                            })
                                            .collect();
                                        data_rows.push(row_data);
                                    }
                                    Err(_) => continue,
                                }
                            }

                            debug!(
                                "[Oracle] Query completed: {} rows, {} columns, {}ms",
                                data_rows.len(),
                                columns.len(),
                                elapsed_ms
                            );
                            Ok(Self::build_query_result(
                                columns,
                                column_meta,
                                data_rows,
                                sql_string,
                                elapsed_ms,
                            ))
                        }
                        Err(e) => {
                            error!("[Oracle] Query failed: {}, SQL: {}", e, sql_preview);
                            Ok(SqlResult::Error(SqlErrorInfo {
                                sql: sql_string,
                                message: e.to_string(),
                            }))
                        }
                    }
                } else {
                    match stmt.execute(&[]) {
                        Ok(()) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            let rows_affected = stmt.row_count().unwrap_or(0);
                            debug!(
                                "[Oracle] Execute completed: {} rows affected, {}ms",
                                rows_affected, elapsed_ms
                            );
                            Ok(Self::build_exec_result(
                                sql_string,
                                rows_affected,
                                elapsed_ms,
                            ))
                        }
                        Err(e) => {
                            error!("[Oracle] Execute failed: {}, SQL: {}", e, sql_preview);
                            Ok(SqlResult::Error(SqlErrorInfo {
                                sql: sql_string,
                                message: e.to_string(),
                            }))
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "[Oracle] Statement build failed: {}, SQL: {}",
                    e, sql_preview
                );
                Ok(SqlResult::Error(SqlErrorInfo {
                    sql: sql_string,
                    message: e.to_string(),
                }))
            }
        }
    }
}

#[async_trait]
impl DbConnection for OracleDbConnection {
    fn config(&self) -> &DbConnectionConfig {
        &self.config
    }

    fn set_config_database(&mut self, database: Option<String>) {
        self.config.database = database;
    }

    async fn connect(&mut self) -> Result<(), DbError> {
        let config = self.config.clone();
        info!("[Oracle] Connecting to {}:{}", config.host, config.port);
        let target = resolve_connection_target(&config).await?;
        self.tunnel = target.tunnel;

        let connect_string = Self::build_connect_string(&config, &target.host, target.port);
        debug!("[Oracle] Connect string: {}", connect_string);
        let username = config.username.clone();
        let password = config.password.clone();

        // 获取连接超时，默认 30 秒
        let connect_timeout_secs = config.get_param_as::<u64>("connect_timeout").unwrap_or(30);
        debug!(
            "[Oracle] Connecting with timeout {}s...",
            connect_timeout_secs
        );

        // 使用 tokio::timeout 包装 spawn_blocking
        let conn_result = timeout(
            Duration::from_secs(connect_timeout_secs),
            tokio::task::spawn_blocking(move || {
                oracle::Connection::connect(&username, &password, &connect_string).map_err(|e| {
                    error!("[Oracle] Connection failed: {}", e);
                    DbError::connection_with_source("failed to connect", e)
                })
            }),
        )
        .await;

        let conn = match conn_result {
            Ok(Ok(Ok(conn))) => conn,
            Ok(Ok(Err(e))) => return Err(e),
            Ok(Err(e)) => {
                error!("[Oracle] Task error: {}", e);
                return Err(DbError::Internal(format!("task error: {}", e)));
            }
            Err(_) => {
                error!(
                    "[Oracle] Connection timed out after {}s",
                    connect_timeout_secs
                );
                return Err(DbError::connection(format!(
                    "connection timed out after {}s",
                    connect_timeout_secs
                )));
            }
        };

        {
            let mut guard = self.conn.lock().await;
            *guard = Some(conn);
        }

        info!("[Oracle] Connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DbError> {
        debug!("[Oracle] Disconnecting...");
        let conn_opt = {
            let mut guard = self.conn.lock().await;
            guard.take()
        };

        if let Some(conn) = conn_opt {
            tokio::task::spawn_blocking(move || {
                let _ = conn.close();
            })
            .await
            .map_err(|e| {
                error!("[Oracle] Disconnect task error: {}", e);
                DbError::Internal(format!("task error: {}", e))
            })?;
        }

        info!("[Oracle] Disconnected");
        self.tunnel = None;
        Ok(())
    }

    async fn execute(
        &self,
        plugin: &dyn DatabasePlugin,
        script: &str,
        options: ExecOptions,
    ) -> Result<Vec<SqlResult>, DbError> {
        debug!(
            "[Oracle] execute() called, stop_on_error={}",
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

        debug!("[Oracle] Split into {} statement(s)", statements.len());
        let mut results = Vec::new();
        let conn_arc = self.conn.clone();
        let stop_on_error = options.stop_on_error;

        for (idx, sql) in statements.iter().enumerate() {
            let conn_clone = conn_arc.clone();
            let sql_clone = sql.clone();
            let sql_preview = if sql.len() > 200 {
                format!("{}...", truncate_str(&sql, 200))
            } else {
                sql.clone()
            };
            debug!(
                "[Oracle] Executing statement {}/{}",
                idx + 1,
                statements.len()
            );

            let result = tokio::task::spawn_blocking(move || {
                let guard = conn_clone.blocking_lock();
                let conn = guard.as_ref().ok_or(DbError::NotConnected)?;

                Self::execute_statement_sync(conn, &sql_clone)
            })
            .await
            .map_err(|e| {
                error!("[Oracle] Task error: {}, SQL: {}", e, sql_preview);
                DbError::Internal(format!("task error: {}", e))
            })??;

            let is_error = result.is_error();
            if is_error {
                debug!(
                    "[Oracle] Statement {}/{} returned error",
                    idx + 1,
                    statements.len()
                );
            }
            results.push(result);

            if is_error && stop_on_error {
                debug!("[Oracle] Stopping execution due to error (stop_on_error=true)");
                break;
            }
        }

        debug!(
            "[Oracle] execute() completed with {} result(s)",
            results.len()
        );
        Ok(results)
    }

    async fn query(&self, query: &str) -> Result<SqlResult, DbError> {
        debug!("[Oracle] query() called");
        let conn_arc = self.conn.clone();
        let sql = query.to_string();
        let sql_preview = if query.len() > 200 {
            format!("{}...", truncate_str(query, 200))
        } else {
            query.to_string()
        };

        tokio::task::spawn_blocking(move || {
            let guard = conn_arc.blocking_lock();
            let conn = guard.as_ref().ok_or(DbError::NotConnected)?;

            Self::execute_statement_sync(conn, &sql)
        })
        .await
        .map_err(|e| {
            error!("[Oracle] Query task error: {}, SQL: {}", e, sql_preview);
            DbError::Internal(format!("task error: {}", e))
        })?
    }

    async fn current_database(&self) -> Result<Option<String>, DbError> {
        debug!("[Oracle] Querying current schema");
        let conn_arc = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let guard = conn_arc.blocking_lock();
            let conn = guard.as_ref().ok_or(DbError::NotConnected)?;

            match conn.query_row_as::<String>(
                "SELECT SYS_CONTEXT('USERENV', 'CURRENT_SCHEMA') FROM DUAL",
                &[],
            ) {
                Ok(schema) => {
                    debug!("[Oracle] Current schema: {}", schema);
                    Ok(Some(schema))
                }
                Err(e) => {
                    error!("[Oracle] Failed to get current schema: {}", e);
                    Ok(None)
                }
            }
        })
        .await
        .map_err(|e| {
            error!("[Oracle] Task error: {}", e);
            DbError::Internal(format!("task error: {}", e))
        })?
    }

    async fn switch_database(&self, schema: &str) -> Result<(), DbError> {
        self.switch_schema(schema).await
    }

    async fn switch_schema(&self, schema: &str) -> Result<(), DbError> {
        debug!("[Oracle] Switching to schema: {}", schema);
        let sql = format!(
            "ALTER SESSION SET CURRENT_SCHEMA = \"{}\"",
            schema.replace("\"", "\"\"")
        );
        let conn_arc = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let guard = conn_arc.blocking_lock();
            let conn = guard.as_ref().ok_or(DbError::NotConnected)?;

            conn.execute(&sql, &[]).map_err(|e| {
                error!("[Oracle] Failed to switch schema: {}, SQL: {}", e, sql);
                DbError::query_with_source("failed to switch schema", e)
            })?;

            info!("[Oracle] Switched to schema");
            Ok(())
        })
        .await
        .map_err(|e| {
            error!("[Oracle] Task error: {}", e);
            DbError::Internal(format!("task error: {}", e))
        })?
    }

    async fn execute_streaming(
        &self,
        plugin: &dyn DatabasePlugin,
        source: SqlSource,
        options: ExecOptions,
        sender: mpsc::Sender<StreamingProgress>,
    ) -> Result<(), DbError> {
        debug!(
            "[Oracle] execute_streaming() called, streaming={}",
            options.streaming
        );

        let total_size = source.file_size().unwrap_or(0);
        let is_file_source = source.is_file();
        let stop_on_error = options.stop_on_error;

        let mut parser = plugin
            .create_parser(source)
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;

        if options.streaming || is_file_source {
            let mut current = 0usize;

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
                        if stop_on_error {
                            break;
                        }
                        continue;
                    }
                };

                current += 1;
                let sql_preview = if sql.len() > 200 {
                    format!("{}...", truncate_str(&sql, 200))
                } else {
                    sql.clone()
                };
                debug!("[Oracle] Streaming statement {}", current);

                let conn_arc = self.conn.clone();
                let sql_clone = sql.clone();

                let result = match tokio::task::spawn_blocking(move || {
                    let guard = conn_arc.blocking_lock();
                    let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                    Self::execute_statement_sync(conn, &sql_clone)
                })
                .await
                {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => {
                        error!(
                            "[Oracle] Streaming statement {} failed: {}, SQL: {}",
                            current, e, sql_preview
                        );
                        SqlResult::Error(SqlErrorInfo {
                            sql: sql.clone(),
                            message: e.to_string(),
                        })
                    }
                    Err(e) => {
                        error!("[Oracle] Streaming task error: {}, SQL: {}", e, sql_preview);
                        SqlResult::Error(SqlErrorInfo {
                            sql: sql.clone(),
                            message: e.to_string(),
                        })
                    }
                };

                let is_error = result.is_error();
                let progress =
                    StreamingProgress::with_file_progress(current, result, bytes_read, total_size);
                if sender.send(progress).await.is_err() {
                    break;
                }

                if is_error && stop_on_error {
                    break;
                }
            }
        } else {
            let statements: Vec<String> = parser
                .filter_map(|r| r.ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let total = statements.len();
            debug!("[Oracle] Streaming {} statement(s)", total);

            for (index, sql) in statements.into_iter().enumerate() {
                let current = index + 1;
                let sql_preview = if sql.len() > 200 {
                    format!("{}...", truncate_str(&sql, 200))
                } else {
                    sql.clone()
                };
                debug!("[Oracle] Streaming statement {}/{}", current, total);

                let conn_arc = self.conn.clone();
                let sql_clone = sql.clone();

                let result = match tokio::task::spawn_blocking(move || {
                    let guard = conn_arc.blocking_lock();
                    let conn = guard.as_ref().ok_or(DbError::NotConnected)?;
                    Self::execute_statement_sync(conn, &sql_clone)
                })
                .await
                {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => {
                        error!(
                            "[Oracle] Streaming statement {}/{} failed: {}, SQL: {}",
                            current, total, e, sql_preview
                        );
                        SqlResult::Error(SqlErrorInfo {
                            sql: sql.clone(),
                            message: e.to_string(),
                        })
                    }
                    Err(e) => {
                        error!("[Oracle] Streaming task error: {}, SQL: {}", e, sql_preview);
                        SqlResult::Error(SqlErrorInfo {
                            sql: sql.clone(),
                            message: e.to_string(),
                        })
                    }
                };

                let is_error = result.is_error();
                let progress = StreamingProgress::new(current, total, result);
                if sender.send(progress).await.is_err() {
                    break;
                }

                if is_error && stop_on_error {
                    break;
                }
            }
        }

        debug!("[Oracle] execute_streaming() completed");
        Ok(())
    }
}
