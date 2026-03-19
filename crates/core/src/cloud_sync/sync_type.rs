//! 同步类型抽象
//!
//! 提供 `SyncableItem` 和 `SyncTypeHandler` 两个核心 trait，
//! 将不同数据类型的同步逻辑从流程控制中解耦。
//! 新增同步数据类型只需实现 trait 并通过 `SyncEngine::register_type` 注册即可。

use crate::cloud_sync::engine::SyncEngine;
use crate::cloud_sync::models::{CloudSyncData, Team};
use crate::cloud_sync::service::{CloudSyncService, SyncError};
use crate::storage::PendingCloudDeletion;

/// 可同步数据项的统一抽象
///
/// 为各类本地数据（如 `StoredConnection`、`Workspace`）提供统一的字段访问接口，
/// 使通用同步流程无需关心具体数据结构。
pub trait SyncableItem: Clone + Send + Sync + 'static {
    /// 本地数据库 ID
    fn local_id(&self) -> Option<i64>;
    /// 设置本地 ID
    fn set_local_id(&mut self, id: Option<i64>);
    /// 数据项名称（用于日志和错误信息）
    fn item_name(&self) -> &str;
    /// 云端同步 ID
    fn cloud_id(&self) -> Option<&str>;
    /// 设置云端 ID
    fn set_cloud_id(&mut self, cloud_id: Option<String>);
    /// 更新时间戳（秒）
    fn updated_at(&self) -> Option<i64>;

    /// 是否启用同步（默认 true）
    fn is_sync_enabled(&self) -> bool {
        true
    }

    /// 最后同步时间戳（默认 None，Connection 有此字段）
    fn last_synced_at(&self) -> Option<i64> {
        None
    }

    /// 团队归属 ID（默认 None）
    fn team_id(&self) -> Option<&str> {
        None
    }
}

/// 同步计划（通用版）
///
/// 替代原有的 `WorkspaceSyncPlan`，适用于所有简单同步场景。
#[derive(Debug)]
pub struct GenericSyncPlan<T: SyncableItem> {
    /// 需要上传的数据项（本地新增）
    pub to_upload: Vec<T>,
    /// 需要更新到云端的数据项 (本地数据, 对应的云端同步数据)
    pub to_update_cloud: Vec<(T, CloudSyncData)>,
    /// 需要下载的数据项（云端新增）
    pub to_download: Vec<CloudSyncData>,
    /// 需要更新到本地的数据项 (云端同步数据, 对应的本地数据)
    pub to_update_local: Vec<(CloudSyncData, T)>,
}

impl<T: SyncableItem> Default for GenericSyncPlan<T> {
    fn default() -> Self {
        Self {
            to_upload: Vec::new(),
            to_update_cloud: Vec::new(),
            to_download: Vec::new(),
            to_update_local: Vec::new(),
        }
    }
}

/// 数据类型同步处理器
///
/// 封装特定数据类型的存储操作、加解密逻辑和同步回调。
/// 实现此 trait 后，通过 `SyncEngine::register_type` 注册即可自动接入通用同步流程。
pub trait SyncTypeHandler: Send + Sync + 'static {
    type Item: SyncableItem;

    // --- 标识 ---

    /// 数据类型标识（对应 `CloudSyncData.data_type`，如 "connection" / "workspace"）
    fn data_type(&self) -> &'static str;

    /// 显示名称（用于日志，如 "连接" / "工作空间"）
    fn display_name(&self) -> &'static str;

    /// 操作队列 key
    fn queue_key(&self) -> &'static str;

    // --- 本地存储操作 ---

    /// 获取所有本地数据项
    fn list_local(&self, engine: &SyncEngine) -> Result<Vec<Self::Item>, SyncError>;

    /// 插入新的本地数据项
    fn insert_local(&self, engine: &SyncEngine, item: &mut Self::Item) -> Result<(), SyncError>;

    /// 更新本地数据项（从云端同步下来后的更新）
    fn update_local_item(&self, engine: &SyncEngine, item: &Self::Item) -> Result<(), SyncError>;

    /// 删除本地数据项
    fn delete_local(&self, engine: &SyncEngine, id: i64) -> Result<(), SyncError>;

    // --- 同步后回调 ---

    /// 上传成功后的回调（通常更新 cloud_id）
    fn on_uploaded(
        &self,
        engine: &SyncEngine,
        local_id: i64,
        cloud_id: &str,
    ) -> Result<(), SyncError>;

    // --- 加解密 ---

    /// 尝试解密云端数据获取名称（用于构建 name_map，解密失败返回 None）
    fn decrypt_name(&self, service: &CloudSyncService, data: &CloudSyncData) -> Option<String>;

    /// 解密云端数据为本地数据项
    fn decrypt(
        &self,
        service: &CloudSyncService,
        data: &CloudSyncData,
    ) -> Result<Self::Item, SyncError>;

    /// 加密本地数据项为云端数据
    fn encrypt(
        &self,
        service: &CloudSyncService,
        item: &Self::Item,
        teams: &[Team],
    ) -> Result<CloudSyncData, SyncError>;

    // --- 待删除处理（有默认实现） ---

    /// 待删除实体类型标识（默认与 data_type 相同）
    fn pending_deletion_entity_type(&self) -> &'static str {
        self.data_type()
    }

    /// 获取待删除列表
    fn list_pending_deletions(&self, engine: &SyncEngine) -> Vec<PendingCloudDeletion> {
        let Some(repo) = engine
            .storage
            .get::<crate::storage::PendingCloudDeletionRepository>()
        else {
            return Vec::new();
        };

        let entity_type = self.pending_deletion_entity_type();
        let result = if entity_type == "connection" {
            repo.list_connections()
        } else {
            repo.list_workspaces()
        };

        result.unwrap_or_default()
    }

    /// 删除一条待删除记录
    fn remove_pending_deletion(
        &self,
        engine: &SyncEngine,
        cloud_id: &str,
    ) -> Result<(), SyncError> {
        let repo = engine
            .storage
            .get::<crate::storage::PendingCloudDeletionRepository>()
            .ok_or_else(|| {
                SyncError::StorageError("PendingCloudDeletionRepository not found".to_string())
            })?;

        repo.remove(cloud_id)
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }
}
