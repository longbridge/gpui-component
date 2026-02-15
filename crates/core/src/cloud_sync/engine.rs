//! 云同步引擎
//!
//! 统一管理同步流程，将同步逻辑从 UI 层解耦。
//!
//! ## 设计原则
//!
//! - 参考 Dropbox Nucleus 架构的三棵树模型
//! - 支持冲突检测和多种解决策略
//! - 提供完整同步和增量同步两种模式

use crate::cloud_sync::client::CloudApiClient;
use crate::cloud_sync::models::{ConflictResolution, SyncResult};
use crate::cloud_sync::queue::OperationQueue;
use crate::cloud_sync::service::{CloudSyncService, SyncError};
use crate::crypto;
use crate::storage::StorageManager;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use super::connection_sync::ConnectionSyncHandler;
use super::workspace_sync::WorkspaceSyncHandler;

pub type SyncFuture<'a> = Pin<Box<dyn Future<Output = Result<SyncResult, SyncError>> + Send + 'a>>;

pub trait SyncHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn sync<'a>(&'a self, engine: &'a SyncEngine) -> SyncFuture<'a>;
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
                Box::new(WorkspaceSyncHandler),
                Box::new(ConnectionSyncHandler),
            ],
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
                let mut service_write = self.crypto_service
                    .write()
                    .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;
                if !service_write.is_unlocked() {
                    tracing::info!("[同步引擎] 从本地 crypto 模块同步密钥状态");
                    service_write.set_master_key_directly(raw_key);
                }
            }
        }

        let service = self.crypto_service
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
    /// 1. 先同步工作空间（无外键依赖）
    /// 2. 再同步连接（依赖工作空间）
    pub async fn sync(&self) -> Result<SyncResult, SyncError> {
        tracing::info!("========== 开始云同步 ==========");

        self.ensure_unlocked()?;

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
        let mut service = self.crypto_service
            .write()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

        Ok(service.take_operation_queue(key))
    }

    pub(crate) fn store_operation_queue(
        &self,
        key: &str,
        queue: OperationQueue,
    ) -> Result<(), SyncError> {
        let mut service = self.crypto_service
            .write()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

        service.store_operation_queue(key, queue);
        Ok(())
    }

}
