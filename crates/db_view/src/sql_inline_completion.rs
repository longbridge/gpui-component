use crate::sql_editor::SqlSchema;
use db::plugin::SqlCompletionInfo;
use db::sql_editor::sql_context_inferrer::{ContextInferrer, SqlContext, SqlContextInfo};
use db::sql_editor::sql_symbol_table::SymbolTable;
use db::sql_editor::sql_tokenizer::{SqlKeyword, SqlToken, SqlTokenKind, SqlTokenizer};

/// SQL inline completion engine.
///
/// Provides context-aware "ghost text" suggestions for SQL editing.
/// Pattern-based completion engine for keywords, clause templates,
/// and schema-aware identifiers.
pub struct SqlInlineCompleter<'a> {
    schema: &'a SqlSchema,
    db_info: Option<&'a SqlCompletionInfo>,
}

/// Compound keyword completions: maps uppercase prefix to its natural continuation.
/// Ordered by expected frequency for readability.
const COMPOUND_KEYWORDS: &[(&str, &str)] = &[
    // SELECT
    ("SEL", "ECT "),
    ("SELE", "CT "),
    ("SELEC", "T "),
    // INSERT INTO
    ("INS", "ERT INTO "),
    ("INSE", "RT INTO "),
    ("INSER", "T INTO "),
    ("INSERT", " INTO "),
    // UPDATE
    ("UPD", "ATE "),
    ("UPDA", "TE "),
    ("UPDAT", "E "),
    // DELETE FROM
    ("DEL", "ETE FROM "),
    ("DELE", "TE FROM "),
    ("DELET", "E FROM "),
    ("DELETE", " FROM "),
    // CREATE TABLE
    ("CRE", "ATE TABLE "),
    ("CREA", "TE TABLE "),
    ("CREAT", "E TABLE "),
    ("CREATE", " TABLE "),
    // WHERE
    ("WH", "ERE "),
    ("WHE", "RE "),
    ("WHER", "E "),
    // ORDER BY
    ("ORD", "ER BY "),
    ("ORDE", "R BY "),
    ("ORDER", " BY "),
    // GROUP BY
    ("GRO", "UP BY "),
    ("GROU", "P BY "),
    ("GROUP", " BY "),
    // HAVING
    ("HAV", "ING "),
    ("HAVI", "NG "),
    ("HAVIN", "G "),
    // LIMIT
    ("LIM", "IT "),
    ("LIMI", "T "),
    // BETWEEN
    ("BET", "WEEN "),
    ("BETW", "EEN "),
    ("BETWE", "EN "),
    ("BETWEE", "N "),
    // DISTINCT
    ("DIS", "TINCT "),
    ("DIST", "INCT "),
    ("DISTI", "NCT "),
    ("DISTIN", "CT "),
    ("DISTINC", "T "),
    // TRUNCATE
    ("TRUN", "CATE "),
    ("TRUNC", "ATE "),
    ("TRUNCA", "TE "),
    ("TRUNCAT", "E "),
    // ALTER
    ("ALT", "ER "),
    ("ALTE", "R "),
    // EXISTS
    ("EXI", "STS "),
    ("EXIS", "TS "),
    ("EXIST", "S "),
    // VALUES
    ("VAL", "UES "),
    ("VALU", "ES "),
    ("VALUE", "S "),
    // JOIN variants
    ("INNER", " JOIN "),
    ("INN", "ER JOIN "),
    ("INNE", "R JOIN "),
    ("LEFT", " JOIN "),
    ("RIGHT", " JOIN "),
    ("RIGH", "T JOIN "),
    ("CROSS", " JOIN "),
    ("CROS", "S JOIN "),
    ("FULL", " JOIN "),
    // COALESCE
    ("COA", "LESCE()"),
    ("COAL", "ESCE()"),
    ("COALE", "SCE()"),
    ("COALES", "CE()"),
    ("COALESC", "E()"),
];

