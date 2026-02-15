use crate::sql_editor::sql_symbol_table::SymbolTable;
use crate::sql_editor::sql_tokenizer::{SqlKeyword, SqlToken, SqlTokenKind};
/// SQL Context Inferrer module for smart completion
///
/// This module provides context inference based on token stream analysis,
/// determining the semantic context at cursor position for contextually
/// relevant completions.

/// SQL context for smarter completion suggestions.
///
/// Represents the semantic context at cursor position, used to filter
/// and prioritize completion suggestions.
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
    /// After dot (alias.column), expecting column name from specific table
    DotColumn(String),
    /// After function name with open paren
    FunctionArgs,
}

/// Extended context information including scope and keyword tracking.
#[derive(Debug, Clone)]
pub struct SqlContextInfo {
    /// The inferred context type
    pub context: SqlContext,
    /// Current scope depth (0 = top level)
    pub scope_depth: usize,
    /// Last significant keyword before cursor
    pub last_keyword: Option<SqlKeyword>,
    /// Whether cursor is inside a subquery
    pub in_subquery: bool,
}

impl Default for SqlContextInfo {
    fn default() -> Self {
        Self {
            context: SqlContext::Start,
            scope_depth: 0,
            last_keyword: None,
            in_subquery: false,
        }
    }
}

/// Context inferrer for SQL completion.
///
/// Analyzes token stream to determine the semantic context at cursor position.
pub struct ContextInferrer;

impl ContextInferrer {
    /// Infer SQL context from tokens at cursor position.
    ///
    /// Analyzes the token stream before the cursor to determine what kind
    /// of completion suggestions are appropriate.
    ///
    /// # Arguments
    /// * `tokens` - The complete token stream from SqlTokenizer
    /// * `offset` - Cursor position (byte offset)
    /// * `symbol_table` - Symbol table with alias mappings
    ///
    /// # Returns
    /// The inferred SqlContext based on preceding tokens.
    pub fn infer(tokens: &[SqlToken], offset: usize, symbol_table: &SymbolTable) -> SqlContext {
        Self::infer_with_info(tokens, offset, symbol_table).context
    }

