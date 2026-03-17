use anyhow::Result;
use gpui::{App, SharedString};
use rusqlite::params;

use crate::storage::connection::SqliteConnection;
use crate::storage::manager::{GlobalStorageState, now};
use crate::storage::models::has_decrypt_failure_in_sensitive_fields;
use crate::storage::quick_command::QuickCommandRepository;
use crate::storage::row_mapping::FromSqliteRow;
use crate::storage::traits::Repository;
use crate::storage::{ConnectionType, StoredConnection, Workspace};

struct ConnectionRow {
    id: i64,
    name: String,
    connection_type: String,
    params: String,
    workspace_id: Option<i64>,
    selected_databases: Option<String>,
    remark: Option<String>,
    sync_enabled: bool,
    cloud_id: Option<String>,
    last_synced_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
    team_id: Option<String>,
    owner_id: Option<String>,
}

impl FromSqliteRow for ConnectionRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(ConnectionRow {
            id: row.get("id")?,
            name: row.get("name")?,
            connection_type: row.get("connection_type")?,
            params: row.get("params")?,
            workspace_id: row.get("workspace_id")?,
            selected_databases: row.get("selected_databases")?,
            remark: row.get("remark")?,
            sync_enabled: row
                .get::<_, i64>("sync_enabled")
                .map(|v| v != 0)
                .unwrap_or(true),
            cloud_id: row.get("cloud_id")?,
            last_synced_at: row.get("last_synced_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            team_id: row.get("team_id").unwrap_or(None),
            owner_id: row.get("owner_id").unwrap_or(None),
        })
    }
}

impl From<ConnectionRow> for StoredConnection {
    fn from(row: ConnectionRow) -> Self {
        let mut conn = StoredConnection {
            id: Some(row.id),
            name: row.name,
            connection_type: ConnectionType::from_str(&row.connection_type),
            params: row.params,
            workspace_id: row.workspace_id,
            selected_databases: row.selected_databases,
            remark: row.remark,
            sync_enabled: row.sync_enabled,
            cloud_id: row.cloud_id,
            last_synced_at: row.last_synced_at,
            created_at: Some(row.created_at),
            updated_at: Some(row.updated_at),
            team_id: row.team_id,
            owner_id: row.owner_id,
        };
        // 从数据库读取后自动解密敏感字段
        conn.params = conn.decrypt_params();
        conn
    }
}

struct WorkspaceRow {
    id: i64,
    name: String,
    color: Option<String>,
    icon: Option<String>,
    created_at: i64,
    updated_at: i64,
    cloud_id: Option<String>,
}

impl FromSqliteRow for WorkspaceRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(WorkspaceRow {
            id: row.get("id")?,
            name: row.get("name")?,
            color: row.get("color")?,
            icon: row.get("icon")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            cloud_id: row.get("cloud_id")?,
        })
    }
}

impl From<WorkspaceRow> for Workspace {
    fn from(row: WorkspaceRow) -> Self {
        Workspace {
            id: Some(row.id),
            name: row.name,
            color: row.color,
            icon: row.icon,
            created_at: Some(row.created_at),
            updated_at: Some(row.updated_at),
            cloud_id: row.cloud_id,
        }
    }
}

#[derive(Clone)]
pub struct ConnectionRepository {
    conn: SqliteConnection,
}

impl ConnectionRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }
}

impl Repository for ConnectionRepository {
    type Entity = StoredConnection;

    fn entity_type(&self) -> SharedString {
        SharedString::from("Connection")
    }