/// Single-word SQL keywords for fallback matching (no docs needed for inline completion).
const SINGLE_KEYWORDS: &[&str] = &[
    "SELECT",
    "INSERT",
    "UPDATE",
    "DELETE",
    "CREATE",
    "ALTER",
    "DROP",
    "TRUNCATE",
    "FROM",
    "WHERE",
    "JOIN",
    "ON",
    "USING",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "VALUES",
    "INTO",
    "SET",
    "AND",
    "OR",
    "NOT",
    "IN",
    "EXISTS",
    "BETWEEN",
    "LIKE",
    "AS",
    "DISTINCT",
    "ALL",
    "UNION",
    "INTERSECT",
    "EXCEPT",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "WITH",
    "TABLE",
    "INDEX",
    "VIEW",
    "REFERENCES",
    "UNIQUE",
    "CHECK",
    "DEFAULT",
    "NULL",
    "TRUE",
    "FALSE",
    "ASC",
    "DESC",
];

/// SQL function names for inline keyword matching.
const FUNCTION_NAMES: &[&str] = &[
    "COUNT",
    "SUM",
    "AVG",
    "MIN",
    "MAX",
    "COALESCE",
    "NULLIF",
    "CAST",
    "UPPER",
    "LOWER",
    "TRIM",
    "LENGTH",
    "SUBSTRING",
    "CONCAT",
    "REPLACE",
    "ABS",
    "ROUND",
    "FLOOR",
    "CEIL",
    "NOW",
];

/// SQL data types for inline keyword matching.
const DATA_TYPE_NAMES: &[&str] = &[
    "INT",
    "INTEGER",
    "BIGINT",
    "SMALLINT",
    "TINYINT",
    "FLOAT",
    "DOUBLE",
    "DECIMAL",
    "NUMERIC",
    "REAL",
    "CHAR",
    "VARCHAR",
    "TEXT",
    "NCHAR",
    "NVARCHAR",
    "BOOLEAN",
    "BOOL",
    "DATE",
    "TIME",
    "DATETIME",
    "TIMESTAMP",
    "BLOB",
    "CLOB",
    "BINARY",
    "VARBINARY",
    "JSON",
    "XML",
    "UUID",
    "SERIAL",
];

/// Category of a keyword match, used to determine the suffix appended by inline completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeywordCategory {
    Keyword,
    Function,
    DataType,
}

/// Tracks which SQL clauses already exist in the current statement,
/// used to avoid suggesting duplicate clauses.
struct ExistingClauses {
    has_where: bool,
    has_order: bool,
    has_group: bool,
    has_limit: bool,
    has_on: bool,
}

impl ExistingClauses {
    fn scan(tokens: &[&SqlToken]) -> Self {
        let mut clauses = Self {
            has_where: false,
            has_order: false,
            has_group: false,
            has_limit: false,
            has_on: false,
        };
        for t in tokens {
            if let SqlTokenKind::Keyword(kw) = &t.kind {
                match kw {
                    SqlKeyword::Where => clauses.has_where = true,
                    SqlKeyword::Order => clauses.has_order = true,
                    SqlKeyword::Group => clauses.has_group = true,
                    SqlKeyword::Limit => clauses.has_limit = true,
                    SqlKeyword::On => clauses.has_on = true,
                    _ => {}
                }
            }
        }
        clauses
    }
}

impl<'a> SqlInlineCompleter<'a> {
    pub fn new(schema: &'a SqlSchema, db_info: Option<&'a SqlCompletionInfo>) -> Self {
        Self { schema, db_info }
    }

    /// Compute an inline completion suggestion.
    ///
    /// Returns the text to insert after cursor position, or `None` if no suggestion.
    pub fn suggest(&self, text: &str, offset: usize) -> Option<String> {
        if offset > text.len() {
            return None;
        }

        let before_cursor = &text[..offset];

        // Guard: skip if the text after the last semicolon is all whitespace
        if let Some(last_semi) = before_cursor.rfind(';') {
            let after_semi = &before_cursor[last_semi + 1..];
            if after_semi.trim().is_empty() {
                return None;
            }
        }

        // Tokenize once, share across all matchers
        let mut tokenizer = SqlTokenizer::new(text);
        let tokens = tokenizer.tokenize();

        // Guard: skip if inside comment or string
        if self.is_in_comment_or_string(&tokens, offset) {
            return None;
        }

        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        let context_info = ContextInferrer::infer_with_info(&tokens, offset, &symbol_table);

        let partial = extract_partial_word(before_cursor);

        // Context-aware pipeline priority:
        // For contexts where schema items are most relevant, try schema first.
        let schema_first = matches!(
            &context_info.context,
            SqlContext::TableName
                | SqlContext::DotColumn(_)
                | SqlContext::SelectColumns
                | SqlContext::Condition
                | SqlContext::OrderBy
        );

        if schema_first {
            self.try_schema_completion(&context_info.context, partial, &symbol_table)
                .or_else(|| {
                    self.try_clause_template(
                        &context_info,
                        &tokens,
                        offset,
                        &symbol_table,
                        before_cursor,
                    )
                })
                .or_else(|| self.try_keyword_completion(partial))
        } else {
            self.try_keyword_completion(partial)
                .or_else(|| {
                    self.try_clause_template(
                        &context_info,
                        &tokens,
                        offset,
                        &symbol_table,
                        before_cursor,
                    )
                })
                .or_else(|| {
                    self.try_schema_completion(&context_info.context, partial, &symbol_table)
                })
        }
    }

