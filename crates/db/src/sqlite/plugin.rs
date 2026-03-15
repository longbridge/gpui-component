use anyhow::Result;
use async_trait::async_trait;
use gpui_component::table::Column;
use one_core::storage::{DatabaseType, DbConnectionConfig};
use std::collections::HashMap;

use crate::connection::{DbConnection, DbError};
use crate::executor::SqlResult;
use crate::import_export::{
    ExportConfig, ExportProgressSender, ExportResult, ImportConfig, ImportProgressSender,
    ImportResult,
};
use crate::plugin::{DatabasePlugin, SqlCompletionInfo};
use crate::sqlite::SqliteDbConnection;
use crate::types::*;

/// SQLite data types (name, description)
pub const SQLITE_DATA_TYPES: &[(&str, &str)] = &[
    ("INTEGER", "Signed integer (up to 8 bytes)"),
    ("REAL", "8-byte floating point"),
    ("TEXT", "UTF-8 text string"),
    ("BLOB", "Binary large object"),
    ("NUMERIC", "Numeric affinity"),
    ("BOOLEAN", "Boolean (stored as INTEGER)"),
    ("DATE", "Date (stored as TEXT)"),
    ("DATETIME", "Date and time (stored as TEXT)"),
];

/// SQLite database plugin implementation
pub struct SqlitePlugin;

impl SqlitePlugin {
    pub fn new() -> Self {
        Self
    }

    fn build_sqlite_simple_alter_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        let mut statements: Vec<String> = Vec::new();
        let table_name = self.quote_identifier(&new.table_name);

        let original_cols: HashMap<&str, &ColumnDefinition> = original
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();

        for col in new.columns.iter() {
            if !original_cols.contains_key(col.name.as_str()) {
                let col_def = self.build_column_def(col);
                statements.push(format!(
                    "ALTER TABLE {} ADD COLUMN {};",
                    table_name, col_def
                ));
            }
        }

        let original_indexes: HashMap<&str, &IndexDefinition> = original
            .indexes
            .iter()
            .map(|i| (i.name.as_str(), i))
            .collect();
        let new_indexes: HashMap<&str, &IndexDefinition> =
            new.indexes.iter().map(|i| (i.name.as_str(), i)).collect();

        for name in original_indexes.keys() {
            if !new_indexes.contains_key(name) {
                statements.push(format!(
                    "DROP INDEX IF EXISTS {};",
                    self.quote_identifier(name)
                ));
            }
        }

        for (name, idx) in &new_indexes {
            if !original_indexes.contains_key(name) {
                let idx_cols: Vec<String> = idx
                    .columns
                    .iter()
                    .map(|c| self.quote_identifier(c))
                    .collect();

                let unique_str = if idx.is_unique { "UNIQUE " } else { "" };
                statements.push(format!(
                    "CREATE {}INDEX {} ON {} ({});",
                    unique_str,
                    self.quote_identifier(name),
                    table_name,
                    idx_cols.join(", ")
                ));
            }
        }

