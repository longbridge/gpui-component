//! 冲突检测与解决
//!
//! 提供同步冲突的检测、分类和解决策略。
//!
//! ## 冲突类型
//!
//! - BothModified: 本地和云端都修改了同一连接
//! - LocalDeletedCloudModified: 本地删除了连接，但云端有更新
//! - LocalModifiedCloudDeleted: 本地有更新，但云端删除了连接
//!
//! ## 解决策略
//!
//! - UseCloud: 使用云端版本（丢弃本地修改）
//! - UseLocal: 使用本地版本（覆盖云端）
//! - KeepBoth: 保留两个版本（创建副本）

use crate::cloud_sync::models::{CloudConnection, ConflictType, SyncConflict, ConflictResolution};
use crate::storage::StoredConnection;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// 冲突解决器
///
/// 负责：
/// - 检测冲突
/// - 根据策略自动解决冲突
/// - 创建冲突副本
pub struct ConflictResolver {
    /// 默认解决策略
    default_strategy: ConflictResolution,
}

impl ConflictResolver {
    /// 创建新的冲突解决器
    pub fn new(default_strategy: ConflictResolution) -> Self {
        Self { default_strategy }
    }

    /// 使用默认策略创建冲突解决器（使用云端版本）
    pub fn with_cloud_priority() -> Self {
        Self::new(ConflictResolution::UseCloud)
    }

    /// 使用本地优先策略创建冲突解决器
    pub fn with_local_priority() -> Self {
        Self::new(ConflictResolution::UseLocal)
    }

    /// 使用保留两者策略创建冲突解决器
    pub fn with_keep_both() -> Self {
        Self::new(ConflictResolution::KeepBoth)
    }

    /// 获取当前默认策略
    pub fn default_strategy(&self) -> ConflictResolution {
        self.default_strategy
    }

    /// 设置默认策略
    pub fn set_default_strategy(&mut self, strategy: ConflictResolution) {
        self.default_strategy = strategy;
    }

    /// 检测冲突
    ///
    /// 比较本地连接和云端连接，判断是否存在冲突以及冲突类型
    pub fn detect_conflict(
        local: &StoredConnection,
        cloud: &CloudConnection,
        last_synced_at: i64,
    ) -> Option<SyncConflict> {
        let local_updated = local.updated_at.unwrap_or(0);
        let cloud_updated = cloud.updated_at / 1000; // 云端是毫秒

        let local_changed = local_updated > last_synced_at;
        let cloud_changed = cloud_updated > last_synced_at;

        // 只有双方都修改时才是冲突
        if local_changed && cloud_changed {
            // 进一步检查内容是否实际相同（通过 checksum）
            let local_checksum = Self::calculate_checksum(local);
            if local_checksum != cloud.checksum && !cloud.checksum.is_empty() {
                return Some(SyncConflict {
                    local: local.clone(),
                    cloud: cloud.clone(),
                    conflict_type: ConflictType::BothModified,
                });
            }
        }

        None
    }

    /// 检测本地修改但云端已删除的情况
    pub fn detect_local_modified_cloud_deleted(
        local: &StoredConnection,
        cloud_id: &str,
    ) -> SyncConflict {
        SyncConflict {
            local: local.clone(),
            cloud: CloudConnection {
                id: cloud_id.to_string(),
                local_id: local.id,
                name: local.name.clone(),
                connection_type: local.connection_type.to_string(),
                workspace_id: local.workspace_id.map(|id| id.to_string()),
                encrypted_params: String::new(),
                key_version: 0,
                updated_at: 0,
                checksum: String::new(),
                deleted_at: None,
            },
            conflict_type: ConflictType::LocalModifiedCloudDeleted,
        }
    }

    /// 检测本地删除但云端已修改的情况
    pub fn detect_local_deleted_cloud_modified(
        local: &StoredConnection,
        cloud: &CloudConnection,
    ) -> SyncConflict {
        SyncConflict {
            local: local.clone(),
            cloud: cloud.clone(),
            conflict_type: ConflictType::LocalDeletedCloudModified,
        }
    }

    /// 自动解决冲突（使用默认策略）
    pub fn auto_resolve(&self, conflict: &SyncConflict) -> ResolvedAction {
        self.resolve_with_strategy(conflict, self.default_strategy)
    }

    /// 使用指定策略解决冲突
    pub fn resolve_with_strategy(
        &self,
        conflict: &SyncConflict,
        strategy: ConflictResolution,
    ) -> ResolvedAction {
        match strategy {
            ConflictResolution::UseCloud => {
                ResolvedAction::UpdateLocal(conflict.cloud.clone())
            }
            ConflictResolution::UseLocal => {
                ResolvedAction::UpdateCloud(conflict.local.clone())
            }
            ConflictResolution::KeepBoth => {
                let copy = self.create_conflict_copy(&conflict.local, "本地");
                ResolvedAction::KeepBoth {
                    keep_cloud: conflict.cloud.clone(),
                    create_local_copy: copy,
                }
            }
        }
    }