    /// Match 1: Complete a partially-typed SQL keyword.
    fn try_keyword_completion(&self, partial: &str) -> Option<String> {
        if partial.len() < 2 {
            return None;
        }

        let upper = partial.to_uppercase();

        // Compound keywords first (higher value, e.g. INSERT → INSERT INTO)
        for &(prefix, suffix) in COMPOUND_KEYWORDS {
            if upper == prefix {
                return Some(apply_case(suffix, partial));
            }
        }

        // Fallback: single-word keyword from SQL_KEYWORDS
        self.best_keyword_match(&upper, partial)
            .map(|(kw, category)| {
                let remaining = &kw[partial.len()..];
                let cased = apply_case(remaining, partial);
                match category {
                    KeywordCategory::Function => format!("{cased}()"),
                    KeywordCategory::Keyword | KeywordCategory::DataType => {
                        format!("{cased} ")
                    }
                }
            })
    }

    /// Find the best matching keyword for a prefix.
    /// Prefers shorter keywords to avoid over-completing.
    /// Returns the matched keyword (uppercased) and its category.
    fn best_keyword_match(
        &self,
        upper_prefix: &str,
        _partial: &str,
    ) -> Option<(String, KeywordCategory)> {
        let mut best: Option<(&str, KeywordCategory)> = None;

        // Search built-in keywords
        for &kw in SINGLE_KEYWORDS {
            if kw.starts_with(upper_prefix) && kw.len() > upper_prefix.len() {
                if best.map_or(true, |b| kw.len() < b.0.len()) {
                    best = Some((kw, KeywordCategory::Keyword));
                }
            }
        }

        // Search function names
        for &func in FUNCTION_NAMES {
            if func.starts_with(upper_prefix) && func.len() > upper_prefix.len() {
                if best.map_or(true, |b| func.len() < b.0.len()) {
                    best = Some((func, KeywordCategory::Function));
                }
            }
        }

        // Search data types
        for &dt in DATA_TYPE_NAMES {
            if dt.starts_with(upper_prefix) && dt.len() > upper_prefix.len() {
                if best.map_or(true, |b| dt.len() < b.0.len()) {
                    best = Some((dt, KeywordCategory::DataType));
                }
            }
        }

        // Search DB-specific keywords
        if let Some(db_info) = self.db_info {
            for (kw, _) in &db_info.keywords {
                if !kw.contains(' ')
                    && kw.to_uppercase().starts_with(upper_prefix)
                    && kw.len() > upper_prefix.len()
                {
                    if best.map_or(true, |b| kw.len() < b.0.len()) {
                        best = Some((kw, KeywordCategory::Keyword));
                    }
                }
            }
            for (dt, _) in &db_info.data_types {
                if !dt.contains(' ')
                    && dt.to_uppercase().starts_with(upper_prefix)
                    && dt.len() > upper_prefix.len()
                {
                    if best.map_or(true, |b| dt.len() < b.0.len()) {
                        best = Some((dt, KeywordCategory::DataType));
                    }
                }
            }
        }

        best.map(|(s, cat)| (s.to_uppercase(), cat))
    }