        if statements.is_empty() {
            "-- No changes detected".to_string()
        } else {
            statements.join("\n")
        }
    }

    fn build_sqlite_recreate_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        let mut statements: Vec<String> = Vec::new();
        let table_name = &new.table_name;
        let temp_table_name = format!("{}_dg_tmp", table_name);

        let mut column_defs: Vec<String> = Vec::new();
        let mut primary_key_cols: Vec<String> = Vec::new();

        for col in &new.columns {
            let mut col_def = format!("{} {}", self.quote_identifier(&col.name), col.data_type);

            if let Some(len) = col.length {
                if let Some(scale) = col.scale {
                    col_def = format!(
                        "{} {}({},{})",
                        self.quote_identifier(&col.name),
                        col.data_type,
                        len,
                        scale
                    );
                } else {
                    col_def = format!(
                        "{} {}({})",
                        self.quote_identifier(&col.name),
                        col.data_type,
                        len
                    );
                }
            }

            if col.is_primary_key && new.columns.iter().filter(|c| c.is_primary_key).count() == 1 {
                col_def.push_str("\n        primary key");
                if col.is_auto_increment {
                    col_def.push_str(" autoincrement");
                }
            }

            if col.is_primary_key {
                primary_key_cols.push(col.name.clone());
            }

            if !col.is_nullable && !col.is_primary_key {
                col_def.push_str(" not null");
            }

            if let Some(default) = &col.default_value {
                col_def.push_str(&format!(" default {}", default));
            }

            column_defs.push(col_def);
        }

        if primary_key_cols.len() > 1 {
            let pk_cols: Vec<String> = primary_key_cols
                .iter()
                .map(|c| self.quote_identifier(c))
                .collect();
            column_defs.push(format!("primary key ({})", pk_cols.join(", ")));
        }

        statements.push(format!(
            "create table {}\n(\n    {}\n);",
            self.quote_identifier(&temp_table_name),
            column_defs.join(",\n    ")
        ));

        let original_col_names: std::collections::HashSet<&str> =
            original.columns.iter().map(|c| c.name.as_str()).collect();

        let common_columns: Vec<&str> = new
            .columns
            .iter()
            .filter(|c| original_col_names.contains(c.name.as_str()))
            .map(|c| c.name.as_str())
            .collect();

        if !common_columns.is_empty() {
            let col_list: Vec<String> = common_columns
                .iter()
                .map(|c| self.quote_identifier(c))
                .collect();
            let col_str = col_list.join(", ");

            statements.push(format!(
                "insert into {}({})\nselect {}\nfrom {};",
                self.quote_identifier(&temp_table_name),
                col_str,
                col_str,
                self.quote_identifier(table_name)
            ));
        }

        statements.push(format!("drop table {};", self.quote_identifier(table_name)));

        statements.push(format!(
            "alter table {}\n    rename to {};",
            self.quote_identifier(&temp_table_name),
            self.quote_identifier(table_name)
        ));

        for idx in &new.indexes {
            if !idx.is_primary && !idx.is_unique {
                let idx_cols: Vec<String> = idx
                    .columns
                    .iter()
                    .map(|c| self.quote_identifier(c))
                    .collect();

                statements.push(format!(
                    "create index {}\n    on {} ({});",
                    self.quote_identifier(&idx.name),
                    self.quote_identifier(table_name),
                    idx_cols.join(", ")
                ));
            }
        }

        for idx in &new.indexes {
            if idx.is_unique && !idx.is_primary {
                let idx_cols: Vec<String> = idx
                    .columns
                    .iter()
                    .map(|c| self.quote_identifier(c))
                    .collect();

                let nullable_cols: Vec<String> = idx
                    .columns
                    .iter()
                    .filter(|col_name| {
                        new.columns
                            .iter()
                            .find(|c| &c.name == *col_name)
                            .map(|c| c.is_nullable)
                            .unwrap_or(false)
                    })
                    .map(|c| format!("{} IS NOT NULL", self.quote_identifier(c)))
                    .collect();

                if nullable_cols.is_empty() {
                    statements.push(format!(
                        "create unique index {}\n    on {} ({});",
                        self.quote_identifier(&idx.name),
                        self.quote_identifier(table_name),
                        idx_cols.join(", ")
                    ));
                } else {
                    statements.push(format!(
                        "create unique index {}\n    on {} ({})\n    where {};",
                        self.quote_identifier(&idx.name),
                        self.quote_identifier(table_name),
                        idx_cols.join(", "),
                        nullable_cols.join(" AND ")
                    ));
                }
            }
        }

        statements.join("\n\n")
    }
}

#[async_trait]
impl DatabasePlugin for SqlitePlugin {
    fn name(&self) -> DatabaseType {
        DatabaseType::SQLite
    }

    fn quote_identifier(&self, identifier: &str) -> String {
        format!("\"{}\"", identifier.replace("\"", "\"\""))
    }

