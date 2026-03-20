//! 通用同步流程
//!
//! 为实现了 `SyncTypeHandler` 的数据类型提供统一的同步逻辑，
//! 涵盖：待删除处理 → 数据拉取 → 软删除 → 同步计划 → 操作队列 → 执行。

use crate::cloud_sync::engine::SyncEngine;
use crate::cloud_sync::models::{CloudSyncData, SyncResult};
use crate::cloud_sync::queue::SyncOperation;
use crate::cloud_sync::service::SyncError;
use crate::cloud_sync::sync_type::{GenericSyncPlan, SyncTypeHandler, SyncableItem};
use std::collections::{HashMap, HashSet};

/// 通用同步入口
///
/// 按统一流程同步指定类型的数据：
/// 1. 处理待删除列表
/// 2. 获取本地 / 云端数据
/// 3. 过滤未解锁团队数据
/// 4. 构建名称映射
/// 5. 处理云端软删除
/// 6. 计算同步计划
/// 7. 构建并执行操作队列
pub(crate) async fn generic_sync<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
) -> Result<SyncResult, SyncError> {
    let type_name = handler.display_name();
    let mut result = SyncResult::default();

    // ========== 1. 处理待删除列表 ==========
    let deleted_ids = process_pending_deletions(engine, handler).await;
    tracing::info!(
        "[{}] 处理待删除列表完成: {} 个",
        type_name,
        deleted_ids.len()
    );

    // ========== 2. 获取本地数据 ==========
    let local_items = handler.list_local(engine)?;
    tracing::info!("[{}] 本地数据: {} 个", type_name, local_items.len());

    // ========== 3. 获取云端数据（按 data_type 过滤） ==========
    let cloud_sync_data = engine
        .cloud_client
        .list_sync_data(Some(handler.data_type()), None, None)
        .await
        .map_err(|e| SyncError::NetworkError(e.to_string()))?;
    tracing::info!("[{}] 云端同步数据: {} 个", type_name, cloud_sync_data.len());

    // ========== 4. 过滤未解锁团队数据 ==========
    let cloud_sync_data: Vec<_> = cloud_sync_data
        .into_iter()
        .filter(|d| match &d.team_id {
            Some(tid) => {
                let unlocked = engine.is_team_unlocked(tid);
                if !unlocked {
                    tracing::info!("[{}] 跳过未解锁团队 {} 的云端数据 {}", type_name, tid, d.id);
                }
                unlocked
            }
            None => true,
        })
        .collect();
    tracing::info!(
        "[{}] 可处理的云端数据: {} 个",
        type_name,
        cloud_sync_data.len()
    );

    // ========== 5. 构建名称映射 ==========
    let cloud_name_map = build_name_map(engine, handler, &cloud_sync_data);

    // ========== 6. 处理云端软删除 ==========
    let soft_deleted = process_soft_deletions(engine, handler, &cloud_sync_data, &local_items)?;
    if soft_deleted > 0 {
        tracing::info!(
            "[{}] 处理云端软删除: 删除了 {} 个本地数据",
            type_name,
            soft_deleted
        );
        result.deleted += soft_deleted;
    }

    // 过滤出活跃数据
    let active_cloud_data: Vec<_> = cloud_sync_data
        .into_iter()
        .filter(|d| d.deleted_at.is_none())
        .collect();
    tracing::info!(
        "[{}] 活跃云端数据: {} 个",
        type_name,
        active_cloud_data.len()
    );

    // ========== 7. 计算同步计划 ==========
    let plan = calculate_sync_plan(
        engine,
        handler,
        &local_items,
        &active_cloud_data,
        &cloud_name_map,
    )?;
    tracing::info!(
        "[{}计划] 上传: {}, 更新云端: {}, 下载: {}, 更新本地: {}",
        type_name,
        plan.to_upload.len(),
        plan.to_update_cloud.len(),
        plan.to_download.len(),
        plan.to_update_local.len()
    );

    // ========== 8. 构建操作队列 ==========
    let local_item_map: HashMap<i64, H::Item> = local_items
        .iter()
        .filter_map(|item| item.local_id().map(|id| (id, item.clone())))
        .collect();
    let cloud_data_map: HashMap<String, CloudSyncData> = active_cloud_data
        .iter()
        .map(|d| (d.id.clone(), d.clone()))
        .collect();

    let mut operations = Vec::new();
    for item in &plan.to_upload {
        if let Some(local_id) = item.local_id() {
            operations.push(SyncOperation::Upload { local_id });
        } else {
            result.errors.push(format!(
                "上传{}失败 {}: 缺少本地 ID",
                type_name,
                item.item_name()
            ));
        }
    }

    for (item, cloud_data) in &plan.to_update_cloud {
        if let Some(local_id) = item.local_id() {
            operations.push(SyncOperation::UpdateCloud {
                local_id,
                cloud_id: cloud_data.id.clone(),
            });
        } else {
            result.errors.push(format!(
                "更新云端{}失败 {}: 缺少本地 ID",
                type_name,
                item.item_name()
            ));
        }
    }

    for cloud_data in &plan.to_download {
        operations.push(SyncOperation::Download(cloud_data.id.clone()));
    }

    for (cloud_data, item) in &plan.to_update_local {
        if let Some(local_id) = item.local_id() {
            operations.push(SyncOperation::UpdateLocal {
                local_id,
                cloud_id: cloud_data.id.clone(),
            });
        } else {
            let name = cloud_name_map
                .get(&cloud_data.id)
                .cloned()
                .unwrap_or_else(|| cloud_data.id.clone());
            result
                .errors
                .push(format!("更新本地{}失败 {}: 缺少本地 ID", type_name, name));
        }
    }

    // ========== 9. 执行操作 ==========
    let mut queue = engine.take_operation_queue(handler.queue_key())?;
    queue.enqueue_all(operations);
    queue.retry_failed();

    while let Some(queued_operation) = queue.dequeue() {
        let operation = queued_operation.operation.clone();
        match operation {
            SyncOperation::Upload { local_id } => {
                let Some(local_item) = local_item_map.get(&local_id) else {
                    result.errors.push(format!(
                        "上传{}失败 {}: 本地数据不存在",
                        type_name, local_id
                    ));
                    continue;
                };

                match upload_item(engine, handler, local_item).await {
                    Ok(cloud_id) => match handler.on_uploaded(engine, local_id, &cloud_id) {
                        Ok(()) => {
                            result.uploaded += 1;
                            tracing::info!("[上传{}] 成功: {}", type_name, local_item.item_name());
                        }
                        Err(e) => {
                            let msg =
                                format!("上传{}失败 {}: {}", type_name, local_item.item_name(), e);
                            result.errors.push(msg.clone());
                            queue.mark_failed(queued_operation, msg);
                        }
                    },
                    Err(e) => {
                        let msg =
                            format!("上传{}失败 {}: {}", type_name, local_item.item_name(), e);
                        result.errors.push(msg.clone());
                        queue.mark_failed(queued_operation, msg);
                    }
                }
            }
            SyncOperation::UpdateCloud { local_id, cloud_id } => {
                let Some(local_item) = local_item_map.get(&local_id) else {
                    result.errors.push(format!(
                        "更新云端{}失败 {}: 本地数据不存在",
                        type_name, local_id
                    ));
                    continue;
                };
                let Some(cloud_data) = cloud_data_map.get(&cloud_id) else {
                    result.errors.push(format!(
                        "更新云端{}失败 {}: 云端数据不存在",
                        type_name, cloud_id
                    ));
                    continue;
                };

                match update_cloud_item(engine, handler, local_item, cloud_data).await {
                    Ok(()) => {
                        result.uploaded += 1;
                        tracing::info!("[更新云端{}] 成功: {}", type_name, local_item.item_name());
                    }
                    Err(e) => {
                        let msg = format!(
                            "更新云端{}失败 {}: {}",
                            type_name,
                            local_item.item_name(),
                            e
                        );
                        result.errors.push(msg.clone());
                        queue.mark_failed(queued_operation, msg);
                    }
                }
            }
            SyncOperation::UpdateLocal { local_id, cloud_id } => {
                let Some(local_item) = local_item_map.get(&local_id) else {
                    result.errors.push(format!(
                        "更新本地{}失败 {}: 本地数据不存在",
                        type_name, local_id
                    ));
                    continue;
                };
                let Some(cloud_data) = cloud_data_map.get(&cloud_id) else {
                    result.errors.push(format!(
                        "更新本地{}失败 {}: 云端数据不存在",
                        type_name, cloud_id
                    ));
                    continue;
                };

                let name = cloud_name_map
                    .get(&cloud_data.id)
                    .cloned()
                    .unwrap_or_else(|| cloud_data.id.clone());
                match download_and_update_item(engine, handler, cloud_data, local_item).await {
                    Ok(()) => {
                        result.downloaded += 1;
                        tracing::info!("[更新本地{}] 成功: {}", type_name, name);
                    }
                    Err(e) => {
                        let msg = format!("更新本地{}失败 {}: {}", type_name, name, e);
                        result.errors.push(msg.clone());
                        queue.mark_failed(queued_operation, msg);
                    }
                }
            }
            SyncOperation::Download(cloud_id) => {
                let Some(cloud_data) = cloud_data_map.get(&cloud_id) else {
                    result.errors.push(format!(
                        "下载{}失败 {}: 云端数据不存在",
                        type_name, cloud_id
                    ));
                    continue;
                };

                let name = cloud_name_map
                    .get(&cloud_data.id)
                    .cloned()
                    .unwrap_or_else(|| cloud_data.id.clone());
                match download_item(engine, handler, cloud_data).await {
                    Ok(()) => {
                        result.downloaded += 1;
                        tracing::info!("[下载{}] 成功: {}", type_name, name);
                    }
                    Err(e) => {
                        let msg = format!("下载{}失败 {}: {}", type_name, name, e);
                        result.errors.push(msg.clone());
                        queue.mark_failed(queued_operation, msg);
                    }
                }
            }
            SyncOperation::DeleteCloud(cloud_id) => {
                match engine
                    .cloud_client
                    .delete_sync_data(&cloud_id)
                    .await
                    .map_err(|e| SyncError::NetworkError(e.to_string()))
                {
                    Ok(()) => {
                        result.deleted += 1;
                        tracing::info!("[删除云端{}] 成功: {}", type_name, cloud_id);
                    }
                    Err(e) => {
                        let msg = format!("删除云端{}失败 {}: {}", type_name, cloud_id, e);
                        result.errors.push(msg.clone());
                        queue.mark_failed(queued_operation, msg);
                    }
                }
            }
            SyncOperation::DeleteLocal(local_id) => match handler.delete_local(engine, local_id) {
                Ok(()) => {
                    result.deleted += 1;
                    tracing::info!("[删除本地{}] 成功: {}", type_name, local_id);
                }
                Err(e) => {
                    let msg = format!("删除本地{}失败 {}: {}", type_name, local_id, e);
                    result.errors.push(msg.clone());
                    queue.mark_failed(queued_operation, msg);
                }
            },
        }
    }

    // ========== 10. 保存队列 ==========
    engine.store_operation_queue(handler.queue_key(), queue)?;

    Ok(result)
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 处理待删除列表
async fn process_pending_deletions<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
) -> Vec<String> {
    let mut deleted = Vec::new();
    let pending_list = handler.list_pending_deletions(engine);

    for pending in pending_list {
        tracing::info!(
            "[同步] 处理待删除云端{}: {}",
            handler.display_name(),
            pending.cloud_id
        );
        match engine
            .cloud_client
            .delete_sync_data(&pending.cloud_id)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "[同步] 云端{}删除成功: {}",
                    handler.display_name(),
                    pending.cloud_id
                );
                if let Err(e) = handler.remove_pending_deletion(engine, &pending.cloud_id) {
                    tracing::error!("[同步] 移除待删除记录失败: {}", e);
                }
                deleted.push(pending.cloud_id);
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("404") || error_str.contains("not found") {
                    tracing::info!(
                        "[同步] 云端{}已不存在，移除待删除记录: {}",
                        handler.display_name(),
                        pending.cloud_id
                    );
                    if let Err(e) = handler.remove_pending_deletion(engine, &pending.cloud_id) {
                        tracing::error!("[同步] 移除待删除记录失败: {}", e);
                    }
                    deleted.push(pending.cloud_id);
                } else {
                    tracing::warn!(
                        "[同步] 删除云端{}失败: {} - {}（保留在待删除列表）",
                        handler.display_name(),
                        pending.cloud_id,
                        e
                    );
                }
            }
        }
    }

    deleted
}

