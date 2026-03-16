-- ============================================================================
-- 1. teams：团队定义
-- ============================================================================
CREATE TABLE teams (
                       id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                       name        TEXT        NOT NULL,
                       owner_id    UUID        NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
                       description TEXT,
    -- 团队密钥验证数据（由 owner 设置，成员验证用）
                       key_verification TEXT,
    -- 团队密钥版本号
                       key_version INTEGER     NOT NULL DEFAULT 0,
                       created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_teams_owner ON teams(owner_id);


-- ============================================================================
-- 2. team_members：团队成员
-- ============================================================================
CREATE TABLE team_members (
                              id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                              team_id   UUID        NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                              user_id   UUID        NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
                              role      TEXT        NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'member')),
                              joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                              UNIQUE(team_id, user_id)
);

CREATE INDEX idx_team_members_team ON team_members(team_id);
CREATE INDEX idx_team_members_user ON team_members(user_id);


-- ============================================================================
-- 2.1 辅助函数：获取当前用户所在的所有 team_id
-- 必须在 team_members 表创建之后、RLS 策略之前定义
-- SECURITY DEFINER 让函数以定义者权限执行，绕过 RLS，打破策略递归
-- ============================================================================
CREATE OR REPLACE FUNCTION get_my_team_ids()
    RETURNS SETOF UUID
    LANGUAGE sql SECURITY DEFINER STABLE AS $$
SELECT team_id FROM team_members WHERE user_id = auth.uid();
$$;


-- ============================================================================
-- 3. sync_data：统一加密数据表（核心）
-- ============================================================================
CREATE TABLE sync_data (
    -- 主键 UUID
                           id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 记录创建者
                           owner_id       UUID        NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,

    -- 团队归属：NULL = 个人数据，非 NULL = 团队共享数据
                           team_id        UUID        REFERENCES teams(id) ON DELETE CASCADE,

    -- 数据类型标识（可扩展）
    -- 当前: 'connection', 'workspace'
    -- 未来: 'setting', 'snippet', 'query_template', ...
                           data_type      TEXT        NOT NULL,

    -- 加密后的完整数据 blob
    -- 格式: base64(nonce + AES-256-GCM ciphertext)
    -- 个人数据用 personal_key 加密，团队数据用 team_key 加密
                           encrypted_data TEXT        NOT NULL,

    -- 加密密钥版本
    -- 个人: 对应 user_configs.key_version (scope='personal')
    -- 团队: 对应 teams.key_version
                           key_version    INTEGER     NOT NULL DEFAULT 1,

    -- 明文数据的 SHA-256 校验和（加密前计算，用于冲突检测）
                           checksum       TEXT        NOT NULL DEFAULT '',

    -- 数据版本号（每次更新自动递增，用于乐观并发控制）
                           version        INTEGER     NOT NULL DEFAULT 1,

    -- 时间戳
                           created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
                           updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- 软删除
                           deleted_at     TIMESTAMPTZ
);

-- 索引
CREATE INDEX idx_sync_data_owner      ON sync_data(owner_id);
CREATE INDEX idx_sync_data_team       ON sync_data(team_id)              WHERE team_id IS NOT NULL;
CREATE INDEX idx_sync_data_type       ON sync_data(data_type);
CREATE INDEX idx_sync_data_owner_type ON sync_data(owner_id, data_type)  WHERE team_id IS NULL;
CREATE INDEX idx_sync_data_team_type  ON sync_data(team_id, data_type)   WHERE team_id IS NOT NULL;
CREATE INDEX idx_sync_data_updated    ON sync_data(updated_at DESC);
CREATE INDEX idx_sync_data_not_deleted ON sync_data(id)                  WHERE deleted_at IS NULL;


-- ============================================================================
-- 4. user_configs：密钥验证（扩展 scope 支持团队）
-- ============================================================================
CREATE TABLE user_configs (
                              id               BIGSERIAL   PRIMARY KEY,
                              user_id          UUID        NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    -- 配置范围：'personal' | 'team:{team_id}'
                              scope            TEXT        NOT NULL DEFAULT 'personal',
                              key_verification TEXT        NOT NULL,
                              key_version      INTEGER     NOT NULL DEFAULT 1,
                              updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
                              UNIQUE(user_id, scope)
);

