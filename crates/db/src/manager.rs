use crate::cache::CacheContext;
use crate::cache_manager::GlobalNodeCache;
use crate::clickhouse::ClickHousePlugin;
use crate::connection::{DbConnection, DbError, StreamingProgress};
use crate::duckdb::DuckDbPlugin;
use crate::import_export::{
    ExportConfig, ExportProgressSender, ExportResult, ImportConfig, ImportResult,
};
use crate::mssql::MsSqlPlugin;
use crate::mysql::MySqlPlugin;
use crate::oracle::OraclePlugin;
use crate::plugin::DatabasePlugin;
use crate::postgresql::PostgresPlugin;
use crate::sqlite::SqlitePlugin;
use crate::{
    DbNode, DbNodeType, ExecOptions, SqlErrorInfo, SqlResult, SqlSource, TableSaveResponse,
};
use dashmap::DashMap;
use gpui::{AppContext, AsyncApp, Global};
use one_core::connection_notifier::{ConnectionDataEvent, GlobalConnectionNotifier};
use one_core::gpui_tokio::Tokio;
use one_core::storage::{DatabaseType, DbConnectionConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Macro to reduce boilerplate for plugin operations with session management
macro_rules! with_plugin_session {
    ($self:expr, $cx:expr, $connection_id:expr, |$plugin:ident, $conn:ident| $body:expr) => {{
        let config = $self.get_config(&$connection_id);
        if config.is_none() {
            error!(
                "with_plugin_session: Connection not found: {}",
                $connection_id
            );
        }
        let config =
            config.ok_or_else(|| anyhow::anyhow!("Connection not found: {}", $connection_id))?;

        let clone_self = $self.clone();
        Tokio::spawn_result($cx, async move {
            let $plugin = clone_self.get_plugin(&config.database_type)?;
            info!(
                "with_plugin_session: creating session for config_id={}",
                config.id
            );
            let session_id = clone_self
                .connection_manager
                .create_session(config.clone(), &clone_self.db_manager)
                .await?;
            info!("with_plugin_session: session created: {}", session_id);

            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let $conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
                $body.map_err(|e| anyhow::anyhow!("{}", e))
            };

            clone_self
                .connection_manager
                .release_session(&session_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            result
        })
        .await
    }};
}

/// Macro with database parameter for PostgreSQL and other databases that require connection-level database selection
macro_rules! with_plugin_session_db {
    ($self:expr, $cx:expr, $connection_id:expr, $database:expr, |$plugin:ident, $conn:ident| $body:expr) => {{
        let config = $self.get_config(&$connection_id);
        if config.is_none() {
            error!(
                "with_plugin_session_db: Connection not found: {}",
                $connection_id
            );
        }
        let mut config = config
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", $connection_id))?
            .clone();
        config.database = Some($database.to_string());

        let clone_self = $self.clone();
        Tokio::spawn_result($cx, async move {
            let $plugin = clone_self.get_plugin(&config.database_type)?;
            let session_id = clone_self
                .connection_manager
                .create_session(config, &clone_self.db_manager)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                let $conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
                $body.map_err(|e| anyhow::anyhow!("{}", e))
            };

            clone_self
                .connection_manager
                .release_session(&session_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            result
        })
        .await
    }};
}

/// Database manager - creates database plugins
pub struct DbManager {
    mysql: Arc<dyn DatabasePlugin>,
    postgresql: Arc<dyn DatabasePlugin>,
    sqlite: Arc<dyn DatabasePlugin>,
    duckdb: Arc<dyn DatabasePlugin>,
    clickhouse: Arc<dyn DatabasePlugin>,
    mssql: Arc<dyn DatabasePlugin>,
    oracle: Arc<dyn DatabasePlugin>,
}

impl DbManager {
    pub fn new() -> Self {
        Self {
            mysql: Arc::new(MySqlPlugin::new()),
            postgresql: Arc::new(PostgresPlugin::new()),
            sqlite: Arc::new(SqlitePlugin::new()),
            duckdb: Arc::new(DuckDbPlugin::new()),
            clickhouse: Arc::new(ClickHousePlugin::new()),
            mssql: Arc::new(MsSqlPlugin::new()),
            oracle: Arc::new(OraclePlugin::new()),
        }
    }

    pub fn get_plugin(&self, db_type: &DatabaseType) -> Result<Arc<dyn DatabasePlugin>, DbError> {
        match db_type {
            DatabaseType::MySQL => Ok(Arc::clone(&self.mysql)),
            DatabaseType::PostgreSQL => Ok(Arc::clone(&self.postgresql)),
            DatabaseType::SQLite => Ok(Arc::clone(&self.sqlite)),
            DatabaseType::DuckDB => Ok(Arc::clone(&self.duckdb)),
            DatabaseType::ClickHouse => Ok(Arc::clone(&self.clickhouse)),
            DatabaseType::MSSQL => Ok(Arc::clone(&self.mssql)),
            DatabaseType::Oracle => Ok(Arc::clone(&self.oracle)),
        }
    }
}

impl Default for DbManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DbManager {
    fn clone(&self) -> Self {
        Self {
            mysql: Arc::clone(&self.mysql),
            postgresql: Arc::clone(&self.postgresql),
            sqlite: Arc::clone(&self.sqlite),
            duckdb: Arc::clone(&self.duckdb),
            clickhouse: Arc::clone(&self.clickhouse),
            mssql: Arc::clone(&self.mssql),
            oracle: Arc::clone(&self.oracle),
        }
    }
}

/// Connection session - represents a single database connection
struct ConnectionSession {
    connection: Box<dyn DbConnection + Send + Sync>,
    last_active: Instant,
    created_at: Instant,
    session_id: String,
    in_use: bool,
}

impl ConnectionSession {
    fn new(connection: Box<dyn DbConnection + Send + Sync>, session_id: String) -> Self {
        let now = Instant::now();
        Self {
            connection,
            last_active: now,
            created_at: now,
            session_id,
            in_use: false,
        }
    }

    fn mark_in_use(&mut self) {
        self.in_use = true;
        self.update_last_active();
    }

    fn release(&mut self) {
        self.in_use = false;
        self.update_last_active();
    }

    fn update_last_active(&mut self) {
        self.last_active = Instant::now();
    }

    fn is_expired(&self, timeout: Duration) -> bool {
        if self.in_use {
            return false;
        }
        self.last_active.elapsed() > timeout
    }

    fn is_lifetime_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }

    /// Check if current database matches config database
    /// Returns Ok(true) if consistent, Ok(false) if updated config, Err if check failed
    async fn verify_and_sync_database(&mut self) -> Result<bool, DbError> {
        // Skip check for databases that don't support switching
        if !self.connection.supports_database_switch() {
            return Ok(true);
        }

        let config_db = self.connection.config().database.clone();
        let current_db = self.connection.current_database().await?;

        if config_db == current_db {
            Ok(true)
        } else {
            // Database changed, update config
            self.connection.set_config_database(current_db.clone());
            info!(
                "Session {} database changed: {:?} -> {:?}",
                self.session_id, config_db, current_db
            );
            Ok(false)
        }
    }

    async fn close(&mut self) {
        if let Err(e) = self.connection.disconnect().await {
            error!("Failed to disconnect session {}: {}", self.session_id, e);
        } else {
            info!("Closed session: {}", self.session_id);
        }
    }
}

