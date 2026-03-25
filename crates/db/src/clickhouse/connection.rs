use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::ssh_tunnel::resolve_connection_target;
use crate::{format_message, truncate_str, DatabasePlugin};

use async_trait::async_trait;
use clickhouse::Client;
use one_core::storage::DbConnectionConfig;
use serde::Deserialize;
use ssh::LocalPortForwardTunnel;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info};

pub struct ClickHouseDbConnection {
    config: DbConnectionConfig,
    client: Option<Client>,
    tunnel: Option<LocalPortForwardTunnel>,
}

impl ClickHouseDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            client: None,
            tunnel: None,
        }
    }

    fn ensure_connected(&self) -> Result<&Client, DbError> {
        self.client.as_ref().ok_or(DbError::NotConnected)
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

    async fn execute_single(client: &Client, sql: &str) -> Result<SqlResult, DbError> {
        let start = Instant::now();
        let sql_string = sql.to_string();
        let sql_preview = if sql.len() > 200 {
            format!("{}...", truncate_str(&sql, 200))
        } else {
            sql.to_string()
        };
        debug!("[ClickHouse] Executing SQL: {}", sql_preview);

        match Self::fetch_json_compact(client, sql).await {
            Ok(result) => {
                let elapsed_ms = start.elapsed().as_millis();

                let columns: Vec<String> =
                    result.meta.iter().map(|meta| meta.name.clone()).collect();
                let column_meta: Vec<QueryColumnMeta> = result
                    .meta
                    .iter()
                    .map(|meta| QueryColumnMeta::new(meta.name.clone(), meta.data_type.clone()))
                    .collect();
                let all_rows = Self::map_json_rows(&columns, result.data);

                debug!(
                    "[ClickHouse] Query completed: {} rows, {} columns, {}ms",
                    all_rows.len(),
                    columns.len(),
                    elapsed_ms
                );
                Ok(Self::build_query_result(
                    columns,
                    column_meta,
                    all_rows,
                    sql_string,
                    elapsed_ms,
                ))
            }
            Err(query_err) => {
                debug!(
                    "[ClickHouse] Query fetch failed, trying execute: {}",
                    query_err
                );
                match client.query(sql).execute().await {
                    Ok(_) => {
                        let elapsed_ms = start.elapsed().as_millis();
                        debug!("[ClickHouse] Execute completed: {}ms", elapsed_ms);
                        Ok(Self::build_exec_result(sql_string, 0, elapsed_ms))
                    }
                    Err(e) => {
                        error!("[ClickHouse] Execute failed: {}, SQL: {}", e, sql_preview);
                        Ok(SqlResult::Error(SqlErrorInfo {
                            sql: sql_string,
                            message: e.to_string(),
                        }))
                    }
                }
            }
        }
    }

    async fn fetch_json_compact(
        client: &Client,
        sql: &str,
    ) -> Result<ClickHouseJsonCompactResult, DbError> {
        let mut cursor = client
            .query(sql)
            .fetch_bytes("JSONCompact")
            .map_err(|e| DbError::query_with_source("failed to fetch query bytes", e))?;
        let bytes = cursor
            .collect()
            .await
            .map_err(|e| DbError::query_with_source("failed to read query bytes", e))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| DbError::query_with_source("failed to parse JSON response", e))
    }

    fn map_json_rows(
        columns: &[String],
        data: Vec<Vec<serde_json::Value>>,
    ) -> Vec<Vec<Option<String>>> {
        data.into_iter()
            .map(|row| {
                let mut values = Vec::with_capacity(columns.len());
                for index in 0..columns.len() {
                    let value = row
                        .get(index)
                        .and_then(|value| Self::json_value_to_string(value));
                    values.push(value);
                }
                values
            })
            .collect()
    }

    fn json_value_to_string(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Null => None,
            serde_json::Value::Bool(_) => Some(value.to_string()),
            serde_json::Value::Number(_) => Some(value.to_string()),
            serde_json::Value::String(text) => Some(text.clone()),
            serde_json::Value::Array(_) => Some(value.to_string()),
            serde_json::Value::Object(_) => Some(value.to_string()),
        }
    }

    fn configured_database(config: &DbConnectionConfig) -> Option<String> {
        config
            .database
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }
}

