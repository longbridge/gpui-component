use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use one_core::storage::DbConnectionConfig;
use tokio::sync::Mutex;
use tokio_postgres::{types::Type, Client, Config, NoTls, Row, Statement};
use tracing::{debug, error, info};

use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::{format_message, truncate_str, DatabasePlugin};
use tokio::sync::mpsc;

pub struct PostgresDbConnection {
    config: DbConnectionConfig,
    client: Arc<Mutex<Option<Client>>>,
}

impl PostgresDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            client: Arc::new(Mutex::new(None)),
        }
    }

    /// Extract value from PostgreSQL row
    fn extract_value(row: &Row, index: usize) -> Option<String> {
        // Get column type
        let column = &row.columns()[index];
        let col_type = column.type_();

        // Try to get the value based on type
        match col_type {
            // Boolean
            &Type::BOOL => row
                .try_get::<_, Option<bool>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),

            // Integer types
            &Type::INT2 => row
                .try_get::<_, Option<i16>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),
            &Type::INT4 => row
                .try_get::<_, Option<i32>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),
            &Type::INT8 => row
                .try_get::<_, Option<i64>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),

            // Floating point types
            &Type::FLOAT4 => row
                .try_get::<_, Option<f32>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),
            &Type::FLOAT8 => row
                .try_get::<_, Option<f64>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),

            // Numeric/Decimal - try as f64
            &Type::NUMERIC => row
                .try_get::<_, Option<f64>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),

            // Text types
            &Type::TEXT | &Type::VARCHAR | &Type::BPCHAR | &Type::NAME => {
                row.try_get::<_, Option<String>>(index).ok().flatten()
            }

            // Date and Time types
            &Type::TIMESTAMP => {
                use chrono::NaiveDateTime;
                row.try_get::<_, Option<NaiveDateTime>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string())
            }
            &Type::TIMESTAMPTZ => {
                use chrono::{DateTime, Utc};
                row.try_get::<_, Option<DateTime<Utc>>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.format("%Y-%m-%d %H:%M:%S %z").to_string())
            }
            &Type::DATE => {
                use chrono::NaiveDate;
                row.try_get::<_, Option<NaiveDate>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.format("%Y-%m-%d").to_string())
            }
            &Type::TIME => {
                use chrono::NaiveTime;
                row.try_get::<_, Option<NaiveTime>>(index)
                    .ok()
                    .flatten()
                    .map(|v| v.format("%H:%M:%S").to_string())
            }

            // Binary types
            &Type::BYTEA => row
                .try_get::<_, Option<Vec<u8>>>(index)
                .ok()
                .flatten()
                .map(|v| format!("\\x{}", hex::encode(&v))),

            // JSON types
            &Type::JSON | &Type::JSONB => row
                .try_get::<_, Option<serde_json::Value>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),

            // UUID
            &Type::UUID => row
                .try_get::<_, Option<uuid::Uuid>>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string()),

            // Array types - try to get as string representation
            _ if col_type.name().ends_with("[]") => {
                // For arrays, try to get as string
                row.try_get::<_, Option<String>>(index)
                    .ok()
                    .flatten()
                    .or_else(|| Some(format!("<array: {}>", col_type.name())))
            }

            // Default: try as string, otherwise show type info
            _ => row
                .try_get::<_, Option<String>>(index)
                .ok()
                .flatten()
                .or_else(|| Some(format!("<{}>", col_type.name()))),
        }
    }

    fn build_query_result(
        stmt: &Statement,
        rows: Vec<Row>,
        sql: String,
        elapsed_ms: u128,
    ) -> SqlResult {
        let columns: Vec<String> = stmt
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect();

        let column_meta: Vec<QueryColumnMeta> = stmt
            .columns()
            .iter()
            .map(|col| {
                let name = col.name().to_string();
                let db_type = col.type_().name().to_string();
                QueryColumnMeta::new(name, db_type)
            })
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
}

#[async_trait]
impl DbConnection for PostgresDbConnection {
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
        let config = &self.config;
        info!("[PostgreSQL] Connecting to {}:{}", config.host, config.port);

        let mut pg_config = Config::new();
        pg_config
            .host(&config.host)
            .port(config.port)
            .user(&config.username)
            .password(&config.password);

