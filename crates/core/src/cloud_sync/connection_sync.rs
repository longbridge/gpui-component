use crate::cloud_sync::engine::{SyncEngine, SyncFuture, SyncHandler};
use crate::cloud_sync::models::{
    CloudConnection, ResolvedConflict, SyncConflict, SyncPlan, SyncResult,
};
use crate::cloud_sync::queue::SyncOperation;
use crate::cloud_sync::service::SyncError;
use crate::storage::traits::Repository;
use crate::storage::{ConnectionRepository, PendingCloudDeletionRepository, StoredConnection};
use std::collections::{HashMap, HashSet};

const CONNECTION_QUEUE_KEY: &str = "connection";

pub struct ConnectionSyncHandler;

impl SyncHandler for ConnectionSyncHandler {
    fn name(&self) -> &'static str {
        "连接"
    }

    fn sync<'a>(&'a self, engine: &'a SyncEngine) -> SyncFuture<'a> {
        Box::pin(async move { engine.sync_connections().await })
    }
}

impl SyncEngine {
    async fn sync_connections(&self) -> Result<SyncResult, SyncError> {
        let mut result = SyncResult::default();

        let pending_deletions = self.process_pending_deletions().await;
        tracing::info!("[同步] 处理待删除列表完成: {} 个", pending_deletions.len());

        let local_connections = self.get_local_connections()?;
        let sync_enabled_count = local_connections.iter().filter(|c| c.sync_enabled).count();
        tracing::info!(
            "[同步] 本地连接: {} 个，其中 {} 个启用同步",
            local_connections.len(),
            sync_enabled_count
        );

        tracing::info!("[同步] 正在获取云端连接列表...");
        let cloud_connections = self
            .cloud_client
            .list_connections()
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;
        tracing::info!("[同步] 云端连接: {} 个", cloud_connections.len());

        let deleted_count =
            self.process_cloud_soft_deleted(&cloud_connections, &local_connections)?;
        if deleted_count > 0 {
            tracing::info!("[同步] 处理云端软删除: 删除了 {} 个本地连接", deleted_count);
            result.deleted += deleted_count;
        }

        let active_cloud_connections: Vec<_> = cloud_connections
            .into_iter()
            .filter(|c| c.deleted_at.is_none())
            .collect();
        tracing::info!("[同步] 活跃云端连接: {} 个", active_cloud_connections.len());

        let plan = self.calculate_sync_plan(&local_connections, &active_cloud_connections)?;
        tracing::info!(
            "[同步计划] 上传: {}, 更新云端: {}, 下载: {}, 更新本地: {}, 冲突: {}",
            plan.to_upload.len(),
            plan.to_update_cloud.len(),
            plan.to_download.len(),
            plan.to_update_local.len(),
            plan.conflicts.len()
        );

        let resolved_conflicts = self.resolve_conflicts(&plan.conflicts)?;
        result.conflicts = plan.conflicts.clone();

        let local_connection_map: HashMap<i64, StoredConnection> = local_connections
            .iter()
            .filter_map(|conn| conn.id.map(|id| (id, conn.clone())))
            .collect();
        let cloud_connection_map: HashMap<String, CloudConnection> = active_cloud_connections
            .iter()
            .map(|conn| (conn.id.clone(), conn.clone()))
            .collect();

        let mut operations = Vec::new();
        for local_conn in &plan.to_upload {
            if let Some(local_id) = local_conn.id {
                operations.push(SyncOperation::Upload { local_id });
            } else {
                result
                    .errors
                    .push(format!("上传失败 {}: 缺少本地 ID", local_conn.name));
            }
        }

        for (local_conn, cloud_conn) in &plan.to_update_cloud {
            if let Some(local_id) = local_conn.id {
                operations.push(SyncOperation::UpdateCloud {
                    local_id,
                    cloud_id: cloud_conn.id.clone(),
                });
            } else {
                result
                    .errors
                    .push(format!("更新云端失败 {}: 缺少本地 ID", local_conn.name));
            }
        }

        for cloud_conn in &plan.to_download {
            operations.push(SyncOperation::Download(cloud_conn.id.clone()));
        }

        for (cloud_conn, local_conn) in &plan.to_update_local {
            if let Some(local_id) = local_conn.id {
                operations.push(SyncOperation::UpdateLocal {
                    local_id,
                    cloud_id: cloud_conn.id.clone(),
                });
            } else {
                result
                    .errors
                    .push(format!("更新本地失败 {}: 缺少本地 ID", cloud_conn.name));
            }
        }

        let mut queue = self.take_operation_queue(CONNECTION_QUEUE_KEY)?;
        queue.enqueue_all(operations);
        queue.retry_failed();

        while let Some(queued_operation) = queue.dequeue() {
            let operation = queued_operation.operation.clone();
            match operation {
                SyncOperation::Upload { local_id } => {
                    let Some(local_conn) = local_connection_map.get(&local_id) else {
                        result
                            .errors
                            .push(format!("上传失败 {}: 本地数据不存在", local_id));
                        continue;
                    };
                    match self.upload_connection(local_conn).await {
                        Ok(cloud_id) => {
                            match self.update_sync_status(local_id, Some(cloud_id), None) {
                                Ok(()) => {
                                    result.uploaded += 1;
                                    tracing::info!("[上传] 成功: {}", local_conn.name);
                                }
                                Err(e) => {
                                    let error_message =
                                        format!("上传失败 {}: {}", local_conn.name, e);
                                    result.errors.push(error_message.clone());
                                    queue.mark_failed(queued_operation, error_message);
                                }
                            }
                        }
                        Err(e) => {
                            let error_message = format!("上传失败 {}: {}", local_conn.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::UpdateCloud { local_id, cloud_id } => {
                    let Some(local_conn) = local_connection_map.get(&local_id) else {
                        result
                            .errors
                            .push(format!("更新云端失败 {}: 本地数据不存在", local_id));
                        continue;
                    };
                    let Some(cloud_conn) = cloud_connection_map.get(&cloud_id) else {
                        result
                            .errors
                            .push(format!("更新云端失败 {}: 云端数据不存在", cloud_id));
                        continue;
                    };
                    match self.update_cloud_connection(local_conn, cloud_conn).await {
                        Ok(()) => {
                            match self.update_sync_status(
                                local_id,
                                Some(cloud_conn.id.clone()),
                                None,
                            ) {
                                Ok(()) => {
                                    result.uploaded += 1;
                                    tracing::info!("[更新云端] 成功: {}", local_conn.name);
                                }
                                Err(e) => {
                                    let error_message =
                                        format!("更新云端失败 {}: {}", local_conn.name, e);
                                    result.errors.push(error_message.clone());
                                    queue.mark_failed(queued_operation, error_message);
                                }
                            }
                        }
                        Err(e) => {
                            let error_message = format!("更新云端失败 {}: {}", local_conn.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::UpdateLocal { local_id, cloud_id } => {
                    let Some(local_conn) = local_connection_map.get(&local_id) else {
                        result
                            .errors
                            .push(format!("更新本地失败 {}: 本地数据不存在", local_id));
                        continue;
                    };
                    let Some(cloud_conn) = cloud_connection_map.get(&cloud_id) else {
                        result
                            .errors
                            .push(format!("更新本地失败 {}: 云端数据不存在", cloud_id));
                        continue;
                    };
                    match self.update_local_connection(cloud_conn, local_conn).await {
                        Ok(()) => {
                            result.downloaded += 1;
                            tracing::info!("[更新本地] 成功: {}", cloud_conn.name);
                        }
                        Err(e) => {
                            let error_message = format!("更新本地失败 {}: {}", cloud_conn.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::Download(cloud_id) => {
                    let Some(cloud_conn) = cloud_connection_map.get(&cloud_id) else {
                        result
                            .errors
                            .push(format!("下载失败 {}: 云端数据不存在", cloud_id));
                        continue;
                    };
                    match self.download_connection(cloud_conn).await {
                        Ok(()) => {
                            result.downloaded += 1;
                            tracing::info!("[下载] 成功: {}", cloud_conn.name);
                        }
                        Err(e) => {
                            let error_message = format!("下载失败 {}: {}", cloud_conn.name, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::DeleteCloud(cloud_id) => {
                    match self.delete_cloud_connection(&cloud_id).await {
                        Ok(()) => {
                            result.deleted += 1;
                            tracing::info!("[删除云端] 成功: {}", cloud_id);
                        }
                        Err(e) => {
                            let error_message = format!("删除云端失败 {}: {}", cloud_id, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
                SyncOperation::DeleteLocal(local_id) => {
                    match self.delete_local_connection(local_id) {
                        Ok(()) => {
                            result.deleted += 1;
                            tracing::info!("[删除本地] 成功: {}", local_id);
                        }
                        Err(e) => {
                            let error_message = format!("删除本地失败 {}: {}", local_id, e);
                            result.errors.push(error_message.clone());
                            queue.mark_failed(queued_operation, error_message);
                        }
                    }
                }
            }
        }

        for resolved in &resolved_conflicts {
            match self.apply_resolved_conflict(resolved).await {
                Ok(()) => {
                    tracing::info!("[冲突解决] 成功应用");
                }
                Err(e) => {
                    result.errors.push(format!("应用冲突解决失败: {}", e));
                }
            }
        }

        self.store_operation_queue(CONNECTION_QUEUE_KEY, queue)?;

        Ok(result)
    }

    fn get_local_connections(&self) -> Result<Vec<StoredConnection>, SyncError> {
        let repo = self
            .storage
            .get::<ConnectionRepository>()
            .ok_or_else(|| SyncError::StorageError("ConnectionRepository not found".to_string()))?;

        repo.list()
            .map_err(|e| SyncError::StorageError(e.to_string()))
    }

    async fn process_pending_deletions(&self) -> Vec<String> {
        let mut deleted = Vec::new();

        let pending_repo = match self.storage.get::<PendingCloudDeletionRepository>() {
            Some(repo) => repo,
            None => {
                tracing::warn!("[同步] PendingCloudDeletionRepository not found");
                return deleted;
            }
        };

        let pending_list = match pending_repo.list_connections() {
            Ok(list) => list,
            Err(e) => {
                tracing::error!("[同步] 获取待删除列表失败: {}", e);
                return deleted;
            }
        };

        for pending in pending_list {
            tracing::info!("[同步] 处理待删除云端连接: {}", pending.cloud_id);
            match self.cloud_client.delete_connection(&pending.cloud_id).await {
                Ok(_) => {
                    tracing::info!("[同步] 云端连接删除成功: {}", pending.cloud_id);
                    if let Err(e) = pending_repo.remove(&pending.cloud_id) {
                        tracing::error!("[同步] 移除待删除记录失败: {}", e);
                    }
                    deleted.push(pending.cloud_id);
                }
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("404") || error_str.contains("not found") {
                        tracing::info!(
                            "[同步] 云端连接已不存在，移除待删除记录: {}",
                            pending.cloud_id
                        );
                        if let Err(e) = pending_repo.remove(&pending.cloud_id) {
                            tracing::error!("[同步] 移除待删除记录失败: {}", e);
                        }
                        deleted.push(pending.cloud_id);
                    } else {
                        tracing::warn!(
                            "[同步] 删除云端连接失败: {} - {}（保留在待删除列表）",
                            pending.cloud_id,
                            e
                        );
                    }
                }
            }
        }

        deleted
    }

    fn process_cloud_soft_deleted(
        &self,
        cloud_connections: &[CloudConnection],
        local_connections: &[StoredConnection],
    ) -> Result<usize, SyncError> {
        let repo = self
            .storage
            .get::<ConnectionRepository>()
            .ok_or_else(|| SyncError::StorageError("ConnectionRepository not found".to_string()))?;

        let mut deleted_count = 0;

        for cloud_conn in cloud_connections {
            if cloud_conn.deleted_at.is_some() {
                if let Some(local_conn) = local_connections
                    .iter()
                    .find(|c| c.cloud_id.as_ref() == Some(&cloud_conn.id))
                {
                    if let Some(local_id) = local_conn.id {
                        tracing::info!(
                            "[软删除] 云端连接 {} 已被删除，删除对应的本地连接 {}",
                            cloud_conn.name,
                            local_id
                        );
                        if let Err(e) = repo.delete(local_id) {
                            tracing::error!("[软删除] 删除本地连接失败: {} - {}", local_id, e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    fn calculate_sync_plan(
        &self,
        local_connections: &[StoredConnection],
        cloud_connections: &[CloudConnection],
    ) -> Result<SyncPlan, SyncError> {
        let mut plan = SyncPlan::default();

        let cloud_map: HashMap<&str, &CloudConnection> = cloud_connections
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect();

        let local_cloud_ids: HashSet<String> = local_connections
            .iter()
            .filter_map(|c| c.cloud_id.clone())
            .collect();

        let local_unlinked_by_name: HashMap<&str, &StoredConnection> = local_connections
            .iter()
            .filter(|c| c.cloud_id.is_none() && c.sync_enabled)
            .map(|c| (c.name.as_str(), c))
            .collect();

        for local_conn in local_connections {
            if !local_conn.sync_enabled {
                continue;
            }

            match &local_conn.cloud_id {
                Some(cloud_id) => {
                    if let Some(cloud_conn) = cloud_map.get(cloud_id.as_str()) {
                        let local_updated = local_conn.updated_at.unwrap_or(0);
                        let last_synced = local_conn.last_synced_at.unwrap_or(0);
                        let cloud_updated = cloud_conn.updated_at / 1000;

                        let local_changed = local_updated > last_synced;
                        let cloud_changed = cloud_updated > last_synced;

                        match (local_changed, cloud_changed) {
                            (true, true) => {
                                plan.conflicts.push(SyncConflict {
                                    local: local_conn.clone(),
                                    cloud: (*cloud_conn).clone(),
                                    conflict_type:
                                        crate::cloud_sync::models::ConflictType::BothModified,
                                });
                            }
                            (true, false) => {
                                plan.to_update_cloud
                                    .push((local_conn.clone(), (*cloud_conn).clone()));
                            }
                            (false, true) => {
                                plan.to_update_local
                                    .push(((*cloud_conn).clone(), local_conn.clone()));
                            }
                            (false, false) => {}
                        }
                    } else {
                        plan.conflicts.push(SyncConflict {
                            local: local_conn.clone(),
                            cloud: CloudConnection {
                                id: cloud_id.clone(),
                                local_id: local_conn.id,
                                name: local_conn.name.clone(),
                                connection_type: local_conn.connection_type.to_string(),
                                workspace_id: local_conn.workspace_id.map(|id| id.to_string()),
                                encrypted_params: String::new(),
                                key_version: 0,
                                updated_at: 0,
                                checksum: String::new(),
                                deleted_at: None,
                            },
                            conflict_type:
                                crate::cloud_sync::models::ConflictType::LocalModifiedCloudDeleted,
                        });
                    }
                }
                None => {
                    let has_cloud_match = cloud_connections
                        .iter()
                        .any(|cc| cc.name == local_conn.name);
                    if !has_cloud_match {
                        plan.to_upload.push(local_conn.clone());
                    }
                }
            }
        }

        let pending_cloud_ids = self.get_pending_deletion_cloud_ids();
        for cloud_conn in cloud_connections {
            if !local_cloud_ids.contains(&cloud_conn.id) {
                if pending_cloud_ids.contains(&cloud_conn.id) {
                    tracing::info!("[同步计划] 跳过待删除的云端连接: {}", cloud_conn.name);
                    continue;
                }

                if let Some(local_conn) = local_unlinked_by_name.get(cloud_conn.name.as_str()) {
                    tracing::info!(
                        "[同步计划] 按名称匹配连接: {} (云端 {} -> 本地 {:?})",
                        cloud_conn.name,
                        cloud_conn.id,
                        local_conn.id
                    );
                    plan.to_update_local
                        .push((cloud_conn.clone(), (*local_conn).clone()));
                } else {
                    plan.to_download.push(cloud_conn.clone());
                }
            }
        }

        Ok(plan)
    }

    fn resolve_conflicts(
        &self,
        conflicts: &[SyncConflict],
    ) -> Result<Vec<ResolvedConflict>, SyncError> {
        let mut resolved = Vec::new();

        for conflict in conflicts {
            match self.conflict_strategy {
                crate::cloud_sync::models::ConflictResolution::UseCloud => {
                    resolved.push(ResolvedConflict {
                        conflict: conflict.clone(),
                        resolution: crate::cloud_sync::models::ConflictResolution::UseCloud,
                        result_connection: None,
                    });
                }
                crate::cloud_sync::models::ConflictResolution::UseLocal => {
                    resolved.push(ResolvedConflict {
                        conflict: conflict.clone(),
                        resolution: crate::cloud_sync::models::ConflictResolution::UseLocal,
                        result_connection: Some(conflict.local.clone()),
                    });
                }
                crate::cloud_sync::models::ConflictResolution::KeepBoth => {
                    let mut copy = conflict.local.clone();
                    copy.id = None;
                    copy.cloud_id = None;
                    let timestamp = Self::current_timestamp();
                    copy.name = format!("{} (冲突副本 {})", copy.name, timestamp);

                    resolved.push(ResolvedConflict {
                        conflict: conflict.clone(),
                        resolution: crate::cloud_sync::models::ConflictResolution::KeepBoth,
                        result_connection: Some(copy),
                    });
                }
            }
        }

        Ok(resolved)
    }

    async fn upload_connection(&self, conn: &StoredConnection) -> Result<String, SyncError> {
        let cloud_conn = {
            let service = self
                .crypto_service
                .read()
                .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;
            service.prepare_upload(conn)?
        };
        let created = self
            .cloud_client
            .create_connection(&cloud_conn)
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

        Ok(created.id)
    }

    async fn update_cloud_connection(
        &self,
        local_conn: &StoredConnection,
        cloud_conn: &CloudConnection,
    ) -> Result<(), SyncError> {
        let updated_cloud_conn = {
            let service = self
                .crypto_service
                .read()
                .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;
            let mut updated_cloud_conn = service.prepare_upload(local_conn)?;
            updated_cloud_conn.id = cloud_conn.id.clone();
            updated_cloud_conn
        };

        self.cloud_client
            .update_connection(&updated_cloud_conn)
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

        Ok(())
    }

    async fn download_connection(&self, cloud_conn: &CloudConnection) -> Result<(), SyncError> {
        let service = self
            .crypto_service
            .read()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

        let mut local_conn = service.decrypt_connection(cloud_conn)?;
        local_conn.id = None;
        local_conn.cloud_id = Some(cloud_conn.id.clone());
        local_conn.last_synced_at = Some(Self::current_timestamp());

        let repo = self
            .storage
            .get::<ConnectionRepository>()
            .ok_or_else(|| SyncError::StorageError("ConnectionRepository not found".to_string()))?;

        repo.insert(&mut local_conn)
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn update_local_connection(
        &self,
        cloud_conn: &CloudConnection,
        local_conn: &StoredConnection,
    ) -> Result<(), SyncError> {
        let service = self
            .crypto_service
            .read()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

        let mut updated = service.decrypt_connection(cloud_conn)?;
        updated.id = local_conn.id;
        updated.cloud_id = Some(cloud_conn.id.clone());
        updated.last_synced_at = Some(Self::current_timestamp());

        let repo = self
            .storage
            .get::<ConnectionRepository>()
            .ok_or_else(|| SyncError::StorageError("ConnectionRepository not found".to_string()))?;

        repo.update(&updated)
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        Ok(())
    }

    fn update_sync_status(
        &self,
        local_id: i64,
        cloud_id: Option<String>,
        error: Option<String>,
    ) -> Result<(), SyncError> {
        let repo = self
            .storage
            .get::<ConnectionRepository>()
            .ok_or_else(|| SyncError::StorageError("ConnectionRepository not found".to_string()))?;

        let now = Self::current_timestamp();
        repo.update_sync_status(local_id, cloud_id, Some(now))
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        if let Some(err) = error {
            tracing::warn!("[同步状态] 连接 {} 同步出错: {}", local_id, err);
        }

        Ok(())
    }

    async fn apply_resolved_conflict(&self, resolved: &ResolvedConflict) -> Result<(), SyncError> {
        match resolved.resolution {
            crate::cloud_sync::models::ConflictResolution::UseCloud => {
                self.update_local_connection(&resolved.conflict.cloud, &resolved.conflict.local)
                    .await
            }
            crate::cloud_sync::models::ConflictResolution::UseLocal => {
                self.update_cloud_connection(&resolved.conflict.local, &resolved.conflict.cloud)
                    .await
            }
            crate::cloud_sync::models::ConflictResolution::KeepBoth => {
                if let Some(copy) = &resolved.result_connection {
                    let repo = self.storage.get::<ConnectionRepository>().ok_or_else(|| {
                        SyncError::StorageError("ConnectionRepository not found".to_string())
                    })?;

                    let mut new_conn = copy.clone();
                    repo.insert(&mut new_conn)
                        .map_err(|e| SyncError::StorageError(e.to_string()))?;
                }
                self.update_local_connection(&resolved.conflict.cloud, &resolved.conflict.local)
                    .await
            }
        }
    }

    pub async fn delete_cloud_connection(&self, cloud_id: &str) -> Result<(), SyncError> {
        self.cloud_client
            .delete_connection(cloud_id)
            .await
            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

        Ok(())
    }

    pub fn delete_local_connection(&self, local_id: i64) -> Result<(), SyncError> {
        let repo = self
            .storage
            .get::<ConnectionRepository>()
            .ok_or_else(|| SyncError::StorageError("ConnectionRepository not found".to_string()))?;

        repo.delete(local_id)
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_connection(&self, local_id: i64) -> Result<(), SyncError> {
        let repo = self
            .storage
            .get::<ConnectionRepository>()
            .ok_or_else(|| SyncError::StorageError("ConnectionRepository not found".to_string()))?;

        let connections = repo
            .list()
            .map_err(|e| SyncError::StorageError(e.to_string()))?;

        let conn = connections
            .iter()
            .find(|c| c.id == Some(local_id))
            .ok_or_else(|| SyncError::StorageError(format!("连接 {} 不存在", local_id)))?;

        if let Some(cloud_id) = &conn.cloud_id {
            if let Err(e) = self.delete_cloud_connection(cloud_id).await {
                tracing::warn!("[删除] 云端删除失败: {} - {}（继续删除本地）", cloud_id, e);
            }
        }

        self.delete_local_connection(local_id)
    }

    fn get_pending_deletion_cloud_ids(&self) -> HashSet<String> {
        let pending_repo = match self.storage.get::<PendingCloudDeletionRepository>() {
            Some(repo) => repo,
            None => return HashSet::new(),
        };

        match pending_repo.list_connections() {
            Ok(list) => list.into_iter().map(|p| p.cloud_id).collect(),
            Err(_) => HashSet::new(),
        }
    }
}
