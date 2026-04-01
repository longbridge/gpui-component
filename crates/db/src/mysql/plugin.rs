use std::collections::HashMap;

use anyhow::Result;
use gpui_component::table::Column;
use one_core::storage::{DatabaseType, DbConnectionConfig};

use crate::connection::{DbConnection, DbError};
use crate::executor::SqlResult;
use crate::import_export::{
    ExportConfig, ExportProgressSender, ExportResult, ImportConfig, ImportProgressSender,
    ImportResult,
};
use crate::mysql::connection::MysqlDbConnection;
use crate::plugin::{DatabasePlugin, SqlCompletionInfo};
use crate::types::*;

/// MySQL data types (name, description)
pub const MYSQL_DATA_TYPES: &[(&str, &str)] = &[
    ("TINYINT", "Very small integer (-128 to 127)"),
    ("SMALLINT", "Small integer (-32768 to 32767)"),
    ("MEDIUMINT", "Medium integer (-8388608 to 8388607)"),
    ("INT", "Standard integer (-2147483648 to 2147483647)"),
    ("BIGINT", "Large integer"),
    ("DECIMAL", "Fixed-point number"),
    ("FLOAT", "Single-precision floating-point"),
    ("DOUBLE", "Double-precision floating-point"),
    ("BIT", "Bit field"),
    ("CHAR", "Fixed-length string"),
    ("VARCHAR", "Variable-length string"),
    ("TINYTEXT", "Very small text (255 bytes)"),
    ("TEXT", "Text (65KB)"),
    ("MEDIUMTEXT", "Medium text (16MB)"),
    ("LONGTEXT", "Large text (4GB)"),
    ("BINARY", "Fixed-length binary"),
    ("VARBINARY", "Variable-length binary"),
    ("TINYBLOB", "Very small BLOB (255 bytes)"),
    ("BLOB", "BLOB (65KB)"),
    ("MEDIUMBLOB", "Medium BLOB (16MB)"),
    ("LONGBLOB", "Large BLOB (4GB)"),
    ("DATE", "Date (YYYY-MM-DD)"),
    ("TIME", "Time (HH:MM:SS)"),
    ("DATETIME", "Date and time"),
    ("TIMESTAMP", "Timestamp with timezone"),
    ("YEAR", "Year (1901-2155)"),
    ("BOOLEAN", "Boolean (TINYINT(1))"),
    ("JSON", "JSON document"),
    ("ENUM", "Enumeration"),
    ("SET", "Set of values"),
];

/// MySQL database plugin implementation (stateless)
pub struct MySqlPlugin;

impl MySqlPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl DatabasePlugin for MySqlPlugin {
    fn name(&self) -> DatabaseType {
        DatabaseType::MySQL
    }

    fn quote_identifier(&self, identifier: &str) -> String {
        format!("`{}`", identifier.replace("`", "``"))
    }

    fn get_completion_info(&self) -> SqlCompletionInfo {
        SqlCompletionInfo {
            keywords: vec![
                // MySQL-specific keywords only
                ("AUTO_INCREMENT", "Auto-increment column attribute"),
                ("ENGINE", "Storage engine specification"),
                ("CHARSET", "Character set specification"),
                ("COLLATE", "Collation specification"),
                ("UNSIGNED", "Unsigned integer attribute"),
                ("ZEROFILL", "Zero-fill display attribute"),
                ("BINARY", "Binary string comparison"),
                ("IGNORE", "Ignore errors during operation"),
                ("REPLACE", "Replace existing rows"),
                ("DUPLICATE KEY UPDATE", "On duplicate key update"),
                ("STRAIGHT_JOIN", "Force join order"),
                ("SQL_CALC_FOUND_ROWS", "Calculate total rows"),
                ("HIGH_PRIORITY", "High priority query"),
                ("LOW_PRIORITY", "Low priority query"),
                ("DELAYED", "Delayed insert"),
                ("FORCE INDEX", "Force index usage"),
                ("USE INDEX", "Suggest index usage"),
                ("IGNORE INDEX", "Ignore index"),
            ],
            functions: vec![
                // MySQL-specific functions only (standard SQL functions are added via with_standard_sql())
                ("CONCAT_WS(sep, str1, str2, ...)", "Concatenate with separator"),
                ("CHAR_LENGTH(str)", "String length in characters"),
                ("LPAD(str, len, pad)", "Left pad string"),
                ("RPAD(str, len, pad)", "Right pad string"),
                ("LOCATE(substr, str)", "Find substring position"),
                ("INSTR(str, substr)", "Find substring position"),
                ("REPEAT(str, count)", "Repeat string"),
                ("SPACE(n)", "Generate spaces"),
                ("FORMAT(num, decimals)", "Format number"),
                ("TRUNCATE(x, d)", "Truncate to d decimal places"),
                ("POW(x, y)", "Power function"),
                ("RAND()", "Random number 0-1"),
                ("CURDATE()", "Current date"),
                ("CURTIME()", "Current time"),
                ("DATE(expr)", "Extract date part"),
                ("TIME(expr)", "Extract time part"),
                ("YEAR(date)", "Extract year"),
                ("MONTH(date)", "Extract month"),
                ("DAY(date)", "Extract day"),
                ("HOUR(time)", "Extract hour"),
                ("MINUTE(time)", "Extract minute"),
                ("SECOND(time)", "Extract second"),
                ("DAYOFWEEK(date)", "Day of week (1=Sunday)"),
                ("DAYOFMONTH(date)", "Day of month"),
                ("DAYOFYEAR(date)", "Day of year"),
                ("WEEK(date)", "Week number"),
                ("WEEKDAY(date)", "Weekday (0=Monday)"),
                ("DATE_ADD(date, INTERVAL)", "Add interval to date"),
                ("DATE_SUB(date, INTERVAL)", "Subtract interval from date"),
                ("DATEDIFF(date1, date2)", "Difference in days"),
                ("TIMESTAMPDIFF(unit, dt1, dt2)", "Difference in specified unit"),
                ("DATE_FORMAT(date, format)", "Format date"),
                ("STR_TO_DATE(str, format)", "Parse string to date"),
                ("UNIX_TIMESTAMP()", "Current Unix timestamp"),
                ("FROM_UNIXTIME(ts)", "Convert Unix timestamp"),
                ("GROUP_CONCAT(col)", "Concatenate group values"),
                ("IF(cond, then, else)", "Conditional expression"),
                ("IFNULL(expr, alt)", "Return alt if expr is NULL"),
                ("JSON_EXTRACT(doc, path)", "Extract JSON value"),
                ("JSON_UNQUOTE(json)", "Unquote JSON string"),
                ("JSON_OBJECT(key, val, ...)", "Create JSON object"),
                ("JSON_ARRAY(val, ...)", "Create JSON array"),
                ("JSON_CONTAINS(doc, val)", "Check if JSON contains value"),
                ("JSON_LENGTH(doc)", "JSON document length"),
                ("CONVERT(expr, type)", "Type conversion"),
                ("UUID()", "Generate UUID"),
                ("LAST_INSERT_ID()", "Last auto-increment ID"),
                ("FOUND_ROWS()", "Rows found by previous query"),
                ("ROW_COUNT()", "Affected rows count"),
                ("DATABASE()", "Current database name"),
                ("USER()", "Current user"),
                ("VERSION()", "MySQL version"),
            ],
            operators: vec![
                ("REGEXP", "Regular expression match"),
                ("RLIKE", "Regular expression match (alias)"),
                ("SOUNDS LIKE", "Soundex comparison"),
                ("<=>", "NULL-safe equal"),
                ("DIV", "Integer division"),
                ("XOR", "Logical XOR"),
                (":=", "Assignment operator"),
            ],
            data_types: MYSQL_DATA_TYPES.to_vec(),
            snippets: vec![
                ("crt", "CREATE TABLE $1 (\n  id INT AUTO_INCREMENT PRIMARY KEY,\n  $2\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4", "Create table"),
                ("idx", "CREATE INDEX $1 ON $2 ($3)", "Create index"),
                ("alt", "ALTER TABLE $1 ADD COLUMN $2", "Add column"),
                ("jn", "JOIN $1 ON $2.$3 = $4.$5", "Join clause"),
                ("lj", "LEFT JOIN $1 ON $2.$3 = $4.$5", "Left join clause"),
            ],
        }.with_standard_sql()
    }

