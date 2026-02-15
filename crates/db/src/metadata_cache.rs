//! 数据库元数据缓存模块
//!
//! 实现分层缓存策略，支持：
//! - 内存缓存（DashMap）+ 文件持久化（JSON）
//! - 按层级配置不同的 TTL
//! - DDL 执行后自动失效
//! - LRU 淘汰策略

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::fs;
use tracing::{debug, warn};

use crate::types::{
    ColumnInfo, ForeignKeyDefinition, FunctionInfo, IndexInfo, SequenceInfo, TableInfo,
    TriggerInfo, ViewInfo,
};

/// 缓存层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheLevel {
    /// 连接级别（数据库列表）
    Connection,
    /// 数据库级别（表/视图/函数列表）
    Database,
    /// 表级别（表结构详情）
    Table,
    /// 详情级别（DDL定义、数据预览）
    Detail,
}

impl CacheLevel {
    /// 获取该层级的默认 TTL
    pub fn default_ttl(&self) -> Duration {
        match self {
            CacheLevel::Connection => Duration::from_secs(30 * 60), // 30分钟
            CacheLevel::Database => Duration::from_secs(15 * 60),   // 15分钟
            CacheLevel::Table => Duration::from_secs(10 * 60),      // 10分钟
            CacheLevel::Detail => Duration::from_secs(5 * 60),      // 5分钟
        }
    }
}

/// 缓存条目类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetadataEntry {
    /// 数据库列表
    Databases(Vec<String>),
    /// Schema 列表
    Schemas(Vec<String>),
    /// 表列表
    Tables(Vec<TableInfo>),
    /// 视图列表
    Views(Vec<ViewInfo>),
    /// 列信息
    Columns(Vec<ColumnInfo>),
    /// 索引信息
    Indexes(Vec<IndexInfo>),
    /// 外键信息
    ForeignKeys(Vec<ForeignKeyDefinition>),
    /// 函数列表
    Functions(Vec<FunctionInfo>),
    /// 存储过程列表
    Procedures(Vec<FunctionInfo>),
    /// 触发器列表
    Triggers(Vec<TriggerInfo>),
    /// 序列列表
    Sequences(Vec<SequenceInfo>),
    /// 表 DDL
    TableDDL(String),
}

/// 缓存条目类型标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryType {
    Databases,
    Schemas,
    Tables,
    Views,
    Columns,
    Indexes,
    ForeignKeys,
    Functions,
    Procedures,
    Triggers,
    Sequences,
    TableDDL,
    Checks,
    TableTriggers,
}

impl EntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntryType::Databases => "databases",
            EntryType::Schemas => "schemas",
            EntryType::Tables => "tables",
            EntryType::Views => "views",
            EntryType::Columns => "columns",
            EntryType::Indexes => "indexes",
            EntryType::ForeignKeys => "foreign_keys",
            EntryType::Functions => "functions",
            EntryType::Procedures => "procedures",
            EntryType::Triggers => "triggers",
            EntryType::Sequences => "sequences",
            EntryType::TableDDL => "table_ddl",
            EntryType::Checks => "checks",
            EntryType::TableTriggers => "table_triggers",
        }
    }
}

/// 缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// 连接 ID
    pub connection_id: String,
    /// 数据库名
    pub database: Option<String>,
    /// Schema 名
    pub schema: Option<String>,
    /// 对象名（表名、视图名等）
    pub object_name: Option<String>,
    /// 条目类型
    pub entry_type: EntryType,
}