/// Connection manager - manages database connections for a client application
pub struct ConnectionManager {
    /// config_id -> list of sessions for that config
    sessions: Arc<RwLock<HashMap<String, Vec<ConnectionSession>>>>,
    /// Connection idle timeout (default: 5 minutes)
    idle_timeout: Duration,
    /// Maximum connection lifetime (default: 30 minutes)
    max_lifetime: Duration,
    /// Session counter for generating unique IDs
    session_counter: Arc<tokio::sync::Mutex<u64>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            session_counter: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }

    pub fn with_config(idle_timeout: Duration, max_lifetime: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            idle_timeout,
            max_lifetime,
            session_counter: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }

    /// Generate unique session ID
    async fn generate_session_id(&self, config_id: &str) -> String {
        let mut counter = self.session_counter.lock().await;
        *counter += 1;
        format!("{}:session:{}", config_id, *counter)
    }

    /// Create a new connection session
    pub async fn create_session(
        &self,
        config: DbConnectionConfig,
        db_manager: &DbManager,
    ) -> Result<String, DbError> {
        let session_started = Instant::now();
        let config_id = config.id.clone();
        let database_type = config.database_type;
        let database = config.database.clone();

        // Try to acquire an existing session and switch database if needed
        if let Some(session_id) = self.try_acquire_session(&config).await? {
            info!(
                "[DB][Timing] create_session reused config_id={} database_type={:?} database={:?} session_id={} elapsed={}ms",
                config_id,
                database_type,
                database,
                session_id,
                session_started.elapsed().as_millis()
            );
            return Ok(session_id);
        }

        let session_id = self.generate_session_id(&config_id).await;

        // Create new connection
        let plugin = db_manager.get_plugin(&config.database_type)?;
        let connect_started = Instant::now();
        let connection = plugin.create_connection(config.clone()).await?;
        info!(
            "[DB][Timing] create_session connect config_id={} database_type={:?} database={:?} session_id={} elapsed={}ms",
            config_id,
            database_type,
            database,
            session_id,
            connect_started.elapsed().as_millis()
        );
        info!(
            "Created new session: {} (database: {:?})",
            session_id, config.database
        );

        // Store session
        let mut session = ConnectionSession::new(connection, session_id.clone());
        session.mark_in_use();

        let mut sessions = self.sessions.write().await;
        sessions
            .entry(config_id)
            .or_insert_with(Vec::new)
            .push(session);

        info!(
            "[DB][Timing] create_session total database_type={:?} database={:?} session_id={} elapsed={}ms",
            database_type,
            database,
            session_id,
            session_started.elapsed().as_millis()
        );
        Ok(session_id)
    }

    /// Get mutable access to a session's connection
    /// Returns the connection wrapped in the write guard to maintain lock
    pub async fn get_session_connection(
        &self,
        session_id: &str,
    ) -> Result<SessionConnectionGuard<'_>, DbError> {
        let sessions = self.sessions.write().await;

        // Check if session exists
        let exists = sessions
            .values()
            .any(|list| list.iter().any(|s| s.session_id == session_id));

        if !exists {
            return Err(DbError::Internal(format!(
                "session not found: {}",
                session_id
            )));
        }

        Ok(SessionConnectionGuard {
            sessions,
            session_id: session_id.to_string(),
        })
    }

    fn db_equals(db1: &DbConnectionConfig, db2: &DbConnectionConfig) -> bool {
        match db1.database_type {
            DatabaseType::Oracle => {
                (db1.sid.is_some() && db1.sid == db2.sid)
                    || (db1.service_name.is_some() && db1.service_name == db2.service_name)
            }
            _ => db1.database.is_some() && db1.database == db2.database,
        }
    }

    /// Try to acquire an existing idle session with matching database
    async fn try_acquire_session(
        &self,
        config: &DbConnectionConfig,
    ) -> Result<Option<String>, DbError> {
        let mut sessions = self.sessions.write().await;

        if let Some(session_list) = sessions.get_mut(&config.id) {
            // Find an idle session with matching database
            if let Some(session) = session_list
                .iter_mut()
                .find(|s| !s.in_use && Self::db_equals(s.connection.config(), config))
            {
                session.mark_in_use();

                info!(
                    "Reusing session: {} (database: {:?})",
                    session.session_id, config.database
                );
                return Ok(Some(session.session_id.clone()));
            }
        }

        Ok(None)
    }
}

/// Guard that holds the write lock and provides access to a session's connection
pub struct SessionConnectionGuard<'a> {
    sessions: tokio::sync::RwLockWriteGuard<'a, HashMap<String, Vec<ConnectionSession>>>,
    session_id: String,
}

impl<'a> SessionConnectionGuard<'a> {
    /// Get mutable reference to the connection and update last active time
    pub fn connection(&mut self) -> Option<&mut (dyn DbConnection + Send + Sync)> {
        for session_list in self.sessions.values_mut() {
            if let Some(session) = session_list
                .iter_mut()
                .find(|s| s.session_id == self.session_id)
            {
                session.mark_in_use();
                return Some(&mut *session.connection);
            }
        }
        None
    }
}

impl ConnectionManager {
    /// Get session config
    pub async fn get_session_config(&self, session_id: &str) -> Option<DbConnectionConfig> {
        let sessions = self.sessions.read().await;

        for session_list in sessions.values() {
            if let Some(session) = session_list.iter().find(|s| s.session_id == session_id) {
                return Some(session.connection.config().clone());
            }
        }

        None
    }

    pub async fn release_session(&self, session_id: &str) -> Result<(), DbError> {
        let mut sessions = self.sessions.write().await;

        // First, find and verify the session
        let mut should_close = false;
        let mut found_config_id: Option<String> = None;

        for (config_id, session_list) in sessions.iter_mut() {
            if let Some(session) = session_list.iter_mut().find(|s| s.session_id == session_id) {
                // Verify database consistency before release
                match session.verify_and_sync_database().await {
                    Ok(_) => {
                        // Check passed (consistent or updated), release normally
                        session.release();
                        info!("Session {} released", session_id);
                        return Ok(());
                    }
                    Err(e) => {
                        // Check failed, mark for closing
                        warn!(
                            "Session {} database check failed: {}, closing connection",
                            session_id, e
                        );
                        should_close = true;
                        found_config_id = Some(config_id.clone());
                        break;
                    }
                }
            }
        }

        // If check failed, close and remove the session
        if should_close {
            if let Some(config_id) = found_config_id {
                if let Some(session_list) = sessions.get_mut(&config_id) {
                    if let Some(pos) = session_list.iter().position(|s| s.session_id == session_id)
                    {
                        let mut session = session_list.remove(pos);
                        session.close().await;

                        // Remove empty config entry
                        if session_list.is_empty() {
                            sessions.remove(&config_id);
                        }
                        return Ok(());
                    }
                }
            }
        }

        Err(DbError::Internal(format!(
            "session not found: {}",
            session_id
        )))
    }