    async fn create_connection(
        &self,
        config: DbConnectionConfig,
    ) -> Result<Box<dyn DbConnection + Send + Sync>, DbError> {
        let mut conn = MysqlDbConnection::new(config);
        conn.connect().await?;
        Ok(Box::new(conn))
    }

    async fn list_databases(&self, connection: &dyn DbConnection) -> Result<Vec<String>> {
        let result = connection
            .query("SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA ORDER BY SCHEMA_NAME")
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

    // === Database/Schema Level Operations ===

    async fn list_databases_view(&self, connection: &dyn DbConnection) -> Result<ObjectView> {
        use gpui::px;

        let databases = self.list_databases_detailed(connection).await?;

        let columns = vec![
            Column::new("name", "Name").width(px(180.0)),
            Column::new("charset", "Charset").width(px(120.0)),
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
                s.SCHEMA_NAME as name,
                s.DEFAULT_CHARACTER_SET_NAME as charset,
                s.DEFAULT_COLLATION_NAME as collation,
                COUNT(t.TABLE_NAME) as table_count
            FROM INFORMATION_SCHEMA.SCHEMATA s
            LEFT JOIN INFORMATION_SCHEMA.TABLES t 
                ON s.SCHEMA_NAME = t.TABLE_SCHEMA AND t.TABLE_TYPE = 'BASE TABLE'
            GROUP BY s.SCHEMA_NAME, s.DEFAULT_CHARACTER_SET_NAME, s.DEFAULT_COLLATION_NAME
            ORDER BY s.SCHEMA_NAME",
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
                    let table_count = row
                        .get(3)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse::<i64>().ok());

                    Some(DatabaseInfo {
                        name,
                        charset,
                        collation,
                        size: None,
                        table_count,
                        comment: None,
                    })
                })
                .collect();
            Ok(databases)
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    fn sql_dialect(&self) -> Box<dyn sqlparser::dialect::Dialect> {
        Box::new(sqlparser::dialect::MySqlDialect {})
    }

    // === Table Operations ===

    async fn list_tables(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        _schema: Option<String>,
    ) -> Result<Vec<TableInfo>> {
        // Query to get all tables with their description/metadata
        let sql = format!(
            "SELECT \
                TABLE_NAME, \
                TABLE_COMMENT, \
                ENGINE, \
                TABLE_ROWS, \
                CREATE_TIME, \
                TABLE_COLLATION \
             FROM INFORMATION_SCHEMA.TABLES \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_TYPE IN ('BASE TABLE','SYSTEM VIEW') \
             ORDER BY TABLE_NAME",
            database
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
                    let collation = row.get(5).and_then(|v| v.clone());
                    // Extract charset from collation (e.g., "utf8mb4_general_ci" -> "utf8mb4")
                    let charset = collation
                        .as_ref()
                        .and_then(|c| c.split('_').next().map(|s| s.to_string()));

                    // Parse row count
                    let row_count = row
                        .get(3)
                        .and_then(|v| v.clone())
                        .and_then(|s| s.parse::<i64>().ok());

                    TableInfo {
                        name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                        schema: None,
                        comment: row.get(1).and_then(|v| v.clone()).filter(|s| !s.is_empty()),
                        engine: row.get(2).and_then(|v| v.clone()),
                        row_count,
                        create_time: row.get(4).and_then(|v| v.clone()),
                        charset,
                        collation,
                    }
                })
                .collect();

            Ok(tables)
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

        let columns = vec![
            Column::new("name", "Name").width(px(200.0)),
            Column::new("engine", "Engine").width(px(150.0)),
            Column::new("rows", "Rows").width(px(100.0)).text_right(),
            Column::new("created", "Created").width(px(180.0)),
            Column::new("comment", "Comment").width(px(300.0)),
        ];

