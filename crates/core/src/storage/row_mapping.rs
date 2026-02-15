use rusqlite::{Result as SqliteResult, Row};

pub trait FromSqliteRow: Sized {
    fn from_row(row: &Row<'_>) -> SqliteResult<Self>;
}

impl FromSqliteRow for (i64,) {
    fn from_row(row: &Row<'_>) -> SqliteResult<Self> {
        Ok((row.get(0)?,))
    }
}

impl FromSqliteRow for (i32,) {
    fn from_row(row: &Row<'_>) -> SqliteResult<Self> {
        Ok((row.get(0)?,))
    }
}