    /// Close a specific session
    pub async fn close_session(&self, session_id: &str) -> Result<(), DbError> {
        let mut sessions = self.sessions.write().await;

        let mut found_config_id: Option<String> = None;
        let mut removed_session: Option<ConnectionSession> = None;

        for (config_id, session_list) in sessions.iter_mut() {
            if let Some(pos) = session_list.iter().position(|s| s.session_id == session_id) {
                removed_session = Some(session_list.remove(pos));
                if session_list.is_empty() {
                    found_config_id = Some(config_id.clone());
                }
                break;
            }
        }

        // Remove empty config entry after iteration
        if let Some(config_id) = found_config_id {
            sessions.remove(&config_id);
        }

        // Close session after releasing iteration
        if let Some(mut session) = removed_session {
            session.release();
            session.close().await;
            return Ok(());
        }

        Err(DbError::Internal(format!(
            "session not found: {}",
            session_id
        )))
    }

    /// Remove all sessions for a connection config
    pub async fn remove_all_sessions(&self, config_id: &str) {
        let mut sessions = self.sessions.write().await;

        if let Some(mut session_list) = sessions.remove(config_id) {
            info!(
                "Closing {} sessions for config: {}",
                session_list.len(),
                config_id
            );

            for session in session_list.iter_mut() {
                session.close().await;
            }
        }
    }

    /// Clean up expired sessions
    async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let idle_timeout = self.idle_timeout;
        let max_lifetime = self.max_lifetime;

        for (config_id, session_list) in sessions.iter_mut() {
            let mut i = 0;
            while i < session_list.len() {
                let should_remove = session_list[i].is_expired(idle_timeout)
                    || session_list[i].is_lifetime_expired(max_lifetime);

                if should_remove {
                    let mut session = session_list.remove(i);
                    warn!(
                        "Closing expired session {} for config {} (in_use: {}, idle: {}s, lifetime: {}s)",
                        session.session_id,
                        config_id,
                        session.in_use,
                        session.last_active.elapsed().as_secs(),
                        session.created_at.elapsed().as_secs()
                    );
                    session.close().await;
                } else {
                    i += 1;
                }
            }
        }

