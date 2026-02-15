use anyhow::Result;
use rusqlite::Connection;

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "20241219000001",
        include_str!("../../migrations/20241219000001_create_workspaces.sql"),
    ),
    (
        "20241219000002",
        include_str!("../../migrations/20241219000002_create_connections.sql"),
    ),
    (
        "20241219000003",
        include_str!("../../migrations/20241219000003_create_queries.sql"),
    ),
    (
        "20241219000004",
        include_str!("../../migrations/20241219000004_create_llm_providers.sql"),
    ),
    (
        "20241219000005",
        include_str!("../../migrations/20241219000005_create_chat_sessions.sql"),
    ),
    (
        "20241219000006",
        include_str!("../../migrations/20241219000006_create_chat_messages.sql"),
    ),
    (
        "20241219000007",
        include_str!("../../migrations/20241219000007_add_selected_databases_to_connections.sql"),
    ),
    (
        "20241219000008",
        include_str!("../../migrations/20241219000008_add_remark_to_connections.sql"),
    ),
    (
        "20250128000001",
        include_str!("../../migrations/20250128000001_create_terminal_commands.sql"),
    ),
    (
        "20250131000001",
        include_str!("../../migrations/20250131000001_create_quick_commands.sql"),
    ),
    (
        "20250201000001",
        include_str!("../../migrations/20250201000001_add_sync_fields_to_connections.sql"),
    ),
    (
        "20250202000001",
        include_str!("../../migrations/20250202000001_remove_unique_constraint_on_connection_name.sql"),
    ),
    (
        "20250203000001",
        include_str!("../../migrations/20250203000001_create_pending_cloud_deletions.sql"),
    ),
    (
        "20250203000002",
        include_str!("../../migrations/20250203000002_add_cloud_id_to_workspaces.sql"),
    ),
    (
        "20260208000001",
        include_str!("../../migrations/20260208000001_add_models_to_llm_providers.sql"),
    ),
];

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );",
    )?;

    for (version, sql) in MIGRATIONS {
        let applied: i64 = conn.query_row(
            "SELECT COUNT(*) FROM _migrations WHERE version = ?1",
            [version],
            |row| row.get(0),
        )?;

        if applied == 0 {
            if let Err(e) = conn.execute_batch(sql) {
                let err_msg = e.to_string();
                if err_msg.contains("duplicate column name") {
                    tracing::warn!(
                        "Migration {} skipped (column already exists): {}",
                        version,
                        err_msg
                    );
                } else {
                    return Err(e.into());
                }
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as i64;

            conn.execute(
                "INSERT INTO _migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![version, now],
            )?;
        }
    }

    Ok(())
}
