use anyhow::{anyhow, Result};
use async_trait::async_trait;
use gpui_component::table::Column;
use one_core::storage::{DatabaseType, DbConnectionConfig};
use std::collections::{HashMap, HashSet};

use crate::connection::{DbConnection, DbError};
use crate::duckdb::DuckDbConnection;
use crate::executor::SqlResult;
use crate::import_export::{
    ExportConfig, ExportProgressSender, ExportResult, ImportConfig, ImportProgressSender,
    ImportResult,
};
use crate::plugin::{DatabaseOperationRequest, DatabasePlugin, SqlCompletionInfo};
use crate::sqlite::SqlitePlugin;
use crate::types::*;

pub const DUCKDB_DATA_TYPES: &[(&str, &str)] = &[
    ("BIGINT", "Signed 64-bit integer"),
    ("INTEGER", "Signed 32-bit integer"),
    ("SMALLINT", "Signed 16-bit integer"),
    ("TINYINT", "Signed 8-bit integer"),
    ("UBIGINT", "Unsigned 64-bit integer"),
    ("UINTEGER", "Unsigned 32-bit integer"),
    ("USMALLINT", "Unsigned 16-bit integer"),
    ("UTINYINT", "Unsigned 8-bit integer"),
    ("HUGEINT", "Signed 128-bit integer"),
    ("DECIMAL", "Fixed-point decimal"),
    ("DOUBLE", "Double precision floating point"),
    ("REAL", "Single precision floating point"),
    ("BOOLEAN", "Boolean value"),
    ("VARCHAR", "Variable-length string"),
    ("TEXT", "Text string"),
    ("BLOB", "Binary large object"),
    ("DATE", "Calendar date"),
    ("TIME", "Time of day"),
    ("TIMESTAMP", "Timestamp without timezone"),
    ("TIMESTAMPTZ", "Timestamp with timezone"),
    ("JSON", "JSON document"),
    ("UUID", "Universally unique identifier"),
];

pub struct DuckDbPlugin {
    sqlite: SqlitePlugin,
}

impl DuckDbPlugin {
    pub fn new() -> Self {
        Self {
            sqlite: SqlitePlugin::new(),
        }
    }

    fn escape_sql_literal(value: &str) -> String {
        value.replace('\'', "''")
    }

    fn query_filter(
        database: &str,
        schema: Option<&str>,
        object_column: &str,
        object_name: &str,
    ) -> String {
        let mut filters = vec![format!(
            "{} = '{}'",
            object_column,
            Self::escape_sql_literal(object_name)
        )];

        if !database.is_empty() && database != "main" {
            filters.push(format!(
                "database_name = '{}'",
                Self::escape_sql_literal(database)
            ));
        }

        if let Some(schema_name) =
            schema.filter(|schema_name| Self::should_filter_schema(schema_name))
        {
            filters.push(format!(
                "schema_name = '{}'",
                Self::escape_sql_literal(schema_name)
            ));
        }

        filters.join(" AND ")
    }

    fn normalize_column_definition(&self, column: &ColumnDefinition) -> ColumnDefinition {
        let mut normalized = column.clone();
        normalized.is_auto_increment = false;
        normalized
    }

    fn normalize_design(&self, design: &TableDesign) -> TableDesign {
        let mut normalized = design.clone();
        normalized.columns = design
            .columns
            .iter()
            .map(|column| self.normalize_column_definition(column))
            .collect();
        normalized
    }

    fn build_type_string(&self, column: &ColumnDefinition) -> String {
        if let Some(length) = column.length {
            if let Some(scale) = column.scale {
                format!("{}({},{})", column.data_type, length, scale)
            } else {
                format!("{}({})", column.data_type, length)
            }
        } else if let Some(precision) = column.precision {
            if let Some(scale) = column.scale {
                format!("{}({},{})", column.data_type, precision, scale)
            } else {
                format!("{}({})", column.data_type, precision)
            }
        } else {
            column.data_type.clone()
        }
    }

    fn primary_key_columns(design: &TableDesign) -> Vec<&str> {
        design
            .columns
            .iter()
            .filter(|column| column.is_primary_key)
            .map(|column| column.name.as_str())
            .collect()
    }

    fn primary_key_changed(original: &TableDesign, new: &TableDesign) -> bool {
        Self::primary_key_columns(original) != Self::primary_key_columns(new)
    }

