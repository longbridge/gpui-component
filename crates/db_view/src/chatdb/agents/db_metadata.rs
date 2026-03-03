//! DatabaseMetadataProvider - 数据库元数据访问能力
//!
//! 封装 GlobalDbState 的纯 async 方法，作为 AgentContext 的 capability 注入。

use db::{ExecOptions, GlobalDbState, QueryResult, SqlResult};
use one_core::storage::DatabaseType;

use crate::chatdb::agents::query_workflow::{ColumnMeta, TableBrief, TableMeta};

/// Capability key for database metadata access.
pub const CAP_DB_METADATA: &str = "database_metadata";

/// Provides async access to database metadata (tables, columns) without requiring `AsyncApp`.
#[derive(Clone)]
pub struct DatabaseMetadataProvider {
    global_db_state: GlobalDbState,
    pub connection_id: String,
    pub database: String,
    pub schema: Option<String>,
    pub database_type: DatabaseType,
}

impl DatabaseMetadataProvider {
    pub fn new(
        global_db_state: GlobalDbState,
        connection_id: String,
        database: String,
        schema: Option<String>,
        database_type: DatabaseType,
    ) -> Self {
        Self {
            global_db_state,
            connection_id,
            database,
            schema,
            database_type,
        }
    }

    /// List all tables (name + comment only).
    pub async fn list_tables(&self) -> anyhow::Result<Vec<TableBrief>> {
        let tables = self
            .global_db_state
            .list_tables_direct(&self.connection_id, &self.database, self.schema.clone())
            .await?;

        Ok(tables
            .into_iter()
            .map(|t| TableBrief {
                name: t.name,
                comment: t.comment,
            })
            .collect())
    }

    /// Fetch full metadata for a single table.
    pub async fn fetch_table_metadata(&self, table_name: &str) -> anyhow::Result<TableMeta> {
        let columns = self
            .global_db_state
            .list_columns_direct(
                &self.connection_id,
                &self.database,
                self.schema.clone(),
                table_name,
            )
            .await?;

        Ok(TableMeta {
            name: table_name.to_string(),
            comment: None,
            columns: columns
                .into_iter()
                .map(|c| ColumnMeta {
                    name: c.name,
                    data_type: c.data_type,
                    nullable: c.is_nullable,
                    comment: c.comment,
                    is_primary_key: c.is_primary_key,
                })
                .collect(),
        })
    }

    /// Execute a query and return the first query result set.
    pub async fn execute_query_preview(
        &self,
        sql: &str,
        max_rows: usize,
    ) -> anyhow::Result<QueryResult> {
        let opts = ExecOptions {
            max_rows: Some(max_rows),
            ..Default::default()
        };

        let results = self
            .global_db_state
            .execute_script_direct(
                &self.connection_id,
                sql,
                Some(self.database.clone()),
                self.schema.clone(),
                Some(opts),
            )
            .await?;

        for result in results {
            match result {
                SqlResult::Query(query) => return Ok(query),
                SqlResult::Error(err) => return Err(anyhow::anyhow!(err.message)),
                SqlResult::Exec(_) => continue,
            }
        }

        Err(anyhow::anyhow!("No query result returned"))
    }
}
