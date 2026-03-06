use std::collections::HashMap;

use anyhow::Result;
use rusqlite::Connection;
use tracing::{error, info};

use super::manager::get_config_dir;
use super::models::{DatabaseType, DbConnectionConfig, StoredConnection};
use super::repository::ConnectionRepository;
use super::traits::Repository;

const DEMO_SQL: &str = include_str!("demo_orders.sql");
const DEMO_DB_FILENAME: &str = "demo_orders.db";
const DEMO_CONNECTION_NAME: &str = "Demo Orders (SQLite)";

/// 首次启动时创建演示数据库。
///
/// 仅当连接数为 0 时触发，确保只在全新安装时生效。
/// 所有错误只记录日志，不阻止应用启动。
pub fn try_init_demo(repo: &ConnectionRepository) {
    let count = match repo.count() {
        Ok(c) => c,
        Err(e) => {
            error!("检查连接数失败，跳过演示数据库创建: {e}");
            return;
        }
    };

    if count > 0 {
        return;
    }

    info!("首次启动，开始创建演示数据库...");

    if let Err(e) = init_demo_inner(repo) {
        error!("创建演示数据库失败: {e}");
    }
}

fn init_demo_inner(repo: &ConnectionRepository) -> Result<()> {
    let config_dir = get_config_dir()?;
    let db_path = config_dir.join(DEMO_DB_FILENAME);

    if !db_path.exists() {
        create_demo_db_file(&db_path)?;
        info!("演示数据库文件已创建: {}", db_path.display());
    }

    register_demo_connection(repo, &db_path)?;
    info!("演示连接已注册: {DEMO_CONNECTION_NAME}");

    Ok(())
}

/// 创建 SQLite 数据库文件并执行建表与数据插入脚本。
fn create_demo_db_file(path: &std::path::Path) -> Result<()> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(DEMO_SQL)?;
    Ok(())
}

/// 构建 StoredConnection 并写入应用数据库。
fn register_demo_connection(repo: &ConnectionRepository, db_path: &std::path::Path) -> Result<i64> {
    let path_str = db_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("演示数据库路径包含非法字符"))?
        .to_string();

    let config = DbConnectionConfig {
        id: String::new(),
        database_type: DatabaseType::SQLite,
        name: DEMO_CONNECTION_NAME.to_string(),
        host: path_str,
        port: 0,
        username: String::new(),
        password: String::new(),
        database: None,
        service_name: None,
        sid: None,
        workspace_id: None,
        extra_params: HashMap::new(),
    };

    let mut stored = StoredConnection::new_database(DEMO_CONNECTION_NAME.to_string(), config, None);
    stored.sync_enabled = false;

    let id = repo.insert(&mut stored)?;
    Ok(id)
}
