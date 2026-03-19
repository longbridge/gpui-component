//! 工作空间同步处理器
//!
//! 通过实现 `SyncTypeHandler` trait，将工作空间同步逻辑接入通用同步流程 `generic_sync`。

use crate::cloud_sync::engine::SyncEngine;
use crate::cloud_sync::models::{CloudSyncData, Team};
use crate::cloud_sync::service::{CloudSyncService, SyncError};
use crate::cloud_sync::sync_type::{SyncTypeHandler, SyncableItem};
use crate::storage::traits::Repository;
use crate::storage::{Workspace, WorkspaceRepository};

/// 工作空间同步类型处理器
pub struct WorkspaceSyncType;

impl SyncTypeHandler for WorkspaceSyncType {
    type Item = Workspace;

    fn data_type(&self) -> &'static str {
        "workspace"
    }

    fn display_name(&self) -> &'static str {
        "工作空间"
    }

    fn queue_key(&self) -> &'static str {
        "workspace"
    }

    fn list_local(&self, engine: &SyncEngine) -> Result<Vec<Workspace>, SyncError> {
        let repo = engine
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.list()
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }

    fn insert_local(&self, engine: &SyncEngine, item: &mut Workspace) -> Result<(), SyncError> {
        let repo = engine
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.insert(item)
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        Ok(())
    }

    fn update_local_item(&self, engine: &SyncEngine, item: &Workspace) -> Result<(), SyncError> {
        let repo = engine
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.update_from_cloud(item)
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }

    fn delete_local(&self, engine: &SyncEngine, id: i64) -> Result<(), SyncError> {
        let repo = engine
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.delete(id)
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }

    fn on_uploaded(
        &self,
        engine: &SyncEngine,
        local_id: i64,
        cloud_id: &str,
    ) -> Result<(), SyncError> {
        let repo = engine
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.update_cloud_id(local_id, Some(cloud_id.to_string()))
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }

    fn decrypt_name(&self, service: &CloudSyncService, data: &CloudSyncData) -> Option<String> {
        service
            .decrypt_sync_data_workspace(data)
            .ok()
            .map(|ws| ws.name)
    }

    fn decrypt(
        &self,
        service: &CloudSyncService,
        data: &CloudSyncData,
    ) -> Result<Workspace, SyncError> {
        service.decrypt_sync_data_workspace(data)
    }

    fn encrypt(
        &self,
        service: &CloudSyncService,
        item: &Workspace,
        teams: &[Team],
    ) -> Result<CloudSyncData, SyncError> {
        service.prepare_workspace_sync_data_upload(item, item.team_id(), teams)
    }

    fn pending_deletion_entity_type(&self) -> &'static str {
        "workspace"
    }
}
