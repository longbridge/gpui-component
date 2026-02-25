use std::rc::Rc;

use anyhow::Result;
use db::plugin::SqlCompletionInfo;
use gpui::{
    App, AppContext, Context, Entity, IntoElement, Render, Styled as _, Subscription, Task, Window,
};
use gpui_component::highlighter::Language;
use gpui_component::input::{
    CodeActionProvider, CompletionProvider, HoverProvider, Input, InputEvent, InputState, TabSize,
};
use gpui_component::{Rope, RopeExt};
use lsp_types::{CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit, InlineCompletionContext, InlineCompletionItem, InlineCompletionResponse, InsertReplaceEdit, InsertTextFormat, Range as LspRange};
use db::sql_editor::sql_context_inferrer::{ContextInferrer, SqlContext as InferredSqlContext};
use db::sql_editor::sql_symbol_table::SymbolTable;
use db::sql_editor::sql_tokenizer::SqlTokenizer;
use sum_tree::Bias;

/// Simple schema hints to improve autocomplete suggestions.
#[derive(Clone, Default)]
pub struct SqlSchema {
    pub tables: Vec<(String, String)>,  // (name, doc)
    pub columns: Vec<(String, String)>, // global (name, doc)
    /// 表→列映射，每列包含 (name, data_type, doc)
    pub columns_by_table: std::collections::HashMap<String, Vec<(String, String, String)>>,
}

impl SqlSchema {
    pub fn with_tables(
        mut self,
        tables: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.tables = tables
            .into_iter()
            .map(|(n, d)| (n.into(), d.into()))
            .collect();
        self
    }
    pub fn with_columns(
        mut self,
        columns: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.columns = columns
            .into_iter()
            .map(|(n, d)| (n.into(), d.into()))
            .collect();
        self
    }
    /// 添加表的列信息（兼容旧 API，无类型信息）
    pub fn with_table_columns(
        mut self,
        table: impl Into<String>,
        columns: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.columns_by_table.insert(
            table.into(),
            columns
                .into_iter()
                .map(|(n, d)| (n.into(), String::new(), d.into()))
                .collect(),
        );
        self
    }
    /// 添加表的列信息（含类型信息）
    pub fn with_table_columns_typed(
        mut self,
        table: impl Into<String>,
        columns: impl IntoIterator<Item = (impl Into<String>, impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.columns_by_table.insert(
            table.into(),
            columns
                .into_iter()
                .map(|(n, t, d)| (n.into(), t.into(), d.into()))
                .collect(),
        );
        self
    }
}

/// SQL context for smarter completion suggestions
#[derive(Debug, Clone, PartialEq)]
pub enum SqlContext {
    /// Start of statement or unknown context
    Start,
    /// After SELECT keyword, expecting columns
    SelectColumns,
    /// After FROM/JOIN/INTO/UPDATE, expecting table name
    TableName,
    /// After WHERE/AND/OR/ON, expecting condition
    Condition,
    /// After ORDER BY/GROUP BY, expecting column
    OrderBy,
    /// After SET (in UPDATE), expecting column = value
    SetClause,
    /// After VALUES, expecting values
    Values,
    /// After CREATE TABLE, expecting table definition
    CreateTable,
    /// After dot (table.column), expecting column name
    DotColumn(String),
    /// After function name with open paren
    FunctionArgs,
}

/// Priority scores for context-aware completion sorting.
/// Lower scores appear first in completion list (higher priority).
///
/// Default priority order (without context):
/// 1. Keywords (1000-1999)
/// 2. Tables (2000-2999)
/// 3. Columns (3000-3999)
/// 4. Functions (4000-4999)
/// 5. Snippets (5000+)
///
/// In specific contexts, relevant items get boosted to appear before keywords.
pub mod completion_priority {
    // Base priorities by item type (lower = higher priority)
    pub const KEYWORDS_BASE: i32 = 1000;
    pub const DATA_TYPES_BASE: i32 = 1500;
    pub const TABLES_BASE: i32 = 2000;
    pub const COLUMNS_BASE: i32 = 3000;
    pub const SNIPPETS_BASE: i32 = 4000;
    pub const OPERATORS_BASE: i32 = 4500;
    pub const FUNCTIONS_BASE: i32 = 5000;

    // Context boost (subtract from base to increase priority)
    // Large boost to make context-relevant items appear before keywords
    pub const CONTEXT_BOOST: i32 = 2500;
    pub const PREFIX_MATCH_BOOST: i32 = 200;

    use super::SqlContext;
    use lsp_types::CompletionItemKind;

    /// Calculate priority score for a completion item based on context.
    /// Lower scores appear first (higher priority).
    pub fn calculate_score(
        context: &SqlContext,
        item_kind: Option<CompletionItemKind>,
        matches_prefix: bool,
    ) -> i32 {
        // Determine base score by item type
        let base_score = match item_kind {
            Some(CompletionItemKind::KEYWORD) => KEYWORDS_BASE,
            Some(CompletionItemKind::TYPE_PARAMETER) => DATA_TYPES_BASE,
            Some(CompletionItemKind::STRUCT) => TABLES_BASE,
            Some(CompletionItemKind::FIELD) => COLUMNS_BASE,
            Some(CompletionItemKind::FUNCTION) => FUNCTIONS_BASE,
            Some(CompletionItemKind::OPERATOR) => OPERATORS_BASE,
            Some(CompletionItemKind::SNIPPET) => SNIPPETS_BASE,
            _ => COLUMNS_BASE, // Default to columns priority
        };

        // Apply context boost for relevant items
        let context_boost = match (context, item_kind) {
            // DotColumn: columns get boost
            (SqlContext::DotColumn(_), Some(CompletionItemKind::FIELD)) => CONTEXT_BOOST,

            // TableName: tables get boost
            (SqlContext::TableName, Some(CompletionItemKind::STRUCT)) => CONTEXT_BOOST,

            // SelectColumns: columns get boost
            (SqlContext::SelectColumns, Some(CompletionItemKind::FIELD)) => CONTEXT_BOOST,

            // Condition/OrderBy/SetClause: columns get boost
            (
                SqlContext::Condition | SqlContext::OrderBy | SqlContext::SetClause,
                Some(CompletionItemKind::FIELD),
            ) => CONTEXT_BOOST,

            // FunctionArgs: columns get boost
            (SqlContext::FunctionArgs, Some(CompletionItemKind::FIELD)) => CONTEXT_BOOST,

            // CreateTable: data types get boost (appear before keywords)
            (SqlContext::CreateTable, Some(CompletionItemKind::TYPE_PARAMETER)) => CONTEXT_BOOST,

            _ => 0,
        };

        // Apply prefix match boost
        let prefix_boost = if matches_prefix {
            PREFIX_MATCH_BOOST
        } else {
            0
        };

        // Lower score = higher priority
        base_score - context_boost - prefix_boost
    }

