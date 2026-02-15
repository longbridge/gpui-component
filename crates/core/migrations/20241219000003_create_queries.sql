-- Create queries table
CREATE TABLE IF NOT EXISTS queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    connection_id TEXT NOT NULL,
    database_name TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(connection_id, name)
);

CREATE INDEX IF NOT EXISTS idx_queries_connection ON queries(connection_id);
CREATE INDEX IF NOT EXISTS idx_queries_database ON queries(database_name) WHERE database_name IS NOT NULL;