    fn insert(&self, item: &mut Self::Entity) -> Result<i64> {
        let name = item.name.clone();
        let connection_type = item.connection_type.to_string();
        let params_str = item.encrypt_params();
        let workspace_id = item.workspace_id;
        let selected_databases = item.selected_databases.clone();
        let remark = item.remark.clone();
        let sync_enabled = if item.sync_enabled { 1i64 } else { 0i64 };
        let cloud_id = item.cloud_id.clone();
        let last_synced_at = item.last_synced_at;
        let team_id = item.team_id.clone();
        let owner_id = item.owner_id.clone();
        let ts = now();

        let id = self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT INTO connections (name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, team_id, owner_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![name, connection_type, params_str, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, team_id, owner_id, ts, ts],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        item.id = Some(id);
        item.created_at = Some(ts);
        item.updated_at = Some(ts);

        Ok(id)
    }

    fn update(&self, item: &Self::Entity) -> Result<()> {
        let id = item
            .id
            .ok_or_else(|| anyhow::anyhow!("Cannot update without ID"))?;
        let name = item.name.clone();
        let connection_type = item.connection_type.to_string();
        let params_str = item.encrypt_params();
        let workspace_id = item.workspace_id;
        let selected_databases = item.selected_databases.clone();
        let remark = item.remark.clone();
        let sync_enabled = if item.sync_enabled { 1i64 } else { 0i64 };
        let cloud_id = item.cloud_id.clone();
        let last_synced_at = item.last_synced_at;
        let team_id = item.team_id.clone();
        let owner_id = item.owner_id.clone();
        let ts = now();

        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE connections SET name = ?1, connection_type = ?2, params = ?3, workspace_id = ?4, selected_databases = ?5, remark = ?6, sync_enabled = ?7, cloud_id = ?8, last_synced_at = ?9, team_id = ?10, owner_id = ?11, updated_at = ?12 WHERE id = ?13",
                params![name, connection_type, params_str, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, team_id, owner_id, ts, id],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute("DELETE FROM connections WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn get(&self, id: i64) -> Result<Option<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id FROM connections WHERE id = ?1",
            )?;
            let mut rows = stmt.query(params![id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(ConnectionRow::from_row(row)?.into()))
            } else {
                Ok(None)
            }
        })
    }

    fn list(&self) -> Result<Vec<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id FROM connections ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| ConnectionRow::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }

    fn count(&self) -> Result<i64> {
        self.conn.with_connection(|conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM connections", [], |row| row.get(0))?;
            Ok(count)
        })
    }

    fn exists(&self, id: i64) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let exists: i64 = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM connections WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
            Ok(exists == 1)
        })
    }
}

impl ConnectionRepository {
    pub fn list_by_workspace(&self, workspace_id: Option<i64>) -> Result<Vec<StoredConnection>> {
        self.conn.with_connection(|conn| {
            let sql = if workspace_id.is_some() {
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id FROM connections WHERE workspace_id = ?1 ORDER BY updated_at DESC"
            } else {
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id FROM connections WHERE workspace_id IS NULL ORDER BY updated_at DESC"
            };
            let mut stmt = conn.prepare(sql)?;

            let mut results = Vec::new();
            if let Some(wid) = workspace_id {
                let rows = stmt.query_map(params![wid], |row| ConnectionRow::from_row(row))?;
                for row in rows {
                    results.push(row?.into());
                }
            } else {
                let rows = stmt.query_map([], |row| ConnectionRow::from_row(row))?;
                for row in rows {
                    results.push(row?.into());
                }
            }
            Ok(results)
        })
    }

    /// 更新连接的同步状态
    ///
    /// 同步成功后调用，设置 cloud_id 和 last_synced_at
    pub fn update_sync_status(
        &self,
        id: i64,
        cloud_id: Option<String>,
        last_synced_at: Option<i64>,
    ) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE connections SET cloud_id = ?1, last_synced_at = ?2 WHERE id = ?3",
                params![cloud_id, last_synced_at, id],
            )?;
            Ok(())
        })
    }

    /// 查询需要同步的连接（sync_enabled=true 且 cloud_id 为空或 updated_at > last_synced_at）
    pub fn list_pending_sync(&self) -> Result<Vec<StoredConnection>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id
                 FROM connections
                 WHERE sync_enabled = 1 AND (cloud_id IS NULL OR updated_at > COALESCE(last_synced_at, 0))
                 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| ConnectionRow::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }

    /// 根据 cloud_id 查询连接
    pub fn get_by_cloud_id(&self, cloud_id: &str) -> Result<Option<StoredConnection>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id
                 FROM connections WHERE cloud_id = ?1",
            )?;
            let mut rows = stmt.query(params![cloud_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(ConnectionRow::from_row(row)?.into()))
            } else {
                Ok(None)
            }
        })
    }

    /// 检测启用同步的连接中是否存在解密失败的数据。
    ///
    /// 返回值为 (id, name) 列表，便于上层记录日志与阻断同步。
    pub fn list_sync_decrypt_failures(&self) -> Result<Vec<(i64, String)>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, params FROM connections WHERE sync_enabled = 1 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                let id: i64 = row.get("id")?;
                let name: String = row.get("name")?;
                let params: String = row.get("params")?;
                Ok((id, name, params))
            })?;

            let mut failures = Vec::new();
            for row in rows {
                let (id, name, params) = row?;
                if has_decrypt_failure_in_sensitive_fields(&params) {
                    failures.push((id, name));
                }
            }
            Ok(failures)
        })
    }

    /// 按团队 ID 查询连接
    pub fn list_by_team(&self, team_id: &str) -> Result<Vec<StoredConnection>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id FROM connections WHERE team_id = ?1 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map(params![team_id], |row| ConnectionRow::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }

    /// 查询个人连接（team_id 为 NULL）
    pub fn list_personal(&self) -> Result<Vec<StoredConnection>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, connection_type, params, workspace_id, selected_databases, remark, sync_enabled, cloud_id, last_synced_at, created_at, updated_at, team_id, owner_id FROM connections WHERE team_id IS NULL ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| ConnectionRow::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }
}