CREATE INDEX idx_user_configs_user       ON user_configs(user_id);
CREATE INDEX idx_user_configs_user_scope ON user_configs(user_id, scope);


-- ============================================================================
-- 5. Triggers：自动更新 updated_at 和 version
-- ============================================================================

-- sync_data: 更新时自动递增 version 和刷新 updated_at
CREATE OR REPLACE FUNCTION fn_sync_data_before_update()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    NEW.version    = OLD.version + 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_sync_data_before_update
    BEFORE UPDATE ON sync_data
    FOR EACH ROW
EXECUTE FUNCTION fn_sync_data_before_update();

-- teams: 更新时刷新 updated_at
CREATE OR REPLACE FUNCTION fn_teams_before_update()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_teams_before_update
    BEFORE UPDATE ON teams
    FOR EACH ROW
EXECUTE FUNCTION fn_teams_before_update();

-- user_configs: 更新时刷新 updated_at
CREATE OR REPLACE FUNCTION fn_user_configs_before_update()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_user_configs_before_update
    BEFORE UPDATE ON user_configs
    FOR EACH ROW
EXECUTE FUNCTION fn_user_configs_before_update();


-- ============================================================================
-- 6. RLS 策略
-- ============================================================================

ALTER TABLE sync_data    ENABLE ROW LEVEL SECURITY;
ALTER TABLE teams        DISABLE ROW LEVEL SECURITY;
ALTER TABLE team_members ENABLE ROW LEVEL SECURITY;  -- 现在可以安全启用
ALTER TABLE user_configs ENABLE ROW LEVEL SECURITY;

-- --------------------------------------------------------------------------
-- sync_data
-- 使用 get_my_team_ids() 代替直接子查询，避免递归
-- --------------------------------------------------------------------------

-- SELECT: 个人数据 + 所在团队的数据
CREATE POLICY sync_data_select ON sync_data FOR SELECT USING (
    (team_id IS NULL AND owner_id = auth.uid())
        OR
    (team_id IN (SELECT get_my_team_ids()))
    );

-- INSERT: 个人数据必须是自己的；团队数据必须是所在团队
CREATE POLICY sync_data_insert ON sync_data FOR INSERT WITH CHECK (
    owner_id = auth.uid()
        AND (
        team_id IS NULL
            OR team_id IN (SELECT get_my_team_ids())
        )
    );

-- UPDATE: 个人数据自己改；团队数据：自己创建的可改，团队 owner 可改所有
CREATE POLICY sync_data_update ON sync_data FOR UPDATE USING (
    (team_id IS NULL  AND owner_id = auth.uid())
        OR
    (team_id IS NOT NULL AND owner_id = auth.uid())
        OR
    (team_id IS NOT NULL AND team_id IN (
        SELECT id FROM teams WHERE owner_id = auth.uid()
    ))
    );

-- DELETE: 同 UPDATE 权限
CREATE POLICY sync_data_delete ON sync_data FOR DELETE USING (
    (team_id IS NULL  AND owner_id = auth.uid())
        OR
    (team_id IS NOT NULL AND owner_id = auth.uid())
        OR
    (team_id IS NOT NULL AND team_id IN (
        SELECT id FROM teams WHERE owner_id = auth.uid()
    ))
    );

-- --------------------------------------------------------------------------
-- teams
-- --------------------------------------------------------------------------

-- SELECT: 团队成员可查看所在团队
CREATE POLICY teams_select ON teams FOR SELECT USING (
    id IN (SELECT get_my_team_ids())
    );

-- INSERT: 已认证用户可创建团队（owner 必须是自己）
CREATE POLICY teams_insert ON teams FOR INSERT WITH CHECK (
    owner_id = auth.uid()
    );

-- UPDATE: 仅 owner
CREATE POLICY teams_update ON teams FOR UPDATE USING (
    owner_id = auth.uid()
    );

-- DELETE: 仅 owner
CREATE POLICY teams_delete ON teams FOR DELETE USING (
    owner_id = auth.uid()
    );