impl CacheKey {
    /// 创建数据库列表缓存键
    pub fn databases(connection_id: &str) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: None,
            schema: None,
            object_name: None,
            entry_type: EntryType::Databases,
        }
    }

    /// 创建 Schema 列表缓存键
    pub fn schemas(connection_id: &str, database: &str) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: None,
            object_name: None,
            entry_type: EntryType::Schemas,
        }
    }

    /// 创建表列表缓存键
    pub fn tables(connection_id: &str, database: &str, schema: Option<&str>) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: None,
            entry_type: EntryType::Tables,
        }
    }

    /// 创建视图列表缓存键
    pub fn views(connection_id: &str, database: &str, schema: Option<&str>) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: None,
            entry_type: EntryType::Views,
        }
    }

    /// 创建列信息缓存键
    pub fn columns(
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: Some(table.to_string()),
            entry_type: EntryType::Columns,
        }
    }

    /// 创建索引信息缓存键
    pub fn indexes(
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: Some(table.to_string()),
            entry_type: EntryType::Indexes,
        }
    }

    /// 创建外键信息缓存键
    pub fn foreign_keys(
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: Some(table.to_string()),
            entry_type: EntryType::ForeignKeys,
        }
    }

    /// 创建函数列表缓存键
    pub fn functions(connection_id: &str, database: &str) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: None,
            object_name: None,
            entry_type: EntryType::Functions,
        }
    }

    /// 创建存储过程列表缓存键
    pub fn procedures(connection_id: &str, database: &str) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: None,
            object_name: None,
            entry_type: EntryType::Procedures,
        }
    }

    /// 创建触发器列表缓存键
    pub fn triggers(connection_id: &str, database: &str) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: None,
            object_name: None,
            entry_type: EntryType::Triggers,
        }
    }

    /// 创建序列列表缓存键
    pub fn sequences(connection_id: &str, database: &str, schema: Option<&str>) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: None,
            entry_type: EntryType::Sequences,
        }
    }

    /// 创建表触发器缓存键
    pub fn table_triggers(
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: Some(table.to_string()),
            entry_type: EntryType::TableTriggers,
        }
    }

    /// 创建检查约束缓存键
    pub fn checks(
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: Some(database.to_string()),
            schema: schema.map(|s| s.to_string()),
            object_name: Some(table.to_string()),
            entry_type: EntryType::Checks,
        }
    }

    /// 转换为字符串键
    pub fn to_string_key(&self) -> String {
        let mut parts = vec![self.connection_id.clone()];
        if let Some(db) = &self.database {
            parts.push(db.clone());
        }
        if let Some(schema) = &self.schema {
            parts.push(schema.clone());
        }
        if let Some(obj) = &self.object_name {
            parts.push(obj.clone());
        }
        parts.push(self.entry_type.as_str().to_string());
        parts.join(":")
    }
}

/// 带元信息的缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    /// 缓存数据
    pub data: MetadataEntry,
    /// 缓存时间
    pub cached_at: SystemTime,
    /// TTL（毫秒）
    pub ttl_millis: u64,
    /// 版本号（用于版本检测）
    pub version: Option<String>,
}

impl CachedEntry {
    /// 创建新的缓存条目
    pub fn new(data: MetadataEntry, level: CacheLevel) -> Self {
        Self {
            data,
            cached_at: SystemTime::now(),
            ttl_millis: level.default_ttl().as_millis() as u64,
            version: None,
        }
    }

    /// 使用自定义 TTL 创建缓存条目
    pub fn with_ttl(data: MetadataEntry, ttl: Duration) -> Self {
        Self {
            data,
            cached_at: SystemTime::now(),
            ttl_millis: ttl.as_millis() as u64,
            version: None,
        }
    }

    /// 设置版本号
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        match self.cached_at.elapsed() {
            Ok(elapsed) => elapsed.as_millis() as u64 > self.ttl_millis,
            Err(_) => true,
        }
    }

    /// 获取剩余有效时间（秒）
    pub fn remaining_ttl_secs(&self) -> Option<u64> {
        match self.cached_at.elapsed() {
            Ok(elapsed) => {
                let elapsed_millis = elapsed.as_millis() as u64;
                if elapsed_millis >= self.ttl_millis {
                    Some(0)
                } else {
                    Some((self.ttl_millis - elapsed_millis) / 1000)
                }
            }
            Err(_) => None,
        }
    }
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct MetadataCacheConfig {
    /// 连接级别 TTL
    pub connection_ttl: Duration,
    /// 数据库级别 TTL
    pub database_ttl: Duration,
    /// 表级别 TTL
    pub table_ttl: Duration,
    /// 详情级别 TTL
    pub detail_ttl: Duration,
    /// 最大内存缓存条目数
    pub max_memory_entries: usize,
    /// 是否启用文件缓存
    pub enable_file_cache: bool,
}

impl Default for MetadataCacheConfig {
    fn default() -> Self {
        Self {
            connection_ttl: Duration::from_secs(30 * 60), // 30分钟
            database_ttl: Duration::from_secs(15 * 60),   // 15分钟
            table_ttl: Duration::from_secs(10 * 60),      // 10分钟
            detail_ttl: Duration::from_secs(5 * 60),      // 5分钟
            max_memory_entries: 10000,
            enable_file_cache: true,
        }
    }
}

