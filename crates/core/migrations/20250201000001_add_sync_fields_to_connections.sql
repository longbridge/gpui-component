-- 为 connections 表添加云同步相关字段
ALTER TABLE connections ADD COLUMN sync_enabled INTEGER NOT NULL DEFAULT 1;
ALTER TABLE connections ADD COLUMN cloud_id TEXT;
ALTER TABLE connections ADD COLUMN last_synced_at INTEGER;

-- 为 cloud_id 创建索引以便快速查找
CREATE INDEX IF NOT EXISTS idx_connections_cloud_id ON connections(cloud_id);
