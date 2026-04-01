use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use gpui_component::table::Column;
use one_core::storage::{DatabaseType, DbConnectionConfig};

use crate::connection::{DbConnection, DbError};
use crate::executor::SqlResult;
use crate::import_export::{
    ExportConfig, ExportProgressSender, ExportResult, ImportConfig, ImportProgressSender,
    ImportResult,
};
use crate::plugin::{DatabasePlugin, SqlCompletionInfo};
use crate::postgresql::connection::PostgresDbConnection;
use crate::types::*;

/// PostgreSQL data types (name, description)
pub const POSTGRESQL_DATA_TYPES: &[(&str, &str)] = &[
    ("SMALLINT", "Small integer (-32768 to 32767)"),
    ("INTEGER", "Standard integer"),
    ("BIGINT", "Large integer"),
    ("SERIAL", "Auto-incrementing integer"),
    ("BIGSERIAL", "Auto-incrementing bigint"),
    ("SMALLSERIAL", "Auto-incrementing smallint"),
    ("DECIMAL", "Exact numeric with precision"),
    ("NUMERIC", "Exact numeric with precision"),
    ("REAL", "Single-precision floating-point"),
    ("DOUBLE PRECISION", "Double-precision floating-point"),
    ("MONEY", "Currency amount"),
    ("CHAR", "Fixed-length string"),
    ("VARCHAR", "Variable-length string"),
    ("TEXT", "Unlimited text"),
    ("BYTEA", "Binary data"),
    ("DATE", "Date (no time)"),
    ("TIME", "Time without timezone"),
    ("TIMETZ", "Time with timezone"),
    ("TIMESTAMP", "Date and time without timezone"),
    ("TIMESTAMPTZ", "Date and time with timezone"),
    ("INTERVAL", "Time interval"),
    ("BOOLEAN", "True/False"),
    ("UUID", "Universally unique identifier"),
    ("JSON", "JSON document"),
    ("JSONB", "Binary JSON (indexed)"),
    ("XML", "XML document"),
    ("ARRAY", "Array type"),
    ("BIT", "Fixed-length bit string"),
    ("BIT VARYING", "Variable-length bit string"),
    ("INT4RANGE", "Range of integer"),
    ("INT8RANGE", "Range of bigint"),
    ("NUMRANGE", "Range of numeric"),
    ("TSRANGE", "Range of timestamp"),
    ("TSTZRANGE", "Range of timestamptz"),
    ("DATERANGE", "Range of date"),
    ("INET", "IPv4/IPv6 host address"),
    ("CIDR", "IPv4/IPv6 network address"),
    ("MACADDR", "MAC address"),
    ("POINT", "Geometric point"),
    ("LINE", "Infinite line"),
    ("LSEG", "Line segment"),
    ("BOX", "Rectangular box"),
    ("PATH", "Geometric path"),
    ("POLYGON", "Closed geometric path"),
    ("CIRCLE", "Circle"),
    ("TSVECTOR", "Text search document"),
    ("TSQUERY", "Text search query"),
];

/// PostgreSQL database plugin implementation (stateless)
pub struct PostgresPlugin;

impl PostgresPlugin {
    pub fn new() -> Self {
        Self
    }

    fn normalize_type_name(type_name: &str) -> String {
        let type_lower = type_name.to_lowercase();

        let (base_type, suffix) = if let Some(paren_pos) = type_lower.find('(') {
            (&type_lower[..paren_pos], &type_name[paren_pos..])
        } else if let Some(bracket_pos) = type_lower.find('[') {
            (&type_lower[..bracket_pos], &type_name[bracket_pos..])
        } else {
            (type_lower.as_str(), "")
        };

        let short_name = match base_type.trim() {
            "character varying" => "varchar",
            "character" => "char",
            "double precision" => "float8",
            "timestamp without time zone" => "timestamp",
            "timestamp with time zone" => "timestamptz",
            "time without time zone" => "time",
            "time with time zone" => "timetz",
            "bit varying" => "varbit",
            other => other,
        };

        format!("{}{}", short_name, suffix)
    }
}

#[async_trait]
impl DatabasePlugin for PostgresPlugin {
    fn name(&self) -> DatabaseType {
        DatabaseType::PostgreSQL
    }

    fn quote_identifier(&self, identifier: &str) -> String {
        format!("\"{}\"", identifier.replace("\"", "\"\""))
    }

