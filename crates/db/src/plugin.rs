use crate::connection::{DbConnection, DbError};
use crate::executor::{SqlResult, SqlSource, StatementType};
use crate::import_export::{
    formats::{
        CsvFormatHandler, JsonFormatHandler, SqlFormatHandler, TxtFormatHandler, XmlFormatHandler,
    },
    DataFormat, ExportConfig, ExportProgressSender, ExportResult, FormatHandler, ImportConfig,
    ImportProgressSender, ImportResult,
};
use crate::streaming_parser::StreamingSqlParser;
use crate::types::*;
use crate::QueryResult;
use anyhow::{anyhow, bail, Error, Result};
use async_trait::async_trait;
use one_core::storage::manager::get_queries_dir;
use one_core::storage::{DatabaseType, DbConnectionConfig};
use rust_i18n::t;
use sqlparser::ast;
use sqlparser::ast::{Expr, SetExpr, Statement, TableFactor};
use sqlparser::dialect::Dialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::io;
use tracing::log::error;

/// Standard SQL functions common to most databases
pub const STANDARD_SQL_FUNCTIONS: &[(&str, &str)] = &[
    // String functions
    ("CONCAT(str1, str2, ...)", "Concatenate strings"),
    ("SUBSTRING(str, pos, len)", "Extract substring"),
    ("LENGTH(str)", "String length"),
    ("UPPER(str)", "Convert to uppercase"),
    ("LOWER(str)", "Convert to lowercase"),
    ("TRIM(str)", "Remove leading/trailing spaces"),
    ("LTRIM(str)", "Remove leading spaces"),
    ("RTRIM(str)", "Remove trailing spaces"),
    ("REPLACE(str, from, to)", "Replace occurrences"),
    ("REVERSE(str)", "Reverse string"),
    ("LEFT(str, len)", "Left substring"),
    ("RIGHT(str, len)", "Right substring"),
    // Numeric functions
    ("ABS(x)", "Absolute value"),
    ("CEIL(x)", "Round up"),
    ("FLOOR(x)", "Round down"),
    ("ROUND(x, d)", "Round to decimal places"),
    ("MOD(x, y)", "Modulo operation"),
    ("POWER(x, y)", "Power function"),
    ("SQRT(x)", "Square root"),
    ("SIGN(x)", "Sign of number (-1, 0, 1)"),
    // Date/Time functions
    ("NOW()", "Current date and time"),
    ("CURRENT_DATE", "Current date"),
    ("CURRENT_TIME", "Current time"),
    ("CURRENT_TIMESTAMP", "Current timestamp"),
    // Aggregate functions
    ("COUNT(*)", "Count rows"),
    ("COUNT(DISTINCT col)", "Count distinct values"),
    ("SUM(col)", "Sum of values"),
    ("AVG(col)", "Average value"),
    ("MIN(col)", "Minimum value"),
    ("MAX(col)", "Maximum value"),
    // Control flow
    ("COALESCE(val1, val2, ...)", "First non-NULL value"),
    ("NULLIF(val1, val2)", "Return NULL if equal"),
    ("CASE WHEN ... THEN ... END", "Case expression"),
    // Type conversion
    ("CAST(expr AS type)", "Type conversion"),
];

/// Standard SQL keywords common to most databases
pub const STANDARD_SQL_KEYWORDS: &[(&str, &str)] = &[
    ("IF EXISTS", "Conditional existence check"),
    ("IF NOT EXISTS", "Conditional non-existence check"),
];

/// SQL completion information for a specific database type
#[derive(Clone, Default)]
pub struct SqlCompletionInfo {
    /// Database-specific keywords (e.g., LIMIT for MySQL, FETCH for PostgreSQL)
    pub keywords: Vec<(&'static str, &'static str)>,
    /// Database-specific functions with documentation
    pub functions: Vec<(&'static str, &'static str)>,
    /// Database-specific operators
    pub operators: Vec<(&'static str, &'static str)>,
    /// Database-specific data types for CREATE TABLE etc.
    pub data_types: Vec<(&'static str, &'static str)>,
    /// Database-specific snippets (e.g., common query patterns)
    pub snippets: Vec<(&'static str, &'static str, &'static str)>, // (label, insert_text, doc)
}

/// Database operation request
#[derive(Clone, Debug)]
pub struct DatabaseOperationRequest {
    pub database_name: String,
    pub field_values: HashMap<String, String>,
}

impl SqlCompletionInfo {
    /// Create completion info with standard SQL functions and keywords included
    pub fn with_standard_sql(mut self) -> Self {
        // Prepend standard functions
        let mut all_functions = STANDARD_SQL_FUNCTIONS.to_vec();
        all_functions.extend(self.functions);
        self.functions = all_functions;

        // Prepend standard keywords
        let mut all_keywords = STANDARD_SQL_KEYWORDS.to_vec();
        all_keywords.extend(self.keywords);
        self.keywords = all_keywords;

        self
    }
}

/// Database plugin trait for supporting multiple database types
#[async_trait]
pub trait DatabasePlugin: Send + Sync {
    fn name(&self) -> DatabaseType;

    /// Quote an identifier (table name, column name, etc.) according to database syntax
    fn quote_identifier(&self, identifier: &str) -> String;

    /// Get database-specific SQL completion information
    fn get_completion_info(&self) -> SqlCompletionInfo {
        SqlCompletionInfo::default()
    }

    async fn create_connection(
        &self,
        config: DbConnectionConfig,
    ) -> Result<Box<dyn DbConnection + Send + Sync>, DbError>;

    // === Database/Schema Level Operations ===
    async fn list_databases(&self, connection: &dyn DbConnection) -> Result<Vec<String>>;

    async fn list_databases_view(&self, connection: &dyn DbConnection) -> Result<ObjectView>;
    async fn list_databases_detailed(
        &self,
        connection: &dyn DbConnection,
    ) -> Result<Vec<DatabaseInfo>>;

    /// Whether this database supports schemas (e.g., PostgreSQL, MSSQL)
    fn supports_schema(&self) -> bool {
        false
    }

    /// Whether this database uses schemas as top-level nodes instead of databases.
    /// Oracle uses this because it connects via service_name and then lists schemas (users).
    fn uses_schema_as_database(&self) -> bool {
        false
    }

    /// Whether this database supports sequences (e.g., PostgreSQL, Oracle, MSSQL)
    fn supports_sequences(&self) -> bool {
        false
    }

    /// Whether this database supports rowid for row identification (e.g., Oracle, SQLite)
    fn supports_rowid(&self) -> bool {
        false
    }

    /// Get the rowid column name for this database
    fn rowid_column_name(&self) -> &'static str {
        "rowid"
    }

    /// Get the SQL dialect for this database type
    fn sql_dialect(&self) -> Box<dyn Dialect>;

    /// 创建 SQL 解析器（统一接口，支持脚本和文件）
    fn create_parser(&self, source: SqlSource) -> io::Result<StreamingSqlParser> {
        StreamingSqlParser::from_source(source, self.name())
    }

    /// Format SQL for display (each database can customize this)
    fn format_sql(&self, sql: &str) -> String {
        crate::format_sql(sql)
    }

    /// Check if a SQL statement is a query (returns rows)
    fn is_query_statement(&self, sql: &str) -> bool {
        if let Ok(statements) = Parser::parse_sql(self.sql_dialect().as_ref(), sql) {
            if let Some(stmt) = statements.first() {
                return is_query_stmt(stmt);
            }
        }
        is_query_statement_fallback(sql)
    }

    /// Split SQL text into statements using the database-specific parser.
    fn split_sql_statements(&self, sql: &str) -> Vec<String> {
        let trimmed = sql.trim();
        let Ok(parser) = self.create_parser(SqlSource::Script(trimmed.to_string())) else {
            return vec![trimmed.to_string()];
        };

        let mut statements = Vec::new();
        for statement in parser {
            match statement {
                Ok(statement) => {
                    let statement = statement.trim();
                    if !statement.is_empty() {
                        statements.push(statement.to_string());
                    }
                }
                Err(_) => return vec![trimmed.to_string()],
            }
        }

        if statements.is_empty() {
            return vec![trimmed.to_string()];
        }

        statements
    }

    /// Build a single EXPLAIN statement for this database type.
    fn build_explain_statement(&self, sql: &str) -> String {
        let sql = sql.trim();
        match self.name() {
            DatabaseType::MySQL
            | DatabaseType::PostgreSQL
            | DatabaseType::DuckDB
            | DatabaseType::ClickHouse => {
                format!("EXPLAIN {sql}")
            }
            DatabaseType::SQLite => format!("EXPLAIN QUERY PLAN {sql}"),
            DatabaseType::MSSQL => {
                format!("SET SHOWPLAN_TEXT ON;\n{sql}\nSET SHOWPLAN_TEXT OFF;")
            }
            DatabaseType::Oracle => {
                format!(
                    "EXPLAIN PLAN FOR {sql};\nSELECT PLAN_TABLE_OUTPUT FROM TABLE(DBMS_XPLAN.DISPLAY())"
                )
            }
        }
    }

    /// Check whether SQL already is an EXPLAIN/SHOWPLAN statement or script.
    fn is_explain_statement(&self, sql: &str) -> bool {
        let trimmed = sql.trim_start();
        let upper = trimmed.to_ascii_uppercase();

        match self.name() {
            DatabaseType::MSSQL => upper.starts_with("SET SHOWPLAN_TEXT ON"),
            _ => upper.starts_with("EXPLAIN"),
        }
    }