#[derive(Clone)]
pub struct WorkspaceRepository {
    conn: SqliteConnection,
}

impl WorkspaceRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }

    pub fn update_from_cloud(&self, item: &Workspace) -> Result<()> {
        let id = item
            .id
            .ok_or_else(|| anyhow::anyhow!("Cannot update without ID"))?;
        let name = item.name.clone();
        let color = item.color.clone();
        let icon = item.icon.clone();
        let cloud_id = item.cloud_id.clone();
        let updated_at = item.updated_at.unwrap_or_else(now);

        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE workspaces SET name = ?1, color = ?2, icon = ?3, cloud_id = ?4, updated_at = ?5 WHERE id = ?6",
                params![name, color, icon, cloud_id, updated_at, id],
            )?;
            Ok(())
        })
    }

    /// 更新工作空间的云端同步状态
    pub fn update_cloud_id(&self, local_id: i64, cloud_id: Option<String>) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE workspaces SET cloud_id = ?1 WHERE id = ?2",
                params![cloud_id, local_id],
            )?;
            Ok(())
        })
    }
}

impl Repository for WorkspaceRepository {
    type Entity = Workspace;

    fn entity_type(&self) -> SharedString {
        SharedString::from("Workspace")
    }

    fn insert(&self, item: &mut Self::Entity) -> Result<i64> {
        let name = item.name.clone();
        let color = item.color.clone();
        let icon = item.icon.clone();
        let cloud_id = item.cloud_id.clone();
        let ts = now();

        let id = self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT INTO workspaces (name, color, icon, cloud_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![name, color, icon, cloud_id, ts, ts],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        item.id = Some(id);
        item.created_at = Some(ts);
        item.updated_at = Some(ts);

        Ok(id)
    }

    fn update(&self, item: &Self::Entity) -> Result<()> {
        let id = item
            .id
            .ok_or_else(|| anyhow::anyhow!("Cannot update without ID"))?;
        let name = item.name.clone();
        let color = item.color.clone();
        let icon = item.icon.clone();
        let cloud_id = item.cloud_id.clone();
        let ts = now();

        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE workspaces SET name = ?1, color = ?2, icon = ?3, cloud_id = ?4, updated_at = ?5 WHERE id = ?6",
                params![name, color, icon, cloud_id, ts, id],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE connections SET workspace_id = NULL WHERE workspace_id = ?1",
                params![id],
            )?;
            conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn get(&self, id: i64) -> Result<Option<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, color, icon, created_at, updated_at, cloud_id FROM workspaces WHERE id = ?1")?;
            let mut rows = stmt.query(params![id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(WorkspaceRow::from_row(row)?.into()))
            } else {
                Ok(None)
            }
        })
    }

    fn list(&self) -> Result<Vec<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, color, icon, created_at, updated_at, cloud_id FROM workspaces ORDER BY updated_at DESC")?;
            let rows = stmt.query_map([], |row| WorkspaceRow::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }

    fn count(&self) -> Result<i64> {
        self.conn.with_connection(|conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM workspaces", [], |row| row.get(0))?;
            Ok(count)
        })
    }

    fn exists(&self, id: i64) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let exists: i64 = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
            Ok(exists == 1)
        })
    }
}

/// 待删除云端记录
#[derive(Debug, Clone)]
pub struct PendingCloudDeletion {
    pub id: Option<i64>,
    pub cloud_id: String,
    pub entity_type: String,
    pub created_at: i64,
}

/// 待删除云端记录仓库
#[derive(Clone)]
pub struct PendingCloudDeletionRepository {
    conn: SqliteConnection,
}