#[derive(Debug, Deserialize)]
struct ClickHouseJsonMeta {
    name: String,
    #[serde(rename = "type")]
    data_type: String,
}

#[derive(Debug, Deserialize)]
struct ClickHouseJsonCompactResult {
    #[serde(default)]
    meta: Vec<ClickHouseJsonMeta>,
    #[serde(default)]
    data: Vec<Vec<serde_json::Value>>,
}

#[async_trait]
impl DbConnection for ClickHouseDbConnection {
    fn config(&self) -> &DbConnectionConfig {
        &self.config
    }

    fn set_config_database(&mut self, database: Option<String>) {
        self.config.database = database;
    }

    async fn connect(&mut self) -> Result<(), DbError> {
        let config = &self.config;
        info!("[ClickHouse] Connecting to {}:{}", config.host, config.port);
        let target = resolve_connection_target(config).await?;
        self.tunnel = target.tunnel;

        let protocol = config
            .get_param("schema")
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| matches!(value.as_str(), "http" | "https"))
            .unwrap_or_else(|| "http".to_string());

        let url = format!("{}://{}:{}", protocol, target.host, target.port);

        let mut client = Client::default()
            .with_url(&url)
            .with_user(&config.username)
            .with_password(&config.password);

        if let Some(db) = Self::configured_database(config) {
            client = client.with_database(&db);
            debug!("[ClickHouse] Using database: {}", db);
        }

        if let Some(compression) = config.get_param("compression") {
            if compression == "lz4" {
                client = client.with_compression(clickhouse::Compression::Lz4);
                debug!("[ClickHouse] Using LZ4 compression");
            }
        }

        // 获取连接超时，默认 30 秒
        let connect_timeout_secs = config.get_param_as::<u64>("connect_timeout").unwrap_or(30);
        debug!(
            "[ClickHouse] Testing connection with timeout {}s...",
            connect_timeout_secs
        );

        // 使用 tokio::timeout 包装连接测试
        let test_result = timeout(
            Duration::from_secs(connect_timeout_secs),
            client.query("SELECT 1").fetch_all::<u8>(),
        )
        .await;