    /// Convert score to sort_text format.
    /// Lower scores appear first (higher priority).
    /// Format: "{score:05}_{label}" for stable sorting.
    pub fn score_to_sort_text(score: i32, label: &str) -> String {
        // Lower score = higher priority, so use score directly
        format!("{:05}_{}", score.max(0).min(99999), label)
    }
}

// Built-in SQL keywords and docs
pub(crate) const SQL_KEYWORDS: &[(&str, &str)] = &[
    ("SELECT", "Query rows from table(s)"),
    ("INSERT", "Insert new rows"),
    ("UPDATE", "Update existing rows"),
    ("DELETE", "Delete rows"),
    ("CREATE", "Create database object"),
    ("ALTER", "Modify database object"),
    ("DROP", "Remove database object"),
    ("TRUNCATE", "Remove all rows from table"),
    ("FROM", "Specify source table(s)"),
    ("WHERE", "Filter rows with predicates"),
    ("JOIN", "Combine rows from tables"),
    ("INNER JOIN", "Inner join tables"),
    ("LEFT JOIN", "Left outer join"),
    ("RIGHT JOIN", "Right outer join"),
    ("FULL JOIN", "Full outer join"),
    ("CROSS JOIN", "Cross product of tables"),
    ("ON", "Join condition"),
    ("USING", "Join using common columns"),
    ("GROUP BY", "Group rows for aggregation"),
    ("HAVING", "Filter grouped rows"),
    ("ORDER BY", "Sort result set"),
    ("ASC", "Ascending order"),
    ("DESC", "Descending order"),
    ("LIMIT", "Limit number of rows"),
    ("OFFSET", "Skip rows"),
    ("VALUES", "Specify values for INSERT"),
    ("INTO", "Target table for INSERT"),
    ("SET", "Set column values for UPDATE"),
    ("AND", "Logical AND"),
    ("OR", "Logical OR"),
    ("NOT", "Logical NOT"),
    ("IN", "Value in list"),
    ("EXISTS", "Subquery returns rows"),
    ("BETWEEN", "Value in range"),
    ("LIKE", "Pattern matching"),
    ("IS NULL", "Check for NULL"),
    ("IS NOT NULL", "Check for non-NULL"),
    ("AS", "Alias"),
    ("DISTINCT", "Remove duplicates"),
    ("ALL", "Include all rows"),
    ("UNION", "Combine result sets"),
    ("UNION ALL", "Combine without dedup"),
    ("INTERSECT", "Common rows"),
    ("EXCEPT", "Difference of sets"),
    ("CASE", "Conditional expression"),
    ("WHEN", "Condition in CASE"),
    ("THEN", "Result in CASE"),
    ("ELSE", "Default in CASE"),
    ("END", "End CASE expression"),
    ("WITH", "Common table expression"),
    ("TABLE", "Table keyword"),
    ("INDEX", "Index keyword"),
    ("VIEW", "View keyword"),
    ("PRIMARY KEY", "Primary key constraint"),
    ("FOREIGN KEY", "Foreign key constraint"),
    ("REFERENCES", "Reference constraint"),
    ("UNIQUE", "Unique constraint"),
    ("CHECK", "Check constraint"),
    ("DEFAULT", "Default value"),
    ("NOT NULL", "Not null constraint"),
    ("NULL", "NULL value"),
    ("TRUE", "Boolean true"),
    ("FALSE", "Boolean false"),
];

const SQL_FUNCTIONS: &[(&str, &str)] = &[
    ("COUNT(*)", "Count all rows"),
    ("COUNT(col)", "Count non-NULL values"),
    ("SUM(col)", "Sum of values"),
    ("AVG(col)", "Average value"),
    ("MIN(col)", "Minimum value"),
    ("MAX(col)", "Maximum value"),
    ("COALESCE(val1, val2, ...)", "First non-NULL value"),
    ("NULLIF(val1, val2)", "NULL if values equal"),
    ("CAST(expr AS type)", "Type conversion"),
    ("UPPER(str)", "Convert to uppercase"),
    ("LOWER(str)", "Convert to lowercase"),
    ("TRIM(str)", "Remove whitespace"),
    ("LENGTH(str)", "String length"),
    ("SUBSTRING(str, pos, len)", "Extract substring"),
    ("CONCAT(str1, str2)", "Concatenate strings"),
    ("REPLACE(str, from, to)", "Replace substring"),
    ("ABS(x)", "Absolute value"),
    ("ROUND(x, d)", "Round number"),
    ("FLOOR(x)", "Round down"),
    ("CEIL(x)", "Round up"),
    ("NOW()", "Current timestamp"),
    ("CURRENT_DATE", "Current date"),
    ("CURRENT_TIME", "Current time"),
];

/// 内置 SQL 数据类型（通用标准 SQL 类型）
pub(crate) const SQL_DATA_TYPES: &[(&str, &str)] = &[
    ("INT", "32-bit integer"),
    ("INTEGER", "32-bit integer"),
    ("BIGINT", "64-bit integer"),
    ("SMALLINT", "16-bit integer"),
    ("TINYINT", "8-bit integer"),
    ("FLOAT", "Floating-point number"),
    ("DOUBLE", "Double-precision floating-point"),
    ("DECIMAL", "Fixed-point number"),
    ("NUMERIC", "Fixed-point number"),
    ("REAL", "Floating-point number"),
    ("CHAR", "Fixed-length string"),
    ("VARCHAR", "Variable-length string"),
    ("TEXT", "Variable-length text"),
    ("NCHAR", "Fixed-length Unicode string"),
    ("NVARCHAR", "Variable-length Unicode string"),
    ("BOOLEAN", "Boolean value"),
    ("BOOL", "Boolean value"),
    ("DATE", "Date value"),
    ("TIME", "Time value"),
    ("DATETIME", "Date and time"),
    ("TIMESTAMP", "Timestamp value"),
    ("BLOB", "Binary large object"),
    ("CLOB", "Character large object"),
    ("BINARY", "Fixed-length binary"),
    ("VARBINARY", "Variable-length binary"),
    ("JSON", "JSON data"),
    ("XML", "XML data"),
    ("UUID", "UUID value"),
    ("SERIAL", "Auto-incrementing integer"),
];

#[derive(Clone)]
pub struct DefaultSqlCompletionProvider {
    schema: SqlSchema,
    db_completion_info: Option<SqlCompletionInfo>,
}

impl DefaultSqlCompletionProvider {
    pub fn new(schema: SqlSchema) -> Self {
        Self {
            schema,
            db_completion_info: None,
        }
    }