impl PendingCloudDeletionRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }

    /// 添加待删除记录
    pub fn add(&self, cloud_id: &str, entity_type: &str) -> Result<()> {
        let ts = now();
        self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO pending_cloud_deletions (cloud_id, entity_type, created_at) VALUES (?1, ?2, ?3)",
                params![cloud_id, entity_type, ts],
            )?;
            Ok(())
        })
    }

    /// 获取所有待删除的连接
    pub fn list_connections(&self) -> Result<Vec<PendingCloudDeletion>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, cloud_id, entity_type, created_at FROM pending_cloud_deletions WHERE entity_type = 'connection'"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(PendingCloudDeletion {
                    id: row.get(0)?,
                    cloud_id: row.get(1)?,
                    entity_type: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    /// 获取所有待删除的工作空间
    pub fn list_workspaces(&self) -> Result<Vec<PendingCloudDeletion>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, cloud_id, entity_type, created_at FROM pending_cloud_deletions WHERE entity_type = 'workspace'"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(PendingCloudDeletion {
                    id: row.get(0)?,
                    cloud_id: row.get(1)?,
                    entity_type: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    /// 删除记录（同步成功后调用）
    pub fn remove(&self, cloud_id: &str) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute(
                "DELETE FROM pending_cloud_deletions WHERE cloud_id = ?1",
                params![cloud_id],
            )?;
            Ok(())
        })
    }

    /// 检查 cloud_id 是否在待删除列表中
    pub fn is_pending(&self, cloud_id: &str) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM pending_cloud_deletions WHERE cloud_id = ?1",
                params![cloud_id],
                |row| row.get(0),
            )?;
            Ok(count > 0)
        })
    }
}

/// 团队密钥缓存（本地存储，用 personal_key 加密 team_key）
#[derive(Debug, Clone)]
pub struct TeamKeyCache {
    pub team_id: String,
    pub team_name: String,
    pub key_version: u32,
    /// 用 personal_key 加密后的 team_key
    pub encrypted_team_key: Option<String>,
    pub last_verified_at: Option<i64>,
    pub updated_at: i64,
    /// 当前用户在该团队中的角色（owner / member）
    pub role: Option<String>,
}

/// 团队密钥缓存仓库
#[derive(Clone)]
pub struct TeamKeyCacheRepository {
    conn: SqliteConnection,
}

impl TeamKeyCacheRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }

    /// 获取团队密钥缓存
    pub fn get(&self, team_id: &str) -> Result<Option<TeamKeyCache>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT team_id, team_name, key_version, encrypted_team_key, last_verified_at, updated_at, role FROM team_key_cache WHERE team_id = ?1",
            )?;
            let mut rows = stmt.query(params![team_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(TeamKeyCache {
                    team_id: row.get(0)?,
                    team_name: row.get(1)?,
                    key_version: row.get::<_, i64>(2)? as u32,
                    encrypted_team_key: row.get(3)?,
                    last_verified_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    role: row.get(6).unwrap_or(None),
                }))
            } else {
                Ok(None)
            }
        })
    }

    /// 保存或更新团队密钥缓存
    pub fn upsert(&self, cache: &TeamKeyCache) -> Result<()> {
        let ts = now();
        self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT INTO team_key_cache (team_id, team_name, key_version, encrypted_team_key, last_verified_at, updated_at, role)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(team_id) DO UPDATE SET
                 team_name = excluded.team_name,
                 key_version = excluded.key_version,
                 encrypted_team_key = excluded.encrypted_team_key,
                 last_verified_at = excluded.last_verified_at,
                 updated_at = excluded.updated_at,
                 role = excluded.role",
                params![cache.team_id, cache.team_name, cache.key_version as i64, cache.encrypted_team_key, cache.last_verified_at, ts, cache.role],
            )?;
            Ok(())
        })
    }

    /// 获取所有缓存的团队密钥
    pub fn list(&self) -> Result<Vec<TeamKeyCache>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT team_id, team_name, key_version, encrypted_team_key, last_verified_at, updated_at, role FROM team_key_cache ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(TeamKeyCache {
                    team_id: row.get(0)?,
                    team_name: row.get(1)?,
                    key_version: row.get::<_, i64>(2)? as u32,
                    encrypted_team_key: row.get(3)?,
                    last_verified_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    role: row.get(6).unwrap_or(None),
                })
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    /// 删除团队密钥缓存
    pub fn delete(&self, team_id: &str) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute(
                "DELETE FROM team_key_cache WHERE team_id = ?1",
                params![team_id],
            )?;
            Ok(())
        })
    }
}

pub fn init(cx: &mut App) {
    let storage_state = cx.global::<GlobalStorageState>();
    let storage = storage_state.storage.clone();

    let conn = storage.connection();
    let conn_repo = ConnectionRepository::new(conn.clone());
    let workspace_repo = WorkspaceRepository::new(conn.clone());
    let quick_cmd_repo = QuickCommandRepository::new(conn.clone());
    let pending_deletion_repo = PendingCloudDeletionRepository::new(conn.clone());
    let team_key_cache_repo = TeamKeyCacheRepository::new(conn.clone());

    storage.register(workspace_repo);
    storage.register(conn_repo);
    storage.register(quick_cmd_repo);
    storage.register(pending_deletion_repo);
    storage.register(team_key_cache_repo);
}