-- --------------------------------------------------------------------------
-- team_members
-- 使用 get_my_team_ids() 打破自引用递归，现在可以真正启用 RLS
-- --------------------------------------------------------------------------

-- SELECT: 同团队成员可互相查看
CREATE POLICY team_members_select ON team_members FOR SELECT USING (
    team_id IN (SELECT get_my_team_ids())
    );

-- INSERT: 仅团队 owner 可添加成员
CREATE POLICY team_members_insert ON team_members FOR INSERT WITH CHECK (
    team_id IN (SELECT id FROM teams WHERE owner_id = auth.uid())
    );

-- UPDATE: 仅团队 owner 可修改角色
CREATE POLICY team_members_update ON team_members FOR UPDATE USING (
    team_id IN (SELECT id FROM teams WHERE owner_id = auth.uid())
    );

-- DELETE: 团队 owner 可移除任意成员，成员可自己退出
CREATE POLICY team_members_delete ON team_members FOR DELETE USING (
    user_id = auth.uid()
        OR team_id IN (SELECT id FROM teams WHERE owner_id = auth.uid())
    );

-- --------------------------------------------------------------------------
-- user_configs
-- --------------------------------------------------------------------------

CREATE POLICY user_configs_select ON user_configs FOR SELECT USING (user_id = auth.uid());
CREATE POLICY user_configs_insert ON user_configs FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY user_configs_update ON user_configs FOR UPDATE USING (user_id = auth.uid());
CREATE POLICY user_configs_delete ON user_configs FOR DELETE USING (user_id = auth.uid());


-- ============================================================================
-- 7. 辅助函数：创建团队时自动插入 owner 为成员
-- ============================================================================
CREATE OR REPLACE FUNCTION fn_auto_add_team_owner()
    RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO team_members (team_id, user_id, role)
    VALUES (NEW.id, NEW.owner_id, 'owner');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER trg_auto_add_team_owner
    AFTER INSERT ON teams
    FOR EACH ROW
EXECUTE FUNCTION fn_auto_add_team_owner();


-- ============================================================================
-- 8. 辅助函数：通过邮箱添加团队成员
-- ============================================================================
-- 由于 auth.users 受 RLS 保护，普通用户无法直接查询
-- 使用 SECURITY DEFINER 函数安全地根据 email 查找用户并添加为团队成员
CREATE OR REPLACE FUNCTION add_team_member_by_email(
    p_team_id UUID,
    p_email   TEXT
) RETURNS json AS $$
DECLARE
    v_user_id   UUID;
    v_member_id UUID;
    v_joined_at TIMESTAMPTZ;
BEGIN
    -- 仅团队 owner 可调用
    IF NOT EXISTS (
        SELECT 1 FROM teams WHERE id = p_team_id AND owner_id = auth.uid()
    ) THEN
        RAISE EXCEPTION 'Only team owner can add members';
    END IF;

    -- 通过 email 查找用户
    SELECT id INTO v_user_id FROM auth.users WHERE email = p_email;
    IF v_user_id IS NULL THEN
        RAISE EXCEPTION 'User with email % not found', p_email;
    END IF;

    -- 不能添加自己（owner 已在创建时自动加入）
    IF v_user_id = auth.uid() THEN
        RAISE EXCEPTION 'Cannot add yourself as a member';
    END IF;

    -- 检查是否已是成员
    IF EXISTS (
        SELECT 1 FROM team_members WHERE team_id = p_team_id AND user_id = v_user_id
    ) THEN
        RAISE EXCEPTION 'User is already a member of this team';
    END IF;

    -- 插入成员
    INSERT INTO team_members (team_id, user_id, role)
    VALUES (p_team_id, v_user_id, 'member')
    RETURNING id, joined_at INTO v_member_id, v_joined_at;

    RETURN json_build_object(
            'id',        v_member_id,
            'team_id',   p_team_id,
            'user_id',   v_user_id,
            'role',      'member',
            'joined_at', v_joined_at
           );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;


-- ============================================================================
-- 9. 保留不变的表
-- ============================================================================
-- user_subscriptions - 保持不变
-- model_list         - 保持不变