    fn column_changed_for_alter(original: &ColumnDefinition, new: &ColumnDefinition) -> bool {
        original.data_type.to_uppercase() != new.data_type.to_uppercase()
            || original.length != new.length
            || original.precision != new.precision
            || original.scale != new.scale
            || original.is_nullable != new.is_nullable
            || original.default_value != new.default_value
    }

    fn index_changed(original: &IndexDefinition, new: &IndexDefinition) -> bool {
        original.columns != new.columns
            || original.is_unique != new.is_unique
            || original.index_type != new.index_type
    }

    fn build_index_sql(&self, table_name: &str, index: &IndexDefinition) -> String {
        let columns: Vec<String> = index
            .columns
            .iter()
            .map(|column| self.quote_identifier(column))
            .collect();
        let unique = if index.is_unique { "UNIQUE " } else { "" };

        format!(
            "CREATE {}INDEX {} ON {} ({});",
            unique,
            self.quote_identifier(&index.name),
            self.quote_identifier(table_name),
            columns.join(", ")
        )
    }

    fn build_recreate_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        let mut statements = Vec::new();
        let temp_table_name = format!("{}_duckdb_tmp", new.table_name);

        let mut temp_design = self.normalize_design(new);
        temp_design.table_name = temp_table_name.clone();
        temp_design.indexes = vec![];

        statements.push(self.sqlite.build_create_table_sql(&temp_design));

        let original_column_names: HashSet<&str> = original
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect();
        let common_columns: Vec<String> = new
            .columns
            .iter()
            .filter(|column| original_column_names.contains(column.name.as_str()))
            .map(|column| self.quote_identifier(&column.name))
            .collect();

        if !common_columns.is_empty() {
            let column_list = common_columns.join(", ");
            statements.push(format!(
                "INSERT INTO {} ({}) SELECT {} FROM {};",
                self.quote_identifier(&temp_table_name),
                column_list,
                column_list,
                self.quote_identifier(&original.table_name)
            ));
        }

        statements.push(format!(
            "DROP TABLE {};",
            self.quote_identifier(&original.table_name)
        ));
        statements.push(format!(
            "ALTER TABLE {} RENAME TO {};",
            self.quote_identifier(&temp_table_name),
            self.quote_identifier(&new.table_name)
        ));

        for index in &new.indexes {
            if index.is_primary {
                continue;
            }
            statements.push(self.build_index_sql(&new.table_name, index));
        }