    fn get_completion_info(&self) -> SqlCompletionInfo {
        SqlCompletionInfo {
            keywords: vec![
                ("AUTOINCREMENT", "Auto-increment column"),
                ("VACUUM", "Rebuild database file"),
                ("ATTACH", "Attach another database"),
                ("DETACH", "Detach attached database"),
                ("PRAGMA", "SQLite configuration"),
                ("GLOB", "Unix-style pattern matching"),
                ("REPLACE", "Insert or replace row"),
                ("INDEXED BY", "Force index usage"),
                ("NOT INDEXED", "Disable index usage"),
                ("NULLS FIRST", "Sort NULLs first"),
                ("NULLS LAST", "Sort NULLs last"),
            ],
            functions: vec![
                ("IFNULL(x, y)", "Return y if x is NULL"),
                ("NULLIF(x, y)", "Return NULL if x equals y"),
                ("IIF(cond, x, y)", "If-then-else expression"),
                ("TYPEOF(x)", "Return type name"),
                ("INSTR(str, substr)", "Find substring position"),
                ("PRINTF(fmt, ...)", "Formatted string"),
                ("SUBSTR(str, start, len)", "Extract substring"),
                ("UNICODE(str)", "First character Unicode code"),
                ("CHAR(x1, x2, ...)", "Create string from codes"),
                ("HEX(x)", "Convert to hexadecimal"),
                ("ZEROBLOB(n)", "Create n zero bytes"),
                ("LAST_INSERT_ROWID()", "Last inserted rowid"),
                ("CHANGES()", "Rows changed by last statement"),
                ("TOTAL_CHANGES()", "Total rows changed"),
                ("RANDOM()", "Random 64-bit integer"),
                ("ABS(x)", "Absolute value"),
                ("DATE(time, ...)", "Extract date"),
                ("TIME(time, ...)", "Extract time"),
                ("DATETIME(time, ...)", "Date and time"),
                ("JULIANDAY(time)", "Julian day number"),
                ("STRFTIME(fmt, time)", "Format date/time"),
                ("JSON(json)", "Parse JSON"),
                ("JSON_ARRAY(...)", "Create JSON array"),
                ("JSON_OBJECT(...)", "Create JSON object"),
                ("JSON_EXTRACT(json, path)", "Extract JSON value"),
                ("JSON_TYPE(json, path)", "Get JSON type"),
                ("GROUP_CONCAT(x, sep)", "Concatenate group values"),
            ],
            operators: vec![
                ("||", "String concatenation"),
                ("->", "JSON extract (value)"),
                ("->>", "JSON extract (text)"),
                ("GLOB", "Unix pattern match"),
                ("REGEXP", "Regular expression (if loaded)"),
            ],
            data_types: SQLITE_DATA_TYPES.to_vec(),
            snippets: vec![
                (
                    "crt",
                    "CREATE TABLE $1 (\n  id INTEGER PRIMARY KEY AUTOINCREMENT,\n  $2\n)",
                    "Create table",
                ),
                ("idx", "CREATE INDEX $1 ON $2 ($3)", "Create index"),
                (
                    "uidx",
                    "CREATE UNIQUE INDEX $1 ON $2 ($3)",
                    "Create unique index",
                ),
                ("vac", "VACUUM", "Vacuum database"),
                ("pragma", "PRAGMA $1", "Pragma statement"),
            ],
        }
        .with_standard_sql()
    }

    async fn create_connection(
        &self,
        config: DbConnectionConfig,
    ) -> Result<Box<dyn DbConnection + Send + Sync>, DbError> {
        let mut conn = SqliteDbConnection::new(config);
        conn.connect().await?;
        Ok(Box::new(conn))
    }

    async fn list_databases(&self, _connection: &dyn DbConnection) -> Result<Vec<String>> {
        Ok(vec!["main".to_string()])
    }

    async fn list_databases_view(&self, _connection: &dyn DbConnection) -> Result<ObjectView> {
        use gpui::px;

        let columns = vec![Column::new("name", "Name").width(px(180.0))];

        let rows = vec![vec!["main".to_string()]];

        Ok(ObjectView {
            db_node_type: DbNodeType::Database,
            title: "1 database(s)".to_string(),
            columns,
            rows,
        })
    }

    async fn list_databases_detailed(
        &self,
        _connection: &dyn DbConnection,
    ) -> Result<Vec<DatabaseInfo>> {
        Ok(vec![DatabaseInfo {
            name: "main".to_string(),
            charset: None,
            collation: None,
            size: None,
            table_count: None,
            comment: None,
        }])
    }

    fn supports_rowid(&self) -> bool {
        true
    }

