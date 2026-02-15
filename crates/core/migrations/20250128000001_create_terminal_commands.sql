-- 终端命令历史表
CREATE TABLE IF NOT EXISTS terminal_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER,
    connection_id INTEGER,
    command TEXT NOT NULL,
    working_directory TEXT,
    executed_at INTEGER NOT NULL,
    exit_code INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);

-- 索引：按连接ID查询
CREATE INDEX IF NOT EXISTS idx_terminal_commands_connection ON terminal_commands(connection_id);

-- 索引：按执行时间排序
CREATE INDEX IF NOT EXISTS idx_terminal_commands_executed ON terminal_commands(executed_at DESC);

-- 索引：按命令内容搜索
CREATE INDEX IF NOT EXISTS idx_terminal_commands_command ON terminal_commands(command);
