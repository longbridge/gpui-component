//! 同步操作队列
//!
//! 支持离线操作排队、失败重试、持久化等功能。
//!
//! ## 设计原则
//!
//! - 操作先入队，批量处理
//! - 支持失败重试（指数退避）
//! - 可持久化到本地（支持重启恢复）

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// 同步操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncOperation {
    /// 上传本地数据到云端
    Upload { local_id: i64 },
    /// 更新云端数据
    UpdateCloud { local_id: i64, cloud_id: String },
    /// 更新本地数据
    UpdateLocal { local_id: i64, cloud_id: String },
    /// 删除云端数据
    DeleteCloud(String),
    /// 删除本地数据
    DeleteLocal(i64),
    /// 下载云端数据到本地
    Download(String),
}

impl SyncOperation {
    /// 获取操作描述
    pub fn description(&self) -> String {
        match self {
            SyncOperation::Upload { local_id } => format!("上传本地数据 {}", local_id),
            SyncOperation::UpdateCloud { local_id, cloud_id } => {
                format!("更新云端数据 {} -> {}", local_id, cloud_id)
            }
            SyncOperation::UpdateLocal { local_id, cloud_id } => {
                format!("更新本地数据 {} <- {}", local_id, cloud_id)
            }
            SyncOperation::DeleteCloud(id) => format!("删除云端数据 {}", id),
            SyncOperation::DeleteLocal(id) => format!("删除本地数据 {}", id),
            SyncOperation::Download(id) => format!("下载云端数据 {}", id),
        }
    }

    /// 获取本地 ID（如果有）
    pub fn local_id(&self) -> Option<i64> {
        match self {
            SyncOperation::Upload { local_id } | SyncOperation::DeleteLocal(local_id) => {
                Some(*local_id)
            }
            SyncOperation::UpdateCloud { local_id, .. }
            | SyncOperation::UpdateLocal { local_id, .. } => Some(*local_id),
            _ => None,
        }
    }

    /// 获取云端 ID（如果有）
    pub fn cloud_id(&self) -> Option<&str> {
        match self {
            SyncOperation::UpdateCloud { cloud_id, .. }
            | SyncOperation::UpdateLocal { cloud_id, .. }
            | SyncOperation::DeleteCloud(cloud_id)
            | SyncOperation::Download(cloud_id) => Some(cloud_id),
            _ => None,
        }
    }
}

/// 队列中的操作项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedOperation {
    /// 操作内容
    pub operation: SyncOperation,
    /// 创建时间
    pub created_at: i64,
    /// 重试次数
    pub retry_count: u32,
    /// 下次重试时间（用于指数退避）
    pub next_retry_at: Option<i64>,
    /// 最后错误信息
    pub last_error: Option<String>,
}

impl QueuedOperation {
    /// 创建新的队列操作
    pub fn new(operation: SyncOperation) -> Self {
        Self {
            operation,
            created_at: Self::current_timestamp(),
            retry_count: 0,
            next_retry_at: None,
            last_error: None,
        }
    }

    /// 标记失败并计算下次重试时间
    pub fn mark_failed(&mut self, error: String) {
        self.retry_count += 1;
        self.last_error = Some(error);
        // 指数退避：2^retry_count 秒，最大 5 分钟
        let delay_secs = (2_u64.pow(self.retry_count)).min(300);
        self.next_retry_at = Some(Self::current_timestamp() + delay_secs as i64);
    }

    /// 检查是否可以重试
    pub fn can_retry(&self, max_retries: u32) -> bool {
        if self.retry_count >= max_retries {
            return false;
        }
        match self.next_retry_at {
            Some(next) => Self::current_timestamp() >= next,
            None => true,
        }
    }

    /// 获取当前时间戳
    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// 操作队列
///
/// 管理待处理的同步操作，支持：
/// - 操作入队和去重
/// - 失败重试（指数退避）
/// - 批量处理
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OperationQueue {
    /// 待处理操作
    pending: VecDeque<QueuedOperation>,
    /// 失败的操作（待重试）
    failed: Vec<QueuedOperation>,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    max_retries: u32,
}

fn default_max_retries() -> u32 {
    3
}

impl OperationQueue {
    /// 创建新的操作队列
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            failed: Vec::new(),
            max_retries: 3,
        }
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 添加操作到队列
    ///
    /// 如果已存在相同操作，则跳过（去重）
    pub fn enqueue(&mut self, operation: SyncOperation) {
        // 去重：检查是否已存在相同操作
        let exists = self.pending.iter().any(|q| q.operation == operation)
            || self.failed.iter().any(|q| q.operation == operation);

        if !exists {
            self.pending.push_back(QueuedOperation::new(operation));
        }
    }

    /// 批量添加操作
    pub fn enqueue_all(&mut self, operations: impl IntoIterator<Item = SyncOperation>) {
        for op in operations {
            self.enqueue(op);
        }
    }

    /// 取出下一个待处理操作
    pub fn dequeue(&mut self) -> Option<QueuedOperation> {
        self.pending.pop_front()
    }

