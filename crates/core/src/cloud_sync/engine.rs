//! 云同步引擎
//!
//! 统一管理同步流程，将同步逻辑从 UI 层解耦。
//!
//! ## 设计原则
//!
//! - 参考 Dropbox Nucleus 架构的三棵树模型
//! - 支持冲突检测和多种解决策略
//! - 提供完整同步和增量同步两种模式

use super::connection_sync::ConnectionSyncHandler;
use super::generic_sync::generic_sync;
use super::sync_type::SyncTypeHandler;
use super::workspace_sync::WorkspaceSyncType;
use crate::cloud_sync::client::CloudApiClient;
use crate::cloud_sync::models::{ConflictResolution, SyncResult, Team};
use crate::cloud_sync::queue::OperationQueue;
use crate::cloud_sync::service::{CloudSyncService, SyncError};
use crate::crypto;
use crate::storage::{StorageManager, TeamKeyCacheRepository};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub type SyncFuture<'a> = Pin<Box<dyn Future<Output = Result<SyncResult, SyncError>> + Send + 'a>>;

pub trait SyncHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn sync<'a>(&'a self, engine: &'a SyncEngine) -> SyncFuture<'a>;
}

/// 泛型桥接器：将 `SyncTypeHandler` 适配为 `SyncHandler`
///
/// 通过 `generic_sync` 通用流程执行同步，使新数据类型只需实现
/// `SyncTypeHandler` trait 即可接入同步引擎。
pub struct TypedSyncBridge<H: SyncTypeHandler> {
    handler: H,
}

impl<H: SyncTypeHandler> SyncHandler for TypedSyncBridge<H> {
    fn name(&self) -> &'static str {
        self.handler.display_name()
    }

    fn sync<'a>(&'a self, engine: &'a SyncEngine) -> SyncFuture<'a> {
        Box::pin(generic_sync(engine, &self.handler))
    }
}

/// 同步引擎
///
/// 核心职责：
/// 1. 协调本地存储和云端 API 的交互
/// 2. 计算同步计划，检测冲突
/// 3. 执行同步操作并更新状态
pub struct SyncEngine {
    /// 云端 API 客户端
    pub(crate) cloud_client: Arc<dyn CloudApiClient>,
    /// 加解密服务
    pub(crate) crypto_service: Arc<std::sync::RwLock<CloudSyncService>>,
    /// 本地存储管理器
    pub(crate) storage: StorageManager,
    /// 冲突解决策略
    pub(crate) conflict_strategy: ConflictResolution,
    handlers: Vec<Box<dyn SyncHandler>>,
    /// 当前用户所在团队列表（同步开始时获取）
    pub(crate) cached_teams: std::sync::RwLock<Vec<Team>>,
}