impl MetadataCacheConfig {
    /// 获取指定层级的 TTL
    pub fn ttl_for_level(&self, level: CacheLevel) -> Duration {
        match level {
            CacheLevel::Connection => self.connection_ttl,
            CacheLevel::Database => self.database_ttl,
            CacheLevel::Table => self.table_ttl,
            CacheLevel::Detail => self.detail_ttl,
        }
    }
}

/// 元数据缓存管理器
pub struct MetadataCacheManager {
    /// 内存缓存
    memory_cache: DashMap<String, CachedEntry>,
    /// 文件缓存目录
    cache_dir: PathBuf,
    /// 缓存配置
    config: MetadataCacheConfig,
}

impl MetadataCacheManager {
    /// 创建新的缓存管理器
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        Self::with_config(cache_dir, MetadataCacheConfig::default())
    }

    /// 使用自定义配置创建缓存管理器
    pub fn with_config(cache_dir: PathBuf, config: MetadataCacheConfig) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            memory_cache: DashMap::new(),
            cache_dir,
            config,
        })
    }

    /// 获取缓存（优先内存 → 文件）
    pub async fn get(&self, key: &CacheKey) -> Option<MetadataEntry> {
        let string_key = key.to_string_key();

        // 1. 尝试从内存获取
        if let Some(entry) = self.memory_cache.get(&string_key) {
            if !entry.is_expired() {
                debug!("Metadata cache hit (memory): {}", string_key);
                return Some(entry.data.clone());
            }
            drop(entry);
            self.memory_cache.remove(&string_key);
            debug!("Metadata cache expired (memory): {}", string_key);
        }

        // 2. 尝试从文件加载
        if self.config.enable_file_cache {
            if let Ok(Some(entry)) = self.load_from_file(key).await {
                if !entry.is_expired() {
                    debug!("Metadata cache hit (file): {}", string_key);
                    self.memory_cache.insert(string_key.clone(), entry.clone());
                    return Some(entry.data);
                }
                debug!("Metadata cache expired (file): {}", string_key);
                // 删除过期的文件缓存
                let _ = self.remove_file_cache(key).await;
            }
        }

        debug!("Metadata cache miss: {}", string_key);
        None
    }

    /// 写入缓存
    pub async fn set(&self, key: &CacheKey, data: MetadataEntry, level: CacheLevel) {
        let string_key = key.to_string_key();
        let ttl = self.config.ttl_for_level(level);
        let entry = CachedEntry::with_ttl(data, ttl);

        // 写入内存
        self.memory_cache.insert(string_key.clone(), entry.clone());
        debug!("Metadata cached (memory): {}", string_key);

        // 异步写入文件
        if self.config.enable_file_cache {
            let cache_dir = self.cache_dir.clone();
            let key_clone = key.clone();
            let entry_clone = entry.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    Self::save_to_file_static(&cache_dir, &key_clone, &entry_clone).await
                {
                    warn!("Failed to save metadata cache to file: {}", e);
                }
            });
        }

        // LRU 淘汰
        self.evict_if_needed();
    }

    /// 使缓存失效
    pub async fn invalidate(&self, key: &CacheKey) {
        let string_key = key.to_string_key();
        self.memory_cache.remove(&string_key);
        debug!("Metadata cache invalidated: {}", string_key);

        if self.config.enable_file_cache {
            let _ = self.remove_file_cache(key).await;
        }
    }

    /// 使连接下所有缓存失效
    pub async fn invalidate_connection(&self, connection_id: &str) {
        let prefix = format!("{}:", connection_id);
        let count = self.memory_cache.len();
        self.memory_cache.retain(|k, _| !k.starts_with(&prefix));
        let removed = count - self.memory_cache.len();
        debug!(
            "Metadata cache invalidated for connection {}: {} entries removed",
            connection_id, removed
        );

        // 删除文件缓存目录
        if self.config.enable_file_cache {
            let dir = self.cache_dir.join("metadata").join(connection_id);
            if dir.exists() {
                let _ = fs::remove_dir_all(dir).await;
            }
        }
    }

    /// 使数据库下所有缓存失效
    pub async fn invalidate_database(&self, connection_id: &str, database: &str) {
        let prefix = format!("{}:{}:", connection_id, database);
        self.memory_cache.retain(|k, _| !k.starts_with(&prefix));
        debug!(
            "Metadata cache invalidated for database {}:{}",
            connection_id, database
        );

        // 删除对应的文件缓存
        if self.config.enable_file_cache {
            let dir = self
                .cache_dir
                .join("metadata")
                .join(connection_id)
                .join(database);
            if dir.exists() {
                let _ = fs::remove_dir_all(dir).await;
            }
        }
    }

    /// 使表相关缓存失效（DDL 执行后调用）
    pub async fn invalidate_table(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) {
        // 失效表结构缓存（列、索引、外键、触发器、检查约束）
        let keys_to_invalidate = vec![
            CacheKey::columns(connection_id, database, schema, table),
            CacheKey::indexes(connection_id, database, schema, table),
            CacheKey::foreign_keys(connection_id, database, schema, table),
            CacheKey::table_triggers(connection_id, database, schema, table),
            CacheKey::checks(connection_id, database, schema, table),
        ];

        for key in keys_to_invalidate {
            self.invalidate(&key).await;
        }

        debug!(
            "Metadata cache invalidated for table {}:{}:{:?}:{}",
            connection_id, database, schema, table
        );
    }

    /// 使表列表缓存失效
    pub async fn invalidate_table_list(
        &self,
        connection_id: &str,
        database: &str,
        schema: Option<&str>,
    ) {
        let key = CacheKey::tables(connection_id, database, schema);
        self.invalidate(&key).await;
    }

    /// 清除所有缓存
    pub async fn clear_all(&self) {
        self.memory_cache.clear();
        debug!("Metadata cache cleared (memory)");

        if self.config.enable_file_cache {
            let metadata_dir = self.cache_dir.join("metadata");
            if metadata_dir.exists() {
                let _ = fs::remove_dir_all(&metadata_dir).await;
                let _ = fs::create_dir_all(&metadata_dir).await;
            }
        }
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> MetadataCacheStats {
        MetadataCacheStats {
            memory_entries: self.memory_cache.len(),
            cache_dir: self.cache_dir.clone(),
        }
    }

    /// LRU 淘汰
    fn evict_if_needed(&self) {
        if self.memory_cache.len() <= self.config.max_memory_entries {
            return;
        }

        // 收集所有条目及其缓存时间
        let mut entries: Vec<(String, SystemTime)> = self
            .memory_cache
            .iter()
            .map(|e| (e.key().clone(), e.cached_at))
            .collect();

        // 按缓存时间排序（最老的在前）
        entries.sort_by_key(|(_, time)| *time);

        // 移除最老的 10%
        let remove_count = self.config.max_memory_entries / 10;
        for (key, _) in entries.into_iter().take(remove_count) {
            self.memory_cache.remove(&key);
        }

        debug!(
            "Metadata cache evicted {} entries",
            remove_count
        );
    }

    /// 获取缓存文件路径
    fn cache_file_path(&self, key: &CacheKey) -> PathBuf {
        let mut path = self.cache_dir.join("metadata");

        path = path.join(&key.connection_id);

        if let Some(db) = &key.database {
            let safe_db = db.replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");
            path = path.join(safe_db);
        }

        if let Some(schema) = &key.schema {
            let safe_schema = schema.replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");
            path = path.join(safe_schema);
        }

        let filename = format!("{}_{}.json",
            key.object_name.as_deref().unwrap_or("_list"),
            key.entry_type.as_str()
        ).replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");

        path.join(filename)
    }

    /// 从文件加载缓存
    async fn load_from_file(&self, key: &CacheKey) -> Result<Option<CachedEntry>> {
        let path = self.cache_file_path(key);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).await?;
        let entry: CachedEntry = serde_json::from_str(&content)?;
        Ok(Some(entry))
    }

    /// 保存到文件
    async fn save_to_file_static(
        cache_dir: &Path,
        key: &CacheKey,
        entry: &CachedEntry,
    ) -> Result<()> {
        let mut path = cache_dir.join("metadata");
        path = path.join(&key.connection_id);

        if let Some(db) = &key.database {
            let safe_db = db.replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");
            path = path.join(safe_db);
        }

        if let Some(schema) = &key.schema {
            let safe_schema = schema.replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");
            path = path.join(safe_schema);
        }

        // 确保目录存在
        fs::create_dir_all(&path).await?;

        let filename = format!("{}_{}.json",
            key.object_name.as_deref().unwrap_or("_list"),
            key.entry_type.as_str()
        ).replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");

        let file_path = path.join(filename);
        let content = serde_json::to_string_pretty(entry)?;
        fs::write(&file_path, content).await?;

        Ok(())
    }

    /// 删除文件缓存
    async fn remove_file_cache(&self, key: &CacheKey) -> Result<()> {
        let path = self.cache_file_path(key);
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        Ok(())
    }
}