    pub fn with_db_completion_info(mut self, info: SqlCompletionInfo) -> Self {
        self.db_completion_info = Some(info);
        self
    }

    /// Parse SQL text and return both context and symbol table.
    ///
    /// This method is used when we need the symbol table for DotColumn filtering.
    fn parse_context_with_symbols(text: &str, offset: usize) -> (SqlContext, SymbolTable) {
        let mut tokenizer = SqlTokenizer::new(text);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        let inferred = ContextInferrer::infer(&tokens, offset, &symbol_table);
        (Self::convert_context(inferred), symbol_table)
    }

    /// Convert InferredSqlContext to local SqlContext enum.
    fn convert_context(inferred: InferredSqlContext) -> SqlContext {
        match inferred {
            InferredSqlContext::Start => SqlContext::Start,
            InferredSqlContext::SelectColumns => SqlContext::SelectColumns,
            InferredSqlContext::TableName => SqlContext::TableName,
            InferredSqlContext::Condition => SqlContext::Condition,
            InferredSqlContext::OrderBy => SqlContext::OrderBy,
            InferredSqlContext::SetClause => SqlContext::SetClause,
            InferredSqlContext::Values => SqlContext::Values,
            InferredSqlContext::CreateTable => SqlContext::CreateTable,
            InferredSqlContext::DotColumn(alias) => SqlContext::DotColumn(alias),
            InferredSqlContext::FunctionArgs => SqlContext::FunctionArgs,
        }
    }
}

impl CompletionProvider for DefaultSqlCompletionProvider {
    fn completions(
        &self,
        rope: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let rope = rope.clone();
        let schema = self.schema.clone();
        let db_info = self.db_completion_info.clone();

        cx.background_spawn(async move {
            let text = rope.to_string();

            // Check if inside a comment (-- style)
            let before_cursor = &text[..offset.min(text.len())];
            let last_newline = before_cursor.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let current_line = &before_cursor[last_newline..];
            if current_line.contains("--") {
                return Ok(CompletionResponse::Array(vec![]));
            }

            // 分号后不显示补全（语句结束）
            if before_cursor.ends_with(';') {
                return Ok(CompletionResponse::Array(vec![]));
            }

            // Use tokenizer-based context parsing with symbol table
            let (context, symbol_table) = Self::parse_context_with_symbols(&text, offset);

            // Current word - find word start by scanning backwards from offset
            // Use clip_offset to ensure we're on a char boundary
            let mut start_offset = rope.clip_offset(offset, Bias::Left);
            while start_offset > 0 {
                let prev_offset = rope.clip_offset(start_offset.saturating_sub(1), Bias::Left);
                if prev_offset >= start_offset {
                    break;
                }
                let ch = rope.char(prev_offset);
                // 只将 ASCII 字母数字和下划线视为 SQL 标识符字符
                // 中文等非 ASCII 字符作为词边界处理，以便输入中文后仍能显示 SQL 补全
                if !(ch.is_ascii_alphanumeric() || ch == '_') {
                    break;
                }
                start_offset = prev_offset;
            }
            let current_word = rope.slice(start_offset..offset).to_string().to_uppercase();

            let start_pos = rope.offset_to_position(start_offset);
            let end_pos = rope.offset_to_position(offset);
            let replace_range = LspRange::new(start_pos, end_pos);

            let mut items = Vec::new();

            let matches_filter = |label: &str| -> bool {
                current_word.is_empty() || label.to_uppercase().starts_with(&current_word)
            };

            let matched_prefix = |label: &str| -> String {
                let lu = label.to_uppercase();
                if !current_word.is_empty() && lu.starts_with(&current_word) {
                    label.chars().take(current_word.chars().count()).collect()
                } else {
                    String::new()
                }
            };

            // Handle dot context (table.column) - highest priority
            // Uses SymbolTable to resolve alias to actual table name
            if let SqlContext::DotColumn(alias_or_table) = &context {
                // Resolve alias to table name using symbol table
                // If alias is found, use the resolved table name; otherwise use as-is
                let resolved_table = symbol_table
                    .resolve(alias_or_table)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| alias_or_table.clone());

                // 处理子查询别名：显示提示信息
                if resolved_table == "#subquery" {
                    items.push(CompletionItem {
                        label: "(subquery)".to_string(),
                        kind: Some(CompletionItemKind::TEXT),
                        detail: Some("子查询列 - 无法自动推断".to_string()),
                        documentation: Some(lsp_types::Documentation::String(
                            format!("'{}' 是子查询的别名，列信息需要从子查询定义中获取", alias_or_table),
                        )),
                        ..Default::default()
                    });
                    return Ok(CompletionResponse::Array(items));
                }

                // 处理 CTE 别名：显示提示信息
                if resolved_table == "#cte" {
                    items.push(CompletionItem {
                        label: "(CTE)".to_string(),
                        kind: Some(CompletionItemKind::TEXT),
                        detail: Some("CTE 列 - 无法自动推断".to_string()),
                        documentation: Some(lsp_types::Documentation::String(
                            format!("'{}' 是 CTE (WITH 子句) 的名称，列信息需要从 CTE 定义中获取", alias_or_table),
                        )),
                        ..Default::default()
                    });
                    return Ok(CompletionResponse::Array(items));
                }

                // Try to find columns for the resolved table
                // First try exact match, then case-insensitive match
                let columns = schema.columns_by_table.get(&resolved_table).or_else(|| {
                    // Case-insensitive lookup
                    let lower = resolved_table.to_lowercase();
                    schema
                        .columns_by_table
                        .iter()
                        .find(|(k, _)| k.to_lowercase() == lower)
                        .map(|(_, v)| v)
                });

                if let Some(cols) = columns {
                    for (column, data_type, doc) in cols {
                        if matches_filter(column) {
                            let matches_prefix = !current_word.is_empty()
                                && column.to_uppercase().starts_with(&current_word);
                            let score = completion_priority::calculate_score(
                                &context,
                                Some(CompletionItemKind::FIELD),
                                matches_prefix,
                            );
                            // 在 detail 中显示类型信息
                            let detail = if data_type.is_empty() {
                                format!("{}.{}", resolved_table, column)
                            } else {
                                format!("{}: {}", column, data_type)
                            };
                            items.push(CompletionItem {
                                label: column.clone(),
                                kind: Some(CompletionItemKind::FIELD),
                                detail: Some(detail),
                                text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                    InsertReplaceEdit {
                                        new_text: column.clone(),
                                        insert: replace_range,
                                        replace: replace_range,
                                    },
                                )),
                                filter_text: Some(matched_prefix(column)),
                                documentation: Some(lsp_types::Documentation::String(doc.clone())),
                                sort_text: Some(completion_priority::score_to_sort_text(
                                    score, column,
                                )),
                                ..Default::default()
                            });
                        }
                    }
                }
                // Sort by score and truncate (Requirement 5.6: limit to 50 items)
                items.sort_by(|a, b| {
                    a.sort_text
                        .as_ref()
                        .unwrap_or(&a.label)
                        .cmp(b.sort_text.as_ref().unwrap_or(&b.label))
                });
                items.truncate(50);
                return Ok(CompletionResponse::Array(items));
            }

