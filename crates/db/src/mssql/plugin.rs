use std::collections::HashMap;

use anyhow::Result;
use gpui_component::table::Column;
use one_core::storage::{DatabaseType, DbConnectionConfig};
use regex::Regex;
use tracing::info;

use crate::connection::{DbConnection, DbError};
use crate::executor::SqlResult;
use crate::import_export::{
    ExportConfig, ExportProgressSender, ExportResult, ImportConfig, ImportProgressSender,
    ImportResult,
};
use crate::mssql::connection::MssqlDbConnection;
use crate::plugin::{DatabasePlugin, SqlCompletionInfo};
use crate::types::*;

/// MSSQL data types (name, description)
pub const MSSQL_DATA_TYPES: &[(&str, &str)] = &[
    ("BIT", "Boolean (0/1)"),
    ("TINYINT", "1 byte integer (0-255)"),
    ("SMALLINT", "2 byte integer"),
    ("INT", "4 byte integer"),
    ("BIGINT", "8 byte integer"),
    ("DECIMAL", "Fixed-point number"),
    ("NUMERIC", "Fixed-point number"),
    ("MONEY", "Currency (8 bytes)"),
    ("SMALLMONEY", "Currency (4 bytes)"),
    ("FLOAT", "Floating point"),
    ("REAL", "Single-precision float"),
    ("CHAR", "Fixed-length string"),
    ("VARCHAR", "Variable-length string"),
    ("TEXT", "Large text (deprecated)"),
    ("NCHAR", "Fixed-length Unicode string"),
    ("NVARCHAR", "Variable-length Unicode string"),
    ("NTEXT", "Large Unicode text (deprecated)"),
    ("BINARY", "Fixed-length binary"),
    ("VARBINARY", "Variable-length binary"),
    ("IMAGE", "Large binary (deprecated)"),
    ("DATE", "Date only"),
    ("TIME", "Time only"),
    ("DATETIME", "Date and time (legacy)"),
    ("DATETIME2", "High precision datetime"),
    ("SMALLDATETIME", "Low precision datetime"),
    ("DATETIMEOFFSET", "Datetime with timezone"),
    ("TIMESTAMP", "Row version number"),
    ("UNIQUEIDENTIFIER", "GUID"),
    ("XML", "XML document"),
    ("SQL_VARIANT", "Variable type"),
    ("GEOGRAPHY", "Spatial geography data"),
    ("GEOMETRY", "Spatial geometry data"),
];

/// Fix MSSQL bracket identifier spacing issue
/// sql format formats [name] as [ name ], this fixes it back
fn fix_mssql_brackets(sql: &str) -> String {
    let re = Regex::new(r"\[\s*([^\]]+?)\s*\]").expect("Invalid regex");
    re.replace_all(sql, "[$1]").to_string()
}

/// MSSQL database plugin implementation (stateless)
pub struct MsSqlPlugin;

impl MsSqlPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl DatabasePlugin for MsSqlPlugin {
    fn name(&self) -> DatabaseType {
        DatabaseType::MSSQL
    }

    fn quote_identifier(&self, identifier: &str) -> String {
        format!("[{}]", identifier.replace("]", "]]"))
    }

    fn get_completion_info(&self) -> SqlCompletionInfo {
        SqlCompletionInfo {
            keywords: vec![
                // MSSQL-specific keywords
                ("IDENTITY", "Auto-increment column"),
                ("CLUSTERED", "Clustered index"),
                ("NONCLUSTERED", "Non-clustered index"),
                ("FILEGROUP", "Filegroup specification"),
                ("GO", "Batch separator"),
                ("TRY", "Begin try block"),
                ("CATCH", "Begin catch block"),
                ("THROW", "Throw exception"),
                ("RAISERROR", "Raise error"),
                ("EXEC", "Execute stored procedure"),
                ("EXECUTE", "Execute SQL statement"),
                ("WAITFOR", "Wait for time/statement"),
                ("TOP", "Limit rows"),
                ("OFFSET", "Skip rows"),
                ("FETCH", "Fetch rows"),
                ("PARTITION BY", "Partition window function"),
                ("PIVOT", "Pivot rows to columns"),
                ("UNPIVOT", "Unpivot columns to rows"),
                ("CROSS APPLY", "Apply right expression for each left row"),
                ("OUTER APPLY", "Outer apply"),
                ("MERGE", "Merge statement"),
                ("OUTPUT", "Output clause"),
                ("INSERTED", "Inserted pseudo table"),
                ("DELETED", "Deleted pseudo table"),
                ("OVER", "Window function"),
                ("ROW_NUMBER", "Row number window function"),
            ],
            functions: vec![
                // MSSQL-specific functions
                ("LEN(str)", "String length"),
                ("DATALENGTH(expr)", "Data length in bytes"),
                ("CHARINDEX(substr, str)", "Find substring position"),
                ("PATINDEX(pattern, str)", "Pattern index"),
                ("STUFF(str, start, len, new)", "Replace part of string"),
                ("REPLICATE(str, count)", "Repeat string"),
                ("QUOTENAME(str)", "Quote identifier"),
                ("FORMAT(value, format)", "Format value"),
                ("ISNULL(expr, alt)", "Return alt if expr is NULL"),
                ("COALESCE(expr1, expr2, ...)", "Return first non-NULL"),
                ("NULLIF(expr1, expr2)", "Return NULL if equal"),
                ("IIF(cond, then, else)", "Conditional expression"),
                ("CHOOSE(index, val1, val2, ...)", "Choose value by index"),
                ("CAST(expr AS type)", "Type conversion"),
                ("CONVERT(type, expr)", "Type conversion"),
                ("TRY_CAST(expr AS type)", "Safe type conversion"),
                ("TRY_CONVERT(type, expr)", "Safe type conversion"),
                ("GETDATE()", "Current datetime"),
                ("GETUTCDATE()", "Current UTC datetime"),
                ("SYSDATETIME()", "High precision datetime"),
                ("DATEADD(part, num, date)", "Add interval to date"),
                ("DATEDIFF(part, date1, date2)", "Difference between dates"),
                ("DATEPART(part, date)", "Extract date part"),
                ("YEAR(date)", "Extract year"),
                ("MONTH(date)", "Extract month"),
                ("DAY(date)", "Extract day"),
                ("EOMONTH(date)", "End of month"),
                ("DATEFROMPARTS(y,m,d)", "Create date from parts"),
                ("ROUND(num, decimals)", "Round number"),
                ("CEILING(num)", "Ceiling function"),
                ("FLOOR(num)", "Floor function"),
                ("ABS(num)", "Absolute value"),
                ("POWER(x, y)", "Power function"),
                ("SQRT(num)", "Square root"),
                ("RAND()", "Random number 0-1"),
                ("NEWID()", "Generate GUID"),
                ("NEWSEQUENTIALID()", "Generate sequential GUID"),
                ("SCOPE_IDENTITY()", "Last identity value"),
                ("@@IDENTITY", "Last identity value"),
                ("@@ROWCOUNT", "Affected rows count"),
                ("@@VERSION", "SQL Server version"),
                ("DB_NAME()", "Current database name"),
                ("USER_NAME()", "Current user name"),
                ("OBJECT_NAME(id)", "Object name by ID"),
                ("OBJECT_ID(name)", "Object ID by name"),
                ("STRING_AGG(col, sep)", "Aggregate strings"),
                ("STRING_SPLIT(str, sep)", "Split string to table"),
                ("JSON_VALUE(json, path)", "Extract JSON scalar"),
                ("JSON_QUERY(json, path)", "Extract JSON object/array"),
                ("OPENJSON(json)", "Parse JSON to table"),
                ("FOR JSON", "Format result as JSON"),
                ("OPENXML(doc, xpath)", "Parse XML to table"),
                ("ISJSON(expr)", "Check if valid JSON"),
            ],
            operators: vec![
                ("+=", "Add and assign"),
                ("-=", "Subtract and assign"),
                ("*=", "Multiply and assign"),
                ("/=", "Divide and assign"),
                ("%=", "Modulo and assign"),
                ("!=", "Not equal"),
                ("!<", "Not less than"),
                ("!>", "Not greater than"),
            ],
            data_types: MSSQL_DATA_TYPES.to_vec(),
            snippets: vec![
                (
                    "crt",
                    "CREATE TABLE $1 (\n  id INT IDENTITY(1,1) PRIMARY KEY,\n  $2\n)",
                    "Create table",
                ),
                ("idx", "CREATE INDEX $1 ON $2 ($3)", "Create index"),
                ("alt", "ALTER TABLE $1 ADD $2", "Add column"),
                ("jn", "JOIN $1 ON $2.$3 = $4.$5", "Join clause"),
                ("lj", "LEFT JOIN $1 ON $2.$3 = $4.$5", "Left join clause"),
                (
                    "sp",
                    "CREATE PROCEDURE $1\nAS\nBEGIN\n  $2\nEND",
                    "Create stored procedure",
                ),
                (
                    "try",
                    "BEGIN TRY\n  $1\nEND TRY\nBEGIN CATCH\n  SELECT ERROR_MESSAGE()\nEND CATCH",
                    "Try-catch block",
                ),
            ],
        }
        .with_standard_sql()
    }

