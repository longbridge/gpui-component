-- connections 表新增创建者字段
ALTER TABLE connections ADD COLUMN owner_id TEXT;

-- team_key_cache 表新增角色字段（缓存当前用户在团队中的角色）
ALTER TABLE team_key_cache ADD COLUMN role TEXT;