        // Remove empty config entries
        sessions.retain(|_, list| !list.is_empty());
    }

    /// Get connection statistics
    pub async fn stats(&self) -> ConnectionStats {
        let sessions = self.sessions.read().await;
        let mut total = 0;
        let mut in_use_count = 0;

        for session_list in sessions.values() {
            total += session_list.len();
            in_use_count += session_list.iter().filter(|s| s.in_use).count();
        }

        ConnectionStats {
            total_sessions: total,
            active_sessions: in_use_count,
            configs_with_sessions: sessions.len(),
        }
    }

    /// List all sessions for a config
    pub async fn list_sessions(&self, config_id: &str) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;

        sessions
            .get(config_id)
            .map(|list| {
                list.iter()
                    .map(|s| SessionInfo {
                        session_id: s.session_id.clone(),
                        database: s.connection.config().database.clone(),
                        in_use: s.in_use,
                        idle_time: s.last_active.elapsed(),
                        lifetime: s.created_at.elapsed(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConnectionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            idle_timeout: self.idle_timeout,
            max_lifetime: self.max_lifetime,
            session_counter: Arc::clone(&self.session_counter),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub configs_with_sessions: usize,
}

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub database: Option<String>,
    pub in_use: bool,
    pub idle_time: Duration,
    pub lifetime: Duration,
}

/// Connection pool compatibility layer
#[derive(Clone)]
pub struct ConnectionPool {
    db_manager: DbManager,
}

impl ConnectionPool {
    pub fn new(db_manager: DbManager) -> Self {
        Self { db_manager }
    }

    pub async fn get_connection(
        &self,
        config: DbConnectionConfig,
        _db_manager: &DbManager,
    ) -> anyhow::Result<Arc<RwLock<Box<dyn DbConnection + Send + Sync>>>> {
        let plugin = self.db_manager.get_plugin(&config.database_type)?;
        let connection = plugin.create_connection(config).await?;
        Ok(Arc::new(RwLock::new(connection)))
    }
}

/// Global database state - stores DbManager and ConnectionManager
#[derive(Clone)]
pub struct GlobalDbState {
    pub db_manager: DbManager,
    pub connection_manager: ConnectionManager,
    pub connection_pool: ConnectionPool,
    /// connection_id -> config mapping
    connections: Arc<DashMap<String, DbConnectionConfig>>,
}

impl GlobalDbState {
    pub fn new() -> Self {
        let manager = ConnectionManager::new();
        let db_manager = DbManager::new();

        Self {
            db_manager: db_manager.clone(),
            connection_manager: manager,
            connection_pool: ConnectionPool::new(db_manager),
            connections: Arc::new(DashMap::new()),
        }
    }

    /// Start the cleanup task (should be called after Tokio runtime is available)
    pub fn start_cleanup_task<C>(&self, cx: &mut C)
    where
        C: AppContext,
    {
        let manager = Arc::new(self.connection_manager.clone());
        let _ = Tokio::spawn(cx, async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                manager.cleanup_expired_sessions().await;
            }
        });
    }

    /// Internal method for get_config
    pub fn get_config(&self, connection_id: &str) -> Option<DbConnectionConfig> {
        let config_ref = self.connections.get(connection_id);
        if let Some(config) = config_ref {
            return Some(config.value().clone());
        }
        None
    }

    pub fn get_plugin(
        &self,
        database_type: &DatabaseType,
    ) -> Result<Arc<dyn DatabasePlugin>, DbError> {
        self.db_manager.get_plugin(database_type)
    }

    fn wrapper_result(result: Vec<SqlResult>) -> anyhow::Result<SqlResult> {
        match result.into_iter().next() {
            Some(re) => Ok(re),
            None => Err(anyhow::anyhow!("No result returned")),
        }
    }

    pub async fn drop_database(
        &self,
        cx: &mut AsyncApp,
        config_id: String,
        database_name: String,
    ) -> anyhow::Result<SqlResult> {
        let config = self
            .get_config(&config_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", config_id))?;
        let plugin = self.get_plugin(&config.database_type)?;
        let sql = plugin.drop_database(&database_name);

        let result = self.execute_with_session(cx, config, sql, None).await?;

        Self::wrapper_result(result)
    }

    /// Drop table
    pub async fn drop_table(
        &self,
        cx: &mut AsyncApp,
        config_id: String,
        database: String,
        schema: Option<String>,
        table_name: String,
    ) -> anyhow::Result<SqlResult> {
        let mut config = self
            .get_config(&config_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", config_id))?;
        let plugin = self.get_plugin(&config.database_type)?;
        let sql = plugin.drop_table(&database, schema.as_deref(), &table_name);

        // For non-Oracle databases, modify config.database to switch database
        if config.database_type != DatabaseType::Oracle {
            config.database = Some(database);
        }

        // Pass schema to switch before executing
        let result = self
            .execute_with_session_internal(cx, config, sql, None, schema)
            .await?;

        Self::wrapper_result(result)
    }

    /// Truncate table
    pub async fn truncate_table(
        &self,
        cx: &mut AsyncApp,
        config_id: String,
        database: String,
        table_name: String,
    ) -> anyhow::Result<SqlResult> {
        let config = self
            .get_config(&config_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", config_id))?;
        let plugin = self.get_plugin(&config.database_type)?;
        let sql = plugin.truncate_table(&database, &table_name);

        let result = self.execute_with_session(cx, config, sql, None).await?;

        Self::wrapper_result(result)
    }

    /// Rename table
    pub async fn rename_table(
        &self,
        cx: &mut AsyncApp,
        config_id: String,
        database: String,
        old_name: String,
        new_name: String,
    ) -> anyhow::Result<SqlResult> {
        let config = self
            .get_config(&config_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", config_id))?;
        let plugin = self.get_plugin(&config.database_type)?;
        let sql = plugin.rename_table(&database, &old_name, &new_name);

        let result = self.execute_with_session(cx, config, sql, None).await?;

        Self::wrapper_result(result)
    }

    /// Drop view
    pub async fn drop_view(
        &self,
        cx: &mut AsyncApp,
        config_id: String,
        database: String,
        view_name: String,
    ) -> anyhow::Result<SqlResult> {
        let config = self
            .get_config(&config_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", config_id))?;
        let plugin = self.get_plugin(&config.database_type)?;
        let sql = plugin.drop_view(&database, &view_name);

        let result = self.execute_with_session(cx, config, sql, None).await?;

        Self::wrapper_result(result)
    }

    /// Register a connection configuration
    pub fn register_connection(&mut self, config: DbConnectionConfig) {
        self.connections.insert(config.id.clone(), config);
    }

    pub async fn update_connection(
        &mut self,
        cx: &mut AsyncApp,
        config: DbConnectionConfig,
    ) -> anyhow::Result<()> {
        self.unregister_connection(cx, config.id.clone()).await?;
        self.register_connection(config);
        Ok(())
    }

    /// Unregister a connection configuration
    pub async fn unregister_connection(
        &mut self,
        cx: &mut AsyncApp,
        connection_id: String,
    ) -> anyhow::Result<()> {
        self.connections.remove(&connection_id);
        let clone_self = self.clone();
        // Remove from registry
        Tokio::spawn_result(cx, async move {
            // Close all sessions for this connection
            clone_self
                .connection_manager
                .remove_all_sessions(&connection_id)
                .await;
            Ok(())
        })
        .await
    }

    /// Create a new session for executing queries
    pub async fn create_session(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: Option<String>,
    ) -> anyhow::Result<String> {
        let clone_self = self.clone();
        let mut config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        // Override database if specified
        if let Some(db) = database {
            config.database = Some(db);
        }
        Tokio::spawn_result(cx, async move {
            clone_self
                .connection_manager
                .create_session(config, &clone_self.db_manager)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
        .await
    }

    /// Execute SQL  (simplified - creates session per execution)
    pub async fn execute_single(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        script: String,
        database: Option<String>,
        opts: Option<ExecOptions>,
    ) -> anyhow::Result<SqlResult> {
        let result = self
            .execute_script(cx, connection_id, script, database, None, opts)
            .await?;
        Self::wrapper_result(result)
    }

    /// Execute SQL script (simplified - creates session per execution)
    pub async fn execute_script(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        script: String,
        database: Option<String>,
        schema: Option<String>,
        opts: Option<ExecOptions>,
    ) -> anyhow::Result<Vec<SqlResult>> {
        //  Get config
        let mut config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        // Schema to switch before executing
        let schema_to_switch = schema;

        // For non-Oracle databases, modify config.database to switch database
        if config.database_type != DatabaseType::Oracle {
            if let Some(db) = database {
                config.database = Some(db);
            }
        }

        self.execute_with_session_internal(cx, config, script, opts, schema_to_switch)
            .await
    }

    /// Execute script with existing session (for transaction scenarios)
    pub async fn execute_with_session(
        &self,
        cx: &mut AsyncApp,
        config: DbConnectionConfig,
        script: String,
        opts: Option<ExecOptions>,
    ) -> anyhow::Result<Vec<SqlResult>> {
        self.execute_with_session_internal(cx, config, script, opts, None)
            .await
    }

    async fn execute_with_session_internal(
        &self,
        cx: &mut AsyncApp,
        config: DbConnectionConfig,
        script: String,
        opts: Option<ExecOptions>,
        schema_to_switch: Option<String>,
    ) -> anyhow::Result<Vec<SqlResult>> {
        // Access the cache used for DDL invalidation.
        let cache = cx.update(|cx| cx.try_global::<GlobalNodeCache>().cloned());

        let cache_ctx = cx.update(|cx| {
            cx.try_global::<GlobalDbState>()
                .and_then(|state| state.get_config(&config.id))
                .map(|cfg| CacheContext::from_config(&cfg))
        });

        let notifier = cx.update(|cx| cx.try_global::<GlobalConnectionNotifier>().cloned());

        let clone_self = self.clone();
        let config_id = config.id.clone();
        let current_database = config.database.clone().unwrap_or_default();
        let current_schema = schema_to_switch.clone();
        let script_for_ddl = script.clone();

        let result = Tokio::spawn_result(cx, async move {
            // Create session
            let session_id = clone_self
                .connection_manager
                .create_session(config.clone(), &clone_self.db_manager)
                .await?;

            // Execute query on session
            let opts = opts.unwrap_or_default();
            let is_transactional = opts.transactional;

            let plugin = clone_self.get_plugin(&config.database_type)?;

            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;

                // Switch schema before executing
                if let Some(schema) = &schema_to_switch {
                    conn.switch_schema(schema)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to switch schema: {}", e))?;
                }

                conn.execute(plugin.as_ref(), &script, opts).await?
            };

            // Determine if session should stay open based on script content
            let upper_script = script.to_uppercase();
            let has_begin =
                upper_script.contains("BEGIN") || upper_script.contains("START TRANSACTION");
            let has_commit = upper_script.contains("COMMIT");
            let has_rollback = upper_script.contains("ROLLBACK");

            // Keep session open if: in transactional mode, or has BEGIN without COMMIT/ROLLBACK
            let keep_session = is_transactional || (has_begin && !has_commit && !has_rollback);

            if keep_session {
                // Release but don't close - session can be reused later
                clone_self
                    .connection_manager
                    .release_session(&session_id)
                    .await?;
            } else {
                // Close session completely
                clone_self
                    .connection_manager
                    .close_session(&session_id)
                    .await?;
            }

            Ok(result)
        })
        .await?;

        // Process DDL cache invalidation after successful execution.
        if let Some(cache) = cache {
            let ddl_info = Tokio::spawn_result(cx, async move {
                Ok(cache
                    .process_sql_for_invalidation(
                        &config_id,
                        &script_for_ddl,
                        &current_database,
                        current_schema.as_deref(),
                        cache_ctx.as_ref(),
                    )
                    .await)
            })
            .await;

            // Emit a SchemaChanged event when DDL changes are detected.
            if let Ok(Some((conn_id, database, schema))) = ddl_info {
                if let Some(notifier) = notifier {
                    cx.update(|cx| {
                        notifier.0.update(cx, |_, cx| {
                            cx.emit(ConnectionDataEvent::SchemaChanged {
                                connection_id: conn_id,
                                database,
                                schema,
                            });
                        });
                    });
                }
            }
        }

        Ok(result)
    }

    /// Execute SQL with streaming progress (supports both script string and file)
    /// Returns a receiver that will receive progress updates for each statement
    /// For file source, the file is read incrementally to avoid loading the entire file into memory
    pub fn execute_streaming(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        source: SqlSource,
        database: Option<String>,
        schema: Option<String>,
        opts: Option<ExecOptions>,
    ) -> anyhow::Result<mpsc::Receiver<StreamingProgress>> {
        let (tx, rx) = mpsc::channel::<StreamingProgress>(100);
        let mut config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        let schema_to_switch = schema;

        if config.database_type != DatabaseType::Oracle {
            if let Some(db) = database {
                config.database = Some(db);
            }
        }

        let mut opts = opts.unwrap_or_default();
        if source.is_file() {
            opts.streaming = true;
        }

        let clone_self = self.clone();
        Tokio::spawn(cx, async move {
            let plugin = match clone_self.get_plugin(&config.database_type) {
                Ok(c) => c,
                Err(_) => return,
            };

            let session_result = clone_self
                .connection_manager
                .create_session(config.clone(), &clone_self.db_manager)
                .await;

            let session_id = match session_result {
                Ok(id) => id,
                Err(e) => {
                    let total_size = source.file_size().unwrap_or(0);
                    let progress = StreamingProgress::with_file_progress(
                        0,
                        SqlResult::Error(SqlErrorInfo {
                            sql: String::new(),
                            message: format!("Failed to create session: {}", e),
                        }),
                        0,
                        total_size,
                    );
                    let _ = tx.send(progress).await;
                    return;
                }
            };

            let exec_result = async {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;

                if let Some(schema) = &schema_to_switch {
                    conn.switch_schema(schema)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to switch schema: {}", e))?;
                }

                conn.execute_streaming(plugin.as_ref(), source, opts, tx)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                Ok::<_, anyhow::Error>(())
            }
            .await;

            let _ = clone_self
                .connection_manager
                .close_session(&session_id)
                .await;

            if let Err(e) = exec_result {
                error!("Streaming execution error: {}", e);
            }
        })
        .detach();

        Ok(rx)
    }

    pub async fn with_session_connection<R, F>(
        &self,
        cx: &mut AsyncApp,
        config: DbConnectionConfig,
        f: F,
    ) -> anyhow::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&dyn DatabasePlugin, &mut (dyn DbConnection + Send + Sync)) -> anyhow::Result<R>
            + Send
            + 'static,
    {
        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            let plugin = clone_self.get_plugin(&config.database_type)?;
            let session_id = clone_self
                .connection_manager
                .create_session(config.clone(), &clone_self.db_manager)
                .await?;

            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
                f(&*plugin, conn)
            };

            clone_self
                .connection_manager
                .close_session(&session_id)
                .await?;

            result
        })
        .await
    }

    /// Get connection statistics
    pub async fn stats(&self, cx: &mut AsyncApp) -> anyhow::Result<ConnectionStats> {
        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            Ok(clone_self.connection_manager.stats().await)
        })
        .await
    }

    /// List all sessions for a connection
    pub async fn list_sessions(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
    ) -> anyhow::Result<Vec<SessionInfo>> {
        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            Ok(clone_self
                .connection_manager
                .list_sessions(&connection_id)
                .await)
        })
        .await
    }

    /// Close a specific session
    pub async fn close_session(&self, cx: &mut AsyncApp, session_id: String) -> anyhow::Result<()> {
        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            clone_self
                .connection_manager
                .close_session(&session_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
        .await
    }

    /// Disconnect all sessions for a connection
    pub async fn disconnect_all(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
    ) -> anyhow::Result<()> {
        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            clone_self
                .connection_manager
                .remove_all_sessions(&connection_id)
                .await;
            Ok(())
        })
        .await
    }

    /// Query table data
    pub async fn query_table_data(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        request: crate::types::TableDataRequest,
    ) -> anyhow::Result<crate::types::TableDataResponse> {
        info!("query_table_data: connection_id={}", connection_id);
        let database = request.database.clone();
        with_plugin_session_db!(self, cx, connection_id, database, |plugin, conn| {
            plugin.query_table_data(&*conn, request).await
        })
    }

    fn cached_children_ready(cached: &DbNode) -> bool {
        cached.children_loaded
    }

    /// Load node children for tree view
    pub async fn load_node_children(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        node: DbNode,
    ) -> anyhow::Result<Vec<DbNode>> {
        let load_started = Instant::now();
        // Resolve the connection config for the current node.
        let mut config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?
            .clone();

        // Build the cache context up front for cache lookup and write-back.
        let cache_ctx = crate::CacheContext::from_config(&config);

        // Access the global node cache if it is available.
        let cache = cx.update(|cx| cx.try_global::<crate::GlobalNodeCache>().cloned());

        // For Database and Schema nodes, we need to connect to the specific database
        // This is especially important for PostgreSQL which doesn't support database switching
        let target_database = node.get_database_name();

        if let Some(db) = target_database {
            config.database = Some(db);
        }

        let clone_self = self.clone();
        let node_clone = node.clone();
        let connection_id_for_ui = connection_id.clone();
        let node_for_ui = node.clone();

        let result = Tokio::spawn_result(cx, async move {
            let async_started = Instant::now();
            // Try cache first to avoid unnecessary session creation.
            if let Some(ref cache) = cache {
                if let Some(cached) = cache.get_node(&cache_ctx, &node_clone.id).await {
                    if Self::cached_children_ready(&cached) {
                        tracing::debug!("Cache hit for node: {}", node_clone.id);
                        info!(
                            "[DB][Timing] load_node_children cache_hit connection_id={} node_id={} node_type={:?} children={} elapsed={}ms",
                            connection_id,
                            node_clone.id,
                            node_clone.node_type,
                            cached.children.len(),
                            async_started.elapsed().as_millis()
                        );
                        return Ok(cached.children);
                    }
                }
            }

            // Cache miss. Load children from the database.
            tracing::debug!(
                "Cache miss for node: {}, loading from database",
                node_clone.id
            );

            let plugin = clone_self.get_plugin(&config.database_type)?;
            let session_started = Instant::now();
            let session_id = clone_self
                .connection_manager
                .create_session(config.clone(), &clone_self.db_manager)
                .await?;
            info!(
                "[DB][Timing] load_node_children create_session connection_id={} node_id={} node_type={:?} session_id={} elapsed={}ms",
                connection_id,
                node_clone.id,
                node_clone.node_type,
                session_id,
                session_started.elapsed().as_millis()
            );

            let fetch_started = Instant::now();
            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
                plugin
                    .load_node_children(&*conn, &node_clone)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            };
            info!(
                "[DB][Timing] load_node_children fetch connection_id={} node_id={} node_type={:?} session_id={} elapsed={}ms",
                connection_id,
                node_clone.id,
                node_clone.node_type,
                session_id,
                fetch_started.elapsed().as_millis()
            );

            let release_started = Instant::now();
            if let Err(e) = clone_self
                .connection_manager
                .release_session(&session_id)
                .await
            {
                warn!("Failed to release session {}: {}", session_id, e);
            } else {
                info!(
                    "[DB][Timing] load_node_children release_session connection_id={} node_id={} session_id={} elapsed={}ms",
                    connection_id,
                    node_clone.id,
                    session_id,
                    release_started.elapsed().as_millis()
                );
            }

            // Persist successful results back to the cache.
            if let Ok(ref children) = result {
                if let Some(ref cache) = cache {
                    let mut node_with_children = node_clone.clone();
                    node_with_children.children = children.clone();
                    node_with_children.children_loaded = true;

                    cache
                        .cache_node(&cache_ctx, &node_with_children.id, &node_with_children)
                        .await;
                    tracing::debug!(
                        "Cached node: {} with {} children",
                        node_with_children.id,
                        children.len()
                    );
                }
                info!(
                    "[DB][Timing] load_node_children total connection_id={} node_id={} node_type={:?} children={} elapsed={}ms",
                    connection_id,
                    node_clone.id,
                    node_clone.node_type,
                    children.len(),
                    async_started.elapsed().as_millis()
                );
            } else if let Err(ref error) = result {
                warn!(
                    "[DB][Timing] load_node_children failed connection_id={} node_id={} node_type={:?} elapsed={}ms error={}",
                    connection_id,
                    node_clone.id,
                    node_clone.node_type,
                    async_started.elapsed().as_millis(),
                    error
                );
            }

            result
        })
        .await;

        if let Ok(children) = &result {
            info!(
                "[DB][Timing] load_node_children ui_total connection_id={} node_id={} node_type={:?} children={} elapsed={}ms",
                connection_id_for_ui,
                node_for_ui.id,
                node_for_ui.node_type,
                children.len(),
                load_started.elapsed().as_millis()
            );
        }

        result
    }

    /// Apply table changes
    pub async fn apply_table_changes(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        request: crate::types::TableSaveRequest,
    ) -> anyhow::Result<TableSaveResponse> {
        let database = request.database.clone();
        with_plugin_session_db!(self, cx, connection_id, database, |plugin, conn| {
            let mut success_count = 0;
            let mut errors = Vec::new();

            for change in &request.changes {
                let Some(sql) = plugin.build_table_change_sql(&request, change) else {
                    continue;
                };

                match conn
                    .execute(plugin.as_ref(), &sql, ExecOptions::default())
                    .await
                {
                    Ok(results) => {
                        for result in results {
                            match result {
                                SqlResult::Exec(_) => {
                                    success_count += 1;
                                }
                                SqlResult::Error(err) => {
                                    errors.push(err.message);
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(e.to_string());
                    }
                }
            }

            anyhow::Ok(TableSaveResponse {
                success_count,
                errors,
            })
        })
    }

    /// List databases (with caching)
    pub async fn list_databases(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
    ) -> anyhow::Result<Vec<String>> {
        // Access the cache instance.
        let cache = cx.update(|cx| cx.try_global::<GlobalNodeCache>().cloned());

        // Try the cache first.
        if let Some(cache) = cache.clone() {
            let conn_id = connection_id.clone();
            let result = Tokio::spawn_result(cx, async move {
                if let Some(databases) = cache.get_databases(&conn_id).await {
                    tracing::debug!("Cache hit for databases: {}", conn_id);
                    return Ok(databases);
                }
                Err(anyhow::anyhow!("Cache miss"))
            })
            .await;

            if let Ok(databases) = result {
                return Ok(databases);
            }
        }

        // Cache miss. Query the database.
        let conn_id = connection_id.clone();
        let databases = with_plugin_session!(self, cx, connection_id, |plugin, conn| {
            plugin.list_databases(&*conn).await
        })?;

        // Persist the result in cache.
        if let Some(cache) = cache {
            let databases_clone = databases.clone();
            Tokio::spawn(cx, async move {
                cache.cache_databases(&conn_id, databases_clone).await;
                tracing::debug!("Cached databases for: {}", conn_id);
            })
            .detach();
        }

        Ok(databases)
    }

    /// List databases view
    pub async fn list_databases_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session!(self, cx, connection_id, |plugin, conn| {
            plugin.list_databases_view(&*conn).await
        })
    }

    /// Check if database type supports schemas
    pub fn supports_schema(&self, database_type: &DatabaseType) -> bool {
        self.db_manager
            .get_plugin(database_type)
            .map(|plugin| plugin.supports_schema())
            .unwrap_or(false)
    }

    /// Check if database type uses schemas as top-level nodes (like Oracle)
    pub fn uses_schema_as_database(&self, database_type: &DatabaseType) -> bool {
        self.db_manager
            .get_plugin(database_type)
            .map(|plugin| plugin.uses_schema_as_database())
            .unwrap_or(false)
    }

    /// List schemas in a database (with caching)
    pub async fn list_schemas(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
    ) -> anyhow::Result<Vec<String>> {
        // Access the cache instance.
        let cache = cx.update(|cx| cx.try_global::<GlobalNodeCache>().cloned());

        // Try the cache first.
        if let Some(cache) = cache.clone() {
            let conn_id = connection_id.clone();
            let db = database.clone();
            let result = Tokio::spawn_result(cx, async move {
                if let Some(schemas) = cache.get_schemas(&conn_id, &db).await {
                    tracing::debug!("Cache hit for schemas: {}:{}", conn_id, db);
                    return Ok(schemas);
                }
                Err(anyhow::anyhow!("Cache miss"))
            })
            .await;

            if let Ok(schemas) = result {
                return Ok(schemas);
            }
        }

        // Cache miss. Query the database.
        let conn_id = connection_id.clone();
        let db = database.clone();
        let schemas =
            with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
                plugin.list_schemas(&*conn, &database).await
            })?;

        // Persist the result in cache.
        if let Some(cache) = cache {
            let schemas_clone = schemas.clone();
            Tokio::spawn(cx, async move {
                cache.cache_schemas(&conn_id, &db, schemas_clone).await;
                tracing::debug!("Cached schemas for: {}:{}", conn_id, db);
            })
            .detach();
        }

        Ok(schemas)
    }

    /// List tables (with caching)
    pub async fn list_tables(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
        schema: Option<String>,
    ) -> anyhow::Result<Vec<crate::types::TableInfo>> {
        // Access the cache instance.
        let cache = cx.update(|cx| cx.try_global::<GlobalNodeCache>().cloned());

        // Try the cache first.
        if let Some(cache) = cache.clone() {
            let conn_id = connection_id.clone();
            let db = database.clone();
            let sch = schema.clone();
            let result = Tokio::spawn_result(cx, async move {
                if let Some(tables) = cache.get_tables(&conn_id, &db, sch.as_deref()).await {
                    tracing::debug!("Cache hit for tables: {}:{}:{:?}", conn_id, db, sch);
                    return Ok(tables);
                }
                Err(anyhow::anyhow!("Cache miss"))
            })
            .await;

            if let Ok(tables) = result {
                return Ok(tables);
            }
        }

        // Cache miss. Query the database.
        let conn_id = connection_id.clone();
        let db = database.clone();
        let sch = schema.clone();
        let tables =
            with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
                plugin.list_tables(&*conn, &database, schema).await
            })?;

        // Persist the result in cache.
        if let Some(cache) = cache {
            let tables_clone = tables.clone();
            Tokio::spawn(cx, async move {
                cache
                    .cache_tables(&conn_id, &db, sch.as_deref(), tables_clone)
                    .await;
                tracing::debug!("Cached tables for: {}:{}:{:?}", conn_id, db, sch);
            })
            .detach();
        }

        Ok(tables)
    }

    /// List tables view
    pub async fn list_tables_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
        schema: Option<String>,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin.list_tables_view(&*conn, &database, schema).await
        })
    }

    /// List columns (with caching)
    pub async fn list_columns(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
        schema: Option<String>,
        table: String,
    ) -> anyhow::Result<Vec<crate::types::ColumnInfo>> {
        // Access the cache instance.
        let cache = cx.update(|cx| cx.try_global::<GlobalNodeCache>().cloned());

        // Try the cache first.
        if let Some(cache) = cache.clone() {
            let conn_id = connection_id.clone();
            let db = database.clone();
            let sch = schema.clone();
            let tbl = table.clone();
            let result = Tokio::spawn_result(cx, async move {
                if let Some(columns) = cache.get_columns(&conn_id, &db, sch.as_deref(), &tbl).await
                {
                    tracing::debug!(
                        "Cache hit for columns: {}:{}:{:?}:{}",
                        conn_id,
                        db,
                        sch,
                        tbl
                    );
                    return Ok(columns);
                }
                Err(anyhow::anyhow!("Cache miss"))
            })
            .await;

            if let Ok(columns) = result {
                return Ok(columns);
            }
        }

        // Cache miss. Query the database.
        let conn_id = connection_id.clone();
        let db = database.clone();
        let sch = schema.clone();
        let tbl = table.clone();
        let columns =
            with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
                plugin.list_columns(&*conn, &database, schema, &table).await
            })?;

        // Persist the result in cache.
        if let Some(cache) = cache {
            let columns_clone = columns.clone();
            Tokio::spawn(cx, async move {
                cache
                    .cache_columns(&conn_id, &db, sch.as_deref(), &tbl, columns_clone)
                    .await;
                tracing::debug!("Cached columns for: {}:{}:{:?}:{}", conn_id, db, sch, tbl);
            })
            .detach();
        }

        Ok(columns)
    }

    /// List columns view
    pub async fn list_columns_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
        schema: Option<String>,
        table: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin
                .list_columns_view(&*conn, &database, schema, &table)
                .await
        })
    }

    /// List indexes (with caching)
    pub async fn list_indexes(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
        schema: Option<String>,
        table: String,
    ) -> anyhow::Result<Vec<crate::types::IndexInfo>> {
        // Access the cache instance.
        let cache = cx.update(|cx| cx.try_global::<GlobalNodeCache>().cloned());

        // Try the cache first.
        if let Some(cache) = cache.clone() {
            let conn_id = connection_id.clone();
            let db = database.clone();
            let sch = schema.clone();
            let tbl = table.clone();
            let result = Tokio::spawn_result(cx, async move {
                if let Some(indexes) = cache.get_indexes(&conn_id, &db, sch.as_deref(), &tbl).await
                {
                    tracing::debug!(
                        "Cache hit for indexes: {}:{}:{:?}:{}",
                        conn_id,
                        db,
                        sch,
                        tbl
                    );
                    return Ok(indexes);
                }
                Err(anyhow::anyhow!("Cache miss"))
            })
            .await;

            if let Ok(indexes) = result {
                return Ok(indexes);
            }
        }

        // Cache miss. Query the database.
        let conn_id = connection_id.clone();
        let db = database.clone();
        let sch = schema.clone();
        let tbl = table.clone();
        let indexes =
            with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
                plugin.list_indexes(&*conn, &database, schema, &table).await
            })?;

        // Persist the result in cache.
        if let Some(cache) = cache {
            let indexes_clone = indexes.clone();
            Tokio::spawn(cx, async move {
                cache
                    .cache_indexes(&conn_id, &db, sch.as_deref(), &tbl, indexes_clone)
                    .await;
                tracing::debug!("Cached indexes for: {}:{}:{:?}:{}", conn_id, db, sch, tbl);
            })
            .detach();
        }

        Ok(indexes)
    }

    /// List views
    pub async fn list_views_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin.list_views_view(&*conn, &database).await
        })
    }

    /// List functions view
    pub async fn list_functions_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin.list_functions_view(&*conn, &database).await
        })
    }

    /// List procedures view
    pub async fn list_procedures_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin.list_procedures_view(&*conn, &database).await
        })
    }

    /// List triggers view
    pub async fn list_triggers_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin.list_triggers_view(&*conn, &database).await
        })
    }

    /// List sequences view
    pub async fn list_sequences_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin.list_sequences_view(&*conn, &database).await
        })
    }

    /// List schemas view
    pub async fn list_schemas_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        database: String,
    ) -> anyhow::Result<crate::types::ObjectView> {
        with_plugin_session_db!(self, cx, connection_id, database.clone(), |plugin, conn| {
            plugin.list_schemas_view(&*conn, &database).await
        })
    }

    /// Load object view based on node type
    pub async fn load_object_view(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        node: DbNode,
    ) -> anyhow::Result<Option<crate::types::ObjectView>> {
        if node.node_type == DbNodeType::Connection && !node.children_loaded {
            info!(
                "[DB][Timing] load_object_view skipped connection_id={} node_id={} reason=connection_children_not_loaded",
                connection_id,
                node.id
            );
            return Ok(None);
        }

        let mut config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?
            .clone();

        let target_database = node.get_database_name();
        if let Some(db) = target_database {
            config.database = Some(db);
        }

        let database = config.database.clone().unwrap_or_default();
        let schema = node.get_schema_name();
        let table = node.get_table_name().unwrap_or_default();
        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            let plugin = clone_self.get_plugin(&config.database_type)?;
            let session_id = clone_self
                .connection_manager
                .create_session(config.clone(), &clone_self.db_manager)
                .await?;

            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
                let view = match node.node_type {
                    DbNodeType::Connection => {
                        if node.children_loaded {
                            if plugin.uses_schema_as_database() {
                                plugin.list_schemas_view(&*conn, &database).await.ok()
                            } else {
                                plugin.list_databases_view(&*conn).await.ok()
                            }
                        } else {
                            None
                        }
                    }
                    DbNodeType::Database => {
                        if plugin.supports_schema() {
                            plugin.list_schemas_view(&*conn, &database).await.ok()
                        } else {
                            plugin.list_tables_view(&*conn, &database, None).await.ok()
                        }
                    }
                    DbNodeType::TablesFolder => plugin
                        .list_tables_view(&*conn, &database, schema)
                        .await
                        .ok(),
                    DbNodeType::Schema => plugin
                        .list_tables_view(&*conn, &database, schema)
                        .await
                        .ok(),
                    DbNodeType::Table | DbNodeType::ColumnsFolder => plugin
                        .list_columns_view(&*conn, &database, schema, &table)
                        .await
                        .ok(),
                    DbNodeType::ViewsFolder => plugin.list_views_view(&*conn, &database).await.ok(),
                    DbNodeType::FunctionsFolder => {
                        plugin.list_functions_view(&*conn, &database).await.ok()
                    }
                    DbNodeType::ProceduresFolder => {
                        plugin.list_procedures_view(&*conn, &database).await.ok()
                    }
                    DbNodeType::TriggersFolder => {
                        plugin.list_triggers_view(&*conn, &database).await.ok()
                    }
                    DbNodeType::SequencesFolder => {
                        plugin.list_sequences_view(&*conn, &database).await.ok()
                    }
                    _ => None,
                };
                Ok::<_, anyhow::Error>(view)
            };

            if let Err(e) = clone_self
                .connection_manager
                .release_session(&session_id)
                .await
            {
                warn!("Failed to release session {}: {}", session_id, e);
            }

            result
        })
        .await
    }

    /// Get completion info
    pub fn get_completion_info(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
    ) -> anyhow::Result<crate::plugin::SqlCompletionInfo> {
        let _ = cx;
        if let Some(config) = self.get_config(&connection_id) {
            match self.get_plugin(&config.database_type) {
                Ok(plugin) => Ok(plugin.get_completion_info()),
                Err(_) => Ok(crate::plugin::SqlCompletionInfo::default()),
            }
        } else {
            Ok(crate::plugin::SqlCompletionInfo::default())
        }
    }

    /// Export data
    pub async fn export_data(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        config: ExportConfig,
    ) -> anyhow::Result<ExportResult> {
        self.export_data_with_progress(cx, connection_id, config, None)
            .await
    }

    /// Export data with progress callback
    pub async fn export_data_with_progress(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        config: ExportConfig,
        progress_tx: Option<ExportProgressSender>,
    ) -> anyhow::Result<ExportResult> {
        let db_config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            let plugin = clone_self.get_plugin(&db_config.database_type)?;
            let session_id = clone_self
                .connection_manager
                .create_session(db_config.clone(), &clone_self.db_manager)
                .await?;

            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
                plugin
                    .export_data_with_progress(conn, &config, progress_tx)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            };

            clone_self
                .connection_manager
                .release_session(&session_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            result
        })
        .await
    }

    /// Export data with progress callback (sync version for background tasks)
    pub async fn export_data_with_progress_sync(
        &self,
        connection_id: String,
        config: ExportConfig,
        progress_tx: Option<ExportProgressSender>,
    ) -> anyhow::Result<ExportResult> {
        let db_config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        let plugin = self.get_plugin(&db_config.database_type)?;
        let session_id = self
            .connection_manager
            .create_session(db_config.clone(), &self.db_manager)
            .await?;

        let result = {
            let mut guard = self
                .connection_manager
                .get_session_connection(&session_id)
                .await?;
            let conn = guard
                .connection()
                .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
            plugin
                .export_data_with_progress(conn, &config, progress_tx)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        };

        self.connection_manager
            .release_session(&session_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        result
    }

    /// Import data
    pub async fn import_data(
        &self,
        cx: &mut AsyncApp,
        connection_id: String,
        config: ImportConfig,
        data: String,
    ) -> anyhow::Result<ImportResult> {
        let db_config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        let clone_self = self.clone();
        Tokio::spawn_result(cx, async move {
            let session_id = clone_self
                .connection_manager
                .create_session(db_config.clone(), &clone_self.db_manager)
                .await?;

            let plugin = clone_self.get_plugin(&db_config.database_type)?;

            let result = {
                let mut guard = clone_self
                    .connection_manager
                    .get_session_connection(&session_id)
                    .await?;
                let conn = guard
                    .connection()
                    .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
                plugin
                    .import_data(&*conn, &config, &data)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            };

            clone_self
                .connection_manager
                .release_session(&session_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            result
        })
        .await
    }

    /// Import data with progress callback (sync version for background tasks)
    pub async fn import_data_with_progress_sync(
        &self,
        connection_id: String,
        config: ImportConfig,
        data: String,
        file_name: &str,
        progress_tx: Option<crate::import_export::ImportProgressSender>,
    ) -> anyhow::Result<ImportResult> {
        let db_config = self
            .get_config(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        let plugin = self.get_plugin(&db_config.database_type)?;
        let session_id = self
            .connection_manager
            .create_session(db_config.clone(), &self.db_manager)
            .await?;

        let result = {
            let mut guard = self
                .connection_manager
                .get_session_connection(&session_id)
                .await?;
            let conn = guard
                .connection()
                .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
            plugin
                .import_data_with_progress(conn, &config, &data, file_name, progress_tx)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        };

        self.connection_manager
            .release_session(&session_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        result
    }

    /// Pure async version of `list_tables` — can be called from any tokio context
    /// without `AsyncApp`. Skips `GlobalNodeCache`.
    pub async fn list_tables_direct(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<String>,
    ) -> anyhow::Result<Vec<crate::types::TableInfo>> {
        let config = self
            .get_config(connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;
        let mut config = config.clone();
        config.database = Some(database.to_string());

        let plugin = self.get_plugin(&config.database_type)?;
        let session_id = self
            .connection_manager
            .create_session(config, &self.db_manager)
            .await?;

        let result = {
            let mut guard = self
                .connection_manager
                .get_session_connection(&session_id)
                .await?;
            let conn = guard
                .connection()
                .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
            plugin
                .list_tables(conn, database, schema)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        };

        self.connection_manager
            .release_session(&session_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        result
    }

    /// Pure async version of `list_columns` — can be called from any tokio context
    /// without `AsyncApp`. Skips `GlobalNodeCache`.
    pub async fn list_columns_direct(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> anyhow::Result<Vec<crate::types::ColumnInfo>> {
        let config = self
            .get_config(connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;
        let mut config = config.clone();
        config.database = Some(database.to_string());

        let plugin = self.get_plugin(&config.database_type)?;
        let session_id = self
            .connection_manager
            .create_session(config, &self.db_manager)
            .await?;

        let result = {
            let mut guard = self
                .connection_manager
                .get_session_connection(&session_id)
                .await?;
            let conn = guard
                .connection()
                .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;
            plugin
                .list_columns(conn, database, schema, table)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        };

        self.connection_manager
            .release_session(&session_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        result
    }

    /// Pure async SQL execution version — can be called from any tokio context
    /// without `AsyncApp`. Skips cache invalidation and notifier side effects.
    pub async fn execute_script_direct(
        &self,
        connection_id: &str,
        script: &str,
        database: Option<String>,
        schema: Option<String>,
        opts: Option<ExecOptions>,
    ) -> anyhow::Result<Vec<SqlResult>> {
        let mut config = self
            .get_config(connection_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?
            .clone();

        // For non-Oracle databases, switch database through config override.
        if config.database_type != DatabaseType::Oracle {
            if let Some(db) = database {
                config.database = Some(db);
            }
        }

        let plugin = self.get_plugin(&config.database_type)?;
        let session_id = self
            .connection_manager
            .create_session(config, &self.db_manager)
            .await?;

        let result = {
            let mut guard = self
                .connection_manager
                .get_session_connection(&session_id)
                .await?;
            let conn = guard
                .connection()
                .ok_or_else(|| anyhow::anyhow!("Session connection not found"))?;

            if let Some(schema) = &schema {
                conn.switch_schema(schema)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to switch schema: {}", e))?;
            }

            conn.execute(plugin.as_ref(), script, opts.unwrap_or_default())
                .await
        };

        self.connection_manager.close_session(&session_id).await?;
        result.map_err(|e| anyhow::anyhow!("{}", e))
    }
}

impl Default for GlobalDbState {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for GlobalDbState {}

#[cfg(test)]
mod tests {
    use super::*;
    use one_core::storage::DatabaseType;

    #[test]
    fn test_db_manager_registers_duckdb_plugin() {
        let plugin = DbManager::default()
            .get_plugin(&DatabaseType::DuckDB)
            .expect("DuckDB plugin should be registered");

        assert_eq!(plugin.name(), DatabaseType::DuckDB);
    }

    #[test]
    fn test_cached_children_ready_allows_empty_children() {
        let node = DbNode::new(
            "node-id",
            "node",
            DbNodeType::Table,
            "conn-id".to_string(),
            DatabaseType::SQLite,
        )
        .with_children_loaded(true);

        assert!(GlobalDbState::cached_children_ready(&node));
    }

    #[test]
    fn test_cached_children_ready_blocks_unloaded_children() {
        let node = DbNode::new(
            "node-id",
            "node",
            DbNodeType::Table,
            "conn-id".to_string(),
            DatabaseType::SQLite,
        );

        assert!(!GlobalDbState::cached_children_ready(&node));
    }
}
