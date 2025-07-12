///记忆体，实现会话级记忆功能，当会话结束触发会话总结归纳后长期保存，同时提供记忆检索功能
use super::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

/// 基于内存的记忆体实现 - 适合测试和临时使用
pub struct InMemoryMemory {
    short_term: Arc<RwLock<HashMap<String, String>>>,
    long_term: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryMemory {
    pub fn new() -> Self {
        Self {
            short_term: Arc::new(RwLock::new(HashMap::new())),
            long_term: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取指定类型的存储
    fn get_storage(&self, memory_type: MemoryType) -> &Arc<RwLock<HashMap<String, String>>> {
        match memory_type {
            MemoryType::ShortTerm => &self.short_term,
            MemoryType::LongTerm => &self.long_term,
        }
    }

    /// 搜索匹配的记忆条目
    fn search_in_storage(
        storage: &HashMap<String, String>,
        query: &str,
        memory_type: MemoryType,
    ) -> Vec<MemoryEntry> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (key, value) in storage {
            // 简单的关键词匹配
            if key.to_lowercase().contains(&query_lower)
                || value.to_lowercase().contains(&query_lower)
            {
                results.push(MemoryEntry {
                    key: key.clone(),
                    value: value.clone(),
                    memory_type,
                    timestamp: Some(chrono::Utc::now().timestamp() as u64),
                });
            }
        }

        results
    }
}

impl Memory for InMemoryMemory {
    async fn store(
        &self, // 改为 &self
        key: &str,
        value: &str,
        memory_type: MemoryType,
    ) -> anyhow::Result<()> {
        let storage = self.get_storage(memory_type);
        let mut storage_guard = storage.write().await;
        storage_guard.insert(key.to_string(), value.to_string());

        tracing::debug!(
            "Stored memory: {} -> {} (type: {:?})",
            key,
            value,
            memory_type
        );
        Ok(())
    }

    async fn get(&self, key: &str, memory_type: MemoryType) -> anyhow::Result<Option<String>> {
        let storage = self.get_storage(memory_type);
        let storage_guard = storage.read().await;
        Ok(storage_guard.get(key).cloned())
    }

    async fn clear(&self, memory_type: MemoryType) -> anyhow::Result<()> {
        // 改为 &self
        let storage = self.get_storage(memory_type);
        let mut storage_guard = storage.write().await;
        storage_guard.clear();

        tracing::info!("Cleared {:?} memory", memory_type);
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        memory_type: Option<MemoryType>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let mut results = Vec::new();

        match memory_type {
            Some(mem_type) => {
                let storage = self.get_storage(mem_type);
                let storage_guard = storage.read().await;
                results.extend(Self::search_in_storage(&storage_guard, query, mem_type));
            }
            None => {
                // 搜索所有类型
                let short_term_guard = self.short_term.read().await;
                let long_term_guard = self.long_term.read().await;

                results.extend(Self::search_in_storage(
                    &short_term_guard,
                    query,
                    MemoryType::ShortTerm,
                ));
                results.extend(Self::search_in_storage(
                    &long_term_guard,
                    query,
                    MemoryType::LongTerm,
                ));
            }
        }

        Ok(results)
    }

    async fn list_keys(&self, memory_type: MemoryType) -> anyhow::Result<Vec<String>> {
        let storage = self.get_storage(memory_type);
        let storage_guard = storage.read().await;
        Ok(storage_guard.keys().cloned().collect())
    }
}

/// 基于文件的持久化记忆体实现 - 适合生产环境
pub struct FileBasedMemory {
    base_path: PathBuf,
    short_term_cache: Arc<RwLock<HashMap<String, String>>>,
    long_term_cache: Arc<RwLock<HashMap<String, String>>>,
    cache_dirty: Arc<RwLock<bool>>,
}

impl FileBasedMemory {
    pub async fn new(base_path: PathBuf) -> anyhow::Result<Self> {
        // 确保目录存在
        fs::create_dir_all(&base_path).await?;

        let mut memory = Self {
            base_path,
            short_term_cache: Arc::new(RwLock::new(HashMap::new())),
            long_term_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_dirty: Arc::new(RwLock::new(false)),
        };

        // 加载现有数据
        memory.load_from_disk().await?;

        Ok(memory)
    }

    /// 获取存储文件路径
    fn get_file_path(&self, memory_type: MemoryType) -> PathBuf {
        match memory_type {
            MemoryType::ShortTerm => self.base_path.join("short_term_memory.json"),
            MemoryType::LongTerm => self.base_path.join("long_term_memory.json"),
        }
    }

    /// 获取指定类型的缓存
    fn get_cache(&self, memory_type: MemoryType) -> &Arc<RwLock<HashMap<String, String>>> {
        match memory_type {
            MemoryType::ShortTerm => &self.short_term_cache,
            MemoryType::LongTerm => &self.long_term_cache,
        }
    }

    /// 从磁盘加载数据
    async fn load_from_disk(&mut self) -> anyhow::Result<()> {
        self.load_memory_type(MemoryType::ShortTerm).await?;
        self.load_memory_type(MemoryType::LongTerm).await?;
        Ok(())
    }

    /// 加载指定类型的记忆
    async fn load_memory_type(&mut self, memory_type: MemoryType) -> anyhow::Result<()> {
        let file_path = self.get_file_path(memory_type);

        if !file_path.exists() {
            return Ok(());
        }

        let mut file = fs::File::open(&file_path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        if contents.is_empty() {
            return Ok(());
        }

        let data: HashMap<String, String> = serde_json::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse memory file: {}", e))?;

        let cache = self.get_cache(memory_type);
        let mut cache_guard = cache.write().await;
        *cache_guard = data;

        tracing::info!(
            "Loaded {:?} memory from disk: {} entries",
            memory_type,
            cache_guard.len()
        );
        Ok(())
    }

    /// 保存到磁盘
    async fn save_to_disk(&self) -> anyhow::Result<()> {
        self.save_memory_type(MemoryType::ShortTerm).await?;
        self.save_memory_type(MemoryType::LongTerm).await?;

        // 标记为已保存
        let mut dirty = self.cache_dirty.write().await;
        *dirty = false;

        Ok(())
    }

    /// 保存指定类型的记忆
    async fn save_memory_type(&self, memory_type: MemoryType) -> anyhow::Result<()> {
        let file_path = self.get_file_path(memory_type);
        let cache = self.get_cache(memory_type);
        let cache_guard = cache.read().await;

        let json_data = serde_json::to_string_pretty(&*cache_guard)
            .map_err(|e| anyhow::anyhow!("Failed to serialize memory: {}", e))?;

        let mut file = fs::File::create(&file_path).await?;
        file.write_all(json_data.as_bytes()).await?;
        file.flush().await?;

        tracing::debug!(
            "Saved {:?} memory to disk: {} entries",
            memory_type,
            cache_guard.len()
        );
        Ok(())
    }

    /// 标记缓存为脏
    async fn mark_dirty(&self) {
        let mut dirty = self.cache_dirty.write().await;
        *dirty = true;
    }

    /// 定期保存（应该由外部定时器调用）
    pub async fn periodic_save(&self) -> anyhow::Result<()> {
        let dirty = {
            let dirty_guard = self.cache_dirty.read().await;
            *dirty_guard
        };

        if dirty {
            self.save_to_disk().await?;
        }

        Ok(())
    }

    /// 搜索匹配的记忆条目
    fn search_in_cache(
        cache: &HashMap<String, String>,
        query: &str,
        memory_type: MemoryType,
    ) -> Vec<MemoryEntry> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (key, value) in cache {
            // 更复杂的搜索逻辑
            let key_lower = key.to_lowercase();
            let value_lower = value.to_lowercase();

            // 精确匹配得分更高
            let mut score = 0.0;
            if key_lower.contains(&query_lower) {
                score += 1.0;
            }
            if value_lower.contains(&query_lower) {
                score += 0.5;
            }

            // 词汇匹配
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
            for word in query_words {
                if key_lower.contains(word) || value_lower.contains(word) {
                    score += 0.2;
                }
            }

            if score > 0.0 {
                results.push(MemoryEntry {
                    key: key.clone(),
                    value: value.clone(),
                    memory_type,
                    timestamp: Some(chrono::Utc::now().timestamp() as u64),
                });
            }
        }

        // 按相关性排序（简单实现）
        results.sort_by(|a, b| {
            let score_a = if a.key.to_lowercase().contains(&query_lower) {
                1.0
            } else {
                0.5
            };
            let score_b = if b.key.to_lowercase().contains(&query_lower) {
                1.0
            } else {
                0.5
            };
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }
}

impl Memory for FileBasedMemory {
    async fn store(
        &self, // 改为 &self
        key: &str,
        value: &str,
        memory_type: MemoryType,
    ) -> anyhow::Result<()> {
        let cache = self.get_cache(memory_type);
        let mut cache_guard = cache.write().await;
        cache_guard.insert(key.to_string(), value.to_string());

        // 标记为需要保存
        self.mark_dirty().await;

        tracing::debug!(
            "Stored memory: {} -> {} (type: {:?})",
            key,
            value,
            memory_type
        );
        Ok(())
    }

    async fn get(&self, key: &str, memory_type: MemoryType) -> anyhow::Result<Option<String>> {
        let cache = self.get_cache(memory_type);
        let cache_guard = cache.read().await;
        Ok(cache_guard.get(key).cloned())
    }

    async fn clear(&self, memory_type: MemoryType) -> anyhow::Result<()> {
        // 改为 &self
        let cache = self.get_cache(memory_type);
        let mut cache_guard = cache.write().await;
        cache_guard.clear();

        // 立即保存清空状态
        drop(cache_guard);
        self.save_memory_type(memory_type).await?;

        tracing::info!("Cleared {:?} memory", memory_type);
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        memory_type: Option<MemoryType>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let mut results = Vec::new();

        match memory_type {
            Some(mem_type) => {
                let cache = self.get_cache(mem_type);
                let cache_guard = cache.read().await;
                results.extend(Self::search_in_cache(&cache_guard, query, mem_type));
            }
            None => {
                // 搜索所有类型
                let short_term_guard = self.short_term_cache.read().await;
                let long_term_guard = self.long_term_cache.read().await;

                results.extend(Self::search_in_cache(
                    &short_term_guard,
                    query,
                    MemoryType::ShortTerm,
                ));
                results.extend(Self::search_in_cache(
                    &long_term_guard,
                    query,
                    MemoryType::LongTerm,
                ));
            }
        }

        Ok(results)
    }

    async fn list_keys(&self, memory_type: MemoryType) -> anyhow::Result<Vec<String>> {
        let cache = self.get_cache(memory_type);
        let cache_guard = cache.read().await;
        Ok(cache_guard.keys().cloned().collect())
    }
}

impl Drop for FileBasedMemory {
    fn drop(&mut self) {
        // 注意：这里不能使用 async，但我们可以尝试同步保存
        // 在实际应用中，应该确保在程序退出前调用 save_to_disk
        tracing::warn!("FileBasedMemory dropped, some data might not be saved");
    }
}

/// 高级记忆体实现 - 支持会话管理和智能总结
pub struct AdvancedMemory {
    base_memory: FileBasedMemory,
    session_memories: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
    session_summaries: Arc<RwLock<HashMap<String, String>>>,
}

impl AdvancedMemory {
    pub async fn new(base_path: PathBuf) -> anyhow::Result<Self> {
        let base_memory = FileBasedMemory::new(base_path).await?;

        Ok(Self {
            base_memory,
            session_memories: Arc::new(RwLock::new(HashMap::new())),
            session_summaries: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 开始新会话
    pub async fn start_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.session_memories.write().await;
        sessions.insert(session_id.to_string(), HashMap::new());

        tracing::info!("Started new memory session: {}", session_id);
        Ok(())
    }

    /// 结束会话并生成总结
    pub async fn end_session(&self, session_id: &str) -> anyhow::Result<Option<String>> {
        // 改为 &self
        let session_data = {
            let mut sessions = self.session_memories.write().await;
            sessions.remove(session_id)
        };

        if let Some(data) = session_data {
            if !data.is_empty() {
                // 生成会话总结
                let summary = self.generate_session_summary(&data).await?;

                // 保存总结到长期记忆
                let summary_key = format!("session_summary_{}", session_id);
                self.base_memory
                    .store(&summary_key, &summary, MemoryType::LongTerm)
                    .await?;

                // 缓存总结
                let mut summaries = self.session_summaries.write().await;
                summaries.insert(session_id.to_string(), summary.clone());

                tracing::info!("Ended session {} with summary", session_id);
                return Ok(Some(summary));
            }
        }

        Ok(None)
    }

    /// 存储会话记忆
    pub async fn store_session_memory(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let mut sessions = self.session_memories.write().await;
        if let Some(session_data) = sessions.get_mut(session_id) {
            session_data.insert(key.to_string(), value.to_string());
        }
        Ok(())
    }

    /// 获取会话记忆
    pub async fn get_session_memory(
        &self,
        session_id: &str,
        key: &str,
    ) -> anyhow::Result<Option<String>> {
        let sessions = self.session_memories.read().await;
        Ok(sessions
            .get(session_id)
            .and_then(|session| session.get(key).cloned()))
    }

    /// 生成会话总结（这里是简化版本，实际应该使用LLM）
    async fn generate_session_summary(
        &self,
        session_data: &HashMap<String, String>,
    ) -> anyhow::Result<String> {
        let mut summary = String::new();
        summary.push_str("会话总结:\n");

        // 提取关键信息
        let mut key_interactions = Vec::new();
        for (key, value) in session_data {
            if key.contains("input") || key.contains("important") {
                key_interactions.push(format!("- {}: {}", key, value));
            }
        }

        if key_interactions.is_empty() {
            summary.push_str("没有重要交互记录");
        } else {
            summary.push_str("关键交互:\n");
            for interaction in key_interactions.iter().take(10) {
                summary.push_str(interaction);
                summary.push('\n');
            }
        }

        summary.push_str(&format!("总交互数: {}\n", session_data.len()));
        summary.push_str(&format!(
            "会话时间: {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));

        Ok(summary)
    }

    /// 获取会话总结
    pub async fn get_session_summary(&self, session_id: &str) -> anyhow::Result<Option<String>> {
        let summaries = self.session_summaries.read().await;
        Ok(summaries.get(session_id).cloned())
    }

    /// 定期保存
    pub async fn periodic_save(&self) -> anyhow::Result<()> {
        self.base_memory.periodic_save().await
    }
}

impl Memory for AdvancedMemory {
    async fn store(
        &self, // 改为 &self
        key: &str,
        value: &str,
        memory_type: MemoryType,
    ) -> anyhow::Result<()> {
        self.base_memory.store(key, value, memory_type).await
    }

    async fn get(&self, key: &str, memory_type: MemoryType) -> anyhow::Result<Option<String>> {
        self.base_memory.get(key, memory_type).await
    }

    async fn clear(&self, memory_type: MemoryType) -> anyhow::Result<()> {
        // 改为 &self
        self.base_memory.clear(memory_type).await
    }

    async fn search(
        &self,
        query: &str,
        memory_type: Option<MemoryType>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.base_memory.search(query, memory_type).await
    }

    async fn list_keys(&self, memory_type: MemoryType) -> anyhow::Result<Vec<String>> {
        self.base_memory.list_keys(memory_type).await
    }
}

/// 记忆体配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// 存储类型
    pub storage_type: StorageType,
    /// 基础路径（用于文件存储）
    pub base_path: Option<PathBuf>,
    /// 自动保存间隔（秒）
    pub auto_save_interval: Option<u64>,
    /// 是否启用会话管理
    pub session_management: bool,
    /// 短期记忆大小限制（条数）
    pub short_term_limit: Option<usize>,
    /// 长期记忆大小限制（条数）
    pub long_term_limit: Option<usize>,
    /// 搜索配置
    pub search_config: SearchConfig,
}

/// 存储类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    /// 内存存储
    InMemory,
    /// 文件存储
    File,
    /// 高级存储（包含会话管理）
    Advanced,
}

/// 搜索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 是否启用模糊搜索
    pub fuzzy_search: bool,
    /// 最大搜索结果数
    pub max_results: usize,
    /// 搜索相关性阈值
    pub relevance_threshold: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::InMemory,
            base_path: None,
            auto_save_interval: Some(300), // 5分钟
            session_management: false,
            short_term_limit: Some(1000),
            long_term_limit: Some(10000),
            search_config: SearchConfig::default(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            fuzzy_search: true,
            max_results: 50,
            relevance_threshold: 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let mut memory = InMemoryMemory::new();

        // 测试存储和检索
        memory
            .store("test_key", "test_value", MemoryType::ShortTerm)
            .await
            .unwrap();
        let result = memory.get("test_key", MemoryType::ShortTerm).await.unwrap();
        assert_eq!(result, Some("test_value".to_string()));

        // 测试搜索
        let results = memory
            .search("test", Some(MemoryType::ShortTerm))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "test_key");
        assert_eq!(results[0].value, "test_value");
    }

    #[tokio::test]
    async fn test_file_based_storage() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = FileBasedMemory::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // 测试存储和检索
        memory
            .store("file_key", "file_value", MemoryType::LongTerm)
            .await
            .unwrap();
        let result = memory.get("file_key", MemoryType::LongTerm).await.unwrap();
        assert_eq!(result, Some("file_value".to_string()));

        // 测试持久化
        memory.save_to_disk().await.unwrap();

        // 重新加载
        let memory2 = FileBasedMemory::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();
        let result2 = memory2.get("file_key", MemoryType::LongTerm).await.unwrap();
        assert_eq!(result2, Some("file_value".to_string()));
    }

    #[tokio::test]
    async fn test_advanced_memory_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = AdvancedMemory::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // 测试会话管理
        memory.start_session("session1").await.unwrap();
        memory
            .store_session_memory("session1", "key1", "value1")
            .await
            .unwrap();

        let session_value = memory.get_session_memory("session1", "key1").await.unwrap();
        assert_eq!(session_value, Some("value1".to_string()));

        // 测试会话结束
        let summary = memory.end_session("session1").await.unwrap();
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("会话总结"));
    }

    #[tokio::test]
    async fn test_memory_search() {
        let mut memory = InMemoryMemory::new();

        // 添加测试数据
        memory
            .store("user_profile", "name: John, age: 30", MemoryType::LongTerm)
            .await
            .unwrap();
        memory
            .store("task_1", "complete project proposal", MemoryType::ShortTerm)
            .await
            .unwrap();
        memory
            .store(
                "note_1",
                "meeting with John tomorrow",
                MemoryType::ShortTerm,
            )
            .await
            .unwrap();

        // 搜索测试
        let results = memory.search("John", None).await.unwrap();
        assert_eq!(results.len(), 2);

        let project_results = memory.search("project", None).await.unwrap();
        assert_eq!(project_results.len(), 1);
        assert_eq!(project_results[0].key, "task_1");
    }

    #[tokio::test]
    async fn test_memory_clear() {
        let mut memory = InMemoryMemory::new();

        // 添加测试数据
        memory
            .store("short1", "value1", MemoryType::ShortTerm)
            .await
            .unwrap();
        memory
            .store("short2", "value2", MemoryType::ShortTerm)
            .await
            .unwrap();
        memory
            .store("long1", "value3", MemoryType::LongTerm)
            .await
            .unwrap();

        // 清除短期记忆
        memory.clear(MemoryType::ShortTerm).await.unwrap();

        // 验证清除结果
        let short_result = memory.get("short1", MemoryType::ShortTerm).await.unwrap();
        assert_eq!(short_result, None);

        let long_result = memory.get("long1", MemoryType::LongTerm).await.unwrap();
        assert_eq!(long_result, Some("value3".to_string()));
    }
}
