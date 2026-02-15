-- 待删除云端记录表
-- 用于记录离线删除时未能同步删除的云端连接，下次同步时处理
CREATE TABLE IF NOT EXISTS pending_cloud_deletions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cloud_id TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL DEFAULT 'connection',
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pending_cloud_deletions_entity_type ON pending_cloud_deletions(entity_type);