        let rows: Vec<Vec<String>> = tables
            .iter()
            .map(|table| {
                vec![
                    table.name.clone(),
                    table.engine.as_deref().unwrap_or("-").to_string(),
                    table
                        .row_count
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    table.create_time.as_deref().unwrap_or("-").to_string(),
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
        database: &str,
        _schema: Option<String>,
        table: &str,
    ) -> Result<Vec<ColumnInfo>> {
        let sql = format!(
            "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_KEY, COLUMN_DEFAULT, COLUMN_COMMENT, \
             CHARACTER_SET_NAME, COLLATION_NAME \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' \
             ORDER BY ORDINAL_POSITION",
            database, table
        );

        let result = connection
            .query(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list columns: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            Ok(query_result
                .rows
                .iter()
                .map(|row| ColumnInfo {
                    name: row.first().and_then(|v| v.clone()).unwrap_or_default(),
                    data_type: row.get(1).and_then(|v| v.clone()).unwrap_or_default(),
                    is_nullable: row
                        .get(2)
                        .and_then(|v| v.clone())
                        .map(|v| v == "YES")
                        .unwrap_or(true),
                    is_primary_key: row
                        .get(3)
                        .and_then(|v| v.clone())
                        .map(|v| v == "PRI")
                        .unwrap_or(false),
                    default_value: row.get(4).and_then(|v| v.clone()),
                    comment: row.get(5).and_then(|v| v.clone()),
                    charset: row.get(6).and_then(|v| v.clone()),
                    collation: row.get(7).and_then(|v| v.clone()),
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
            Column::new("comment", "Comment").width(px(250.0)),
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
                    col.comment.as_deref().unwrap_or("").to_string(),
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
        database: &str,
        _schema: Option<String>,
        table: &str,
    ) -> Result<Vec<IndexInfo>> {
        let sql = format!(
            "SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE, INDEX_TYPE \
             FROM INFORMATION_SCHEMA.STATISTICS \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' AND INDEX_NAME != 'PRIMARY' \
             ORDER BY INDEX_NAME, SEQ_IN_INDEX",
            database, table
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
                    .map(|v| v == "0")
                    .unwrap_or(false);
                let index_type = row.get(3).and_then(|v| v.clone());

                indexes
                    .entry(index_name.clone())
                    .or_insert_with(|| IndexInfo {
                        name: index_name,
                        columns: Vec::new(),
                        is_unique,
                        index_type: index_type.clone(),
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
    // === View Operations ===

    async fn list_table_checks(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
        _table: &str,
    ) -> Result<Vec<CheckInfo>> {
        let sql = format!(
            "SELECT cc.CONSTRAINT_NAME, tc.TABLE_NAME, cc.CHECK_CLAUSE \
             FROM INFORMATION_SCHEMA.CHECK_CONSTRAINTS cc \
             JOIN INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
                ON cc.CONSTRAINT_SCHEMA = tc.CONSTRAINT_SCHEMA \
                AND cc.CONSTRAINT_NAME = tc.CONSTRAINT_NAME \
             WHERE tc.CONSTRAINT_SCHEMA = '{}' AND tc.TABLE_NAME = '{}' \
               AND tc.CONSTRAINT_TYPE = 'CHECK' \
             ORDER BY cc.CONSTRAINT_NAME",
            _database, _table
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

    async fn list_views(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        _schema: Option<String>,
    ) -> Result<Vec<ViewInfo>> {
        let sql = format!(
            "SELECT TABLE_NAME, VIEW_DEFINITION \
             FROM INFORMATION_SCHEMA.VIEWS \
             WHERE TABLE_SCHEMA = '{}' \
             ORDER BY TABLE_NAME",
            database
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
                    schema: None,
                    definition: row.get(1).and_then(|v| v.clone()),
                    comment: None,
                })
                .collect())
        } else {
            Err(anyhow::anyhow!("Unexpected result type"))
        }
    }

    // === Function Operations ===

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

    async fn list_functions(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let sql = format!(
            "SELECT ROUTINE_NAME, DTD_IDENTIFIER \
             FROM INFORMATION_SCHEMA.ROUTINES \
             WHERE ROUTINE_SCHEMA = '{}' AND ROUTINE_TYPE = 'FUNCTION' \
             ORDER BY ROUTINE_NAME",
            database
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

    // === Procedure Operations ===

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

    async fn list_procedures(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let sql = format!(
            "SELECT ROUTINE_NAME \
             FROM INFORMATION_SCHEMA.ROUTINES \
             WHERE ROUTINE_SCHEMA = '{}' AND ROUTINE_TYPE = 'PROCEDURE' \
             ORDER BY ROUTINE_NAME",
            database
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

    // === Trigger Operations ===

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

    async fn list_triggers(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<TriggerInfo>> {
        let sql = format!(
            "SELECT TRIGGER_NAME, EVENT_OBJECT_TABLE, EVENT_MANIPULATION, ACTION_TIMING \
             FROM INFORMATION_SCHEMA.TRIGGERS \
             WHERE TRIGGER_SCHEMA = '{}' \
             ORDER BY TRIGGER_NAME",
            database
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

    // === Sequence Operations ===
    // MySQL doesn't support sequences natively (until MySQL 8.0 which has AUTO_INCREMENT only)
    // Return empty results

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

        if let Some(comment) = &column.comment {
            def.push_str(&format!(" COMMENT '{}'", comment.replace("'", "''")));
        }

        def
    }

    // === Database Management Operations ===
    fn build_create_database_sql(
        &self,
        request: &crate::plugin::DatabaseOperationRequest,
    ) -> String {
        let db_name = self.quote_identifier(&request.database_name);
        let charset = request
            .field_values
            .get("charset")
            .map(|s| s.as_str())
            .unwrap_or("utf8mb4");
        let collation = request
            .field_values
            .get("collation")
            .map(|s| s.as_str())
            .unwrap_or("utf8mb4_general_ci");

        format!(
            "CREATE DATABASE {} CHARACTER SET {} COLLATE {};",
            db_name, charset, collation
        )
    }

    fn build_modify_database_sql(
        &self,
        request: &crate::plugin::DatabaseOperationRequest,
    ) -> String {
        let db_name = self.quote_identifier(&request.database_name);
        let charset = request
            .field_values
            .get("charset")
            .map(|s| s.as_str())
            .unwrap_or("utf8mb4");
        let collation = request
            .field_values
            .get("collation")
            .map(|s| s.as_str())
            .unwrap_or("utf8mb4_general_ci");

        format!(
            "ALTER DATABASE {} CHARACTER SET {} COLLATE {};",
            db_name, charset, collation
        )
    }

    fn build_drop_database_sql(&self, database_name: &str) -> String {
        format!("DROP DATABASE {};", self.quote_identifier(database_name))
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

    async fn export_table_create_sql(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        _schema: Option<&str>,
        table: &str,
    ) -> Result<String> {
        let table_ref = self.format_table_reference(database, None, table);
        let show_create = format!("SHOW CREATE TABLE {}", table_ref);
        let result = connection
            .query(&show_create)
            .await
            .map_err(|e| anyhow::anyhow!("Query failed: {}", e))?;

        if let SqlResult::Query(query_result) = result {
            if let Some(row) = query_result.rows.first() {
                if let Some(Some(create_sql)) = row.get(1) {
                    return Ok(create_sql.clone());
                }
            }
        }
        Ok(String::new())
    }

    fn get_charsets(&self) -> Vec<CharsetInfo> {
        vec![
            CharsetInfo {
                name: "utf8mb4".into(),
                description: "UTF-8 Unicode (4 bytes)".into(),
                default_collation: "utf8mb4_general_ci".into(),
            },
            CharsetInfo {
                name: "utf8mb3".into(),
                description: "UTF-8 Unicode (3 bytes)".into(),
                default_collation: "utf8mb3_general_ci".into(),
            },
            CharsetInfo {
                name: "utf8".into(),
                description: "UTF-8 Unicode (alias for utf8mb3)".into(),
                default_collation: "utf8_general_ci".into(),
            },
            CharsetInfo {
                name: "latin1".into(),
                description: "West European (ISO 8859-1)".into(),
                default_collation: "latin1_swedish_ci".into(),
            },
            CharsetInfo {
                name: "latin2".into(),
                description: "Central European (ISO 8859-2)".into(),
                default_collation: "latin2_general_ci".into(),
            },
            CharsetInfo {
                name: "ascii".into(),
                description: "US ASCII".into(),
                default_collation: "ascii_general_ci".into(),
            },
            CharsetInfo {
                name: "gbk".into(),
                description: "GBK Simplified Chinese".into(),
                default_collation: "gbk_chinese_ci".into(),
            },
            CharsetInfo {
                name: "gb2312".into(),
                description: "GB2312 Simplified Chinese".into(),
                default_collation: "gb2312_chinese_ci".into(),
            },
            CharsetInfo {
                name: "gb18030".into(),
                description: "GB18030 Chinese".into(),
                default_collation: "gb18030_chinese_ci".into(),
            },
            CharsetInfo {
                name: "big5".into(),
                description: "Big5 Traditional Chinese".into(),
                default_collation: "big5_chinese_ci".into(),
            },
            CharsetInfo {
                name: "sjis".into(),
                description: "Shift-JIS Japanese".into(),
                default_collation: "sjis_japanese_ci".into(),
            },
            CharsetInfo {
                name: "euckr".into(),
                description: "EUC-KR Korean".into(),
                default_collation: "euckr_korean_ci".into(),
            },
            CharsetInfo {
                name: "greek".into(),
                description: "ISO 8859-7 Greek".into(),
                default_collation: "greek_general_ci".into(),
            },
            CharsetInfo {
                name: "hebrew".into(),
                description: "ISO 8859-8 Hebrew".into(),
                default_collation: "hebrew_general_ci".into(),
            },
            CharsetInfo {
                name: "cp1251".into(),
                description: "Windows Cyrillic".into(),
                default_collation: "cp1251_general_ci".into(),
            },
            CharsetInfo {
                name: "cp1256".into(),
                description: "Windows Arabic".into(),
                default_collation: "cp1256_general_ci".into(),
            },
            CharsetInfo {
                name: "binary".into(),
                description: "Binary pseudo charset".into(),
                default_collation: "binary".into(),
            },
        ]
    }

    fn get_collations(&self, charset: &str) -> Vec<CollationInfo> {
        match charset {
            "utf8mb4" => vec![
                CollationInfo {
                    name: "utf8mb4_general_ci".into(),
                    charset: "utf8mb4".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "utf8mb4_unicode_ci".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8mb4_unicode_520_ci".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8mb4_bin".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8mb4_0900_ai_ci".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8mb4_0900_as_ci".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8mb4_0900_as_cs".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8mb4_zh_0900_as_cs".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8mb4_ja_0900_as_cs".into(),
                    charset: "utf8mb4".into(),
                    is_default: false,
                },
            ],
            "utf8mb3" | "utf8" => vec![
                CollationInfo {
                    name: "utf8_general_ci".into(),
                    charset: "utf8".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "utf8_unicode_ci".into(),
                    charset: "utf8".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "utf8_bin".into(),
                    charset: "utf8".into(),
                    is_default: false,
                },
            ],
            "latin1" => vec![
                CollationInfo {
                    name: "latin1_swedish_ci".into(),
                    charset: "latin1".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "latin1_general_ci".into(),
                    charset: "latin1".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "latin1_general_cs".into(),
                    charset: "latin1".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "latin1_bin".into(),
                    charset: "latin1".into(),
                    is_default: false,
                },
            ],
            "latin2" => vec![
                CollationInfo {
                    name: "latin2_general_ci".into(),
                    charset: "latin2".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "latin2_bin".into(),
                    charset: "latin2".into(),
                    is_default: false,
                },
            ],
            "ascii" => vec![
                CollationInfo {
                    name: "ascii_general_ci".into(),
                    charset: "ascii".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "ascii_bin".into(),
                    charset: "ascii".into(),
                    is_default: false,
                },
            ],
            "gbk" => vec![
                CollationInfo {
                    name: "gbk_chinese_ci".into(),
                    charset: "gbk".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "gbk_bin".into(),
                    charset: "gbk".into(),
                    is_default: false,
                },
            ],
            "gb2312" => vec![
                CollationInfo {
                    name: "gb2312_chinese_ci".into(),
                    charset: "gb2312".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "gb2312_bin".into(),
                    charset: "gb2312".into(),
                    is_default: false,
                },
            ],
            "gb18030" => vec![
                CollationInfo {
                    name: "gb18030_chinese_ci".into(),
                    charset: "gb18030".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "gb18030_bin".into(),
                    charset: "gb18030".into(),
                    is_default: false,
                },
                CollationInfo {
                    name: "gb18030_unicode_520_ci".into(),
                    charset: "gb18030".into(),
                    is_default: false,
                },
            ],
            "big5" => vec![
                CollationInfo {
                    name: "big5_chinese_ci".into(),
                    charset: "big5".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "big5_bin".into(),
                    charset: "big5".into(),
                    is_default: false,
                },
            ],
            "sjis" => vec![
                CollationInfo {
                    name: "sjis_japanese_ci".into(),
                    charset: "sjis".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "sjis_bin".into(),
                    charset: "sjis".into(),
                    is_default: false,
                },
            ],
            "euckr" => vec![
                CollationInfo {
                    name: "euckr_korean_ci".into(),
                    charset: "euckr".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "euckr_bin".into(),
                    charset: "euckr".into(),
                    is_default: false,
                },
            ],
            "greek" => vec![
                CollationInfo {
                    name: "greek_general_ci".into(),
                    charset: "greek".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "greek_bin".into(),
                    charset: "greek".into(),
                    is_default: false,
                },
            ],
            "hebrew" => vec![
                CollationInfo {
                    name: "hebrew_general_ci".into(),
                    charset: "hebrew".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "hebrew_bin".into(),
                    charset: "hebrew".into(),
                    is_default: false,
                },
            ],
            "cp1251" => vec![
                CollationInfo {
                    name: "cp1251_general_ci".into(),
                    charset: "cp1251".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "cp1251_bin".into(),
                    charset: "cp1251".into(),
                    is_default: false,
                },
            ],
            "cp1256" => vec![
                CollationInfo {
                    name: "cp1256_general_ci".into(),
                    charset: "cp1256".into(),
                    is_default: true,
                },
                CollationInfo {
                    name: "cp1256_bin".into(),
                    charset: "cp1256".into(),
                    is_default: false,
                },
            ],
            "binary" => vec![CollationInfo {
                name: "binary".into(),
                charset: "binary".into(),
                is_default: true,
            }],
            _ => vec![],
        }
    }

    fn get_data_types(&self) -> &[(&'static str, &'static str)] {
        MYSQL_DATA_TYPES
    }

    fn parse_column_type(&self, type_str: &str) -> ParsedColumnType {
        let upper = type_str.to_uppercase();
        let is_unsigned = upper.contains("UNSIGNED");
        let is_auto_increment = upper.contains("AUTO_INCREMENT");

        let base_upper = upper.split_whitespace().next().unwrap_or(&upper);
        if base_upper.starts_with("ENUM") || base_upper.starts_with("SET") {
            if let Some(start) = type_str.find('(') {
                if let Some(end) = type_str.rfind(')') {
                    let base_type = type_str[..start].trim().to_string();
                    let enum_values = type_str[start + 1..end].to_string();
                    return ParsedColumnType {
                        base_type,
                        length: None,
                        scale: None,
                        enum_values: Some(enum_values),
                        is_unsigned,
                        is_auto_increment,
                    };
                }
            }
        }

        if let Some(start) = type_str.find('(') {
            if let Some(end) = type_str.find(')') {
                let base_type = type_str[..start].trim().to_string();
                let params = &type_str[start + 1..end];

                if let Some(comma) = params.find(',') {
                    let length = params[..comma].trim().parse().ok();
                    let scale = params[comma + 1..].trim().parse().ok();
                    return ParsedColumnType {
                        base_type,
                        length,
                        scale,
                        enum_values: None,
                        is_unsigned,
                        is_auto_increment,
                    };
                }

                let length = params.trim().parse().ok();
                return ParsedColumnType {
                    base_type,
                    length,
                    scale: None,
                    enum_values: None,
                    is_unsigned,
                    is_auto_increment,
                };
            }
        }

        ParsedColumnType {
            base_type: type_str
                .split_whitespace()
                .next()
                .unwrap_or(type_str)
                .to_string(),
            length: None,
            scale: None,
            enum_values: None,
            is_unsigned,
            is_auto_increment,
        }
    }

    fn is_enum_type(&self, type_name: &str) -> bool {
        let upper = type_name.to_uppercase();
        upper.starts_with("ENUM") || upper.starts_with("SET")
    }

    fn rename_table(&self, _database: &str, old_name: &str, new_name: &str) -> String {
        format!(
            "RENAME TABLE {} TO {}",
            self.quote_identifier(old_name),
            self.quote_identifier(new_name)
        )
    }

    fn build_backup_table_sql(
        &self,
        database: &str,
        _schema: Option<&str>,
        source_table: &str,
        target_table: &str,
    ) -> String {
        let source = format!(
            "{}.{}",
            self.quote_identifier(database),
            self.quote_identifier(source_table)
        );
        let target = format!(
            "{}.{}",
            self.quote_identifier(database),
            self.quote_identifier(target_table)
        );
        format!(
            "CREATE TABLE {} LIKE {};\nINSERT INTO {} SELECT * FROM {};",
            target, source, target, source
        )
    }

    fn build_column_def(&self, col: &ColumnDefinition) -> String {
        let mut def = String::new();
        def.push_str(&self.quote_identifier(&col.name));
        def.push(' ');

        let type_str = self.build_type_string(col);
        def.push_str(&type_str);

        if col.is_unsigned {
            def.push_str(" UNSIGNED");
        }

        if !col.is_nullable {
            def.push_str(" NOT NULL");
        }

        if col.is_auto_increment {
            def.push_str(" AUTO_INCREMENT");
        }

        if let Some(default) = &col.default_value {
            if !default.is_empty() {
                def.push_str(&format!(" DEFAULT {}", default));
            }
        }

        if !col.comment.is_empty() {
            def.push_str(&format!(" COMMENT '{}'", col.comment.replace("'", "''")));
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

        for idx in &design.indexes {
            if idx.is_primary {
                continue;
            }
            let idx_cols: Vec<String> = idx
                .columns
                .iter()
                .map(|c| self.quote_identifier(c))
                .collect();
            let idx_type = if idx.is_unique {
                "UNIQUE INDEX"
            } else {
                "INDEX"
            };
            definitions.push(format!(
                "  {} {} ({})",
                idx_type,
                self.quote_identifier(&idx.name),
                idx_cols.join(", ")
            ));
        }

        sql.push_str(&definitions.join(",\n"));
        sql.push_str("\n)");

        if let Some(engine) = &design.options.engine {
            sql.push_str(&format!(" ENGINE={}", engine));
        }
        if let Some(charset) = &design.options.charset {
            sql.push_str(&format!(" DEFAULT CHARSET={}", charset));
        }
        if let Some(collation) = &design.options.collation {
            sql.push_str(&format!(" COLLATE={}", collation));
        }
        if !design.options.comment.is_empty() {
            sql.push_str(&format!(
                " COMMENT='{}'",
                design.options.comment.replace("'", "''")
            ));
        }

        sql.push(';');
        sql
    }

    /// MySQL 使用 CHANGE COLUMN 语法进行列重命名，需要完整列定义。
    fn build_column_rename_sql(
        &self,
        table_name: &str,
        old_name: &str,
        new_name: &str,
        new_column: Option<&ColumnDefinition>,
    ) -> String {
        let quoted_table = self.quote_identifier(table_name);
        let quoted_old = self.quote_identifier(old_name);
        if let Some(col) = new_column {
            let col_def = self.build_column_def(col);
            format!(
                "ALTER TABLE {} CHANGE COLUMN {} {};",
                quoted_table, quoted_old, col_def
            )
        } else {
            let quoted_new = self.quote_identifier(new_name);
            format!(
                "ALTER TABLE {} RENAME COLUMN {} TO {};",
                quoted_table, quoted_old, quoted_new
            )
        }
    }

    fn build_alter_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String {
        let mut statements: Vec<String> = Vec::new();
        let table_name = self.quote_identifier(&new.table_name);

        let original_cols: HashMap<&str, &ColumnDefinition> = original
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let new_cols: HashMap<&str, &ColumnDefinition> =
            new.columns.iter().map(|c| (c.name.as_str(), c)).collect();
        let original_order: HashMap<&str, usize> = original
            .columns
            .iter()
            .enumerate()
            .map(|(idx, col)| (col.name.as_str(), idx))
            .collect();
        let original_existing: Vec<&str> = original
            .columns
            .iter()
            .map(|col| col.name.as_str())
            .collect();
        let new_existing: Vec<&str> = new
            .columns
            .iter()
            .filter(|col| original_cols.contains_key(col.name.as_str()))
            .map(|col| col.name.as_str())
            .collect();
        let order_changed = original_existing != new_existing;
        let new_existing_positions: HashMap<&str, usize> = new_existing
            .iter()
            .enumerate()
            .map(|(idx, name)| (*name, idx))
            .collect();

        for name in original_cols.keys() {
            if !new_cols.contains_key(name) {
                statements.push(format!(
                    "ALTER TABLE {} DROP COLUMN {};",
                    table_name,
                    self.quote_identifier(name)
                ));
            }
        }

        for (idx, col) in new.columns.iter().enumerate() {
            if let Some(orig_col) = original_cols.get(col.name.as_str()) {
                if self.column_changed(orig_col, col) {
                    let col_def = self.build_column_def(col);
                    let position = if idx == 0 {
                        " FIRST".to_string()
                    } else {
                        format!(
                            " AFTER {}",
                            self.quote_identifier(&new.columns[idx - 1].name)
                        )
                    };
                    statements.push(format!(
                        "ALTER TABLE {} MODIFY COLUMN {}{};",
                        table_name, col_def, position
                    ));
                } else if order_changed {
                    let original_idx = original_order.get(col.name.as_str());
                    let new_idx = new_existing_positions.get(col.name.as_str());
                    if let (Some(original_idx), Some(new_idx)) = (original_idx, new_idx) {
                        if original_idx != new_idx {
                            let col_def = self.build_column_def(col);
                            let position = if idx == 0 {
                                " FIRST".to_string()
                            } else {
                                format!(
                                    " AFTER {}",
                                    self.quote_identifier(&new.columns[idx - 1].name)
                                )
                            };
                            statements.push(format!(
                                "ALTER TABLE {} MODIFY COLUMN {}{};",
                                table_name, col_def, position
                            ));
                        }
                    }
                }
            } else {
                let col_def = self.build_column_def(col);
                let position = if idx == 0 {
                    " FIRST".to_string()
                } else {
                    format!(
                        " AFTER {}",
                        self.quote_identifier(&new.columns[idx - 1].name)
                    )
                };

                statements.push(format!(
                    "ALTER TABLE {} ADD COLUMN {}{};",
                    table_name, col_def, position
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

        for (name, idx) in &original_indexes {
            if !new_indexes.contains_key(name) {
                if idx.is_primary {
                    statements.push(format!("ALTER TABLE {} DROP PRIMARY KEY;", table_name));
                } else {
                    statements.push(format!(
                        "ALTER TABLE {} DROP INDEX {};",
                        table_name,
                        self.quote_identifier(name)
                    ));
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
                    let idx_type = if idx.is_unique {
                        "UNIQUE INDEX"
                    } else {
                        "INDEX"
                    };
                    statements.push(format!(
                        "ALTER TABLE {} ADD {} {} ({});",
                        table_name,
                        idx_type,
                        self.quote_identifier(name),
                        idx_cols.join(", ")
                    ));
                }
            }
        }

        let mut options_changed = false;
        let mut option_parts: Vec<String> = Vec::new();

        if original.options.engine != new.options.engine
            && original.options.engine.is_some()
            && new.options.engine.is_some()
        {
            if let Some(engine) = &new.options.engine {
                option_parts.push(format!("ENGINE={}", engine));
                options_changed = true;
            }
        }

        if original.options.charset != new.options.charset
            && original.options.charset.is_some()
            && new.options.charset.is_some()
        {
            if let Some(charset) = &new.options.charset {
                option_parts.push(format!("DEFAULT CHARSET={}", charset));
                options_changed = true;
            }
        }

        if original.options.collation != new.options.collation
            && original.options.collation.is_some()
            && new.options.collation.is_some()
        {
            if let Some(collation) = &new.options.collation {
                option_parts.push(format!("COLLATE={}", collation));
                options_changed = true;
            }
        }

        if original.options.comment != new.options.comment
            && !original.options.comment.is_empty()
            && !new.options.comment.is_empty()
        {
            option_parts.push(format!(
                "COMMENT='{}'",
                new.options.comment.replace("'", "''")
            ));
            options_changed = true;
        }

        if options_changed && !option_parts.is_empty() {
            statements.push(format!(
                "ALTER TABLE {} {};",
                table_name,
                option_parts.join(" ")
            ));
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

impl Default for MySqlPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::DatabasePlugin;
    use crate::types::{ColumnDefinition, IndexDefinition, TableDesign, TableOptions};

    fn create_plugin() -> MySqlPlugin {
        MySqlPlugin::new()
    }

    // ==================== Basic Plugin Info Tests ====================

    #[test]
    fn test_plugin_name() {
        let plugin = create_plugin();
        assert_eq!(plugin.name(), DatabaseType::MySQL);
    }

    #[test]
    fn test_quote_identifier() {
        let plugin = create_plugin();
        assert_eq!(plugin.quote_identifier("table_name"), "`table_name`");
        assert_eq!(plugin.quote_identifier("column"), "`column`");
        assert_eq!(plugin.quote_identifier("col`umn"), "`col``umn`");
    }

    // ==================== DDL SQL Generation Tests ====================

    #[test]
    fn test_drop_database() {
        let plugin = create_plugin();
        let sql = plugin.drop_database("test_db");
        assert!(sql.contains("DROP DATABASE"));
        assert!(sql.contains("`test_db`"));
    }

    #[test]
    fn test_drop_table() {
        let plugin = create_plugin();
        let sql = plugin.drop_table("test_db", None, "users");
        assert!(sql.contains("DROP TABLE"));
        assert!(sql.contains("`test_db`"));
        assert!(sql.contains("`users`"));
    }

    #[test]
    fn test_truncate_table() {
        let plugin = create_plugin();
        let sql = plugin.truncate_table("test_db", "users");
        assert!(sql.contains("TRUNCATE TABLE"));
        assert!(sql.contains("`users`"));
    }

    #[test]
    fn test_rename_table() {
        let plugin = create_plugin();
        let sql = plugin.rename_table("test_db", "old_name", "new_name");
        assert!(sql.contains("RENAME TABLE"));
        assert!(sql.contains("`old_name`"));
        assert!(sql.contains("`new_name`"));
    }

    #[test]
    fn test_build_backup_table_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_backup_table_sql("test_db", None, "orders", "orders_bak");
        assert!(sql.contains("CREATE TABLE `test_db`.`orders_bak` LIKE `test_db`.`orders`;"));
        assert!(
            sql.contains("INSERT INTO `test_db`.`orders_bak` SELECT * FROM `test_db`.`orders`;")
        );
    }

    #[test]
    fn test_drop_view() {
        let plugin = create_plugin();
        let sql = plugin.drop_view("test_db", "my_view");
        assert!(sql.contains("DROP VIEW"));
        assert!(sql.contains("`my_view`"));
    }

    // ==================== Database Operations Tests ====================

    #[test]
    fn test_build_create_database_sql() {
        let plugin = create_plugin();
        let mut field_values = HashMap::new();
        field_values.insert("charset".to_string(), "utf8mb4".to_string());
        field_values.insert("collation".to_string(), "utf8mb4_unicode_ci".to_string());

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "new_db".to_string(),
            field_values,
        };

        let sql = plugin.build_create_database_sql(&request);
        assert!(sql.contains("CREATE DATABASE"));
        assert!(sql.contains("`new_db`"));
        assert!(sql.contains("utf8mb4"));
        assert!(sql.contains("utf8mb4_unicode_ci"));
    }

    #[test]
    fn test_build_create_database_sql_escapes_identifier() {
        let plugin = create_plugin();
        let mut field_values = HashMap::new();
        field_values.insert("charset".to_string(), "utf8mb4".to_string());
        field_values.insert("collation".to_string(), "utf8mb4_general_ci".to_string());

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "new`db".to_string(),
            field_values,
        };

        let sql = plugin.build_create_database_sql(&request);
        assert!(sql.contains("CREATE DATABASE"));
        assert!(sql.contains("`new``db`"));
    }

    #[test]
    fn test_build_modify_database_sql() {
        let plugin = create_plugin();
        let mut field_values = HashMap::new();
        field_values.insert("charset".to_string(), "utf8mb4".to_string());
        field_values.insert("collation".to_string(), "utf8mb4_bin".to_string());

        let request = crate::plugin::DatabaseOperationRequest {
            database_name: "my_db".to_string(),
            field_values,
        };

        let sql = plugin.build_modify_database_sql(&request);
        assert!(sql.contains("ALTER DATABASE"));
        assert!(sql.contains("`my_db`"));
        assert!(sql.contains("utf8mb4_bin"));
    }

    #[test]
    fn test_build_drop_database_sql() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_database_sql("old_db");
        assert_eq!(sql, "DROP DATABASE `old_db`;");
    }

    #[test]
    fn test_build_drop_database_sql_escapes_identifier() {
        let plugin = create_plugin();
        let sql = plugin.build_drop_database_sql("old`db");
        assert_eq!(sql, "DROP DATABASE `old``db`;");
    }

    // ==================== Column Definition Tests ====================

    #[test]
    fn test_build_column_def_simple() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("id")
            .data_type("INT")
            .nullable(false)
            .primary_key(true)
            .auto_increment(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("`id`"));
        assert!(def.contains("INT"));
        assert!(def.contains("NOT NULL"));
        assert!(def.contains("AUTO_INCREMENT"));
    }

    #[test]
    fn test_build_column_def_with_length() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("name")
            .data_type("VARCHAR")
            .length(255)
            .nullable(true);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("`name`"));
        assert!(def.contains("VARCHAR(255)"));
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

    #[test]
    fn test_build_column_def_with_comment() {
        let plugin = create_plugin();
        let col = ColumnDefinition::new("email")
            .data_type("VARCHAR")
            .length(100)
            .comment("User email address");

        let def = plugin.build_column_def(&col);
        assert!(def.contains("COMMENT 'User email address'"));
    }

    #[test]
    fn test_build_column_def_unsigned() {
        let plugin = create_plugin();
        let mut col = ColumnDefinition::new("age").data_type("INT");
        col.is_unsigned = true;
        col.is_nullable = false;

        let def = plugin.build_column_def(&col);
        assert!(def.contains("UNSIGNED"));
    }

    #[test]
    fn test_build_column_def_decimal() {
        let plugin = create_plugin();
        let mut col = ColumnDefinition::new("price").data_type("DECIMAL");
        col.length = Some(10);

        let def = plugin.build_column_def(&col);
        assert!(def.contains("DECIMAL(10)"));
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
                    .data_type("VARCHAR")
                    .length(100),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_create_table_sql(&design);
        assert!(sql.contains("CREATE TABLE `users`"));
        assert!(sql.contains("`id`"));
        assert!(sql.contains("INT"));
        assert!(sql.contains("AUTO_INCREMENT"));
        assert!(sql.contains("`name`"));
        assert!(sql.contains("VARCHAR(100)"));
        assert!(sql.contains("PRIMARY KEY"));
    }

    #[test]
    fn test_build_create_table_sql_with_options() {
        let plugin = create_plugin();
        let design = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "products".to_string(),
            columns: vec![ColumnDefinition::new("id")
                .data_type("INT")
                .nullable(false)
                .primary_key(true)],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions {
                engine: Some("InnoDB".to_string()),
                charset: Some("utf8mb4".to_string()),
                collation: Some("utf8mb4_unicode_ci".to_string()),
                comment: "Product table".to_string(),
                auto_increment: None,
            },
        };

        let sql = plugin.build_create_table_sql(&design);
        assert!(sql.contains("ENGINE=InnoDB"));
        assert!(sql.contains("DEFAULT CHARSET=utf8mb4"));
        assert!(sql.contains("COLLATE=utf8mb4_unicode_ci"));
        assert!(sql.contains("COMMENT='Product table'"));
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
        assert!(sql.contains("INDEX `idx_user_id`"));
        assert!(sql.contains("UNIQUE INDEX `idx_email`"));
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
                    .data_type("VARCHAR")
                    .length(100),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("ADD COLUMN"));
        assert!(sql.contains("`email`"));
    }

    #[test]
    fn test_build_alter_table_sql_add_column_no_reorder() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INT"),
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
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("email")
                    .data_type("VARCHAR")
                    .length(100),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .length(50),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("ADD COLUMN"));
        assert!(sql.contains("`email`"));
        assert!(!sql.contains("MODIFY COLUMN `name`"));
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
            columns: vec![ColumnDefinition::new("id").data_type("INT")],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("DROP COLUMN"));
        assert!(sql.contains("`old_column`"));
    }

    #[test]
    fn test_build_alter_table_sql_modify_column() {
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
        assert!(sql.contains("MODIFY COLUMN"));
        assert!(sql.contains("`name`"));
        assert!(sql.contains("VARCHAR(100)"));
    }

    #[test]
    fn test_build_alter_table_sql_reorder_columns() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .length(50),
                ColumnDefinition::new("age").data_type("INT"),
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
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("age").data_type("INT"),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert!(sql.contains("MODIFY COLUMN"));
        assert!(sql.contains("`name`"));
        assert!(sql.contains(" AFTER `id`") || sql.contains(" FIRST"));
    }

    #[test]
    fn test_build_alter_table_sql_reorder_with_modify_column() {
        let plugin = create_plugin();

        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .length(50),
                ColumnDefinition::new("age").data_type("INT"),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("age").data_type("INT"),
                ColumnDefinition::new("id").data_type("INT"),
                ColumnDefinition::new("name")
                    .data_type("VARCHAR")
                    .length(120),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        let modify_count = sql.matches("MODIFY COLUMN `name`").count();
        assert_eq!(modify_count, 1);
        assert!(sql.contains("VARCHAR(120)"));
        assert!(sql.contains(" AFTER `id`") || sql.contains(" FIRST"));
    }

    #[test]
    fn test_build_alter_table_sql_no_changes_with_text_metadata() {
        let plugin = create_plugin();

        let column = ColumnDefinition {
            name: "session_id".to_string(),
            data_type: "varchar".to_string(),
            length: Some(255),
            is_nullable: false,
            comment: "会话ID".to_string(),
            charset: Some("utf8mb4".to_string()),
            collation: Some("utf8mb4_general_ci".to_string()),
            ..Default::default()
        };
        let original = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "task_execution_record".to_string(),
            columns: vec![column.clone()],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };
        let new = TableDesign {
            database_name: "test_db".to_string(),
            table_name: "task_execution_record".to_string(),
            columns: vec![column],
            indexes: vec![],
            foreign_keys: vec![],
            options: TableOptions::default(),
        };

        let sql = plugin.build_alter_table_sql(&original, &new);
        assert_eq!(sql, "-- No changes detected");
    }

    // ==================== Charset & Collation Tests ====================

    #[test]
    fn test_get_charsets() {
        let plugin = create_plugin();
        let charsets = plugin.get_charsets();

        assert!(!charsets.is_empty());
        assert!(charsets.iter().any(|c| c.name == "utf8mb4"));
        assert!(charsets.iter().any(|c| c.name == "latin1"));
        assert!(charsets.iter().any(|c| c.name == "gbk"));
    }

    #[test]
    fn test_get_collations_utf8mb4() {
        let plugin = create_plugin();
        let collations = plugin.get_collations("utf8mb4");

        assert!(!collations.is_empty());
        assert!(collations.iter().any(|c| c.name == "utf8mb4_general_ci"));
        assert!(collations.iter().any(|c| c.name == "utf8mb4_unicode_ci"));
        assert!(collations.iter().any(|c| c.name == "utf8mb4_bin"));
    }

    #[test]
    fn test_get_collations_latin1() {
        let plugin = create_plugin();
        let collations = plugin.get_collations("latin1");

        assert!(!collations.is_empty());
        assert!(collations.iter().any(|c| c.name == "latin1_swedish_ci"));
    }

    #[test]
    fn test_get_collations_unknown() {
        let plugin = create_plugin();
        let collations = plugin.get_collations("unknown_charset");
        assert!(collations.is_empty());
    }

    // ==================== Data Types Tests ====================

    #[test]
    fn test_get_data_types() {
        let plugin = create_plugin();
        let types = plugin.get_data_types();

        assert!(!types.is_empty());
        assert!(types.iter().any(|t| t.0 == "INT"));
        assert!(types.iter().any(|t| t.0 == "VARCHAR"));
        assert!(types.iter().any(|t| t.0 == "TEXT"));
        assert!(types.iter().any(|t| t.0 == "DATETIME"));
        assert!(types.iter().any(|t| t.0 == "JSON"));
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

        assert!(info.keywords.iter().any(|(k, _)| *k == "AUTO_INCREMENT"));
        assert!(info
            .functions
            .iter()
            .any(|(f, _)| f.starts_with("GROUP_CONCAT")));
        assert!(info.operators.iter().any(|(o, _)| *o == "REGEXP"));
    }
}
