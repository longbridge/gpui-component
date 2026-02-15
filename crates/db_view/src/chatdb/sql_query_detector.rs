//! SQL 查询判定 - 使用数据库插件的 SQLParser 进行判定

use db::{GlobalDbState, is_query_statement_fallback};

/// 基于连接配置与数据库插件判定 SQL 是否为查询语句
pub fn is_query_statement_for_connection(
    global_state: &GlobalDbState,
    connection_id: &str,
    sql: &str,
) -> bool {
    let Some(config) = global_state.get_config(connection_id) else {
        return is_query_statement_fallback(sql);
    };

    match global_state.get_plugin(&config.database_type) {
        Ok(plugin) => plugin.is_query_statement(sql),
        Err(_) => is_query_statement_fallback(sql),
    }
}
