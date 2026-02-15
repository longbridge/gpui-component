use anyhow::Result;
use gpui::SharedString;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::connection::SqliteConnection;
use crate::storage::now;
use crate::storage::row_mapping::FromSqliteRow;
use crate::storage::traits::Repository;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: i64,
    pub name: String,
    pub provider_id: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl FromSqliteRow for ChatSession {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(ChatSession {
            id: row.get("id")?,
            name: row.get("name")?,
            provider_id: row.get("provider_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

impl crate::storage::traits::Entity for ChatSession {
    fn id(&self) -> Option<i64> {
        Some(self.id)
    }

    fn created_at(&self) -> i64 {
        self.created_at
    }

    fn updated_at(&self) -> i64 {
        self.updated_at
    }
}

impl ChatSession {
    pub fn new(name: String, provider_id: String) -> Self {
        let now = now();
        Self {
            id: 0,
            name,
            provider_id,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

impl FromSqliteRow for ChatMessage {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(ChatMessage {
            id: row.get("id")?,
            session_id: row.get("session_id")?,
            role: row.get("role")?,
            content: row.get("content")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl crate::storage::traits::Entity for ChatMessage {
    fn id(&self) -> Option<i64> {
        Some(self.id)
    }

    fn created_at(&self) -> i64 {
        self.created_at
    }

    fn updated_at(&self) -> i64 {
        self.created_at
    }
}

impl ChatMessage {
    pub fn new(session_id: i64, role: String, content: String) -> Self {
        Self {
            id: 0,
            session_id,
            role,
            content,
            created_at: now(),
        }
    }

    pub fn user(session_id: i64, content: String) -> Self {
        Self::new(session_id, "user".to_string(), content)
    }

    pub fn assistant(session_id: i64, content: String) -> Self {
        Self::new(session_id, "assistant".to_string(), content)
    }

    pub fn system(session_id: i64, content: String) -> Self {
        Self::new(session_id, "system".to_string(), content)
    }
}

#[derive(Clone)]
pub struct SessionRepository {
    conn: SqliteConnection,
}

impl SessionRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }
}

impl Repository for SessionRepository {
    type Entity = ChatSession;

    fn entity_type(&self) -> SharedString {
        SharedString::from("ChatSession")
    }

    fn insert(&self, item: &mut Self::Entity) -> Result<i64> {
        let name = item.name.clone();
        let provider_id = item.provider_id.clone();
        let created_at = item.created_at;
        let updated_at = item.updated_at;

        let id = self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT INTO chat_sessions (name, provider_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![name, provider_id, created_at, updated_at],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        item.id = id;
        Ok(id)
    }

    fn update(&self, item: &Self::Entity) -> Result<()> {
        let id = item.id;
        let name = item.name.clone();
        let provider_id = item.provider_id.clone();
        let updated_at = now();

        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE chat_sessions SET name = ?1, provider_id = ?2, updated_at = ?3 WHERE id = ?4",
                params![name, provider_id, updated_at, id],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn get(&self, id: i64) -> Result<Option<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, provider_id, created_at, updated_at FROM chat_sessions WHERE id = ?1")?;
            let mut rows = stmt.query(params![id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(ChatSession::from_row(row)?))
            } else {
                Ok(None)
            }
        })
    }

    fn list(&self) -> Result<Vec<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, provider_id, created_at, updated_at FROM chat_sessions ORDER BY updated_at DESC")?;
            let rows = stmt.query_map([], |row| ChatSession::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    fn count(&self) -> Result<i64> {
        self.conn.with_connection(|conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM chat_sessions", [], |row| row.get(0))?;
            Ok(count)
        })
    }

    fn exists(&self, id: i64) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let exists: i64 = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM chat_sessions WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
            Ok(exists == 1)
        })
    }
}

impl SessionRepository {
    pub fn list_by_provider(&self, provider_id: &str) -> Result<Vec<ChatSession>> {
        let provider_id = provider_id.to_string();
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, provider_id, created_at, updated_at FROM chat_sessions WHERE provider_id = ?1 ORDER BY updated_at DESC")?;
            let rows = stmt.query_map(params![provider_id], |row| ChatSession::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }
}

#[derive(Clone)]
pub struct MessageRepository {
    conn: SqliteConnection,
}

impl MessageRepository {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn }
    }
}

impl Repository for MessageRepository {
    type Entity = ChatMessage;

    fn entity_type(&self) -> SharedString {
        SharedString::from("ChatMessage")
    }

    fn insert(&self, item: &mut Self::Entity) -> Result<i64> {
        let session_id = item.session_id;
        let role = item.role.clone();
        let content = item.content.clone();
        let created_at = item.created_at;

        let id = self.conn.with_connection(|conn| {
            conn.execute(
                "INSERT INTO chat_messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![session_id, role, content, created_at],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        item.id = id;
        Ok(id)
    }

    fn update(&self, item: &Self::Entity) -> Result<()> {
        let id = item.id;
        let session_id = item.session_id;
        let role = item.role.clone();
        let content = item.content.clone();

        self.conn.with_connection(|conn| {
            conn.execute(
                "UPDATE chat_messages SET session_id = ?1, role = ?2, content = ?3 WHERE id = ?4",
                params![session_id, role, content, id],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute("DELETE FROM chat_messages WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn get(&self, id: i64) -> Result<Option<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, session_id, role, content, created_at FROM chat_messages WHERE id = ?1")?;
            let mut rows = stmt.query(params![id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(ChatMessage::from_row(row)?))
            } else {
                Ok(None)
            }
        })
    }

    fn list(&self) -> Result<Vec<Self::Entity>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, session_id, role, content, created_at FROM chat_messages ORDER BY created_at ASC")?;
            let rows = stmt.query_map([], |row| ChatMessage::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    fn count(&self) -> Result<i64> {
        self.conn.with_connection(|conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM chat_messages", [], |row| row.get(0))?;
            Ok(count)
        })
    }

    fn exists(&self, id: i64) -> Result<bool> {
        self.conn.with_connection(|conn| {
            let exists: i64 = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM chat_messages WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
            Ok(exists == 1)
        })
    }
}

impl MessageRepository {
    pub fn list_by_session(&self, session_id: i64) -> Result<Vec<ChatMessage>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, session_id, role, content, created_at FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC")?;
            let rows = stmt.query_map(params![session_id], |row| ChatMessage::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    pub fn list_recent(&self, limit: i32) -> Result<Vec<ChatMessage>> {
        self.conn.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, session_id, role, content, created_at FROM chat_messages ORDER BY created_at DESC LIMIT ?1")?;
            let rows = stmt.query_map(params![limit], |row| ChatMessage::from_row(row))?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    pub fn delete_by_session(&self, session_id: i64) -> Result<()> {
        self.conn.with_connection(|conn| {
            conn.execute(
                "DELETE FROM chat_messages WHERE session_id = ?1",
                params![session_id],
            )?;
            Ok(())
        })
    }

}