    async fn create_connection(
        &self,
        config: DbConnectionConfig,
    ) -> Result<Box<dyn DbConnection + Send + Sync>, DbError> {
        info!(
            "[MSSQL Plugin] Creating connection to {}:{}",
            config.host, config.port
        );
        let mut conn = MssqlDbConnection::new(config);
        conn.connect().await?;
        info!("[MSSQL Plugin] Connection created successfully");
        Ok(Box::new(conn))
    }

    async fn list_databases(&self, connection: &dyn DbConnection) -> Result<Vec<String>> {
        info!("[MSSQL Plugin] Listing databases...");
        let result = connection.query(
            "SELECT name FROM sys.databases WHERE name NOT IN ('master', 'tempdb', 'model', 'msdb') ORDER BY name"
        ).await.map_err(|e| anyhow::anyhow!("Failed to list databases: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let databases: Vec<String> = query_result
                .rows
                .iter()
                .filter_map(|row| row.first().and_then(|v| v.clone()))
                .collect();
            info!("[MSSQL Plugin] Found {} databases", databases.len());
            Ok(databases)
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    async fn list_databases_view(&self, connection: &dyn DbConnection) -> Result<ObjectView> {
        use gpui::px;

        let sql = r#"
            SELECT
                d.name,
                SUSER_SNAME(d.owner_sid) as owner,
                d.create_date,
                d.compatibility_level,
                d.collation_name
            FROM sys.databases d
            WHERE d.name NOT IN ('master', 'tempdb', 'model', 'msdb')
            ORDER BY d.name
        "#;

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list databases: {}", e))?;

        let rows: Vec<Vec<String>> = if let SqlResult::Query(query_result) = result {
            query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(3)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(4)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                    ]
                })
                .collect()
        } else {
            vec![]
        };

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("owner", "Owner").width(px(120.0)),
            Column::new("created", "Created").width(px(180.0)),
            Column::new("compat_level", "Compat Level").width(px(100.0)),
            Column::new("collation", "Collation").width(px(200.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Database,
            title: "Databases".to_string(),
        })
    }

    async fn list_databases_detailed(
        &self,
        connection: &dyn DbConnection,
    ) -> Result<Vec<DatabaseInfo>> {
        let sql = r#"
            SELECT
                d.name,
                SUSER_SNAME(d.owner_sid) as owner,
                d.create_date,
                d.collation_name,
                COUNT(t.name) as table_count
            FROM sys.databases d
            LEFT JOIN sys.tables t ON d.database_id = DB_ID(d.name)
            WHERE d.name NOT IN ('master', 'tempdb', 'model', 'msdb')
            GROUP BY d.name, d.owner_sid, d.create_date, d.collation_name
            ORDER BY d.name
        "#;

        let result = connection
            .query(sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list databases: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| DatabaseInfo {
                    name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                    charset: None,
                    collation: row.get(3).and_then(|v| v.clone()),
                    size: None,
                    table_count: row
                        .get(4)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse().ok()),
                    comment: None,
                })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    fn supports_schema(&self) -> bool {
        true
    }

    fn supports_sequences(&self) -> bool {
        true
    }

    fn sql_dialect(&self) -> Box<dyn sqlparser::dialect::Dialect> {
        Box::new(sqlparser::dialect::MsSqlDialect {})
    }

    fn format_sql(&self, sql: &str) -> String {
        let formatted = crate::format_sql(sql);
        fix_mssql_brackets(&formatted)
    }