    fn rowid_column_name(&self) -> &'static str {
        "rowid"
    }

    fn sql_dialect(&self) -> Box<dyn sqlparser::dialect::Dialect> {
        Box::new(sqlparser::dialect::SQLiteDialect {})
    }

    async fn list_tables(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
    ) -> Result<Vec<TableInfo>> {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list tables: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| TableInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    schema: None,
                    comment: None,
                    engine: None,
                    row_count: None,
                    create_time: None,
                    charset: None,
                    collation: None,
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_tables_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        _schema: Option<String>,
    ) -> Result<ObjectView> {
        use gpui::px;

        let tables = self.list_tables(connection, database, None).await?;

        let columns = vec![Column::new("name", "Name").width(px(200.0))];

        let rows: Vec<Vec<String>> = tables
            .iter()
            .map(|table| vec![table.name.clone()])
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Table,
            title: format!("{} table(s)", tables.len()),
            columns,
            rows,
        })
    }

    async fn list_columns(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
        table: &str,
    ) -> Result<Vec<ColumnInfo>> {
        let sql = format!("PRAGMA table_info(\"{}\")", table);
        tracing::info!("SQLite list_columns: executing SQL: {}", sql);

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list columns: {}", e))?;

        tracing::info!(
            "SQLite list_columns: result type: {:?}",
            match &result {
                SqlResult::Query(_) => "Query",
                SqlResult::Exec(_) => "Exec",
                SqlResult::Error(e) => &e.message,
            }
        );

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| ColumnInfo {
                    name: row.get(1).and_then(|v| v.clone()).unwrap_or_default(),
                    data_type: row.get(2).and_then(|v| v.clone()).unwrap_or_default(),
                    is_nullable: row
                        .get(3)
                        .and_then(|v| v.clone())
                        .map(|v| v == "0")
                        .unwrap_or(true),
                    is_primary_key: row
                        .get(5)
                        .and_then(|v| v.clone())
                        .map(|v| v == "1")
                        .unwrap_or(false),
                    default_value: row.get(4).and_then(|v| v.clone()),
                    comment: None,
                    charset: None,
                    collation: None,
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_columns_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let columns_data = self
            .list_columns(connection, database, schema, table)
            .await?;

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("type", "Type").width(px(150.0)),
            Column::new("nullable", "Nullable").width(px(80.0)),
            Column::new("key", "Key").width(px(80.0)),
            Column::new("default", "Default").width(px(120.0)),
        ];

        let rows: Vec<Vec<String>> = columns_data
            .iter()
            .map(|col| {
                vec![
                    col.name.clone(),
                    col.data_type.clone(),
                    if col.is_nullable { "YES" } else { "NO" }.to_string(),
                    if col.is_primary_key { "PRI" } else { "" }.to_string(),
                    col.default_value.as_deref().unwrap_or("").to_string(),
                ]
            })
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Column,
            title: format!("{} column(s)", columns_data.len()),
            columns,
            rows,
        })
    }

    async fn list_indexes(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
        table: &str,
    ) -> Result<Vec<IndexInfo>> {
        let sql = format!("PRAGMA index_list(\"{}\")", table);

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list indexes: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let mut indexes = Vec::new();

            for row in query_result.rows {
                let origin = row.get(3).and_then(|v| v.clone()).unwrap_or_default();
                if origin == "pk" {
                    continue;
                }

                let index_name = row.get(1).and_then(|v| v.clone()).unwrap_or_default();
                let is_unique = row
                    .get(2)
                    .and_then(|v| v.clone())
                    .map(|v| v == "1")
                    .unwrap_or(false);

                let info_sql = format!("PRAGMA index_info(\"{}\")", index_name);
                let info_result = connection.query(&info_sql).await;

                let columns = if let Ok(SqlResult::Query(info_query)) = info_result {
                    info_query
                        .rows
                        .iter()
                        .filter_map(|r| r.get(2).and_then(|v| v.clone()))
                        .collect()
                } else {
                    Vec::new()
                };

                indexes.push(IndexInfo {
                    name: index_name,
                    columns,
                    is_unique,
                    index_type: None,
                });
            }

            Ok(indexes)
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_indexes_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let indexes = self
            .list_indexes(connection, database, schema.map(|s| s.to_string()), table)
            .await?;

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("columns", "Columns").width(px(250.0)),
            Column::new("unique", "Unique").width(px(80.0)),
        ];

        let rows: Vec<Vec<String>> = indexes
            .iter()
            .map(|idx| {
                vec![
                    idx.name.clone(),
                    idx.columns.join(", "),
                    if idx.is_unique { "YES" } else { "NO" }.to_string(),
                ]
            })
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Index,
            title: format!("{} index(es)", indexes.len()),
            columns,
            rows,
        })
    }

    async fn list_views(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
    ) -> Result<Vec<ViewInfo>> {
        let sql = "SELECT name, sql FROM sqlite_master WHERE type='view' ORDER BY name";

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list views: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| ViewInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    schema: None,
                    definition: row.get(1).and_then(|v| v.clone()),
                    comment: None,
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_views_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let views = self.list_views(connection, database, None).await?;

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("definition", "Definition").width(px(400.0)),
        ];

        let rows: Vec<Vec<String>> = views
            .iter()
            .map(|view| {
                vec![
                    view.name.clone(),
                    view.definition.as_deref().unwrap_or("").to_string(),
                ]
            })
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::View,
            title: format!("{} view(s)", views.len()),
            columns,
            rows,
        })
    }

    fn supports_functions(&self) -> bool {
        false
    }

    async fn list_functions(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        Ok(Vec::new())
    }

    async fn list_functions_view(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let columns = vec![Column::new("name", "Name").width(px(200.0))];

        Ok(ObjectView {
            db_node_type: DbNodeType::Function,
            title: "0 function(s)".to_string(),
            columns,
            rows: vec![],
        })
    }

    fn supports_procedures(&self) -> bool {
        false
    }

    async fn list_procedures(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        Ok(Vec::new())
    }

    async fn list_procedures_view(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let columns = vec![Column::new("name", "Name").width(px(200.0))];

        Ok(ObjectView {
            db_node_type: DbNodeType::Procedure,
            title: "0 procedure(s)".to_string(),
            columns,
            rows: vec![],
        })
    }

    async fn list_triggers(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<TriggerInfo>> {
        let sql =
            "SELECT name, tbl_name, sql FROM sqlite_master WHERE type='trigger' ORDER BY name";

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list triggers: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| TriggerInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    table_name: row.get(1).and_then(|v| v.clone()).unwrap_or_default(),
                    event: String::new(),
                    timing: String::new(),
                    definition: row.get(2).and_then(|v| v.clone()),
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_triggers_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let triggers = self.list_triggers(connection, database).await?;

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("table", "Table").width(px(150.0)),
        ];

        let rows: Vec<Vec<String>> = triggers
            .iter()
            .map(|trigger| vec![trigger.name.clone(), trigger.table_name.clone()])
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Trigger,
            title: format!("{} trigger(s)", triggers.len()),
            columns,
            rows,
        })
    }

    async fn list_sequences(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
    ) -> Result<Vec<SequenceInfo>> {
        Ok(Vec::new())
    }

    async fn list_sequences_view(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let columns = vec![Column::new("name", "Name").width(px(200.0))];

        Ok(ObjectView {
            db_node_type: DbNodeType::Sequence,
            title: "0 sequence(s)".to_string(),
            columns,
            rows: vec![],
        })
    }

    fn build_column_definition(&self, column: &ColumnInfo, include_name: bool) -> String {
        let mut def = String::new();

        if include_name {
            def.push_str(&self.quote_identifier(&column.name));
            def.push(' ');
        }

        def.push_str(&column.data_type);

        if !column.is_nullable {
            def.push_str(" NOT NULL");
        }

        if let Some(default) = &column.default_value {
            def.push_str(&format!(" DEFAULT {}", default));
        }

        if column.is_primary_key {
            def.push_str(" PRIMARY KEY");
        }

        def
    }

    fn build_create_database_sql(
        &self,
        _request: &crate::plugin::DatabaseOperationRequest,
    ) -> String {
        "-- SQLite: database is created when opening a file".to_string()
    }

    fn build_modify_database_sql(
        &self,
        _request: &crate::plugin::DatabaseOperationRequest,
    ) -> String {
        "-- SQLite: database modification not supported".to_string()
    }

    fn build_drop_database_sql(&self, _database_name: &str) -> String {
        "-- SQLite: delete the database file to drop the database".to_string()
    }

    fn format_table_reference(
        &self,
        _database: &str,
        _schema: Option<&str>,
        table: &str,
    ) -> String {
        self.quote_identifier(table)
    }

    fn build_limit_clause(&self) -> String {
        String::new()
    }

    fn build_where_and_limit_clause(
        &self,
        request: &TableSaveRequest,
        original_data: &[String],
    ) -> (String, String) {
        let where_clause = self.build_table_change_where_clause(request, original_data);
        let has_primary_key = request.columns.iter().any(|c| c.is_primary_key);
        let has_unique_key = has_primary_key || request.index_infos.iter().any(|idx| idx.is_unique);

        if has_unique_key {
            (where_clause, String::new())
        } else {
            (where_clause, " __SQLITE_ROWID_LIMIT__".to_string())
        }
    }

    async fn export_table_create_sql(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<&str>,
        table: &str,
    ) -> Result<String> {
        let query = format!(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'",
            table.replace('\'', "''")
        );
        let result = connection
            .query(&query)
            .await
            .map_err(|e| anyhow::anyhow!("Query failed: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            if let Some(row) = query_result.rows.first() {
                if let Some(Some(create_sql)) = row.first() {
                    return Ok(create_sql.clone());
                }
            }
        }
        Ok(String::new())
    }

    fn get_data_types(&self) -> &[(&'static str, &'static str)] {
        SQLITE_DATA_TYPES
    }

    fn drop_table(&self, _database: &str, _schema: Option<&str>, table: &str) -> String {
        format!("DROP TABLE IF EXISTS {}", self.quote_identifier(table))
    }

    fn truncate_table(&self, _database: &str, table: &str) -> String {
        format!("DELETE FROM {}", self.quote_identifier(table))
    }

    fn rename_table(&self, _database: &str, old_name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            self.quote_identifier(old_name),
            self.quote_identifier(new_name)
        )
    }

    fn drop_view(&self, _database: &str, view: &str) -> String {
        format!("DROP VIEW IF EXISTS {}", self.quote_identifier(view))
    }

    fn build_column_def(&self, col: &ColumnDefinition) -> String {
        let mut def = String::new();
        def.push_str(&self.quote_identifier(&col.name));
        def.push(' ');

        let type_str = self.build_type_string(col);
        def.push_str(&type_str);

        if col.is_primary_key && col.is_auto_increment {
            def.push_str(" PRIMARY KEY AUTOINCREMENT");
        } else {
            if !col.is_nullable {
                def.push_str(" NOT NULL");
            }

            if let Some(default) = &col.default_value {
                if !default.is_empty() {
                    def.push_str(&format!(" DEFAULT {}", default));
                }
            }
        }

        def
    }

    fn build_create_table_sql(&self, design: &TableDesign) -> String {
        let mut sql = String::new();
        sql.push_str("CREATE TABLE ");
        sql.push_str(&self.quote_identifier(&design.table_name));
        sql.push_str(" (\n");

        let mut definitions: Vec<String> = Vec::new();

        for col in &design.columns {
            definitions.push(format!("  {}", self.build_column_def(col)));
        }

        let pk_columns: Vec<&str> = design
            .columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| c.name.as_str())
            .collect();
        if !pk_columns.is_empty() {
            let pk_cols: Vec<String> = pk_columns
                .iter()
                .map(|c| self.quote_identifier(c))
                .collect();
            definitions.push(format!("  PRIMARY KEY ({})", pk_cols.join(", ")));
        }

        sql.push_str(&definitions.join(",\n"));
        sql.push_str("\n);");

        for idx in &design.indexes {
            if idx.is_primary {
                continue;
            }
            let idx_cols: Vec<String> = idx
                .columns
                .iter()
                .map(|c| self.quote_identifier(c))
                .collect();
            let unique_str = if idx.is_unique { "UNIQUE " } else { "" };
            sql.push_str(&format!(
                "\nCREATE {}INDEX {} ON {} ({});",
                unique_str,
                self.quote_identifier(&idx.name),
                self.quote_identifier(&design.table_name),
                idx_cols.join(", ")
            ));
        }

        sql
    }

    fn build_alter_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        let original_cols: HashMap<&str, &ColumnDefinition> = original
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let new_cols: HashMap<&str, &ColumnDefinition> =
            new.columns.iter().map(|c| (c.name.as_str(), c)).collect();

        let mut dropped_columns: Vec<&str> = Vec::new();
        let mut modified_columns: Vec<&str> = Vec::new();

        for (name, orig_col) in &original_cols {
            if !new_cols.contains_key(name) {
                dropped_columns.push(name);
            } else if let Some(new_col) = new_cols.get(name) {
                if self.column_changed(orig_col, new_col) {
                    modified_columns.push(name);
                }
            }
        }

        let has_structure_change = !dropped_columns.is_empty() || !modified_columns.is_empty();

        if has_structure_change {
            self.build_sqlite_recreate_table_sql(original, new)
        } else {
            self.build_sqlite_simple_alter_sql(original, new)
        }
    }

    async fn import_data_with_progress(
        &self,
        connection: &dyn DbConnection,
        config: &ImportConfig,
        data: &str,
        file_name: &str,
        progress_tx: Option<ImportProgressSender>,
    ) -> Result<ImportResult> {
        crate::plugin::default_import_data_with_progress(
            self,
            connection,
            config,
            data,
            file_name,
            progress_tx,
        )
        .await
    }

    async fn export_data_with_progress(
        &self,
        connection: &dyn DbConnection,
        config: &ExportConfig,
        progress_tx: Option<ExportProgressSender>,
    ) -> Result<ExportResult> {
        crate::plugin::default_export_data_with_progress(self, connection, config, progress_tx)
            .await
    }
}

