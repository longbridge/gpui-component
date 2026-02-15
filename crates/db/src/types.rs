use crate::QueryResult;
use gpui_component::table::Column;
use one_core::storage::DatabaseType;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

/// SQL value type for parameter binding
#[derive(Debug, Clone)]
pub enum SqlValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
}

/// Database tree node types for hierarchical display
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum DbNodeType {
    #[default]
    Connection,
    Database,
    Schema,
    TablesFolder,
    Table,
    ColumnsFolder,
    Column,
    IndexesFolder,
    Index,
    ForeignKeysFolder,
    ForeignKey,
    TriggersFolder,
    Trigger,
    ChecksFolder,
    Check,
    ViewsFolder,
    View,
    FunctionsFolder,
    Function,
    ProceduresFolder,
    Procedure,
    SequencesFolder,
    Sequence,
    QueriesFolder,
    NamedQuery,
}

impl fmt::Display for DbNodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbNodeType::Connection => write!(f, "Connection"),
            DbNodeType::Database => write!(f, "Database"),
            DbNodeType::Schema => write!(f, "Schema"),
            DbNodeType::TablesFolder => write!(f, "Tables"),
            DbNodeType::Table => write!(f, "Table"),
            DbNodeType::ColumnsFolder => write!(f, "Columns"),
            DbNodeType::Column => write!(f, "Column"),
            DbNodeType::IndexesFolder => write!(f, "Indexes"),
            DbNodeType::Index => write!(f, "Index"),
            DbNodeType::ForeignKeysFolder => write!(f, "Foreign Keys"),
            DbNodeType::ForeignKey => write!(f, "Foreign Key"),
            DbNodeType::TriggersFolder => write!(f, "Triggers"),
            DbNodeType::Trigger => write!(f, "Trigger"),
            DbNodeType::ChecksFolder => write!(f, "Checks"),
            DbNodeType::Check => write!(f, "Check"),
            DbNodeType::ViewsFolder => write!(f, "Views"),
            DbNodeType::View => write!(f, "View"),
            DbNodeType::FunctionsFolder => write!(f, "Functions"),
            DbNodeType::Function => write!(f, "Function"),
            DbNodeType::ProceduresFolder => write!(f, "Procedures"),
            DbNodeType::Procedure => write!(f, "Procedure"),
            DbNodeType::QueriesFolder => write!(f, "Queries"),
            DbNodeType::NamedQuery => write!(f, "Query"),
            DbNodeType::SequencesFolder => write!(f, "Sequences"),
            DbNodeType::Sequence => write!(f, "Sequence"),
        }
    }
}

/// Database tree node for lazy-loading hierarchical display
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbNode {
    pub id: String,
    pub name: String,
    pub node_type: DbNodeType,
    pub database_type: DatabaseType,
    pub children_loaded: bool,
    pub children: Vec<DbNode>,
    pub metadata: HashMap<String, String>,
    pub connection_id: String,
    pub parent_context: Option<String>,
}

impl PartialEq for DbNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for DbNode {}

impl PartialOrd for DbNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DbNode {
    fn cmp(&self, other: &Self) -> Ordering {
        let type_ordering = self.node_type.cmp(&other.node_type);
        if type_ordering != Ordering::Equal {
            return type_ordering;
        }
        let name_ordering = self.name.to_lowercase().cmp(&other.name.to_lowercase());
        if name_ordering != Ordering::Equal {
            return name_ordering;
        }
        self.id.cmp(&other.id)
    }
}

impl DbNode {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        node_type: DbNodeType,
        connection_id: String,
        database_type: DatabaseType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            node_type,
            children_loaded: false,
            children: Vec::new(),
            metadata: HashMap::new(),
            connection_id,
            parent_context: None,
            database_type,
        }
    }

    pub fn with_children_loaded(mut self, children_loaded: bool) -> Self {
        self.children_loaded = children_loaded;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_parent_context(mut self, context: impl Into<String>) -> Self {
        self.parent_context = Some(context.into());
        self
    }

    pub fn sort_children(&mut self) {
        self.children.sort();
    }

    pub fn set_children(&mut self, children: Vec<DbNode>) {
        self.children = children;
        self.children_loaded = true;
    }

    pub fn sort_children_recursive(&mut self) {
        self.children.sort();
        for child in &mut self.children {
            child.sort_children_recursive();
        }
    }

    pub fn get_database_name(&self) -> Option<String> {
        if self.node_type == DbNodeType::Database {
            Some(self.name.clone())
        } else {
            self.metadata.get("database").cloned()
        }
    }

    pub fn get_schema_name(&self) -> Option<String> {
        if self.node_type == DbNodeType::Schema {
            Some(self.name.clone())
        } else {
            self.metadata.get("schema").cloned()
        }
    }

    pub fn get_table_name(&self) -> Option<String> {
        if self.node_type == DbNodeType::Table {
            Some(self.name.clone())
        } else {
            self.metadata.get("table").cloned()
        }
    }
}