        statements.join("\n")
    }

    fn build_native_alter_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        let table_name = self.quote_identifier(&new.table_name);
        let original_columns: HashMap<&str, &ColumnDefinition> = original
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column))
            .collect();
        let new_columns: HashMap<&str, &ColumnDefinition> = new
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column))
            .collect();

        let dropped_columns: Vec<&str> = original_columns
            .keys()
            .filter(|name| !new_columns.contains_key(*name))
            .copied()
            .collect();

        let added_columns: Vec<&ColumnDefinition> = new
            .columns
            .iter()
            .filter(|column| !original_columns.contains_key(column.name.as_str()))
            .collect();

        let modified_columns: Vec<(&ColumnDefinition, &ColumnDefinition)> = new
            .columns
            .iter()
            .filter_map(|new_column| {
                original_columns
                    .get(new_column.name.as_str())
                    .map(|original_column| (*original_column, new_column))
            })
            .filter(|(original_column, new_column)| {
                Self::column_changed_for_alter(original_column, new_column)
            })
            .collect();

        let affected_columns: HashSet<&str> = dropped_columns
            .iter()
            .copied()
            .chain(
                modified_columns
                    .iter()
                    .map(|(original_column, _)| original_column.name.as_str()),
            )
            .collect();

        let original_indexes: HashMap<&str, &IndexDefinition> = original
            .indexes
            .iter()
            .map(|index| (index.name.as_str(), index))
            .collect();
        let new_indexes: HashMap<&str, &IndexDefinition> = new
            .indexes
            .iter()
            .map(|index| (index.name.as_str(), index))
            .collect();

        let indexes_to_drop: Vec<&IndexDefinition> = original
            .indexes
            .iter()
            .filter(|index| {
                affected_columns.iter().any(|column| {
                    index
                        .columns
                        .iter()
                        .any(|index_column| index_column == column)
                }) || !new_indexes.contains_key(index.name.as_str())
                    || new_indexes
                        .get(index.name.as_str())
                        .is_some_and(|new_index| Self::index_changed(index, new_index))
            })
            .collect();

        let indexes_to_create: Vec<&IndexDefinition> = new
            .indexes
            .iter()
            .filter(|index| {
                affected_columns.iter().any(|column| {
                    index
                        .columns
                        .iter()
                        .any(|index_column| index_column == column)
                }) || !original_indexes.contains_key(index.name.as_str())
                    || original_indexes
                        .get(index.name.as_str())
                        .is_some_and(|original_index| Self::index_changed(original_index, index))
            })
            .collect();

        let mut statements = Vec::new();

        for index in indexes_to_drop {
            statements.push(format!(
                "DROP INDEX IF EXISTS {};",
                self.quote_identifier(&index.name)
            ));
        }

        for column_name in dropped_columns {
            statements.push(format!(
                "ALTER TABLE {} DROP COLUMN {};",
                table_name,
                self.quote_identifier(column_name)
            ));
        }

        for (original_column, new_column) in modified_columns {
            let quoted_column = self.quote_identifier(&new_column.name);
            if original_column.data_type.to_uppercase() != new_column.data_type.to_uppercase()
                || original_column.length != new_column.length
                || original_column.precision != new_column.precision
                || original_column.scale != new_column.scale
            {
                statements.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET DATA TYPE {};",
                    table_name,
                    quoted_column,
                    self.build_type_string(new_column)
                ));
            }

            if original_column.is_nullable != new_column.is_nullable {
                let nullable_sql = if new_column.is_nullable {
                    "DROP NOT NULL".to_string()
                } else {
                    "SET NOT NULL".to_string()
                };
                statements.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} {};",
                    table_name, quoted_column, nullable_sql
                ));
            }

            if original_column.default_value != new_column.default_value {
                match &new_column.default_value {
                    Some(default_value) if !default_value.is_empty() => statements.push(format!(
                        "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};",
                        table_name, quoted_column, default_value
                    )),
                    _ => statements.push(format!(
                        "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;",
                        table_name, quoted_column
                    )),
                }
            }
        }

        for column in added_columns {
            statements.push(format!(
                "ALTER TABLE {} ADD COLUMN {};",
                table_name,
                self.build_column_def(column)
            ));
        }

        for index in indexes_to_create {
            if index.is_primary {
                continue;
            }
            statements.push(self.build_index_sql(&new.table_name, index));
        }

        if statements.is_empty() {
            "-- No changes detected".to_string()
        } else {
            statements.join("\n")
        }
    }

    async fn query_rows(
        &self,
        connection: &dyn DbConnection,
        sql: &str,
        context: &str,
    ) -> Result<Vec<Vec<Option<String>>>> {
        match connection
            .query(sql)
            .await
            .map_err(|error| anyhow!("{}: {}", context, error))?
        {
            SqlResult::Query(query_result) => Ok(query_result.rows),
            _ => Err(anyhow!("Unexpected result type")),
        }
    }

    fn parse_bool(value: Option<&str>) -> bool {
        matches!(
            value.map(|text| text.to_ascii_lowercase()),
            Some(value) if matches!(value.as_str(), "true" | "t" | "1" | "yes" | "y")
        )
    }

    fn should_filter_schema(schema_name: &str) -> bool {
        !schema_name.is_empty() && schema_name != "main"
    }
}

impl Default for DuckDbPlugin {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_index_columns_from_sql(sql: &str) -> Vec<String> {
    let Some(open_idx) = sql.rfind('(') else {
        return Vec::new();
    };
    let Some(close_idx) = sql.rfind(')') else {
        return Vec::new();
    };
    if close_idx <= open_idx {
        return Vec::new();
    }