            // Handle CREATE TABLE context - special logic to distinguish different positions
            if context == SqlContext::CreateTable {
                let before_word = &text[..start_offset];

                // 检查是否在括号内
                let has_open_paren = before_word.contains('(');

                if !has_open_paren {
                    // 括号外：用户在输入表名，不显示补全
                    // 例如：CREATE TABLE users|
                    return Ok(CompletionResponse::Array(vec![]));
                }

                // 在括号内，检查光标前的 token 来判断用户在输入什么
                let prev_char = before_word.chars().rev().find(|c| !c.is_whitespace());

                match prev_char {
                    // 括号或逗号后：用户在输入字段名称，不显示补全
                    // 例如：(id INT, name|  或  (id|
                    Some('(') | Some(',') => {
                        return Ok(CompletionResponse::Array(vec![]));
                    }
                    // 右括号后：约束定义结束
                    Some(')') => {
                        if current_word.is_empty() {
                            return Ok(CompletionResponse::Array(vec![]));
                        }
                    }
                    // 其他情况（标识符或数据类型后）：显示数据类型和约束
                    // 例如：(id |  或  (id INT |
                    _ => {}
                }
            }

            // Context-aware completion priorities
            // (show_tables, show_columns, show_keywords, show_functions, show_types)
            let (show_tables, show_columns, show_keywords, show_functions, show_types) =
                match context {
                    SqlContext::TableName => (true, false, false, false, false),
                    SqlContext::SelectColumns => (false, true, true, true, false), // Allow keywords like FROM, AS, DISTINCT
                    SqlContext::OrderBy | SqlContext::SetClause => (false, true, true, true, false),
                    SqlContext::Condition => (false, true, true, true, false),
                    SqlContext::FunctionArgs => (false, true, false, true, false),
                    SqlContext::CreateTable => (false, false, true, false, true),
                    SqlContext::Values => (false, false, false, true, false),
                    SqlContext::Start => (true, false, true, false, false), // 语句开始，没有 FROM，不显示字段
                    SqlContext::DotColumn(_) => (false, true, false, false, false), // Only show columns for table.column
                };

