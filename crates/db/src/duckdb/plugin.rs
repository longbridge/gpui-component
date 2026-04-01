use anyhow::Result;
use async_trait::async_trait;
use one_core::storage::{DatabaseType, DbConnectionConfig};

use crate::connection::{DbConnection, DbError};
use crate::duckdb::DuckDbConnection;
use crate::import_export::{
    ExportConfig, ExportProgressSender, ExportResult, ImportConfig, ImportProgressSender,
    ImportResult,
};
use crate::plugin::{DatabaseOperationRequest, DatabasePlugin, SqlCompletionInfo};
use crate::sqlite::SqlitePlugin;
use crate::types::*;

pub struct DuckDbPlugin {
    sqlite: SqlitePlugin,
}

impl DuckDbPlugin {
    pub fn new() -> Self {
        Self {
            sqlite: SqlitePlugin::new(),
        }
    }
}

impl Default for DuckDbPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabasePlugin for DuckDbPlugin {
    fn name(&self) -> DatabaseType {
        DatabaseType::DuckDB
    }

    fn quote_identifier(&self, identifier: &str) -> String {
        format!("\"{}\"", identifier.replace("\"", "\"\""))
    }

    fn get_completion_info(&self) -> SqlCompletionInfo {
        let mut info = self.sqlite.get_completion_info();
        info.keywords.extend([
            ("COPY", "Copy query results to a file"),
            ("INSTALL", "Install an extension"),
            ("LOAD", "Load an installed extension"),
        ]);
        info
    }

    async fn create_connection(
        &self,
        config: DbConnectionConfig,
    ) -> Result<Box<dyn DbConnection + Send + Sync>, DbError> {
        let mut conn = DuckDbConnection::new(config);
        conn.connect().await?;
        Ok(Box::new(conn))
    }

    async fn list_databases(&self, connection: &dyn DbConnection) -> Result<Vec<String>> {
        self.sqlite.list_databases(connection).await
    }

    async fn list_databases_view(&self, connection: &dyn DbConnection) -> Result<ObjectView> {
        self.sqlite.list_databases_view(connection).await
    }

    async fn list_databases_detailed(
        &self,
        connection: &dyn DbConnection,
    ) -> Result<Vec<DatabaseInfo>> {
        self.sqlite.list_databases_detailed(connection).await
    }

    fn sql_dialect(&self) -> Box<dyn sqlparser::dialect::Dialect> {
        Box::new(sqlparser::dialect::DuckDbDialect {})
    }

    async fn list_tables(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<TableInfo>> {
        self.sqlite.list_tables(connection, database, schema).await
    }

    async fn list_tables_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<ObjectView> {
        self.sqlite.list_tables_view(connection, database, schema).await
    }

    async fn list_columns(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<ColumnInfo>> {
        self.sqlite
            .list_columns(connection, database, schema, table)
            .await
    }

    async fn list_columns_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<ObjectView> {
        self.sqlite
            .list_columns_view(connection, database, schema, table)
            .await
    }

    async fn list_indexes(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<IndexInfo>> {
        self.sqlite
            .list_indexes(connection, database, schema, table)
            .await
    }

    async fn list_indexes_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Result<ObjectView> {
        self.sqlite
            .list_indexes_view(connection, database, schema, table)
            .await
    }

    async fn list_views(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<ViewInfo>> {
        self.sqlite.list_views(connection, database, schema).await
    }

    async fn list_views_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        self.sqlite.list_views_view(connection, database).await
    }

    fn supports_functions(&self) -> bool {
        false
    }

    async fn list_functions(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        self.sqlite.list_functions(connection, database).await
    }

    async fn list_functions_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        self.sqlite.list_functions_view(connection, database).await
    }

    fn supports_procedures(&self) -> bool {
        false
    }

    async fn list_procedures(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        self.sqlite.list_procedures(connection, database).await
    }

    async fn list_procedures_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        self.sqlite.list_procedures_view(connection, database).await
    }

    async fn list_triggers(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<TriggerInfo>> {
        self.sqlite.list_triggers(connection, database).await
    }

    async fn list_triggers_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        self.sqlite.list_triggers_view(connection, database).await
    }

    async fn list_sequences(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<SequenceInfo>> {
        self.sqlite
            .list_sequences(connection, database, schema)
            .await
    }

    async fn list_sequences_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        self.sqlite.list_sequences_view(connection, database).await
    }

    fn build_column_definition(&self, column: &ColumnInfo, include_name: bool) -> String {
        self.sqlite.build_column_definition(column, include_name)
    }

    fn build_create_database_sql(&self, _request: &DatabaseOperationRequest) -> String {
        "-- DuckDB: database is created when opening a file".to_string()
    }

    fn build_modify_database_sql(&self, _request: &DatabaseOperationRequest) -> String {
        "-- DuckDB: database modification not supported".to_string()
    }

    fn build_drop_database_sql(&self, _database_name: &str) -> String {
        "-- DuckDB: delete the database file to drop the database".to_string()
    }

    fn format_table_reference(
        &self,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> String {
        self.sqlite.format_table_reference(database, schema, table)
    }

    fn build_limit_clause(&self) -> String {
        self.sqlite.build_limit_clause()
    }

    fn build_where_and_limit_clause(
        &self,
        request: &TableSaveRequest,
        original_data: &[String],
    ) -> (String, String) {
        self.sqlite
            .build_where_and_limit_clause(request, original_data)
    }

    async fn export_table_create_sql(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Result<String> {
        self.sqlite
            .export_table_create_sql(connection, database, schema, table)
            .await
    }

    fn get_data_types(&self) -> &[(&'static str, &'static str)] {
        self.sqlite.get_data_types()
    }

    fn drop_table(&self, database: &str, schema: Option<&str>, table: &str) -> String {
        self.sqlite.drop_table(database, schema, table)
    }

    fn truncate_table(&self, database: &str, table: &str) -> String {
        self.sqlite.truncate_table(database, table)
    }

    fn rename_table(&self, database: &str, old_name: &str, new_name: &str) -> String {
        self.sqlite.rename_table(database, old_name, new_name)
    }

    fn build_backup_table_sql(
        &self,
        database: &str,
        schema: Option<&str>,
        original_table: &str,
        backup_table: &str,
    ) -> String {
        self.sqlite
            .build_backup_table_sql(database, schema, original_table, backup_table)
    }

    fn drop_view(&self, database: &str, view: &str) -> String {
        self.sqlite.drop_view(database, view)
    }

    fn build_column_def(&self, col: &ColumnDefinition) -> String {
        self.sqlite.build_column_def(col)
    }

    fn build_create_table_sql(&self, design: &TableDesign) -> String {
        self.sqlite.build_create_table_sql(design)
    }

    fn build_alter_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        self.sqlite.build_alter_table_sql(original, new)
    }

    async fn import_data_with_progress(
        &self,
        connection: &dyn DbConnection,
        config: &ImportConfig,
        data: &str,
        file_name: &str,
        progress_tx: Option<ImportProgressSender>,
    ) -> Result<ImportResult> {
        self.sqlite
            .import_data_with_progress(connection, config, data, file_name, progress_tx)
            .await
    }

    async fn export_data_with_progress(
        &self,
        connection: &dyn DbConnection,
        config: &ExportConfig,
        progress_tx: Option<ExportProgressSender>,
    ) -> Result<ExportResult> {
        self.sqlite
            .export_data_with_progress(connection, config, progress_tx)
            .await
    }
}
