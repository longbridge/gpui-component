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
    pub cloud: CloudSyncData,
    /// 解密后的云端名称，用于 UI 展示
    pub cloud_name: String,
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
    /// 需要更新到云端的连接 (本地连接, 对应的云端同步数据)
    pub to_update_cloud: Vec<(crate::storage::StoredConnection, CloudSyncData)>,
    /// 需要下载的连接（云端新增）
    pub to_download: Vec<CloudSyncData>,
    /// 需要更新到本地的连接 (云端同步数据, 对应的本地连接)
    pub to_update_local: Vec<(CloudSyncData, crate::storage::StoredConnection)>,
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

// ============================================================================
// 统一加密同步数据模型
// ============================================================================

/// 统一云端同步数据
///
/// 用一张 `sync_data` 表替代 `connections` + `workspaces`，
/// 通过 `data_type` 区分数据类型，所有业务数据加密为一个 blob 上传。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncData {
    /// 云端 UUID 主键
    pub id: String,
    /// 记录创建者
    pub owner_id: String,
    /// 团队归属：None = 个人数据，Some = 团队共享数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// 数据类型标识（"connection" | "workspace" | ...）
    pub data_type: String,
    /// 加密后的完整数据 blob（base64(nonce + AES-256-GCM ciphertext)）
    pub encrypted_data: String,
    /// 加密密钥版本
    pub key_version: u32,
    /// 明文数据的 SHA-256 校验和（加密前计算，用于冲突检测）
    #[serde(default)]
    pub checksum: String,
    /// 数据版本号（每次更新自动递增，用于乐观并发控制）
    #[serde(default = "default_version")]
    pub version: u32,
    /// 更新时间戳（毫秒）
    pub updated_at: i64,
    /// 软删除时间戳（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
}

fn default_version() -> u32 {
    1
}

/// 数据类型常量
pub mod data_type {
    pub const CONNECTION: &str = "connection";
    pub const WORKSPACE: &str = "workspace";
}

/// 团队
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    /// 团队 UUID
    pub id: String,
    /// 团队名称
    pub name: String,
    /// 团队拥有者 ID
    pub owner_id: String,
    /// 团队描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 团队密钥验证数据（由 owner 设置，成员验证用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_verification: Option<String>,
    /// 团队密钥版本号
    #[serde(default)]
    pub key_version: u32,
    /// 创建时间戳（毫秒）
    pub created_at: i64,
    /// 更新时间戳（毫秒）
    pub updated_at: i64,
}

/// 团队成员角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamRole {
    #[serde(rename = "owner")]
    Owner,
    #[serde(rename = "member")]
    Member,
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamRole::Owner => write!(f, "owner"),
            TeamRole::Member => write!(f, "member"),
        }
    }
}

/// 团队成员
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    /// 成员记录 UUID
    pub id: String,
    /// 所属团队 ID
    pub team_id: String,
    /// 用户 ID
    pub user_id: String,
    /// 成员角色
    pub role: TeamRole,
    /// 加入时间戳（毫秒）
    pub joined_at: i64,
}

/// 连接明文数据结构（加密前 / 解密后的 JSON blob）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPlainData {
    /// 连接名称
    pub name: String,
    /// 连接类型
    pub connection_type: String,
    /// 关联的工作空间云端 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_cloud_id: Option<String>,
    /// 已选中的数据库列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_databases: Option<String>,
    /// 备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    /// 连接参数（完整 JSON，包含密码等）
    pub params: serde_json::Value,
    /// 连接创建者 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
}

/// 工作空间明文数据结构（加密前 / 解密后的 JSON blob）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePlainData {
    /// 工作空间名称
    pub name: String,
    /// 颜色
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 图标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}
