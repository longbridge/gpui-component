-- 团队同步支持：新增 team_id 字段和 team_key_cache 表

-- connections 表新增团队归属字段
ALTER TABLE connections ADD COLUMN team_id TEXT;
CREATE INDEX IF NOT EXISTS idx_connections_team ON connections(team_id);

-- 团队密钥本地缓存表
CREATE TABLE IF NOT EXISTS team_key_cache (
    team_id TEXT PRIMARY KEY,
    team_name TEXT NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 0,
    encrypted_team_key TEXT,
    last_verified_at INTEGER,
    updated_at INTEGER NOT NULL
);
