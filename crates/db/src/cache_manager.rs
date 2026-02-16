//! 全局缓存管理器
//!
//! 提供统一的缓存管理接口，包括：
//! - 节点缓存（UI 树结构）
//! - 元数据缓存（数据库结构信息）
//! - DDL 失效器（自动缓存失效）

use crate::cache::{CacheContext, NodeCache};
use crate::ddl_invalidator::{DdlEvent, DdlInvalidator};
use crate::metadata_cache::{
    CacheKey, CacheLevel, MetadataCacheConfig, MetadataCacheManager, MetadataEntry,
};
use crate::types::{
    ColumnInfo, DbNode, ForeignKeyDefinition, FunctionInfo, IndexInfo, SequenceInfo,
    TableInfo, TriggerInfo, ViewInfo,
};
use anyhow::Result;
use gpui::{App, Global};
use one_core::storage::manager::get_config_dir;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// 全局缓存状态
#[derive(Clone)]
pub struct GlobalNodeCache {
    /// 节点缓存（UI 树结构）
    node_cache: Arc<NodeCache>,
    /// 元数据缓存（数据库结构信息）
    metadata_cache: Arc<MetadataCacheManager>,
    /// DDL 失效器
    ddl_invalidator: Arc<DdlInvalidator>,
}

impl GlobalNodeCache {
    /// 创建新的全局缓存实例
    pub fn new() -> Result<Self> {
        let cache_dir = get_config_dir()?.join("cache");
        let node_cache = NodeCache::new(cache_dir.clone())?;
        let metadata_cache = MetadataCacheManager::new(cache_dir)?;
        let metadata_cache_arc = Arc::new(metadata_cache);
        let ddl_invalidator = DdlInvalidator::new(metadata_cache_arc.clone());

        Ok(Self {
            node_cache: Arc::new(node_cache),
            metadata_cache: metadata_cache_arc,
            ddl_invalidator: Arc::new(ddl_invalidator),
        })
    }

    /// 使用自定义配置创建全局缓存实例
    pub fn with_config(config: MetadataCacheConfig) -> Result<Self> {
        let cache_dir = get_config_dir()?.join("cache");
        let node_cache = NodeCache::new(cache_dir.clone())?;
        let metadata_cache = MetadataCacheManager::with_config(cache_dir, config)?;
        let metadata_cache_arc = Arc::new(metadata_cache);
        let ddl_invalidator = DdlInvalidator::new(metadata_cache_arc.clone());

        Ok(Self {
            node_cache: Arc::new(node_cache),
            metadata_cache: metadata_cache_arc,
            ddl_invalidator: Arc::new(ddl_invalidator),
        })
    }

    // ========== 节点缓存方法 ==========

    /// 获取节点（优先从缓存）
    pub async fn get_node(&self, ctx: &CacheContext, node_id: &str) -> Option<DbNode> {
        self.node_cache.get_node(ctx, node_id).await
    }

    /// 缓存节点数据
    pub async fn cache_node(&self, ctx: &CacheContext, node_id: &str, node: &DbNode) {
        self.node_cache.cache_node(ctx, node_id, node).await;
    }

    /// 使指定节点的缓存失效
    pub async fn invalidate_node(&self, ctx: &CacheContext, node_id: &str) {
        self.node_cache.invalidate_node(ctx, node_id).await;
    }

    /// 递归使节点及其所有后代的缓存失效
    pub async fn invalidate_node_recursive(&self, ctx: &CacheContext, node_id: &str) {
        self.node_cache.invalidate_node_recursive(ctx, node_id).await;
    }

    /// 清除指定连接的所有节点缓存
    pub async fn clear_connection_cache(&self, ctx: &CacheContext) {
        self.node_cache.clear_connection_cache(ctx).await;
    }

    /// 清除所有节点缓存
    pub async fn clear_all(&self) {
        self.node_cache.clear_all().await;
    }

    /// 获取节点缓存统计信息
    pub fn stats(&self) -> crate::cache::CacheStats {
        self.node_cache.stats()
    }

    // ========== 元数据缓存方法 ==========

    /// 获取元数据缓存管理器
    pub fn metadata_cache(&self) -> &Arc<MetadataCacheManager> {
        &self.metadata_cache
    }

