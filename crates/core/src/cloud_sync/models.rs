//! 云同步数据模型

use serde::{Deserialize, Serialize};

/// 云端用户加密配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudUserConfig {
    /// 用户 ID
    pub user_id: String,
    /// 密钥验证数据（用于验证主密钥是否正确，不含密钥本身）
    pub key_verification: String,
    /// 密钥版本号（每次修改主密钥时递增）
    pub key_version: u32,
    /// 最后更新时间戳
    pub updated_at: i64,
}

/// 云端连接数据（加密存储）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConnection {
    /// 云端 ID（UUID）
    pub id: String,
    /// 本地数据库 ID（用于关联）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_id: Option<i64>,
    /// 连接名称
    pub name: String,
    /// 连接类型
    pub connection_type: String,
    /// 工作区 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// 加密的连接参数（ENC:base64...）
    pub encrypted_params: String,
    /// 加密时使用的密钥版本
    pub key_version: u32,
    /// 更新时间戳
    pub updated_at: i64,
    /// 数据校验和（用于冲突检测）
    pub checksum: String,
    /// 软删除时间戳（毫秒），None 表示未删除
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
}

/// 同步状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// 已同步
    Synced,
    /// 本地有修改，待上传
    LocalModified,
    /// 云端有修改，待下载
    CloudModified,
    /// 冲突
    Conflict,
    /// 待上传（新创建）
    PendingUpload,
    /// 待下载（云端新建）
    PendingDownload,
    /// 本地已删除
    LocalDeleted,
    /// 云端已删除
    CloudDeleted,
}

/// 本地同步状态记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// 本地连接 ID
    pub connection_id: i64,
    /// 云端连接 ID
    pub cloud_id: String,
    /// 本地版本时间戳
    pub local_version: i64,
    /// 云端版本时间戳
    pub cloud_version: i64,
    /// 同步状态
    pub sync_status: SyncStatus,
    /// 最后同步时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<i64>,
}

/// 同步冲突
#[derive(Debug, Clone)]
pub struct SyncConflict {
    /// 本地数据
    pub local: crate::storage::StoredConnection,
    /// 云端数据
    pub cloud: CloudConnection,
    /// 冲突类型
    pub conflict_type: ConflictType,
}

/// 冲突类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// 两边都修改了
    BothModified,
    /// 本地删除，云端修改
    LocalDeletedCloudModified,
    /// 本地修改，云端删除
    LocalModifiedCloudDeleted,
}

/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictResolution {
    /// 使用云端版本
    #[default]
    UseCloud,
    /// 使用本地版本
    UseLocal,
    /// 保留两者（创建副本）
    KeepBoth,
}

impl std::fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictResolution::UseCloud => write!(f, "使用云端版本"),
            ConflictResolution::UseLocal => write!(f, "使用本地版本"),
            ConflictResolution::KeepBoth => write!(f, "保留两者"),
        }
    }
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictType::BothModified => write!(f, "本地和云端都有修改"),
            ConflictType::LocalDeletedCloudModified => write!(f, "本地已删除，但云端有更新"),
            ConflictType::LocalModifiedCloudDeleted => write!(f, "本地有更新，但云端已删除"),
        }
    }
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Synced => write!(f, "已同步"),
            SyncStatus::LocalModified => write!(f, "本地修改待同步"),
            SyncStatus::CloudModified => write!(f, "云端修改待下载"),
            SyncStatus::Conflict => write!(f, "存在冲突"),
            SyncStatus::PendingUpload => write!(f, "待上传"),
            SyncStatus::PendingDownload => write!(f, "待下载"),
            SyncStatus::LocalDeleted => write!(f, "本地已删除"),
            SyncStatus::CloudDeleted => write!(f, "云端已删除"),
        }
    }
}

/// 同步操作结果
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// 上传成功的数量
    pub uploaded: usize,
    /// 下载成功的数量
    pub downloaded: usize,
    /// 删除成功的数量
    pub deleted: usize,
    /// 冲突列表
    pub conflicts: Vec<SyncConflict>,
    /// 错误列表
    pub errors: Vec<String>,
}

