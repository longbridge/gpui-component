//! 快捷命令存储模块

use anyhow::Result;
use gpui::SharedString;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::connection::SqliteConnection;
use crate::storage::manager::now;
use crate::storage::row_mapping::FromSqliteRow;
use crate::storage::traits::{Entity, Repository};

/// 快捷命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickCommand {
    /// 记录 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// 命令名称（用于显示）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 命令内容
    pub command: String,
    /// 命令描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否置顶
    pub pinned: bool,
    /// 排序顺序（越小越靠前）
    pub sort_order: i32,
    /// 连接 ID（可选，null 表示全局命令）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<i64>,
    /// 创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    /// 更新时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

impl Entity for QuickCommand {
    fn id(&self) -> Option<i64> {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at.unwrap_or(0)
    }

    fn updated_at(&self) -> i64 {
        self.updated_at.unwrap_or(0)
    }
}

impl QuickCommand {
    /// 创建新的快捷命令
    pub fn new(command: String) -> Self {
        let ts = now();
        Self {
            id: None,
            name: None,
            command,
            description: None,
            pinned: false,
            sort_order: 0,
            connection_id: None,
            created_at: Some(ts),
            updated_at: Some(ts),
        }
    }

    /// 创建带名称的快捷命令
    pub fn with_name(command: String, name: String) -> Self {
        let mut cmd = Self::new(command);
        cmd.name = Some(name);
        cmd
    }

    /// 设置连接 ID
    pub fn for_connection(mut self, connection_id: i64) -> Self {
        self.connection_id = Some(connection_id);
        self
    }

    /// 设置为置顶
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }
}

/// 快捷命令行映射
struct QuickCommandRow {
    id: i64,
    name: Option<String>,
    command: String,
    description: Option<String>,
    pinned: i32,
    sort_order: i32,
    connection_id: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

impl FromSqliteRow for QuickCommandRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(QuickCommandRow {
            id: row.get("id")?,
            name: row.get("name")?,
            command: row.get("command")?,
            description: row.get("description")?,
            pinned: row.get("pinned")?,
            sort_order: row.get("sort_order")?,
            connection_id: row.get("connection_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

impl From<QuickCommandRow> for QuickCommand {
    fn from(row: QuickCommandRow) -> Self {
        QuickCommand {
            id: Some(row.id),
            name: row.name,
            command: row.command,
            description: row.description,
            pinned: row.pinned != 0,
            sort_order: row.sort_order,
            connection_id: row.connection_id,
            created_at: Some(row.created_at),
            updated_at: Some(row.updated_at),
        }
    }
}

/// 快捷命令仓库
#[derive(Clone)]
pub struct QuickCommandRepository {
    conn: SqliteConnection,
}

impl QuickCommandRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }

    /// 按连接 ID 获取快捷命令（置顶优先，然后按排序顺序）
    pub fn list_by_connection(&self, connection_id: Option<i64>) -> Result<Vec<QuickCommand>> {
        self.conn.with_connection(|conn| {
            let mut results = Vec::new();

            if let Some(cid) = connection_id {
                let mut stmt = conn.prepare(
                    "SELECT id, name, command, description, pinned, sort_order, connection_id, created_at, updated_at
                     FROM quick_commands
                     WHERE connection_id = ?1 OR connection_id IS NULL
                     ORDER BY pinned DESC, sort_order ASC, created_at DESC"
                )?;
                let rows = stmt.query_map(params![cid], |row| QuickCommandRow::from_row(row))?;
                for row in rows {
                    results.push(row?.into());
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, name, command, description, pinned, sort_order, connection_id, created_at, updated_at
                     FROM quick_commands
                     WHERE connection_id IS NULL
                     ORDER BY pinned DESC, sort_order ASC, created_at DESC"
                )?;
                let rows = stmt.query_map([], |row| QuickCommandRow::from_row(row))?;
                for row in rows {
                    results.push(row?.into());
                }
            }

            Ok(results)
        })
    }

    /// 搜索快捷命令
    pub fn search(&self, query: &str, connection_id: Option<i64>) -> Result<Vec<QuickCommand>> {
        let pattern = format!("%{}%", query);
        self.conn.with_connection(|conn| {
            let mut results = Vec::new();

            if let Some(cid) = connection_id {
                let mut stmt = conn.prepare(
                    "SELECT id, name, command, description, pinned, sort_order, connection_id, created_at, updated_at
                     FROM quick_commands
                     WHERE (command LIKE ?1 OR name LIKE ?1 OR description LIKE ?1)
                       AND (connection_id = ?2 OR connection_id IS NULL)
                     ORDER BY pinned DESC, sort_order ASC, created_at DESC"
                )?;
                let rows = stmt.query_map(params![pattern, cid], |row| QuickCommandRow::from_row(row))?;
                for row in rows {
                    results.push(row?.into());
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, name, command, description, pinned, sort_order, connection_id, created_at, updated_at
                     FROM quick_commands
                     WHERE (command LIKE ?1 OR name LIKE ?1 OR description LIKE ?1)
                       AND connection_id IS NULL
                     ORDER BY pinned DESC, sort_order ASC, created_at DESC"
                )?;
                let rows = stmt.query_map(params![pattern], |row| QuickCommandRow::from_row(row))?;
                for row in rows {
                    results.push(row?.into());
                }
            }

            Ok(results)
        })
    }

    /// 切换置顶状态
    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let current: i32 = conn.query_row(
                "SELECT pinned FROM quick_commands WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )?;
            let new_pinned = if current == 0 { 1 } else { 0 };
            let ts = now();
            conn.execute(
                "UPDATE quick_commands SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_pinned, ts, id],
            )?;
            Ok(new_pinned != 0)
        })
    }

