use std::time::{Duration, Instant};

use async_trait::async_trait;
use mysql_async::{prelude::*, Conn, Opts, OptsBuilder, SslOpts, Value};
use one_core::storage::DbConnectionConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, error, info};

use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::rustls_provider::ensure_rustls_crypto_provider;
use crate::ssh_tunnel::resolve_connection_target;
use crate::{format_message, truncate_str, DatabasePlugin};
use ssh::LocalPortForwardTunnel;

pub struct MysqlDbConnection {
    config: DbConnectionConfig,
    conn: Arc<Mutex<Option<Conn>>>,
    tunnel: Option<LocalPortForwardTunnel>,
}

impl MysqlDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            conn: Arc::new(Mutex::new(None)),
            tunnel: None,
        }
    }

    fn build_ssl_opts(config: &DbConnectionConfig) -> Option<SslOpts> {
        ensure_rustls_crypto_provider();

        let require_ssl = config.get_param_bool("require_ssl");
        let verify_ca = config
            .get_param("verify_ca")
            .map(|value| value != "false")
            .unwrap_or(true);
        let verify_identity = config
            .get_param("verify_identity")
            .map(|value| value != "false")
            .unwrap_or(true);
        let root_cert_path = config
            .get_param("ssl_root_cert_path")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        let hostname_override = config
            .get_param("tls_hostname_override")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());

        if !require_ssl && verify_ca && verify_identity && root_cert_path.is_none() {
            return None;
        }

        let mut ssl_opts = SslOpts::default();
        if let Some(root_cert_path) = root_cert_path {
            ssl_opts = ssl_opts.with_root_certs(vec![PathBuf::from(root_cert_path).into()]);
        }
        if !verify_ca {
            ssl_opts = ssl_opts.with_danger_accept_invalid_certs(true);
        }
        if !verify_identity {
            ssl_opts = ssl_opts.with_danger_skip_domain_validation(true);
        }
        if let Some(hostname_override) = hostname_override {
            ssl_opts =
                ssl_opts.with_danger_tls_hostname_override(Some(hostname_override.to_string()));
        }

        Some(ssl_opts)
    }

    /// Extract value from mysql_async::Value
    /// For SQL client display, similar to Navicat behavior:
    /// - NULL -> None (displayed as NULL in UI)
    /// - Binary data -> hex string with prefix (0x...)
    /// - Text -> UTF-8 string
    /// - DateTime -> ISO format with microseconds when present
    fn extract_value(value: &Value) -> Option<String> {
        match value {
            Value::NULL => None,
            Value::Bytes(b) => Some(Self::format_bytes(b)),
            Value::Int(i) => Some(i.to_string()),
            Value::UInt(u) => Some(u.to_string()),
            Value::Float(f) => Some(f.to_string()),
            Value::Double(d) => Some(d.to_string()),
            Value::Date(year, month, day, hour, min, sec, micro) => Some(Self::format_datetime(
                *year, *month, *day, *hour, *min, *sec, *micro,
            )),
            Value::Time(is_neg, days, hours, minutes, seconds, micros) => Some(Self::format_time(
                *is_neg, *days, *hours, *minutes, *seconds, *micros,
            )),
        }
    }

    fn format_bytes(bytes: &[u8]) -> String {
        if bytes.is_empty() {
            return String::new();
        }
        if Self::is_valid_utf8_text(bytes) {
            String::from_utf8_lossy(bytes).into_owned()
        } else {
            Self::format_as_hex(bytes)
        }
    }

    fn is_valid_utf8_text(bytes: &[u8]) -> bool {
        match std::str::from_utf8(bytes) {
            Ok(s) => s
                .chars()
                .all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t'),
            Err(_) => false,
        }
    }

    fn format_as_hex(bytes: &[u8]) -> String {
        const MAX_HEX_DISPLAY: usize = 1024;
        let display_bytes = if bytes.len() > MAX_HEX_DISPLAY {
            &bytes[..MAX_HEX_DISPLAY]
        } else {
            bytes
        };
        let hex: String = display_bytes.iter().map(|b| format!("{:02X}", b)).collect();
        if bytes.len() > MAX_HEX_DISPLAY {
            format!("0x{}... ({} bytes)", hex, bytes.len())
        } else {
            format!("0x{}", hex)
        }
    }

    fn format_datetime(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        min: u8,
        sec: u8,
        micro: u32,
    ) -> String {
        if year == 0 && month == 0 && day == 0 {
            return String::from("0000-00-00");
        }
        if hour == 0 && min == 0 && sec == 0 && micro == 0 {
            format!("{:04}-{:02}-{:02}", year, month, day)
        } else if micro == 0 {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                year, month, day, hour, min, sec
            )
        } else {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                year, month, day, hour, min, sec, micro
            )
        }
    }

    fn format_time(
        is_neg: bool,
        days: u32,
        hours: u8,
        minutes: u8,
        seconds: u8,
        micros: u32,
    ) -> String {
        let sign = if is_neg { "-" } else { "" };
        let total_hours = (days * 24) + hours as u32;
        if micros == 0 {
            format!("{}{}:{:02}:{:02}", sign, total_hours, minutes, seconds)
        } else {
            format!(
                "{}{}:{:02}:{:02}.{:06}",
                sign, total_hours, minutes, seconds, micros
            )
        }
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

    async fn process_query_result<'a, 't, P>(
        mut query_result: mysql_async::QueryResult<'a, 't, P>,
        sql: String,
        elapsed_ms: u128,
    ) -> Result<SqlResult, DbError>
    where
        P: Protocol,
    {
        let columns_arc = query_result.columns();
        let affected_rows = query_result.affected_rows();

        if columns_arc.is_none()
            || columns_arc
                .as_ref()
                .map_or(true, |cols: &Arc<[mysql_async::Column]>| cols.is_empty())
        {
            return Ok(Self::build_exec_result(sql, affected_rows, elapsed_ms));
        }

        let columns_arc = columns_arc.unwrap();
        let columns: Vec<String> = columns_arc
            .iter()
            .map(|col: &mysql_async::Column| col.name_str().to_string())
            .collect();

        let column_meta: Vec<QueryColumnMeta> = columns_arc
            .iter()
            .map(|col: &mysql_async::Column| {
                let name = col.name_str().to_string();
                let db_type = format!("{:?}", col.column_type());
                QueryColumnMeta::new(name, db_type)
            })
            .collect();

        let mut all_rows = Vec::new();
        while let Some(row) = query_result
            .next()
            .await
            .map_err(|e| DbError::query_with_source("failed to fetch row", e))?
        {
            let row_data: Vec<Option<String>> = (0..row.len())
                .map(|i| Self::extract_value(&row[i]))
                .collect();
            all_rows.push(row_data);
        }

        Ok(SqlResult::Query(QueryResult {
            sql,
            columns,
            column_meta,
            rows: all_rows,
            elapsed_ms,
        }))
    }

    async fn execute_single(conn: &mut Conn, sql: &str) -> Result<SqlResult, DbError> {
        let start = Instant::now();
        let sql_string = sql.to_string();
        let sql_preview = if sql.len() > 1000 {
            format!("{}...", truncate_str(sql, 1000))
        } else {
            sql.to_string()
        };
        debug!("[MySQL] Executing SQL: {}", sql_preview);

        match conn.query_iter(sql).await {
            Ok(query_result) => {
                let elapsed_ms = start.elapsed().as_millis();
                debug!("[MySQL] Query submitted successfully, processing result...");
                Self::process_query_result(query_result, sql_string, elapsed_ms).await
            }
            Err(e) => {
                error!(
                    "[MySQL] Query execution failed: {}, SQL: {}",
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
impl DbConnection for MysqlDbConnection {
    fn config(&self) -> &DbConnectionConfig {
        &self.config
    }

    fn set_config_database(&mut self, database: Option<String>) {
        self.config.database = database;
    }

    async fn connect(&mut self) -> Result<(), DbError> {
        let config = &self.config;
        info!("[MySQL] Connecting to {}:{}", config.host, config.port);
        let target = resolve_connection_target(config).await?;
        self.tunnel = target.tunnel;

        let mut opts_builder = OptsBuilder::default()
            .ip_or_hostname(&target.host)
            .tcp_port(target.port)
            .user(Some(&config.username))
            .pass(Some(&config.password));

        if let Some(ref db) = config.database {
            opts_builder = opts_builder.db_name(Some(db));
            debug!("[MySQL] Using database: {}", db);
        }

        if let Some(ssl_opts) = Self::build_ssl_opts(config) {
            opts_builder = opts_builder.ssl_opts(ssl_opts);
            debug!("[MySQL] SSL/TLS enabled");
        }

        // 获取连接超时，默认 30 秒
        let connect_timeout_secs = config.get_param_as::<u64>("connect_timeout").unwrap_or(30);

        debug!(
            "[MySQL] Establishing connection with timeout {}s...",
            connect_timeout_secs
        );
        let opts = Opts::from(opts_builder);

        // 使用 tokio::timeout 包装连接操作
        let conn_result = timeout(Duration::from_secs(connect_timeout_secs), Conn::new(opts)).await;

        let conn = match conn_result {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => {
                error!("[MySQL] Connection failed: {}", e);
                return Err(DbError::connection_with_source("failed to connect", e));
            }
            Err(_) => {
                error!(
                    "[MySQL] Connection timed out after {}s",
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

        info!("[MySQL] Connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DbError> {
        debug!("[MySQL] Disconnecting...");
        let conn_opt = {
            let mut guard = self.conn.lock().await;
            guard.take()
        };

        if let Some(conn) = conn_opt {
            conn.disconnect().await.map_err(|e| {
                error!("[MySQL] Disconnect failed: {}", e);
                DbError::connection_with_source("failed to disconnect", e)
            })?;
        }

        info!("[MySQL] Disconnected");
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
            "[MySQL] execute() called, transactional={}, stop_on_error={}",
            options.transactional, options.stop_on_error
        );
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or(DbError::NotConnected)?;

        let parser = plugin
            .create_parser(SqlSource::Script(script.to_string()))
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;
        let statements: Vec<String> = parser
            .filter_map(|r| r.ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        debug!("[MySQL] Split into {} statement(s)", statements.len());
        let mut results = Vec::new();

        if options.transactional {
            debug!("[MySQL] Starting transaction...");
            let mut tx = conn
                .start_transaction(Default::default())
                .await
                .map_err(|e| {
                    error!("[MySQL] Failed to begin transaction: {}", e);
                    DbError::transaction_with_source("failed to begin transaction", e)
                })?;

            for (idx, sql) in statements.iter().enumerate() {
                let sql = sql.trim();
                if sql.is_empty() {
                    continue;
                }

                let sql_preview = if sql.len() > 200 {
                    format!("{}...", truncate_str(&sql, 200))
                } else {
                    sql.to_string()
                };
                debug!(
                    "[MySQL] TX executing statement {}/{}: {}",
                    idx + 1,
                    statements.len(),
                    sql_preview
                );
                let start = Instant::now();

                let result = match tx.query_iter(sql).await {
                    Ok(query_result) => {
                        let elapsed_ms = start.elapsed().as_millis();
                        match Self::process_query_result(query_result, sql.to_string(), elapsed_ms)
                            .await
                        {
                            Ok(result) => result,
                            Err(e) => {
                                error!(
                                    "[MySQL] TX failed to process result: {}, SQL: {}",
                                    e, sql_preview
                                );
                                SqlResult::Error(SqlErrorInfo {
                                    sql: sql.to_string(),
                                    message: e.to_string(),
                                })
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "[MySQL] TX query execution failed: {}, SQL: {}",
                            e, sql_preview
                        );
                        SqlResult::Error(SqlErrorInfo {
                            sql: sql.to_string(),
                            message: e.to_string(),
                        })
                    }
                };

                let is_error = result.is_error();
                results.push(result);

                if is_error {
                    debug!(
                        "[MySQL] TX statement {}/{} returned error, will rollback",
                        idx + 1,
                        statements.len()
                    );
                    break;
                }
            }

            let has_error = results.iter().any(|r| r.is_error());
            if has_error {
                debug!("[MySQL] Rolling back transaction...");
                tx.rollback().await.map_err(|e| {
                    error!("[MySQL] Failed to rollback: {}", e);
                    DbError::transaction_with_source("failed to rollback", e)
                })?;
                debug!("[MySQL] Transaction rolled back");
            } else {
                debug!("[MySQL] Committing transaction...");
                tx.commit().await.map_err(|e| {
                    error!("[MySQL] Failed to commit: {}", e);
                    DbError::transaction_with_source("failed to commit", e)
                })?;
                debug!("[MySQL] Transaction committed");
            }
        } else {
            for (idx, sql) in statements.iter().enumerate() {
                let sql = sql.trim();
                if sql.is_empty() {
                    continue;
                }

                debug!(
                    "[MySQL] Executing statement {}/{}",
                    idx + 1,
                    statements.len()
                );
                let result = Self::execute_single(conn, sql).await?;

                let is_error = result.is_error();
                if is_error {
                    debug!(
                        "[MySQL] Statement {}/{} returned error",
                        idx + 1,
                        statements.len()
                    );
                }
                results.push(result);

                if is_error && options.stop_on_error {
                    debug!("[MySQL] Stopping execution due to error (stop_on_error=true)");
                    break;
                }
            }
        }

        debug!(
            "[MySQL] execute() completed with {} result(s)",
            results.len()
        );
        Ok(results)
    }

    async fn query(&self, query: &str) -> Result<SqlResult, DbError> {
        debug!("[MySQL] query() called");
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or(DbError::NotConnected)?;
        Self::execute_single(conn, query).await
    }

    async fn current_database(&self) -> Result<Option<String>, DbError> {
        debug!("[MySQL] Querying current database");
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or(DbError::NotConnected)?;

        let result: Option<Option<String>> =
            conn.query_first("SELECT DATABASE()").await.map_err(|e| {
                error!("[MySQL] Failed to get current database: {}", e);
                DbError::query_with_source("failed to get current database", e)
            })?;

        let db = result.flatten();
        debug!("[MySQL] Current database: {:?}", db);
        Ok(db)
    }

    async fn switch_database(&self, database: &str) -> Result<(), DbError> {
        debug!("[MySQL] Switching to database: {}", database);
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or(DbError::NotConnected)?;

        let sql = format!("USE `{}`", database.replace("`", "``"));
        debug!("[MySQL] Executing: {}", sql);
        conn.query_drop(&sql).await.map_err(|e| {
            error!("[MySQL] Failed to switch database: {}, SQL: {}", e, sql);
            DbError::query_with_source("failed to switch database", e)
        })?;

        info!("[MySQL] Switched to database: {}", database);
        Ok(())
    }

    async fn execute_streaming(
        &self,
        plugin: &dyn DatabasePlugin,
        source: SqlSource,
        options: ExecOptions,
        sender: mpsc::Sender<StreamingProgress>,
    ) -> Result<(), DbError> {
        debug!(
            "[MySQL] execute_streaming() called, transactional={}, streaming={}",
            options.transactional, options.streaming
        );
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or(DbError::NotConnected)?;

        let total_size = source.file_size().unwrap_or(0);
        let is_file_source = source.is_file();

        let mut parser = plugin
            .create_parser(source)
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;

        if options.streaming || is_file_source {
            let mut current = 0usize;
            let mut has_error = false;

            if options.transactional {
                debug!("[MySQL] Starting transaction for streaming...");
                let mut tx = conn
                    .start_transaction(Default::default())
                    .await
                    .map_err(|e| {
                        error!("[MySQL] Failed to begin transaction: {}", e);
                        DbError::transaction_with_source("failed to begin transaction", e)
                    })?;

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
                    let sql_preview = if sql.len() > 200 {
                        format!("{}...", truncate_str(&sql, 200))
                    } else {
                        sql.clone()
                    };
                    debug!("[MySQL] Streaming TX statement {}", current);
                    let start = Instant::now();

                    let result = match tx.query_iter(&sql).await {
                        Ok(query_result) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            match Self::process_query_result(query_result, sql.clone(), elapsed_ms)
                                .await
                            {
                                Ok(result) => result,
                                Err(e) => {
                                    error!(
                                        "[MySQL] Streaming TX failed to process result: {}, SQL: {}",
                                        e, sql_preview
                                    );
                                    SqlResult::Error(SqlErrorInfo {
                                        sql: sql.clone(),
                                        message: e.to_string(),
                                    })
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "[MySQL] Streaming TX query failed: {}, SQL: {}",
                                e, sql_preview
                            );
                            SqlResult::Error(SqlErrorInfo {
                                sql: sql.clone(),
                                message: e.to_string(),
                            })
                        }
                    };

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

                if has_error {
                    let _ = tx.rollback().await;
                } else {
                    let _ = tx.commit().await;
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
                    debug!("[MySQL] Streaming statement {}", current);

                    let result = match Self::execute_single(conn, &sql).await {
                        Ok(r) => r,
                        Err(e) => {
                            let sql_preview = if sql.len() > 200 {
                                format!("{}...", truncate_str(&sql, 200))
                            } else {
                                sql.clone()
                            };
                            error!(
                                "[MySQL] Streaming statement {} failed: {}, SQL: {}",
                                current, e, sql_preview
                            );
                            SqlResult::Error(SqlErrorInfo {
                                sql: sql.clone(),
                                message: e.to_string(),
                            })
                        }
                    };

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
            debug!("[MySQL] Streaming {} statement(s)", total);

            if total == 0 {
                debug!("[MySQL] No statements to execute");
                return Ok(());
            }

            if options.transactional {
                debug!("[MySQL] Starting transaction for streaming...");
                let mut tx = conn
                    .start_transaction(Default::default())
                    .await
                    .map_err(|e| {
                        error!("[MySQL] Failed to begin transaction: {}", e);
                        DbError::transaction_with_source("failed to begin transaction", e)
                    })?;

                let mut has_error = false;

                for (index, sql) in statements.into_iter().enumerate() {
                    let current = index + 1;
                    let sql_preview = if sql.len() > 200 {
                        format!("{}...", truncate_str(&sql, 200))
                    } else {
                        sql.clone()
                    };
                    debug!("[MySQL] Streaming TX statement {}/{}", current, total);
                    let start = Instant::now();

                    let result = match tx.query_iter(&sql).await {
                        Ok(query_result) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            match Self::process_query_result(query_result, sql.clone(), elapsed_ms)
                                .await
                            {
                                Ok(result) => result,
                                Err(e) => {
                                    error!(
                                        "[MySQL] Streaming TX failed to process result: {}, SQL: {}",
                                        e, sql_preview
                                    );
                                    SqlResult::Error(SqlErrorInfo {
                                        sql: sql.clone(),
                                        message: e.to_string(),
                                    })
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "[MySQL] Streaming TX query failed: {}, SQL: {}",
                                e, sql_preview
                            );
                            SqlResult::Error(SqlErrorInfo {
                                sql: sql.clone(),
                                message: e.to_string(),
                            })
                        }
                    };

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

                if has_error {
                    let _ = tx.rollback().await;
                } else {
                    let _ = tx.commit().await;
                }
            } else {
                for (index, sql) in statements.into_iter().enumerate() {
                    let current = index + 1;
                    debug!("[MySQL] Streaming statement {}/{}", current, total);

                    let result = match Self::execute_single(conn, &sql).await {
                        Ok(r) => r,
                        Err(e) => {
                            let sql_preview = if sql.len() > 200 {
                                format!("{}...", truncate_str(&sql, 200))
                            } else {
                                sql.clone()
                            };
                            error!(
                                "[MySQL] Streaming statement {}/{} failed: {}, SQL: {}",
                                current, total, e, sql_preview
                            );
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

                    if is_error && options.stop_on_error {
                        break;
                    }
                }
            }
        }

        debug!("[MySQL] execute_streaming() completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use one_core::storage::DatabaseType;

    fn build_config(extra_params: &[(&str, &str)]) -> DbConnectionConfig {
        DbConnectionConfig {
            id: String::new(),
            database_type: DatabaseType::MySQL,
            name: "mysql".to_string(),
            host: "localhost".to_string(),
            port: 3306,
            username: "root".to_string(),
            password: String::new(),
            database: None,
            service_name: None,
            sid: None,
            workspace_id: None,
            extra_params: extra_params
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    #[test]
    fn build_ssl_opts_returns_none_when_ssl_not_requested() {
        let config = build_config(&[]);

        assert!(MysqlDbConnection::build_ssl_opts(&config).is_none());
    }

    #[test]
    fn build_ssl_opts_enables_tls_for_custom_settings() {
        let config = build_config(&[
            ("require_ssl", "true"),
            ("verify_ca", "false"),
            ("verify_identity", "false"),
            ("ssl_root_cert_path", "/tmp/ca.pem"),
            ("tls_hostname_override", "db.internal"),
        ]);

        let ssl_opts = MysqlDbConnection::build_ssl_opts(&config).expect("SSL 配置应被构造出来");

        assert!(ssl_opts.accept_invalid_certs());
        assert!(ssl_opts.skip_domain_validation());
        assert_eq!(ssl_opts.root_certs().len(), 1);
        assert_eq!(ssl_opts.tls_hostname_override(), Some("db.internal"));
    }
}