    /// Build EXPLAIN SQL for all query statements in a SQL script.
    fn build_explain_sql(&self, sql: &str) -> Option<String> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return None;
        }

        if matches!(self.name(), DatabaseType::MSSQL) && self.is_explain_statement(trimmed) {
            return Some(trimmed.to_string());
        }

        let separator = if matches!(self.name(), DatabaseType::MSSQL) {
            "\n"
        } else {
            ";\n"
        };

        let explain_statements = self
            .split_sql_statements(trimmed)
            .into_iter()
            .filter_map(|statement| {
                if self.is_explain_statement(&statement) {
                    return Some(statement.trim().to_string());
                }
                if self.is_query_statement(&statement) {
                    return Some(self.build_explain_statement(&statement));
                }
                None
            })
            .collect::<Vec<_>>();

        if explain_statements.is_empty() {
            return None;
        }

        Some(explain_statements.join(separator))
    }

    /// Determine the statement category
    fn classify_statement(&self, sql: &str) -> StatementType {
        if let Ok(statements) = Parser::parse_sql(self.sql_dialect().as_ref(), sql) {
            if let Some(stmt) = statements.first() {
                return classify_stmt(stmt);
            }
        }
        classify_fallback(sql)
    }

    /// Check if a SELECT query might be editable
    /// Returns None if cannot determine, Some(table_name) if looks like simple single-table query
    fn analyze_select_editability(&self, sql: &str) -> Option<String> {
        if let Ok(statements) = Parser::parse_sql(self.sql_dialect().as_ref(), sql) {
            if let Some(Statement::Query(query)) = statements.first() {
                return analyze_query_editability(query);
            }
        }
        analyze_select_editability_fallback(sql)
    }

    /// List schemas in a database (for databases that support schemas)
    async fn list_schemas(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// List schemas view (for databases that support schemas)
    async fn list_schemas_view(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
    ) -> Result<ObjectView> {
        Ok(ObjectView::default())
    }

    // === Table Operations ===
    async fn list_tables(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<TableInfo>>;

    async fn list_tables_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<ObjectView>;
    async fn list_columns(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<ColumnInfo>>;
    async fn list_columns_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<ObjectView>;
    async fn list_indexes(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
        table: &str,
    ) -> Result<Vec<IndexInfo>>;

    async fn list_indexes_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Result<ObjectView>;

    /// List foreign keys for a table
    async fn list_foreign_keys(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
        _table: &str,
    ) -> Result<Vec<ForeignKeyDefinition>> {
        Ok(Vec::new())
    }

    /// List triggers for a specific table
    async fn list_table_triggers(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
        _table: &str,
    ) -> Result<Vec<TriggerInfo>> {
        Ok(Vec::new())
    }

    /// List check constraints for a specific table
    async fn list_table_checks(
        &self,
        _connection: &dyn DbConnection,
        _database: &str,
        _schema: Option<String>,
        _table: &str,
    ) -> Result<Vec<CheckInfo>> {
        Ok(Vec::new())
    }

    // === View Operations ===
    async fn list_views(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<ViewInfo>>;

    async fn list_views_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView>;

    // === Function Operations ===

    fn supports_functions(&self) -> bool {
        true
    }

    async fn list_functions(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>>;

    async fn list_functions_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView>;

    fn supports_procedures(&self) -> bool {
        true
    }
    // === Procedure Operations ===
    async fn list_procedures(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<FunctionInfo>>;

    async fn list_procedures_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView>;

    // === Trigger Operations ===
    async fn list_triggers(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<Vec<TriggerInfo>>;

    async fn list_triggers_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView>;

    // === Sequence Operations ===
    async fn list_sequences(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<String>,
    ) -> Result<Vec<SequenceInfo>>;

    async fn list_sequences_view(
        &self,
        connection: &dyn DbConnection,
        database: &str,
    ) -> Result<ObjectView>;

    // === Helper Methods ===
    fn build_column_definition(&self, column: &ColumnInfo, include_name: bool) -> String;

    // === Database Management Operations ===
    /// Build SQL for creating a new database
    fn build_create_database_sql(&self, request: &DatabaseOperationRequest) -> String;

    /// Build SQL for modifying an existing database
    fn build_modify_database_sql(&self, request: &DatabaseOperationRequest) -> String;

    /// Build SQL for dropping a database
    fn build_drop_database_sql(&self, database_name: &str) -> String;

    // === Schema Management Operations ===
    /// Build SQL for creating a new schema
    fn build_create_schema_sql(&self, schema_name: &str) -> String {
        format!("CREATE SCHEMA {}", self.quote_identifier(schema_name))
    }

    /// Build SQL for dropping a schema
    fn build_drop_schema_sql(&self, schema_name: &str) -> String {
        format!("DROP SCHEMA {}", self.quote_identifier(schema_name))
    }

    /// Build SQL for adding/updating schema comment
    /// Returns None if the database doesn't support schema comments
    fn build_comment_schema_sql(&self, _schema_name: &str, _comment: &str) -> Option<String> {
        None
    }

    async fn build_database_tree(
        &self,
        connection: &dyn DbConnection,
        node: &DbNode,
    ) -> Result<Vec<DbNode>> {
        let id = &node.id;
        let databases = self.list_databases(connection).await?;
        Ok(databases
            .into_iter()
            .map(|db| {
                DbNode::new(
                    format!("{}:{}", &node.id, db),
                    db.clone(),
                    DbNodeType::Database,
                    node.id.clone(),
                    node.database_type,
                )
                .with_parent_context(id)
            })
            .collect())
    }

    async fn build_schema_tree(
        &self,
        connection: &dyn DbConnection,
        node: &DbNode,
    ) -> Result<Vec<DbNode>> {
        let id = &node.id;
        let schemas;
        let mut metadata: HashMap<String, String> = HashMap::new();
        if self.uses_schema_as_database() {
            schemas = self.list_schemas(connection, "").await?;
            metadata.insert("database".to_string(), "".to_string());
        } else {
            let database = node
                .get_database_name()
                .ok_or_else(|| anyhow!("Database name is required"))?;
            schemas = self.list_schemas(connection, &database).await?;
            metadata.insert("database".to_string(), database);
        }
        let mut nodes = Vec::new();
        for schema in schemas {
            let schema_node = DbNode::new(
                format!("{}:{}", id, schema),
                schema.clone(),
                DbNodeType::Schema,
                node.connection_id.clone(),
                node.database_type,
            )
            .with_parent_context(id)
            .with_metadata(metadata.clone());
            nodes.push(schema_node);
        }

        Ok(nodes)
    }

    async fn build_database_or_schema_children(
        &self,
        connection: &dyn DbConnection,
        node: &DbNode,
        schema: Option<String>,
    ) -> Result<Vec<DbNode>> {
        let mut nodes = Vec::new();
        let database = &*node
            .get_database_name()
            .ok_or_else(|| anyhow!("Database name not"))?;
        let id = &node.id;
        let mut metadata: HashMap<String, String> = HashMap::new();
        metadata.insert("database".to_string(), database.to_string());
        if let Some(s) = schema.clone() {
            metadata.insert("schema".to_string(), s.to_string());
        }

        let tables = self
            .list_tables(connection, database, schema.clone())
            .await?;
        let table_count = tables.len();
        let mut table_folder = DbNode::new(
            format!("{}:table_folder", id),
            "DbTree.Tables".to_string(),
            DbNodeType::TablesFolder,
            node.connection_id.clone(),
            node.database_type,
        )
        .with_parent_context(id)
        .with_metadata(metadata.clone());
        if table_count > 0 {
            let children: Vec<DbNode> = tables
                .into_iter()
                .map(|table_info| {
                    let mut meta: HashMap<String, String> = metadata.clone();
                    if let Some(comment) = &table_info.comment {
                        if !comment.is_empty() {
                            meta.insert("comment".to_string(), comment.clone());
                        }
                    }

                    DbNode::new(
                        format!("{}:table_folder:{}", id, table_info.name),
                        table_info.name.clone(),
                        DbNodeType::Table,
                        node.connection_id.clone(),
                        node.database_type,
                    )
                    .with_parent_context(format!("{}:table_folder", id))
                    .with_metadata(meta)
                })
                .collect();
            table_folder.set_children(children)
        }
        nodes.push(table_folder);

        let views = self
            .list_views(connection, database, schema.clone())
            .await?;
        let view_count = views.len();
        let mut views_folder = DbNode::new(
            format!("{}:views_folder", id),
            "DbTree.Views".to_string(),
            DbNodeType::ViewsFolder,
            node.connection_id.clone(),
            node.database_type,
        )
        .with_parent_context(id)
        .with_metadata(metadata.clone());
        if view_count > 0 {
            let children: Vec<DbNode> = views
                .into_iter()
                .map(|view| {
                    let mut meta: HashMap<String, String> = metadata.clone();
                    if let Some(comment) = view.comment {
                        meta.insert("comment".to_string(), comment);
                    }

                    let mut vnode = DbNode::new(
                        format!("{}:views_folder:{}", id, view.name),
                        view.name.clone(),
                        DbNodeType::View,
                        node.connection_id.clone(),
                        node.database_type,
                    )
                    .with_parent_context(format!("{}:views_folder", id));

                    if !meta.is_empty() {
                        vnode = vnode.with_metadata(meta);
                    }
                    vnode
                })
                .collect();
            views_folder.set_children(children);
        }
        nodes.push(views_folder);

        // Functions folder
        if self.supports_functions() {
            let functions = self
                .list_functions(connection, database)
                .await
                .unwrap_or_default();
            let function_count = functions.len();
            let mut functions_folder = DbNode::new(
                format!("{}:functions_folder", id),
                "DbTree.Functions".to_string(),
                DbNodeType::FunctionsFolder,
                node.connection_id.clone(),
                node.database_type,
            )
            .with_parent_context(id)
            .with_metadata(metadata.clone());
            if function_count > 0 {
                let children: Vec<DbNode> = functions
                    .into_iter()
                    .map(|func| {
                        DbNode::new(
                            format!("{}:functions_folder:{}", id, func.name),
                            func.name.clone(),
                            DbNodeType::Function,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(format!("{}:functions_folder", id))
                        .with_metadata(metadata.clone())
                    })
                    .collect();
                functions_folder.set_children(children);
            }
            nodes.push(functions_folder);
        }

        // Procedures folder
        if self.supports_procedures() {
            let procedures = self
                .list_procedures(connection, database)
                .await
                .unwrap_or_default();
            let procedure_count = procedures.len();
            let mut procedures_folder = DbNode::new(
                format!("{}:procedures_folder", id),
                "DbTree.Procedures".to_string(),
                DbNodeType::ProceduresFolder,
                node.connection_id.clone(),
                node.database_type,
            )
            .with_parent_context(id)
            .with_metadata(metadata.clone());
            if procedure_count > 0 {
                let children: Vec<DbNode> = procedures
                    .into_iter()
                    .map(|proc| {
                        DbNode::new(
                            format!("{}:procedures_folder:{}", id, proc.name),
                            proc.name.clone(),
                            DbNodeType::Procedure,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(format!("{}:procedures_folder", id))
                        .with_metadata(metadata.clone())
                    })
                    .collect();
                procedures_folder.set_children(children);
            }
            nodes.push(procedures_folder);
        }

        // Sequences folder (only for databases that support sequences)
        if self.supports_sequences() {
            let sequences = self
                .list_sequences(connection, database, schema)
                .await
                .unwrap_or_default();
            let sequence_count = sequences.len();
            let mut sequences_folder = DbNode::new(
                format!("{}:sequences_folder", id),
                "DbTree.Sequences".to_string(),
                DbNodeType::SequencesFolder,
                node.connection_id.clone(),
                node.database_type,
            )
            .with_parent_context(id)
            .with_metadata(metadata.clone());
            if sequence_count > 0 {
                let children: Vec<DbNode> = sequences
                    .into_iter()
                    .map(|seq| {
                        let mut seq_meta: HashMap<String, String> = metadata.clone();
                        if let Some(start) = seq.start_value {
                            seq_meta.insert("start_value".to_string(), start.to_string());
                        }
                        if let Some(inc) = seq.increment {
                            seq_meta.insert("increment".to_string(), inc.to_string());
                        }
                        if let Some(min) = seq.min_value {
                            seq_meta.insert("min_value".to_string(), min.to_string());
                        }
                        if let Some(max) = seq.max_value {
                            seq_meta.insert("max_value".to_string(), max.to_string());
                        }
                        DbNode::new(
                            format!("{}:sequences_folder:{}", id, seq.name),
                            seq.name.clone(),
                            DbNodeType::Sequence,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(format!("{}:sequences_folder", id))
                        .with_metadata(seq_meta)
                    })
                    .collect();
                sequences_folder.set_children(children);
            }
            nodes.push(sequences_folder);
        }

        let queries_folder = self.load_queries(node, metadata.clone()).await?;
        nodes.push(queries_folder);
        Ok(nodes)
    }

    async fn load_queries(
        &self,
        node: &DbNode,
        metadata: HashMap<String, String>,
    ) -> Result<DbNode> {
        let node_id_for_queries = node.id.clone();
        let connection_id_for_queries = node.connection_id.clone();

        let queries_folder_node = DbNode::new(
            format!("{}:queries_folder", &node_id_for_queries),
            "DbTree.Queries".to_string(),
            DbNodeType::QueriesFolder,
            connection_id_for_queries.clone(),
            node.database_type,
        )
        .with_parent_context(node_id_for_queries.clone())
        .with_metadata(metadata);

        Ok(queries_folder_node)
    }

    async fn load_node_children(
        &self,
        connection: &dyn DbConnection,
        node: &DbNode,
    ) -> Result<Vec<DbNode>> {
        let id = &node.id;
        match node.node_type {
            DbNodeType::Connection => {
                if self.uses_schema_as_database() {
                    self.build_schema_tree(connection, node).await
                } else {
                    self.build_database_tree(connection, node).await
                }
            }
            DbNodeType::Database => {
                if self.supports_schema() {
                    self.build_schema_tree(connection, node).await
                } else {
                    self.build_database_or_schema_children(connection, node, None)
                        .await
                }
            }
            DbNodeType::Schema => {
                let schema_name = node.get_schema_name();
                self.build_database_or_schema_children(connection, node, schema_name)
                    .await
            }
            DbNodeType::TablesFolder
            | DbNodeType::ViewsFolder
            | DbNodeType::FunctionsFolder
            | DbNodeType::ProceduresFolder
            | DbNodeType::SequencesFolder => {
                if node.children_loaded {
                    return Ok(node.children.clone());
                }
                self.load_schema_folder_children(connection, node, id).await
            }
            DbNodeType::QueriesFolder => {
                if node.children_loaded {
                    return Ok(node.children.clone());
                }
                self.load_queries_children(node, id).await
            }
            DbNodeType::Table => self.load_table_children(connection, node, id).await,
            DbNodeType::ColumnsFolder
            | DbNodeType::IndexesFolder
            | DbNodeType::ForeignKeysFolder
            | DbNodeType::TriggersFolder
            | DbNodeType::ChecksFolder => {
                if node.children_loaded {
                    return Ok(node.children.clone());
                }
                self.load_table_folder_children(connection, node, id).await
            }
            _ => Ok(Vec::new()),
        }
    }

    async fn load_schema_folder_children(
        &self,
        connection: &dyn DbConnection,
        node: &DbNode,
        id: &str,
    ) -> Result<Vec<DbNode>> {
        let database = &*node.get_database_name().unwrap_or_default();
        let schema = node.get_schema_name();
        match node.node_type {
            DbNodeType::TablesFolder => {
                let tables = self.list_tables(connection, database, schema).await?;
                Ok(tables
                    .into_iter()
                    .map(|t| {
                        let mut meta = node.metadata.clone();
                        if let Some(comment) = &t.comment {
                            if !comment.is_empty() {
                                meta.insert("comment".to_string(), comment.clone());
                            }
                        }
                        DbNode::new(
                            format!("{}:{}", id, t.name),
                            t.name.clone(),
                            DbNodeType::Table,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(id)
                        .with_metadata(meta)
                    })
                    .collect())
            }
            DbNodeType::ViewsFolder => {
                let views = self.list_views(connection, database, schema).await?;
                Ok(views
                    .into_iter()
                    .map(|v| {
                        let mut meta = node.metadata.clone();
                        if let Some(comment) = v.comment {
                            meta.insert("comment".to_string(), comment);
                        }
                        DbNode::new(
                            format!("{}:{}", id, v.name),
                            v.name.clone(),
                            DbNodeType::View,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(id)
                        .with_metadata(meta)
                    })
                    .collect())
            }
            DbNodeType::FunctionsFolder => {
                let functions = self
                    .list_functions(connection, database)
                    .await
                    .unwrap_or_default();
                Ok(functions
                    .into_iter()
                    .map(|f| {
                        DbNode::new(
                            format!("{}:{}", id, f.name),
                            f.name.clone(),
                            DbNodeType::Function,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(id)
                        .with_metadata(node.metadata.clone())
                    })
                    .collect())
            }
            DbNodeType::ProceduresFolder => {
                let procedures = self
                    .list_procedures(connection, database)
                    .await
                    .unwrap_or_default();
                Ok(procedures
                    .into_iter()
                    .map(|p| {
                        DbNode::new(
                            format!("{}:{}", id, p.name),
                            p.name.clone(),
                            DbNodeType::Procedure,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(id)
                        .with_metadata(node.metadata.clone())
                    })
                    .collect())
            }
            DbNodeType::SequencesFolder => {
                let sequences = self
                    .list_sequences(connection, database, schema.clone())
                    .await
                    .unwrap_or_default();
                let filtered: Vec<_> = match schema {
                    Some(s) => sequences
                        .into_iter()
                        .filter(|seq| seq.name.starts_with(&format!("{}.", s)))
                        .collect(),
                    None => sequences,
                };
                Ok(filtered
                    .into_iter()
                    .map(|seq| {
                        let mut meta = node.metadata.clone();
                        if let Some(v) = seq.start_value {
                            meta.insert("start_value".to_string(), v.to_string());
                        }
                        if let Some(v) = seq.increment {
                            meta.insert("increment".to_string(), v.to_string());
                        }
                        if let Some(v) = seq.min_value {
                            meta.insert("min_value".to_string(), v.to_string());
                        }
                        if let Some(v) = seq.max_value {
                            meta.insert("max_value".to_string(), v.to_string());
                        }
                        DbNode::new(
                            format!("{}:{}", id, seq.name),
                            seq.name.clone(),
                            DbNodeType::Sequence,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_parent_context(id)
                        .with_metadata(meta)
                    })
                    .collect())
            }
            _ => Ok(Vec::new()),
        }
    }

    async fn load_queries_children(&self, node: &DbNode, id: &str) -> Result<Vec<DbNode>> {
        let metadata = node.metadata.clone();
        let database_name = node.get_database_name().unwrap_or_default();
        let database_type = node.database_type.as_str();

        let queries_dir = match get_queries_dir() {
            Ok(dir) => dir,
            Err(e) => {
                error!("Failed to get queries directory: {}", e);
                return Ok(Vec::new());
            }
        };

        let query_path = queries_dir
            .join(&database_type)
            .join(&node.connection_id)
            .join(&database_name);

        if !query_path.exists() {
            return Ok(Vec::new());
        }

        let entries = match std::fs::read_dir(&query_path) {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to read queries directory {:?}: {}", query_path, e);
                return Ok(Vec::new());
            }
        };

        let mut query_nodes = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "sql") {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let mut meta = metadata.clone();
                meta.insert("file_path".to_string(), path.to_string_lossy().to_string());

                let query_node = DbNode::new(
                    format!("{}:{}", id, file_name),
                    file_name.clone(),
                    DbNodeType::NamedQuery,
                    node.connection_id.clone(),
                    node.database_type,
                )
                .with_parent_context(id)
                .with_metadata(meta);

                query_nodes.push(query_node);
            }
        }

        query_nodes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(query_nodes)
    }

    async fn load_table_children(
        &self,
        connection: &dyn DbConnection,
        node: &DbNode,
        id: &str,
    ) -> Result<Vec<DbNode>> {
        let db = &*node
            .get_database_name()
            .ok_or_else(|| anyhow::anyhow!("Database name not found"))?;
        let schema = node.get_schema_name();
        let table = &*node
            .get_table_name()
            .ok_or_else(|| anyhow::anyhow!("Table name not found"))?;

        let mut folder_metadata: HashMap<String, String> = node.metadata.clone();
        folder_metadata.insert("table".to_string(), table.to_string());

        let mut children = Vec::new();

        let columns = self
            .list_columns(connection, db, schema.clone(), table)
            .await?;
        children.push(
            self.build_table_subfolder(
                node,
                id,
                "columns_folder",
                "DbTree.Columns",
                DbNodeType::ColumnsFolder,
                &folder_metadata,
                columns
                    .into_iter()
                    .map(|c| {
                        (c.name.clone(), DbNodeType::Column, {
                            let mut m = folder_metadata.clone();
                            m.insert("type".to_string(), c.data_type);
                            m.insert("is_nullable".to_string(), c.is_nullable.to_string());
                            m.insert("is_primary_key".to_string(), c.is_primary_key.to_string());
                            m
                        })
                    })
                    .collect(),
            ),
        );

        let indexes: Vec<_> = self
            .list_indexes(connection, db, schema.clone(), table)
            .await?
            .into_iter()
            .filter(|idx| idx.name.to_uppercase() != "PRIMARY")
            .collect();
        children.push(
            self.build_table_subfolder(
                node,
                id,
                "indexes_folder",
                "DbTree.Indexes",
                DbNodeType::IndexesFolder,
                &folder_metadata,
                indexes
                    .into_iter()
                    .map(|idx| {
                        (idx.name.clone(), DbNodeType::Index, {
                            let mut m = folder_metadata.clone();
                            m.insert("unique".to_string(), idx.is_unique.to_string());
                            m.insert("columns".to_string(), idx.columns.join(", "));
                            m
                        })
                    })
                    .collect(),
            ),
        );

        let foreign_keys = self
            .list_foreign_keys(connection, db, schema.clone(), table)
            .await
            .unwrap_or_default();
        children.push(
            self.build_table_subfolder(
                node,
                id,
                "foreign_keys_folder",
                "DbTree.ForeignKeys",
                DbNodeType::ForeignKeysFolder,
                &folder_metadata,
                foreign_keys
                    .into_iter()
                    .map(|fk| {
                        (fk.name.clone(), DbNodeType::ForeignKey, {
                            let mut m = folder_metadata.clone();
                            m.insert("columns".to_string(), fk.columns.join(", "));
                            m.insert("ref_table".to_string(), fk.ref_table.clone());
                            m.insert("ref_columns".to_string(), fk.ref_columns.join(", "));
                            m
                        })
                    })
                    .collect(),
            ),
        );

        let triggers = self
            .list_table_triggers(connection, db, schema.clone(), table)
            .await
            .unwrap_or_default();
        children.push(
            self.build_table_subfolder(
                node,
                id,
                "triggers_folder",
                "DbTree.Triggers",
                DbNodeType::TriggersFolder,
                &folder_metadata,
                triggers
                    .into_iter()
                    .map(|t| {
                        (t.name.clone(), DbNodeType::Trigger, {
                            let mut m = folder_metadata.clone();
                            m.insert("event".to_string(), t.event.clone());
                            m.insert("timing".to_string(), t.timing.clone());
                            m
                        })
                    })
                    .collect(),
            ),
        );

        let checks = self
            .list_table_checks(connection, db, schema.clone(), table)
            .await
            .unwrap_or_default();
        children.push(
            self.build_table_subfolder(
                node,
                id,
                "checks_folder",
                "DbTree.Checks",
                DbNodeType::ChecksFolder,
                &folder_metadata,
                checks
                    .into_iter()
                    .map(|c| {
                        (c.name.clone(), DbNodeType::Check, {
                            let mut m = folder_metadata.clone();
                            if let Some(def) = &c.definition {
                                m.insert("definition".to_string(), def.clone());
                            }
                            m
                        })
                    })
                    .collect(),
            ),
        );

        Ok(children)
    }

    fn build_table_subfolder(
        &self,
        node: &DbNode,
        parent_id: &str,
        folder_suffix: &str,
        display_prefix: &str,
        folder_type: DbNodeType,
        folder_metadata: &HashMap<String, String>,
        items: Vec<(String, DbNodeType, HashMap<String, String>)>,
    ) -> DbNode {
        let folder_id = format!("{}:{}", parent_id, folder_suffix);
        let count = items.len();
        let mut folder = DbNode::new(
            folder_id.clone(),
            display_prefix,
            folder_type,
            node.connection_id.clone(),
            node.database_type,
        )
        .with_parent_context(parent_id)
        .with_metadata(folder_metadata.clone());
        if count > 0 {
            let child_nodes: Vec<DbNode> = items
                .into_iter()
                .map(|(name, node_type, meta)| {
                    DbNode::new(
                        format!("{}:{}", folder_id, name),
                        name,
                        node_type,
                        node.connection_id.clone(),
                        node.database_type,
                    )
                    .with_metadata(meta)
                    .with_parent_context(&folder_id)
                })
                .collect();
            folder.set_children(child_nodes);
        }
        folder
    }

    async fn load_table_folder_children(
        &self,
        connection: &dyn DbConnection,
        node: &DbNode,
        id: &str,
    ) -> Result<Vec<DbNode>> {
        let database = &*node
            .get_database_name()
            .ok_or_else(|| anyhow::anyhow!("Database name not found"))?;
        let schema = node.get_schema_name();
        let table = &*node
            .get_table_name()
            .ok_or_else(|| anyhow::anyhow!("Table name not found"))?;
        match node.node_type {
            DbNodeType::ColumnsFolder => {
                let columns = self
                    .list_columns(connection, database, schema, table)
                    .await?;
                Ok(columns
                    .into_iter()
                    .map(|c| {
                        let mut meta = node.metadata.clone();
                        meta.insert("type".to_string(), c.data_type);
                        meta.insert("is_nullable".to_string(), c.is_nullable.to_string());
                        meta.insert("is_primary_key".to_string(), c.is_primary_key.to_string());
                        DbNode::new(
                            format!("{}:{}", id, c.name),
                            c.name,
                            DbNodeType::Column,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_metadata(meta)
                        .with_parent_context(id)
                    })
                    .collect())
            }
            DbNodeType::IndexesFolder => {
                let indexes: Vec<_> = self
                    .list_indexes(connection, database, schema, table)
                    .await?
                    .into_iter()
                    .filter(|idx| idx.name.to_uppercase() != "PRIMARY")
                    .collect();
                Ok(indexes
                    .into_iter()
                    .map(|idx| {
                        let mut meta = node.metadata.clone();
                        meta.insert("unique".to_string(), idx.is_unique.to_string());
                        meta.insert("columns".to_string(), idx.columns.join(", "));
                        DbNode::new(
                            format!("{}:{}", id, idx.name),
                            idx.name,
                            DbNodeType::Index,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_metadata(meta)
                        .with_parent_context(id)
                    })
                    .collect())
            }
            DbNodeType::ForeignKeysFolder => {
                let foreign_keys = self
                    .list_foreign_keys(connection, database, schema, table)
                    .await
                    .unwrap_or_default();
                Ok(foreign_keys
                    .into_iter()
                    .map(|fk| {
                        let mut meta = node.metadata.clone();
                        meta.insert("columns".to_string(), fk.columns.join(", "));
                        meta.insert("ref_table".to_string(), fk.ref_table.clone());
                        meta.insert("ref_columns".to_string(), fk.ref_columns.join(", "));
                        DbNode::new(
                            format!("{}:{}", id, fk.name),
                            fk.name,
                            DbNodeType::ForeignKey,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_metadata(meta)
                        .with_parent_context(id)
                    })
                    .collect())
            }
            DbNodeType::TriggersFolder => {
                let triggers = self
                    .list_table_triggers(connection, database, schema, table)
                    .await
                    .unwrap_or_default();
                Ok(triggers
                    .into_iter()
                    .map(|t| {
                        let mut meta = node.metadata.clone();
                        meta.insert("event".to_string(), t.event.clone());
                        meta.insert("timing".to_string(), t.timing.clone());
                        DbNode::new(
                            format!("{}:{}", id, t.name),
                            t.name,
                            DbNodeType::Trigger,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_metadata(meta)
                        .with_parent_context(id)
                    })
                    .collect())
            }
            DbNodeType::ChecksFolder => {
                let checks = self
                    .list_table_checks(connection, database, schema, table)
                    .await
                    .unwrap_or_default();
                Ok(checks
                    .into_iter()
                    .map(|c| {
                        let mut meta = node.metadata.clone();
                        if let Some(def) = &c.definition {
                            meta.insert("definition".to_string(), def.clone());
                        }
                        DbNode::new(
                            format!("{}:{}", id, c.name),
                            c.name,
                            DbNodeType::Check,
                            node.connection_id.clone(),
                            node.database_type,
                        )
                        .with_metadata(meta)
                        .with_parent_context(id)
                    })
                    .collect())
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Format pagination SQL clause. Override for databases with different syntax.
    fn format_pagination(&self, limit: usize, offset: usize, _order_clause: &str) -> String {
        format!(" LIMIT {} OFFSET {}", limit, offset)
    }

    /// Format table reference for queries. Override for databases with different syntax.
    /// - MySQL: `database`.`table`
    /// - PostgreSQL: "schema"."table" (uses schema, ignores database since connection is db-specific)
    /// - MSSQL: [database]..[table] or [database].[schema].[table]
    fn format_table_reference(&self, database: &str, _schema: Option<&str>, table: &str) -> String {
        format!(
            "{}.{}",
            self.quote_identifier(database),
            self.quote_identifier(table)
        )
    }

    /// Format table reference for SQL export output (omit database, keep schema when present).
    fn format_export_table_reference(
        &self,
        _database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> String {
        match schema {
            Some(schema) => format!(
                "{}.{}",
                self.quote_identifier(schema),
                self.quote_identifier(table)
            ),
            None => self.quote_identifier(table),
        }
    }

    // === Table Data Operations ===
    /// Query table data with pagination, filtering and sorting
    async fn query_table_data(
        &self,
        connection: &dyn DbConnection,
        request: TableDataRequest,
    ) -> Result<TableDataResponse> {
        let start_time = std::time::Instant::now();

        let where_clause = match request.where_clause {
            Some(ref c) if !c.trim().is_empty() => format!(" WHERE {}", c.trim()),
            _ => String::new(),
        };
        let order_clause = match request.order_by_clause {
            Some(ref c) if !c.trim().is_empty() => format!(" ORDER BY {}", c.trim()),
            _ => String::new(),
        };

        // Calculate offset
        let offset = (request.page.saturating_sub(1)) * request.page_size;

        // Build table reference
        let table_ref = self.format_table_reference(
            &request.database,
            request.schema.as_deref(),
            &request.table,
        );

        // Build count query
        let count_sql = format!("SELECT COUNT(*) FROM {}{}", table_ref, where_clause);

        // Get total count
        let total_count = match connection.query(&count_sql).await? {
            SqlResult::Query(result) => result
                .rows
                .first()
                .and_then(|r| r.first())
                .and_then(|v| v.as_ref())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0),
            _ => 0,
        };

        // Query with pagination, include rowid if supported
        let pagination = self.format_pagination(request.page_size, offset, &order_clause);
        let data_sql = if self.supports_rowid() {
            let rowid_col = self.rowid_column_name();
            format!(
                "SELECT {} AS __rowid__, t.* FROM {} t{}{}{}",
                rowid_col, table_ref, where_clause, order_clause, pagination
            )
        } else {
            format!(
                "SELECT * FROM {}{}{}{}",
                table_ref, where_clause, order_clause, pagination
            )
        };
        let sql_result = connection.query(&data_sql).await?;
        let duration = start_time.elapsed().as_millis();

        let query_result = match sql_result {
            SqlResult::Query(query_result) => Ok::<QueryResult, Error>(query_result),
            SqlResult::Exec(_) => bail!(t!("Error.query_type_error")),
            SqlResult::Error(sql_error_info) => bail!(sql_error_info.message),
        }?;

        Ok(TableDataResponse {
            query_result,
            total_count,
            page: request.page,
            page_size: request.page_size,
            duration,
        })
    }

    /// Generate SQL preview for table changes without executing them
    fn generate_table_changes_sql(&self, request: &TableSaveRequest) -> String {
        let mut sql_statements = Vec::new();

        for change in &request.changes {
            if let Some(sql) = self.build_table_change_sql(request, change) {
                sql_statements.push(sql);
            }
        }

        if sql_statements.is_empty() {
            t!("Error.no_changes").to_string()
        } else {
            sql_statements.join(";\n\n") + ";"
        }
    }

    // === Copy SQL Generation Methods ===

    /// Generate INSERT SQL statements for copying
    fn generate_copy_insert_sql(&self, request: &CopySqlRequest) -> String {
        if request.rows.is_empty() || request.column_names.is_empty() {
            return String::new();
        }

        let table_name = self.format_copy_table_name(request.schema.as_deref(), &request.table);
        let quoted_columns: Vec<String> = request
            .column_names
            .iter()
            .map(|c| self.quote_identifier(c))
            .collect();
        let columns_str = quoted_columns.join(", ");

        let mut statements = Vec::new();

        for row in &request.rows {
            let values: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, val)| {
                    let col_info = request.columns.get(i);
                    self.format_copy_value(val, col_info)
                })
                .collect();
            let values_str = values.join(", ");

            statements.push(format!(
                "INSERT INTO {} ({}) VALUES ({});",
                table_name, columns_str, values_str
            ));
        }

        statements.join("\n")
    }

    /// Generate INSERT SQL statements with column comments for copying
    fn generate_copy_insert_with_comments_sql(&self, request: &CopySqlRequest) -> String {
        if request.rows.is_empty() || request.column_names.is_empty() {
            return String::new();
        }

        let table_name = self.format_copy_table_name(request.schema.as_deref(), &request.table);

        // Generate column names with comments
        let columns_with_comments: Vec<String> = request
            .column_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let quoted = self.quote_identifier(name);
                if let Some(col_info) = request.columns.get(i) {
                    if let Some(comment) = &col_info.comment {
                        if !comment.is_empty() {
                            return format!("{} /* {} */", quoted, comment);
                        }
                    }
                }
                quoted
            })
            .collect();
        let columns_str = columns_with_comments.join(", ");

        let mut statements = Vec::new();

        for row in &request.rows {
            let values: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, val)| {
                    let col_info = request.columns.get(i);
                    self.format_copy_value(val, col_info)
                })
                .collect();
            let values_str = values.join(", ");

            statements.push(format!(
                "INSERT INTO {} ({}) VALUES ({});",
                table_name, columns_str, values_str
            ));
        }

        statements.join("\n")
    }

    /// Generate UPDATE SQL statements for copying
    fn generate_copy_update_sql(&self, request: &CopySqlRequest) -> String {
        if request.rows.is_empty() || request.column_names.is_empty() {
            return String::new();
        }

        let original_rows = request.original_rows.as_ref().unwrap_or(&request.rows);
        let table_name = self.format_copy_table_name(request.schema.as_deref(), &request.table);
        let mut statements = Vec::new();

        for (row, original_row) in request.rows.iter().zip(original_rows.iter()) {
            // Generate SET clause
            let set_parts: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, val)| {
                    let col_name = self.quote_identifier(
                        request
                            .column_names
                            .get(i)
                            .map(|s| s.as_str())
                            .unwrap_or(""),
                    );
                    let col_info = request.columns.get(i);
                    let value = self.format_copy_value(val, col_info);
                    format!("{} = {}", col_name, value)
                })
                .collect();
            let set_str = set_parts.join(", ");

            // Generate WHERE clause
            let where_str = self.generate_copy_where_clause(request, original_row);

            statements.push(format!(
                "UPDATE {} SET {} WHERE {};",
                table_name, set_str, where_str
            ));
        }

        statements.join("\n")
    }

    /// Generate DELETE SQL statements for copying
    fn generate_copy_delete_sql(&self, request: &CopySqlRequest) -> String {
        if request.rows.is_empty() || request.column_names.is_empty() {
            return String::new();
        }

        let table_name = self.format_copy_table_name(request.schema.as_deref(), &request.table);
        let mut statements = Vec::new();

        for row in &request.rows {
            let where_str = self.generate_copy_where_clause(request, row);
            statements.push(format!("DELETE FROM {} WHERE {};", table_name, where_str));
        }

        statements.join("\n")
    }

    /// Format table name for copy SQL (with optional schema)
    fn format_copy_table_name(&self, schema: Option<&str>, table: &str) -> String {
        let quoted_table = self.quote_identifier(table);
        match schema {
            Some(s) if !s.is_empty() => {
                format!("{}.{}", self.quote_identifier(s), quoted_table)
            }
            _ => quoted_table,
        }
    }

    /// Format a value for copy SQL based on column type
    fn format_copy_value(&self, value: &Option<String>, col_info: Option<&ColumnInfo>) -> String {
        match value {
            None => "NULL".to_string(),
            Some(v) if v.is_empty() => "NULL".to_string(),
            Some(v) => {
                if let Some(info) = col_info {
                    let data_type = info.data_type.to_uppercase();
                    // Numeric types don't need quotes
                    if self.is_numeric_type(&data_type) {
                        if v.parse::<f64>().is_ok() || v.parse::<i64>().is_ok() {
                            return v.clone();
                        }
                    }
                    // Boolean types
                    if self.is_boolean_type(&data_type) {
                        return self.format_boolean_value(v);
                    }
                    // Binary types
                    if self.is_binary_type(&data_type) {
                        return self.format_binary_value(v);
                    }
                }
                // Default: escape as string
                self.escape_copy_string(v)
            }
        }
    }

    /// Check if data type is numeric
    fn is_numeric_type(&self, data_type: &str) -> bool {
        let numeric_types = [
            "INT",
            "INTEGER",
            "BIGINT",
            "SMALLINT",
            "TINYINT",
            "MEDIUMINT",
            "DECIMAL",
            "NUMERIC",
            "FLOAT",
            "DOUBLE",
            "REAL",
            "NUMBER",
            "MONEY",
            "SMALLMONEY",
            "BIT",
        ];
        numeric_types.iter().any(|t| data_type.contains(t))
    }

    /// Check if data type is boolean
    fn is_boolean_type(&self, data_type: &str) -> bool {
        data_type.contains("BOOL") || data_type == "BIT"
    }

    /// Check if data type is binary
    fn is_binary_type(&self, data_type: &str) -> bool {
        let binary_types = ["BLOB", "BINARY", "VARBINARY", "BYTEA", "RAW"];
        binary_types.iter().any(|t| data_type.contains(t))
    }

    /// Format boolean value (database-specific, can be overridden)
    fn format_boolean_value(&self, v: &str) -> String {
        if v == "1" || v.eq_ignore_ascii_case("true") {
            "TRUE".to_string()
        } else {
            "FALSE".to_string()
        }
    }

    /// Format binary value (database-specific, can be overridden)
    fn format_binary_value(&self, v: &str) -> String {
        self.escape_copy_string(v)
    }

    /// Escape string for copy SQL (database-specific, can be overridden)
    fn escape_copy_string(&self, s: &str) -> String {
        let escaped = s.replace('\'', "''");
        format!("'{}'", escaped)
    }

    /// Generate WHERE clause for copy SQL
    fn generate_copy_where_clause(
        &self,
        request: &CopySqlRequest,
        row: &[Option<String>],
    ) -> String {
        // Prefer primary key columns
        let primary_key_indices: Vec<usize> = request
            .columns
            .iter()
            .enumerate()
            .filter(|(_, col)| col.is_primary_key)
            .map(|(i, _)| i)
            .collect();

        let indices_to_use = if !primary_key_indices.is_empty() {
            primary_key_indices
        } else {
            // If no primary key, use all columns
            (0..request.column_names.len()).collect()
        };

        let conditions: Vec<String> = indices_to_use
            .iter()
            .filter_map(|&i| {
                let col_name = request.column_names.get(i)?;
                let val = row.get(i)?;
                let col_info = request.columns.get(i);

                let quoted_col = self.quote_identifier(col_name);
                match val {
                    None => Some(format!("{} IS NULL", quoted_col)),
                    Some(v) if v.is_empty() => Some(format!("{} IS NULL", quoted_col)),
                    _ => {
                        let formatted = self.format_copy_value(val, col_info);
                        Some(format!("{} = {}", quoted_col, formatted))
                    }
                }
            })
            .collect();

        if conditions.is_empty() {
            "1=1".to_string() // Safe fallback
        } else {
            conditions.join(" AND ")
        }
    }

    fn build_table_change_sql(
        &self,
        request: &TableSaveRequest,
        change: &TableRowChange,
    ) -> Option<String> {
        let table_ident = self.format_table_reference(
            &request.database,
            request.schema.as_deref(),
            &request.table,
        );

        match change {
            TableRowChange::Added { data } => {
                if data.is_empty() {
                    return None;
                }
                let columns: Vec<String> = request
                    .columns
                    .iter()
                    .map(|column| self.quote_identifier(&*column.name))
                    .collect();
                let values: Vec<String> = data
                    .iter()
                    .map(|value| {
                        if value == "NULL" || value.is_empty() {
                            "NULL".to_string()
                        } else {
                            format!("'{}'", value.replace('\'', "''"))
                        }
                    })
                    .collect();

                Some(format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    table_ident,
                    columns.join(", "),
                    values.join(", ")
                ))
            }
            TableRowChange::Updated {
                original_data,
                changes,
                rowid,
            } => {
                if changes.is_empty() {
                    return None;
                }

                let set_clause: Vec<String> = changes
                    .iter()
                    .map(|change| {
                        let column_name = if change.column_name.is_empty() {
                            request
                                .columns
                                .get(change.column_index)
                                .map(|c| c.name.clone())
                                .unwrap_or_default()
                        } else {
                            change.column_name.clone()
                        };
                        let ident = self.quote_identifier(&column_name);
                        let value = if change.new_value == "NULL" {
                            "NULL".to_string()
                        } else {
                            format!("'{}'", change.new_value.replace('\'', "''"))
                        };
                        format!("{} = {}", ident, value)
                    })
                    .collect();

                if let Some(rid) = rowid {
                    let rowid_col = self.rowid_column_name();
                    return Some(format!(
                        "UPDATE {} SET {} WHERE {} = '{}'",
                        table_ident,
                        set_clause.join(", "),
                        rowid_col,
                        rid.replace('\'', "''")
                    ));
                }

                let (where_clause, limit_clause) =
                    self.build_where_and_limit_clause(request, original_data);

                if limit_clause == " __SQLITE_ROWID_LIMIT__" {
                    let simple_table = self.quote_identifier(&request.table);
                    Some(format!(
                        "UPDATE {} SET {} WHERE rowid IN (SELECT rowid FROM {} WHERE {} LIMIT 1)",
                        table_ident,
                        set_clause.join(", "),
                        simple_table,
                        where_clause
                    ))
                } else {
                    Some(format!(
                        "UPDATE {} SET {}{}{}{}",
                        table_ident,
                        set_clause.join(", "),
                        if where_clause.is_empty() {
                            ""
                        } else {
                            " WHERE "
                        },
                        where_clause,
                        limit_clause
                    ))
                }
            }
            TableRowChange::Deleted {
                original_data,
                rowid,
            } => {
                if let Some(rid) = rowid {
                    let rowid_col = self.rowid_column_name();
                    return Some(format!(
                        "DELETE FROM {} WHERE {} = '{}'",
                        table_ident,
                        rowid_col,
                        rid.replace('\'', "''")
                    ));
                }

                let (where_clause, limit_clause) =
                    self.build_where_and_limit_clause(request, original_data);

                if limit_clause == " __SQLITE_ROWID_LIMIT__" {
                    let simple_table = self.quote_identifier(&request.table);
                    Some(format!(
                        "DELETE FROM {} WHERE rowid IN (SELECT rowid FROM {} WHERE {} LIMIT 1)",
                        table_ident, simple_table, where_clause
                    ))
                } else {
                    Some(format!(
                        "DELETE FROM {}{}{}{}",
                        table_ident,
                        if where_clause.is_empty() {
                            ""
                        } else {
                            " WHERE "
                        },
                        where_clause,
                        limit_clause
                    ))
                }
            }
        }
    }

    fn build_limit_clause(&self) -> String;

    fn build_where_and_limit_clause(
        &self,
        request: &TableSaveRequest,
        original_data: &[String],
    ) -> (String, String);

    fn build_table_change_where_clause(
        &self,
        request: &TableSaveRequest,
        original_data: &[String],
    ) -> String {
        let column_names: Vec<&str> = request.columns.iter().map(|c| c.name.as_str()).collect();

        let primary_key_indices: Vec<usize> = request
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_primary_key)
            .map(|(i, _)| i)
            .collect();

        let unique_key_indices: Vec<usize> = request
            .index_infos
            .iter()
            .filter(|idx| idx.is_unique)
            .flat_map(|idx| {
                idx.columns
                    .iter()
                    .filter_map(|col_name| column_names.iter().position(|n| n == col_name))
            })
            .collect();

        let indices: Vec<usize> = if !primary_key_indices.is_empty() {
            primary_key_indices
        } else if !unique_key_indices.is_empty() {
            unique_key_indices
        } else {
            (0..column_names.len()).collect()
        };

        let mut parts = Vec::new();
        for index in indices {
            if let (Some(column), Some(value)) = (column_names.get(index), original_data.get(index))
            {
                let ident = self.quote_identifier(column);
                if value == "NULL" {
                    parts.push(format!("{} IS NULL", ident));
                } else {
                    parts.push(format!("{} = '{}'", ident, value.replace('\'', "''")));
                }
            }
        }

        parts.join(" AND ")
    }

    // === Export Operations ===
    /// Export table CREATE statement
    async fn export_table_create_sql(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<&str>,
        table: &str,
    ) -> Result<String> {
        let columns = self
            .list_columns(connection, database, schema.map(|s| s.to_string()), table)
            .await?;
        if columns.is_empty() {
            return Ok(String::new());
        }

        let table_ref = self.format_export_table_reference(database, schema, table);
        let mut sql = format!("CREATE TABLE {} (\n", table_ref);
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                sql.push_str(",\n");
            }
            sql.push_str("    ");
            sql.push_str(&self.build_column_definition(col, true));
        }
        sql.push_str("\n)");
        Ok(sql)
    }

    /// Export table data as INSERT statements
    async fn export_table_data_sql(
        &self,
        connection: &dyn DbConnection,
        database: &str,
        schema: Option<&str>,
        table: &str,
        where_clause: Option<&str>,
        limit: Option<usize>,
    ) -> Result<String> {
        let table_ref = self.format_table_reference(database, schema, table);
        let mut select_sql = format!("SELECT * FROM {}", table_ref);
        if let Some(where_c) = where_clause {
            select_sql.push_str(" WHERE ");
            select_sql.push_str(where_c);
        }
        if let Some(lim) = limit {
            let pagination = self.format_pagination(lim, 0, "");
            select_sql.push_str(&pagination);
        }

        let result = connection
            .query(&select_sql)
            .await
            .map_err(|e| anyhow::anyhow!("Query failed: {}", e))?;

        let mut output = String::new();
        if let SqlResult::Query(query_result) = result {
            if !query_result.rows.is_empty() {
                let table_ident = self.format_export_table_reference(database, schema, table);
                for row in &query_result.rows {
                    output.push_str("INSERT INTO ");
                    output.push_str(&table_ident);
                    output.push_str(" (");
                    for (i, col) in query_result.columns.iter().enumerate() {
                        if i > 0 {
                            output.push_str(", ");
                        }
                        output.push_str(&self.quote_identifier(col));
                    }
                    output.push_str(") VALUES (");

                    for (i, value) in row.iter().enumerate() {
                        if i > 0 {
                            output.push_str(", ");
                        }
                        match value {
                            Some(v) => {
                                output.push('\'');
                                output.push_str(&v.replace('\'', "''"));
                                output.push('\'');
                            }
                            None => output.push_str("NULL"),
                        }
                    }

                    output.push_str(");\n");
                }
            }
        }

        Ok(output)
    }

    // === Charset and Collation ===
    /// Get list of available character sets for this database
    fn get_charsets(&self) -> Vec<CharsetInfo> {
        vec![]
    }

    /// Get collations for a specific charset
    fn get_collations(&self, _charset: &str) -> Vec<CollationInfo> {
        vec![]
    }

    // === Data Types ===
    /// Get list of available data types for this database
    /// Returns a slice of (type_name, description) tuples
    fn get_data_types(&self) -> &[(&'static str, &'static str)] {
        // Default implementation with common types
        &[
            ("INT", "Integer number"),
            ("VARCHAR", "Variable-length string"),
            ("TEXT", "Long text"),
            ("DATE", "Date"),
            ("DATETIME", "Date and time"),
            ("BOOLEAN", "True/False"),
            ("DECIMAL", "Decimal number"),
        ]
    }

    /// Parse a column type string into its components
    /// e.g., "VARCHAR(255)" -> ParsedColumnType { base_type: "VARCHAR", length: Some(255), ... }
    fn parse_column_type(&self, type_str: &str) -> ParsedColumnType {
        let upper = type_str.to_uppercase();
        let is_unsigned = upper.contains("UNSIGNED");
        let is_auto_increment = upper.contains("AUTO_INCREMENT");

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

    /// Check if a data type is an enum or set type (database-specific)
    fn is_enum_type(&self, _type_name: &str) -> bool {
        false
    }

    // === DDL Operations ===
    /// Drop database
    fn drop_database(&self, database: &str) -> String {
        format!(
            "DROP DATABASE IF EXISTS {}",
            self.quote_identifier(database)
        )
    }

    /// Drop table
    fn drop_table(&self, database: &str, schema: Option<&str>, table: &str) -> String {
        // Default implementation for MySQL/ClickHouse: database.table
        // PostgreSQL/SQL Server with schema: database.schema.table or schema.table
        // Oracle: schema.table (database is ignored)
        if let Some(schema) = schema {
            format!(
                "DROP TABLE IF EXISTS {}.{}.{}",
                self.quote_identifier(database),
                self.quote_identifier(schema),
                self.quote_identifier(table)
            )
        } else {
            format!(
                "DROP TABLE IF EXISTS {}.{}",
                self.quote_identifier(database),
                self.quote_identifier(table)
            )
        }
    }

    /// Truncate table
    fn truncate_table(&self, _database: &str, table: &str) -> String {
        format!("TRUNCATE TABLE {}", self.quote_identifier(table))
    }

    /// Rename table
    fn rename_table(&self, database: &str, old_name: &str, new_name: &str) -> String;

    /// Build native backup-table SQL.
    /// 默认实现使用 `CREATE TABLE ... AS SELECT ...`，数据库插件可按方言覆盖。
    fn build_backup_table_sql(
        &self,
        _database: &str,
        _schema: Option<&str>,
        source_table: &str,
        target_table: &str,
    ) -> String {
        format!(
            "CREATE TABLE {} AS SELECT * FROM {};",
            self.quote_identifier(target_table),
            self.quote_identifier(source_table)
        )
    }

    /// Drop view
    fn drop_view(&self, _database: &str, view: &str) -> String {
        format!("DROP VIEW IF EXISTS {}", self.quote_identifier(view))
    }

    /// Build column definition from ColumnDefinition (for table designer)
    fn build_column_def(&self, col: &ColumnDefinition) -> String;

    /// Build CREATE TABLE SQL from TableDesign
    fn build_create_table_sql(&self, design: &TableDesign) -> String;

    /// Build ALTER TABLE SQL from original and new TableDesign
    /// Returns a series of ALTER TABLE statements for the differences
    fn build_alter_table_sql(&self, original: &TableDesign, new: &TableDesign) -> String;

    /// 生成单列重命名 SQL，默认使用标准 RENAME COLUMN 语法。
    /// MySQL 需使用 CHANGE COLUMN，MSSQL 需使用 sp_rename，应覆盖此方法。
    fn build_column_rename_sql(
        &self,
        table_name: &str,
        old_name: &str,
        new_name: &str,
        _new_column: Option<&ColumnDefinition>,
    ) -> String {
        let quoted_table = self.quote_identifier(table_name);
        let quoted_old = self.quote_identifier(old_name);
        let quoted_new = self.quote_identifier(new_name);
        format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {};",
            quoted_table, quoted_old, quoted_new
        )
    }

    /// 带列重命名支持的 ALTER TABLE SQL 生成。
    /// 默认实现：map_design_for_diff → build_alter_table_sql → 追加 rename。
    fn build_alter_table_sql_with_renames(
        &self,
        original: &TableDesign,
        new: &TableDesign,
        column_renames: &[(String, String)],
    ) -> String {
        let design_for_diff = map_design_for_diff(new, column_renames);
        let base_sql = self.build_alter_table_sql(original, &design_for_diff);
        let rename_statements: Vec<String> = column_renames
            .iter()
            .map(|(old_name, new_name)| {
                let new_column = new.columns.iter().find(|col| col.name == *new_name);
                self.build_column_rename_sql(&new.table_name, old_name, new_name, new_column)
            })
            .collect();
        merge_alter_sql(base_sql, rename_statements)
    }

    /// Check if a column definition has changed
    fn column_changed(&self, original: &ColumnDefinition, new: &ColumnDefinition) -> bool {
        original.data_type.to_uppercase() != new.data_type.to_uppercase()
            || original.length != new.length
            || original.precision != new.precision
            || original.scale != new.scale
            || original.is_nullable != new.is_nullable
            || original.is_auto_increment != new.is_auto_increment
            || original.is_unsigned != new.is_unsigned
            || original.default_value != new.default_value
            || original.comment != new.comment
            || original.charset != new.charset
            || original.collation != new.collation
    }

    /// Build type string for a column (used in ALTER statements)
    fn build_type_string(&self, col: &ColumnDefinition) -> String {
        let mut type_str = col.data_type.clone();
        if let Some(precision) = col.precision {
            if let Some(scale) = col.scale {
                type_str = format!("{}({},{})", type_str, precision, scale);
            } else {
                type_str = format!("{}({})", type_str, precision);
            }
        } else if let Some(len) = col.length {
            type_str = format!("{}({})", type_str, len);
        }
        type_str
    }

    // === Import/Export Operations ===

    /// Build INSERT statement for a single row
    fn build_insert_statement(&self, table: &str, columns: &[String], values: &[String]) -> String {
        let mut sql = format!("INSERT INTO {} (", self.quote_identifier(table));
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&self.quote_identifier(col));
        }
        sql.push_str(") VALUES (");
        for (i, val) in values.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&self.escape_sql_value(val));
        }
        sql.push(')');
        sql
    }

    /// Escape a string value for SQL (override for database-specific escaping)
    fn escape_sql_value(&self, value: &str) -> String {
        if value.is_empty() || value.eq_ignore_ascii_case("null") {
            "NULL".to_string()
        } else {
            format!("'{}'", value.replace('\'', "''"))
        }
    }

    /// Import data from the specified format
    async fn import_data(
        &self,
        connection: &dyn DbConnection,
        config: &ImportConfig,
        data: &str,
    ) -> Result<ImportResult> {
        self.import_data_with_progress(connection, config, data, "", None)
            .await
    }

    /// Import data with progress callback
    async fn import_data_with_progress(
        &self,
        connection: &dyn DbConnection,
        config: &ImportConfig,
        data: &str,
        file_name: &str,
        progress_tx: Option<ImportProgressSender>,
    ) -> Result<ImportResult>;

    /// Export data to the specified format
    async fn export_data(
        &self,
        connection: &dyn DbConnection,
        config: &ExportConfig,
    ) -> Result<ExportResult> {
        self.export_data_with_progress(connection, config, None)
            .await
    }

    /// Export data with progress callback
    async fn export_data_with_progress(
        &self,
        connection: &dyn DbConnection,
        config: &ExportConfig,
        progress_tx: Option<ExportProgressSender>,
    ) -> Result<ExportResult>;
}

/// 将 design 中被重命名的列名回退为旧名，以便与 original 做 diff 时不会产生误删/误增。
pub fn map_design_for_diff(
    design: &TableDesign,
    normalized_renames: &[(String, String)],
) -> TableDesign {
    let mut design_for_diff = design.clone();
    for (old_name, new_name) in normalized_renames {
        if let Some(column) = design_for_diff
            .columns
            .iter_mut()
            .find(|column| column.name == *new_name)
        {
            column.name = old_name.clone();
        }
        for index in &mut design_for_diff.indexes {
            for idx_col in &mut index.columns {
                if idx_col == new_name {
                    *idx_col = old_name.clone();
                }
            }
        }
    }
    design_for_diff
}

/// 合并 base ALTER SQL 和 rename 语句，跳过 "-- No changes" 前缀。
pub fn merge_alter_sql(base_sql: String, rename_statements: Vec<String>) -> String {
    let mut statements = Vec::new();
    let trimmed = base_sql.trim();
    if !trimmed.is_empty() && !trimmed.starts_with("-- No changes") {
        statements.push(trimmed.to_string());
    }
    statements.extend(rename_statements);

    if statements.is_empty() {
        "-- No changes detected".to_string()
    } else {
        statements.join("\n")
    }
}

/// Default import data implementation - can be called by database plugins
pub async fn default_import_data_with_progress(
    plugin: &dyn DatabasePlugin,
    connection: &dyn DbConnection,
    config: &ImportConfig,
    data: &str,
    file_name: &str,
    progress_tx: Option<ImportProgressSender>,
) -> Result<ImportResult> {
    match config.format {
        DataFormat::Sql => {
            SqlFormatHandler
                .import_with_progress(plugin, connection, config, data, file_name, progress_tx)
                .await
        }
        DataFormat::Json => {
            JsonFormatHandler
                .import_with_progress(plugin, connection, config, data, file_name, progress_tx)
                .await
        }
        DataFormat::Csv => {
            CsvFormatHandler
                .import_with_progress(plugin, connection, config, data, file_name, progress_tx)
                .await
        }
        DataFormat::Txt => {
            TxtFormatHandler
                .import_with_progress(plugin, connection, config, data, file_name, progress_tx)
                .await
        }
        DataFormat::Xml => {
            XmlFormatHandler
                .import_with_progress(plugin, connection, config, data, file_name, progress_tx)
                .await
        }
    }
}

/// Default export data implementation - can be called by database plugins
pub async fn default_export_data_with_progress(
    plugin: &dyn DatabasePlugin,
    connection: &dyn DbConnection,
    config: &ExportConfig,
    progress_tx: Option<ExportProgressSender>,
) -> Result<ExportResult> {
    match config.format {
        DataFormat::Sql => {
            SqlFormatHandler
                .export_with_progress(plugin, connection, config, progress_tx)
                .await
        }
        DataFormat::Json => {
            JsonFormatHandler
                .export_with_progress(plugin, connection, config, progress_tx)
                .await
        }
        DataFormat::Csv => {
            CsvFormatHandler
                .export_with_progress(plugin, connection, config, progress_tx)
                .await
        }
        DataFormat::Txt => {
            TxtFormatHandler
                .export_with_progress(plugin, connection, config, progress_tx)
                .await
        }
        DataFormat::Xml => {
            XmlFormatHandler
                .export_with_progress(plugin, connection, config, progress_tx)
                .await
        }
    }
}

pub fn is_query_stmt(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Query(_)
            | Statement::ShowTables { .. }
            | Statement::ShowColumns { .. }
            | Statement::ShowDatabases { .. }
            | Statement::ShowFunctions { .. }
            | Statement::ShowVariable { .. }
            | Statement::ShowVariables { .. }
            | Statement::ShowCreate { .. }
            | Statement::ShowStatus { .. }
            | Statement::ShowCollation { .. }
            | Statement::ExplainTable { .. }
            | Statement::Explain { .. }
            | Statement::Pragma { .. }
    )
}

pub fn is_query_statement_fallback(sql: &str) -> bool {
    let trimmed = sql.trim().to_uppercase();
    trimmed.starts_with("SELECT")
        || trimmed.starts_with("SHOW")
        || trimmed.starts_with("DESC")
        || trimmed.starts_with("DESCRIBE")
        || trimmed.starts_with("EXPLAIN")
        || trimmed.starts_with("WITH")
        || trimmed.starts_with("TABLE")
        || trimmed.starts_with("PRAGMA")
}

pub fn classify_stmt(stmt: &Statement) -> StatementType {
    if is_query_stmt(stmt) {
        return StatementType::Query;
    }

    match stmt {
        Statement::Insert(_)
        | Statement::Update { .. }
        | Statement::Delete(_)
        | Statement::Merge { .. } => StatementType::Dml,

        Statement::CreateTable { .. }
        | Statement::CreateView { .. }
        | Statement::CreateIndex(_)
        | Statement::CreateFunction { .. }
        | Statement::CreateProcedure { .. }
        | Statement::CreateTrigger { .. }
        | Statement::CreateSchema { .. }
        | Statement::CreateDatabase { .. }
        | Statement::CreateSequence { .. }
        | Statement::AlterTable { .. }
        | Statement::AlterView { .. }
        | Statement::AlterIndex { .. }
        | Statement::Drop { .. }
        | Statement::DropFunction { .. }
        | Statement::DropProcedure { .. }
        | Statement::DropTrigger { .. }
        | Statement::DropSecret { .. }
        | Statement::Truncate { .. }
        | Statement::RenameTable { .. } => StatementType::Ddl,

        Statement::StartTransaction { .. }
        | Statement::Commit { .. }
        | Statement::Rollback { .. }
        | Statement::Savepoint { .. } => StatementType::Transaction,

        Statement::Use(_) | Statement::Set(_) => StatementType::Command,

        _ => StatementType::Exec,
    }
}

pub fn classify_fallback(sql: &str) -> StatementType {
    let trimmed = sql.trim().to_uppercase();

    if is_query_statement_fallback(sql) {
        return StatementType::Query;
    }

    if trimmed.starts_with("INSERT")
        || trimmed.starts_with("UPDATE")
        || trimmed.starts_with("DELETE")
        || trimmed.starts_with("REPLACE")
    {
        return StatementType::Dml;
    }

    if trimmed.starts_with("CREATE")
        || trimmed.starts_with("ALTER")
        || trimmed.starts_with("DROP")
        || trimmed.starts_with("TRUNCATE")
        || trimmed.starts_with("RENAME")
    {
        return StatementType::Ddl;
    }

    if trimmed.starts_with("BEGIN")
        || trimmed.starts_with("COMMIT")
        || trimmed.starts_with("ROLLBACK")
        || trimmed.starts_with("START TRANSACTION")
    {
        return StatementType::Transaction;
    }

    if trimmed.starts_with("USE") || trimmed.starts_with("SET") {
        return StatementType::Command;
    }

    StatementType::Exec
}

pub fn analyze_query_editability(query: &Box<ast::Query>) -> Option<String> {
    let body = &query.body;

    let select = match body.as_ref() {
        SetExpr::Select(s) => s,
        _ => return None,
    };

    if select.distinct.is_some() {
        return None;
    }

    let has_group_by = match &select.group_by {
        ast::GroupByExpr::All(_) => true,
        ast::GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
    };
    if has_group_by {
        return None;
    }

    if select.having.is_some() {
        return None;
    }

    for item in &select.projection {
        if has_aggregate_function_in_select_item(item) {
            return None;
        }
    }

    if select.from.len() != 1 {
        return None;
    }

    let table_with_joins = &select.from[0];
    if !table_with_joins.joins.is_empty() {
        return None;
    }

    match &table_with_joins.relation {
        TableFactor::Table { name, .. } => {
            let table_name = name.to_string();
            Some(table_name)
        }
        _ => None,
    }
}

fn has_aggregate_function_in_select_item(item: &ast::SelectItem) -> bool {
    match item {
        ast::SelectItem::UnnamedExpr(expr) | ast::SelectItem::ExprWithAlias { expr, .. } => {
            has_aggregate_function(expr)
        }
        _ => false,
    }
}

fn has_aggregate_function(expr: &Expr) -> bool {
    match expr {
        Expr::Function(func) => {
            let name = func.name.to_string().to_uppercase();
            matches!(
                name.as_str(),
                "COUNT" | "SUM" | "AVG" | "MAX" | "MIN" | "GROUP_CONCAT" | "STRING_AGG"
            )
        }
        Expr::BinaryOp { left, right, .. } => {
            has_aggregate_function(left) || has_aggregate_function(right)
        }
        Expr::UnaryOp { expr, .. } => has_aggregate_function(expr),
        Expr::Nested(inner) => has_aggregate_function(inner),
        _ => false,
    }
}

pub fn analyze_select_editability_fallback(sql: &str) -> Option<String> {
    let upper = sql.trim().to_uppercase();

    if !upper.starts_with("SELECT") {
        return None;
    }

    let complex_keywords = [
        " JOIN ",
        " INNER JOIN ",
        " LEFT JOIN ",
        " RIGHT JOIN ",
        " OUTER JOIN ",
        " CROSS JOIN ",
        " FULL JOIN ",
        " UNION ",
        " INTERSECT ",
        " EXCEPT ",
        " GROUP BY ",
        " HAVING ",
        "DISTINCT",
        " DISTINCT ",
    ];

    for keyword in &complex_keywords {
        if upper.contains(keyword) {
            return None;
        }
    }

    let aggregate_functions = [
        "COUNT(",
        "SUM(",
        "AVG(",
        "MAX(",
        "MIN(",
        "GROUP_CONCAT(",
        "STRING_AGG(",
    ];

    for func in &aggregate_functions {
        if upper.contains(func) {
            return None;
        }
    }

    if let Some(from_pos) = upper.find(" FROM ") {
        let after_from = &sql[from_pos + 6..].trim();
        let table_name = after_from
            .split_whitespace()
            .next()?
            .trim_end_matches(';')
            .trim_matches('`')
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();

        if table_name.contains('(') || table_name.contains(',') {
            return None;
        }

        return Some(table_name);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::dialect::MySqlDialect;
    use sqlparser::parser::Parser;

    // ==================== is_query_stmt tests (AST-based) ====================

    #[test]
    fn test_is_query_stmt_select() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "SELECT * FROM users").unwrap();
        assert!(is_query_stmt(&stmts[0]));
    }

    #[test]
    fn test_is_query_stmt_show() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "SHOW TABLES").unwrap();
        assert!(is_query_stmt(&stmts[0]));
    }

    #[test]
    fn test_is_query_stmt_explain() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "EXPLAIN SELECT * FROM users").unwrap();
        assert!(is_query_stmt(&stmts[0]));
    }

    #[test]
    fn test_is_query_stmt_insert() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "INSERT INTO users VALUES (1)").unwrap();
        assert!(!is_query_stmt(&stmts[0]));
    }

    #[test]
    fn test_is_query_stmt_update() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "UPDATE users SET name = 'test'").unwrap();
        assert!(!is_query_stmt(&stmts[0]));
    }

    #[test]
    fn test_is_query_stmt_delete() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "DELETE FROM users").unwrap();
        assert!(!is_query_stmt(&stmts[0]));
    }

    // ==================== is_query_statement_fallback tests ====================

    #[test]
    fn test_is_query_statement_fallback_select() {
        assert!(is_query_statement_fallback("SELECT * FROM users"));
        assert!(is_query_statement_fallback("  select id from t  "));
    }

    #[test]
    fn test_is_query_statement_fallback_show() {
        assert!(is_query_statement_fallback("SHOW TABLES"));
        assert!(is_query_statement_fallback("SHOW DATABASES"));
    }

    #[test]
    fn test_is_query_statement_fallback_describe() {
        assert!(is_query_statement_fallback("DESCRIBE users"));
        assert!(is_query_statement_fallback("DESC users"));
    }

    #[test]
    fn test_is_query_statement_fallback_explain() {
        assert!(is_query_statement_fallback("EXPLAIN SELECT * FROM users"));
    }

    #[test]
    fn test_is_query_statement_fallback_with() {
        assert!(is_query_statement_fallback(
            "WITH cte AS (SELECT 1) SELECT * FROM cte"
        ));
    }

    #[test]
    fn test_is_query_statement_fallback_pragma() {
        assert!(is_query_statement_fallback("PRAGMA table_info(users)"));
    }

    #[test]
    fn test_is_query_statement_fallback_non_query() {
        assert!(!is_query_statement_fallback("INSERT INTO users VALUES (1)"));
        assert!(!is_query_statement_fallback(
            "UPDATE users SET name = 'test'"
        ));
        assert!(!is_query_statement_fallback("DELETE FROM users"));
        assert!(!is_query_statement_fallback("CREATE TABLE t (id INT)"));
    }

    // ==================== classify_stmt tests (AST-based) ====================

    #[test]
    fn test_classify_stmt_query() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "SELECT * FROM users").unwrap();
        assert_eq!(classify_stmt(&stmts[0]), StatementType::Query);
    }

    #[test]
    fn test_classify_stmt_dml() {
        let insert = Parser::parse_sql(&MySqlDialect {}, "INSERT INTO users VALUES (1)").unwrap();
        assert_eq!(classify_stmt(&insert[0]), StatementType::Dml);

        let update = Parser::parse_sql(&MySqlDialect {}, "UPDATE users SET name = 'test'").unwrap();
        assert_eq!(classify_stmt(&update[0]), StatementType::Dml);

        let delete = Parser::parse_sql(&MySqlDialect {}, "DELETE FROM users").unwrap();
        assert_eq!(classify_stmt(&delete[0]), StatementType::Dml);
    }

    #[test]
    fn test_classify_stmt_ddl() {
        let create = Parser::parse_sql(&MySqlDialect {}, "CREATE TABLE t (id INT)").unwrap();
        assert_eq!(classify_stmt(&create[0]), StatementType::Ddl);

        let alter = Parser::parse_sql(
            &MySqlDialect {},
            "ALTER TABLE t ADD COLUMN name VARCHAR(100)",
        )
        .unwrap();
        assert_eq!(classify_stmt(&alter[0]), StatementType::Ddl);

        let drop = Parser::parse_sql(&MySqlDialect {}, "DROP TABLE t").unwrap();
        assert_eq!(classify_stmt(&drop[0]), StatementType::Ddl);
    }

    #[test]
    fn test_classify_stmt_transaction() {
        let commit = Parser::parse_sql(&MySqlDialect {}, "COMMIT").unwrap();
        assert_eq!(classify_stmt(&commit[0]), StatementType::Transaction);

        let rollback = Parser::parse_sql(&MySqlDialect {}, "ROLLBACK").unwrap();
        assert_eq!(classify_stmt(&rollback[0]), StatementType::Transaction);
    }

    #[test]
    fn test_classify_stmt_command() {
        let use_stmt = Parser::parse_sql(&MySqlDialect {}, "USE mydb").unwrap();
        assert_eq!(classify_stmt(&use_stmt[0]), StatementType::Command);

        let set = Parser::parse_sql(&MySqlDialect {}, "SET autocommit = 1").unwrap();
        assert_eq!(classify_stmt(&set[0]), StatementType::Command);
    }

    // ==================== classify_fallback tests ====================

    #[test]
    fn test_classify_fallback_query() {
        assert_eq!(
            classify_fallback("SELECT * FROM users"),
            StatementType::Query
        );
        assert_eq!(classify_fallback("SHOW TABLES"), StatementType::Query);
        assert_eq!(classify_fallback("DESCRIBE users"), StatementType::Query);
    }

    #[test]
    fn test_classify_fallback_dml() {
        assert_eq!(
            classify_fallback("INSERT INTO users VALUES (1)"),
            StatementType::Dml
        );
        assert_eq!(
            classify_fallback("UPDATE users SET name = 'test'"),
            StatementType::Dml
        );
        assert_eq!(classify_fallback("DELETE FROM users"), StatementType::Dml);
        assert_eq!(
            classify_fallback("REPLACE INTO users VALUES (1)"),
            StatementType::Dml
        );
    }

    #[test]
    fn test_classify_fallback_ddl() {
        assert_eq!(
            classify_fallback("CREATE TABLE users (id INT)"),
            StatementType::Ddl
        );
        assert_eq!(
            classify_fallback("ALTER TABLE users ADD COLUMN name VARCHAR(100)"),
            StatementType::Ddl
        );
        assert_eq!(classify_fallback("DROP TABLE users"), StatementType::Ddl);
        assert_eq!(
            classify_fallback("TRUNCATE TABLE users"),
            StatementType::Ddl
        );
        assert_eq!(
            classify_fallback("RENAME TABLE old TO new"),
            StatementType::Ddl
        );
    }

    #[test]
    fn test_classify_fallback_transaction() {
        assert_eq!(classify_fallback("BEGIN"), StatementType::Transaction);
        assert_eq!(classify_fallback("COMMIT"), StatementType::Transaction);
        assert_eq!(classify_fallback("ROLLBACK"), StatementType::Transaction);
        assert_eq!(
            classify_fallback("START TRANSACTION"),
            StatementType::Transaction
        );
    }

    #[test]
    fn test_classify_fallback_command() {
        assert_eq!(classify_fallback("USE mydb"), StatementType::Command);
        assert_eq!(
            classify_fallback("SET autocommit = 1"),
            StatementType::Command
        );
    }

    #[test]
    fn test_classify_fallback_exec() {
        assert_eq!(
            classify_fallback("CALL my_procedure()"),
            StatementType::Exec
        );
        assert_eq!(
            classify_fallback("EXECUTE my_statement"),
            StatementType::Exec
        );
    }

    // ==================== analyze_query_editability tests (AST-based) ====================

    #[test]
    fn test_analyze_query_editability_simple() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "SELECT * FROM users").unwrap();
        if let Statement::Query(query) = &stmts[0] {
            let result = analyze_query_editability(query);
            assert!(result.is_some());
            assert!(result.unwrap().contains("users"));
        }
    }

    #[test]
    fn test_analyze_query_editability_with_where() {
        let stmts =
            Parser::parse_sql(&MySqlDialect {}, "SELECT * FROM users WHERE id = 1").unwrap();
        if let Statement::Query(query) = &stmts[0] {
            let result = analyze_query_editability(query);
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_analyze_query_editability_with_join() {
        let stmts = Parser::parse_sql(
            &MySqlDialect {},
            "SELECT * FROM users JOIN orders ON users.id = orders.user_id",
        )
        .unwrap();
        if let Statement::Query(query) = &stmts[0] {
            let result = analyze_query_editability(query);
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_analyze_query_editability_with_group_by() {
        let stmts = Parser::parse_sql(
            &MySqlDialect {},
            "SELECT name, COUNT(*) FROM users GROUP BY name",
        )
        .unwrap();
        if let Statement::Query(query) = &stmts[0] {
            let result = analyze_query_editability(query);
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_analyze_query_editability_with_distinct() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "SELECT DISTINCT name FROM users").unwrap();
        if let Statement::Query(query) = &stmts[0] {
            let result = analyze_query_editability(query);
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_analyze_query_editability_with_aggregate() {
        let stmts = Parser::parse_sql(&MySqlDialect {}, "SELECT COUNT(*) FROM users").unwrap();
        if let Statement::Query(query) = &stmts[0] {
            let result = analyze_query_editability(query);
            assert!(result.is_none());
        }
    }

    // ==================== analyze_select_editability_fallback tests ====================

    #[test]
    fn test_analyze_select_editability_fallback_simple() {
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM users"),
            Some("users".to_string())
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_quoted() {
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM `users`"),
            Some("users".to_string())
        );
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM \"users\""),
            Some("users".to_string())
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_with_where() {
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM users WHERE id = 1"),
            Some("users".to_string())
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_with_join() {
        assert_eq!(
            analyze_select_editability_fallback(
                "SELECT * FROM users JOIN orders ON users.id = orders.user_id"
            ),
            None
        );
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM users INNER JOIN orders"),
            None
        );
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM users LEFT JOIN orders"),
            None
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_with_group_by() {
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM users GROUP BY name"),
            None
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_with_aggregate() {
        assert_eq!(
            analyze_select_editability_fallback("SELECT COUNT(*) FROM users"),
            None
        );
        assert_eq!(
            analyze_select_editability_fallback("SELECT SUM(amount) FROM orders"),
            None
        );
        assert_eq!(
            analyze_select_editability_fallback("SELECT AVG(price) FROM products"),
            None
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_with_distinct() {
        assert_eq!(
            analyze_select_editability_fallback("SELECT DISTINCT * FROM users"),
            None
        );
        assert_eq!(
            analyze_select_editability_fallback("SELECT DISTINCT name FROM users"),
            None
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_with_union() {
        assert_eq!(
            analyze_select_editability_fallback("SELECT * FROM users UNION SELECT * FROM admins"),
            None
        );
    }

    #[test]
    fn test_analyze_select_editability_fallback_non_select() {
        assert_eq!(
            analyze_select_editability_fallback("INSERT INTO users VALUES (1)"),
            None
        );
        assert_eq!(
            analyze_select_editability_fallback("UPDATE users SET name = 'test'"),
            None
        );
    }
}