        match test_result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                error!("[ClickHouse] Connection failed: {}", e);
                return Err(DbError::connection_with_source("failed to connect", e));
            }
            Err(_) => {
                error!(
                    "[ClickHouse] Connection timed out after {}s",
                    connect_timeout_secs
                );
                return Err(DbError::connection(format!(
                    "connection timed out after {}s",
                    connect_timeout_secs
                )));
            }
        }

        self.client = Some(client);
        info!("[ClickHouse] Connected successfully");

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DbError> {
        debug!("[ClickHouse] Disconnecting...");
        self.client = None;
        self.tunnel = None;
        info!("[ClickHouse] Disconnected");
        Ok(())
    }

    async fn execute(
        &self,
        plugin: &dyn DatabasePlugin,
        script: &str,
        options: ExecOptions,
    ) -> Result<Vec<SqlResult>, DbError> {
        debug!(
            "[ClickHouse] execute() called, stop_on_error={}",
            options.stop_on_error
        );
        let client = self.ensure_connected()?;

        let parser = plugin
            .create_parser(SqlSource::Script(script.to_string()))
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;
        let statements: Vec<String> = parser
            .filter_map(|r| r.ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        debug!("[ClickHouse] Split into {} statement(s)", statements.len());
        let mut results = Vec::new();

        for (idx, sql) in statements.iter().enumerate() {
            let sql = sql.trim();
            if sql.is_empty() {
                continue;
            }

            debug!(
                "[ClickHouse] Executing statement {}/{}",
                idx + 1,
                statements.len()
            );
            let result = Self::execute_single(client, sql).await?;

            let is_error = result.is_error();
            if is_error {
                debug!(
                    "[ClickHouse] Statement {}/{} returned error",
                    idx + 1,
                    statements.len()
                );
            }
            results.push(result);

            if is_error && options.stop_on_error {
                debug!("[ClickHouse] Stopping execution due to error (stop_on_error=true)");
                break;
            }
        }

        debug!(
            "[ClickHouse] execute() completed with {} result(s)",
            results.len()
        );
        Ok(results)
    }

    async fn query(&self, query: &str) -> Result<SqlResult, DbError> {
        debug!("[ClickHouse] query() called");
        let client = self.ensure_connected()?;

        Self::execute_single(client, query).await
    }

    async fn current_database(&self) -> Result<Option<String>, DbError> {
        debug!("[ClickHouse] Querying current database");
        let client = self.ensure_connected()?;
        let result = Self::execute_single(client, "SELECT currentDatabase() as name").await;
        match result {
            Ok(SqlResult::Query(query_result)) => {
                if let Some(row) = query_result.rows.first() {
                    if let Some(Some(name)) = row.first() {
                        debug!("[ClickHouse] Current database: {}", name);
                        return Ok(Some(name.clone()));
                    }
                }
                Ok(Self::configured_database(&self.config))
            }
            Ok(other) => {
                error!(
                    "[ClickHouse] Unexpected result when querying current database: {:?}",
                    other
                );
                Ok(Self::configured_database(&self.config))
            }
            Err(e) => {
                error!("[ClickHouse] Failed to query current database: {}", e);
                Ok(Self::configured_database(&self.config))
            }
        }
    }

    async fn switch_database(&self, database: &str) -> Result<(), DbError> {
        debug!("[ClickHouse] Switching to database: {}", database);
        let client = self.ensure_connected()?;

        let sql = format!("USE `{}`", database.replace("`", "``"));
        debug!("[ClickHouse] Executing: {}", sql);
        client.query(&sql).execute().await.map_err(|e| {
            error!(
                "[ClickHouse] Failed to switch database: {}, SQL: {}",
                e, sql
            );
            DbError::query_with_source("failed to switch database", e)
        })?;

        info!("[ClickHouse] Switched to database: {}", database);
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
            "[ClickHouse] execute_streaming() called, streaming={}",
            options.streaming
        );
        let client = self.ensure_connected()?;

        let total_size = source.file_size().unwrap_or(0);
        let is_file_source = source.is_file();

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
                        if options.stop_on_error {
                            break;
                        }
                        continue;
                    }
                };

                current += 1;
                debug!("[ClickHouse] Streaming statement {}", current);

                let result = match Self::execute_single(client, &sql).await {
                    Ok(r) => r,
                    Err(e) => {
                        let sql_preview = if sql.len() > 200 {
                            format!("{}...", truncate_str(&sql, 200))
                        } else {
                            sql.clone()
                        };
                        error!(
                            "[ClickHouse] Streaming statement {} failed: {}, SQL: {}",
                            current, e, sql_preview
                        );
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

                if is_error && options.stop_on_error {
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
            debug!("[ClickHouse] Streaming {} statement(s)", total);

            for (index, sql) in statements.into_iter().enumerate() {
                let current = index + 1;
                debug!("[ClickHouse] Streaming statement {}/{}", current, total);

                let result = match Self::execute_single(client, &sql).await {
                    Ok(r) => r,
                    Err(e) => {
                        let sql_preview = if sql.len() > 200 {
                            format!("{}...", truncate_str(&sql, 200))
                        } else {
                            sql.clone()
                        };
                        error!(
                            "[ClickHouse] Streaming statement {}/{} failed: {}, SQL: {}",
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

        debug!("[ClickHouse] execute_streaming() completed");
        Ok(())
    }
}