/// 缓存统计信息
#[derive(Debug)]
pub struct MetadataCacheStats {
    pub memory_entries: usize,
    pub cache_dir: PathBuf,
}

impl std::fmt::Display for MetadataCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MetadataCache Stats: {} memory entries, cache dir: {}",
            self.memory_entries,
            self.cache_dir.display()
        )
    }
}

/// 刷新粒度
#[derive(Debug, Clone)]
pub enum RefreshScope {
    /// 刷新单个节点
    Node(String),
    /// 刷新表结构（列、索引、外键等）
    TableStructure {
        database: String,
        schema: Option<String>,
        table: String,
    },
    /// 刷新数据库下所有对象列表
    Database { database: String },
    /// 刷新整个连接
    Connection,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_key_to_string() {
        let key = CacheKey::databases("conn1");
        assert_eq!(key.to_string_key(), "conn1:databases");

        let key = CacheKey::tables("conn1", "mydb", Some("public"));
        assert_eq!(key.to_string_key(), "conn1:mydb:public:tables");

        let key = CacheKey::columns("conn1", "mydb", None, "users");
        assert_eq!(key.to_string_key(), "conn1:mydb:users:columns");
    }

    #[test]
    fn test_cache_level_ttl() {
        assert_eq!(
            CacheLevel::Connection.default_ttl(),
            Duration::from_secs(30 * 60)
        );
        assert_eq!(
            CacheLevel::Database.default_ttl(),
            Duration::from_secs(15 * 60)
        );
        assert_eq!(
            CacheLevel::Table.default_ttl(),
            Duration::from_secs(10 * 60)
        );
        assert_eq!(
            CacheLevel::Detail.default_ttl(),
            Duration::from_secs(5 * 60)
        );
    }