/// 构建 cloud_id → name 映射
fn build_name_map<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
    cloud_data_list: &[CloudSyncData],
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let service = match engine.crypto_service.read() {
        Ok(s) => s,
        Err(_) => return map,
    };

    for data in cloud_data_list {
        if let Some(name) = handler.decrypt_name(&service, data) {
            map.insert(data.id.clone(), name);
        }
    }
    map
}

/// 处理云端软删除
fn process_soft_deletions<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
    cloud_data_list: &[CloudSyncData],
    local_items: &[H::Item],
) -> Result<usize, SyncError> {
    let mut deleted_count = 0;

    for cloud_data in cloud_data_list {
        if cloud_data.deleted_at.is_some() {
            if let Some(local_item) = local_items
                .iter()
                .find(|item| item.cloud_id() == Some(cloud_data.id.as_str()))
            {
                if let Some(local_id) = local_item.local_id() {
                    tracing::info!(
                        "[软删除] 云端{} {} 已被删除，删除对应的本地数据 {}",
                        handler.display_name(),
                        cloud_data.id,
                        local_id
                    );
                    match handler.delete_local(engine, local_id) {
                        Ok(()) => deleted_count += 1,
                        Err(e) => {
                            tracing::error!("[软删除] 删除本地数据失败: {} - {}", local_id, e);
                        }
                    }
                }
            }
        }
    }

    Ok(deleted_count)
}