    /// 创建冲突副本
    ///
    /// 复制连接并重命名，标记来源和时间
    pub fn create_conflict_copy(&self, conn: &StoredConnection, source: &str) -> StoredConnection {
        let timestamp = Self::current_timestamp();
        let formatted_time = Self::format_timestamp(timestamp);

        let mut copy = conn.clone();
        copy.id = None;  // 清除 ID，作为新连接插入
        copy.cloud_id = None;  // 清除云端关联
        copy.last_synced_at = None;  // 清除同步状态
        copy.name = format!("{} ({} {})", conn.name, source, formatted_time);

        copy
    }

    /// 计算连接的 checksum
    fn calculate_checksum(conn: &StoredConnection) -> String {
        let mut hasher = Sha256::new();
        hasher.update(conn.name.as_bytes());
        hasher.update(conn.connection_type.to_string().as_bytes());
        hasher.update(conn.params.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 获取当前时间戳（秒）
    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// 格式化时间戳为可读字符串
    fn format_timestamp(timestamp: i64) -> String {
        use chrono::{DateTime, Utc};
        DateTime::from_timestamp(timestamp, 0)
            .map(|dt: DateTime<Utc>| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| timestamp.to_string())
    }
}

/// 冲突解决后的操作
#[derive(Debug, Clone)]
pub enum ResolvedAction {
    /// 更新本地（使用云端版本）
    UpdateLocal(CloudConnection),
    /// 更新云端（使用本地版本）
    UpdateCloud(StoredConnection),
    /// 保留两者（云端版本更新到本地，同时创建本地副本）
    KeepBoth {
        /// 保留的云端连接
        keep_cloud: CloudConnection,
        /// 创建的本地副本
        create_local_copy: StoredConnection,
    },
    /// 删除本地连接
    DeleteLocal(i64),
    /// 删除云端连接
    DeleteCloud(String),
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::with_cloud_priority()
    }
}

/// 冲突详情，用于 UI 展示
#[derive(Debug, Clone)]
pub struct ConflictDetail {
    /// 冲突类型描述
    pub type_description: String,
    /// 本地版本信息
    pub local_info: ConnectionInfo,
    /// 云端版本信息
    pub cloud_info: ConnectionInfo,
    /// 建议的解决策略
    pub suggested_resolution: ConflictResolution,
}

/// 连接信息摘要，用于 UI 展示
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// 连接名称
    pub name: String,
    /// 最后修改时间
    pub last_modified: String,
    /// 连接类型
    pub connection_type: String,
}

impl ConflictDetail {
    /// 从冲突创建详情
    pub fn from_conflict(conflict: &SyncConflict) -> Self {
        let type_description = match conflict.conflict_type {
            ConflictType::BothModified => "本地和云端都有修改".to_string(),
            ConflictType::LocalDeletedCloudModified => "本地已删除，但云端有更新".to_string(),
            ConflictType::LocalModifiedCloudDeleted => "本地有更新，但云端已删除".to_string(),
        };

        let local_info = ConnectionInfo {
            name: conflict.local.name.clone(),
            last_modified: conflict.local.updated_at
                .map(|t| ConflictResolver::format_timestamp_static(t))
                .unwrap_or_else(|| "未知".to_string()),
            connection_type: conflict.local.connection_type.to_string(),
        };

        let cloud_info = ConnectionInfo {
            name: conflict.cloud.name.clone(),
            last_modified: ConflictResolver::format_timestamp_static(conflict.cloud.updated_at / 1000),
            connection_type: conflict.cloud.connection_type.clone(),
        };

        let suggested_resolution = match conflict.conflict_type {
            ConflictType::BothModified => ConflictResolution::KeepBoth,
            ConflictType::LocalDeletedCloudModified => ConflictResolution::UseCloud,
            ConflictType::LocalModifiedCloudDeleted => ConflictResolution::UseLocal,
        };

        Self {
            type_description,
            local_info,
            cloud_info,
            suggested_resolution,
        }
    }
}

impl ConflictResolver {
    /// 格式化时间戳（静态方法，用于 ConflictDetail）
    fn format_timestamp_static(timestamp: i64) -> String {
        use chrono::{DateTime, Utc};
        DateTime::from_timestamp(timestamp, 0)
            .map(|dt: DateTime<Utc>| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| timestamp.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_resolver_default() {
        let resolver = ConflictResolver::default();
        assert_eq!(resolver.default_strategy(), ConflictResolution::UseCloud);
    }

    #[test]
    fn test_conflict_copy_naming() {
        let resolver = ConflictResolver::default();
        let conn = StoredConnection {
            id: Some(1),
            name: "Test Connection".to_string(),
            connection_type: crate::storage::ConnectionType::Database,
            workspace_id: None,
            params: "{}".to_string(),
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: Some("cloud-123".to_string()),
            last_synced_at: Some(100),
            created_at: None,
            updated_at: Some(200),
        };

        let copy = resolver.create_conflict_copy(&conn, "本地");
        assert!(copy.name.contains("Test Connection"));
        assert!(copy.name.contains("本地"));
        assert!(copy.id.is_none());
        assert!(copy.cloud_id.is_none());
    }
}
