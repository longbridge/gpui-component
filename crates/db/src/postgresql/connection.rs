use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use one_core::storage::DbConnectionConfig;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig as RustlsClientConfig, DigitallySignedStruct,
    Error as RustlsError, RootCertStore, SignatureScheme,
};
use tokio::sync::Mutex;
use tokio_postgres::{config::SslMode, types::Type, Client, Config, NoTls, Row, Statement};
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::{debug, error, info, warn};

use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::executor::{
    ExecOptions, ExecResult, QueryColumnMeta, QueryResult, SqlErrorInfo, SqlResult, SqlSource,
};
use crate::rustls_provider::ensure_rustls_crypto_provider;
use crate::ssh_tunnel::resolve_connection_target;
use crate::{format_message, truncate_str, DatabasePlugin};
use ssh::LocalPortForwardTunnel;
use tokio::sync::mpsc;

#[derive(Debug)]
struct PostgresServerCertVerifier {
    inner: Arc<dyn ServerCertVerifier>,
    accept_invalid_certs: bool,
    accept_invalid_hostnames: bool,
}

impl PostgresServerCertVerifier {
    fn new(
        inner: Arc<dyn ServerCertVerifier>,
        accept_invalid_certs: bool,
        accept_invalid_hostnames: bool,
    ) -> Self {
        Self {
            inner,
            accept_invalid_certs,
            accept_invalid_hostnames,
        }
    }

    fn should_ignore_certificate_error(&self, error: &CertificateError) -> bool {
        if self.accept_invalid_certs {
            return self.accept_invalid_hostnames
                || !matches!(
                    error,
                    CertificateError::NotValidForName
                        | CertificateError::NotValidForNameContext { .. }
                );
        }

        self.accept_invalid_hostnames
            && matches!(
                error,
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. }
            )
    }
}

impl ServerCertVerifier for PostgresServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        match self.inner.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Ok(verified) => Ok(verified),
            Err(RustlsError::InvalidCertificate(error))
                if self.should_ignore_certificate_error(&error) =>
            {
                Ok(ServerCertVerified::assertion())
            }
            Err(error) => Err(error),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

pub struct PostgresDbConnection {
    config: DbConnectionConfig,
    client: Arc<Mutex<Option<Client>>>,
    tunnel: Option<LocalPortForwardTunnel>,
}

impl PostgresDbConnection {
    pub fn new(config: DbConnectionConfig) -> Self {
        Self {
            config,
            client: Arc::new(Mutex::new(None)),
            tunnel: None,
        }
    }