impl SyncEngine {
    /// 创建新的同步引擎
    pub fn new(
        cloud_client: Arc<dyn CloudApiClient>,
        crypto_service: Arc<std::sync::RwLock<CloudSyncService>>,
        storage: StorageManager,
    ) -> Self {
        Self {
            cloud_client,
            crypto_service,
            storage,
            conflict_strategy: ConflictResolution::UseCloud, // 默认使用云端版本
            handlers: vec![
                Box::new(TypedSyncBridge {
                    handler: WorkspaceSyncType,
                }),
                Box::new(ConnectionSyncHandler),
            ],
            cached_teams: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// 设置冲突解决策略
    pub fn with_conflict_strategy(mut self, strategy: ConflictResolution) -> Self {
        self.conflict_strategy = strategy;
        self
    }

    pub fn register_handler(&mut self, handler: Box<dyn SyncHandler>) {
        self.handlers.push(handler);
    }

    /// 注册一个类型化同步处理器
    ///
    /// 通过 `TypedSyncBridge` 适配为 `SyncHandler`，自动接入 `generic_sync` 通用流程。
    pub fn register_type<H: SyncTypeHandler>(mut self, handler: H) -> Self {
        self.handlers.push(Box::new(TypedSyncBridge { handler }));
        self
    }

    /// 获取当前时间戳（秒）
    pub(crate) fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// 确保加密服务已解锁
    fn ensure_unlocked(&self) -> Result<(), SyncError> {
        // 如果本地 crypto 模块已解锁但同步服务未解锁，同步密钥状态
        if crypto::has_master_key() {
            if let Some(raw_key) = crypto::get_raw_master_key() {
                let mut service_write = self
                    .crypto_service
                    .write()
                    .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;
                if !service_write.is_unlocked() {
                    tracing::info!("[同步引擎] 从本地 crypto 模块同步密钥状态");
                    service_write.set_master_key_directly(raw_key);
                }
            }
        }

        let service = self
            .crypto_service
            .read()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

        if !service.is_unlocked() {
            return Err(SyncError::NotUnlocked);
        }

        Ok(())
    }

    /// 执行完整同步
    ///
    /// ## 同步流程
    /// 1. 获取团队列表并缓存
    /// 2. 先同步工作空间（无外键依赖）
    /// 3. 再同步连接（依赖工作空间）
    pub async fn sync(&self) -> Result<SyncResult, SyncError> {
        tracing::info!("========== 开始云同步 ==========");

        self.ensure_unlocked()?;

        // 获取并缓存团队列表
        match self.cloud_client.list_teams().await {
            Ok(teams) => {
                tracing::info!("[同步] 获取到 {} 个团队", teams.len());

                // 获取当前用户 ID
                let user_id = self
                    .crypto_service
                    .read()
                    .ok()
                    .and_then(|s| s.user_id().map(|id| id.to_string()));

                // 缓存团队角色信息到 team_key_cache
                if let Some(uid) = &user_id {
                    self.cache_team_roles(&teams, uid).await;
                }

                if let Ok(mut cache) = self.cached_teams.write() {
                    *cache = teams;
                }
            }
            Err(e) => {
                tracing::warn!("[同步] 获取团队列表失败: {}（将仅同步个人数据）", e);
            }
        }

        let mut result = SyncResult::default();

        for handler in &self.handlers {
            match handler.sync(self).await {
                Ok(sync_result) => {
                    result.uploaded += sync_result.uploaded;
                    result.downloaded += sync_result.downloaded;
                    result.deleted += sync_result.deleted;
                    result.conflicts.extend(sync_result.conflicts);
                    result.errors.extend(sync_result.errors);
                }
                Err(e) => {
                    tracing::error!("[同步] {}同步失败: {}", handler.name(), e);
                    result
                        .errors
                        .push(format!("{}同步失败: {}", handler.name(), e));
                }
            }
        }

        tracing::info!(
            "========== 同步完成: 上传 {} 个, 下载 {} 个, 错误 {} 个 ==========",
            result.uploaded,
            result.downloaded,
            result.errors.len()
        );

        Ok(result)
    }

    pub(crate) fn take_operation_queue(&self, key: &str) -> Result<OperationQueue, SyncError> {
        let mut service = self
            .crypto_service
            .write()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

        Ok(service.take_operation_queue(key))
    }

    pub(crate) fn store_operation_queue(
        &self,
        key: &str,
        queue: OperationQueue,
    ) -> Result<(), SyncError> {
        let mut service = self
            .crypto_service
            .write()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

        service.store_operation_queue(key, queue);
        Ok(())
    }

    /// 获取缓存的团队列表
    pub(crate) fn get_cached_teams(&self) -> Vec<Team> {
        self.cached_teams
            .read()
            .map(|teams| teams.clone())
            .unwrap_or_default()
    }

    /// 检查团队密钥是否已解锁
    pub(crate) fn is_team_unlocked(&self, team_id: &str) -> bool {
        self.crypto_service
            .read()
            .map(|service| service.is_team_unlocked(team_id))
            .unwrap_or(false)
    }

    /// 缓存团队角色信息到 team_key_cache 表
    async fn cache_team_roles(&self, teams: &[Team], user_id: &str) {
        let repo = match self.storage.get::<TeamKeyCacheRepository>() {
            Some(repo) => repo,
            None => return,
        };

        for team in teams {
            match self.cloud_client.list_team_members(&team.id).await {
                Ok(members) => {
                    if let Some(member) = members.iter().find(|m| m.user_id == user_id) {
                        let role_str = match member.role {
                            crate::cloud_sync::models::TeamRole::Owner => "owner",
                            crate::cloud_sync::models::TeamRole::Member => "member",
                        };
                        // 更新已有缓存的 role 字段
                        if let Ok(Some(mut cache)) = repo.get(&team.id) {
                            cache.role = Some(role_str.to_string());
                            if let Err(e) = repo.upsert(&cache) {
                                tracing::warn!(
                                    "[同步] 更新团队 {} 角色缓存失败: {}",
                                    team.id,
                                    e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "[同步] 获取团队 {} 成员列表失败: {}",
                        team.id,
                        e
                    );
                }
            }
        }
    }

    /// 使用指定的策略映射应用冲突解决方案
    ///
    /// 允许为每个冲突单独指定解决策略，而不是使用全局策略
    pub async fn apply_conflict_resolutions(
        &self,
        conflicts: Vec<crate::cloud_sync::models::SyncConflict>,
        strategies: std::collections::HashMap<String, ConflictResolution>,
    ) -> Result<SyncResult, SyncError> {
        self.ensure_unlocked()?;

        let mut result = SyncResult::default();
        result.conflicts = conflicts.clone();

        // 为每个冲突应用指定的策略
        for conflict in &conflicts {
            let cloud_id = &conflict.cloud.id;
            let strategy = strategies
                .get(cloud_id)
                .copied()
                .unwrap_or(self.conflict_strategy);

            let resolved_action = self.create_resolved_action(conflict, strategy);

            if let Err(e) = self.apply_single_conflict(&resolved_action).await {
                result.errors.push(format!("应用冲突解决失败: {}", e));
            }
        }

        Ok(result)
    }

    /// 创建冲突解决操作
    fn create_resolved_action(
        &self,
        conflict: &crate::cloud_sync::models::SyncConflict,
        strategy: ConflictResolution,
    ) -> crate::cloud_sync::connection_sync::ResolvedConflictAction {
        use crate::cloud_sync::connection_sync::ResolvedConflictAction;

        match strategy {
            ConflictResolution::UseCloud => ResolvedConflictAction {
                conflict: conflict.clone(),
                resolution: ConflictResolution::UseCloud,
                result_connection: None,
            },
            ConflictResolution::UseLocal => ResolvedConflictAction {
                conflict: conflict.clone(),
                resolution: ConflictResolution::UseLocal,
                result_connection: Some(conflict.local.clone()),
            },
            ConflictResolution::KeepBoth => {
                let mut copy = conflict.local.clone();
                copy.id = None;
                copy.cloud_id = None;
                let timestamp = Self::current_timestamp();
                copy.name = format!("{} (冲突副本 {})", copy.name, timestamp);

                ResolvedConflictAction {
                    conflict: conflict.clone(),
                    resolution: ConflictResolution::KeepBoth,
                    result_connection: Some(copy),
                }
            }
        }
    }

    /// 应用单个冲突解决方案
    async fn apply_single_conflict(
        &self,
        resolved: &crate::cloud_sync::connection_sync::ResolvedConflictAction,
    ) -> Result<(), SyncError> {
        use crate::storage::ConnectionRepository;
        use crate::storage::traits::Repository;

        match resolved.resolution {
            ConflictResolution::UseCloud => {
                // 更新本地连接
                let service = self
                    .crypto_service
                    .read()
                    .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

                let mut updated = service.decrypt_sync_data_connection(&resolved.conflict.cloud)?;
                updated.id = resolved.conflict.local.id;
                updated.cloud_id = Some(resolved.conflict.cloud.id.clone());
                updated.last_synced_at = Some(Self::current_timestamp());

                let repo = self
                    .storage
                    .get::<ConnectionRepository>()
                    .ok_or_else(|| {
                        SyncError::StorageError("ConnectionRepository not found".to_string())
                    })?;

                repo.update(&updated)
                    .map_err(|e| SyncError::StorageError(e.to_string()))?;

                Ok(())
            }
            ConflictResolution::UseLocal => {
                // 更新云端连接
                let teams = self.get_cached_teams();
                let updated_data = {
                    let service = self
                        .crypto_service
                        .read()
                        .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;
                    let mut data = service.prepare_sync_data_upload(
                        &resolved.conflict.local,
                        resolved.conflict.local.team_id.as_deref(),
                        &teams,
                    )?;
                    data.id = resolved.conflict.cloud.id.clone();
                    data.version = resolved.conflict.cloud.version;
                    data
                };

                self.cloud_client
                    .update_sync_data(&updated_data)
                    .await
                    .map_err(|e| SyncError::NetworkError(e.to_string()))?;

                Ok(())
            }
            ConflictResolution::KeepBoth => {
                // 创建本地副本
                if let Some(copy) = &resolved.result_connection {
                    let repo = self.storage.get::<ConnectionRepository>().ok_or_else(|| {
                        SyncError::StorageError("ConnectionRepository not found".to_string())
                    })?;

                    let mut new_conn = copy.clone();
                    repo.insert(&mut new_conn)
                        .map_err(|e| SyncError::StorageError(e.to_string()))?;
                }

                // 同时更新本地连接为云端版本
                let service = self
                    .crypto_service
                    .read()
                    .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

                let mut updated = service.decrypt_sync_data_connection(&resolved.conflict.cloud)?;
                updated.id = resolved.conflict.local.id;
                updated.cloud_id = Some(resolved.conflict.cloud.id.clone());
                updated.last_synced_at = Some(Self::current_timestamp());

                let repo = self
                    .storage
                    .get::<ConnectionRepository>()
                    .ok_or_else(|| {
                        SyncError::StorageError("ConnectionRepository not found".to_string())
                    })?;

                repo.update(&updated)
                    .map_err(|e| SyncError::StorageError(e.to_string()))?;

                Ok(())
            }
        }
    }
}
