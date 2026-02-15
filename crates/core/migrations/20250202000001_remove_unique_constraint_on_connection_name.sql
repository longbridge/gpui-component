-- 去掉 connections 表 name 字段的 UNIQUE 约束
-- SQLite 不支持 ALTER TABLE DROP CONSTRAINT，需要重建表

-- 1. 创建不带 UNIQUE 约束的新表
CREATE TABLE IF NOT EXISTS connections_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    connection_type TEXT NOT NULL,
    params TEXT NOT NULL,
    workspace_id INTEGER,
    selected_databases TEXT,
    remark TEXT,
    sync_enabled INTEGER NOT NULL DEFAULT 1,
    cloud_id TEXT,
    last_synced_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

-- 2. 复制数据
INSERT INTO connections_new (id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at)
SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at
FROM connections;

-- 3. 删除旧表
DROP TABLE connections;

-- 4. 重命名新表
ALTER TABLE connections_new RENAME TO connections;

-- 5. 重建索引（普通索引，非唯一）
CREATE INDEX IF NOT EXISTS idx_connections_name ON connections(name);
CREATE INDEX IF NOT EXISTS idx_connections_workspace ON connections(workspace_id);
CREATE INDEX IF NOT EXISTS idx_connections_cloud_id ON connections(cloud_id);
