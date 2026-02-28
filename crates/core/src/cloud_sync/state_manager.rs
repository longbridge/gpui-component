//! 同步状态管理器
//!
//! 追踪每个连接的同步状态，用于：
//! - 判断哪些连接需要同步
//! - 检测冲突
//! - 记录同步历史

use crate::cloud_sync::models::{SyncState, SyncStatus};
use crate::storage::traits::Repository;
use crate::storage::{ConnectionRepository, StorageManager, StoredConnection};
use std::collections::HashMap;

/// 同步状态管理器
///
/// 维护连接的同步状态，支持：
/// - 获取单个连接的同步状态
/// - 批量获取同步状态
/// - 更新同步状态
pub struct SyncStateManager {
    /// 本地存储管理器
    storage: StorageManager,
    /// 内存中的状态缓存
    cache: HashMap<i64, SyncState>,
}

impl SyncStateManager {
    /// 创建新的同步状态管理器
    pub fn new(storage: StorageManager) -> Self {
        Self {
            storage,
            cache: HashMap::new(),
        }
    }

    /// 获取连接的同步状态
    ///
    /// 优先从缓存获取，缓存未命中时从本地连接信息构建
    pub fn get_state(&mut self, conn_id: i64) -> Option<SyncState> {
        // 先检查缓存
        if let Some(state) = self.cache.get(&conn_id) {
            return Some(state.clone());
        }

        // 缓存未命中，从存储构建
        let repo = self.storage.get::<ConnectionRepository>()?;
        let connections = repo.list().ok()?;
        let conn = connections.into_iter().find(|c| c.id == Some(conn_id))?;

        let state = self.build_state_from_connection(&conn)?;
        self.cache.insert(conn_id, state.clone());
        Some(state)
    }

    /// 获取所有启用同步的连接的状态
    pub fn get_all_sync_states(&mut self) -> Vec<SyncState> {
        let repo = match self.storage.get::<ConnectionRepository>() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let connections = match repo.list() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        connections
            .iter()
            .filter(|c| c.sync_enabled && c.id.is_some())
            .filter_map(|c| self.build_state_from_connection(c))
            .collect()
    }

    /// 从 StoredConnection 构建 SyncState
    fn build_state_from_connection(&self, conn: &StoredConnection) -> Option<SyncState> {
        let conn_id = conn.id?;

        // 确定同步状态
        let sync_status = self.determine_sync_status(conn);

        Some(SyncState {
            connection_id: conn_id,
            cloud_id: conn.cloud_id.clone().unwrap_or_default(),
            local_version: conn.updated_at.unwrap_or(0),
            cloud_version: conn.last_synced_at.unwrap_or(0), // 上次同步时的云端版本
            sync_status,
            last_synced_at: conn.last_synced_at,
        })
    }

    /// 确定连接的同步状态
    fn determine_sync_status(&self, conn: &StoredConnection) -> SyncStatus {
        if conn.cloud_id.is_none() {
            // 没有 cloud_id，是新连接待上传
            return SyncStatus::PendingUpload;
        }

        let local_updated = conn.updated_at.unwrap_or(0);
        let last_synced = conn.last_synced_at.unwrap_or(0);

        if local_updated > last_synced {
            // 本地有更新
            SyncStatus::LocalModified
        } else {
            // 已同步
            SyncStatus::Synced
        }
    }

    /// 更新同步状态
    pub fn update_state(&mut self, state: SyncState) {
        self.cache.insert(state.connection_id, state);
    }

    /// 标记连接为已同步
    pub fn mark_synced(&mut self, conn_id: i64, cloud_id: String, synced_at: i64) {
        let state = SyncState {
            connection_id: conn_id,
            cloud_id,
            local_version: synced_at,
            cloud_version: synced_at,
            sync_status: SyncStatus::Synced,
            last_synced_at: Some(synced_at),
        };
        self.cache.insert(conn_id, state);
    }

    /// 标记连接为本地修改
    pub fn mark_local_modified(&mut self, conn_id: i64) {
        if let Some(state) = self.cache.get_mut(&conn_id) {
            state.sync_status = SyncStatus::LocalModified;
        }
    }

    /// 标记连接为冲突
    pub fn mark_conflict(&mut self, conn_id: i64) {
        if let Some(state) = self.cache.get_mut(&conn_id) {
            state.sync_status = SyncStatus::Conflict;
        }
    }

    /// 获取需要上传的连接 ID 列表
    pub fn get_pending_uploads(&mut self) -> Vec<i64> {
        self.get_all_sync_states()
            .into_iter()
            .filter(|s| s.sync_status == SyncStatus::PendingUpload)
            .map(|s| s.connection_id)
            .collect()
    }

    /// 获取本地修改的连接 ID 列表
    pub fn get_local_modified(&mut self) -> Vec<i64> {
        self.get_all_sync_states()
            .into_iter()
            .filter(|s| s.sync_status == SyncStatus::LocalModified)
            .map(|s| s.connection_id)
            .collect()
    }

    /// 获取存在冲突的连接 ID 列表
    pub fn get_conflicts(&mut self) -> Vec<i64> {
        self.get_all_sync_states()
            .into_iter()
            .filter(|s| s.sync_status == SyncStatus::Conflict)
            .map(|s| s.connection_id)
            .collect()
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// 刷新单个连接的状态缓存
    pub fn refresh_state(&mut self, conn_id: i64) {
        self.cache.remove(&conn_id);
        // 下次 get_state 时会重新从存储加载
    }
}

/// 同步状态统计
#[derive(Debug, Default)]
pub struct SyncStats {
    /// 已同步的连接数
    pub synced: usize,
    /// 待上传的连接数
    pub pending_upload: usize,
    /// 本地修改待同步的连接数
    pub local_modified: usize,
    /// 云端修改待同步的连接数
    pub cloud_modified: usize,
    /// 存在冲突的连接数
    pub conflicts: usize,
}

impl SyncStateManager {
    /// 获取同步状态统计
    pub fn get_stats(&mut self) -> SyncStats {
        let states = self.get_all_sync_states();
        let mut stats = SyncStats::default();

        for state in states {
            match state.sync_status {
                SyncStatus::Synced => stats.synced += 1,
                SyncStatus::PendingUpload => stats.pending_upload += 1,
                SyncStatus::LocalModified => stats.local_modified += 1,
                SyncStatus::CloudModified => stats.cloud_modified += 1,
                SyncStatus::Conflict => stats.conflicts += 1,
                SyncStatus::PendingDownload => stats.cloud_modified += 1,
                SyncStatus::LocalDeleted | SyncStatus::CloudDeleted => {}
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_stats_default() {
        let stats = SyncStats::default();
        assert_eq!(stats.synced, 0);
        assert_eq!(stats.pending_upload, 0);
        assert_eq!(stats.conflicts, 0);
    }
}