    /// 更新排序顺序
    pub fn update_sort_order(&self, id: i64, sort_order: i32) -> Result<()> {
        let ts = now();
        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE quick_commands SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                params![sort_order, ts, id],
            )?;
            Ok(())
        })
    }

    /// 获取下一个排序顺序值
    pub fn next_sort_order(&self, connection_id: Option<i64>) -> Result<i32> {
        self.conn.with_connection(|conn| {
            let max: Option<i32> = if connection_id.is_some() {
                conn.query_row(
                    "SELECT MAX(sort_order) FROM quick_commands WHERE connection_id = ?1 OR connection_id IS NULL",
                    params![connection_id],
                    |row| row.get(0),
                )?
            } else {
                conn.query_row(
                    "SELECT MAX(sort_order) FROM quick_commands WHERE connection_id IS NULL",
                    [],
                    |row| row.get(0),
                )?
            };
            Ok(max.unwrap_or(0) + 1)
        })
    }
}

impl Repository for QuickCommandRepository {
    type Entity = QuickCommand;

    fn entity_type(&self) -> SharedString {
        SharedString::from("QuickCommand")
    }

    fn insert(&self, item: &mut Self::Entity) -> Result<i64> {
        let ts = now();
        let pinned = if item.pinned { 1 } else { 0 };

        let id = self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT INTO quick_commands (name, command, description, pinned, sort_order, connection_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    item.name,
                    item.command,
                    item.description,
                    pinned,
                    item.sort_order,
                    item.connection_id,
                    ts,
                    ts
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        item.id = Some(id);
        item.created_at = Some(ts);
        item.updated_at = Some(ts);

        Ok(id)
    }

    fn update(&self, item: &Self::Entity) -> Result<()> {
        let id = item.id.ok_or_else(|| anyhow::anyhow!("Cannot update without ID"))?;
        let ts = now();
        let pinned = if item.pinned { 1 } else { 0 };

        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE quick_commands SET name = ?1, command = ?2, description = ?3, pinned = ?4, sort_order = ?5, connection_id = ?6, updated_at = ?7 WHERE id = ?8",
                params![
                    item.name,
                    item.command,
                    item.description,
                    pinned,
                    item.sort_order,
                    item.connection_id,
                    ts,
                    id
                ],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute("DELETE FROM quick_commands WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn get(&self, id: i64) -> Result<Option<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, command, description, pinned, sort_order, connection_id, created_at, updated_at FROM quick_commands WHERE id = ?1",
            )?;
            let mut rows = stmt.query(params![id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(QuickCommandRow::from_row(row)?.into()))
            } else {
                Ok(None)
            }
        })
    }

    fn list(&self) -> Result<Vec<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, command, description, pinned, sort_order, connection_id, created_at, updated_at FROM quick_commands ORDER BY pinned DESC, sort_order ASC, created_at DESC",
            )?;
            let rows = stmt.query_map([], |row| QuickCommandRow::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }

    fn count(&self) -> Result<i64> {
        self.conn.with_connection(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM quick_commands", [], |row| row.get(0))?;
            Ok(count)
        })
    }

    fn exists(&self, id: i64) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let exists: i64 = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM quick_commands WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
            Ok(exists == 1)
        })
    }
}