        if let Some(ref db) = config.database {
            pg_config.dbname(db);
            debug!("[PostgreSQL] Using database: {}", db);
        }

        // Apply extra params
        if let Some(timeout) = config.get_param_as::<u64>("connect_timeout") {
            pg_config.connect_timeout(std::time::Duration::from_secs(timeout));
            debug!("[PostgreSQL] Connect timeout: {}s", timeout);
        }
        if let Some(app_name) = config.get_param("application_name") {
            pg_config.application_name(app_name);
            debug!("[PostgreSQL] Application name: {}", app_name);
        }

        // Connect to PostgreSQL
        debug!("[PostgreSQL] Establishing connection...");
        let (client, connection) = pg_config.connect(NoTls).await.map_err(|e| {
            error!("[PostgreSQL] Connection failed: {}", e);
            DbError::connection_with_source("failed to connect", e)
        })?;

        // Spawn the connection task in background - it handles communication with the server
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("[PostgreSQL] Connection error: {}", e);
            }
        });

        {
            let mut guard = self.client.lock().await;
            *guard = Some(client);
        }

        info!("[PostgreSQL] Connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DbError> {
        debug!("[PostgreSQL] Disconnecting...");
        let mut guard = self.client.lock().await;
        *guard = None;
        info!("[PostgreSQL] Disconnected");
        Ok(())
    }

    async fn execute(
        &self,
        plugin: &dyn DatabasePlugin,
        script: &str,
        options: ExecOptions,
    ) -> Result<Vec<SqlResult>, DbError> {
        debug!(
            "[PostgreSQL] execute() called, transactional={}, stop_on_error={}",
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
        debug!("[PostgreSQL] Split into {} statement(s)", statements.len());
        let mut results = Vec::new();

        if options.transactional {
            debug!("[PostgreSQL] Starting transaction...");
            let tx = client.transaction().await.map_err(|e| {
                error!("[PostgreSQL] Failed to begin transaction: {}", e);
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
                    "[PostgreSQL] TX executing statement {}/{}, {}",
                    idx + 1,
                    statements.len(),
                    sql_preview
                );
                let start = Instant::now();

                let result = match tx.prepare(sql).await {
                    Ok(stmt) => {
                        if stmt.columns().is_empty() {
                            match tx.execute(&stmt, &[]).await {
                                Ok(rows_affected) => {
                                    let elapsed_ms = start.elapsed().as_millis();
                                    debug!(
                                        "[PostgreSQL] TX execute completed: {} rows affected, {}ms",
                                        rows_affected, elapsed_ms
                                    );
                                    Self::build_exec_result(
                                        sql.to_string(),
                                        rows_affected,
                                        elapsed_ms,
                                    )
                                }
                                Err(e) => {
                                    error!(
                                        "[PostgreSQL] TX execute failed: {}, SQL: {}",
                                        e, sql_preview
                                    );
                                    SqlResult::Error(SqlErrorInfo {
                                        sql: sql.to_string(),
                                        message: e.to_string(),
                                    })
                                }
                            }
                        } else {
                            match tx.query(&stmt, &[]).await {
                                Ok(rows) => {
                                    let elapsed_ms = start.elapsed().as_millis();
                                    debug!(
                                        "[PostgreSQL] TX query completed: {} rows, {}ms",
                                        rows.len(),
                                        elapsed_ms
                                    );
                                    Self::build_query_result(
                                        &stmt,
                                        rows,
                                        sql.to_string(),
                                        elapsed_ms,
                                    )
                                }
                                Err(e) => {
                                    error!(
                                        "[PostgreSQL] TX query failed: {}, SQL: {}",
                                        e, sql_preview
                                    );
                                    SqlResult::Error(SqlErrorInfo {
                                        sql: sql.to_string(),
                                        message: e.to_string(),
                                    })
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "[PostgreSQL] TX prepare failed: {}, SQL: {}",
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
                        "[PostgreSQL] TX statement {}/{} returned error, will rollback",
                        idx + 1,
                        statements.len()
                    );
                    break;
                }
            }

            let has_error = results.iter().any(|r| r.is_error());
            if has_error {
                debug!("[PostgreSQL] Rolling back transaction...");
                tx.rollback().await.map_err(|e| {
                    error!("[PostgreSQL] Failed to rollback: {}", e);
                    DbError::transaction_with_source("failed to rollback", e)
                })?;
                debug!("[PostgreSQL] Transaction rolled back");
            } else {
                debug!("[PostgreSQL] Committing transaction...");
                tx.commit().await.map_err(|e| {
                    error!("[PostgreSQL] Failed to commit: {}", e);
                    DbError::transaction_with_source("failed to commit", e)
                })?;
                debug!("[PostgreSQL] Transaction committed");
            }
        } else {
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

                // PostgreSQL doesn't have USE statement, but has SET search_path
                let sql_upper = sql.to_uppercase();
                if sql_upper.starts_with("SET SEARCH_PATH") {
                    debug!("[PostgreSQL] Executing search_path: {}", sql);
                    let start = Instant::now();
                    match client.execute(sql, &[]).await {
                        Ok(_) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!("[PostgreSQL] Search path changed, {}ms", elapsed_ms);
                            results.push(SqlResult::Exec(ExecResult {
                                sql: sql.to_string(),
                                rows_affected: 0,
                                elapsed_ms,
                                message: Some("Search path changed".to_string()),
                            }));
                        }
                        Err(e) => {
                            error!(
                                "[PostgreSQL] Failed to change search path: {}, SQL: {}",
                                e, sql_preview
                            );
                            results.push(SqlResult::Error(SqlErrorInfo {
                                sql: sql.to_string(),
                                message: e.to_string(),
                            }));

                            if options.stop_on_error {
                                debug!("[PostgreSQL] Stopping execution due to error (stop_on_error=true)");
                                break;
                            }
                        }
                    }
                    continue;
                }

                debug!(
                    "[PostgreSQL] Executing statement {}/{}",
                    idx + 1,
                    statements.len()
                );
                let start = Instant::now();

                let result = match client.prepare(sql).await {
                    Ok(stmt) => {
                        if stmt.columns().is_empty() {
                            match client.execute(&stmt, &[]).await {
                                Ok(rows_affected) => {
                                    let elapsed_ms = start.elapsed().as_millis();
                                    debug!(
                                        "[PostgreSQL] Execute completed: {} rows affected, {}ms",
                                        rows_affected, elapsed_ms
                                    );
                                    Self::build_exec_result(
                                        sql.to_string(),
                                        rows_affected,
                                        elapsed_ms,
                                    )
                                }
                                Err(e) => {
                                    error!(
                                        "[PostgreSQL] Execute failed: {}, SQL: {}",
                                        e, sql_preview
                                    );
                                    results.push(SqlResult::Error(SqlErrorInfo {
                                        sql: sql.to_string(),
                                        message: e.to_string(),
                                    }));

                                    if options.stop_on_error {
                                        debug!("[PostgreSQL] Stopping execution due to error (stop_on_error=true)");
                                        break;
                                    }
                                    continue;
                                }
                            }
                        } else {
                            match client.query(&stmt, &[]).await {
                                Ok(rows) => {
                                    let elapsed_ms = start.elapsed().as_millis();
                                    debug!(
                                        "[PostgreSQL] Query completed: {} rows, {}ms",
                                        rows.len(),
                                        elapsed_ms
                                    );
                                    Self::build_query_result(
                                        &stmt,
                                        rows,
                                        sql.to_string(),
                                        elapsed_ms,
                                    )
                                }
                                Err(e) => {
                                    error!(
                                        "[PostgreSQL] Query failed: {}, SQL: {}",
                                        e, sql_preview
                                    );
                                    results.push(SqlResult::Error(SqlErrorInfo {
                                        sql: sql.to_string(),
                                        message: e.to_string(),
                                    }));

                                    if options.stop_on_error {
                                        debug!("[PostgreSQL] Stopping execution due to error (stop_on_error=true)");
                                        break;
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("[PostgreSQL] Prepare failed: {}, SQL: {}", e, sql_preview);
                        results.push(SqlResult::Error(SqlErrorInfo {
                            sql: sql.to_string(),
                            message: e.to_string(),
                        }));

                        if options.stop_on_error {
                            debug!(
                                "[PostgreSQL] Stopping execution due to error (stop_on_error=true)"
                            );
                            break;
                        }
                        continue;
                    }
                };

                results.push(result);
            }
        }

        debug!(
            "[PostgreSQL] execute() completed with {} result(s)",
            results.len()
        );
        Ok(results)
    }

    async fn query(&self, query: &str) -> Result<SqlResult, DbError> {
        debug!("[PostgreSQL] query() called");
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;

        let start = Instant::now();
        let query_string = query.to_string();
        let sql_preview = if query.len() > 200 {
            format!("{}...", truncate_str(query, 200))
        } else {
            query.to_string()
        };
        debug!("[PostgreSQL] Executing query: {}", sql_preview);

        match client.prepare(&query_string).await {
            Ok(stmt) => {
                if stmt.columns().is_empty() {
                    match client.execute(&stmt, &vec![]).await {
                        Ok(rows_affected) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!(
                                "[PostgreSQL] Execute completed: {} rows affected, {}ms",
                                rows_affected, elapsed_ms
                            );
                            Ok(Self::build_exec_result(
                                query_string,
                                rows_affected,
                                elapsed_ms,
                            ))
                        }
                        Err(e) => {
                            error!("[PostgreSQL] Execute failed: {}, SQL: {}", e, sql_preview);
                            Ok(SqlResult::Error(SqlErrorInfo {
                                sql: query_string,
                                message: e.to_string(),
                            }))
                        }
                    }
                } else {
                    match client.query(&stmt, &vec![]).await {
                        Ok(rows) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            debug!(
                                "[PostgreSQL] Query completed: {} rows, {}ms",
                                rows.len(),
                                elapsed_ms
                            );
                            Ok(Self::build_query_result(
                                &stmt,
                                rows,
                                query_string,
                                elapsed_ms,
                            ))
                        }
                        Err(e) => {
                            error!("[PostgreSQL] Query failed: {}, SQL: {}", e, sql_preview);
                            Ok(SqlResult::Error(SqlErrorInfo {
                                sql: query_string,
                                message: e.to_string(),
                            }))
                        }
                    }
                }
            }
            Err(e) => {
                error!("[PostgreSQL] Prepare failed: {}, SQL: {}", e, sql_preview);
                Ok(SqlResult::Error(SqlErrorInfo {
                    sql: query_string,
                    message: e.to_string(),
                }))
            }
        }
    }

    async fn current_database(&self) -> Result<Option<String>, DbError> {
        debug!("[PostgreSQL] Querying current database");
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;

        let row = client
            .query_one("SELECT current_database()", &[])
            .await
            .map_err(|e| {
                error!("[PostgreSQL] Failed to get current database: {}", e);
                DbError::query_with_source("failed to get current database", e)
            })?;

        let db = row.try_get::<_, Option<String>>(0).ok().flatten();
        debug!("[PostgreSQL] Current database: {:?}", db);
        Ok(db)
    }

    async fn switch_database(&self, _database: &str) -> Result<(), DbError> {
        // PostgreSQL doesn't support switching databases within a connection
        // The connection must be recreated to connect to a different database
        error!("[PostgreSQL] Attempted to switch database - not supported");
        Err(DbError::NotSupported(
            "PostgreSQL does not support switching databases within a connection. Please create a new connection.".to_string()
        ))
    }

    async fn switch_schema(&self, schema: &str) -> Result<(), DbError> {
        debug!("[PostgreSQL] Switching to schema: {}", schema);
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or(DbError::NotConnected)?;

        let sql = format!("SET search_path TO \"{}\"", schema.replace("\"", "\"\""));
        debug!("[PostgreSQL] Executing: {}", sql);
        client.execute(&sql, &[]).await.map_err(|e| {
            error!("[PostgreSQL] Failed to switch schema: {}, SQL: {}", e, sql);
            DbError::query_with_source("failed to switch schema", e)
        })?;

        info!("[PostgreSQL] Switched to schema: {}", schema);
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
            "[PostgreSQL] execute_streaming() called, transactional={}, streaming={}",
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
            let mut has_error = false;

            if options.transactional {
                debug!("[PostgreSQL] Starting transaction for streaming...");
                let tx = client.transaction().await.map_err(|e| {
                    error!("[PostgreSQL] Failed to begin transaction: {}", e);
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
                    debug!("[PostgreSQL] Streaming TX statement {}", current);
                    let start = Instant::now();

                    let result = match tx.prepare(&sql).await {
                        Ok(stmt) => {
                            if stmt.columns().is_empty() {
                                match tx.execute(&stmt, &[]).await {
                                    Ok(rows_affected) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_exec_result(
                                            sql.clone(),
                                            rows_affected,
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming TX execute failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            } else {
                                match tx.query(&stmt, &[]).await {
                                    Ok(rows) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_query_result(
                                            &stmt,
                                            rows,
                                            sql.clone(),
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming TX query failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "[PostgreSQL] Streaming TX prepare failed: {}, SQL: {}",
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
                    let sql_preview = if sql.len() > 200 {
                        format!("{}...", truncate_str(&sql, 200))
                    } else {
                        sql.clone()
                    };
                    debug!("[PostgreSQL] Streaming statement {}", current);
                    let start = Instant::now();

                    let result = match client.prepare(&sql).await {
                        Ok(stmt) => {
                            if stmt.columns().is_empty() {
                                match client.execute(&stmt, &[]).await {
                                    Ok(rows_affected) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_exec_result(
                                            sql.clone(),
                                            rows_affected,
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming execute failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            } else {
                                match client.query(&stmt, &[]).await {
                                    Ok(rows) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_query_result(
                                            &stmt,
                                            rows,
                                            sql.clone(),
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming query failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "[PostgreSQL] Streaming prepare failed: {}, SQL: {}",
                                e, sql_preview
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
            debug!("[PostgreSQL] Streaming {} statement(s)", total);

            if options.transactional {
                debug!("[PostgreSQL] Starting transaction for streaming...");
                let tx = client.transaction().await.map_err(|e| {
                    error!("[PostgreSQL] Failed to begin transaction: {}", e);
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
                    debug!("[PostgreSQL] Streaming TX statement {}/{}", current, total);
                    let start = Instant::now();

                    let result = match tx.prepare(&sql).await {
                        Ok(stmt) => {
                            if stmt.columns().is_empty() {
                                match tx.execute(&stmt, &[]).await {
                                    Ok(rows_affected) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_exec_result(
                                            sql.clone(),
                                            rows_affected,
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming TX execute failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            } else {
                                match tx.query(&stmt, &[]).await {
                                    Ok(rows) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_query_result(
                                            &stmt,
                                            rows,
                                            sql.clone(),
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming TX query failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "[PostgreSQL] Streaming TX prepare failed: {}, SQL: {}",
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
                    let sql_preview = if sql.len() > 200 {
                        format!("{}...", truncate_str(&sql, 200))
                    } else {
                        sql.clone()
                    };
                    debug!("[PostgreSQL] Streaming statement {}/{}", current, total);
                    let start = Instant::now();

                    let result = match client.prepare(&sql).await {
                        Ok(stmt) => {
                            if stmt.columns().is_empty() {
                                match client.execute(&stmt, &[]).await {
                                    Ok(rows_affected) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_exec_result(
                                            sql.clone(),
                                            rows_affected,
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming execute failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            } else {
                                match client.query(&stmt, &[]).await {
                                    Ok(rows) => {
                                        let elapsed_ms = start.elapsed().as_millis();
                                        Self::build_query_result(
                                            &stmt,
                                            rows,
                                            sql.clone(),
                                            elapsed_ms,
                                        )
                                    }
                                    Err(e) => {
                                        error!(
                                            "[PostgreSQL] Streaming query failed: {}, SQL: {}",
                                            e, sql_preview
                                        );
                                        SqlResult::Error(SqlErrorInfo {
                                            sql: sql.clone(),
                                            message: e.to_string(),
                                        })
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "[PostgreSQL] Streaming prepare failed: {}, SQL: {}",
                                e, sql_preview
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

        debug!("[PostgreSQL] execute_streaming() completed");
        Ok(())
    }
}