    fn ssl_mode(config: &DbConnectionConfig) -> SslMode {
        match config
            .get_param("ssl_mode")
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("disable") => SslMode::Disable,
            Some("require") => SslMode::Require,
            _ => SslMode::Prefer,
        }
    }

    fn load_root_certificates(path: &Path) -> Result<Vec<CertificateDer<'static>>, DbError> {
        let cert_bytes = fs::read(path).map_err(|error| {
            DbError::connection(format!(
                "failed to read PostgreSQL root certificate: {}",
                error
            ))
        })?;

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("der") => Ok(vec![CertificateDer::from(cert_bytes)]),
            _ => {
                let mut reader = BufReader::new(cert_bytes.as_slice());
                let certificates = rustls_pemfile::certs(&mut reader)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| {
                        DbError::connection_with_source("invalid PEM certificate", error)
                    })?;

                if certificates.is_empty() {
                    return Err(DbError::connection(
                        "PostgreSQL root certificate file does not contain any certificates",
                    ));
                }

                Ok(certificates)
            }
        }
    }

    fn build_root_cert_store(config: &DbConnectionConfig) -> Result<RootCertStore, DbError> {
        let mut root_store = RootCertStore::empty();
        let native_certificates = rustls_native_certs::load_native_certs();

        for error in native_certificates.errors {
            warn!(
                "[PostgreSQL] Failed to load native root certificate: {}",
                error
            );
        }
        for certificate in native_certificates.certs {
            root_store.add(certificate).map_err(|error| {
                DbError::connection_with_source(
                    "failed to add native PostgreSQL root certificate",
                    error,
                )
            })?;
        }

        if let Some(path) = config
            .get_param("ssl_root_cert_path")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            for certificate in Self::load_root_certificates(Path::new(path))? {
                root_store.add(certificate).map_err(|error| {
                    DbError::connection_with_source(
                        "failed to add PostgreSQL root certificate",
                        error,
                    )
                })?;
            }
        }

        Ok(root_store)
    }

    fn build_tls_connector(config: &DbConnectionConfig) -> Result<MakeRustlsConnect, DbError> {
        ensure_rustls_crypto_provider();

        let accept_invalid_certs = config.get_param_bool("ssl_accept_invalid_certs");
        let accept_invalid_hostnames = config.get_param_bool("ssl_accept_invalid_hostnames");
        let root_store = Self::build_root_cert_store(config)?;
        let base_verifier: Arc<dyn ServerCertVerifier> =
            rustls::client::WebPkiServerVerifier::builder(root_store.into())
                .build()
                .map_err(|error| {
                    DbError::connection(format!(
                        "failed to build PostgreSQL certificate verifier: {}",
                        error
                    ))
                })?;
        let verifier: Arc<dyn ServerCertVerifier> =
            if accept_invalid_certs || accept_invalid_hostnames {
                Arc::new(PostgresServerCertVerifier::new(
                    base_verifier,
                    accept_invalid_certs,
                    accept_invalid_hostnames,
                ))
            } else {
                base_verifier
            };

        let client_config = RustlsClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();

        Ok(MakeRustlsConnect::new(client_config))
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
        let target = resolve_connection_target(config).await?;
        self.tunnel = target.tunnel;

        let mut pg_config = Config::new();
        pg_config
            .host(&target.host)
            .port(target.port)
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

        let ssl_mode = Self::ssl_mode(config);
        pg_config.ssl_mode(ssl_mode);

        // Connect to PostgreSQL
        debug!("[PostgreSQL] Establishing connection...");
        let client = match ssl_mode {
            SslMode::Disable => {
                let (client, connection) = pg_config.connect(NoTls).await.map_err(|e| {
                    error!("[PostgreSQL] Connection failed: {}", e);
                    DbError::connection_with_source("failed to connect", e)
                })?;
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        error!("[PostgreSQL] Connection error: {}", e);
                    }
                });
                client
            }
            _ => {
                let tls_connector = Self::build_tls_connector(config)?;
                let (client, connection) = pg_config.connect(tls_connector).await.map_err(|e| {
                    error!("[PostgreSQL] Connection failed: {}", e);
                    DbError::connection_with_source("failed to connect", e)
                })?;
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        error!("[PostgreSQL] Connection error: {}", e);
                    }
                });
                client
            }
        };

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
        self.tunnel = None;
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
                                debug!(
                                    "[PostgreSQL] Stopping execution due to error (stop_on_error=true)"
                                );
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
                                        debug!(
                                            "[PostgreSQL] Stopping execution due to error (stop_on_error=true)"
                                        );
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
                                        debug!(
                                            "[PostgreSQL] Stopping execution due to error (stop_on_error=true)"
                                        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use one_core::storage::DatabaseType;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[derive(Debug)]
    struct DummyVerifier;

    impl ServerCertVerifier for DummyVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, RustlsError> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, RustlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, RustlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![SignatureScheme::ECDSA_NISTP256_SHA256]
        }
    }

    fn build_config(extra_params: &[(&str, &str)]) -> DbConnectionConfig {
        DbConnectionConfig {
            id: String::new(),
            database_type: DatabaseType::PostgreSQL,
            name: "postgres".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
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
    fn ssl_mode_defaults_to_prefer() {
        let config = build_config(&[]);

        assert_eq!(PostgresDbConnection::ssl_mode(&config), SslMode::Prefer);
    }

    #[test]
    fn ssl_mode_honors_disable_and_require() {
        let disable = build_config(&[("ssl_mode", "disable")]);
        let require = build_config(&[("ssl_mode", "require")]);

        assert_eq!(PostgresDbConnection::ssl_mode(&disable), SslMode::Disable);
        assert_eq!(PostgresDbConnection::ssl_mode(&require), SslMode::Require);
    }

    #[test]
    fn ssl_hostname_override_only_ignores_name_errors() {
        let verifier = PostgresServerCertVerifier::new(Arc::new(DummyVerifier), false, true);

        assert!(verifier.should_ignore_certificate_error(&CertificateError::NotValidForName));
        assert!(!verifier.should_ignore_certificate_error(&CertificateError::UnknownIssuer));
    }

    #[test]
    fn ssl_invalid_certs_keep_hostname_validation_by_default() {
        let verifier = PostgresServerCertVerifier::new(Arc::new(DummyVerifier), true, false);

        assert!(verifier.should_ignore_certificate_error(&CertificateError::UnknownIssuer));
        assert!(!verifier.should_ignore_certificate_error(&CertificateError::NotValidForName));
    }

    #[test]
    fn ssl_load_root_certificates_rejects_empty_pem_file() {
        let mut temp_file = NamedTempFile::new().expect("应创建临时文件");
        writeln!(temp_file, "not a certificate").expect("应写入测试内容");

        let error = PostgresDbConnection::load_root_certificates(temp_file.path())
            .expect_err("空 PEM 文件应返回错误");

        assert!(
            error
                .to_string()
                .contains("does not contain any certificates"),
            "错误信息应指出证书文件为空: {}",
            error
        );
    }
}