    /// Match 2: Suggest next logical clause template after a completed keyword/table name.
    fn try_clause_template(
        &self,
        context_info: &SqlContextInfo,
        tokens: &[SqlToken],
        offset: usize,
        symbol_table: &SymbolTable,
        before_cursor: &str,
    ) -> Option<String> {
        // Only suggest when cursor follows whitespace (not mid-word)
        if !before_cursor.ends_with(|c: char| c.is_ascii_whitespace()) {
            return None;
        }

        let meaningful = SqlTokenizer::tokens_before(tokens, offset);
        let last = meaningful.last()?;
        let existing = ExistingClauses::scan(&meaningful);

        match &context_info.context {
            // After bare SELECT keyword → suggest * FROM
            SqlContext::SelectColumns => {
                if meaningful.len() == 1 && last.is_keyword_of(SqlKeyword::Select) {
                    return Some("* FROM ".to_string());
                }
                // After SELECT DISTINCT → suggest * FROM
                if meaningful.len() == 2
                    && meaningful[0].is_keyword_of(SqlKeyword::Select)
                    && last.is_keyword_of(SqlKeyword::Distinct)
                {
                    return Some("* FROM ".to_string());
                }
                None
            }

            // After table name in FROM/JOIN context → suggest WHERE or ON
            SqlContext::TableName => {
                if !matches!(last.kind, SqlTokenKind::Ident | SqlTokenKind::QuotedIdent) {
                    return None;
                }

                let has_from = meaningful.iter().any(|t| t.is_keyword_of(SqlKeyword::From));
                let has_update = meaningful
                    .iter()
                    .any(|t| t.is_keyword_of(SqlKeyword::Update));
                let has_insert = meaningful
                    .iter()
                    .any(|t| t.is_keyword_of(SqlKeyword::Insert));

                // Check if the last keyword before the ident was JOIN
                let last_kw_is_join = meaningful
                    .iter()
                    .rev()
                    .skip(1)
                    .find(|t| t.is_keyword())
                    .map_or(false, |t| t.is_keyword_of(SqlKeyword::Join));

                if last_kw_is_join && !existing.has_on {
                    return Some("ON ".to_string());
                }
                if has_from && !existing.has_where {
                    return Some("WHERE ".to_string());
                }
                if has_update {
                    return Some("SET ".to_string());
                }
                if has_insert {
                    let table_name = &last.text;
                    if let Some(cols) = self.get_table_columns(table_name, symbol_table) {
                        if !cols.is_empty() {
                            let col_list = cols.join(", ");
                            return Some(format!("({col_list}) VALUES ()"));
                        }
                    }
                    return Some("() VALUES ()".to_string());
                }
                None
            }

            // After a column in condition → suggest = operator
            // After a value in condition → suggest AND
            SqlContext::Condition => {
                if matches!(last.kind, SqlTokenKind::Ident | SqlTokenKind::QuotedIdent) {
                    return Some("= ".to_string());
                }
                // After a value (number, string) → suggest AND
                if matches!(last.kind, SqlTokenKind::Number | SqlTokenKind::String) {
                    return Some("AND ".to_string());
                }
                None
            }

            // After ORDER BY column → suggest ASC
            SqlContext::OrderBy => {
                if matches!(last.kind, SqlTokenKind::Ident | SqlTokenKind::QuotedIdent) {
                    return Some("ASC ".to_string());
                }
                None
            }

            // After SET col = 'value' → suggest WHERE
            SqlContext::SetClause => {
                if matches!(last.kind, SqlTokenKind::Number | SqlTokenKind::String)
                    && !existing.has_where
                {
                    return Some("WHERE ".to_string());
                }
                None
            }

            _ => None,
        }
    }