    sql[open_idx + 1..close_idx]
        .split(',')
        .map(|part| part.trim().trim_matches('"').to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

#[async_trait]
impl DatabasePlugin for DuckDbPlugin {
    fn name(&self) -> DatabaseType {
        DatabaseType::DuckDB
    }

    fn quote_identifier(&self, identifier: &str) -> String {
        format!("\"{}\"", identifier.replace('"', "\"\""))
    }

    fn get_completion_info(&self) -> SqlCompletionInfo {
        let mut info = self.sqlite.get_completion_info();
        info.keywords.extend([
            ("COPY", "Copy query results to a file"),
            ("INSTALL", "Install an extension"),
            ("LOAD", "Load an installed extension"),
        ]);
        info.data_types = DUCKDB_DATA_TYPES.to_vec();
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

    async fn list_databases(&self, _connection: &dyn DbConnection) -> Result<Vec<String>> {
        Ok(vec!["main".to_string()])
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

    fn supports_rowid(&self) -> bool {
        true
    }

    fn rowid_column_name(&self) -> &'static str {
        "rowid"
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
        let mut filters = vec![
            "internal = FALSE".to_string(),
            "temporary = FALSE".to_string(),
        ];

        if !database.is_empty() && database != "main" {
            filters.push(format!(
                "database_name = '{}'",
                Self::escape_sql_literal(database)
            ));
        }

        if let Some(schema_name) = schema
            .as_deref()
            .filter(|schema_name| Self::should_filter_schema(schema_name))
        {
            filters.push(format!(
                "schema_name = '{}'",
                Self::escape_sql_literal(schema_name)
            ));
        }

        let sql = format!(
            "SELECT table_name, schema_name FROM duckdb_tables() WHERE {} ORDER BY schema_name, table_name",
            filters.join(" AND ")
        );
        let rows = self
            .query_rows(connection, &sql, "Failed to list tables")
            .await?;

        Ok(rows
            .iter()
            .map(|row| TableInfo {
                name: row
                    .first()
                    .and_then(|value| value.clone())
                    .unwrap_or_default(),
                schema: row.get(1).and_then(|value| value.clone()),
                comment: None,
                engine: None,
                row_count: None,
                create_time: None,
                charset: None,
                collation: None,
            })
            .collect())
    }

    async fn list_tables_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<ObjectView> {
        use gpui::px;

        let tables = self.list_tables(connection, database, schema).await?;

        let columns = vec![
            Column::new("schema", "Schema").width(px(160.0)),
            Column::new("name", "Name").width(px(220.0)),
        ];

        let rows: Vec<Vec<String>> = tables
            .iter()
            .map(|table| vec![table.schema.clone().unwrap_or_default(), table.name.clone()])
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
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<ColumnInfo>> {
        let mut filters = vec![
            format!("c.table_name = '{}'", Self::escape_sql_literal(table)),
            "c.internal = FALSE".to_string(),
        ];

        if !database.is_empty() && database != "main" {
            filters.push(format!(
                "c.database_name = '{}'",
                Self::escape_sql_literal(database)
            ));
        }

        if let Some(schema_name) = schema
            .as_deref()
            .filter(|schema_name| Self::should_filter_schema(schema_name))
        {
            filters.push(format!(
                "c.schema_name = '{}'",
                Self::escape_sql_literal(schema_name)
            ));
        }

        let sql = format!(
            "SELECT \
                c.column_name, \
                c.data_type, \
                c.is_nullable, \
                (pk.column_name IS NOT NULL) AS is_primary_key, \
                c.column_default \
             FROM duckdb_columns() AS c \
             LEFT JOIN ( \
                SELECT DISTINCT \
                    kcu.table_schema, \
                    kcu.table_name, \
                    kcu.column_name \
                FROM information_schema.table_constraints AS tc \
                JOIN information_schema.key_column_usage AS kcu \
                  ON tc.constraint_schema = kcu.constraint_schema \
                 AND tc.constraint_name = kcu.constraint_name \
                 AND tc.table_schema = kcu.table_schema \
                 AND tc.table_name = kcu.table_name \
                WHERE tc.constraint_type = 'PRIMARY KEY' \
             ) AS pk \
               ON pk.table_schema = c.schema_name \
              AND pk.table_name = c.table_name \
              AND pk.column_name = c.column_name \
             WHERE {} \
             ORDER BY c.column_index",
            filters.join(" AND ")
        );
        let rows = self
            .query_rows(connection, &sql, "Failed to list columns")
            .await?;

        Ok(rows
            .iter()
            .map(|row| ColumnInfo {
                name: row
                    .first()
                    .and_then(|value| value.clone())
                    .unwrap_or_default(),
                data_type: row
                    .get(1)
                    .and_then(|value| value.clone())
                    .unwrap_or_default(),
                is_nullable: Self::parse_bool(row.get(2).and_then(|value| value.as_deref())),
                is_primary_key: Self::parse_bool(row.get(3).and_then(|value| value.as_deref())),
                default_value: row.get(4).and_then(|value| value.clone()),
                comment: None,
                charset: None,
                collation: None,
            })
            .collect())
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
            Column::new("type", "Type").width(px(160.0)),
            Column::new("nullable", "Nullable").width(px(90.0)),
            Column::new("primary", "Primary").width(px(90.0)),
            Column::new("default", "Default").width(px(200.0)),
        ];

        let rows: Vec<Vec<String>> = columns_data
            .iter()
            .map(|column| {
                vec![
                    column.name.clone(),
                    column.data_type.clone(),
                    if column.is_nullable { "YES" } else { "NO" }.to_string(),
                    if column.is_primary_key { "YES" } else { "NO" }.to_string(),
                    column.default_value.clone().unwrap_or_default(),
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
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<IndexInfo>> {
        let sql = match schema {
            Some(schema_name) if Self::should_filter_schema(&schema_name) => format!(
                "SELECT index_name, is_unique, sql FROM duckdb_indexes() WHERE schema_name = '{}' AND table_name = '{}' ORDER BY index_name",
                Self::escape_sql_literal(&schema_name),
                Self::escape_sql_literal(table)
            ),
            None => format!(
                "SELECT index_name, is_unique, sql FROM duckdb_indexes() WHERE table_name = '{}' ORDER BY index_name",
                Self::escape_sql_literal(table)
            ),
            Some(_) => format!(
                "SELECT index_name, is_unique, sql FROM duckdb_indexes() WHERE table_name = '{}' ORDER BY index_name",
                Self::escape_sql_literal(table)
            ),
        };

        let rows = self
            .query_rows(connection, &sql, "Failed to list indexes")
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let index_name = row.first().cloned().flatten().unwrap_or_default();
                let is_unique = row
                    .get(1)
                    .and_then(|value| value.as_deref())
                    .is_some_and(|value| Self::parse_bool(Some(value)));
                let columns = row
                    .get(2)
                    .and_then(|value| value.clone())
                    .map(|sql| parse_index_columns_from_sql(&sql))
                    .unwrap_or_default();

                IndexInfo {
                    name: index_name,
                    columns,
                    is_unique,
                    index_type: None,
                }
            })
            .collect())
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
            .list_indexes(connection, database, schema.map(str::to_string), table)
            .await?;

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("columns", "Columns").width(px(250.0)),
            Column::new("unique", "Unique").width(px(80.0)),
        ];

        let rows: Vec<Vec<String>> = indexes
            .iter()
            .map(|index| {
                vec![
                    index.name.clone(),
                    index.columns.join(", "),
                    if index.is_unique { "YES" } else { "NO" }.to_string(),
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
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<ViewInfo>> {
        let mut filters = vec![
            "internal = FALSE".to_string(),
            "temporary = FALSE".to_string(),
        ];

        if !database.is_empty() && database != "main" {
            filters.push(format!(
                "database_name = '{}'",
                Self::escape_sql_literal(database)
            ));
        }

        if let Some(schema_name) = schema
            .as_deref()
            .filter(|schema_name| Self::should_filter_schema(schema_name))
        {
            filters.push(format!(
                "schema_name = '{}'",
                Self::escape_sql_literal(schema_name)
            ));
        }

        let sql = format!(
            "SELECT view_name, schema_name, sql FROM duckdb_views() WHERE {} ORDER BY schema_name, view_name",
            filters.join(" AND ")
        );
        let rows = self
            .query_rows(connection, &sql, "Failed to list views")
            .await?;

        Ok(rows
            .iter()
            .map(|row| ViewInfo {
                name: row
                    .first()
                    .and_then(|value| value.clone())
                    .unwrap_or_default(),
                schema: row.get(1).and_then(|value| value.clone()),
                definition: row.get(2).and_then(|value| value.clone()),
                comment: None,
            })
            .collect())
    }

    async fn list_views_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let views = self.list_views(connection, database, None).await?;

        let columns = vec![
            Column::new("schema", "Schema").width(px(160.0)),
            Column::new("name", "Name").width(px(180.0)),
            Column::new("definition", "Definition").width(px(320.0)),
        ];

        let rows: Vec<Vec<String>> = views
            .iter()
            .map(|view| {
                vec![
                    view.schema.clone().unwrap_or_default(),
                    view.name.clone(),
                    view.definition.clone().unwrap_or_default(),
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

    fn format_table_reference(&self, _database: &str, schema: Option<&str>, table: &str) -> String {
        match schema {
            Some(schema_name) => format!(
                "{}.{}",
                self.quote_identifier(schema_name),
                self.quote_identifier(table)
            ),
            None => self.quote_identifier(table),
        }
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
        let table_sql_query = format!(
            "SELECT sql FROM duckdb_tables() WHERE {}",
            Self::query_filter(database, schema, "table_name", table)
        );
        let table_rows = self
            .query_rows(connection, &table_sql_query, "Failed to export table DDL")
            .await?;
        let mut statements = table_rows
            .into_iter()
            .filter_map(|row| row.first().cloned().flatten())
            .filter(|sql| !sql.is_empty())
            .collect::<Vec<_>>();

        let index_sql_query = format!(
            "SELECT sql FROM duckdb_indexes() WHERE {} ORDER BY index_name",
            match schema.filter(|schema_name| Self::should_filter_schema(schema_name)) {
                Some(schema_name) => format!(
                    "schema_name = '{}' AND table_name = '{}'",
                    Self::escape_sql_literal(schema_name),
                    Self::escape_sql_literal(table)
                ),
                None => format!("table_name = '{}'", Self::escape_sql_literal(table)),
            }
        );
        let index_rows = self
            .query_rows(connection, &index_sql_query, "Failed to export index DDL")
            .await?;
        statements.extend(
            index_rows
                .into_iter()
                .filter_map(|row| row.first().cloned().flatten())
                .filter(|sql| !sql.is_empty()),
        );

        Ok(statements.join("\n"))
    }

    fn get_data_types(&self) -> &[(&'static str, &'static str)] {
        DUCKDB_DATA_TYPES
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

    fn build_column_def(&self, column: &ColumnDefinition) -> String {
        let normalized = self.normalize_column_definition(column);
        self.sqlite.build_column_def(&normalized)
    }

    fn build_create_table_sql(&self, design: &TableDesign) -> String {
        let normalized = self.normalize_design(design);
        self.sqlite.build_create_table_sql(&normalized)
    }

    fn build_alter_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        if Self::primary_key_changed(original, new) {
            self.build_recreate_table_sql(original, new)
        } else {
            self.build_native_alter_sql(original, new)
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

#[cfg(test)]
mod tests {
    use super::DuckDbPlugin;
    use crate::connection::DbConnection;
    use crate::duckdb::DuckDbConnection;
    use crate::plugin::DatabasePlugin;
    use crate::types::{ColumnDefinition, IndexDefinition, TableDesign, TableOptions};
    use one_core::storage::{DatabaseType, DbConnectionConfig};

    fn build_config(path: String) -> DbConnectionConfig {
        DbConnectionConfig {
            id: "duckdb-plugin-test".to_string(),
            name: "duckdb-plugin-test".to_string(),
            database_type: DatabaseType::DuckDB,
            host: path,
            port: 0,
            workspace_id: None,
            username: String::new(),
            password: String::new(),
            database: None,
            service_name: None,
            sid: None,
            extra_params: Default::default(),
        }
    }

    async fn create_connection() -> (tempfile::TempDir, DuckDbConnection) {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let db_path = temp_dir.path().join("duckdb-plugin-test.duckdb");

        let mut connection =
            DuckDbConnection::new(build_config(db_path.to_string_lossy().to_string()));
        connection.connect().await.expect("duckdb should connect");
        (temp_dir, connection)
    }

    fn create_plugin() -> DuckDbPlugin {
        DuckDbPlugin::new()
    }

    #[tokio::test]
    async fn test_list_indexes_returns_secondary_indexes() {
        let (_temp_dir, mut connection) = create_connection().await;
        connection
            .query("CREATE TABLE test (id INTEGER, email TEXT);")
            .await
            .expect("table creation should succeed");
        connection
            .query("CREATE UNIQUE INDEX idx_test_email ON test (email);")
            .await
            .expect("index creation should succeed");

        let plugin = create_plugin();
        let indexes = plugin
            .list_indexes(&connection, "main", None, "test")
            .await
            .expect("list_indexes should succeed");

        assert_eq!(indexes.len(), 1);
        assert_eq!(indexes[0].name, "idx_test_email");
        assert!(indexes[0].is_unique);

        connection
            .disconnect()
            .await
            .expect("duckdb should disconnect");
    }

    #[tokio::test]
    async fn test_list_tables_columns_views_and_export_sql() {
        let (_temp_dir, mut connection) = create_connection().await;
        connection
            .query(
                "CREATE TABLE test (id INTEGER PRIMARY KEY, email VARCHAR NOT NULL, score INTEGER DEFAULT 0);",
            )
            .await
            .expect("table creation should succeed");
        connection
            .query("CREATE INDEX idx_test_score ON test (score);")
            .await
            .expect("index creation should succeed");
        connection
            .query("CREATE VIEW test_view AS SELECT id, email FROM test WHERE score > 0;")
            .await
            .expect("view creation should succeed");

        let plugin = create_plugin();

        let tables = plugin
            .list_tables(&connection, "main", Some("main".to_string()))
            .await
            .expect("list_tables should succeed");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "test");
        assert_eq!(tables[0].schema.as_deref(), Some("main"));

        let columns = plugin
            .list_columns(&connection, "main", Some("main".to_string()), "test")
            .await
            .expect("list_columns should succeed");
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0].name, "id");
        assert!(columns[0].is_primary_key);
        assert_eq!(columns[1].name, "email");
        assert!(!columns[1].is_nullable);
        assert_eq!(columns[2].default_value.as_deref(), Some("0"));

        let views = plugin
            .list_views(&connection, "main", Some("main".to_string()))
            .await
            .expect("list_views should succeed");
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "test_view");
        assert!(views[0]
            .definition
            .as_deref()
            .is_some_and(|definition| definition.contains("SELECT id, email FROM test")));

        let ddl = plugin
            .export_table_create_sql(&connection, "main", Some("main"), "test")
            .await
            .expect("export_table_create_sql should succeed");
        assert!(ddl.contains("CREATE TABLE"));
        assert!(ddl.contains("test"));
        assert!(ddl.contains("CREATE INDEX idx_test_score"));

        connection
            .disconnect()
            .await
            .expect("duckdb should disconnect");
    }

    #[test]
    fn test_build_create_table_sql_omits_sqlite_autoincrement_keyword() {
        let plugin = create_plugin();
        let design = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INTEGER")
                    .nullable(false)
                    .primary_key(true)
                    .auto_increment(true),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .nullable(false),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_create_table_sql(&design);

        assert!(sql.contains("CREATE TABLE \"users\""));
        assert!(sql.contains("PRIMARY KEY"));
        assert!(!sql.contains("AUTOINCREMENT"));
    }

    #[test]
    fn test_build_alter_table_sql_prefers_native_duckdb_alter_statements() {
        let plugin = create_plugin();
        let original = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INTEGER")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("name").data_type("TEXT"),
                ColumnDefinition::new("score").data_type("INTEGER"),
            ],
            indexes: vec![IndexDefinition::new("idx_score").columns(vec!["score".to_string()])],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };
        let current = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INTEGER")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .nullable(false)
                    .default_value("'anon'"),
                ColumnDefinition::new("email").data_type("VARCHAR"),
            ],
            indexes: vec![IndexDefinition::new("idx_name")
                .columns(vec!["name".to_string()])
                .unique(true)],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &current);

        assert!(sql.contains("DROP INDEX IF EXISTS \"idx_score\";"));
        assert!(sql.contains("ALTER TABLE \"users\" DROP COLUMN \"score\";"));
        assert!(sql.contains("ALTER TABLE \"users\" ALTER COLUMN \"name\" SET DATA TYPE VARCHAR;"));
        assert!(sql.contains("ALTER TABLE \"users\" ALTER COLUMN \"name\" SET NOT NULL;"));
        assert!(sql.contains("ALTER TABLE \"users\" ALTER COLUMN \"name\" SET DEFAULT 'anon';"));
        assert!(sql.contains("ALTER TABLE \"users\" ADD COLUMN \"email\" VARCHAR;"));
        assert!(sql.contains("CREATE UNIQUE INDEX \"idx_name\" ON \"users\" (\"name\");"));
        assert!(!sql.contains("_duckdb_tmp"));
    }

    #[test]
    fn test_build_alter_table_sql_recreates_table_when_primary_key_changes() {
        let plugin = create_plugin();
        let original = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INTEGER")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("email").data_type("VARCHAR"),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };
        let current = TableDesign {
            database_name: "main".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INTEGER")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("email")
                    .data_type("VARCHAR")
                    .nullable(false)
                    .primary_key(true),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &current);

        assert!(sql.contains("users_duckdb_tmp"));
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("DROP TABLE \"users\";"));
        assert!(sql.contains("ALTER TABLE \"users_duckdb_tmp\" RENAME TO \"users\";"));
    }
}