impl Default for SyncResult {
    fn default() -> Self {
        Self {
            uploaded: 0,
            downloaded: 0,
            deleted: 0,
            conflicts: Vec::new(),
            errors: Vec::new(),
        }
    }
}

/// 同步计划
#[derive(Debug, Default)]
pub struct SyncPlan {
    /// 需要上传的连接（本地新增）
    pub to_upload: Vec<crate::storage::StoredConnection>,
    /// 需要更新到云端的连接 (本地连接, 对应的云端连接)
    pub to_update_cloud: Vec<(crate::storage::StoredConnection, CloudConnection)>,
    /// 需要下载的连接（云端新增）
    pub to_download: Vec<CloudConnection>,
    /// 需要更新到本地的连接 (云端连接, 对应的本地连接)
    pub to_update_local: Vec<(CloudConnection, crate::storage::StoredConnection)>,
    /// 需要删除的云端连接 ID
    pub to_delete_cloud: Vec<String>,
    /// 需要删除的本地连接 ID
    pub to_delete_local: Vec<i64>,
    /// 冲突列表
    pub conflicts: Vec<SyncConflict>,
}

impl SyncPlan {
    /// 检查计划是否为空（无需执行任何操作）
    pub fn is_empty(&self) -> bool {
        self.to_upload.is_empty()
            && self.to_update_cloud.is_empty()
            && self.to_download.is_empty()
            && self.to_update_local.is_empty()
            && self.to_delete_cloud.is_empty()
            && self.to_delete_local.is_empty()
            && self.conflicts.is_empty()
    }

    /// 获取计划中的操作总数
    pub fn total_operations(&self) -> usize {
        self.to_upload.len()
            + self.to_update_cloud.len()
            + self.to_download.len()
            + self.to_update_local.len()
            + self.to_delete_cloud.len()
            + self.to_delete_local.len()
    }
}

/// 工作空间同步计划
#[derive(Debug, Default)]
pub struct WorkspaceSyncPlan {
    /// 需要上传的工作空间（本地新增）
    pub to_upload: Vec<crate::storage::Workspace>,
    /// 需要更新到云端的工作空间 (本地工作空间, 对应的云端工作空间)
    pub to_update_cloud: Vec<(crate::storage::Workspace, CloudWorkspace)>,
    /// 需要下载的工作空间（云端新增）
    pub to_download: Vec<CloudWorkspace>,
    /// 需要更新到本地的工作空间 (云端工作空间, 对应的本地工作空间)
    pub to_update_local: Vec<(CloudWorkspace, crate::storage::Workspace)>,
}

/// 已解决的冲突
#[derive(Debug, Clone)]
pub struct ResolvedConflict {
    /// 原始冲突
    pub conflict: SyncConflict,
    /// 解决策略
    pub resolution: ConflictResolution,
    /// 结果连接（如果需要创建副本）
    pub result_connection: Option<crate::storage::StoredConnection>,
}

/// 批量同步请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// 待上传的连接
    pub uploads: Vec<CloudConnection>,
    /// 需要下载的连接 ID 列表
    pub download_ids: Vec<String>,
    /// 待删除的连接 ID 列表
    pub delete_ids: Vec<String>,
}

/// 批量同步响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// 上传成功的 ID 映射（local_id -> cloud_id）
    pub uploaded_ids: Vec<(Option<i64>, String)>,
    /// 下载的连接数据
    pub downloaded: Vec<CloudConnection>,
    /// 删除成功的 ID 列表
    pub deleted_ids: Vec<String>,
    /// 错误信息
    pub errors: Vec<String>,
}

/// 云端工作空间数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudWorkspace {
    /// 云端 ID（UUID）
    pub id: String,
    /// 本地数据库 ID（用于关联）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_id: Option<i64>,
    /// 工作空间名称
    pub name: String,
    /// 颜色
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 图标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// 更新时间戳（毫秒）
    pub updated_at: i64,
    /// 软删除时间戳（毫秒），None 表示未删除
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
}