            // Tables - priority based on context (Requirement 5.2)
            if show_tables {
                for (table, doc) in &schema.tables {
                    if matches_filter(table) {
                        let matches_prefix = !current_word.is_empty()
                            && table.to_uppercase().starts_with(&current_word);
                        let score = completion_priority::calculate_score(
                            &context,
                            Some(CompletionItemKind::STRUCT),
                            matches_prefix,
                        );
                        items.push(CompletionItem {
                            label: table.clone(),
                            kind: Some(CompletionItemKind::STRUCT),
                            detail: Some("Table".to_string()),
                            text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                InsertReplaceEdit {
                                    new_text: table.clone(),
                                    insert: replace_range,
                                    replace: replace_range,
                                },
                            )),
                            filter_text: Some(matched_prefix(table)),
                            documentation: Some(lsp_types::Documentation::String(doc.clone())),
                            sort_text: Some(completion_priority::score_to_sort_text(score, table)),
                            ..Default::default()
                        });
                    }
                }
            }

            // Columns - priority based on context (Requirements 5.3, 5.4)
            if show_columns {
                // In contexts where we have table information (SelectColumns, Condition, OrderBy, SetClause),
                // show columns from tables in FROM/JOIN clauses
                let use_table_columns = matches!(
                    context,
                    SqlContext::SelectColumns
                        | SqlContext::Condition
                        | SqlContext::OrderBy
                        | SqlContext::SetClause
                );

                if use_table_columns {
                    // 检查当前语句是否有 FROM 子句
                    // 需要检查整个语句（包括光标后的部分），这样在 SELECT 列表中编辑时也能正确补全
                    // 找到当前语句的开始位置（从光标往前找分号或文件开头）
                    let statement_start = before_cursor
                        .rfind(';')
                        .map(|p| p + 1)
                        .unwrap_or(0);

                    // 找到当前语句的结束位置（从光标往后找分号或文件结尾）
                    let after_cursor = &text[offset..];
                    let statement_end = after_cursor
                        .find(';')
                        .map(|p| offset + p)
                        .unwrap_or(text.len());

                    // 获取完整的当前语句
                    let current_statement = &text[statement_start..statement_end];

                    // 如果当前语句没有 FROM，不显示列
                    let has_from = current_statement
                        .to_uppercase()
                        .contains(" FROM ");

                    if !has_from {
                        // 当前语句没有 FROM，跳过列显示
                    } else {
                        // Get all tables from symbol table
                        let tables: Vec<String> = symbol_table
                            .all_aliases()
                            .map(|(_, table)| table.to_string())
                            .collect();

                        // Deduplicate tables (in case of multiple aliases for same table)
                        let mut seen_tables = std::collections::HashSet::new();
                        let unique_tables: Vec<String> = tables
                            .into_iter()
                            .filter(|t| seen_tables.insert(t.to_lowercase()))
                            .collect();

                        // 收集所有列及其来源表，用于检测重复列名
                        let mut all_columns: Vec<(String, String, String, String)> = Vec::new(); // (column, table, data_type, doc)
                        for table in &unique_tables {
                            let columns = schema.columns_by_table.get(table).or_else(|| {
                                let lower = table.to_lowercase();
                                schema
                                    .columns_by_table
                                    .iter()
                                    .find(|(k, _)| k.to_lowercase() == lower)
                                    .map(|(_, v)| v)
                            });
                            if let Some(cols) = columns {
                                for (column, data_type, doc) in cols {
                                    all_columns.push((column.clone(), table.clone(), data_type.clone(), doc.clone()));
                                }
                            }
                        }

                        // 统计每个列名出现的次数
                        let mut column_counts: std::collections::HashMap<String, usize> =
                            std::collections::HashMap::new();
                        for (column, _, _, _) in &all_columns {
                            *column_counts.entry(column.to_lowercase()).or_insert(0) += 1;
                        }

                        // 生成补全项，重复列名显示为 table.column
                        for (column, table, data_type, doc) in all_columns {
                            if matches_filter(&column) {
                                let is_duplicate =
                                    column_counts.get(&column.to_lowercase()).unwrap_or(&0) > &1;
                                let (label, new_text) = if is_duplicate {
                                    // 多表同名列：显示 table.column 格式
                                    (format!("{}.{}", table, column), format!("{}.{}", table, column))
                                } else {
                                    // 唯一列名：只显示 column
                                    (column.clone(), column.clone())
                                };

                                // 在 detail 中显示类型信息
                                let detail = if data_type.is_empty() {
                                    format!("{}.{}", table, column)
                                } else {
                                    format!("{}: {}", column, data_type)
                                };

                                let matches_prefix = !current_word.is_empty()
                                    && column.to_uppercase().starts_with(&current_word);
                                let score = completion_priority::calculate_score(
                                    &context,
                                    Some(CompletionItemKind::FIELD),
                                    matches_prefix,
                                );
                                items.push(CompletionItem {
                                    label,
                                    kind: Some(CompletionItemKind::FIELD),
                                    detail: Some(detail),
                                    text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                        InsertReplaceEdit {
                                            new_text,
                                            insert: replace_range,
                                            replace: replace_range,
                                        },
                                    )),
                                    filter_text: Some(matched_prefix(&column)),
                                    documentation: Some(lsp_types::Documentation::String(doc)),
                                    sort_text: Some(completion_priority::score_to_sort_text(
                                        score, &column,
                                    )),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                } else {
                    // For other contexts (FunctionArgs, Start), show global columns
                    for (column, doc) in &schema.columns {
                        if matches_filter(column) {
                            let matches_prefix = !current_word.is_empty()
                                && column.to_uppercase().starts_with(&current_word);
                            let score = completion_priority::calculate_score(
                                &context,
                                Some(CompletionItemKind::FIELD),
                                matches_prefix,
                            );
                            items.push(CompletionItem {
                                label: column.clone(),
                                kind: Some(CompletionItemKind::FIELD),
                                detail: Some("Column".to_string()),
                                text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                    InsertReplaceEdit {
                                        new_text: column.clone(),
                                        insert: replace_range,
                                        replace: replace_range,
                                    },
                                )),
                                filter_text: Some(matched_prefix(column)),
                                documentation: Some(lsp_types::Documentation::String(doc.clone())),
                                sort_text: Some(completion_priority::score_to_sort_text(
                                    score, column,
                                )),
                                ..Default::default()
                            });
                        }
                    }
                }
            }

            // Keywords - lower priority than context-specific items
            if show_keywords {
                // Standard SQL keywords
                for (keyword, doc) in SQL_KEYWORDS {
                    if matches_filter(keyword) {
                        let matches_prefix = !current_word.is_empty()
                            && keyword.to_uppercase().starts_with(&current_word);
                        let score = completion_priority::calculate_score(
                            &context,
                            Some(CompletionItemKind::KEYWORD),
                            matches_prefix,
                        );
                        items.push(CompletionItem {
                            label: keyword.to_string(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                InsertReplaceEdit {
                                    new_text: keyword.to_string(),
                                    insert: replace_range,
                                    replace: replace_range,
                                },
                            )),
                            filter_text: Some(matched_prefix(keyword)),
                            documentation: Some(lsp_types::Documentation::String(doc.to_string())),
                            sort_text: Some(completion_priority::score_to_sort_text(
                                score, keyword,
                            )),
                            ..Default::default()
                        });
                    }
                }
                // Database-specific keywords
                if let Some(ref info) = db_info {
                    for (keyword, doc) in &info.keywords {
                        if matches_filter(keyword) {
                            let matches_prefix = !current_word.is_empty()
                                && keyword.to_uppercase().starts_with(&current_word);
                            let score = completion_priority::calculate_score(
                                &context,
                                Some(CompletionItemKind::KEYWORD),
                                matches_prefix,
                            );
                            items.push(CompletionItem {
                                label: keyword.to_string(),
                                kind: Some(CompletionItemKind::KEYWORD),
                                text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                    InsertReplaceEdit {
                                        new_text: keyword.to_string(),
                                        insert: replace_range,
                                        replace: replace_range,
                                    },
                                )),
                                filter_text: Some(matched_prefix(keyword)),
                                documentation: Some(lsp_types::Documentation::String(
                                    doc.to_string(),
                                )),
                                sort_text: Some(completion_priority::score_to_sort_text(
                                    score, keyword,
                                )),
                                ..Default::default()
                            });
                        }
                    }
                    // Database-specific operators - higher priority in Condition context
                    for (op, doc) in &info.operators {
                        if matches_filter(op) {
                            let matches_prefix = !current_word.is_empty()
                                && op.to_uppercase().starts_with(&current_word);
                            let score = completion_priority::calculate_score(
                                &context,
                                Some(CompletionItemKind::OPERATOR),
                                matches_prefix,
                            );
                            items.push(CompletionItem {
                                label: op.to_string(),
                                kind: Some(CompletionItemKind::OPERATOR),
                                text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                    InsertReplaceEdit {
                                        new_text: op.to_string(),
                                        insert: replace_range,
                                        replace: replace_range,
                                    },
                                )),
                                filter_text: Some(matched_prefix(op)),
                                documentation: Some(lsp_types::Documentation::String(
                                    doc.to_string(),
                                )),
                                sort_text: Some(completion_priority::score_to_sort_text(score, op)),
                                ..Default::default()
                            });
                        }
                    }
                }
            }

            // Functions - priority based on context (Requirement 5.3)
            if show_functions {
                // Standard SQL functions
                for (func, doc) in SQL_FUNCTIONS {
                    let func_name = func.split('(').next().unwrap_or("");
                    if matches_filter(func_name) {
                        let matches_prefix = !current_word.is_empty()
                            && func_name.to_uppercase().starts_with(&current_word);
                        let score = completion_priority::calculate_score(
                            &context,
                            Some(CompletionItemKind::FUNCTION),
                            matches_prefix,
                        );
                        items.push(CompletionItem {
                            label: func.to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                InsertReplaceEdit {
                                    new_text: func.to_string(),
                                    insert: replace_range,
                                    replace: replace_range,
                                },
                            )),
                            filter_text: Some(matched_prefix(func_name)),
                            documentation: Some(lsp_types::Documentation::String(doc.to_string())),
                            sort_text: Some(completion_priority::score_to_sort_text(score, func)),
                            ..Default::default()
                        });
                    }
                }
                // Database-specific functions
                if let Some(ref info) = db_info {
                    for (func, doc) in &info.functions {
                        let func_name = func.split('(').next().unwrap_or("");
                        if matches_filter(func_name) {
                            let matches_prefix = !current_word.is_empty()
                                && func_name.to_uppercase().starts_with(&current_word);
                            let score = completion_priority::calculate_score(
                                &context,
                                Some(CompletionItemKind::FUNCTION),
                                matches_prefix,
                            );
                            items.push(CompletionItem {
                                label: func.to_string(),
                                kind: Some(CompletionItemKind::FUNCTION),
                                text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                    InsertReplaceEdit {
                                        new_text: func.to_string(),
                                        insert: replace_range,
                                        replace: replace_range,
                                    },
                                )),
                                filter_text: Some(matched_prefix(func_name)),
                                documentation: Some(lsp_types::Documentation::String(
                                    doc.to_string(),
                                )),
                                sort_text: Some(completion_priority::score_to_sort_text(
                                    score, func,
                                )),
                                ..Default::default()
                            });
                        }
                    }
                }
            }

            // Data types - priority based on context
            // 在 CreateTable 上下文中，数据类型有最高优先级
            if show_types {
                // 收集数据库特定的数据类型名（大写），用于去重
                let mut seen_types = std::collections::HashSet::new();

                // 先添加数据库特定的数据类型（优先级更高，因为更精确）
                if let Some(ref info) = db_info {
                    for (dtype, doc) in &info.data_types {
                        seen_types.insert(dtype.to_uppercase());
                        if matches_filter(dtype) {
                            let matches_prefix = !current_word.is_empty()
                                && dtype.to_uppercase().starts_with(&current_word);
                            let score = completion_priority::calculate_score(
                                &context,
                                Some(CompletionItemKind::TYPE_PARAMETER),
                                matches_prefix,
                            );
                            items.push(CompletionItem {
                                label: dtype.to_string(),
                                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                                text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                    InsertReplaceEdit {
                                        new_text: dtype.to_string(),
                                        insert: replace_range,
                                        replace: replace_range,
                                    },
                                )),
                                filter_text: Some(matched_prefix(dtype)),
                                documentation: Some(lsp_types::Documentation::String(
                                    doc.to_string(),
                                )),
                                sort_text: Some(completion_priority::score_to_sort_text(
                                    score, dtype,
                                )),
                                ..Default::default()
                            });
                        }
                    }
                }

                // 再添加内置标准 SQL 数据类型（去除与数据库特定类型重复的）
                for (dtype, doc) in SQL_DATA_TYPES {
                    if seen_types.contains(&dtype.to_uppercase()) {
                        continue;
                    }
                    if matches_filter(dtype) {
                        let matches_prefix = !current_word.is_empty()
                            && dtype.to_uppercase().starts_with(&current_word);
                        let score = completion_priority::calculate_score(
                            &context,
                            Some(CompletionItemKind::TYPE_PARAMETER),
                            matches_prefix,
                        );
                        items.push(CompletionItem {
                            label: dtype.to_string(),
                            kind: Some(CompletionItemKind::TYPE_PARAMETER),
                            text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                InsertReplaceEdit {
                                    new_text: dtype.to_string(),
                                    insert: replace_range,
                                    replace: replace_range,
                                },
                            )),
                            filter_text: Some(matched_prefix(dtype)),
                            documentation: Some(lsp_types::Documentation::String(
                                doc.to_string(),
                            )),
                            sort_text: Some(completion_priority::score_to_sort_text(
                                score, dtype,
                            )),
                            ..Default::default()
                        });
                    }
                }
            }

            // Snippets - lowest priority (only at start)
            if matches!(context, SqlContext::Start) {
                // Default snippets
                let default_snippets: &[(&str, &str, &str)] = &[
                    ("sel*", "SELECT * FROM $1 WHERE $2", "Select all columns"),
                    ("selc", "SELECT COUNT(*) FROM $1 WHERE $2", "Count rows"),
                    ("ins", "INSERT INTO $1 ($2) VALUES ($3)", "Insert row"),
                    ("upd", "UPDATE $1 SET $2 WHERE $3", "Update rows"),
                    ("del", "DELETE FROM $1 WHERE $2", "Delete rows"),
                ];
                for (label, insert_text, doc) in default_snippets {
                    if matches_filter(label) {
                        let matches_prefix = !current_word.is_empty()
                            && label.to_uppercase().starts_with(&current_word);
                        let score = completion_priority::calculate_score(
                            &context,
                            Some(CompletionItemKind::SNIPPET),
                            matches_prefix,
                        );
                        items.push(CompletionItem {
                            label: label.to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                InsertReplaceEdit {
                                    new_text: insert_text.to_string(),
                                    insert: replace_range,
                                    replace: replace_range,
                                },
                            )),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            filter_text: Some(matched_prefix(label)),
                            documentation: Some(lsp_types::Documentation::String(doc.to_string())),
                            sort_text: Some(completion_priority::score_to_sort_text(score, label)),
                            ..Default::default()
                        });
                    }
                }
                // Database-specific snippets
                if let Some(ref info) = db_info {
                    for (label, insert_text, doc) in &info.snippets {
                        if matches_filter(label) {
                            let matches_prefix = !current_word.is_empty()
                                && label.to_uppercase().starts_with(&current_word);
                            let score = completion_priority::calculate_score(
                                &context,
                                Some(CompletionItemKind::SNIPPET),
                                matches_prefix,
                            );
                            items.push(CompletionItem {
                                label: label.to_string(),
                                kind: Some(CompletionItemKind::SNIPPET),
                                text_edit: Some(CompletionTextEdit::InsertAndReplace(
                                    InsertReplaceEdit {
                                        new_text: insert_text.to_string(),
                                        insert: replace_range,
                                        replace: replace_range,
                                    },
                                )),
                                insert_text_format: Some(InsertTextFormat::SNIPPET),
                                filter_text: Some(matched_prefix(label)),
                                documentation: Some(lsp_types::Documentation::String(
                                    doc.to_string(),
                                )),
                                sort_text: Some(completion_priority::score_to_sort_text(
                                    score, label,
                                )),
                                ..Default::default()
                            });
                        }
                    }
                }
            }

            items.sort_by(|a, b| {
                a.sort_text
                    .as_ref()
                    .unwrap_or(&a.label)
                    .cmp(b.sort_text.as_ref().unwrap_or(&b.label))
            });
            items.truncate(50);
            Ok(CompletionResponse::Array(items))
        })
    }

    fn inline_completion(
        &self,
        rope: &Rope,
        offset: usize,
        _trigger: InlineCompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<InlineCompletionResponse>> {
        let rope = rope.clone();
        let schema = self.schema.clone();
        let db_info = self.db_completion_info.clone();

        cx.background_spawn(async move {
            let text = rope.to_string();
            let completer =
                crate::sql_inline_completion::SqlInlineCompleter::new(&schema, db_info.as_ref());

            match completer.suggest(&text, offset) {
                Some(insert_text) => Ok(InlineCompletionResponse::Array(vec![
                    InlineCompletionItem {
                        insert_text,
                        filter_text: None,
                        range: None,
                        command: None,
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    },
                ])),
                None => Ok(InlineCompletionResponse::Array(vec![])),
            }
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        self.is_completion_trigger_check(new_text)
    }
}

impl DefaultSqlCompletionProvider {
    /// 检查给定文本是否应该触发自动完成。
    /// 此方法可在测试中直接调用，无需 GPUI Context。
    ///
    /// 设计原则：只有 ASCII 字符触发补全，中文字符和中文标点不触发。
    /// SQL 语法使用 ASCII 字符，中文主要出现在注释或字符串值中。
    pub fn is_completion_trigger_check(&self, new_text: &str) -> bool {
        // 获取最后一个字符
        let last_char = match new_text.chars().last() {
            Some(c) => c,
            None => return false,
        };

        // 非 ASCII 字符不触发（中文字符、中文标点等）
        if !last_char.is_ascii() {
            return false;
        }

        // 换行符/制表符后不触发（用户正在格式化代码）
        if last_char == '\n' || last_char == '\r' || last_char == '\t' {
            return false;
        }

        true
    }
}

#[derive(Clone)]
pub struct TableMentionCompletionProvider {
    schema: SqlSchema,
}

impl TableMentionCompletionProvider {
    pub fn new(schema: SqlSchema) -> Self {
        Self { schema }
    }

    pub(crate) fn format_table_mention(table: &str) -> String {
        if Self::is_simple_mention_name(table) {
            return format!("@{} ", table);
        }
        if !table.contains('`') {
            return format!("@`{}` ", table);
        }
        format!("@\"{}\" ", table)
    }

    pub(crate) fn extract_mention_query(text: &str, offset: usize) -> Option<(usize, String)> {
        let mut offset = offset.min(text.len());
        while offset > 0 && !text.is_char_boundary(offset) {
            offset = offset.saturating_sub(1);
        }
        let before_cursor = &text[..offset];
        let at_index = before_cursor.rfind('@')?;
        if at_index > 0 {
            let before_at = before_cursor[..at_index].chars().last();
            if before_at.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                return None;
            }
        }
        let after_at = &before_cursor[at_index + 1..];
        if after_at.is_empty() {
            return Some((at_index, String::new()));
        }
        let first = after_at.chars().next()?;
        if first == '`' || first == '"' {
            let rest = &after_at[first.len_utf8()..];
            if rest.contains(first) {
                return None;
            }
            return Some((at_index, rest.to_string()));
        }
        if !after_at
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return None;
        }
        Some((at_index, after_at.to_string()))
    }

    fn is_simple_mention_name(name: &str) -> bool {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !(first.is_alphabetic() || first == '_') {
            return false;
        }
        chars.all(|c| c.is_alphanumeric() || c == '_')
    }
}