    /// 将操作标记为失败并加入重试队列
    pub fn mark_failed(&mut self, mut operation: QueuedOperation, error: String) {
        operation.mark_failed(error);
        if operation.retry_count < self.max_retries {
            self.failed.push(operation);
        } else {
            tracing::warn!(
                "操作 {:?} 达到最大重试次数 {}，已放弃",
                operation.operation,
                self.max_retries
            );
        }
    }

    /// 将可重试的失败操作移回待处理队列
    pub fn retry_failed(&mut self) {
        let now = QueuedOperation::current_timestamp();
        let mut to_retry = Vec::new();
        let mut still_waiting = Vec::new();

        for op in self.failed.drain(..) {
            if op.can_retry(self.max_retries) {
                match op.next_retry_at {
                    Some(next) if next <= now => to_retry.push(op),
                    _ => still_waiting.push(op),
                }
            } else {
                // 超过重试次数，丢弃
                tracing::warn!("操作 {:?} 已达最大重试次数，放弃", op.operation);
            }
        }

        self.failed = still_waiting;
        for op in to_retry {
            self.pending.push_back(op);
        }
    }

    /// 获取待处理操作数量
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 获取失败待重试操作数量
    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.failed.is_empty()
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.pending.clear();
        self.failed.clear();
    }

    /// 获取所有待处理操作（只读）
    pub fn pending_operations(&self) -> impl Iterator<Item = &QueuedOperation> {
        self.pending.iter()
    }

    /// 获取所有失败操作（只读）
    pub fn failed_operations(&self) -> impl Iterator<Item = &QueuedOperation> {
        self.failed.iter()
    }

    /// 移除指定本地 ID 相关的所有操作
    pub fn remove_by_local_id(&mut self, local_id: i64) {
        self.pending
            .retain(|q| q.operation.local_id() != Some(local_id));
        self.failed
            .retain(|q| q.operation.local_id() != Some(local_id));
    }

    /// 移除指定云端 ID 相关的所有操作
    pub fn remove_by_cloud_id(&mut self, cloud_id: &str) {
        self.pending
            .retain(|q| q.operation.cloud_id() != Some(cloud_id));
        self.failed
            .retain(|q| q.operation.cloud_id() != Some(cloud_id));
    }
}

/// 队列统计信息
#[derive(Debug, Default, Clone)]
pub struct QueueStats {
    /// 待处理数量
    pub pending: usize,
    /// 失败待重试数量
    pub failed: usize,
    /// 上传操作数
    pub uploads: usize,
    /// 更新操作数
    pub updates: usize,
    /// 删除操作数
    pub deletes: usize,
    /// 下载操作数
    pub downloads: usize,
}

impl OperationQueue {
    /// 获取队列统计信息
    pub fn stats(&self) -> QueueStats {
        let mut stats = QueueStats {
            pending: self.pending.len(),
            failed: self.failed.len(),
            ..Default::default()
        };

        for q in self.pending.iter().chain(self.failed.iter()) {
            match &q.operation {
                SyncOperation::Upload { .. } => stats.uploads += 1,
                SyncOperation::UpdateCloud { .. } | SyncOperation::UpdateLocal { .. } => {
                    stats.updates += 1
                }
                SyncOperation::DeleteCloud(_) | SyncOperation::DeleteLocal(_) => stats.deletes += 1,
                SyncOperation::Download(_) => stats.downloads += 1,
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_enqueue_dequeue() {
        let mut queue = OperationQueue::new();
        queue.enqueue(SyncOperation::Upload { local_id: 1 });
        queue.enqueue(SyncOperation::Upload { local_id: 2 });

        assert_eq!(queue.pending_count(), 2);

        let op1 = queue.dequeue().unwrap();
        assert_eq!(op1.operation, SyncOperation::Upload { local_id: 1 });

        let op2 = queue.dequeue().unwrap();
        assert_eq!(op2.operation, SyncOperation::Upload { local_id: 2 });

        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_queue_dedup() {
        let mut queue = OperationQueue::new();
        queue.enqueue(SyncOperation::Upload { local_id: 1 });
        queue.enqueue(SyncOperation::Upload { local_id: 1 }); // 重复，应被忽略

        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn test_queue_failed_retry() {
        let mut queue = OperationQueue::new().with_max_retries(3);
        queue.enqueue(SyncOperation::Upload { local_id: 1 });

        let op = queue.dequeue().unwrap();
        queue.mark_failed(op.clone(), "测试错误".to_string());

        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.failed_count(), 1);
    }

    #[test]
    fn test_queue_stats() {
        let mut queue = OperationQueue::new();
        queue.enqueue(SyncOperation::Upload { local_id: 1 });
        queue.enqueue(SyncOperation::UpdateCloud {
            local_id: 2,
            cloud_id: "cloud-2".to_string(),
        });
        queue.enqueue(SyncOperation::Download("cloud-3".to_string()));
        queue.enqueue(SyncOperation::DeleteCloud("cloud-4".to_string()));

        let stats = queue.stats();
        assert_eq!(stats.pending, 4);
        assert_eq!(stats.uploads, 1);
        assert_eq!(stats.updates, 1);
        assert_eq!(stats.downloads, 1);
        assert_eq!(stats.deletes, 1);
    }
}
