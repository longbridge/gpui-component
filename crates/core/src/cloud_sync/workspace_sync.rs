use crate::cloud_sync::engine::{SyncEngine, SyncFuture, SyncHandler};
use crate::cloud_sync::models::{CloudWorkspace, SyncResult, WorkspaceSyncPlan};
use crate::cloud_sync::queue::SyncOperation;
use crate::cloud_sync::service::SyncError;
use crate::storage::traits::Repository;
use crate::storage::{PendingCloudDeletionRepository, Workspace, WorkspaceRepository};
use std::collections::{HashMap, HashSet};

const WORKSPACE_QUEUE_KEY: &str = "workspace";

pub struct WorkspaceSyncHandler;

impl SyncHandler for WorkspaceSyncHandler {
    fn name(&self) -> &'static str {
        "工作空间"
    }

    fn sync<'a>(&'a self, engine: &'a SyncEngine) -> SyncFuture<'a> {
        Box::pin(async move { engine.sync_workspaces().await })
    }
}

impl SyncEngine {
    /// 同步工作空间
    async fn sync_workspaces(&self) -> Result<SyncResult, SyncError> {
        let mut result = SyncResult::default();

        let pending_deletions = self.process_pending_workspace_deletions().await;
        tracing::info!(
            "[工作空间] 处理待删除列表完成: {} 个",
            pending_deletions.len()
        );

        let local_workspaces = self.get_local_workspaces()?;
        tracing::info!("[工作空间] 本地工作空间: {} 个", local_workspaces.len());

        let cloud_workspaces = self
            .cloud_client
            .list_workspaces()
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;
        tracing::info!("[工作空间] 云端工作空间: {} 个", cloud_workspaces.len());

        let deleted_count =
            self.process_cloud_soft_deleted_workspaces(&cloud_workspaces, &local_workspaces)?;
        if deleted_count > 0 {
            tracing::info!(
                "[工作空间] 处理云端软删除: 删除了 {} 个本地工作空间",
                deleted_count
            );
            result.deleted += deleted_count;
        }

        let active_cloud_workspaces: Vec<_> = cloud_workspaces
            .into_iter()
            .filter(|ws| ws.deleted_at.is_none())
            .collect();
        tracing::info!(
            "[工作空间] 活跃云端工作空间: {} 个",
            active_cloud_workspaces.len()
        );

        let plan =
            self.calculate_workspace_sync_plan(&local_workspaces, &active_cloud_workspaces)?;
        tracing::info!(
            "[工作空间计划] 上传: {}, 更新云端: {}, 下载: {}, 更新本地: {}",
            plan.to_upload.len(),
            plan.to_update_cloud.len(),
            plan.to_download.len(),
            plan.to_update_local.len()
        );

        let local_workspace_map: HashMap<i64, Workspace> = local_workspaces
            .iter()
            .filter_map(|ws| ws.id.map(|id| (id, ws.clone())))
            .collect();
        let cloud_workspace_map: HashMap<String, CloudWorkspace> = active_cloud_workspaces
            .iter()
            .map(|ws| (ws.id.clone(), ws.clone()))
            .collect();

        let mut operations = Vec::new();
        for local_ws in &plan.to_upload {
            if let Some(local_id) = local_ws.id {
                operations.push(SyncOperation::Upload { local_id });
            } else {
                result
                    .errors
                    .push(format!("上传工作空间失败 {}: 缺少本地 ID", local_ws.name));
            }
        }

        for (local_ws, cloud_ws) in &plan.to_update_cloud {
            if let Some(local_id) = local_ws.id {
                operations.push(SyncOperation::UpdateCloud {
                    local_id,
                    cloud_id: cloud_ws.id.clone(),
                });
            } else {
                result.errors.push(format!(
                    "更新云端工作空间失败 {}: 缺少本地 ID",
                    local_ws.name
                ));
            }
        }

        for cloud_ws in &plan.to_download {
            operations.push(SyncOperation::Download(cloud_ws.id.clone()));
        }

        for (cloud_ws, local_ws) in &plan.to_update_local {
            if let Some(local_id) = local_ws.id {
                operations.push(SyncOperation::UpdateLocal {
                    local_id,
                    cloud_id: cloud_ws.id.clone(),
                });
            } else {
                result.errors.push(format!(
                    "更新本地工作空间失败 {}: 缺少本地 ID",
                    cloud_ws.name
                ));
            }
        }

        let mut queue = self.take_operation_queue(WORKSPACE_QUEUE_KEY)?;
        queue.enqueue_all(operations);
        queue.retry_failed();

        while let Some(queued_operation) = queue.dequeue() {
            let operation = queued_operation.operation.clone();
            match operation {
                SyncOperation::Upload { local_id } => {
                    let Some(local_ws) = local_workspace_map.get(&local_id) else {
                        result
                            .errors
                            .push(format!("上传工作空间失败 {}: 本地数据不存在", local_id));
                        continue;
                    };

                    match self.upload_workspace(local_ws).await {
                        Ok(cloud_id) => match self.update_workspace_cloud_id(local_id, &cloud_id) {
                            Ok(()) => {
                                result.uploaded += 1;
                                tracing::info!("[上传工作空间] 成功: {}", local_ws.name);
                            }
                            Err(e) => {
                                let error_message =
                                    format!("上传工作空间失败 {}: {}", local_ws.name, e);
                                result.errors.push(error_message.clone());
                                queue.mark_failed(queued_operation, error_message);
                            }
                        },
                        Err(e) => {
                            let error_message =
                                format!("上传工作空间失败 {}: {}", local_ws.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::UpdateCloud { local_id, cloud_id } => {
                    let Some(local_ws) = local_workspace_map.get(&local_id) else {
                        result
                            .errors
                            .push(format!("更新云端工作空间失败 {}: 本地数据不存在", local_id));
                        continue;
                    };
                    let Some(cloud_ws) = cloud_workspace_map.get(&cloud_id) else {
                        result
                            .errors
                            .push(format!("更新云端工作空间失败 {}: 云端数据不存在", cloud_id));
                        continue;
                    };

                    match self.update_cloud_workspace(local_ws, cloud_ws).await {
                        Ok(()) => {
                            result.uploaded += 1;
                            tracing::info!("[更新云端工作空间] 成功: {}", local_ws.name);
                        }
                        Err(e) => {
                            let error_message =
                                format!("更新云端工作空间失败 {}: {}", local_ws.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::UpdateLocal { local_id, cloud_id } => {
                    let Some(local_ws) = local_workspace_map.get(&local_id) else {
                        result
                            .errors
                            .push(format!("更新本地工作空间失败 {}: 本地数据不存在", local_id));
                        continue;
                    };
                    let Some(cloud_ws) = cloud_workspace_map.get(&cloud_id) else {
                        result
                            .errors
                            .push(format!("更新本地工作空间失败 {}: 云端数据不存在", cloud_id));
                        continue;
                    };

                    match self.update_local_workspace(cloud_ws, local_ws).await {
                        Ok(()) => {
                            result.downloaded += 1;
                            tracing::info!("[更新本地工作空间] 成功: {}", cloud_ws.name);
                        }
                        Err(e) => {
                            let error_message =
                                format!("更新本地工作空间失败 {}: {}", cloud_ws.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::Download(cloud_id) => {
                    let Some(cloud_ws) = cloud_workspace_map.get(&cloud_id) else {
                        result
                            .errors
                            .push(format!("下载工作空间失败 {}: 云端数据不存在", cloud_id));
                        continue;
                    };

                    match self.download_workspace(cloud_ws).await {
                        Ok(()) => {
                            result.downloaded += 1;
                            tracing::info!("[下载工作空间] 成功: {}", cloud_ws.name);
                        }
                        Err(e) => {
                            let error_message =
                                format!("下载工作空间失败 {}: {}", cloud_ws.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::DeleteCloud(cloud_id) => {
                    match self.delete_cloud_workspace(&cloud_id).await {
                        Ok(()) => {
                            result.deleted += 1;
                            tracing::info!("[删除云端工作空间] 成功: {}", cloud_id);
                        }
                        Err(e) => {
                            let error_message = format!("删除云端工作空间失败 {}: {}", cloud_id, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::DeleteLocal(local_id) => {
                    match self.delete_local_workspace(local_id) {
                        Ok(()) => {
                            result.deleted += 1;
                            tracing::info!("[删除本地工作空间] 成功: {}", local_id);
                        }
                        Err(e) => {
                            let error_message = format!("删除本地工作空间失败 {}: {}", local_id, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
            }
        }

        self.store_operation_queue(WORKSPACE_QUEUE_KEY, queue)?;

        Ok(result)
    }

    /// 计算工作空间同步计划
    fn calculate_workspace_sync_plan(
        &self,
        local_workspaces: &[Workspace],
        cloud_workspaces: &[CloudWorkspace],
    ) -> Result<WorkspaceSyncPlan, SyncError> {
        let mut plan = WorkspaceSyncPlan::default();

        let cloud_map: HashMap<&str, &CloudWorkspace> = cloud_workspaces
            .iter()
            .map(|ws| (ws.id.as_str(), ws))
            .collect();

        let local_cloud_ids: HashSet<String> = local_workspaces
            .iter()
            .filter_map(|ws| ws.cloud_id.clone())
            .collect();

        let local_unlinked_by_name: HashMap<&str, &Workspace> = local_workspaces
            .iter()
            .filter(|ws| ws.cloud_id.is_none())
            .map(|ws| (ws.name.as_str(), ws))
            .collect();

        let local_all_by_name: HashMap<&str, &Workspace> = local_workspaces
            .iter()
            .map(|ws| (ws.name.as_str(), ws))
            .collect();

        for local_ws in local_workspaces {
            match &local_ws.cloud_id {
                Some(cloud_id) => {
                    if let Some(cloud_ws) = cloud_map.get(cloud_id.as_str()) {
                        let local_updated = local_ws.updated_at.unwrap_or(0);
                        let cloud_updated = cloud_ws.updated_at / 1000;

                        if local_updated > cloud_updated {
                            plan.to_update_cloud
                                .push((local_ws.clone(), (*cloud_ws).clone()));
                        } else if cloud_updated > local_updated {
                            plan.to_update_local
                                .push(((*cloud_ws).clone(), local_ws.clone()));
                        }
                    }
                }
                None => {
                    let has_cloud_match =
                        cloud_workspaces.iter().any(|cws| cws.name == local_ws.name);
                    if !has_cloud_match {
                        plan.to_upload.push(local_ws.clone());
                    }
                }
            }
        }

        let pending_cloud_ids = self.get_pending_deletion_workspace_cloud_ids();
        for cloud_ws in cloud_workspaces {
            if !local_cloud_ids.contains(&cloud_ws.id) {
                if pending_cloud_ids.contains(&cloud_ws.id) {
                    tracing::info!("[同步计划] 跳过待删除的云端工作空间: {}", cloud_ws.name);
                    continue;
                }

                if let Some(local_ws) = local_unlinked_by_name.get(cloud_ws.name.as_str()) {
                    tracing::info!(
                        "[同步计划] 按名称匹配工作空间: {} (云端 {} -> 本地 {:?})",
                        cloud_ws.name,
                        cloud_ws.id,
                        local_ws.id
                    );
                    plan.to_update_local
                        .push((cloud_ws.clone(), (*local_ws).clone()));
                } else if let Some(local_ws) = local_all_by_name.get(cloud_ws.name.as_str()) {
                    let local_cloud_id_is_valid = local_ws
                        .cloud_id
                        .as_ref()
                        .is_some_and(|cloud_id| cloud_map.contains_key(cloud_id.as_str()));
                    if !local_cloud_id_is_valid {
                        plan.to_update_local
                            .push((cloud_ws.clone(), (*local_ws).clone()));
                    }
                } else {
                    plan.to_download.push(cloud_ws.clone());
                }
            }
        }

        Ok(plan)
    }

    async fn process_pending_workspace_deletions(&self) -> Vec<String> {
        let mut deleted = Vec::new();

        let pending_repo = match self.storage.get::<PendingCloudDeletionRepository>() {
            Some(repo) => repo,
            None => {
                tracing::warn!("[同步] PendingCloudDeletionRepository not found");
                return deleted;
            }
        };

        let pending_list = match pending_repo.list_workspaces() {
            Ok(list) => list,
            Err(e) => {
                tracing::error!("[同步] 获取待删除工作空间列表失败: {}", e);
                return deleted;
            }
        };

        for pending in pending_list {
            tracing::info!("[同步] 处理待删除云端工作空间: {}", pending.cloud_id);
            match self.cloud_client.delete_workspace(&pending.cloud_id).await {
                Ok(_) => {
                    tracing::info!("[同步] 云端工作空间删除成功: {}", pending.cloud_id);
                    if let Err(e) = pending_repo.remove(&pending.cloud_id) {
                        tracing::error!("[同步] 移除待删除记录失败: {}", e);
                    }
                    deleted.push(pending.cloud_id);
                }
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("404") || error_str.contains("not found") {
                        tracing::info!(
                            "[同步] 云端工作空间已不存在，移除待删除记录: {}",
                            pending.cloud_id
                        );
                        if let Err(e) = pending_repo.remove(&pending.cloud_id) {
                            tracing::error!("[同步] 移除待删除记录失败: {}", e);
                        }
                        deleted.push(pending.cloud_id);
                    } else {
                        tracing::warn!(
                            "[同步] 删除云端工作空间失败: {} - {}（保留在待删除列表）",
                            pending.cloud_id,
                            e
                        );
                    }
                }
            }
        }

        deleted
    }

    fn process_cloud_soft_deleted_workspaces(
        &self,
        cloud_workspaces: &[CloudWorkspace],
        local_workspaces: &[Workspace],
    ) -> Result<usize, SyncError> {
        let repo = self
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        let mut deleted_count = 0;

        for cloud_ws in cloud_workspaces {
            if cloud_ws.deleted_at.is_some() {
                if let Some(local_ws) = local_workspaces
                    .iter()
                    .find(|ws| ws.cloud_id.as_ref() == Some(&cloud_ws.id))
                {
                    if let Some(local_id) = local_ws.id {
                        tracing::info!(
                            "[软删除] 云端工作空间 {} 已被删除，删除对应的本地工作空间 {}",
                            cloud_ws.name,
                            local_id
                        );
                        if let Err(e) = repo.delete(local_id) {
                            tracing::error!("[软删除] 删除本地工作空间失败: {} - {}", local_id, e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    fn get_pending_deletion_workspace_cloud_ids(&self) -> HashSet<String> {
        let pending_repo = match self.storage.get::<PendingCloudDeletionRepository>() {
            Some(repo) => repo,
            None => return HashSet::new(),
        };

        match pending_repo.list_workspaces() {
            Ok(list) => list.into_iter().map(|p| p.cloud_id).collect(),
            Err(_) => HashSet::new(),
        }
    }

    fn get_local_workspaces(&self) -> Result<Vec<Workspace>, SyncError> {
        let repo = self
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.list()
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }

    async fn upload_workspace(&self, ws: &Workspace) -> Result<String, SyncError> {
        let cloud_ws = CloudWorkspace {
            id: String::new(),
            local_id: ws.id,
            name: ws.name.clone(),
            color: ws.color.clone(),
            icon: ws.icon.clone(),
            updated_at: ws.updated_at.unwrap_or(0) * 1000,
            deleted_at: None,
        };

        let created = self
            .cloud_client
            .create_workspace(&cloud_ws)
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

        Ok(created.id)
    }

    async fn update_cloud_workspace(
        &self,
        local_ws: &Workspace,
        cloud_ws: &CloudWorkspace,
    ) -> Result<(), SyncError> {
        let updated_cloud_ws = CloudWorkspace {
            id: cloud_ws.id.clone(),
            local_id: local_ws.id,
            name: local_ws.name.clone(),
            color: local_ws.color.clone(),
            icon: local_ws.icon.clone(),
            updated_at: local_ws.updated_at.unwrap_or(0) * 1000,
            deleted_at: None,
        };

        self.cloud_client
            .update_workspace(&updated_cloud_ws)
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

        Ok(())
    }

    async fn download_workspace(&self, cloud_ws: &CloudWorkspace) -> Result<(), SyncError> {
        let mut local_ws = Workspace {
            id: None,
            name: cloud_ws.name.clone(),
            color: cloud_ws.color.clone(),
            icon: cloud_ws.icon.clone(),
            created_at: None,
            updated_at: Some(cloud_ws.updated_at / 1000),
            cloud_id: Some(cloud_ws.id.clone()),
        };

        let repo = self
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        let new_id = repo
            .insert(&mut local_ws)
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        let updated_cloud_ws = CloudWorkspace {
            id: cloud_ws.id.clone(),
            local_id: Some(new_id),
            name: cloud_ws.name.clone(),
            color: cloud_ws.color.clone(),
            icon: cloud_ws.icon.clone(),
            updated_at: cloud_ws.updated_at,
            deleted_at: cloud_ws.deleted_at,
        };

        self.cloud_client
            .update_workspace(&updated_cloud_ws)
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

        Ok(())
    }

    async fn update_local_workspace(
        &self,
        cloud_ws: &CloudWorkspace,
        local_ws: &Workspace,
    ) -> Result<(), SyncError> {
        let updated = Workspace {
            id: local_ws.id,
            name: cloud_ws.name.clone(),
            color: cloud_ws.color.clone(),
            icon: cloud_ws.icon.clone(),
            created_at: local_ws.created_at,
            updated_at: Some(cloud_ws.updated_at / 1000),
            cloud_id: Some(cloud_ws.id.clone()),
        };

        let repo = self
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.update_from_cloud(&updated)
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn delete_cloud_workspace(&self, cloud_id: &str) -> Result<(), SyncError> {
        self.cloud_client
            .delete_workspace(cloud_id)
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

        Ok(())
    }

    fn delete_local_workspace(&self, local_id: i64) -> Result<(), SyncError> {
        let repo = self
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.delete(local_id)
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }

    fn update_workspace_cloud_id(&self, local_id: i64, cloud_id: &str) -> Result<(), SyncError> {
        let repo = self
            .storage
            .get::<WorkspaceRepository>()
            .ok_or_else(|| SyncError::StorageError("WorkspaceRepository not found".to_string()))?;

        repo.update_cloud_id(local_id, Some(cloud_id.to_string()))
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }
}