impl CompletionProvider for TableMentionCompletionProvider {
    fn completions(
        &self,
        rope: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let rope = rope.clone();
        let schema = self.schema.clone();

        cx.background_spawn(async move {
            let offset = rope.clip_offset(offset, Bias::Left);
            let text = rope.to_string();
            let Some((start_offset, prefix)) =
                TableMentionCompletionProvider::extract_mention_query(&text, offset)
            else {
                return Ok(CompletionResponse::Array(vec![]));
            };

            let prefix_lower = prefix.to_lowercase();
            let start_pos = rope.offset_to_position(start_offset);
            let end_pos = rope.offset_to_position(offset);
            let replace_range = LspRange::new(start_pos, end_pos);

            let mut items = Vec::new();
            for (table, doc) in &schema.tables {
                let table_lower = table.to_lowercase();
                if !prefix_lower.is_empty() && !table_lower.starts_with(&prefix_lower) {
                    continue;
                }
                let mention_text = TableMentionCompletionProvider::format_table_mention(table);
                let documentation = if doc.is_empty() {
                    None
                } else {
                    Some(lsp_types::Documentation::String(doc.clone()))
                };
                items.push(CompletionItem {
                    label: mention_text.clone(),
                    kind: Some(CompletionItemKind::STRUCT),
                    detail: Some("表".to_string()),
                    documentation,
                    text_edit: Some(CompletionTextEdit::InsertAndReplace(
                        InsertReplaceEdit {
                            new_text: mention_text,
                            insert: replace_range,
                            replace: replace_range,
                        },
                    )),
                    filter_text: if prefix.is_empty() {
                        None
                    } else {
                        Some(prefix.clone())
                    },
                    sort_text: Some(table_lower),
                    ..Default::default()
                });
            }

            items.sort_by(|a, b| {
                a.sort_text
                    .as_ref()
                    .unwrap_or(&a.label)
                    .cmp(b.sort_text.as_ref().unwrap_or(&b.label))
            });
            items.truncate(50);
            Ok(CompletionResponse::Array(items))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        let Some(last_char) = new_text.chars().last() else {
            return false;
        };
        !last_char.is_whitespace()
    }
}

/// A reusable SQL editor component built on top of `Input`.
pub struct SqlEditor {
    editor: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl SqlEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            let mut editor = InputState::new(window, cx)
                .code_editor(Language::Sql)
                .line_number(true)
                .searchable(true)
                .indent_guides(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .placeholder("Enter your SQL query here...");

            // Defaults: completion + hover + actions
            let default_schema = SqlSchema::default();
            editor.lsp.completion_provider =
                Some(Rc::new(DefaultSqlCompletionProvider::new(default_schema)));

            editor
        });

        let _subscriptions =
            vec![
                cx.subscribe_in(&editor, window, move |_, _, _: &InputEvent, _window, cx| {
                    cx.notify()
                }),
            ];

        Self {
            editor,
            _subscriptions,
        }
    }