    fn get_completion_info(&self) -> SqlCompletionInfo {
        SqlCompletionInfo {
            keywords: vec![
                // PostgreSQL-specific keywords only
                ("RETURNING", "Return inserted/updated rows"),
                ("SERIAL", "Auto-incrementing integer"),
                ("BIGSERIAL", "Auto-incrementing bigint"),
                ("CASCADE", "Cascade to dependent objects"),
                ("RESTRICT", "Restrict if dependencies exist"),
                ("CONCURRENTLY", "Non-blocking index creation"),
                ("ONLY", "Exclude inherited tables"),
                ("LATERAL", "Lateral subquery"),
                ("FETCH FIRST", "Limit rows (SQL standard)"),
                ("FOR UPDATE", "Lock rows for update"),
                ("FOR SHARE", "Lock rows for share"),
                ("SKIP LOCKED", "Skip locked rows"),
                ("NOWAIT", "Don't wait for locks"),
                ("NULLS FIRST", "Sort NULLs first"),
                ("NULLS LAST", "Sort NULLs last"),
                ("ILIKE", "Case-insensitive LIKE"),
                ("SIMILAR TO", "SQL regex pattern match"),
                ("OVER", "Window function clause"),
                ("PARTITION BY", "Window partition"),
                ("ROWS BETWEEN", "Window frame"),
                ("RANGE BETWEEN", "Window frame range"),
                ("WITH RECURSIVE", "Recursive CTE"),
                ("MATERIALIZED", "Materialized CTE"),
                ("NOT MATERIALIZED", "Non-materialized CTE"),
                ("TABLESAMPLE", "Sample table rows"),
                ("BERNOULLI", "Bernoulli sampling"),
                ("SYSTEM", "System sampling"),
            ],
            functions: vec![
                // PostgreSQL-specific functions only (standard SQL functions are added via with_standard_sql())
                (
                    "CONCAT_WS(sep, str1, str2, ...)",
                    "Concatenate with separator",
                ),
                (
                    "SUBSTRING(str FROM pos FOR len)",
                    "Extract substring (PostgreSQL syntax)",
                ),
                ("CHAR_LENGTH(str)", "Character length"),
                ("LPAD(str, len, fill)", "Left pad string"),
                ("RPAD(str, len, fill)", "Right pad string"),
                ("POSITION(sub IN str)", "Find substring position"),
                ("STRPOS(str, sub)", "Find substring position"),
                ("REPEAT(str, n)", "Repeat string"),
                ("SPLIT_PART(str, delim, n)", "Split and get part"),
                ("STRING_AGG(expr, delim)", "Aggregate strings"),
                ("INITCAP(str)", "Capitalize words"),
                ("REGEXP_REPLACE(str, pat, rep)", "Regex replace"),
                ("REGEXP_MATCHES(str, pat)", "Regex matches"),
                ("REGEXP_SPLIT_TO_ARRAY(str, pat)", "Split by regex"),
                ("TRANSLATE(str, from, to)", "Character translation"),
                ("TRUNC(x, s)", "Truncate to scale"),
                ("RANDOM()", "Random 0-1"),
                ("DIV(x, y)", "Integer division"),
                ("LOG(x)", "Natural logarithm"),
                ("LOG10(x)", "Base-10 logarithm"),
                ("EXP(x)", "Exponential"),
                ("GREATEST(a, b, ...)", "Maximum value"),
                ("LEAST(a, b, ...)", "Minimum value"),
                ("LOCALTIME", "Local time"),
                ("LOCALTIMESTAMP", "Local timestamp"),
                ("DATE_TRUNC(field, source)", "Truncate to precision"),
                ("DATE_PART(field, source)", "Extract field"),
                ("EXTRACT(field FROM source)", "Extract field"),
                ("AGE(ts1, ts2)", "Interval between timestamps"),
                ("AGE(ts)", "Age from current date"),
                ("MAKE_DATE(y, m, d)", "Create date"),
                ("MAKE_TIME(h, m, s)", "Create time"),
                ("MAKE_TIMESTAMP(y,m,d,h,mi,s)", "Create timestamp"),
                ("MAKE_INTERVAL(...)", "Create interval"),
                ("TO_CHAR(val, fmt)", "Format to string"),
                ("TO_DATE(str, fmt)", "Parse date"),
                ("TO_TIMESTAMP(str, fmt)", "Parse timestamp"),
                ("TO_NUMBER(str, fmt)", "Parse number"),
                ("CLOCK_TIMESTAMP()", "Actual current time"),
                ("STATEMENT_TIMESTAMP()", "Statement start time"),
                ("TRANSACTION_TIMESTAMP()", "Transaction start time"),
                ("ARRAY_AGG(col)", "Aggregate to array"),
                ("JSON_AGG(col)", "Aggregate to JSON array"),
                ("JSONB_AGG(col)", "Aggregate to JSONB array"),
                ("JSON_OBJECT_AGG(k, v)", "Aggregate to JSON object"),
                ("BOOL_AND(col)", "Logical AND"),
                ("BOOL_OR(col)", "Logical OR"),
                ("BIT_AND(col)", "Bitwise AND"),
                ("BIT_OR(col)", "Bitwise OR"),
                ("ROW_NUMBER()", "Row number in partition"),
                ("RANK()", "Rank with gaps"),
                ("DENSE_RANK()", "Rank without gaps"),
                ("NTILE(n)", "Divide into n buckets"),
                ("LAG(col, n)", "Previous row value"),
                ("LEAD(col, n)", "Next row value"),
                ("FIRST_VALUE(col)", "First value in frame"),
                ("LAST_VALUE(col)", "Last value in frame"),
                ("NTH_VALUE(col, n)", "Nth value in frame"),
                ("PERCENT_RANK()", "Relative rank"),
                ("CUME_DIST()", "Cumulative distribution"),
                ("JSON_BUILD_OBJECT(k, v, ...)", "Build JSON object"),
                ("JSON_BUILD_ARRAY(v, ...)", "Build JSON array"),
                ("JSONB_BUILD_OBJECT(k, v, ...)", "Build JSONB object"),
                ("JSONB_BUILD_ARRAY(v, ...)", "Build JSONB array"),
                ("JSON_EXTRACT_PATH(json, ...)", "Extract JSON path"),
                ("JSONB_EXTRACT_PATH(json, ...)", "Extract JSONB path"),
                ("JSON_EXTRACT_PATH_TEXT(json, ...)", "Extract as text"),
                ("JSONB_SET(target, path, val)", "Set JSONB value"),
                ("JSONB_INSERT(target, path, val)", "Insert JSONB value"),
                ("JSONB_PRETTY(jsonb)", "Pretty print JSONB"),
                ("JSONB_TYPEOF(jsonb)", "JSONB type"),
                ("JSONB_ARRAY_LENGTH(jsonb)", "JSONB array length"),
                ("JSONB_EACH(jsonb)", "Expand JSONB object"),
                ("JSONB_ARRAY_ELEMENTS(jsonb)", "Expand JSONB array"),
                ("JSONB_STRIP_NULLS(jsonb)", "Remove null values"),
                ("JSONB_PATH_QUERY(target, path)", "JSONPath query"),
                ("ARRAY_LENGTH(arr, dim)", "Array length"),
                ("ARRAY_DIMS(arr)", "Array dimensions"),
                ("ARRAY_UPPER(arr, dim)", "Upper bound"),
                ("ARRAY_LOWER(arr, dim)", "Lower bound"),
                ("ARRAY_POSITION(arr, elem)", "Element position"),
                ("ARRAY_POSITIONS(arr, elem)", "All positions"),
                ("ARRAY_REMOVE(arr, elem)", "Remove element"),
                ("ARRAY_REPLACE(arr, from, to)", "Replace element"),
                ("ARRAY_CAT(arr1, arr2)", "Concatenate arrays"),
                ("ARRAY_APPEND(arr, elem)", "Append element"),
                ("ARRAY_PREPEND(elem, arr)", "Prepend element"),
                ("UNNEST(arr)", "Expand array to rows"),
                ("GEN_RANDOM_UUID()", "Generate UUID"),
                ("MD5(str)", "MD5 hash"),
                ("ENCODE(data, fmt)", "Encode binary"),
                ("DECODE(str, fmt)", "Decode to binary"),
                ("PG_TYPEOF(val)", "Value type"),
                ("CURRENT_USER", "Current user"),
                ("CURRENT_DATABASE()", "Current database"),
                ("CURRENT_SCHEMA()", "Current schema"),
                ("VERSION()", "PostgreSQL version"),
            ],
            operators: vec![
                ("~", "Regex match (case-sensitive)"),
                ("~*", "Regex match (case-insensitive)"),
                ("!~", "Regex not match (case-sensitive)"),
                ("!~*", "Regex not match (case-insensitive)"),
                ("||", "String/Array concatenation"),
                ("->", "JSON object field"),
                ("->>", "JSON object field as text"),
                ("#>", "JSON path"),
                ("#>>", "JSON path as text"),
                ("@>", "Contains"),
                ("<@", "Contained by"),
                ("?", "Key exists"),
                ("?|", "Any key exists"),
                ("?&", "All keys exist"),
                ("@?", "JSONPath exists"),
                ("@@", "JSONPath match"),
                ("-", "Delete key/element"),
                ("#-", "Delete path"),
                ("&&", "Array overlap"),
                ("<<", "Range strictly left"),
                (">>", "Range strictly right"),
                ("&<", "Range not extend right"),
                ("&>", "Range not extend left"),
                ("-|-", "Range adjacent"),
            ],
            data_types: POSTGRESQL_DATA_TYPES.to_vec(),
            snippets: vec![
                (
                    "crt",
                    "CREATE TABLE $1 (\n  id SERIAL PRIMARY KEY,\n  $2\n)",
                    "Create table",
                ),
                ("idx", "CREATE INDEX $1 ON $2 ($3)", "Create index"),
                (
                    "cidx",
                    "CREATE INDEX CONCURRENTLY $1 ON $2 ($3)",
                    "Create index concurrently",
                ),
                (
                    "cte",
                    "WITH $1 AS (\n  $2\n)\nSELECT * FROM $1",
                    "Common table expression",
                ),
                (
                    "rcte",
                    "WITH RECURSIVE $1 AS (\n  $2\n  UNION ALL\n  $3\n)\nSELECT * FROM $1",
                    "Recursive CTE",
                ),
                (
                    "wf",
                    "SELECT $1,\n  ROW_NUMBER() OVER (PARTITION BY $2 ORDER BY $3) AS rn\nFROM $4",
                    "Window function",
                ),
            ],
        }
        .with_standard_sql()
    }