/// 计算同步计划（通用版，按 updated_at 比较，无冲突检测）
fn calculate_sync_plan<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
    local_items: &[H::Item],
    cloud_data_list: &[CloudSyncData],
    cloud_name_map: &HashMap<String, String>,
) -> Result<GenericSyncPlan<H::Item>, SyncError> {
    let mut plan = GenericSyncPlan::default();

    let cloud_map: HashMap<&str, &CloudSyncData> =
        cloud_data_list.iter().map(|d| (d.id.as_str(), d)).collect();

    let local_cloud_ids: HashSet<String> = local_items
        .iter()
        .filter_map(|item| item.cloud_id().map(|s| s.to_string()))
        .collect();

    let local_unlinked_by_name: HashMap<&str, &H::Item> = local_items
        .iter()
        .filter(|item| item.cloud_id().is_none())
        .map(|item| (item.item_name(), item))
        .collect();

    let local_all_by_name: HashMap<&str, &H::Item> = local_items
        .iter()
        .map(|item| (item.item_name(), item))
        .collect();

    // 处理本地数据
    for local_item in local_items {
        match local_item.cloud_id() {
            Some(cloud_id) => {
                if let Some(cloud_data) = cloud_map.get(cloud_id) {
                    let local_updated = local_item.updated_at().unwrap_or(0);
                    let cloud_updated = cloud_data.updated_at / 1000;

                    if local_updated > cloud_updated {
                        plan.to_update_cloud
                            .push((local_item.clone(), (*cloud_data).clone()));
                    } else if cloud_updated > local_updated {
                        plan.to_update_local
                            .push(((*cloud_data).clone(), local_item.clone()));
                    }
                } else {
                    tracing::info!(
                        "[同步计划] {} '{}' 的云端记录 {} 不存在，重新加入上传计划",
                        handler.display_name(),
                        local_item.item_name(),
                        cloud_id
                    );
                    plan.to_upload.push(local_item.clone());
                }
            }
            None => {
                let has_cloud_match = cloud_name_map
                    .values()
                    .any(|name| name == local_item.item_name());
                if !has_cloud_match {
                    plan.to_upload.push(local_item.clone());
                }
            }
        }
    }

    // 处理云端新增数据
    let pending_cloud_ids = get_pending_cloud_ids(engine, handler);
    for cloud_data in cloud_data_list {
        if !local_cloud_ids.contains(&cloud_data.id) {
            if pending_cloud_ids.contains(&cloud_data.id) {
                let name = cloud_name_map
                    .get(&cloud_data.id)
                    .cloned()
                    .unwrap_or_else(|| cloud_data.id.clone());
                tracing::info!(
                    "[同步计划] 跳过待删除的云端{}: {}",
                    handler.display_name(),
                    name
                );
                continue;
            }

            let cloud_name = cloud_name_map
                .get(&cloud_data.id)
                .cloned()
                .unwrap_or_else(|| cloud_data.id.clone());

            if let Some(local_item) = local_unlinked_by_name.get(cloud_name.as_str()) {
                tracing::info!(
                    "[同步计划] 按名称匹配{}: {} (云端 {} -> 本地 {:?})",
                    handler.display_name(),
                    cloud_name,
                    cloud_data.id,
                    local_item.local_id()
                );
                plan.to_update_local
                    .push((cloud_data.clone(), (*local_item).clone()));
            } else if let Some(local_item) = local_all_by_name.get(cloud_name.as_str()) {
                // 检查本地已有同名数据是否有有效的 cloud_id
                let local_cloud_id_is_valid = local_item
                    .cloud_id()
                    .is_some_and(|cid| cloud_map.contains_key(cid));
                if !local_cloud_id_is_valid {
                    plan.to_update_local
                        .push((cloud_data.clone(), (*local_item).clone()));
                }
            } else {
                plan.to_download.push(cloud_data.clone());
            }
        }
    }

    Ok(plan)
}