    /// Set database-specific completion information from plugin
    pub fn set_db_completion_info(
        &mut self,
        info: SqlCompletionInfo,
        schema: SqlSchema,
        cx: &mut Context<Self>,
    ) {
        let completion_provider =
            DefaultSqlCompletionProvider::new(schema).with_db_completion_info(info.clone());
        self.editor.update(cx, |state, _| {
            state.lsp.completion_provider = Some(Rc::new(completion_provider));
        });
    }

    /// Access underlying editor state.
    pub fn input(&self) -> Entity<InputState> {
        self.editor.clone()
    }

    /// Replace default completion provider.
    pub fn set_completion_provider(
        &mut self,
        provider: Rc<dyn CompletionProvider>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editor.update(cx, |state, _| {
            state.lsp.completion_provider = Some(provider)
        });
    }

    /// Set schema for default completion provider.
    pub fn set_schema(&mut self, schema: SqlSchema, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |state, _| {
            state.lsp.completion_provider =
                Some(Rc::new(DefaultSqlCompletionProvider::new(schema)));
        });
    }

    /// Replace hover provider.
    pub fn set_hover_provider(
        &mut self,
        provider: Rc<dyn HoverProvider>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editor
            .update(cx, |state, _| state.lsp.hover_provider = Some(provider));
    }

    /// Add a custom code action provider.
    pub fn add_code_action_provider(
        &mut self,
        provider: Rc<dyn CodeActionProvider>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editor.update(cx, |state, _| {
            state.lsp.code_action_providers.push(provider)
        });
    }

    /// Convenient toggles for consumers
    pub fn set_line_number(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |s, cx| s.set_line_number(on, window, cx));
    }
    pub fn set_soft_wrap(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |s, cx| s.set_soft_wrap(on, window, cx));
    }
    pub fn set_indent_guides(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |s, cx| s.set_indent_guides(on, window, cx));
    }
    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<gpui::SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let placeholder = placeholder.into();
        self.editor
            .update(cx, |s, cx| s.set_placeholder(placeholder, window, cx));
    }
    pub fn set_value(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |s, cx| s.set_value(text, window, cx));
    }

    /// Get the current text content of the editor.
    /// This is a convenience method that accesses the underlying InputState.
    pub fn get_text(&self, cx: &App) -> String {
        self.editor.read(cx).text().to_string()
    }

    /// Get the currently selected text.
    /// Returns an empty string if no text is selected.
    pub fn get_selected_text(&self, cx: &App) -> String {
        self.editor.read(cx).selected_text_string()
    }
}

impl Render for SqlEditor {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.editor)
            .size_full()
    }
}
