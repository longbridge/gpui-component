use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const DEFAULT_POOL_SIZE: usize = 4;

struct PoolInner {
    connections: Vec<Connection>,
    path: PathBuf,
}

pub struct SqliteConnection {
    inner: Arc<Mutex<PoolInner>>,
}

impl Clone for SqliteConnection {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub struct PooledConnection<'a> {
    conn: Option<Connection>,
    pool: &'a SqliteConnection,
}

impl Deref for PooledConnection<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().expect("connection already returned")
    }
}

impl DerefMut for PooledConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().expect("connection already returned")
    }
}

impl Drop for PooledConnection<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if let Ok(mut guard) = self.pool.inner.lock() {
                guard.connections.push(conn);
            }
        }
    }
}

fn create_connection(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
    )?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )?;

    Ok(conn)
}

impl SqliteConnection {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_pool_size(path, DEFAULT_POOL_SIZE)
    }

    pub fn open_with_pool_size(path: impl AsRef<Path>, pool_size: usize) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut connections = Vec::with_capacity(pool_size);

        for _ in 0..pool_size {
            connections.push(create_connection(&path)?);
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(PoolInner { connections, path })),
        })
    }

    fn get_connection(&self) -> Result<PooledConnection<'_>> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let conn = if let Some(conn) = guard.connections.pop() {
            conn
        } else {
            create_connection(&guard.path)?
        };
        drop(guard);

        Ok(PooledConnection {
            conn: Some(conn),
            pool: self,
        })
    }

    pub fn with_connection<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.get_connection()?;
        f(&conn)
    }

    pub fn with_connection_mut<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Connection) -> Result<T>,
    {
        let mut conn = self.get_connection()?;
        f(&mut conn)
    }
}