    /// Match 3: Schema-aware table/column name completion.
    fn try_schema_completion(
        &self,
        context: &SqlContext,
        partial: &str,
        symbol_table: &SymbolTable,
    ) -> Option<String> {
        // DotColumn context needs only 1 character; others need 2
        let min_len = match context {
            SqlContext::DotColumn(_) => 1,
            _ => 2,
        };
        if partial.len() < min_len {
            return None;
        }

        let upper = partial.to_uppercase();

        match context {
            // After FROM/JOIN → complete table name
            SqlContext::TableName => {
                let mut best: Option<&str> = None;
                for (table, _) in &self.schema.tables {
                    if table.to_uppercase().starts_with(&upper) && table.len() > partial.len() {
                        if best.map_or(true, |b| table.len() < b.len()) {
                            best = Some(table);
                        }
                    }
                }
                best.map(|t| t[partial.len()..].to_string())
            }

            // After table.partial → complete column name
            SqlContext::DotColumn(alias_or_table) => {
                let resolved = symbol_table
                    .resolve(alias_or_table)
                    .unwrap_or(alias_or_table.as_str());

                self.find_column_match(resolved, &upper, partial.len())
            }

            // SELECT/WHERE/ORDER BY → complete column from any table
            SqlContext::SelectColumns | SqlContext::Condition | SqlContext::OrderBy => {
                // Search all columns across all tables
                let mut best: Option<&str> = None;
                for columns in self.schema.columns_by_table.values() {
                    for (col, _, _) in columns {
                        if col.to_uppercase().starts_with(&upper) && col.len() > partial.len() {
                            if best.map_or(true, |b| col.len() < b.len()) {
                                best = Some(col);
                            }
                        }
                    }
                }
                // Also check global columns
                for (col, _) in &self.schema.columns {
                    if col.to_uppercase().starts_with(&upper) && col.len() > partial.len() {
                        if best.map_or(true, |b| col.len() < b.len()) {
                            best = Some(col);
                        }
                    }
                }
                best.map(|c| c[partial.len()..].to_string())
            }

            // CREATE TABLE context → complete data type
            SqlContext::CreateTable => self.best_keyword_match(&upper, partial).map(|(kw, _)| {
                let remaining = &kw[partial.len()..];
                apply_case(remaining, partial)
            }),

            _ => None,
        }
    }

    /// Find a column match in a specific table (case-insensitive).
    fn find_column_match(
        &self,
        table_name: &str,
        upper_prefix: &str,
        prefix_len: usize,
    ) -> Option<String> {
        let columns = self.schema.columns_by_table.get(table_name).or_else(|| {
            let lower = table_name.to_lowercase();
            self.schema
                .columns_by_table
                .iter()
                .find(|(k, _)| k.to_lowercase() == lower)
                .map(|(_, v)| v)
        })?;

        let mut best: Option<&str> = None;
        for (col, _, _) in columns {
            if col.to_uppercase().starts_with(upper_prefix) && col.len() > prefix_len {
                if best.map_or(true, |b| col.len() < b.len()) {
                    best = Some(col);
                }
            }
        }
        best.map(|c| c[prefix_len..].to_string())
    }

    /// Get column names for a table from schema.
    fn get_table_columns(
        &self,
        table_name: &str,
        symbol_table: &SymbolTable,
    ) -> Option<Vec<String>> {
        let resolved = symbol_table.resolve(table_name).unwrap_or(table_name);
        let columns = self.schema.columns_by_table.get(resolved).or_else(|| {
            let lower = resolved.to_lowercase();
            self.schema
                .columns_by_table
                .iter()
                .find(|(k, _)| k.to_lowercase() == lower)
                .map(|(_, v)| v)
        })?;
        Some(columns.iter().map(|(name, _, _)| name.clone()).collect())
    }

    /// Check if offset is inside a comment or string literal.
    fn is_in_comment_or_string(&self, tokens: &[SqlToken], offset: usize) -> bool {
        for token in tokens {
            if offset > token.start
                && offset <= token.end
                && matches!(
                    token.kind,
                    SqlTokenKind::String | SqlTokenKind::LineComment | SqlTokenKind::BlockComment
                )
            {
                return true;
            }
        }
        false
    }
}

/// Extract the partial word at end of `before_cursor` (ASCII alphanumeric + underscore).
fn extract_partial_word(before_cursor: &str) -> &str {
    let bytes = before_cursor.as_bytes();
    let mut start = bytes.len();
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            start -= 1;
        } else {
            break;
        }
    }
    &before_cursor[start..]
}

