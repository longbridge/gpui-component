//! 终端命令历史存储模块

use anyhow::Result;
use gpui::SharedString;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::connection::SqliteConnection;
use crate::storage::manager::now;
use crate::storage::row_mapping::FromSqliteRow;
use crate::storage::traits::{Entity, Repository};

/// 终端命令历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCommand {
    /// 记录 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// 关联的终端会话 ID（可选，用于按会话分组）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<i64>,
    /// 连接 ID（SSH 连接的 StoredConnection.id）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<i64>,
    /// 命令内容
    pub command: String,
    /// 命令执行目录（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    /// 命令执行时间戳
    pub executed_at: i64,
    /// 命令执行结果状态（可选：0=成功，非0=失败）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// 创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

impl Entity for TerminalCommand {
    fn id(&self) -> Option<i64> {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at.unwrap_or(0)
    }

    fn updated_at(&self) -> i64 {
        self.created_at.unwrap_or(0)
    }
}

impl TerminalCommand {
    /// 创建新的终端命令记录
    pub fn new(command: String, connection_id: Option<i64>) -> Self {
        let ts = now();
        Self {
            id: None,
            session_id: None,
            connection_id,
            command,
            working_directory: None,
            executed_at: ts,
            exit_code: None,
            created_at: Some(ts),
        }
    }

    /// 创建带会话 ID 的终端命令记录
    pub fn with_session(command: String, connection_id: Option<i64>, session_id: i64) -> Self {
        let mut cmd = Self::new(command, connection_id);
        cmd.session_id = Some(session_id);
        cmd
    }
}

/// 终端命令行映射
struct TerminalCommandRow {
    id: i64,
    session_id: Option<i64>,
    connection_id: Option<i64>,
    command: String,
    working_directory: Option<String>,
    executed_at: i64,
    exit_code: Option<i32>,
    created_at: i64,
}

impl FromSqliteRow for TerminalCommandRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(TerminalCommandRow {
            id: row.get("id")?,
            session_id: row.get("session_id")?,
            connection_id: row.get("connection_id")?,
            command: row.get("command")?,
            working_directory: row.get("working_directory")?,
            executed_at: row.get("executed_at")?,
            exit_code: row.get("exit_code")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl From<TerminalCommandRow> for TerminalCommand {
    fn from(row: TerminalCommandRow) -> Self {
        TerminalCommand {
            id: Some(row.id),
            session_id: row.session_id,
            connection_id: row.connection_id,
            command: row.command,
            working_directory: row.working_directory,
            executed_at: row.executed_at,
            exit_code: row.exit_code,
            created_at: Some(row.created_at),
        }
    }
}

/// 终端命令历史仓库
#[derive(Clone)]
pub struct TerminalCommandRepository {
    conn: SqliteConnection,
}

impl TerminalCommandRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }

    /// 按连接 ID 获取命令历史（最近 N 条）
    pub fn list_by_connection(&self, connection_id: i64, limit: i32) -> Result<Vec<TerminalCommand>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, connection_id, command, working_directory, executed_at, exit_code, created_at
                 FROM terminal_commands
                 WHERE connection_id = ?1
                 ORDER BY executed_at DESC
                 LIMIT ?2"
            )?;
            let rows = stmt.query_map(params![connection_id, limit], |row| {
                TerminalCommandRow::from_row(row)
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }

    /// 按会话 ID 获取命令历史
    pub fn list_by_session(&self, session_id: i64, limit: i32) -> Result<Vec<TerminalCommand>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, connection_id, command, working_directory, executed_at, exit_code, created_at
                 FROM terminal_commands
                 WHERE session_id = ?1
                 ORDER BY executed_at DESC
                 LIMIT ?2"
            )?;
            let rows = stmt.query_map(params![session_id, limit], |row| {
                TerminalCommandRow::from_row(row)
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?.into());
            }
            Ok(results)
        })
    }

    /// 搜索命令历史（模糊匹配）
    pub fn search(&self, query: &str, connection_id: Option<i64>, limit: i32) -> Result<Vec<TerminalCommand>> {
        let pattern = format!("%{}%", query);
        self.conn.with_connection(|conn| {
            if let Some(cid) = connection_id {
                let mut stmt = conn.prepare(
                    "SELECT id, session_id, connection_id, command, working_directory, executed_at, exit_code, created_at
                     FROM terminal_commands
                     WHERE command LIKE ?1 AND connection_id = ?2
                     ORDER BY executed_at DESC
                     LIMIT ?3"
                )?;
                let rows = stmt.query_map(params![pattern, cid, limit], |row| {
                    TerminalCommandRow::from_row(row)
                })?;
                let mut results = Vec::new();
                for row in rows {
                    results.push(row?.into());
                }
                Ok(results)
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, session_id, connection_id, command, working_directory, executed_at, exit_code, created_at
                     FROM terminal_commands
                     WHERE command LIKE ?1
                     ORDER BY executed_at DESC
                     LIMIT ?2"
                )?;
                let rows = stmt.query_map(params![pattern, limit], |row| {
                    TerminalCommandRow::from_row(row)
                })?;
                let mut results = Vec::new();
                for row in rows {
                    results.push(row?.into());
                }
                Ok(results)
            }
        })
    }

    /// 获取去重的命令列表（用于自动补全）
    pub fn list_unique_commands(&self, connection_id: Option<i64>, limit: i32) -> Result<Vec<String>> {
        self.conn.with_connection(|conn| {
            if let Some(cid) = connection_id {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT command FROM terminal_commands
                     WHERE connection_id = ?1
                     ORDER BY executed_at DESC
                     LIMIT ?2"
                )?;
                let rows = stmt.query_map(params![cid, limit], |row| row.get::<_, String>(0))?;
                let mut results = Vec::new();
                for row in rows {
                    results.push(row?);
                }
                Ok(results)
            } else {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT command FROM terminal_commands
                     ORDER BY executed_at DESC
                     LIMIT ?1"
                )?;
                let rows = stmt.query_map(params![limit], |row| row.get::<_, String>(0))?;
                let mut results = Vec::new();
                for row in rows {
                    results.push(row?);
                }
                Ok(results)
            }
        })
    }

    /// 删除连接相关的所有命令历史
    pub fn delete_by_connection(&self, connection_id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute(
                "DELETE FROM terminal_commands WHERE connection_id = ?1",
                params![connection_id],
            )?;
            Ok(())
        })
    }

    /// 清理旧的命令历史（保留最近 N 条）
    pub fn cleanup_old_commands(&self, connection_id: Option<i64>, keep_count: i32) -> Result<()> {
        self.conn.with_connection(|conn| {
            if let Some(cid) = connection_id {
                conn.execute(
                    "DELETE FROM terminal_commands
                     WHERE connection_id = ?1 AND id NOT IN (
                         SELECT id FROM terminal_commands
                         WHERE connection_id = ?1
                         ORDER BY executed_at DESC
                         LIMIT ?2
                     )",
                    params![cid, keep_count],
                )?;
            } else {
                conn.execute(
                    "DELETE FROM terminal_commands
                     WHERE id NOT IN (
                         SELECT id FROM terminal_commands
                         ORDER BY executed_at DESC
                         LIMIT ?1
                     )",
                    params![keep_count],
                )?;
            }
            Ok(())
        })
    }
}

