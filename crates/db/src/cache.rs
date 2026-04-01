use anyhow::Result;
use moka::sync::Cache;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::types::DbNode;
use one_core::storage::{DatabaseType, DbConnectionConfig};

/// 缓存上下文，用于生成缓存路径
#[derive(Clone, Debug)]
pub struct CacheContext {
    pub database_type: DatabaseType,
    pub host: String,
    pub port: u16,
}

impl CacheContext {
    pub fn new(database_type: DatabaseType, host: String, port: u16) -> Self {
        Self {
            database_type,
            host,
            port,
        }
    }

    /// 从连接配置创建缓存上下文
    pub fn from_config(config: &DbConnectionConfig) -> Self {
        Self {
            database_type: config.database_type,
            host: config.host.clone(),
            port: config.port,
        }
    }

    /// 计算字符串的短 hash（8位十六进制）
    fn short_hash(s: &str) -> String {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        format!("{:08x}", hasher.finish() as u32)
    }

    /// 生成缓存目录名
    /// - 网络数据库: {database_type}/{host}_{port}
    /// - SQLite / DuckDB: {database_type}/{db_name}_{path_hash}
    pub fn cache_dir_name(&self) -> String {
        if matches!(
            self.database_type,
            DatabaseType::SQLite | DatabaseType::DuckDB
        ) || self.port == 0
        {
            let path = std::path::Path::new(&self.host);
            let db_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let safe_name = db_name.replace([':', '/', '\\', '<', '>', '|', '?', '*', '.'], "_");
            let hash = Self::short_hash(&self.host);
            format!("{}/{}_{}", self.database_type.as_str(), safe_name, hash)
        } else {
            let safe_host = self
                .host
                .replace([':', '/', '\\', '<', '>', '|', '?', '*', '.'], "_");
            format!(
                "{}/{}_{}",
                self.database_type.as_str(),
                safe_host,
                self.port
            )
        }
    }

    /// 生成缓存键前缀
    pub fn cache_key_prefix(&self) -> String {
        if matches!(
            self.database_type,
            DatabaseType::SQLite | DatabaseType::DuckDB
        ) || self.port == 0
        {
            let path = std::path::Path::new(&self.host);
            let db_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let safe_name = db_name.replace([':', '/', '\\', '<', '>', '|', '?', '*', '.'], "_");
            let hash = Self::short_hash(&self.host);
            format!(
                "{}_{}_{}",
                self.database_type.as_str().to_ascii_lowercase(),
                safe_name,
                hash
            )
        } else {
            let safe_host = self
                .host
                .replace([':', '/', '\\', '<', '>', '|', '?', '*', '.'], "_");
            format!(
                "{}_{}_{}",
                self.database_type.as_str().to_lowercase(),
                safe_host,
                self.port
            )
        }
    }
}

/// 缓存的节点数据
#[derive(Clone, Serialize, Deserialize)]
struct CachedNode {
    node: DbNode,
    cached_at: std::time::SystemTime,
    ttl_millis: u64,
}

impl CachedNode {
    fn new(node: DbNode, ttl: Duration) -> Self {
        Self {
            node,
            cached_at: std::time::SystemTime::now(),
            ttl_millis: ttl.as_millis() as u64,
        }
    }

    fn is_expired(&self) -> bool {
        match self.cached_at.elapsed() {
            Ok(elapsed) => elapsed.as_millis() as u64 > self.ttl_millis,
            Err(_) => true,
        }
    }
}

/// 节点缓存管理器
pub struct NodeCache {
    cache_dir: PathBuf,
    memory_cache: Cache<String, CachedNode>,
    default_ttl: Duration,
}

impl NodeCache {
    /// 创建新的缓存实例
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;

        let memory_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_idle(Duration::from_secs(120))
            .build();

