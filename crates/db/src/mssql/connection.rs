use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use one_core::storage::DbConnectionConfig;
use tiberius::{AuthMethod, Client, Config, Row};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use tracing::{debug, error, info};

use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::ssh_tunnel::resolve_connection_target;
use crate::{format_message, truncate_str, DatabasePlugin};
use ssh::LocalPortForwardTunnel;

pub struct MssqlDbConnection {
    config: DbConnectionConfig,
    client: Arc<Mutex<Option<Client<Compat<TcpStream>>>>>,
    tunnel: Option<LocalPortForwardTunnel>,
}

impl MssqlDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            client: Arc::new(Mutex::new(None)),
            tunnel: None,
        }
    }

    fn extract_value(row: &Row, index: usize) -> Option<String> {
        row.try_get::<&str, _>(index)
            .ok()
            .flatten()
            .map(|s| s.to_string())
            .or_else(|| {
                row.try_get::<i32, _>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
            .or_else(|| {
                row.try_get::<i64, _>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
            .or_else(|| {
                row.try_get::<f64, _>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
            .or_else(|| {
                row.try_get::<bool, _>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.to_string())
            })
            .or_else(|| {
                use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

                row.try_get::<NaiveDateTime, _>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string())
                    .or_else(|| {
                        row.try_get::<NaiveDate, _>(index)
                            .ok()
                            .flatten()
                            .map(|v| v.format("%Y-%m-%d").to_string())
                    })
                    .or_else(|| {
                        row.try_get::<NaiveTime, _>(index)
                            .ok()
                            .flatten()
                            .map(|v| v.format("%H:%M:%S").to_string())
                    })
            })
    }

    fn build_query_result(
        columns: Vec<String>,
        column_types: Vec<String>,
        rows: Vec<Row>,
        sql: String,
        elapsed_ms: u128,
    ) -> SqlResult {
        debug!(
            "[MSSQL] Query returned {} rows, {} columns: {:?}",
            rows.len(),
            columns.len(),
            columns
        );

        let column_meta: Vec<QueryColumnMeta> = columns
            .iter()
            .zip(column_types.iter())
            .map(|(name, db_type)| QueryColumnMeta::new(name.clone(), db_type.clone()))
            .collect();

        let all_rows: Vec<Vec<Option<String>>> = rows
            .iter()
            .map(|row| {
                (0..columns.len())
                    .map(|i| Self::extract_value(row, i))
                    .collect()
            })
            .collect();

        SqlResult::Query(QueryResult {
            sql,
            columns,
            column_meta,
            rows: all_rows,
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

    async fn execute_single(
        client: &mut Client<Compat<TcpStream>>,
        sql: &str,
    ) -> Result<SqlResult, DbError> {
        let start = Instant::now();
        let sql_string = sql.to_string();
        let sql_preview = if sql.len() > 200 {
            format!("{}...", truncate_str(sql, 200))
        } else {
            sql.to_string()
        };
        debug!("[MSSQL] Executing SQL: {}", sql_preview);

        match client.query(sql, &[]).await {
            Ok(mut stream) => {
                debug!("[MSSQL] Query submitted successfully, fetching columns...");
                let (columns, column_types): (Vec<String>, Vec<String>) =
                    match stream.columns().await {
                        Ok(Some(cols)) => cols
                            .iter()
                            .map(|c| (c.name().to_string(), format!("{:?}", c.column_type())))
                            .unzip(),
                        _ => (Vec::new(), Vec::new()),
                    };

                if columns.is_empty() {
                    let rows_affected = stream
                        .into_results()
                        .await
                        .map(|results| results.iter().map(|r| r.len() as u64).sum())
                        .unwrap_or(0);
                    let elapsed_ms = start.elapsed().as_millis();
                    debug!(
                        "[MSSQL] Execute completed: {} rows affected, {}ms",
                        rows_affected, elapsed_ms
                    );
                    Ok(Self::build_exec_result(
                        sql_string,
                        rows_affected,
                        elapsed_ms,
                    ))
                } else {
                    debug!("[MSSQL] Fetching result rows, columns: {:?}", columns);
                    match stream.into_first_result().await {
                        Ok(rows) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!(
                                "[MSSQL] Query completed: {} rows returned, {}ms",
                                rows.len(),
                                elapsed_ms
                            );
                            Ok(Self::build_query_result(
                                columns,
                                column_types,
                                rows,
                                sql_string,
                                elapsed_ms,
                            ))
                        }
                        Err(e) => {
                            error!("[MSSQL] Failed to fetch result rows: {}", e);
                            Ok(SqlResult::Error(SqlErrorInfo {
                                sql: sql_string,
                                message: e.to_string(),
                            }))
                        }
                    }
                }
            }
            Err(e) => {
                error!("[MSSQL] Query execution failed: {}", e);
                Ok(SqlResult::Error(SqlErrorInfo {
                    sql: sql_string,
                    message: e.to_string(),
                }))
            }
        }
    }
}

#[async_trait]
impl DbConnection for MssqlDbConnection {
    fn config(&self) -> &DbConnectionConfig {
        &self.config
    }

    fn set_config_database(&mut self, database: Option<String>) {
        self.config.database = database;
    }

    async fn connect(&mut self) -> Result<(), DbError> {
        let config = &self.config;
        info!("[MSSQL] Connecting to {}:{}", config.host, config.port);
        let target = resolve_connection_target(config).await?;
        self.tunnel = target.tunnel;

        let mut tiberius_config = Config::new();
        tiberius_config.host(&target.host);
        tiberius_config.port(target.port);
        tiberius_config.authentication(AuthMethod::sql_server(&config.username, &config.password));

        if config
            .get_param("trust_cert")
            .map(|v| v != "false")
            .unwrap_or(true)
        {
            tiberius_config.trust_cert();
        }

        let encrypt = config
            .get_param("encrypt")
            .map(|s| s.as_str())
            .unwrap_or("off");
        match encrypt {
            "on" => tiberius_config.encryption(tiberius::EncryptionLevel::On),
            "required" => tiberius_config.encryption(tiberius::EncryptionLevel::Required),
            _ => tiberius_config.encryption(tiberius::EncryptionLevel::NotSupported),
        };

        if let Some(app_name) = config.get_param("application_name") {
            tiberius_config.application_name(app_name);
        }

        if let Some(ref db) = config.database {
            tiberius_config.database(db);
            debug!("[MSSQL] Using database: {}", db);
        }

        let connect_timeout = config.get_param_as::<u64>("connect_timeout").unwrap_or(30);
        debug!("[MSSQL] Connect timeout: {}s", connect_timeout);

        debug!("[MSSQL] Establishing TCP connection...");
        let tcp = tokio::time::timeout(
            std::time::Duration::from_secs(connect_timeout),
            TcpStream::connect(tiberius_config.get_addr()),
        )
        .await
        .map_err(|_| DbError::connection("connection timeout"))?
        .map_err(|e| {
            error!("[MSSQL] TCP connection failed: {}", e);
            DbError::connection_with_source("failed to connect to TCP", e)
        })?;
        debug!("[MSSQL] TCP connection established");

        debug!("[MSSQL] Authenticating with SQL Server...");
        let client = Client::connect(tiberius_config, tcp.compat_write())
            .await
            .map_err(|e| {
                error!("[MSSQL] Authentication failed: {}", e);
                DbError::connection_with_source("failed to connect to MSSQL", e)
            })?;
        info!("[MSSQL] Connected successfully");

        {
            let mut guard = self.client.lock().await;
            *guard = Some(client);
        }

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DbError> {
        debug!("[MSSQL] Disconnecting...");
        let mut guard = self.client.lock().await;
        *guard = None;
        self.tunnel = None;
        info!("[MSSQL] Disconnected");
        Ok(())
    }

    async fn execute(
        &self,
        plugin: &dyn DatabasePlugin,
        script: &str,
        options: ExecOptions,
    ) -> Result<Vec<SqlResult>, DbError> {
        debug!(
            "[MSSQL] execute() called, transactional={}, stop_on_error={}",
            options.transactional, options.stop_on_error
        );
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;

        let parser = plugin
            .create_parser(SqlSource::Script(script.to_string()))
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;
        let statements: Vec<String> = parser
            .filter_map(|r| r.ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        debug!("[MSSQL] Split into {} statement(s)", statements.len());
        let mut results = Vec::new();

        if options.transactional {
            let non_empty_statements: Vec<&str> = statements
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if non_empty_statements.is_empty() {
                debug!("[MSSQL] No statements to execute");
                return Ok(results);
            }

            let batch_sql = format!(
                "SET XACT_ABORT ON;\nBEGIN TRANSACTION;\n{}\nCOMMIT;",
                non_empty_statements.join(";\n")
            );

            debug!(
                "[MSSQL] Executing transactional batch with {} statement(s)",
                non_empty_statements.len()
            );
            let start = Instant::now();
            match client.execute(&batch_sql, &[]).await {
                Ok(exec_result) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    let rows_affected = exec_result.total();
                    debug!(
                        "[MSSQL] Transactional batch completed: {} rows affected, {}ms",
                        rows_affected, elapsed_ms
                    );
                    results.push(SqlResult::Exec(ExecResult {
                        sql: batch_sql,
                        rows_affected,
                        elapsed_ms,
                        message: Some(format!(
                            "Executed {} statement(s), {} row(s) affected",
                            non_empty_statements.len(),
                            rows_affected
                        )),
                    }));
                }
                Err(e) => {
                    error!("[MSSQL] Transactional batch failed: {}", e);
                    results.push(SqlResult::Error(SqlErrorInfo {
                        sql: batch_sql,
                        message: e.to_string(),
                    }));
                }
            }
        } else {
            for (idx, sql) in statements.iter().enumerate() {
                let sql = sql.trim();
                if sql.is_empty() {
                    continue;
                }

                debug!(
                    "[MSSQL] Executing statement {}/{}",
                    idx + 1,
                    statements.len()
                );
                let result = Self::execute_single(client, sql).await?;

                let is_error = result.is_error();
                if is_error {
                    debug!(
                        "[MSSQL] Statement {}/{} returned error",
                        idx + 1,
                        statements.len()
                    );
                }
                results.push(result);

                if is_error && options.stop_on_error {
                    debug!("[MSSQL] Stopping execution due to error (stop_on_error=true)");
                    break;
                }
            }
        }

        debug!(
            "[MSSQL] execute() completed with {} result(s)",
            results.len()
        );
        Ok(results)
    }

    async fn query(&self, query: &str) -> Result<SqlResult, DbError> {
        debug!("[MSSQL] Acquiring client lock...");
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;
        debug!("[MSSQL] Lock acquired");

        debug!(
            "[MSSQL] Executing query: {}",
            &query[..query.len().min(100)]
        );
        Self::execute_single(client, query).await
    }

    async fn current_database(&self) -> Result<Option<String>, DbError> {
        debug!("[MSSQL] Querying current database");
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;

        let result = match client.query("SELECT DB_NAME()", &[]).await {
            Ok(stream) => match stream.into_first_result().await {
                Ok(rows) => {
                    if let Some(row) = rows.first() {
                        let db = row
                            .try_get::<&str, _>(0)
                            .ok()
                            .flatten()
                            .map(|s| s.to_string());
                        debug!("[MSSQL] Current database: {:?}", db);
                        db
                    } else {
                        debug!("[MSSQL] No rows returned for current database query");
                        None
                    }
                }
                Err(e) => {
                    error!("[MSSQL] Failed to fetch current database result: {}", e);
                    None
                }
            },
            Err(e) => {
                error!("[MSSQL] Failed to query current database: {}", e);
                None
            }
        };
        Ok(result)
    }

    async fn switch_database(&self, database: &str) -> Result<(), DbError> {
        debug!("[MSSQL] Switching to database: {}", database);
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;

        let sql = format!("USE [{}]", database.replace("]", "]]"));
        debug!("[MSSQL] Executing: {}", sql);
        client.execute(&sql, &[]).await.map_err(|e| {
            error!("[MSSQL] Failed to switch database: {}", e);
            DbError::query_with_source("failed to switch database", e)
        })?;

        info!("[MSSQL] Switched to database: {}", database);
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
            "[MSSQL] execute_streaming() called, transactional={}, streaming={}",
            options.transactional, options.streaming
        );
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;

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
                debug!("[MSSQL] Streaming statement {}", current);

                let result = match Self::execute_single(client, &sql).await {
                    Ok(r) => r,
                    Err(e) => {
                        error!("[MSSQL] Streaming statement {} failed: {}", current, e);
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
            debug!("[MSSQL] Streaming {} statement(s)", total);

            if options.transactional {
                if statements.is_empty() {
                    debug!("[MSSQL] No statements to execute");
                    return Ok(());
                }

                let batch_sql = format!(
                    "SET XACT_ABORT ON;\nBEGIN TRANSACTION;\n{}\nCOMMIT;",
                    statements.join(";\n")
                );

                debug!("[MSSQL] Executing transactional batch in streaming mode");
                let start = Instant::now();
                let result = match client.execute(&batch_sql, &[]).await {
                    Ok(exec_result) => {
                        let elapsed_ms = start.elapsed().as_millis();
                        let rows_affected = exec_result.total();
                        SqlResult::Exec(ExecResult {
                            sql: batch_sql,
                            rows_affected,
                            elapsed_ms,
                            message: Some(format!(
                                "Executed {} statement(s), {} row(s) affected",
                                total, rows_affected
                            )),
                        })
                    }
                    Err(e) => {
                        error!("[MSSQL] Transactional batch failed: {}", e);
                        SqlResult::Error(SqlErrorInfo {
                            sql: batch_sql,
                            message: e.to_string(),
                        })
                    }
                };

                let progress = StreamingProgress::new(total, total, result);
                let _ = sender.send(progress).await;
            } else {
                for (index, sql) in statements.into_iter().enumerate() {
                    let current = index + 1;
                    debug!("[MSSQL] Streaming statement {}/{}", current, total);

                    let result = match Self::execute_single(client, &sql).await {
                        Ok(r) => r,
                        Err(e) => {
                            error!(
                                "[MSSQL] Streaming statement {}/{} failed: {}",
                                current, total, e
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

        debug!("[MSSQL] execute_streaming() completed");
        Ok(())
    }
}