impl Default for SqlitePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::DatabasePlugin;
    use crate::types::{ColumnDefinition, IndexDefinition, TableDesign, TableOptions};

    fn create_plugin() -> SqlitePlugin {
        SqlitePlugin::new()
    }

    // ==================== Basic Plugin Info Tests ====================

    #[test]
    fn test_plugin_name() {
        let plugin = create_plugin();
        assert_eq!(plugin.name(), DatabaseType::SQLite);
    }

    #[test]
    fn test_quote_identifier() {
        let plugin = create_plugin();
        assert_eq!(plugin.quote_identifier("table_name"), "\"table_name\"");
        assert_eq!(plugin.quote_identifier("column"), "\"column\"");
        assert_eq!(plugin.quote_identifier("col\"umn"), "\"col\"\"umn\"");
    }

    // ==================== DDL SQL Generation Tests ====================

    #[test]
    fn test_drop_table() {
        let plugin = create_plugin();
        let sql = plugin.drop_table("main", None, "users");
        assert!(sql.contains("DROP TABLE"));
        assert!(sql.contains("\"users\""));
    }

    #[test]
    fn test_truncate_table() {
        let plugin = create_plugin();
        let sql = plugin.truncate_table("main", "users");
        assert!(sql.contains("DELETE FROM"));
        assert!(sql.contains("\"users\""));
    }

    #[test]
    fn test_rename_table() {
        let plugin = create_plugin();
        let sql = plugin.rename_table("main", "old_name", "new_name");
        assert!(sql.contains("ALTER TABLE"));
        assert!(sql.contains("RENAME TO"));
        assert!(sql.contains("\"old_name\""));
        assert!(sql.contains("\"new_name\""));
    }

    #[test]
    fn test_drop_view() {
        let plugin = create_plugin();
        let sql = plugin.drop_view("main", "my_view");
        assert!(sql.contains("DROP VIEW"));
        assert!(sql.contains("\"my_view\""));
    }

    #[test]
    fn test_drop_table_escapes_identifier() {
        let plugin = create_plugin();
        let sql = plugin.drop_table("main", None, "weird\"table");
        assert_eq!(sql, "DROP TABLE IF EXISTS \"weird\"\"table\"");
    }

    #[test]
    fn test_drop_view_escapes_identifier() {
        let plugin = create_plugin();
        let sql = plugin.drop_view("main", "my\"view");
        assert_eq!(sql, "DROP VIEW IF EXISTS \"my\"\"view\"");
    }

    // ==================== Database Operations Tests ====================

    #[test]
    fn test_build_create_database_sql() {
        let plugin = create_plugin();
        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "test.db".to_string(),
            field_values: HashMap::new(),
        };

        let sql = plugin.build_create_database_sql(&request);
        assert!(sql.contains("--"));
    }

    #[test]
    fn test_build_drop_database_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_database_sql("test.db");
        assert!(sql.contains("--"));
    }

    // ==================== Column Definition Tests ====================

    #[test]
    fn test_build_column_def_simple() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("id")
            .data_type("INTEGER")
            .nullable(false)
            .primary_key(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("\"id\""));
        assert!(def.contains("INTEGER"));
        assert!(def.contains("NOT NULL"));
    }

    #[test]
    fn test_build_column_def_text() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("name")
            .data_type("TEXT")
            .nullable(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("\"name\""));
        assert!(def.contains("TEXT"));
        assert!(!def.contains("NOT NULL"));
    }

    #[test]
    fn test_build_column_def_with_default() {
        let plugin = create_plugin();
        let mut col = ColumnDefinition::new("status")
            .data_type("INTEGER")
            .default_value("0");
        col.is_nullable = false;

        let def = plugin.build_column_def(&col);
        assert!(def.contains("DEFAULT 0"));
        assert!(def.contains("NOT NULL"));
    }

    // ==================== CREATE TABLE Tests ====================

    #[test]
    fn test_build_create_table_sql_simple() {
        let plugin = create_plugin();
        let design = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INTEGER")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("name").data_type("TEXT"),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_create_table_sql(&design);
        assert!(sql.contains("CREATE TABLE \"users\""));
        assert!(sql.contains("\"id\""));
        assert!(sql.contains("INTEGER"));
        assert!(sql.contains("\"name\""));
        assert!(sql.contains("TEXT"));
        assert!(sql.contains("PRIMARY KEY"));
    }

    #[test]
    fn test_build_create_table_sql_with_indexes() {
        let plugin = create_plugin();
        let design = TableDesign {
            database_name: "main".to_string(),
            table_name: "orders".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INTEGER")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("user_id")
                    .data_type("INTEGER")
                    .nullable(false),
                ColumnDefinition::new("email").data_type("TEXT"),
            ],
            indexes: vec![
                IndexDefinition::new("idx_user_id")
                    .columns(vec!["user_id".to_string()])
                    .unique(false),
                IndexDefinition::new("idx_email")
                    .columns(vec!["email".to_string()])
                    .unique(true),
            ],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_create_table_sql(&design);
        assert!(sql.contains("INDEX \"idx_user_id\""));
        assert!(sql.contains("UNIQUE INDEX \"idx_email\""));
    }

    // ==================== ALTER TABLE Tests ====================

    #[test]
    fn test_build_alter_table_sql_add_column() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id").data_type("INTEGER")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INTEGER"),
                ColumnDefinition::new("email").data_type("TEXT"),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("ADD COLUMN"));
        assert!(sql.contains("\"email\""));
    }

    #[test]
    fn test_build_alter_table_sql_drop_column() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INTEGER"),
                ColumnDefinition::new("old_column").data_type("TEXT"),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id").data_type("INTEGER")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("create table"));
        assert!(sql.contains("_dg_tmp"));
        assert!(sql.contains("drop table"));
        assert!(sql.contains("rename to"));
    }

    // ==================== Data Types Tests ====================

    #[test]
    fn test_get_data_types() {
        let plugin = create_plugin();
        let types = plugin.get_data_types();

        assert!(!types.is_empty());
        assert!(types.iter().any(|t| t.0 == "INTEGER"));
        assert!(types.iter().any(|t| t.0 == "TEXT"));
        assert!(types.iter().any(|t| t.0 == "REAL"));
        assert!(types.iter().any(|t| t.0 == "BLOB"));
    }

    // ==================== Completion Info Tests ====================

    #[test]
    fn test_get_completion_info() {
        let plugin = create_plugin();
        let info = plugin.get_completion_info();

        assert!(!info.keywords.is_empty());
        assert!(!info.functions.is_empty());
        assert!(!info.data_types.is_empty());
        assert!(!info.snippets.is_empty());
    }
}