    async fn create_connection(
        &self,
        config: DbConnectionConfig,
    ) -> Result<Box<dyn DbConnection + Send + Sync>, DbError> {
        let mut conn = PostgresDbConnection::new(config);
        conn.connect().await?;
        Ok(Box::new(conn))
    }

    async fn list_databases(&self, connection: &dyn DbConnection) -> Result<Vec<String>> {
        let result = connection
            .query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list databases: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .filter_map(|row| row.first().and_then(|v| v.clone()))
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_databases_view(&self, connection: &dyn DbConnection) -> Result<ObjectView> {
        use gpui::px;

        let databases = self.list_databases_detailed(connection).await?;

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("charset", "Encoding").width(px(120.0)),
            Column::new("collation", "Collation").width(px(180.0)),
            Column::new("size", "Size").width(px(100.0)).text_right(),
            Column::new("tables", "Tables").width(px(80.0)).text_right(),
            Column::new("comment", "Comment").width(px(250.0)),
        ];

        let rows: Vec<Vec<String>> = databases
            .iter()
            .map(|db| {
                vec![
                    db.name.clone(),
                    db.charset.as_deref().unwrap_or("-").to_string(),
                    db.collation.as_deref().unwrap_or("-").to_string(),
                    db.size.as_deref().unwrap_or("-").to_string(),
                    db.table_count
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    db.comment.as_deref().unwrap_or("").to_string(),
                ]
            })
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Database,
            title: format!("{} database(s)", databases.len()),
            columns,
            rows,
        })
    }

    async fn list_databases_detailed(
        &self,
        connection: &dyn DbConnection,
    ) -> Result<Vec<DatabaseInfo>> {
        let result = connection
            .query(
                "SELECT 
                d.datname as name,
                pg_encoding_to_char(d.encoding) as charset,
                d.datcollate as collation,
                pg_size_pretty(pg_database_size(d.datname)) as size,
                (SELECT COUNT(*) FROM pg_tables WHERE schemaname = 'public') as table_count,
                shobj_description(d.oid, 'pg_database') as comment
            FROM pg_database d
            WHERE d.datistemplate = false 
            ORDER BY d.datname",
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list databases: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let databases: Vec<DatabaseInfo> = query_result
                .rows
                .iter()
                .filter_map(|row| {
                    let name = row.first().and_then(|v| v.clone())?;
                    let charset = row.get(1).and_then(|v| v.clone());
                    let collation = row.get(2).and_then(|v| v.clone());
                    let size = row.get(3).and_then(|v| v.clone());
                    let table_count = row
                        .get(4)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse::<i64>().ok());
                    let comment = row.get(5).and_then(|v| v.clone());

                    Some(DatabaseInfo {
                        name,
                        charset,
                        collation,
                        size,
                        table_count,
                        comment,
                    })
                })
                .collect();
            Ok(databases)
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    fn supports_schema(&self) -> bool {
        true
    }

    fn supports_sequences(&self) -> bool {
        true
    }

    fn sql_dialect(&self) -> Box<dyn sqlparser::dialect::Dialect> {
        Box::new(sqlparser::dialect::PostgreSqlDialect {})
    }

    // === Database/Schema Level Operations ===

    async fn list_schemas(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<String>> {
        let result = connection
            .query(
                "SELECT schema_name FROM information_schema.schemata \
             WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast') \
             ORDER BY schema_name",
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list schemas: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .filter_map(|row| row.first().and_then(|v| v.clone()))
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_schemas_view(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sql = "SELECT
                n.nspname AS schema_name,
                pg_catalog.pg_get_userbyid(n.nspowner) AS owner,
                (SELECT COUNT(*) FROM pg_tables t WHERE t.schemaname = n.nspname) AS table_count,
                obj_description(n.oid, 'pg_namespace') AS description
            FROM pg_catalog.pg_namespace n
            WHERE n.nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
              AND n.nspname NOT LIKE 'pg_%'
            ORDER BY n.nspname";

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list schemas: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let columns = vec![
                Column::new("name", "Name").width(px(180.0)),
                Column::new("owner", "Owner").width(px(120.0)),
                Column::new("tables", "Tables").width(px(80.0)).text_right(),
                Column::new("description", "Description").width(px(300.0)),
            ];

            let rows: Vec<Vec<String>> = query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.first().and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .unwrap_or_else(|| "0".to_string()),
                        row.get(3).and_then(|v| v.clone()).unwrap_or_default(),
                    ]
                })
                .collect();

            Ok(ObjectView {
                db_node_type: DbNodeType::Schema,
                title: format!("{} schema(s)", rows.len()),
                columns,
                rows,
            })
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_tables(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
        schema: Option<String>,
    ) -> Result<Vec<TableInfo>> {
        let schema_val = schema.unwrap_or_else(|| "public".to_string());
        let sql = format!(
            "SELECT
                t.tablename,
                t.schemaname,
                t.tableowner,
                obj_description((quote_ident(t.schemaname) || '.' || quote_ident(t.tablename))::regclass) AS table_comment,
                (SELECT reltuples::bigint FROM pg_class c JOIN pg_namespace n ON c.relnamespace = n.oid WHERE c.relname = t.tablename AND n.nspname = t.schemaname) AS row_count,
                pg_size_pretty(pg_total_relation_size((quote_ident(t.schemaname) || '.' || quote_ident(t.tablename))::regclass)) AS total_size
             FROM pg_tables t
             WHERE t.schemaname = '{}'
             ORDER BY t.tablename",
            schema_val.replace("'", "''")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list tables: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let tables: Vec<TableInfo> = query_result
                .rows
                .iter()
                .map(|row| {
                    let row_count = row
                        .get(4)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse::<i64>().ok());

                    TableInfo {
                        name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                        schema: row.get(1).and_then(|v| v.clone()),
                        comment: row.get(3).and_then(|v| v.clone()).filter(|s| !s.is_empty()),
                        engine: row.get(5).and_then(|v| v.clone()), // 用 engine 字段存储 size
                        row_count,
                        create_time: row.get(2).and_then(|v| v.clone()), // 用 create_time 字段存储 owner
                        charset: None,
                        collation: None,
                    }
                })
                .collect();

            Ok(tables)
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    // === Table Operations ===

    async fn list_tables_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<ObjectView> {
        use gpui::px;

        let tables = self.list_tables(connection, database, schema).await?;

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("owner", "Owner").width(px(100.0)),
            Column::new("rows", "Rows").width(px(100.0)).text_right(),
            Column::new("size", "Size").width(px(100.0)).text_right(),
            Column::new("comment", "Comment").width(px(300.0)),
        ];

        let rows: Vec<Vec<String>> = tables
            .iter()
            .map(|table| {
                vec![
                    table.name.clone(),
                    table.create_time.as_deref().unwrap_or("-").to_string(), // owner
                    table
                        .row_count
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    table.engine.as_deref().unwrap_or("-").to_string(), // size
                    table.comment.as_deref().unwrap_or("").to_string(),
                ]
            })
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
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<ColumnInfo>> {
        let schema_val = schema.unwrap_or_else(|| "public".to_string());
        let sql = format!(
            "SELECT \
                a.attname AS column_name, \
                format_type(a.atttypid, a.atttypmod) AS data_type, \
                CASE WHEN a.attnotnull THEN 'NO' ELSE 'YES' END AS is_nullable, \
                pg_get_expr(d.adbin, d.adrelid) AS column_default, \
                EXISTS ( \
                    SELECT 1 FROM pg_constraint c \
                    WHERE c.conrelid = a.attrelid \
                    AND a.attnum = ANY(c.conkey) \
                    AND c.contype = 'p' \
                ) AS is_primary \
            FROM pg_attribute a \
            LEFT JOIN pg_attrdef d ON a.attrelid = d.adrelid AND a.attnum = d.adnum \
            JOIN pg_class t ON a.attrelid = t.oid \
            JOIN pg_namespace n ON t.relnamespace = n.oid \
            WHERE n.nspname = '{}' \
            AND t.relname = '{}' \
            AND a.attnum > 0 \
            AND NOT a.attisdropped \
            ORDER BY a.attnum",
            schema_val.replace("'", "''"),
            table.replace("'", "''")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list columns: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| {
                    let raw_type = row.get(1).and_then(|v| v.clone()).unwrap_or_default();
                    ColumnInfo {
                        name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                        data_type: Self::normalize_type_name(&raw_type),
                        is_nullable: row
                            .get(2)
                            .and_then(|v| v.clone())
                            .map(|v| v == "YES")
                            .unwrap_or(true),
                        is_primary_key: row
                            .get(4)
                            .and_then(|v| v.clone())
                            .map(|v| v == "t" || v == "true" || v == "1")
                            .unwrap_or(false),
                        default_value: row.get(3).and_then(|v| v.clone()),
                        comment: None,
                        charset: None,
                        collation: None,
                    }
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
            Column::new("default", "Default").width(px(200.0)),
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
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<IndexInfo>> {
        let schema_val = schema.unwrap_or_else(|| "public".to_string());
        let sql = format!(
            "SELECT i.relname AS index_name, \
             a.attname AS column_name, \
             ix.indisunique AS is_unique \
             FROM pg_class t \
             JOIN pg_index ix ON t.oid = ix.indrelid \
             JOIN pg_class i ON i.oid = ix.indexrelid \
             JOIN pg_namespace n ON t.relnamespace = n.oid \
             JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) \
             WHERE t.relname = '{}' AND t.relkind = 'r' AND n.nspname = '{}' \
             AND NOT ix.indisprimary \
             ORDER BY i.relname, a.attnum",
            table.replace("'", "''"),
            schema_val.replace("'", "''")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list indexes: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let mut indexes: HashMap<String, IndexInfo> = HashMap::new();

            for row in query_result.rows {
                let index_name = row.first().and_then(|v| v.clone()).unwrap_or_default();
                let column_name = row.get(1).and_then(|v| v.clone()).unwrap_or_default();
                let is_unique = row
                    .get(2)
                    .and_then(|v| v.clone())
                    .map(|v| v == "t" || v == "true")
                    .unwrap_or(false);

                indexes
                    .entry(index_name.clone())
                    .or_insert_with(|| IndexInfo {
                        name: index_name,
                        columns: Vec::new(),
                        is_unique,
                        index_type: Some("btree".to_string()),
                    })
                    .columns
                    .push(column_name);
            }

            Ok(indexes.into_values().collect())
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
            Column::new("type", "Type").width(px(120.0)),
        ];

        let rows: Vec<Vec<String>> = indexes
            .iter()
            .map(|idx| {
                vec![
                    idx.name.clone(),
                    idx.columns.join(", "),
                    if idx.is_unique { "YES" } else { "NO" }.to_string(),
                    idx.index_type.as_deref().unwrap_or("-").to_string(),
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

    async fn list_table_checks(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
        _table: &str,
    ) -> Result<Vec<CheckInfo>> {
        let schema_val = _schema.unwrap_or_else(|| "public".to_string());
        let sql = format!(
            "SELECT c.conname AS constraint_name, \
                    t.relname AS table_name, \
                    pg_get_constraintdef(c.oid) AS definition \
             FROM pg_constraint c \
             JOIN pg_class t ON c.conrelid = t.oid \
             JOIN pg_namespace n ON t.relnamespace = n.oid \
             WHERE c.contype = 'c' \
               AND n.nspname = '{}' \
               AND t.relname = '{}' \
             ORDER BY c.conname",
            schema_val.replace("'", "''"),
            _table.replace("'", "''")
        );

        let result = _connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list check constraints: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| CheckInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    table_name: row.get(1).and_then(|v| v.clone()).unwrap_or_default(),
                    definition: row.get(2).and_then(|v| v.clone()),
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    // === View Operations ===

    async fn list_views(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
        schema: Option<String>,
    ) -> Result<Vec<ViewInfo>> {
        let schema_val = schema.unwrap_or_else(|| "public".to_string());
        let sql = format!(
            "SELECT table_name, table_schema, view_definition FROM information_schema.views \
             WHERE table_schema = '{}' \
             ORDER BY table_name",
            schema_val.replace("'", "''")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list views: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| ViewInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    schema: row.get(1).and_then(|v| v.clone()),
                    definition: row.get(2).and_then(|v| v.clone()),
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

    // === Function Operations ===

    async fn list_functions(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let sql = "SELECT routine_name, data_type FROM information_schema.routines WHERE routine_schema = 'public' AND routine_type = 'FUNCTION' ORDER BY routine_name";

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list functions: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| FunctionInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    return_type: row.get(1).and_then(|v| v.clone()),
                    parameters: Vec::new(),
                    definition: None,
                    comment: None,
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_functions_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let functions = self.list_functions(connection, database).await?;

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("return_type", "Return Type").width(px(150.0)),
        ];

        let rows: Vec<Vec<String>> = functions
            .iter()
            .map(|func| {
                vec![
                    func.name.clone(),
                    func.return_type.as_deref().unwrap_or("-").to_string(),
                ]
            })
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Function,
            title: format!("{} function(s)", functions.len()),
            columns,
            rows,
        })
    }

    // === Procedure Operations ===

    async fn list_procedures(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let sql = "SELECT routine_name FROM information_schema.routines WHERE routine_schema = 'public' AND routine_type = 'PROCEDURE' ORDER BY routine_name";

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list procedures: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| FunctionInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    return_type: None,
                    parameters: Vec::new(),
                    definition: None,
                    comment: None,
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_procedures_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let procedures = self.list_procedures(connection, database).await?;

        let columns = vec![Column::new("name", "Name").width(px(200.0))];

        let rows: Vec<Vec<String>> = procedures
            .iter()
            .map(|proc| vec![proc.name.clone()])
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Procedure,
            title: format!("{} procedure(s)", procedures.len()),
            columns,
            rows,
        })
    }

    // === Trigger Operations ===

    async fn list_triggers(
        &self,
        connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<TriggerInfo>> {
        let sql = "SELECT trigger_name, event_object_table, event_manipulation, action_timing \
                   FROM information_schema.triggers \
                   WHERE trigger_schema = 'public' \
                   ORDER BY trigger_name";

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
                    event: row.get(2).and_then(|v| v.clone()).unwrap_or_default(),
                    timing: row.get(3).and_then(|v| v.clone()).unwrap_or_default(),
                    definition: None,
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
            Column::new("event", "Event").width(px(100.0)),
            Column::new("timing", "Timing").width(px(100.0)),
        ];

        let rows: Vec<Vec<String>> = triggers
            .iter()
            .map(|trigger| {
                vec![
                    trigger.name.clone(),
                    trigger.table_name.clone(),
                    trigger.event.clone(),
                    trigger.timing.clone(),
                ]
            })
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
        connection: &dyn DbConnection,
        _database: &str,
        schema: Option<String>,
    ) -> Result<Vec<SequenceInfo>> {
        let schema_val = schema.unwrap_or_else(|| "public".to_string());
        let sql = format!(
            "SELECT sequence_name, start_value::bigint, increment::bigint, min_value::bigint, max_value::bigint \
             FROM information_schema.sequences \
             WHERE sequence_schema = '{}' \
             ORDER BY sequence_name",
            schema_val.replace("'", "''")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list sequences: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| SequenceInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    start_value: row
                        .get(1)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse().ok()),
                    increment: row
                        .get(2)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse().ok()),
                    min_value: row
                        .get(3)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse().ok()),
                    max_value: row
                        .get(4)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse().ok()),
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    // === Sequence Operations ===

    async fn list_sequences_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sequences = self.list_sequences(connection, database, None).await?;

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("start", "Start").width(px(100.0)).text_right(),
            Column::new("increment", "Increment")
                .width(px(100.0))
                .text_right(),
            Column::new("min", "Min").width(px(120.0)).text_right(),
            Column::new("max", "Max").width(px(120.0)).text_right(),
        ];

        let rows: Vec<Vec<String>> = sequences
            .iter()
            .map(|seq| {
                vec![
                    seq.name.clone(),
                    seq.start_value
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    seq.increment
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    seq.min_value
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    seq.max_value
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect();

        Ok(ObjectView {
            db_node_type: DbNodeType::Sequence,
            title: format!("{} sequence(s)", sequences.len()),
            columns,
            rows,
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
        request: &crate::plugin::DatabaseOperationRequest,
    ) -> String {
        let db_name = self.quote_identifier(&request.database_name);
        let encoding = request
            .field_values
            .get("encoding")
            .map(|s| s.as_str())
            .unwrap_or("UTF8")
            .replace('\'', "''");

        format!("CREATE DATABASE {} ENCODING '{}';", db_name, encoding)
    }

    fn build_modify_database_sql(
        &self,
        request: &crate::plugin::DatabaseOperationRequest,
    ) -> String {
        let db_name = self.quote_identifier(&request.database_name);
        format!("ALTER DATABASE {} SET search_path = public;", db_name)
    }

    fn build_drop_database_sql(&self, database_name: &str) -> String {
        format!("DROP DATABASE {};", self.quote_identifier(database_name))
    }

    fn build_create_schema_sql(&self, schema_name: &str) -> String {
        format!("CREATE SCHEMA {};", self.quote_identifier(schema_name))
    }

    fn build_drop_schema_sql(&self, schema_name: &str) -> String {
        format!(
            "DROP SCHEMA {} CASCADE;",
            self.quote_identifier(schema_name)
        )
    }

    fn build_comment_schema_sql(&self, schema_name: &str, comment: &str) -> Option<String> {
        Some(format!(
            "COMMENT ON SCHEMA {} IS '{}';",
            self.quote_identifier(schema_name),
            comment.replace("'", "''")
        ))
    }

    fn format_table_reference(&self, _database: &str, schema: Option<&str>, table: &str) -> String {
        let schema_name = schema.unwrap_or("public");
        format!(
            "{}.{}",
            self.quote_identifier(schema_name),
            self.quote_identifier(table)
        )
    }

    fn build_limit_clause(&self) -> String {
        " LIMIT 1".to_string()
    }

    fn build_where_and_limit_clause(
        &self,
        request: &TableSaveRequest,
        original_data: &[String],
    ) -> (String, String) {
        let where_clause = self.build_table_change_where_clause(request, original_data);
        (where_clause, self.build_limit_clause())
    }

    fn get_data_types(&self) -> &[(&'static str, &'static str)] {
        POSTGRESQL_DATA_TYPES
    }

    fn drop_table(&self, _database: &str, schema: Option<&str>, table: &str) -> String {
        // PostgreSQL uses schema.table format, database is ignored
        // because you can only operate on the current database
        if let Some(schema) = schema {
            format!(
                "DROP TABLE IF EXISTS {}.{}",
                self.quote_identifier(schema),
                self.quote_identifier(table)
            )
        } else {
            format!("DROP TABLE IF EXISTS {}", self.quote_identifier(table))
        }
    }

    fn rename_table(&self, _database: &str, old_name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            self.quote_identifier(old_name),
            self.quote_identifier(new_name)
        )
    }

    fn build_backup_table_sql(
        &self,
        _database: &str,
        schema: Option<&str>,
        source_table: &str,
        target_table: &str,
    ) -> String {
        let qualify = |table: &str| match schema {
            Some(schema) => format!(
                "{}.{}",
                self.quote_identifier(schema),
                self.quote_identifier(table)
            ),
            None => self.quote_identifier(table),
        };
        let source = qualify(source_table);
        let target = qualify(target_table);
        format!(
            "CREATE TABLE {} (LIKE {} INCLUDING ALL);\nINSERT INTO {} SELECT * FROM {};",
            target, source, target, source
        )
    }

    fn build_column_def(&self, col: &ColumnDefinition) -> String {
        let mut def = String::new();
        def.push_str(&self.quote_identifier(&col.name));
        def.push(' ');

        if col.is_auto_increment {
            let upper_type = col.data_type.to_uppercase();
            if upper_type == "BIGINT" || upper_type == "INT8" {
                def.push_str("BIGSERIAL");
            } else if upper_type == "SMALLINT" || upper_type == "INT2" {
                def.push_str("SMALLSERIAL");
            } else {
                def.push_str("SERIAL");
            }
        } else {
            let type_str = self.build_type_string(col);
            def.push_str(&type_str);
        }

        if !col.is_nullable {
            def.push_str(" NOT NULL");
        }

        if let Some(default) = &col.default_value {
            if !default.is_empty() {
                def.push_str(&format!(" DEFAULT {}", default));
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
        let mut statements: Vec<String> = Vec::new();
        let table_name = self.quote_identifier(&new.table_name);

        let original_cols: std::collections::HashMap<&str, &ColumnDefinition> = original
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let new_cols: std::collections::HashMap<&str, &ColumnDefinition> =
            new.columns.iter().map(|c| (c.name.as_str(), c)).collect();

        for name in original_cols.keys() {
            if !new_cols.contains_key(name) {
                statements.push(format!(
                    "ALTER TABLE {} DROP COLUMN {};",
                    table_name,
                    self.quote_identifier(name)
                ));
            }
        }

        for col in new.columns.iter() {
            if let Some(orig_col) = original_cols.get(col.name.as_str()) {
                if self.column_changed(orig_col, col) {
                    let col_name = self.quote_identifier(&col.name);

                    if orig_col.data_type != col.data_type || orig_col.length != col.length {
                        let type_str = self.build_type_string(col);
                        statements.push(format!(
                            "ALTER TABLE {} ALTER COLUMN {} TYPE {};",
                            table_name, col_name, type_str
                        ));
                    }

                    if orig_col.is_nullable != col.is_nullable {
                        if col.is_nullable {
                            statements.push(format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;",
                                table_name, col_name
                            ));
                        } else {
                            statements.push(format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;",
                                table_name, col_name
                            ));
                        }
                    }

                    if orig_col.default_value != col.default_value {
                        if let Some(default) = &col.default_value {
                            statements.push(format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};",
                                table_name, col_name, default
                            ));
                        } else {
                            statements.push(format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;",
                                table_name, col_name
                            ));
                        }
                    }
                }
            } else {
                let col_def = self.build_column_def(col);
                statements.push(format!(
                    "ALTER TABLE {} ADD COLUMN {};",
                    table_name, col_def
                ));
            }
        }

        let original_indexes: std::collections::HashMap<&str, &IndexDefinition> = original
            .indexes
            .iter()
            .map(|i| (i.name.as_str(), i))
            .collect();
        let new_indexes: std::collections::HashMap<&str, &IndexDefinition> =
            new.indexes.iter().map(|i| (i.name.as_str(), i)).collect();

        for (name, idx) in &original_indexes {
            if !new_indexes.contains_key(name) {
                if idx.is_primary {
                    statements.push(format!(
                        "ALTER TABLE {} DROP CONSTRAINT {}_pkey;",
                        table_name, new.table_name
                    ));
                } else {
                    statements.push(format!("DROP INDEX {};", self.quote_identifier(name)));
                }
            }
        }

        for (name, idx) in &new_indexes {
            if !original_indexes.contains_key(name) {
                let idx_cols: Vec<String> = idx
                    .columns
                    .iter()
                    .map(|c| self.quote_identifier(c))
                    .collect();

                if idx.is_primary {
                    statements.push(format!(
                        "ALTER TABLE {} ADD PRIMARY KEY ({});",
                        table_name,
                        idx_cols.join(", ")
                    ));
                } else {
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
        }

        if statements.is_empty() {
            "-- No changes detected".to_string()
        } else {
            statements.join("\n")
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

impl Default for PostgresPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::DatabasePlugin;
    use crate::types::{ColumnDefinition, IndexDefinition, TableDesign, TableOptions};
    use std::collections::HashMap;

    fn create_plugin() -> PostgresPlugin {
        PostgresPlugin::new()
    }

    // ==================== Basic Plugin Info Tests ====================

    #[test]
    fn test_plugin_name() {
        let plugin = create_plugin();
        assert_eq!(plugin.name(), DatabaseType::PostgreSQL);
    }

    #[test]
    fn test_quote_identifier() {
        let plugin = create_plugin();
        assert_eq!(plugin.quote_identifier("table_name"), "\"table_name\"");
        assert_eq!(plugin.quote_identifier("column"), "\"column\"");
        assert_eq!(plugin.quote_identifier("col\"umn"), "\"col\"\"umn\"");
    }

    #[test]
    fn test_format_table_reference() {
        let plugin = create_plugin();
        assert_eq!(
            plugin.format_table_reference("public", None, "users"),
            "\"public\".\"users\""
        );
    }

    #[test]
    fn test_supports_schema() {
        let plugin = create_plugin();
        assert!(plugin.supports_schema());
    }

    #[test]
    fn test_supports_sequences() {
        let plugin = create_plugin();
        assert!(plugin.supports_sequences());
    }

    // ==================== DDL SQL Generation Tests ====================

    #[test]
    fn test_drop_database() {
        let plugin = create_plugin();
        let sql = plugin.drop_database("test_db");
        assert!(sql.contains("DROP DATABASE"));
        assert!(sql.contains("\"test_db\""));
    }

    #[test]
    fn test_drop_table() {
        let plugin = create_plugin();

        // Test without schema
        let sql = plugin.drop_table("test_db", None, "users");
        assert!(sql.contains("DROP TABLE IF EXISTS"));
        assert!(sql.contains("\"users\""));
        assert!(!sql.contains("test_db")); // database should not be in the SQL

        // Test with schema
        let sql_with_schema = plugin.drop_table("test_db", Some("public"), "users");
        assert!(sql_with_schema.contains("DROP TABLE IF EXISTS"));
        assert!(sql_with_schema.contains("\"public\""));
        assert!(sql_with_schema.contains("\"users\""));
        assert!(!sql_with_schema.contains("test_db")); // database should not be in the SQL
    }

    #[test]
    fn test_truncate_table() {
        let plugin = create_plugin();
        let sql = plugin.truncate_table("test_db", "users");
        assert!(sql.contains("TRUNCATE TABLE"));
        assert!(sql.contains("\"users\""));
    }

    #[test]
    fn test_rename_table() {
        let plugin = create_plugin();
        let sql = plugin.rename_table("test_db", "old_name", "new_name");
        assert!(sql.contains("ALTER TABLE"));
        assert!(sql.contains("RENAME TO"));
        assert!(sql.contains("\"old_name\""));
        assert!(sql.contains("\"new_name\""));
    }

    #[test]
    fn test_build_backup_table_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_backup_table_sql("test_db", Some("public"), "orders", "orders_bak");
        assert!(sql.contains(
            "CREATE TABLE \"public\".\"orders_bak\" (LIKE \"public\".\"orders\" INCLUDING ALL);"
        ));
        assert!(sql.contains(
            "INSERT INTO \"public\".\"orders_bak\" SELECT * FROM \"public\".\"orders\";"
        ));
    }

    #[test]
    fn test_drop_view() {
        let plugin = create_plugin();
        let sql = plugin.drop_view("test_db", "my_view");
        assert!(sql.contains("DROP VIEW"));
        assert!(sql.contains("\"my_view\""));
    }

    // ==================== Database Operations Tests ====================

    #[test]
    fn test_build_create_database_sql() {
        let plugin = create_plugin();
        let mut field_values = HashMap::new();
        field_values.insert("encoding".to_string(), "UTF8".to_string());

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "new_db".to_string(),
            field_values,
        };

        let sql = plugin.build_create_database_sql(&request);
        assert!(sql.contains("CREATE DATABASE"));
        assert!(sql.contains("\"new_db\""));
        assert!(sql.contains("UTF8"));
    }

    #[test]
    fn test_build_create_database_sql_escapes_identifier() {
        let plugin = create_plugin();
        let mut field_values = HashMap::new();
        field_values.insert("encoding".to_string(), "UTF8".to_string());

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "new\"db".to_string(),
            field_values,
        };

        let sql = plugin.build_create_database_sql(&request);
        assert!(sql.contains("CREATE DATABASE"));
        assert!(sql.contains("\"new\"\"db\""));
    }

    #[test]
    fn test_build_modify_database_sql() {
        let plugin = create_plugin();
        let field_values = HashMap::new();

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "my_db".to_string(),
            field_values,
        };

        let sql = plugin.build_modify_database_sql(&request);
        assert!(sql.contains("ALTER DATABASE"));
        assert!(sql.contains("\"my_db\""));
    }

    #[test]
    fn test_build_drop_database_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_database_sql("old_db");
        assert_eq!(sql, "DROP DATABASE \"old_db\";");
    }

    #[test]
    fn test_build_drop_database_sql_escapes_identifier() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_database_sql("old\"db");
        assert_eq!(sql, "DROP DATABASE \"old\"\"db\";");
    }

    // ==================== Schema Operations Tests ====================

    #[test]
    fn test_build_create_schema_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_create_schema_sql("my_schema");
        assert!(sql.contains("CREATE SCHEMA"));
        assert!(sql.contains("\"my_schema\""));
    }

    #[test]
    fn test_build_drop_schema_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_schema_sql("my_schema");
        assert!(sql.contains("DROP SCHEMA"));
        assert!(sql.contains("\"my_schema\""));
        assert!(sql.contains("CASCADE"));
    }

    #[test]
    fn test_build_comment_schema_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_comment_schema_sql("my_schema", "Test schema");
        assert!(sql.is_some());
        let sql = sql.unwrap();
        assert!(sql.contains("COMMENT ON SCHEMA"));
        assert!(sql.contains("\"my_schema\""));
        assert!(sql.contains("Test schema"));
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
    fn test_build_column_def_with_length() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("name")
            .data_type("VARCHAR")
            .length(255)
            .nullable(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("\"name\""));
        assert!(def.contains("VARCHAR(255)"));
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

    #[test]
    fn test_build_column_def_serial() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("id")
            .data_type("SERIAL")
            .nullable(false)
            .primary_key(true)
            .auto_increment(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("\"id\""));
        assert!(def.contains("SERIAL"));
    }

    // ==================== CREATE TABLE Tests ====================

    #[test]
    fn test_build_create_table_sql_simple() {
        let plugin = create_plugin();
        let design = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("SERIAL")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .length(100),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_create_table_sql(&design);
        assert!(sql.contains("CREATE TABLE \"users\""));
        assert!(sql.contains("\"id\""));
        assert!(sql.contains("SERIAL"));
        assert!(sql.contains("\"name\""));
        assert!(sql.contains("VARCHAR(100)"));
        assert!(sql.contains("PRIMARY KEY"));
    }

    #[test]
    fn test_build_create_table_sql_with_indexes() {
        let plugin = create_plugin();
        let design = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "orders".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("SERIAL")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("user_id")
                    .data_type("INTEGER")
                    .nullable(false),
                ColumnDefinition::new("email")
                    .data_type("VARCHAR")
                    .length(100),
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
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id").data_type("INTEGER")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INTEGER"),
                ColumnDefinition::new("email")
                    .data_type("VARCHAR")
                    .length(100),
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
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INTEGER"),
                ColumnDefinition::new("old_column")
                    .data_type("VARCHAR")
                    .length(50),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id").data_type("INTEGER")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("DROP COLUMN"));
        assert!(sql.contains("\"old_column\""));
    }

    #[test]
    fn test_build_alter_table_sql_modify_column_type() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("name")
                .data_type("VARCHAR")
                .length(50)],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("name")
                .data_type("VARCHAR")
                .length(100)],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("ALTER COLUMN"));
        assert!(sql.contains("TYPE"));
        assert!(sql.contains("VARCHAR(100)"));
    }

    #[test]
    fn test_build_alter_table_sql_reorder_columns_no_changes() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INTEGER"),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .length(50),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .length(50),
                ColumnDefinition::new("id").data_type("INTEGER"),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert_eq!(sql, "-- No changes detected");
    }

    #[test]
    fn test_build_alter_table_sql_set_default_and_not_null() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("name")
                .data_type("VARCHAR")
                .length(50)],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("name")
                .data_type("VARCHAR")
                .length(50)
                .nullable(false)
                .default_value("'guest'")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("SET NOT NULL"));
        assert!(sql.contains("SET DEFAULT 'guest'"));
    }

    // ==================== Data Types Tests ====================

    #[test]
    fn test_get_data_types() {
        let plugin = create_plugin();
        let types = plugin.get_data_types();

        assert!(!types.is_empty());
        assert!(types.iter().any(|t| t.0 == "INTEGER"));
        assert!(types.iter().any(|t| t.0 == "VARCHAR"));
        assert!(types.iter().any(|t| t.0 == "TEXT"));
        assert!(types.iter().any(|t| t.0 == "TIMESTAMP"));
        assert!(types.iter().any(|t| t.0 == "JSONB"));
        assert!(types.iter().any(|t| t.0 == "UUID"));
        assert!(types.iter().any(|t| t.0 == "SERIAL"));
    }

    // ==================== Completion Info Tests ====================

    #[test]
    fn test_get_completion_info() {
        let plugin = create_plugin();
        let info = plugin.get_completion_info();

        assert!(!info.keywords.is_empty());
        assert!(!info.functions.is_empty());
        assert!(!info.operators.is_empty());
        assert!(!info.data_types.is_empty());
        assert!(!info.snippets.is_empty());

        assert!(info.keywords.iter().any(|(k, _)| *k == "RETURNING"));
        assert!(info
            .functions
            .iter()
            .any(|(f, _)| f.starts_with("ARRAY_AGG")));
    }
}