        Ok(Self {
            cache_dir,
            memory_cache,
            default_ttl: Duration::from_secs(120),
        })
    }

    /// 设置默认TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// 生成缓存键
    pub fn cache_key(&self, ctx: &CacheContext, node_id: &str) -> String {
        let safe_node_id = node_id.replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");
        format!("{}_{}", ctx.cache_key_prefix(), safe_node_id)
    }

    /// 获取缓存文件路径
    pub fn cache_path(&self, ctx: &CacheContext, node_id: &str) -> PathBuf {
        let safe_node_id = node_id.replace([':', '/', '\\', '<', '>', '|', '?', '*'], "_");
        self.cache_dir
            .join(ctx.cache_dir_name())
            .join(format!("{}.json", safe_node_id))
    }

    /// 获取缓存目录
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// 从内存缓存获取节点
    fn get_from_memory(&self, ctx: &CacheContext, node_id: &str) -> Option<DbNode> {
        let key = self.cache_key(ctx, node_id);
        if let Some(cached) = self.memory_cache.get(&key) {
            if cached.is_expired() {
                debug!("Memory cache expired for node: {}", node_id);
                self.memory_cache.invalidate(&key);
                None
            } else {
                debug!("Memory cache hit for node: {}", node_id);
                Some(cached.node.clone())
            }
        } else {
            None
        }
    }

    /// 从文件缓存加载节点
    async fn load_from_file(&self, ctx: &CacheContext, node_id: &str) -> Result<Option<DbNode>> {
        let path = self.cache_path(ctx, node_id);

        if !path.exists() {
            debug!("File cache miss for node: {}", node_id);
            return Ok(None);
        }

        match fs::read_to_string(&path).await {
            Ok(content) => match serde_json::from_str::<CachedNode>(&content) {
                Ok(cached) => {
                    if cached.is_expired() {
                        debug!("File cache expired for node: {}", node_id);
                        if let Err(e) = fs::remove_file(&path).await {
                            warn!(
                                "Failed to remove expired cache file {}: {}",
                                path.display(),
                                e
                            );
                        }
                        Ok(None)
                    } else {
                        debug!("File cache hit for node: {}", node_id);
                        let key = self.cache_key(ctx, node_id);
                        self.memory_cache.insert(key, cached.clone());
                        Ok(Some(cached.node))
                    }
                }
                Err(e) => {
                    warn!("Failed to deserialize cache file {}: {}", path.display(), e);
                    let _ = fs::remove_file(&path).await;
                    Ok(None)
                }
            },
            Err(e) => {
                debug!("Failed to read cache file {}: {}", path.display(), e);
                Ok(None)
            }
        }
    }

    /// 保存节点到缓存
    async fn save_to_cache(&self, ctx: &CacheContext, node_id: &str, node: &DbNode) -> Result<()> {
        let cached = CachedNode::new(node.clone(), self.default_ttl);
        let key = self.cache_key(ctx, node_id);

        self.memory_cache.insert(key, cached.clone());

        let path = self.cache_path(ctx, node_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&cached)?;
        fs::write(&path, content).await?;

        debug!("Cached node {} to {}", node_id, path.display());
        Ok(())
    }

    /// 获取节点（优先从缓存）
    pub async fn get_node(&self, ctx: &CacheContext, node_id: &str) -> Option<DbNode> {
        if let Some(node) = self.get_from_memory(ctx, node_id) {
            return Some(node);
        }

        match self.load_from_file(ctx, node_id).await {
            Ok(Some(node)) => Some(node),
            Ok(None) => None,
            Err(e) => {
                error!("Cache error for node {}: {}", node_id, e);
                None
            }
        }
    }

    /// 缓存节点数据
    pub async fn cache_node(&self, ctx: &CacheContext, node_id: &str, node: &DbNode) {
        if let Err(e) = self.save_to_cache(ctx, node_id, node).await {
            error!("Failed to cache node {}: {}", node_id, e);
        }
    }

    /// 使指定节点的缓存失效
    pub async fn invalidate_node(&self, ctx: &CacheContext, node_id: &str) {
        let key = self.cache_key(ctx, node_id);
        self.memory_cache.invalidate(&key);

        let path = self.cache_path(ctx, node_id);
        if path.exists() {
            if let Err(e) = fs::remove_file(&path).await {
                warn!("Failed to remove cache file {}: {}", path.display(), e);
            } else {
                debug!("Invalidated cache for node: {}", node_id);
            }
        }
    }

    /// 递归使节点及其所有后代的缓存失效
    pub async fn invalidate_node_recursive(&self, ctx: &CacheContext, node_id: &str) {
        if let Some(node) = self.get_node(ctx, node_id).await {
            for child in &node.children {
                Box::pin(self.invalidate_node_recursive(ctx, &child.id)).await;
            }
        }
        self.invalidate_node(ctx, node_id).await;
    }

    /// 清除指定连接的所有缓存
    pub async fn clear_connection_cache(&self, ctx: &CacheContext) {
        let prefix = format!("{}_", ctx.cache_key_prefix());
        // moka 没有 retain，需要收集匹配的 key 后逐个 invalidate
        let keys_to_remove: Vec<Arc<String>> = self
            .memory_cache
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, _)| k)
            .collect();
        for key in keys_to_remove {
            self.memory_cache.invalidate(&*key);
        }

        let connection_dir = self.cache_dir.join(ctx.cache_dir_name());
        if connection_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&connection_dir).await {
                warn!(
                    "Failed to remove connection cache directory {}: {}",
                    connection_dir.display(),
                    e
                );
            } else {
                info!("Cleared all cache for connection: {:?}", ctx);
            }
        }
    }

    /// 清除所有缓存
    pub async fn clear_all(&self) {
        self.memory_cache.invalidate_all();

        if self.cache_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&self.cache_dir).await {
                error!(
                    "Failed to clear cache directory {}: {}",
                    self.cache_dir.display(),
                    e
                );
            } else {
                if let Err(e) = fs::create_dir_all(&self.cache_dir).await {
                    error!("Failed to recreate cache directory: {}", e);
                }
                info!("Cleared all cache");
            }
        }
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            memory_entries: self.memory_cache.entry_count() as usize,
            cache_dir: self.cache_dir.clone(),
        }
    }

    #[cfg(test)]
    pub fn clear_memory_cache(&self) {
        self.memory_cache.invalidate_all();
    }
}

/// 缓存统计信息
#[derive(Debug)]
pub struct CacheStats {
    pub memory_entries: usize,
    pub cache_dir: PathBuf,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeCache Stats: {} memory entries, cache dir: {}",
            self.memory_entries,
            self.cache_dir.display()
        )
    }
}