    /// 获取数据库列表（带缓存）
    pub async fn get_databases(&self, connection_id: &str) -> Option<Vec<String>> {
        let key = CacheKey::databases(connection_id);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Databases(dbs)) => Some(dbs),
            _ => None,
        }
    }

    /// 缓存数据库列表
    pub async fn cache_databases(&self, connection_id: &str, databases: Vec<String>) {
        let key = CacheKey::databases(connection_id);
        self.metadata_cache
            .set(&key, MetadataEntry::Databases(databases), CacheLevel::Connection)
            .await;
    }

    /// 获取 Schema 列表（带缓存）
    pub async fn get_schemas(&self, connection_id: &str, database: &str) -> Option<Vec<String>> {
        let key = CacheKey::schemas(connection_id, database);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Schemas(schemas)) => Some(schemas),
            _ => None,
        }
    }

    /// 缓存 Schema 列表
    pub async fn cache_schemas(
        &self,
        connection_id: &str,
        database: &str,
        schemas: Vec<String>,
    ) {
        let key = CacheKey::schemas(connection_id, database);
        self.metadata_cache
            .set(&key, MetadataEntry::Schemas(schemas), CacheLevel::Connection)
            .await;
    }

    /// 获取表列表（带缓存）
    pub async fn get_tables(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
    ) -> Option<Vec<TableInfo>> {
        let key = CacheKey::tables(connection_id, database, schema);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Tables(tables)) => Some(tables),
            _ => None,
        }
    }

    /// 缓存表列表
    pub async fn cache_tables(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        tables: Vec<TableInfo>,
    ) {
        let key = CacheKey::tables(connection_id, database, schema);
        self.metadata_cache
            .set(&key, MetadataEntry::Tables(tables), CacheLevel::Database)
            .await;
    }

    /// 获取视图列表（带缓存）
    pub async fn get_views(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
    ) -> Option<Vec<ViewInfo>> {
        let key = CacheKey::views(connection_id, database, schema);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Views(views)) => Some(views),
            _ => None,
        }
    }

    /// 缓存视图列表
    pub async fn cache_views(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        views: Vec<ViewInfo>,
    ) {
        let key = CacheKey::views(connection_id, database, schema);
        self.metadata_cache
            .set(&key, MetadataEntry::Views(views), CacheLevel::Database)
            .await;
    }

    /// 获取列信息（带缓存）
    pub async fn get_columns(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Option<Vec<ColumnInfo>> {
        let key = CacheKey::columns(connection_id, database, schema, table);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Columns(columns)) => Some(columns),
            _ => None,
        }
    }

    /// 缓存列信息
    pub async fn cache_columns(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
        columns: Vec<ColumnInfo>,
    ) {
        let key = CacheKey::columns(connection_id, database, schema, table);
        self.metadata_cache
            .set(&key, MetadataEntry::Columns(columns), CacheLevel::Table)
            .await;
    }

    /// 获取索引信息（带缓存）
    pub async fn get_indexes(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Option<Vec<IndexInfo>> {
        let key = CacheKey::indexes(connection_id, database, schema, table);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Indexes(indexes)) => Some(indexes),
            _ => None,
        }
    }

    /// 缓存索引信息
    pub async fn cache_indexes(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
        indexes: Vec<IndexInfo>,
    ) {
        let key = CacheKey::indexes(connection_id, database, schema, table);
        self.metadata_cache
            .set(&key, MetadataEntry::Indexes(indexes), CacheLevel::Table)
            .await;
    }

    /// 获取外键信息（带缓存）
    pub async fn get_foreign_keys(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Option<Vec<ForeignKeyDefinition>> {
        let key = CacheKey::foreign_keys(connection_id, database, schema, table);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::ForeignKeys(fks)) => Some(fks),
            _ => None,
        }
    }

    /// 缓存外键信息
    pub async fn cache_foreign_keys(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
        foreign_keys: Vec<ForeignKeyDefinition>,
    ) {
        let key = CacheKey::foreign_keys(connection_id, database, schema, table);
        self.metadata_cache
            .set(
                &key,
                MetadataEntry::ForeignKeys(foreign_keys),
                CacheLevel::Table,
            )
            .await;
    }

    /// 获取函数列表（带缓存）
    pub async fn get_functions(
        &self,
        connection_id: &str,
        database: &str,
    ) -> Option<Vec<FunctionInfo>> {
        let key = CacheKey::functions(connection_id, database);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Functions(functions)) => Some(functions),
            _ => None,
        }
    }

    /// 缓存函数列表
    pub async fn cache_functions(
        &self,
        connection_id: &str,
        database: &str,
        functions: Vec<FunctionInfo>,
    ) {
        let key = CacheKey::functions(connection_id, database);
        self.metadata_cache
            .set(&key, MetadataEntry::Functions(functions), CacheLevel::Database)
            .await;
    }

    /// 获取存储过程列表（带缓存）
    pub async fn get_procedures(
        &self,
        connection_id: &str,
        database: &str,
    ) -> Option<Vec<FunctionInfo>> {
        let key = CacheKey::procedures(connection_id, database);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Procedures(procedures)) => Some(procedures),
            _ => None,
        }
    }

    /// 缓存存储过程列表
    pub async fn cache_procedures(
        &self,
        connection_id: &str,
        database: &str,
        procedures: Vec<FunctionInfo>,
    ) {
        let key = CacheKey::procedures(connection_id, database);
        self.metadata_cache
            .set(
                &key,
                MetadataEntry::Procedures(procedures),
                CacheLevel::Database,
            )
            .await;
    }

    /// 获取触发器列表（带缓存）
    pub async fn get_triggers(
        &self,
        connection_id: &str,
        database: &str,
    ) -> Option<Vec<TriggerInfo>> {
        let key = CacheKey::triggers(connection_id, database);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Triggers(triggers)) => Some(triggers),
            _ => None,
        }
    }

    /// 缓存触发器列表
    pub async fn cache_triggers(
        &self,
        connection_id: &str,
        database: &str,
        triggers: Vec<TriggerInfo>,
    ) {
        let key = CacheKey::triggers(connection_id, database);
        self.metadata_cache
            .set(&key, MetadataEntry::Triggers(triggers), CacheLevel::Database)
            .await;
    }

    /// 获取序列列表（带缓存）
    pub async fn get_sequences(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
    ) -> Option<Vec<SequenceInfo>> {
        let key = CacheKey::sequences(connection_id, database, schema);
        match self.metadata_cache.get(&key).await {
            Some(MetadataEntry::Sequences(sequences)) => Some(sequences),
            _ => None,
        }
    }

    /// 缓存序列列表
    pub async fn cache_sequences(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        sequences: Vec<SequenceInfo>,
    ) {
        let key = CacheKey::sequences(connection_id, database, schema);
        self.metadata_cache
            .set(
                &key,
                MetadataEntry::Sequences(sequences),
                CacheLevel::Database,
            )
            .await;
    }

    // ========== DDL 失效方法 ==========

    /// 通知 DDL 事件（同步失效）
    pub async fn notify_ddl(&self, connection_id: &str, event: DdlEvent) {
        self.ddl_invalidator.invalidate(connection_id, &event).await;
    }

    /// 解析 SQL 并触发 DDL 失效（MetadataCache + NodeCache）
    ///
    /// 返回解析到的 DDL 事件信息（connection_id, database, schema），供调用方发射 SchemaChanged 事件。
    /// 支持多语句：返回第一个 DDL 事件的范围信息。
    pub async fn process_sql_for_invalidation(
        &self,
        connection_id: &str,
        sql: &str,
        current_database: &str,
        current_schema: Option<&str>,
        cache_ctx: Option<&CacheContext>,
    ) -> Option<(String, String, Option<String>)> {
        let events = DdlInvalidator::parse_ddl_events(sql, current_database, current_schema);

        if events.is_empty() {
            return None;
        }

        // 提取第一个事件的 database 和 schema 信息用于返回
        let (database, schema) = Self::extract_ddl_scope(&events[0]);

        // 同步失效 MetadataCache（处理所有事件）
        for event in &events {
            self.ddl_invalidator.invalidate(connection_id, event).await;
        }

        // 同步失效 NodeCache
        if let Some(ctx) = cache_ctx {
            self.node_cache.clear_connection_cache(ctx).await;
        }

        Some((connection_id.to_string(), database, schema))
    }

    /// 从 DDL 事件中提取数据库和 schema 信息
    fn extract_ddl_scope(event: &DdlEvent) -> (String, Option<String>) {
        match event {
            DdlEvent::CreateTable { database, schema, .. }
            | DdlEvent::AlterTable { database, schema, .. }
            | DdlEvent::DropTable { database, schema, .. }
            | DdlEvent::TruncateTable { database, schema, .. }
            | DdlEvent::RenameTable { database, schema, .. }
            | DdlEvent::CreateIndex { database, schema, .. }
            | DdlEvent::DropIndex { database, schema, .. }
            | DdlEvent::CreateView { database, schema, .. }
            | DdlEvent::DropView { database, schema, .. }
            | DdlEvent::CreateTrigger { database, schema, .. }
            | DdlEvent::DropTrigger { database, schema, .. }
            | DdlEvent::CreateSequence { database, schema, .. }
            | DdlEvent::DropSequence { database, schema, .. } => {
                (database.clone(), schema.clone())
            }
            DdlEvent::CreateSchema { database, schema }
            | DdlEvent::DropSchema { database, schema } => {
                (database.clone(), Some(schema.clone()))
            }
            DdlEvent::CreateDatabase { database }
            | DdlEvent::DropDatabase { database } => {
                (database.clone(), None)
            }
            DdlEvent::CreateFunction { database, .. }
            | DdlEvent::DropFunction { database, .. }
            | DdlEvent::CreateProcedure { database, .. }
            | DdlEvent::DropProcedure { database, .. } => {
                (database.clone(), None)
            }
        }
    }

    /// 使表相关缓存失效
    pub async fn invalidate_table(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) {
        self.metadata_cache
            .invalidate_table(connection_id, database, schema, table)
            .await;
    }

    /// 使数据库相关缓存失效
    pub async fn invalidate_database(&self, connection_id: &str, database: &str) {
        self.metadata_cache
            .invalidate_database(connection_id, database)
            .await;
    }

    /// 使连接相关元数据缓存失效
    pub async fn invalidate_connection_metadata(&self, connection_id: &str) {
        self.metadata_cache
            .invalidate_connection(connection_id)
            .await;
    }

    /// 清除所有元数据缓存
    pub async fn clear_all_metadata(&self) {
        self.metadata_cache.clear_all().await;
    }

    /// 获取元数据缓存统计信息
    pub fn metadata_stats(&self) -> crate::metadata_cache::MetadataCacheStats {
        self.metadata_cache.stats()
    }

    /// 启动后台磁盘缓存清理任务
    ///
    /// 每 5 分钟扫描缓存目录，删除过期的 JSON 文件和空目录。
    /// 使用 GPUI background_executor 确保与应用生命周期一致。
    pub fn start_cleanup_task(&self, cx: &App) {
        let node_cache_dir = self.node_cache.cache_dir().clone();
        let metadata_cache_dir = self.metadata_cache.cache_dir().clone();

        cx.background_executor()
            .spawn(async move {
                let interval = std::time::Duration::from_secs(5 * 60);
                loop {
                    smol::Timer::after(interval).await;
                    debug!("Running periodic cache cleanup");
                    Self::cleanup_expired_files(&node_cache_dir).await;
                    Self::cleanup_expired_files(&metadata_cache_dir.join("metadata")).await;
                }
            })
            .detach();
    }

    /// 扫描目录并清理过期的缓存 JSON 文件
    async fn cleanup_expired_files(dir: &Path) {
        if !dir.exists() {
            return;
        }

        let dir = dir.to_path_buf();
        let result = smol::unblock(move || {
            let mut removed_count = 0usize;
            if let Err(e) = Self::cleanup_dir_recursive_sync(&dir, &mut removed_count) {
                warn!("Error during cache cleanup of {}: {}", dir.display(), e);
            }
            removed_count
        }).await;

        if result > 0 {
            info!(
                "Cache cleanup: removed {} expired files",
                result
            );
        }
    }

    /// 递归清理目录中的过期文件（同步版本，在 smol::unblock 中运行）
    fn cleanup_dir_recursive_sync(dir: &Path, removed_count: &mut usize) -> Result<()> {
        let entries = std::fs::read_dir(dir)?;
        let mut empty_subdirs = vec![];

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::cleanup_dir_recursive_sync(&path, removed_count)?;
                // 检查子目录是否为空
                if std::fs::read_dir(&path)?.next().is_none() {
                    empty_subdirs.push(path);
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if Self::is_cache_file_expired_sync(&path) {
                    if let Err(e) = std::fs::remove_file(&path) {
                        warn!("Failed to remove expired cache file {}: {}", path.display(), e);
                    } else {
                        *removed_count += 1;
                    }
                }
            }
        }

        // 清理空目录
        for empty_dir in empty_subdirs {
            let _ = std::fs::remove_dir(&empty_dir);
        }

        Ok(())
    }

    /// 检查缓存文件是否过期（同步版本）
    fn is_cache_file_expired_sync(path: &Path) -> bool {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return true,
        };

        // 尝试解析为 CachedEntry（MetadataCache 格式）
        if let Ok(entry) = serde_json::from_str::<crate::metadata_cache::CachedEntry>(&content) {
            return entry.is_expired();
        }

        // 尝试解析通用格式（NodeCache 格式）
        #[derive(serde::Deserialize)]
        struct CacheTimestamp {
            cached_at: std::time::SystemTime,
            ttl_millis: u64,
        }

        if let Ok(ts) = serde_json::from_str::<CacheTimestamp>(&content) {
            return match ts.cached_at.elapsed() {
                Ok(elapsed) => elapsed.as_millis() as u64 > ts.ttl_millis,
                Err(_) => true,
            };
        }

        // 无法解析的文件：检查文件修改时间，超过 1 小时视为过期
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = modified.elapsed() {
                    return age.as_secs() > 3600;
                }
            }
        }

        true
    }
}

