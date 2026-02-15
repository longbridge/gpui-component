-- 快捷命令表
CREATE TABLE IF NOT EXISTS quick_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    command TEXT NOT NULL,
    description TEXT,
    pinned INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    connection_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);

-- 索引：按连接ID查询
CREATE INDEX IF NOT EXISTS idx_quick_commands_connection ON quick_commands(connection_id);

-- 索引：按置顶和排序顺序
CREATE INDEX IF NOT EXISTS idx_quick_commands_order ON quick_commands(pinned DESC, sort_order ASC);