impl Repository for TerminalCommandRepository {
    type Entity = TerminalCommand;

    fn entity_type(&self) -> SharedString {
        SharedString::from("TerminalCommand")
    }

    fn insert(&self, item: &mut Self::Entity) -> Result<i64> {
        let session_id = item.session_id;
        let connection_id = item.connection_id;
        let command = item.command.clone();
        let working_directory = item.working_directory.clone();
        let executed_at = item.executed_at;
        let exit_code = item.exit_code;
        let ts = now();

        let id = self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT INTO terminal_commands (session_id, connection_id, command, working_directory, executed_at, exit_code, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![session_id, connection_id, command, working_directory, executed_at, exit_code, ts],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        item.id = Some(id);
        item.created_at = Some(ts);

        Ok(id)
    }

    fn update(&self, item: &Self::Entity) -> Result<()> {
        let id = item
            .id
            .ok_or_else(|| anyhow::anyhow!("Cannot update without ID"))?;
        let session_id = item.session_id;
        let connection_id = item.connection_id;
        let command = item.command.clone();
        let working_directory = item.working_directory.clone();
        let executed_at = item.executed_at;
        let exit_code = item.exit_code;

        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE terminal_commands SET session_id = ?1, connection_id = ?2, command = ?3, working_directory = ?4, executed_at = ?5, exit_code = ?6 WHERE id = ?7",
                params![session_id, connection_id, command, working_directory, executed_at, exit_code, id],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute("DELETE FROM terminal_commands WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn get(&self, id: i64) -> Result<Option<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, connection_id, command, working_directory, executed_at, exit_code, created_at FROM terminal_commands WHERE id = ?1",
            )?;
            let mut rows = stmt.query(params![id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(TerminalCommandRow::from_row(row)?.into()))
            } else {
                Ok(None)
            }
        })
    }

    fn list(&self) -> Result<Vec<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, connection_id, command, working_directory, executed_at, exit_code, created_at FROM terminal_commands ORDER BY executed_at DESC",
            )?;
            let rows = stmt.query_map([], |row| TerminalCommandRow::from_row(row))?;
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
                conn.query_row("SELECT COUNT(*) FROM terminal_commands", [], |row| row.get(0))?;
            Ok(count)
        })
    }

    fn exists(&self, id: i64) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let exists: i64 = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM terminal_commands WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
            Ok(exists == 1)
        })
    }
}