/// Database information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub size: Option<String>,
    pub table_count: Option<i64>,
    pub comment: Option<String>,
}

/// Column information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
}

/// Index information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub index_type: Option<String>,
}

/// Table information with description/metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub comment: Option<String>,
    pub engine: Option<String>,
    pub row_count: Option<i64>,
    pub create_time: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

/// View information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewInfo {
    pub name: String,
    pub schema: Option<String>,
    pub definition: Option<String>,
    pub comment: Option<String>,
}

/// Function information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub return_type: Option<String>,
    pub parameters: Vec<String>,
    pub definition: Option<String>,
    pub comment: Option<String>,
}

/// Trigger information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriggerInfo {
    pub name: String,
    pub table_name: String,
    pub event: String,
    pub timing: String,
    pub definition: Option<String>,
}

/// Sequence information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SequenceInfo {
    pub name: String,
    pub start_value: Option<i64>,
    pub increment: Option<i64>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
}

/// Check constraint information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckInfo {
    pub name: String,
    pub table_name: String,
    pub definition: Option<String>,
}

// === SQL Operation Request Objects ===

#[derive(Debug, Clone)]
pub struct CreateDatabaseRequest {
    pub database_name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DropDatabaseRequest {
    pub database_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct AlterDatabaseRequest {
    pub database_name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateTableRequest {
    pub database_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
    pub if_not_exists: bool,
}

#[derive(Debug, Clone)]
pub struct DropTableRequest {
    pub database_name: String,
    pub table_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct RenameTableRequest {
    pub database_name: String,
    pub old_table_name: String,
    pub new_table_name: String,
}

#[derive(Debug, Clone)]
pub struct TruncateTableRequest {
    pub database_name: String,
    pub table_name: String,
}

#[derive(Debug, Clone)]
pub struct AddColumnRequest {
    pub database_name: String,
    pub table_name: String,
    pub column: ColumnInfo,
}

#[derive(Debug, Clone)]
pub struct DropColumnRequest {
    pub database_name: String,
    pub table_name: String,
    pub column_name: String,
}

#[derive(Debug, Clone)]
pub struct ModifyColumnRequest {
    pub database_name: String,
    pub table_name: String,
    pub column: ColumnInfo,
}

#[derive(Debug, Clone)]
pub struct CreateIndexRequest {
    pub database_name: String,
    pub table_name: String,
    pub index: IndexInfo,
}

#[derive(Debug, Clone)]
pub struct DropIndexRequest {
    pub database_name: String,
    pub table_name: String,
    pub index_name: String,
}

#[derive(Debug, Clone)]
pub struct CreateViewRequest {
    pub database_name: String,
    pub view_name: String,
    pub definition: String,
    pub or_replace: bool,
}

#[derive(Debug, Clone)]
pub struct DropViewRequest {
    pub database_name: String,
    pub view_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct CreateFunctionRequest {
    pub database_name: String,
    pub definition: String,
}

#[derive(Debug, Clone)]
pub struct DropFunctionRequest {
    pub database_name: String,
    pub function_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct CreateProcedureRequest {
    pub database_name: String,
    pub definition: String,
}

#[derive(Debug, Clone)]
pub struct DropProcedureRequest {
    pub database_name: String,
    pub procedure_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct CreateTriggerRequest {
    pub database_name: String,
    pub definition: String,
}

#[derive(Debug, Clone)]
pub struct DropTriggerRequest {
    pub database_name: String,
    pub trigger_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct CreateSequenceRequest {
    pub database_name: String,
    pub sequence: SequenceInfo,
}

#[derive(Debug, Clone)]
pub struct DropSequenceRequest {
    pub database_name: String,
    pub sequence_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct AlterSequenceRequest {
    pub database_name: String,
    pub sequence: SequenceInfo,
}

#[derive(Debug, Clone, Default)]
pub struct ObjectView {
    pub db_node_type: DbNodeType,
    pub title: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

// === Table Data Query Types ===

/// Abstract data type for UI rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FieldType {
    /// Integer numbers (INT, BIGINT, SMALLINT, etc.)
    Integer,
    /// Decimal numbers (DECIMAL, NUMERIC, FLOAT, DOUBLE, etc.)
    Decimal,
    /// Short text (VARCHAR, CHAR)
    Text,
    /// Long text (TEXT, LONGTEXT, CLOB)
    LongText,
    /// Boolean (BOOL, BOOLEAN, BIT)
    Boolean,
    /// Date only (DATE)
    Date,
    /// Time only (TIME)
    Time,
    /// Date and time (DATETIME, TIMESTAMP)
    DateTime,
    /// Binary data (BLOB, BINARY, BYTEA)
    Binary,
    /// JSON data
    Json,
    /// Unknown or unsupported type
    Unknown,
}

impl FieldType {
    /// Infer field type from database type string
    pub fn from_db_type(db_type: &str) -> Self {
        let upper = db_type.to_uppercase();
        let base_type = upper.split('(').next().unwrap_or(&upper).trim();

        match base_type {
            // Integer types
            "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" | "MEDIUMINT" | "SERIAL"
            | "BIGSERIAL" | "SMALLSERIAL" => Self::Integer,
            // Decimal types
            "DECIMAL" | "NUMERIC" | "FLOAT" | "DOUBLE" | "REAL" | "DOUBLE PRECISION" | "MONEY" => {
                Self::Decimal
            }
            // Boolean
            "BOOL" | "BOOLEAN" | "BIT" => Self::Boolean,
            // Date/Time
            "DATE" => Self::Date,
            "TIME" => Self::Time,
            "DATETIME" | "TIMESTAMP" | "TIMESTAMPTZ" => Self::DateTime,
            // Text types
            "CHAR" | "VARCHAR" | "NCHAR" | "NVARCHAR" | "CHARACTER VARYING" | "CHARACTER" => {
                Self::Text
            }
            "TEXT" | "LONGTEXT" | "MEDIUMTEXT" | "TINYTEXT" | "CLOB" | "NTEXT" => Self::LongText,
            // Binary
            "BLOB" | "LONGBLOB" | "MEDIUMBLOB" | "TINYBLOB" | "BINARY" | "VARBINARY" | "BYTEA"
            | "IMAGE" => Self::Binary,
            // JSON
            "JSON" | "JSONB" => Self::Json,
            _ => Self::Text,
        }
    }
}

/// Column metadata for table data display
#[derive(Debug, Clone)]
pub struct TableColumnMeta {
    /// Column name
    pub name: String,
    /// Original database type (e.g., "VARCHAR(255)")
    pub db_type: String,
    /// Abstract field type for UI rendering
    pub field_type: FieldType,
    /// Whether the column is nullable
    pub nullable: bool,
    /// Whether the column is a primary key
    pub is_primary_key: bool,
    /// Column index in the result set
    pub index: usize,
}

/// Filter condition for querying table data
#[derive(Debug, Clone)]
pub struct FilterCondition {
    /// Column name
    pub column: String,
    /// Operator (=, !=, >, <, >=, <=, LIKE, IN, IS NULL, IS NOT NULL)
    pub operator: FilterOperator,
    /// Value (ignored for IS NULL / IS NOT NULL)
    pub value: String,
}

/// Filter operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterOperator {
    #[default]
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Like,
    NotLike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
}

impl FilterOperator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::GreaterThan => ">",
            Self::LessThan => "<",
            Self::GreaterOrEqual => ">=",
            Self::LessOrEqual => "<=",
            Self::Like => "LIKE",
            Self::NotLike => "NOT LIKE",
            Self::In => "IN",
            Self::NotIn => "NOT IN",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
        }
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Sort condition
#[derive(Debug, Clone)]
pub struct SortCondition {
    pub column: String,
    pub direction: SortDirection,
}

/// Represents a single cell change when persisting table edits
#[derive(Debug, Clone)]
pub struct TableCellChange {
    pub column_index: usize,
    pub column_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// Represents a table row change for persistence operations
#[derive(Debug, Clone)]
pub enum TableRowChange {
    Added {
        data: Vec<String>,
    },
    Updated {
        original_data: Vec<String>,
        changes: Vec<TableCellChange>,
        rowid: Option<String>,
    },
    Deleted {
        original_data: Vec<String>,
        rowid: Option<String>,
    },
}

/// Request payload for saving table edits back to the database
#[derive(Debug, Clone)]
pub struct TableSaveRequest {
    pub database: String,
    pub schema: Option<String>,
    pub table: String,
    pub columns: Vec<ColumnInfo>,
    pub index_infos: Vec<IndexInfo>,
    pub changes: Vec<TableRowChange>,
}

/// Request for generating copy SQL (INSERT, UPDATE, DELETE statements)
#[derive(Debug, Clone)]
pub struct CopySqlRequest {
    /// Schema name (optional, for databases that support schemas)
    pub schema: Option<String>,
    /// Table name
    pub table: String,
    /// Column information
    pub columns: Vec<ColumnInfo>,
    /// Row data to generate SQL for
    pub rows: Vec<Vec<Option<String>>>,
    /// Original row data (for UPDATE statements, used to generate WHERE clause)
    pub original_rows: Option<Vec<Vec<Option<String>>>>,
    /// Column names
    pub column_names: Vec<String>,
}

impl CopySqlRequest {
    pub fn new(table: impl Into<String>, columns: Vec<ColumnInfo>) -> Self {
        let column_names = columns.iter().map(|c| c.name.clone()).collect();
        Self {
            schema: None,
            table: table.into(),
            columns,
            rows: Vec::new(),
            original_rows: None,
            column_names,
        }
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn with_rows(mut self, rows: Vec<Vec<Option<String>>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn with_original_rows(mut self, original_rows: Vec<Vec<Option<String>>>) -> Self {
        self.original_rows = Some(original_rows);
        self
    }

    pub fn with_column_names(mut self, column_names: Vec<String>) -> Self {
        self.column_names = column_names;
        self
    }
}

/// 复制为 SQL 的类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyAsSqlType {
    /// INSERT 语句
    Insert,
    /// INSERT 语句（带注释）
    InsertWithComments,
    /// UPDATE 语句
    Update,
    /// DELETE 语句
    Delete,
}

/// Response from applying table edits
#[derive(Debug, Clone)]
pub struct TableSaveResponse {
    pub success_count: usize,
    pub errors: Vec<String>,
}

/// Request for querying table data with pagination and filtering
#[derive(Debug, Clone, Default)]
pub struct TableDataRequest {
    /// Database name
    pub database: String,
    /// Schema name (for databases that support schemas like PostgreSQL, MSSQL)
    pub schema: Option<String>,
    /// Table name
    pub table: String,
    /// Page number (1-based)
    pub page: usize,
    /// Page size
    pub page_size: usize,
    /// Filter conditions (structured)
    pub filters: Vec<FilterCondition>,
    /// Sort conditions (structured)
    pub sorts: Vec<SortCondition>,
    /// Raw WHERE clause (e.g., "id > 10 AND name LIKE '%test%'")
    pub where_clause: Option<String>,
    /// Raw ORDER BY clause (e.g., "id DESC, name ASC")
    pub order_by_clause: Option<String>,
}

impl TableDataRequest {
    pub fn new(database: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            database: database.into(),
            schema: None,
            table: table.into(),
            page: 1,
            page_size: 100,
            filters: Vec::new(),
            sorts: Vec::new(),
            where_clause: None,
            order_by_clause: None,
        }
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn with_page(mut self, page: usize, page_size: usize) -> Self {
        self.page = page;
        self.page_size = page_size;
        self
    }

    pub fn with_filter(mut self, filter: FilterCondition) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn with_sort(mut self, sort: SortCondition) -> Self {
        self.sorts.push(sort);
        self
    }

    pub fn with_where_clause(mut self, clause: impl Into<String>) -> Self {
        let c = clause.into();
        self.where_clause = if c.is_empty() { None } else { Some(c) };
        self
    }

    pub fn with_order_by_clause(mut self, clause: impl Into<String>) -> Self {
        let c = clause.into();
        self.order_by_clause = if c.is_empty() { None } else { Some(c) };
        self
    }
}

/// Response for table data query
#[derive(Debug, Clone)]
pub struct TableDataResponse {
    /// Row data (each cell is Option<String>, None means NULL)
    pub query_result: QueryResult,
    /// Total row count (for pagination)
    pub total_count: usize,
    /// Current page
    pub page: usize,
    /// Page size
    pub page_size: usize,
    /// Duration of the query
    pub duration: u128,
}

/// Character set information
#[derive(Debug, Clone)]
pub struct CharsetInfo {
    pub name: String,
    pub description: String,
    pub default_collation: String,
}

/// Collation information
#[derive(Debug, Clone)]
pub struct CollationInfo {
    pub name: String,
    pub charset: String,
    pub is_default: bool,
}

// === Table Designer Types ===

/// Detailed column definition for table designer
#[derive(Debug, Clone, Default)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub length: Option<u32>,
    pub precision: Option<u32>,
    pub scale: Option<u32>,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub is_unsigned: bool,
    pub default_value: Option<String>,
    pub comment: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

impl ColumnDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_nullable: true,
            ..Default::default()
        }
    }

    pub fn data_type(mut self, data_type: impl Into<String>) -> Self {
        self.data_type = data_type.into();
        self
    }

    pub fn length(mut self, length: u32) -> Self {
        self.length = Some(length);
        self
    }

    pub fn nullable(mut self, nullable: bool) -> Self {
        self.is_nullable = nullable;
        self
    }

    pub fn primary_key(mut self, pk: bool) -> Self {
        self.is_primary_key = pk;
        self
    }

    pub fn auto_increment(mut self, ai: bool) -> Self {
        self.is_auto_increment = ai;
        self
    }

    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }
}

/// Index definition for table designer
#[derive(Debug, Clone, Default)]
pub struct IndexDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: Option<String>,
    pub comment: String,
}

impl IndexDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn columns(mut self, columns: Vec<String>) -> Self {
        self.columns = columns;
        self
    }

    pub fn unique(mut self, unique: bool) -> Self {
        self.is_unique = unique;
        self
    }

    pub fn primary(mut self, primary: bool) -> Self {
        self.is_primary = primary;
        self
    }
}

/// Foreign key definition
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ForeignKeyDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: String,
    pub on_update: String,
}

/// Table options (engine, charset, etc.)
#[derive(Debug, Clone, Default)]
pub struct TableOptions {
    pub engine: Option<String>,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub comment: String,
    pub auto_increment: Option<u64>,
}

/// Complete table design
#[derive(Debug, Clone, Default)]
pub struct TableDesign {
    pub database_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnDefinition>,
    pub indexes: Vec<IndexDefinition>,
    pub foreign_keys: Vec<ForeignKeyDefinition>,
    pub options: TableOptions,
}

impl TableDesign {
    pub fn new(database_name: impl Into<String>, table_name: impl Into<String>) -> Self {
        Self {
            database_name: database_name.into(),
            table_name: table_name.into(),
            ..Default::default()
        }
    }

    pub fn add_column(&mut self, column: ColumnDefinition) {
        self.columns.push(column);
    }

    pub fn add_index(&mut self, index: IndexDefinition) {
        self.indexes.push(index);
    }

    pub fn primary_key_columns(&self) -> Vec<&str> {
        self.columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| c.name.as_str())
            .collect()
    }
}

/// Parsed column type information
#[derive(Debug, Clone, Default)]
pub struct ParsedColumnType {
    pub base_type: String,
    pub length: Option<u32>,
    pub scale: Option<u32>,
    pub enum_values: Option<String>,
    pub is_unsigned: bool,
    pub is_auto_increment: bool,
}

impl ParsedColumnType {
    pub fn new(base_type: impl Into<String>) -> Self {
        Self {
            base_type: base_type.into(),
            ..Default::default()
        }
    }

    pub fn with_length(mut self, length: u32) -> Self {
        self.length = Some(length);
        self
    }

    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = Some(scale);
        self
    }

    pub fn with_enum_values(mut self, values: impl Into<String>) -> Self {
        self.enum_values = Some(values.into());
        self
    }

    pub fn with_unsigned(mut self, unsigned: bool) -> Self {
        self.is_unsigned = unsigned;
        self
    }

    pub fn with_auto_increment(mut self, auto_increment: bool) -> Self {
        self.is_auto_increment = auto_increment;
        self
    }
}