impl Global for GlobalNodeCache {}

/// 初始化全局缓存
pub fn init_cache(cx: &mut App) {
    let global_cache = GlobalNodeCache::new().expect("Failed to create cache");
    cx.set_global(global_cache);
}

/// 使用自定义配置初始化全局缓存
pub fn init_cache_with_config(cx: &mut App, config: MetadataCacheConfig) {
    let global_cache = GlobalNodeCache::with_config(config).expect("Failed to create cache");
    cx.set_global(global_cache);
}

/// 辅助 trait：为 DatabasePlugin 添加带缓存的方法
pub trait CachedDatabaseOps {
    /// 获取数据库列表（带缓存）
    fn list_databases_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<String>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(databases) = cache.get_databases(connection_id).await {
                return Ok(databases);
            }

            // 从数据库查询
            let databases = self.list_databases(connection).await?;

            // 写入缓存
            cache.cache_databases(connection_id, databases.clone()).await;

            Ok(databases)
        }
    }

    /// 获取表列表（带缓存）
    fn list_tables_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
        schema: Option<String>,
    ) -> impl std::future::Future<Output = Result<Vec<TableInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(tables) = cache.get_tables(connection_id, database, schema.as_deref()).await
            {
                return Ok(tables);
            }

            // 从数据库查询
            let tables = self.list_tables(connection, database, schema.clone()).await?;

            // 写入缓存
            cache
                .cache_tables(connection_id, database, schema.as_deref(), tables.clone())
                .await;

            Ok(tables)
        }
    }

    /// 获取列信息（带缓存）
    fn list_columns_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
        schema: Option<String>,
        table: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<ColumnInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(columns) = cache
                .get_columns(connection_id, database, schema.as_deref(), table)
                .await
            {
                return Ok(columns);
            }

            // 从数据库查询
            let columns = self
                .list_columns(connection, database, schema.clone(), table)
                .await?;

            // 写入缓存
            cache
                .cache_columns(
                    connection_id,
                    database,
                    schema.as_deref(),
                    table,
                    columns.clone(),
                )
                .await;

            Ok(columns)
        }
    }

    /// 获取索引信息（带缓存）
    fn list_indexes_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
        schema: Option<String>,
        table: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<IndexInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(indexes) = cache
                .get_indexes(connection_id, database, schema.as_deref(), table)
                .await
            {
                return Ok(indexes);
            }

            // 从数据库查询
            let indexes = self
                .list_indexes(connection, database, schema.clone(), table)
                .await?;

            // 写入缓存
            cache
                .cache_indexes(
                    connection_id,
                    database,
                    schema.as_deref(),
                    table,
                    indexes.clone(),
                )
                .await;

            Ok(indexes)
        }
    }

    /// 获取外键信息（带缓存）
    fn list_foreign_keys_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
        schema: Option<String>,
        table: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<ForeignKeyDefinition>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(fks) = cache
                .get_foreign_keys(connection_id, database, schema.as_deref(), table)
                .await
            {
                return Ok(fks);
            }

            // 从数据库查询
            let fks = self
                .list_foreign_keys(connection, database, schema.clone(), table)
                .await?;

            // 写入缓存
            cache
                .cache_foreign_keys(
                    connection_id,
                    database,
                    schema.as_deref(),
                    table,
                    fks.clone(),
                )
                .await;

            Ok(fks)
        }
    }

    /// 获取视图列表（带缓存）
    fn list_views_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
        schema: Option<String>,
    ) -> impl std::future::Future<Output = Result<Vec<ViewInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(views) = cache.get_views(connection_id, database, schema.as_deref()).await {
                return Ok(views);
            }

            // 从数据库查询
            let views = self.list_views(connection, database, schema.clone()).await?;

            // 写入缓存
            cache
                .cache_views(connection_id, database, schema.as_deref(), views.clone())
                .await;

            Ok(views)
        }
    }

    /// 获取函数列表（带缓存）
    fn list_functions_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<FunctionInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(functions) = cache.get_functions(connection_id, database).await {
                return Ok(functions);
            }

            // 从数据库查询
            let functions = self.list_functions(connection, database).await?;

            // 写入缓存
            cache
                .cache_functions(connection_id, database, functions.clone())
                .await;

            Ok(functions)
        }
    }

    /// 获取存储过程列表（带缓存）
    fn list_procedures_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<FunctionInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(procedures) = cache.get_procedures(connection_id, database).await {
                return Ok(procedures);
            }

            // 从数据库查询
            let procedures = self.list_procedures(connection, database).await?;

            // 写入缓存
            cache
                .cache_procedures(connection_id, database, procedures.clone())
                .await;

            Ok(procedures)
        }
    }

    /// 获取触发器列表（带缓存）
    fn list_triggers_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
    ) -> impl std::future::Future<Output = Result<Vec<TriggerInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(triggers) = cache.get_triggers(connection_id, database).await {
                return Ok(triggers);
            }

            // 从数据库查询
            let triggers = self.list_triggers(connection, database).await?;

            // 写入缓存
            cache
                .cache_triggers(connection_id, database, triggers.clone())
                .await;

            Ok(triggers)
        }
    }

    /// 获取序列列表（带缓存）
    fn list_sequences_cached<'a>(
        &'a self,
        connection: &'a dyn crate::connection::DbConnection,
        cache: &'a GlobalNodeCache,
        connection_id: &'a str,
        database: &'a str,
        schema: Option<String>,
    ) -> impl std::future::Future<Output = Result<Vec<SequenceInfo>>> + Send + 'a
    where
        Self: crate::plugin::DatabasePlugin + Sync,
    {
        async move {
            // 尝试从缓存获取
            if let Some(sequences) = cache
                .get_sequences(connection_id, database, schema.as_deref())
                .await
            {
                return Ok(sequences);
            }

            // 从数据库查询
            let sequences = self
                .list_sequences(connection, database, schema.clone())
                .await?;

            // 写入缓存
            cache
                .cache_sequences(connection_id, database, schema.as_deref(), sequences.clone())
                .await;

            Ok(sequences)
        }
    }
}

// 为所有实现 DatabasePlugin 的类型自动实现 CachedDatabaseOps
impl<T: crate::plugin::DatabasePlugin + ?Sized> CachedDatabaseOps for T {}
