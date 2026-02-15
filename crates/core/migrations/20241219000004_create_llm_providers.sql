-- Create llm_providers table
CREATE TABLE IF NOT EXISTS llm_providers (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    api_key TEXT,
    api_base TEXT,
    model TEXT NOT NULL,
    max_tokens INTEGER,
    temperature REAL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