    /// Infer SQL context with extended information.
    ///
    /// Returns full context info including scope depth and keyword tracking.
    pub fn infer_with_info(
        tokens: &[SqlToken],
        offset: usize,
        _symbol_table: &SymbolTable,
    ) -> SqlContextInfo {
        // Get meaningful tokens before cursor (excluding whitespace, comments, EOF)
        let tokens_before: Vec<&SqlToken> = tokens
            .iter()
            .filter(|t| {
                t.end <= offset
                    && !matches!(
                        t.kind,
                        SqlTokenKind::Whitespace
                            | SqlTokenKind::LineComment
                            | SqlTokenKind::BlockComment
                            | SqlTokenKind::Eof
                    )
            })
            .collect();

        // Also check if we're currently typing after a dot
        let dot_context = Self::check_dot_context(tokens, offset);
        if let Some(alias) = dot_context {
            // Resolve alias through symbol table if available
            let resolved = Self::resolve_dot_alias(&alias, _symbol_table);
            return SqlContextInfo {
                context: SqlContext::DotColumn(resolved),
                scope_depth: 0,
                last_keyword: None,
                in_subquery: false,
            };
        }

        if tokens_before.is_empty() {
            return SqlContextInfo::default();
        }

        // Track parenthesis depth for subquery/function detection
        let mut paren_depth = 0;
        let mut last_keyword: Option<SqlKeyword> = None;
        let mut in_subquery = false;

        // Find the start of current statement (after last semicolon)
        let stmt_start_idx = tokens_before
            .iter()
            .rposition(|t| matches!(t.kind, SqlTokenKind::Semicolon))
            .map(|i| i + 1)
            .unwrap_or(0);

        let current_stmt_tokens = &tokens_before[stmt_start_idx..];

        // Analyze tokens in current statement
        for token in current_stmt_tokens.iter() {
            match &token.kind {
                SqlTokenKind::LParen => {
                    paren_depth += 1;
                    // Check if this starts a subquery (SELECT after paren)
                }
                SqlTokenKind::RParen => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    }
                }
                SqlTokenKind::Keyword(kw) => {
                    last_keyword = Some(kw.clone());
                    if paren_depth > 0 && matches!(kw, SqlKeyword::Select) {
                        in_subquery = true;
                    }
                }
                _ => {}
            }
        }

        // Determine context based on last significant tokens
        let context = Self::determine_context(current_stmt_tokens, paren_depth);

        SqlContextInfo {
            context,
            scope_depth: paren_depth as usize,
            last_keyword,
            in_subquery,
        }
    }

    /// Resolve a dot-prefix alias to its table name using the symbol table.
    ///
    /// If the alias is found in the symbol table, returns the resolved table name.
    /// Otherwise, returns the original alias (it might be a direct table name).
    ///
    /// # Arguments
    /// * `alias` - The alias or table name before the dot
    /// * `symbol_table` - Symbol table with alias mappings
    ///
    /// # Returns
    /// The resolved table name, or the original alias if not found.
    fn resolve_dot_alias(alias: &str, symbol_table: &SymbolTable) -> String {
        // Try to resolve through symbol table
        if let Some(table_name) = symbol_table.resolve(alias) {
            table_name.to_string()
        } else {
            // Not an alias, might be a direct table name
            alias.to_string()
        }
    }

    /// Check if cursor is in a DotColumn context (after `alias.`).
    ///
    /// Returns the alias/table name if in dot context, None otherwise.
    fn check_dot_context(tokens: &[SqlToken], offset: usize) -> Option<String> {
        // Find tokens that are relevant to the cursor position
        // We need to check if there's a pattern: Ident Dot [partial_ident]
        // where cursor is after the dot

        let meaningful_tokens: Vec<&SqlToken> = tokens
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    SqlTokenKind::Whitespace
                        | SqlTokenKind::LineComment
                        | SqlTokenKind::BlockComment
                        | SqlTokenKind::Eof
                )
            })
            .collect();

        // Find the last dot before or at cursor
        let mut last_dot_idx: Option<usize> = None;
        for (i, token) in meaningful_tokens.iter().enumerate() {
            if matches!(token.kind, SqlTokenKind::Dot) && token.end <= offset {
                last_dot_idx = Some(i);
            }
        }

        let dot_idx = last_dot_idx?;

        // Check if cursor is right after the dot or typing after it
        let dot_token = meaningful_tokens[dot_idx];

        // Cursor must be after the dot
        if offset < dot_token.end {
            return None;
        }

        // Check what comes after the dot
        if dot_idx + 1 < meaningful_tokens.len() {
            let next_token = meaningful_tokens[dot_idx + 1];
            // If there's a complete token after dot and cursor is past it,
            // we're not in dot context anymore (unless cursor is within that token)
            if next_token.start >= dot_token.end {
                // Check if cursor is within or right after the identifier after dot
                if offset > next_token.end {
                    // Cursor is past the identifier, check if there's more
                    // If the next token after the ident is not a dot, we're done
                    return None;
                }
                // Cursor is within or at the identifier after dot - still in dot context
            }
        }

        // Get the identifier before the dot
        if dot_idx == 0 {
            return None;
        }

        let before_dot = meaningful_tokens[dot_idx - 1];
        match &before_dot.kind {
            SqlTokenKind::Ident => Some(before_dot.text.clone()),
            SqlTokenKind::QuotedIdent => {
                // Remove quotes and unescape
                let s = &before_dot.text;
                if s.len() >= 2 {
                    Some(s[1..s.len() - 1].replace("\"\"", "\""))
                } else {
                    Some(s.clone())
                }
            }
            _ => None,
        }
    }

    /// Determine context based on the last significant tokens.
    fn determine_context(tokens: &[&SqlToken], paren_depth: i32) -> SqlContext {
        if tokens.is_empty() {
            return SqlContext::Start;
        }

        // Check for function args context (inside parentheses)
        if paren_depth > 0 {
            // Check if this is a function call, subquery, or CREATE TABLE
            // Look for the opening paren and what's before it
            let mut depth = 0;
            let mut paren_index = None;

            // Find the matching opening paren
            for (i, token) in tokens.iter().enumerate().rev() {
                match &token.kind {
                    SqlTokenKind::RParen => depth += 1,
                    SqlTokenKind::LParen => {
                        if depth == 0 {
                            // This is our opening paren
                            paren_index = Some(i);
                            break;
                        }
                        depth -= 1;
                    }
                    _ => {}
                }
            }

            // Check what's before the opening paren
            if let Some(paren_idx) = paren_index {
                // Look backwards from the paren to find TABLE keyword
                for i in (0..paren_idx).rev() {
                    match &tokens[i].kind {
                        SqlTokenKind::Keyword(SqlKeyword::Table) => {
                            // Found TABLE, check if preceded by CREATE
                            if i > 0 {
                                if let SqlTokenKind::Keyword(SqlKeyword::Create) =
                                    &tokens[i - 1].kind
                                {
                                    return SqlContext::CreateTable;
                                }
                            }
                            break;
                        }
                        SqlTokenKind::Whitespace
                        | SqlTokenKind::Ident
                        | SqlTokenKind::QuotedIdent => {
                            // Continue searching
                        }
                        _ => {
                            // Stop if we hit other tokens
                            break;
                        }
                    }
                }
            }

            // Otherwise treat as FunctionArgs
            return SqlContext::FunctionArgs;
        }

        // Find the last significant keyword
        let mut last_keyword_info: Option<(usize, &SqlKeyword)> = None;

        for (i, token) in tokens.iter().enumerate() {
            if let SqlTokenKind::Keyword(kw) = &token.kind {
                last_keyword_info = Some((i, kw));
            }
        }

        // Determine context based on last keyword and what follows
        match last_keyword_info {
            Some((idx, keyword)) => Self::context_from_keyword(keyword, tokens, idx),
            None => SqlContext::Start,
        }
    }

    /// Determine context from a keyword and its position.
    fn context_from_keyword(
        keyword: &SqlKeyword,
        tokens: &[&SqlToken],
        keyword_idx: usize,
    ) -> SqlContext {
        // Check what tokens follow the keyword
        let tokens_after_keyword = &tokens[keyword_idx + 1..];

        match keyword {
            // SELECT context
            SqlKeyword::Select | SqlKeyword::Distinct | SqlKeyword::All => {
                // After SELECT, we expect columns
                // But if we see FROM, we're past the column list
                if Self::has_keyword_after(tokens_after_keyword, &[SqlKeyword::From]) {
                    // We're past SELECT columns, check for later keywords
                    return Self::find_later_context(tokens_after_keyword);
                }
                SqlContext::SelectColumns
            }

            // FROM/JOIN context - expecting table names
            SqlKeyword::From
            | SqlKeyword::Join
            | SqlKeyword::Inner
            | SqlKeyword::Left
            | SqlKeyword::Right
            | SqlKeyword::Full
            | SqlKeyword::Cross => {
                // Check if we've moved past the table name
                // After table name, we might have alias, ON, WHERE, etc.
                if Self::has_keyword_after(
                    tokens_after_keyword,
                    &[
                        SqlKeyword::Where,
                        SqlKeyword::On,
                        SqlKeyword::Group,
                        SqlKeyword::Order,
                        SqlKeyword::Having,
                        SqlKeyword::Limit,
                    ],
                ) {
                    return Self::find_later_context(tokens_after_keyword);
                }
                // Check if we have a complete table reference (ident or ident ident)
                let ident_count = tokens_after_keyword
                    .iter()
                    .take_while(|t| {
                        matches!(
                            t.kind,
                            SqlTokenKind::Ident
                                | SqlTokenKind::QuotedIdent
                                | SqlTokenKind::Dot
                                | SqlTokenKind::Comma
                        ) || t.is_keyword_of(SqlKeyword::As)
                    })
                    .filter(|t| matches!(t.kind, SqlTokenKind::Ident | SqlTokenKind::QuotedIdent))
                    .count();

                // If we have 2+ identifiers (table + alias), might still be in table context
                // if there's a comma indicating more tables
                if let Some(last) = tokens_after_keyword.last() {
                    if matches!(last.kind, SqlTokenKind::Comma) {
                        return SqlContext::TableName;
                    }
                }

                if ident_count >= 2 {
                    // Have table and alias, check for more context
                    SqlContext::TableName
                } else {
                    SqlContext::TableName
                }
            }

            // INTO context (INSERT INTO)
            SqlKeyword::Into => SqlContext::TableName,

            // UPDATE context
            SqlKeyword::Update => {
                if Self::has_keyword_after(tokens_after_keyword, &[SqlKeyword::Set]) {
                    return Self::find_later_context(tokens_after_keyword);
                }
                SqlContext::TableName
            }

            // WHERE/AND/OR/ON/HAVING - condition context
            SqlKeyword::Where
            | SqlKeyword::And
            | SqlKeyword::Or
            | SqlKeyword::On
            | SqlKeyword::Having => SqlContext::Condition,

            // ORDER BY / GROUP BY
            SqlKeyword::Order | SqlKeyword::Group => {
                // Need to check if BY follows
                if Self::has_keyword_after(tokens_after_keyword, &[SqlKeyword::By]) {
                    SqlContext::OrderBy
                } else {
                    SqlContext::Start
                }
            }

            SqlKeyword::By => {
                // Check what's before BY
                if keyword_idx > 0 {
                    if let SqlTokenKind::Keyword(prev_kw) = &tokens[keyword_idx - 1].kind {
                        if matches!(prev_kw, SqlKeyword::Order | SqlKeyword::Group) {
                            return SqlContext::OrderBy;
                        }
                    }
                }
                SqlContext::Start
            }

            // SET clause (UPDATE ... SET)
            SqlKeyword::Set => SqlContext::SetClause,

            // VALUES clause
            SqlKeyword::Values => SqlContext::Values,

            // CREATE TABLE
            SqlKeyword::Create => {
                if Self::has_keyword_after(tokens_after_keyword, &[SqlKeyword::Table]) {
                    SqlContext::CreateTable
                } else {
                    SqlContext::Start
                }
            }

            SqlKeyword::Table => {
                // Check if preceded by CREATE
                if keyword_idx > 0 {
                    if let SqlTokenKind::Keyword(prev_kw) = &tokens[keyword_idx - 1].kind {
                        if matches!(prev_kw, SqlKeyword::Create) {
                            return SqlContext::CreateTable;
                        }
                    }
                }
                SqlContext::Start
            }

            // Default for other keywords
            _ => SqlContext::Start,
        }
    }

    /// Check if any of the specified keywords appear in the token slice.
    fn has_keyword_after(tokens: &[&SqlToken], keywords: &[SqlKeyword]) -> bool {
        tokens.iter().any(|t| {
            if let SqlTokenKind::Keyword(kw) = &t.kind {
                keywords.contains(kw)
            } else {
                false
            }
        })
    }

    /// Find context from later keywords in the token stream.
    fn find_later_context(tokens: &[&SqlToken]) -> SqlContext {
        // Find the last significant keyword
        for token in tokens.iter().rev() {
            if let SqlTokenKind::Keyword(kw) = &token.kind {
                match kw {
                    SqlKeyword::Where
                    | SqlKeyword::And
                    | SqlKeyword::Or
                    | SqlKeyword::On
                    | SqlKeyword::Having => {
                        return SqlContext::Condition;
                    }
                    SqlKeyword::Order | SqlKeyword::Group => {
                        return SqlContext::OrderBy;
                    }
                    SqlKeyword::By => {
                        // Check context from BY
                        continue;
                    }
                    SqlKeyword::Set => {
                        return SqlContext::SetClause;
                    }
                    SqlKeyword::From
                    | SqlKeyword::Join
                    | SqlKeyword::Inner
                    | SqlKeyword::Left
                    | SqlKeyword::Right
                    | SqlKeyword::Full
                    | SqlKeyword::Cross => {
                        return SqlContext::TableName;
                    }
                    SqlKeyword::Select => {
                        return SqlContext::SelectColumns;
                    }
                    _ => continue,
                }
            }
        }
        SqlContext::Start
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_editor::sql_symbol_table::SymbolTable;
    use crate::sql_editor::sql_tokenizer::SqlTokenizer;

    fn infer_context(sql: &str, offset: usize) -> SqlContext {
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        ContextInferrer::infer(&tokens, offset, &symbol_table)
    }

    #[test]
    fn test_start_context() {
        assert_eq!(infer_context("", 0), SqlContext::Start);
        assert_eq!(infer_context("   ", 3), SqlContext::Start);
    }

    #[test]
    fn test_select_columns_context() {
        // After SELECT
        assert_eq!(infer_context("SELECT ", 7), SqlContext::SelectColumns);
        assert_eq!(infer_context("SELECT id, ", 11), SqlContext::SelectColumns);
        assert_eq!(
            infer_context("SELECT DISTINCT ", 16),
            SqlContext::SelectColumns
        );
    }

    #[test]
    fn test_table_name_context() {
        // After FROM
        assert_eq!(infer_context("SELECT * FROM ", 14), SqlContext::TableName);
        // After JOIN
        assert_eq!(
            infer_context("SELECT * FROM users JOIN ", 25),
            SqlContext::TableName
        );
        // After LEFT JOIN
        assert_eq!(
            infer_context("SELECT * FROM users LEFT JOIN ", 30),
            SqlContext::TableName
        );
    }

    #[test]
    fn test_condition_context() {
        // After WHERE
        assert_eq!(
            infer_context("SELECT * FROM users WHERE ", 26),
            SqlContext::Condition
        );
        // After AND
        assert_eq!(
            infer_context("SELECT * FROM users WHERE id = 1 AND ", 37),
            SqlContext::Condition
        );
        // After OR
        assert_eq!(
            infer_context("SELECT * FROM users WHERE id = 1 OR ", 36),
            SqlContext::Condition
        );
    }

    #[test]
    fn test_order_by_context() {
        assert_eq!(
            infer_context("SELECT * FROM users ORDER BY ", 29),
            SqlContext::OrderBy
        );
        assert_eq!(
            infer_context("SELECT * FROM users GROUP BY ", 29),
            SqlContext::OrderBy
        );
    }

    #[test]
    fn test_dot_column_context() {
        // After alias.
        assert_eq!(
            infer_context("SELECT u.", 9),
            SqlContext::DotColumn("u".to_string())
        );
        assert_eq!(
            infer_context("SELECT u.id, u.", 15),
            SqlContext::DotColumn("u".to_string())
        );
        // With table name
        assert_eq!(
            infer_context("SELECT users.", 13),
            SqlContext::DotColumn("users".to_string())
        );
    }

    #[test]
    fn test_function_args_context() {
        // Inside function parentheses
        assert_eq!(infer_context("SELECT COUNT(", 13), SqlContext::FunctionArgs);
        assert_eq!(
            infer_context("SELECT MAX(id, ", 15),
            SqlContext::FunctionArgs
        );
    }

    #[test]
    fn test_set_clause_context() {
        assert_eq!(
            infer_context("UPDATE users SET ", 17),
            SqlContext::SetClause
        );
    }

    #[test]
    fn test_values_context() {
        assert_eq!(
            infer_context("INSERT INTO users VALUES ", 25),
            SqlContext::Values
        );
    }

    #[test]
    fn test_create_table_context() {
        assert_eq!(infer_context("CREATE TABLE ", 13), SqlContext::CreateTable);
        // Test inside CREATE TABLE parentheses - should still be CreateTable context
        assert_eq!(
            infer_context("CREATE TABLE users (", 20),
            SqlContext::CreateTable
        );
        assert_eq!(
            infer_context("CREATE TABLE users (id ", 23),
            SqlContext::CreateTable
        );
        assert_eq!(
            infer_context("CREATE TABLE users (id INT, name ", 33),
            SqlContext::CreateTable
        );
    }

    #[test]
    fn test_complex_query_context() {
        let sql = "SELECT u.id, u.name FROM users u WHERE u.";
        let offset = sql.len();
        // With symbol table, 'u' should resolve to 'users'
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("users".to_string())
        );
    }

    #[test]
    fn test_dot_column_with_alias_resolution() {
        // Test that alias 'u' resolves to table 'users'
        let sql = "SELECT u. FROM users u";
        let offset = 9; // After "SELECT u."
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("users".to_string())
        );
    }

    #[test]
    fn test_dot_column_with_join_alias() {
        // Test alias resolution with JOIN
        let sql = "SELECT u.id, d. FROM users u JOIN departments d ON u.dept_id = d.id";
        let offset = 15; // After "SELECT u.id, d."
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("departments".to_string())
        );
    }

    #[test]
    fn test_dot_column_without_alias() {
        // When no alias is defined, table name should be returned as-is
        let sql = "SELECT users. FROM users";
        let offset = 13; // After "SELECT users."
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("users".to_string())
        );
    }

    #[test]
    fn test_dot_column_with_as_keyword() {
        // Test alias with AS keyword
        let sql = "SELECT u. FROM users AS u";
        let offset = 9; // After "SELECT u."
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("users".to_string())
        );
    }

    #[test]
    fn test_dot_column_unknown_alias() {
        // Unknown alias should be returned as-is
        let sql = "SELECT x. FROM users u";
        let offset = 9; // After "SELECT x."
                        // 'x' is not defined as an alias, so it's returned as-is
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("x".to_string())
        );
    }

    #[test]
    fn test_dot_column_in_where_clause() {
        let sql = "SELECT * FROM users u WHERE u.";
        let offset = sql.len();
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("users".to_string())
        );
    }

    #[test]
    fn test_dot_column_multiple_dots() {
        // Test with schema.table.column pattern - should get the last alias before cursor
        let sql = "SELECT u.id, u.name, u. FROM users u";
        let offset = 23; // After "SELECT u.id, u.name, u."
        assert_eq!(
            infer_context(sql, offset),
            SqlContext::DotColumn("users".to_string())
        );
    }
}