    #[test]
    fn test_cached_entry_expiration() {
        let entry = CachedEntry::with_ttl(
            MetadataEntry::Databases(vec!["db1".to_string()]),
            Duration::from_millis(100),
        );

        assert!(!entry.is_expired());

        std::thread::sleep(Duration::from_millis(150));
        assert!(entry.is_expired());
    }

    #[tokio::test]
    async fn test_metadata_cache_basic() {
        let dir = tempdir().unwrap();
        let cache = MetadataCacheManager::new(dir.path().to_path_buf()).unwrap();

        let key = CacheKey::databases("conn1");
        let data = MetadataEntry::Databases(vec!["db1".to_string(), "db2".to_string()]);

        // 初始应该为空
        assert!(cache.get(&key).await.is_none());

        // 写入缓存
        cache.set(&key, data.clone(), CacheLevel::Connection).await;

        // 应该能够读取
        let cached = cache.get(&key).await;
        assert!(cached.is_some());

        if let Some(MetadataEntry::Databases(dbs)) = cached {
            assert_eq!(dbs.len(), 2);
            assert_eq!(dbs[0], "db1");
        } else {
            panic!("Expected Databases entry");
        }
    }

    #[tokio::test]
    async fn test_metadata_cache_invalidation() {
        let dir = tempdir().unwrap();
        let cache = MetadataCacheManager::new(dir.path().to_path_buf()).unwrap();

        let key = CacheKey::tables("conn1", "mydb", None);
        let data = MetadataEntry::Tables(vec![]);

        cache.set(&key, data, CacheLevel::Database).await;
        assert!(cache.get(&key).await.is_some());

        cache.invalidate(&key).await;
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_metadata_cache_connection_invalidation() {
        let dir = tempdir().unwrap();
        let cache = MetadataCacheManager::new(dir.path().to_path_buf()).unwrap();

        let key1 = CacheKey::databases("conn1");
        let key2 = CacheKey::tables("conn1", "db1", None);
        let key3 = CacheKey::databases("conn2");

        cache
            .set(
                &key1,
                MetadataEntry::Databases(vec![]),
                CacheLevel::Connection,
            )
            .await;
        cache
            .set(&key2, MetadataEntry::Tables(vec![]), CacheLevel::Database)
            .await;
        cache
            .set(
                &key3,
                MetadataEntry::Databases(vec![]),
                CacheLevel::Connection,
            )
            .await;

        // 失效 conn1
        cache.invalidate_connection("conn1").await;

        // conn1 的缓存应该被清除
        assert!(cache.get(&key1).await.is_none());
        assert!(cache.get(&key2).await.is_none());

        // conn2 的缓存应该保留
        assert!(cache.get(&key3).await.is_some());
    }
}