/// 获取待删除的云端 ID 集合
fn get_pending_cloud_ids<H: SyncTypeHandler>(engine: &SyncEngine, handler: &H) -> HashSet<String> {
    handler
        .list_pending_deletions(engine)
        .into_iter()
        .map(|p| p.cloud_id)
        .collect()
}

/// 上传数据项到云端
async fn upload_item<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
    item: &H::Item,
) -> Result<String, SyncError> {
    let teams = engine.get_cached_teams();
    let cloud_data = {
        let service = engine
            .crypto_service
            .read()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;
        handler.encrypt(&service, item, &teams)?
    };

    let created = engine
        .cloud_client
        .create_sync_data(&cloud_data)
        .await
        .map_err(|e| SyncError::NetworkError(e.to_string()))?;

    Ok(created.id)
}

/// 更新云端数据项
async fn update_cloud_item<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
    item: &H::Item,
    cloud_data: &CloudSyncData,
) -> Result<(), SyncError> {
    let teams = engine.get_cached_teams();
    let updated_data = {
        let service = engine
            .crypto_service
            .read()
            .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;
        let mut data = handler.encrypt(&service, item, &teams)?;
        data.id = cloud_data.id.clone();
        data.version = cloud_data.version;
        data
    };

    engine
        .cloud_client
        .update_sync_data(&updated_data)
        .await
        .map_err(|e| SyncError::NetworkError(e.to_string()))?;

    Ok(())
}

/// 下载云端数据项并创建本地记录
async fn download_item<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
    cloud_data: &CloudSyncData,
) -> Result<(), SyncError> {
    let service = engine
        .crypto_service
        .read()
        .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

    let mut local_item = handler.decrypt(&service, cloud_data)?;
    local_item.set_local_id(None);
    local_item.set_cloud_id(Some(cloud_data.id.clone()));
    drop(service);

    handler.insert_local(engine, &mut local_item)
}

/// 下载云端数据项并更新已有本地记录
async fn download_and_update_item<H: SyncTypeHandler>(
    engine: &SyncEngine,
    handler: &H,
    cloud_data: &CloudSyncData,
    existing: &H::Item,
) -> Result<(), SyncError> {
    let service = engine
        .crypto_service
        .read()
        .map_err(|_| SyncError::StorageError("同步服务锁获取失败".to_string()))?;

    let mut updated = handler.decrypt(&service, cloud_data)?;
    updated.set_local_id(existing.local_id());
    updated.set_cloud_id(Some(cloud_data.id.clone()));
    drop(service);

    handler.update_local_item(engine, &updated)
}