/// Apply case style of `reference` to `text`.
/// If reference is all uppercase, return text uppercased.
/// If reference is all lowercase, return text lowercased.
/// Otherwise return text as-is.
fn apply_case(text: &str, reference: &str) -> String {
    if reference
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_uppercase())
    {
        text.to_uppercase()
    } else if reference
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_lowercase())
    {
        text.to_lowercase()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suggest(sql: &str, offset: usize) -> Option<String> {
        let schema = SqlSchema::default();
        let completer = SqlInlineCompleter::new(&schema, None);
        completer.suggest(sql, offset)
    }

    fn suggest_with_schema(sql: &str, offset: usize, schema: &SqlSchema) -> Option<String> {
        let completer = SqlInlineCompleter::new(schema, None);
        completer.suggest(sql, offset)
    }

    // === Keyword completion ===

    #[test]
    fn test_select_completion() {
        assert_eq!(suggest("SEL", 3), Some("ECT ".to_string()));
        assert_eq!(suggest("SELE", 4), Some("CT ".to_string()));
        assert_eq!(suggest("SELEC", 5), Some("T ".to_string()));
    }

    #[test]
    fn test_where_completion() {
        assert_eq!(suggest("SELECT * FROM t WH", 18), Some("ERE ".to_string()));
    }

    #[test]
    fn test_insert_compound() {
        assert_eq!(suggest("INS", 3), Some("ERT INTO ".to_string()));
        assert_eq!(suggest("INSERT", 6), Some(" INTO ".to_string()));
    }

    #[test]
    fn test_order_compound() {
        assert_eq!(suggest("ORDER", 5), Some(" BY ".to_string()));
        assert_eq!(suggest("GROUP", 5), Some(" BY ".to_string()));
    }

    #[test]
    fn test_case_preservation_lower() {
        assert_eq!(suggest("sel", 3), Some("ect ".to_string()));
        assert_eq!(suggest("wh", 2), Some("ere ".to_string()));
        assert_eq!(suggest("ins", 3), Some("ert into ".to_string()));
    }

    #[test]
    fn test_case_preservation_upper() {
        assert_eq!(suggest("SEL", 3), Some("ECT ".to_string()));
        assert_eq!(suggest("WH", 2), Some("ERE ".to_string()));
    }

    #[test]
    fn test_single_char_no_suggestion() {
        assert_eq!(suggest("S", 1), None);
        assert_eq!(suggest("W", 1), None);
    }

    #[test]
    fn test_fallback_keyword() {
        // "FR" should match FROM via fallback
        assert_eq!(suggest("FR", 2), Some("OM ".to_string()));
    }

    // === Clause template ===

    #[test]
    fn test_after_select_suggest_star_from() {
        assert_eq!(suggest("SELECT ", 7), Some("* FROM ".to_string()));
    }

    #[test]
    fn test_after_select_distinct_suggest_star_from() {
        assert_eq!(suggest("SELECT DISTINCT ", 16), Some("* FROM ".to_string()));
    }

    #[test]
    fn test_after_from_table_suggest_where() {
        assert_eq!(
            suggest("SELECT * FROM users ", 20),
            Some("WHERE ".to_string())
        );
    }

    #[test]
    fn test_after_update_table_suggest_set() {
        assert_eq!(suggest("UPDATE users ", 13), Some("SET ".to_string()));
    }

    #[test]
    fn test_after_condition_column_suggest_eq() {
        assert_eq!(
            suggest("SELECT * FROM t WHERE id ", 25),
            Some("= ".to_string())
        );
    }

    // === Schema-aware completion ===

    #[test]
    fn test_table_name_completion() {
        let schema = SqlSchema::default().with_tables([("users", "User table")]);
        assert_eq!(
            suggest_with_schema("SELECT * FROM us", 16, &schema),
            Some("ers".to_string())
        );
    }

    #[test]
    fn test_table_name_no_match() {
        let schema = SqlSchema::default().with_tables([("users", "User table")]);
        assert_eq!(suggest_with_schema("SELECT * FROM zz", 17, &schema), None);
    }

    #[test]
    fn test_dot_column_completion() {
        let schema = SqlSchema::default()
            .with_tables([("users", "")])
            .with_table_columns("users", [("name", "User name"), ("email", "Email")]);

        assert_eq!(
            suggest_with_schema("SELECT users.na", 15, &schema),
            Some("me".to_string())
        );
    }

    #[test]
    fn test_column_in_select() {
        let schema = SqlSchema::default()
            .with_tables([("users", "")])
            .with_table_columns("users", [("username", ""), ("email", "")]);

        assert_eq!(
            suggest_with_schema("SELECT us", 9, &schema),
            Some("ername".to_string())
        );
    }

    #[test]
    fn test_insert_with_schema_columns() {
        let schema = SqlSchema::default()
            .with_tables([("users", "")])
            .with_table_columns("users", [("id", ""), ("name", ""), ("email", "")]);

        assert_eq!(
            suggest_with_schema("INSERT INTO users ", 18, &schema),
            Some("(id, name, email) VALUES ()".to_string())
        );
    }

    // === Guard conditions ===

    #[test]
    fn test_no_suggestion_in_comment() {
        assert_eq!(suggest("-- SEL", 6), None);
    }

    #[test]
    fn test_no_suggestion_in_string() {
        assert_eq!(suggest("SELECT 'SEL", 11), None);
    }

    #[test]
    fn test_no_suggestion_after_semicolon() {
        assert_eq!(suggest("SELECT 1;", 9), None);
    }

    #[test]
    fn test_no_suggestion_empty() {
        assert_eq!(suggest("", 0), None);
    }

    #[test]
    fn test_offset_beyond_text() {
        assert_eq!(suggest("SEL", 100), None);
    }

    // === New tests: schema priority over keyword ===

    #[test]
    fn test_schema_priority_over_keyword_in_from() {
        let schema = SqlSchema::default().with_tables([("users", "User table")]);
        // "us" in FROM context should match schema "users" instead of keyword "USING"
        assert_eq!(
            suggest_with_schema("SELECT * FROM us", 16, &schema),
            Some("ers".to_string())
        );
    }

    #[test]
    fn test_keyword_fallback_when_no_schema() {
        // No schema tables, so "WH" should fall back to keyword "WHERE"
        assert_eq!(suggest("SELECT * FROM t WH", 18), Some("ERE ".to_string()));
    }

    // === New tests: DotColumn single char ===

    #[test]
    fn test_dot_column_single_char() {
        let schema = SqlSchema::default()
            .with_tables([("users", "")])
            .with_table_columns("users", [("name", ""), ("email", "")]);

        // Single char after dot should still complete
        assert_eq!(
            suggest_with_schema("SELECT users.n", 14, &schema),
            Some("ame".to_string())
        );
    }

    // === New tests: function completion with parens ===

    #[test]
    fn test_function_completion_with_parens() {
        // "COU" → "NT()" (function gets parentheses)
        assert_eq!(suggest("SELECT COU", 10), Some("NT()".to_string()));
    }

    #[test]
    fn test_function_sum_with_parens() {
        // "SU" → "M()" (function gets parentheses)
        assert_eq!(suggest("SELECT SU", 9), Some("M()".to_string()));
    }

    #[test]
    fn test_keyword_no_parens() {
        // "FR" → "OM " (keyword gets space, not parens)
        assert_eq!(suggest("FR", 2), Some("OM ".to_string()));
    }

    // === New tests: more clause templates ===

    #[test]
    fn test_after_join_table_suggest_on() {
        let sql = "SELECT * FROM users JOIN orders ";
        assert_eq!(suggest(sql, sql.len()), Some("ON ".to_string()));
    }

    #[test]
    fn test_after_order_by_column_suggest_asc() {
        let sql = "SELECT * FROM t ORDER BY id ";
        assert_eq!(suggest(sql, sql.len()), Some("ASC ".to_string()));
    }

    #[test]
    fn test_after_set_value_suggest_where() {
        let sql = "UPDATE t SET name = 'test' ";
        assert_eq!(suggest(sql, sql.len()), Some("WHERE ".to_string()));
    }

    #[test]
    fn test_after_condition_value_suggest_and() {
        let sql = "SELECT * FROM t WHERE id = 1 ";
        assert_eq!(suggest(sql, sql.len()), Some("AND ".to_string()));
    }

    // === New tests: multi-statement semicolon ===

    #[test]
    fn test_new_statement_after_semicolon() {
        // After semicolon with new text, should still suggest
        assert_eq!(suggest("SELECT 1; SEL", 13), Some("ECT ".to_string()));
    }

    #[test]
    fn test_empty_after_semicolon_no_suggestion() {
        // After semicolon with only whitespace, no suggestion
        assert_eq!(suggest("SELECT 1; ", 10), None);
    }
}