    async fn list_schemas(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<String>> {
        let sql = format!(
            r#"
            SELECT s.name
            FROM [{database}].sys.schemas s
            WHERE s.name NOT IN (
                'INFORMATION_SCHEMA', 'sys',
                'db_owner', 'db_accessadmin', 'db_securityadmin', 'db_ddladmin',
                'db_backupoperator', 'db_datareader', 'db_datawriter',
                'db_denydatareader', 'db_denydatawriter'
            )
            ORDER BY s.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
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

    // === Database/Schema Level Operations ===

    async fn list_schemas_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sql = format!(
            r#"
            SELECT
                s.name AS schema_name,
                dp.name AS owner,
                (SELECT COUNT(*) FROM [{database}].sys.tables t WHERE t.schema_id = s.schema_id) AS table_count
            FROM [{database}].sys.schemas s
            LEFT JOIN [{database}].sys.database_principals dp ON s.principal_id = dp.principal_id
            WHERE s.name NOT IN (
                'INFORMATION_SCHEMA', 'sys',
                'db_owner', 'db_accessadmin', 'db_securityadmin', 'db_ddladmin',
                'db_backupoperator', 'db_datareader', 'db_datawriter',
                'db_denydatareader', 'db_denydatawriter'
            )
            ORDER BY s.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list schemas: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let columns = vec![
                Column::new("name", "Name").width(px(180.0)),
                Column::new("owner", "Owner").width(px(120.0)),
                Column::new("tables", "Tables").width(px(80.0)).text_right(),
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
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<TableInfo>> {
        let schema_filter = match &schema {
            Some(s) => format!("AND s.name = '{}'", s.replace("'", "''")),
            None => String::new(),
        };
        let sql = format!(
            r#"
            SELECT
                t.name AS table_name,
                s.name AS schema_name,
                CAST(ep.value AS NVARCHAR(MAX)) AS table_comment,
                t.create_date
            FROM [{database}].sys.tables t
            INNER JOIN [{database}].sys.schemas s ON t.schema_id = s.schema_id
            LEFT JOIN [{database}].sys.extended_properties ep
                ON ep.major_id = t.object_id
                AND ep.minor_id = 0
                AND ep.name = 'MS_Description'
            WHERE 1=1 {schema_filter}
            ORDER BY s.name, t.name
            "#,
            database = database.replace("]", "]]"),
            schema_filter = schema_filter
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list tables: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| TableInfo {
                    name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                    schema: row.get(1).and_then(|v| v.clone()),
                    comment: row.get(2).and_then(|v| v.clone()),
                    engine: None,
                    row_count: None,
                    create_time: row.get(3).and_then(|v| v.clone()),
                    charset: None,
                    collation: None,
                })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    async fn list_tables_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<ObjectView> {
        use gpui::px;

        let schema_filter = match &schema {
            Some(s) => format!("AND s.name = '{}'", s.replace("'", "''")),
            None => String::new(),
        };
        let sql = format!(
            r#"
            SELECT
                t.name AS table_name,
                s.name AS schema_name,
                CAST(ep.value AS NVARCHAR(MAX)) AS table_comment,
                t.create_date
            FROM [{database}].sys.tables t
            INNER JOIN [{database}].sys.schemas s ON t.schema_id = s.schema_id
            LEFT JOIN [{database}].sys.extended_properties ep
                ON ep.major_id = t.object_id
                AND ep.minor_id = 0
                AND ep.name = 'MS_Description'
            WHERE 1=1 {schema_filter}
            ORDER BY s.name, t.name
            "#,
            database = database.replace("]", "]]"),
            schema_filter = schema_filter
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list tables: {}", e))?;

        let rows: Vec<Vec<String>> = if let SqlResult::Query(query_result) = result {
            query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(3)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                    ]
                })
                .collect()
        } else {
            vec![]
        };

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("schema", "Schema").width(px(100.0)),
            Column::new("comment", "Comment").width(px(250.0)),
            Column::new("created", "Created").width(px(150.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Table,
            title: "Tables".to_string(),
        })
    }

    async fn list_columns(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<ColumnInfo>> {
        let schema_val = schema.unwrap_or_else(|| "dbo".to_string());
        let sql = format!(
            r#"
            SELECT
                c.name AS column_name,
                ty.name + CASE
                    WHEN ty.name IN ('varchar', 'nvarchar', 'char', 'nchar', 'binary', 'varbinary')
                        THEN '(' + CASE WHEN c.max_length = -1 THEN 'MAX' ELSE CAST(CASE WHEN ty.name LIKE 'n%' THEN c.max_length/2 ELSE c.max_length END AS VARCHAR) END + ')'
                    WHEN ty.name IN ('decimal', 'numeric')
                        THEN '(' + CAST(c.precision AS VARCHAR) + ',' + CAST(c.scale AS VARCHAR) + ')'
                    ELSE ''
                END AS data_type,
                c.is_nullable,
                dc.definition AS default_value,
                c.is_identity,
                CAST(ep.value AS NVARCHAR(MAX)) AS column_comment,
                CASE WHEN ic.column_id IS NOT NULL THEN 1 ELSE 0 END AS is_primary_key
            FROM [{database}].sys.columns c
            INNER JOIN [{database}].sys.tables t ON c.object_id = t.object_id
            INNER JOIN [{database}].sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN [{database}].sys.types ty ON c.user_type_id = ty.user_type_id
            LEFT JOIN [{database}].sys.default_constraints dc ON c.default_object_id = dc.object_id
            LEFT JOIN [{database}].sys.extended_properties ep
                ON ep.major_id = c.object_id
                AND ep.minor_id = c.column_id
                AND ep.name = 'MS_Description'
            LEFT JOIN [{database}].sys.indexes i
                ON i.object_id = t.object_id AND i.is_primary_key = 1
            LEFT JOIN [{database}].sys.index_columns ic
                ON ic.object_id = i.object_id AND ic.index_id = i.index_id AND ic.column_id = c.column_id
            WHERE s.name = '{schema}' AND t.name = '{table}'
            ORDER BY c.column_id
            "#,
            database = database.replace("]", "]]"),
            schema = schema_val.replace("'", "''"),
            table = table.replace("'", "''")
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
                    let is_nullable = row
                        .get(2)
                        .and_then(|v| v.clone())
                        .map(|v| v == "1" || v.to_lowercase() == "true")
                        .unwrap_or(true);
                    let is_primary_key = row
                        .get(6)
                        .and_then(|v| v.clone())
                        .map(|v| v == "1")
                        .unwrap_or(false);
                    ColumnInfo {
                        name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        data_type: row.get(1).and_then(|v| v.clone()).unwrap_or_default(),
                        is_nullable,
                        is_primary_key,
                        default_value: row.get(3).and_then(|v| v.clone()),
                        comment: row.get(5).and_then(|v| v.clone()),
                        charset: None,
                        collation: None,
                    }
                })
                .collect())
        } else {
            Ok(vec![])
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

        let rows: Vec<Vec<String>> = columns_data
            .iter()
            .map(|col| {
                vec![
                    col.name.clone(),
                    col.data_type.clone(),
                    if col.is_nullable { "YES" } else { "NO" }.to_string(),
                    col.default_value.as_deref().unwrap_or("-").to_string(),
                    col.comment.as_deref().unwrap_or("-").to_string(),
                ]
            })
            .collect();

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("type", "Type").width(px(120.0)),
            Column::new("nullable", "Null").width(px(60.0)),
            Column::new("default", "Default").width(px(120.0)),
            Column::new("comment", "Comment").width(px(250.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Column,
            title: format!("Columns - {}", table),
        })
    }

    async fn list_indexes(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<IndexInfo>> {
        let schema_val = schema.unwrap_or_else(|| "dbo".to_string());
        let sql = format!(
            r#"
            SELECT
                i.name as index_name,
                COL_NAME(ic.object_id, ic.column_id) as column_name,
                i.type_desc as index_type,
                i.is_unique
            FROM [{database}].sys.indexes i
            INNER JOIN [{database}].sys.index_columns ic
                ON i.object_id = ic.object_id AND i.index_id = ic.index_id
            WHERE i.object_id = OBJECT_ID('[{database}].[{schema}].[{table}]')
                AND i.type > 0
                AND i.is_primary_key = 0
            ORDER BY i.name, ic.key_ordinal
            "#,
            database = database.replace("]", "]]"),
            schema = schema_val.replace("'", "''"),
            table = table.replace("'", "''")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list indexes: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            let mut indexes: HashMap<String, IndexInfo> = HashMap::new();

            for row in &query_result.rows {
                let index_name = row.get(0).and_then(|v| v.clone()).unwrap_or_default();
                let column_name = row.get(1).and_then(|v| v.clone()).unwrap_or_default();
                let index_type = row.get(2).and_then(|v| v.clone()).unwrap_or_default();
                let is_unique = row
                    .get(3)
                    .and_then(|v| v.clone())
                    .unwrap_or("0".to_string())
                    == "1";

                indexes
                    .entry(index_name.clone())
                    .or_insert_with(|| IndexInfo {
                        name: index_name.clone(),
                        columns: vec![],
                        is_unique,
                        index_type: Some(index_type),
                    })
                    .columns
                    .push(column_name);
            }

            Ok(indexes.into_values().collect())
        } else {
            Ok(vec![])
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

        let rows: Vec<Vec<String>> = indexes
            .iter()
            .map(|idx| {
                vec![
                    idx.name.clone(),
                    idx.columns.join(", "),
                    idx.index_type.as_deref().unwrap_or("-").to_string(),
                    if idx.is_unique { "Yes" } else { "No" }.to_string(),
                ]
            })
            .collect();

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("columns", "Columns").width(px(250.0)),
            Column::new("type", "Type").width(px(150.0)),
            Column::new("unique", "Unique").width(px(80.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Index,
            title: format!("Indexes - {}", table),
        })
    }

    async fn list_views(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<ViewInfo>> {
        let schema_filter = match &schema {
            Some(s) => format!("AND s.name = '{}'", s.replace("'", "''")),
            None => String::new(),
        };
        let sql = format!(
            r#"
            SELECT
                v.name AS view_name,
                s.name AS schema_name,
                CAST(ep.value AS NVARCHAR(MAX)) AS view_comment
            FROM [{database}].sys.views v
            INNER JOIN [{database}].sys.schemas s ON v.schema_id = s.schema_id
            LEFT JOIN [{database}].sys.extended_properties ep
                ON ep.major_id = v.object_id
                AND ep.minor_id = 0
                AND ep.name = 'MS_Description'
            WHERE 1=1 {schema_filter}
            ORDER BY s.name, v.name
            "#,
            database = database.replace("]", "]]"),
            schema_filter = schema_filter
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
                    name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                    schema: row.get(1).and_then(|v| v.clone()),
                    definition: None,
                    comment: row.get(2).and_then(|v| v.clone()),
                })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    async fn list_views_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sql = format!(
            r#"
            SELECT
                v.name AS view_name,
                s.name AS schema_name,
                CAST(ep.value AS NVARCHAR(MAX)) AS view_comment,
                v.create_date
            FROM [{database}].sys.views v
            INNER JOIN [{database}].sys.schemas s ON v.schema_id = s.schema_id
            LEFT JOIN [{database}].sys.extended_properties ep
                ON ep.major_id = v.object_id
                AND ep.minor_id = 0
                AND ep.name = 'MS_Description'
            ORDER BY s.name, v.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list views: {}", e))?;

        let rows: Vec<Vec<String>> = if let SqlResult::Query(query_result) = result {
            query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(3)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                    ]
                })
                .collect()
        } else {
            vec![]
        };

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("schema", "Schema").width(px(100.0)),
            Column::new("comment", "Comment").width(px(250.0)),
            Column::new("created", "Created").width(px(150.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::View,
            title: "Views".to_string(),
        })
    }

    async fn list_functions(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let sql = format!(
            r#"
            SELECT
                o.name AS function_name,
                s.name AS schema_name,
                CASE o.type
                    WHEN 'FN' THEN 'Scalar'
                    WHEN 'IF' THEN 'Inline Table'
                    WHEN 'TF' THEN 'Table'
                    WHEN 'FS' THEN 'CLR Scalar'
                    WHEN 'FT' THEN 'CLR Table'
                    ELSE o.type_desc
                END AS function_type,
                CAST(ep.value AS NVARCHAR(MAX)) AS function_comment
            FROM [{database}].sys.objects o
            INNER JOIN [{database}].sys.schemas s ON o.schema_id = s.schema_id
            LEFT JOIN [{database}].sys.extended_properties ep
                ON ep.major_id = o.object_id
                AND ep.minor_id = 0
                AND ep.name = 'MS_Description'
            WHERE o.type IN ('FN', 'IF', 'TF', 'FS', 'FT')
            ORDER BY s.name, o.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list functions: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| FunctionInfo {
                    name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                    return_type: row.get(2).and_then(|v| v.clone()),
                    parameters: vec![],
                    definition: None,
                    comment: row.get(3).and_then(|v| v.clone()),
                })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    async fn list_functions_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sql = format!(
            r#"
            SELECT
                o.name AS function_name,
                s.name AS schema_name,
                CASE o.type
                    WHEN 'FN' THEN 'Scalar'
                    WHEN 'IF' THEN 'Inline Table'
                    WHEN 'TF' THEN 'Table'
                    WHEN 'FS' THEN 'CLR Scalar'
                    WHEN 'FT' THEN 'CLR Table'
                    ELSE o.type_desc
                END AS function_type,
                o.create_date
            FROM [{database}].sys.objects o
            INNER JOIN [{database}].sys.schemas s ON o.schema_id = s.schema_id
            WHERE o.type IN ('FN', 'IF', 'TF', 'FS', 'FT')
            ORDER BY s.name, o.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list functions: {}", e))?;

        let rows: Vec<Vec<String>> = if let SqlResult::Query(query_result) = result {
            query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(3)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                    ]
                })
                .collect()
        } else {
            vec![]
        };

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("schema", "Schema").width(px(100.0)),
            Column::new("type", "Type").width(px(120.0)),
            Column::new("created", "Created").width(px(150.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Function,
            title: "Functions".to_string(),
        })
    }

    async fn list_procedures(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let sql = format!(
            r#"
            SELECT
                p.name AS procedure_name,
                s.name AS schema_name,
                CAST(ep.value AS NVARCHAR(MAX)) AS procedure_comment
            FROM [{database}].sys.procedures p
            INNER JOIN [{database}].sys.schemas s ON p.schema_id = s.schema_id
            LEFT JOIN [{database}].sys.extended_properties ep
                ON ep.major_id = p.object_id
                AND ep.minor_id = 0
                AND ep.name = 'MS_Description'
            ORDER BY s.name, p.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list procedures: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| FunctionInfo {
                    name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                    return_type: None,
                    parameters: vec![],
                    definition: None,
                    comment: row.get(2).and_then(|v| v.clone()),
                })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    // === Missing trait methods ===

    async fn list_procedures_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sql = format!(
            r#"
            SELECT
                p.name AS procedure_name,
                s.name AS schema_name,
                p.create_date,
                p.modify_date
            FROM [{database}].sys.procedures p
            INNER JOIN [{database}].sys.schemas s ON p.schema_id = s.schema_id
            ORDER BY s.name, p.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list procedures: {}", e))?;

        let rows: Vec<Vec<String>> = if let SqlResult::Query(query_result) = result {
            query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(3)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                    ]
                })
                .collect()
        } else {
            vec![]
        };

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("schema", "Schema").width(px(100.0)),
            Column::new("created", "Created").width(px(150.0)),
            Column::new("modified", "Modified").width(px(150.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Procedure,
            title: "Stored Procedures".to_string(),
        })
    }

    async fn list_triggers(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<TriggerInfo>> {
        let sql = format!(
            r#"
            SELECT
                tr.name as trigger_name,
                OBJECT_NAME(tr.parent_id) as table_name,
                tr.is_disabled
            FROM [{database}].sys.triggers tr
            WHERE tr.parent_class = 1
            ORDER BY tr.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list triggers: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| TriggerInfo {
                    name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                    table_name: row.get(1).and_then(|v| v.clone()).unwrap_or_default(),
                    event: "UNKNOWN".to_string(),
                    timing: "UNKNOWN".to_string(),
                    definition: None,
                })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    async fn list_triggers_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sql = format!(
            r#"
            SELECT
                tr.name as trigger_name,
                OBJECT_NAME(tr.parent_id) as table_name,
                tr.is_disabled
            FROM [{database}].sys.triggers tr
            WHERE tr.parent_class = 1
            ORDER BY tr.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list triggers: {}", e))?;

        let rows: Vec<Vec<String>> = if let SqlResult::Query(query_result) = result {
            query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .map(|v| if v == "0" { "Enabled" } else { "Disabled" }.to_string())
                            .unwrap_or("Unknown".to_string()),
                    ]
                })
                .collect()
        } else {
            vec![]
        };

        let columns = vec![
            Column::new("name", "Name").width(px(250.0)),
            Column::new("table", "Table").width(px(200.0)),
            Column::new("status", "Status").width(px(100.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Trigger,
            title: "Triggers".to_string(),
        })
    }

    async fn list_sequences(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        _schema: Option<String>,
    ) -> Result<Vec<SequenceInfo>> {
        let sql = format!(
            r#"
            SELECT
                s.name,
                TYPE_NAME(s.user_type_id) as data_type,
                CAST(s.start_value AS VARCHAR) as start_value,
                CAST(s.increment AS VARCHAR) as increment,
                CAST(s.current_value AS VARCHAR) as current_value
            FROM [{database}].sys.sequences s
            ORDER BY s.name
            "#,
            database = database.replace("]", "]]")
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
                    name: row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                    start_value: row
                        .get(2)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse().ok()),
                    increment: row
                        .get(3)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse().ok()),
                    min_value: None,
                    max_value: None,
                })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    async fn list_sequences_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView> {
        use gpui::px;

        let sql = format!(
            r#"
            SELECT
                s.name,
                TYPE_NAME(s.user_type_id) as data_type,
                CAST(s.start_value AS VARCHAR) as start_value,
                CAST(s.increment AS VARCHAR) as increment,
                CAST(s.current_value AS VARCHAR) as current_value
            FROM [{database}].sys.sequences s
            ORDER BY s.name
            "#,
            database = database.replace("]", "]]")
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list sequences: {}", e))?;

        let rows: Vec<Vec<String>> = if let SqlResult::Query(query_result) = result {
            query_result
                .rows
                .iter()
                .map(|row| {
                    vec![
                        row.get(0).and_then(|v| v.clone()).unwrap_or_default(),
                        row.get(1)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(2)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(3)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                        row.get(4)
                            .and_then(|v| v.clone())
                            .unwrap_or("-".to_string()),
                    ]
                })
                .collect()
        } else {
            vec![]
        };

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("type", "Type").width(px(100.0)),
            Column::new("start", "Start").width(px(100.0)),
            Column::new("increment", "Increment").width(px(100.0)),
            Column::new("current", "Current").width(px(100.0)),
        ];

        Ok(ObjectView {
            columns,
            rows,
            db_node_type: DbNodeType::Sequence,
            title: "Sequences".to_string(),
        })
    }

    fn get_data_types(&self) -> &[(&'static str, &'static str)] {
        MSSQL_DATA_TYPES
    }

    fn get_charsets(&self) -> Vec<CharsetInfo> {
        vec![
            CharsetInfo {
                name: "Chinese_PRC".into(),
                description: "简体中文 (GBK)".into(),
                default_collation: "Chinese_PRC_CI_AS".into(),
            },
            CharsetInfo {
                name: "Chinese_Taiwan".into(),
                description: "繁体中文 (Big5)".into(),
                default_collation: "Chinese_Taiwan_Stroke_CI_AS".into(),
            },
            CharsetInfo {
                name: "Latin1_General".into(),
                description: "Latin1 General (CP1252)".into(),
                default_collation: "Latin1_General_CI_AS".into(),
            },
            CharsetInfo {
                name: "Latin1_General_100".into(),
                description: "Latin1 General 100 (Unicode)".into(),
                default_collation: "Latin1_General_100_CI_AS_SC".into(),
            },
            CharsetInfo {
                name: "SQL_Latin1_General".into(),
                description: "SQL Latin1 General (CP1252)".into(),
                default_collation: "SQL_Latin1_General_CP1_CI_AS".into(),
            },
            CharsetInfo {
                name: "Japanese".into(),
                description: "日本語 (CP932)".into(),
                default_collation: "Japanese_CI_AS".into(),
            },
            CharsetInfo {
                name: "Korean_Wansung".into(),
                description: "한국어 (EUC-KR)".into(),
                default_collation: "Korean_Wansung_CI_AS".into(),
            },
        ]
    }

    fn get_collations(&self, charset: &str) -> Vec<CollationInfo> {
        match charset {
            "Chinese_PRC" => vec![
                CollationInfo {
                    name: "Chinese_PRC_CI_AS".into(),
                    charset: "Chinese_PRC".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "Chinese_PRC_CS_AS".into(),
                    charset: "Chinese_PRC".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Chinese_PRC_CI_AI".into(),
                    charset: "Chinese_PRC".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Chinese_PRC_BIN".into(),
                    charset: "Chinese_PRC".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Chinese_PRC_BIN2".into(),
                    charset: "Chinese_PRC".into(),
                    is_default: false,
                },
            ],
            "Chinese_Taiwan" => vec![
                CollationInfo {
                    name: "Chinese_Taiwan_Stroke_CI_AS".into(),
                    charset: "Chinese_Taiwan".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "Chinese_Taiwan_Stroke_CS_AS".into(),
                    charset: "Chinese_Taiwan".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Chinese_Taiwan_Bopomofo_CI_AS".into(),
                    charset: "Chinese_Taiwan".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Chinese_Taiwan_Stroke_BIN".into(),
                    charset: "Chinese_Taiwan".into(),
                    is_default: false,
                },
            ],
            "Latin1_General" => vec![
                CollationInfo {
                    name: "Latin1_General_CI_AS".into(),
                    charset: "Latin1_General".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "Latin1_General_CS_AS".into(),
                    charset: "Latin1_General".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Latin1_General_CI_AI".into(),
                    charset: "Latin1_General".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Latin1_General_BIN".into(),
                    charset: "Latin1_General".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Latin1_General_BIN2".into(),
                    charset: "Latin1_General".into(),
                    is_default: false,
                },
            ],
            "Latin1_General_100" => vec![
                CollationInfo {
                    name: "Latin1_General_100_CI_AS_SC".into(),
                    charset: "Latin1_General_100".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "Latin1_General_100_CS_AS_SC".into(),
                    charset: "Latin1_General_100".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Latin1_General_100_CI_AS_SC_UTF8".into(),
                    charset: "Latin1_General_100".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Latin1_General_100_BIN2".into(),
                    charset: "Latin1_General_100".into(),
                    is_default: false,
                },
            ],
            "SQL_Latin1_General" => vec![
                CollationInfo {
                    name: "SQL_Latin1_General_CP1_CI_AS".into(),
                    charset: "SQL_Latin1_General".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "SQL_Latin1_General_CP1_CS_AS".into(),
                    charset: "SQL_Latin1_General".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "SQL_Latin1_General_CP1_CI_AI".into(),
                    charset: "SQL_Latin1_General".into(),
                    is_default: false,
                },
            ],
            "Japanese" => vec![
                CollationInfo {
                    name: "Japanese_CI_AS".into(),
                    charset: "Japanese".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "Japanese_CS_AS".into(),
                    charset: "Japanese".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Japanese_BIN".into(),
                    charset: "Japanese".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Japanese_BIN2".into(),
                    charset: "Japanese".into(),
                    is_default: false,
                },
            ],
            "Korean_Wansung" => vec![
                CollationInfo {
                    name: "Korean_Wansung_CI_AS".into(),
                    charset: "Korean_Wansung".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "Korean_Wansung_CS_AS".into(),
                    charset: "Korean_Wansung".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Korean_Wansung_BIN".into(),
                    charset: "Korean_Wansung".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "Korean_Wansung_BIN2".into(),
                    charset: "Korean_Wansung".into(),
                    is_default: false,
                },
            ],
            _ => vec![],
        }
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
        } else {
            def.push_str(" NULL");
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
        let db_name = &request.database_name;
        let collation = request.field_values.get("collation").map(|s| s.as_str());

        let mut sql = format!("CREATE DATABASE [{}]", db_name.replace("]", "]]"));
        if let Some(coll) = collation {
            sql.push_str(&format!(" COLLATE {}", coll));
        }
        sql.push(';');
        sql
    }

    fn build_modify_database_sql(
        &self,
        request: &crate::plugin::DatabaseOperationRequest,
    ) -> String {
        let db_name = &request.database_name;
        let collation = request.field_values.get("collation").map(|s| s.as_str());

        let mut sql = format!("ALTER DATABASE [{}]", db_name.replace("]", "]]"));
        if let Some(coll) = collation {
            sql.push_str(&format!(" COLLATE {}", coll));
        }
        sql.push(';');
        sql
    }

    fn build_drop_database_sql(&self, database_name: &str) -> String {
        format!("DROP DATABASE [{}];", database_name.replace("]", "]]"))
    }

    fn build_create_schema_sql(&self, schema_name: &str) -> String {
        format!("CREATE SCHEMA [{}];", schema_name.replace("]", "]]"))
    }

    fn build_drop_schema_sql(&self, schema_name: &str) -> String {
        format!("DROP SCHEMA [{}];", schema_name.replace("]", "]]"))
    }

    fn build_comment_schema_sql(&self, schema_name: &str, comment: &str) -> Option<String> {
        Some(format!(
            "EXEC sp_addextendedproperty @name=N'MS_Description', @value=N'{}', @level0type=N'SCHEMA', @level0name=N'{}';",
            comment.replace("'", "''"),
            schema_name.replace("'", "''")
        ))
    }

    fn format_pagination(&self, limit: usize, offset: usize, order_clause: &str) -> String {
        if order_clause.is_empty() {
            format!(
                " ORDER BY (SELECT NULL) OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
                offset, limit
            )
        } else {
            format!(" OFFSET {} ROWS FETCH NEXT {} ROWS ONLY", offset, limit)
        }
    }

    fn format_table_reference(&self, database: &str, schema: Option<&str>, table: &str) -> String {
        match schema {
            Some(s) => format!(
                "{}.{}.{}",
                self.quote_identifier(database),
                self.quote_identifier(s),
                self.quote_identifier(table)
            ),
            None => format!(
                "{}..{}",
                self.quote_identifier(database),
                self.quote_identifier(table)
            ),
        }
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
        (where_clause, String::new())
    }

    fn drop_table(&self, _database: &str, schema: Option<&str>, table: &str) -> String {
        // SQL Server uses schema.table format, database is typically set via USE statement
        // SQL Server 2016+ supports IF EXISTS
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
            "EXEC sp_rename '{}', '{}'",
            old_name.replace("'", "''"),
            new_name.replace("'", "''")
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
        format!(
            "SELECT * INTO {} FROM {};",
            qualify(target_table),
            qualify(source_table)
        )
    }

    fn build_column_def(&self, col: &ColumnDefinition) -> String {
        let mut def = String::new();
        def.push_str(&format!("[{}]", col.name.replace("]", "]]")));
        def.push(' ');

        let type_str = self.build_type_string(col);
        def.push_str(&type_str);

        if col.is_auto_increment {
            def.push_str(" IDENTITY(1,1)");
        }

        if !col.is_nullable {
            def.push_str(" NOT NULL");
        } else {
            def.push_str(" NULL");
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
        sql.push_str(&format!("[{}]", design.table_name.replace("]", "]]")));
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
                .map(|c| format!("[{}]", c.replace("]", "]]")))
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
                .map(|c| format!("[{}]", c.replace("]", "]]")))
                .collect();
            let unique_str = if idx.is_unique { "UNIQUE " } else { "" };
            sql.push_str(&format!(
                "\nCREATE {}INDEX [{}] ON [{}] ({});",
                unique_str,
                idx.name.replace("]", "]]"),
                design.table_name.replace("]", "]]"),
                idx_cols.join(", ")
            ));
        }

        sql
    }

    /// MSSQL 使用 EXEC sp_rename 进行列重命名。
    fn build_column_rename_sql(
        &self,
        table_name: &str,
        old_name: &str,
        new_name: &str,
        _new_column: Option<&ColumnDefinition>,
    ) -> String {
        let quoted_table = self.quote_identifier(table_name);
        let quoted_old = self.quote_identifier(old_name);
        let table_column_ref = format!("{}.{}", quoted_table, quoted_old);
        format!(
            "EXEC sp_rename '{}', '{}', 'COLUMN';",
            table_column_ref.replace('\'', "''"),
            new_name.replace('\'', "''")
        )
    }

    fn build_alter_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        let mut statements: Vec<String> = Vec::new();
        let table_name = format!("[{}]", new.table_name.replace("]", "]]"));

        let original_cols: HashMap<&str, &ColumnDefinition> = original
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let new_cols: HashMap<&str, &ColumnDefinition> =
            new.columns.iter().map(|c| (c.name.as_str(), c)).collect();

        for name in original_cols.keys() {
            if !new_cols.contains_key(name) {
                statements.push(format!(
                    "ALTER TABLE {} DROP COLUMN [{}];",
                    table_name,
                    name.replace("]", "]]")
                ));
            }
        }

        for col in new.columns.iter() {
            if let Some(orig_col) = original_cols.get(col.name.as_str()) {
                if self.column_changed(orig_col, col) {
                    let col_name = format!("[{}]", col.name.replace("]", "]]"));
                    let type_str = self.build_type_string(col);
                    let null_str = if col.is_nullable { "NULL" } else { "NOT NULL" };
                    statements.push(format!(
                        "ALTER TABLE {} ALTER COLUMN {} {} {};",
                        table_name, col_name, type_str, null_str
                    ));
                }
            } else {
                let col_def = self.build_column_def(col);
                statements.push(format!("ALTER TABLE {} ADD {};", table_name, col_def));
            }
        }

        let original_indexes: HashMap<&str, &IndexDefinition> = original
            .indexes
            .iter()
            .map(|i| (i.name.as_str(), i))
            .collect();
        let new_indexes: HashMap<&str, &IndexDefinition> =
            new.indexes.iter().map(|i| (i.name.as_str(), i)).collect();

        for (name, idx) in &original_indexes {
            if !new_indexes.contains_key(name) {
                if idx.is_primary {
                    statements.push(format!(
                        "ALTER TABLE {} DROP CONSTRAINT [{}];",
                        table_name,
                        name.replace("]", "]]")
                    ));
                } else {
                    statements.push(format!(
                        "DROP INDEX [{}] ON {};",
                        name.replace("]", "]]"),
                        table_name
                    ));
                }
            }
        }

        for (name, idx) in &new_indexes {
            if !original_indexes.contains_key(name) {
                let idx_cols: Vec<String> = idx
                    .columns
                    .iter()
                    .map(|c| format!("[{}]", c.replace("]", "]]")))
                    .collect();

                if idx.is_primary {
                    statements.push(format!(
                        "ALTER TABLE {} ADD CONSTRAINT [{}] PRIMARY KEY ({});",
                        table_name,
                        name.replace("]", "]]"),
                        idx_cols.join(", ")
                    ));
                } else {
                    let unique_str = if idx.is_unique { "UNIQUE " } else { "" };
                    statements.push(format!(
                        "CREATE {}INDEX [{}] ON {} ({});",
                        unique_str,
                        name.replace("]", "]]"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::DatabasePlugin;
    use crate::types::{ColumnDefinition, IndexDefinition, TableDesign, TableOptions};
    use std::collections::HashMap;

    fn create_plugin() -> MsSqlPlugin {
        MsSqlPlugin::new()
    }

    // ==================== Basic Plugin Info Tests ====================

    #[test]
    fn test_plugin_name() {
        let plugin = create_plugin();
        assert_eq!(plugin.name(), DatabaseType::MSSQL);
    }

    #[test]
    fn test_quote_identifier() {
        let plugin = create_plugin();
        assert_eq!(plugin.quote_identifier("table_name"), "[table_name]");
        assert_eq!(plugin.quote_identifier("column"), "[column]");
        assert_eq!(plugin.quote_identifier("col]umn"), "[col]]umn]");
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

    #[test]
    fn test_format_pagination() {
        let plugin = create_plugin();
        assert_eq!(
            plugin.format_pagination(10, 20, " ORDER BY id"),
            " OFFSET 20 ROWS FETCH NEXT 10 ROWS ONLY"
        );
        assert_eq!(
            plugin.format_pagination(500, 0, ""),
            " ORDER BY (SELECT NULL) OFFSET 0 ROWS FETCH NEXT 500 ROWS ONLY"
        );
    }

    #[test]
    fn test_format_table_reference() {
        let plugin = create_plugin();
        assert_eq!(
            plugin.format_table_reference("mydb", None, "users"),
            "[mydb]..[users]"
        );
    }

    // ==================== DDL SQL Generation Tests ====================

    #[test]
    fn test_drop_database() {
        let plugin = create_plugin();
        let sql = plugin.drop_database("test_db");
        assert!(sql.contains("DROP DATABASE"));
        assert!(sql.contains("[test_db]"));
    }

    #[test]
    fn test_drop_table() {
        let plugin = create_plugin();

        // Test without schema
        let sql = plugin.drop_table("test_db", None, "users");
        assert!(sql.contains("DROP TABLE IF EXISTS"));
        assert!(sql.contains("[users]"));
        assert!(!sql.contains("test_db")); // database should not be in the SQL

        // Test with schema
        let sql_with_schema = plugin.drop_table("test_db", Some("dbo"), "users");
        assert!(sql_with_schema.contains("DROP TABLE IF EXISTS"));
        assert!(sql_with_schema.contains("[dbo]"));
        assert!(sql_with_schema.contains("[users]"));
        assert!(!sql_with_schema.contains("test_db")); // database should not be in the SQL
    }

    #[test]
    fn test_truncate_table() {
        let plugin = create_plugin();
        let sql = plugin.truncate_table("test_db", "users");
        assert!(sql.contains("TRUNCATE TABLE"));
        assert!(sql.contains("[users]"));
    }

    #[test]
    fn test_build_backup_table_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_backup_table_sql("test_db", Some("dbo"), "orders", "orders_bak");
        assert_eq!(sql, "SELECT * INTO [dbo].[orders_bak] FROM [dbo].[orders];");
    }

    #[test]
    fn test_drop_view() {
        let plugin = create_plugin();
        let sql = plugin.drop_view("test_db", "my_view");
        assert!(sql.contains("DROP VIEW"));
        assert!(sql.contains("[my_view]"));
    }

    // ==================== Database Operations Tests ====================

    #[test]
    fn test_build_create_database_sql() {
        let plugin = create_plugin();
        let mut field_values = HashMap::new();
        field_values.insert(
            "collation".to_string(),
            "SQL_Latin1_General_CP1_CI_AS".to_string(),
        );

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "new_db".to_string(),
            field_values,
        };

        let sql = plugin.build_create_database_sql(&request);
        assert!(sql.contains("CREATE DATABASE"));
        assert!(sql.contains("[new_db]"));
        assert!(sql.contains("COLLATE"));
    }

    #[test]
    fn test_build_modify_database_sql() {
        let plugin = create_plugin();
        let mut field_values = HashMap::new();
        field_values.insert(
            "collation".to_string(),
            "SQL_Latin1_General_CP1_CI_AS".to_string(),
        );

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "my_db".to_string(),
            field_values,
        };

        let sql = plugin.build_modify_database_sql(&request);
        assert!(sql.contains("ALTER DATABASE"));
        assert!(sql.contains("[my_db]"));
    }

    #[test]
    fn test_build_drop_database_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_database_sql("old_db");
        assert!(sql.contains("DROP DATABASE"));
        assert!(sql.contains("[old_db]"));
    }

    // ==================== Schema Operations Tests ====================

    #[test]
    fn test_build_create_schema_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_create_schema_sql("my_schema");
        assert!(sql.contains("CREATE SCHEMA"));
        assert!(sql.contains("[my_schema]"));
    }

    #[test]
    fn test_build_drop_schema_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_schema_sql("my_schema");
        assert!(sql.contains("DROP SCHEMA"));
        assert!(sql.contains("[my_schema]"));
    }

    #[test]
    fn test_build_comment_schema_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_comment_schema_sql("my_schema", "Test schema");
        assert!(sql.is_some());
        let sql = sql.unwrap();
        assert!(sql.contains("sp_addextendedproperty"));
        assert!(sql.contains("my_schema"));
        assert!(sql.contains("Test schema"));
    }

    // ==================== Column Definition Tests ====================

    #[test]
    fn test_build_column_def_simple() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("id")
            .data_type("INT")
            .nullable(false)
            .primary_key(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("[id]"));
        assert!(def.contains("INT"));
        assert!(def.contains("NOT NULL"));
    }

    #[test]
    fn test_build_column_def_with_length() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("name")
            .data_type("NVARCHAR")
            .length(255)
            .nullable(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("[name]"));
        assert!(def.contains("NVARCHAR(255)"));
        assert!(!def.contains("NOT NULL"));
    }

    #[test]
    fn test_build_column_def_with_default() {
        let plugin = create_plugin();
        let mut col = ColumnDefinition::new("status")
            .data_type("INT")
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
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id")
                    .data_type("INT")
                    .nullable(false)
                    .primary_key(true)
                    .auto_increment(true),
                ColumnDefinition::new("name")
                    .data_type("NVARCHAR")
                    .length(100),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_create_table_sql(&design);
        assert!(sql.contains("CREATE TABLE [users]"));
        assert!(sql.contains("[id]"));
        assert!(sql.contains("INT"));
        assert!(sql.contains("[name]"));
        assert!(sql.contains("NVARCHAR(100)"));
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
                    .data_type("INT")
                    .nullable(false)
                    .primary_key(true),
                ColumnDefinition::new("user_id")
                    .data_type("INT")
                    .nullable(false),
                ColumnDefinition::new("email")
                    .data_type("NVARCHAR")
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
        assert!(sql.contains("INDEX [idx_user_id]"));
        assert!(sql.contains("UNIQUE INDEX [idx_email]"));
    }

    // ==================== ALTER TABLE Tests ====================

    #[test]
    fn test_build_alter_table_sql_add_column() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id").data_type("INT")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("email")
                    .data_type("NVARCHAR")
                    .length(100),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("ADD"));
        assert!(sql.contains("[email]"));
    }

    #[test]
    fn test_build_alter_table_sql_drop_column() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("old_column")
                    .data_type("NVARCHAR")
                    .length(50),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id").data_type("INT")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("DROP COLUMN"));
        assert!(sql.contains("[old_column]"));
    }

    #[test]
    fn test_build_alter_table_sql_modify_column() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("name")
                .data_type("NVARCHAR")
                .length(50)],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![ColumnDefinition::new("name")
                .data_type("NVARCHAR")
                .length(100)
                .nullable(false)],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("ALTER COLUMN"));
        assert!(sql.contains("[name] NVARCHAR(100)"));
        assert!(sql.contains("NOT NULL"));
    }

    #[test]
    fn test_build_alter_table_sql_add_unique_index() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("name")
                    .data_type("NVARCHAR")
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
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("name")
                    .data_type("NVARCHAR")
                    .length(50),
            ],
            indexes: vec![IndexDefinition::new("idx_name")
                .columns(vec!["name".to_string()])
                .unique(true)],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("CREATE UNIQUE INDEX"));
        assert!(sql.contains("[idx_name]"));
        assert!(sql.contains("[name]"));
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

        assert!(info.keywords.iter().any(|(k, _)| *k == "TOP"));
    }
}
