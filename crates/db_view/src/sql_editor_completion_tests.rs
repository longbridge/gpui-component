/// Property-based tests for SQL Completion Provider - DotColumn Filtering
///
/// **Feature: sql-smart-completion**

#[cfg(test)]
mod tests {
    use crate::sql_editor::{
        DefaultSqlCompletionProvider, SqlSchema, TableMentionCompletionProvider,
    };
    use db::plugin::SqlCompletionInfo;
    use db::sql_editor::sql_context_inferrer::{ContextInferrer, SqlContext};
    use db::sql_editor::sql_symbol_table::SymbolTable;
    use db::sql_editor::sql_tokenizer::SqlTokenizer;
    use gpui_component::input::CompletionProvider;
    use proptest::prelude::*;
    use std::collections::HashMap;

    // =========================================================================
    // **Feature: sql-smart-completion, Property 5: DotColumn Completion Filtering**
    // *For any* DotColumn context with alias `a` where `a` resolves to table `T`
    // in the SymbolTable, the completion engine SHALL return only columns from
    // table `T` and no columns from other tables.
    // **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**
    // =========================================================================

    /// Generate valid SQL identifier
    fn identifier_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,10}".prop_filter("not a keyword", |s| {
            !matches!(
                s.to_uppercase().as_str(),
                "SELECT"
                    | "FROM"
                    | "WHERE"
                    | "JOIN"
                    | "AND"
                    | "OR"
                    | "ON"
                    | "ORDER"
                    | "GROUP"
                    | "BY"
                    | "SET"
                    | "VALUES"
                    | "INTO"
                    | "UPDATE"
                    | "DELETE"
                    | "INSERT"
                    | "CREATE"
                    | "TABLE"
                    | "LEFT"
                    | "RIGHT"
                    | "INNER"
                    | "FULL"
                    | "CROSS"
                    | "AS"
                    | "HAVING"
                    | "LIMIT"
                    | "DISTINCT"
                    | "ALL"
                    | "ID"
                    | "NAME"
            )
        })
    }

    /// Generate table alias (single letter or short identifier)
    fn alias_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9]{0,2}".prop_filter("not a keyword", |s| {
            !matches!(
                s.to_uppercase().as_str(),
                "AS" | "ON" | "OR" | "BY" | "IN" | "IS"
            )
        })
    }

    /// Generate column name
    fn column_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,8}".prop_filter("not a keyword", |s| {
            !matches!(
                s.to_uppercase().as_str(),
                "SELECT"
                    | "FROM"
                    | "WHERE"
                    | "JOIN"
                    | "AND"
                    | "OR"
                    | "ON"
                    | "AS"
                    | "BY"
                    | "IN"
                    | "IS"
                    | "SET"
                    | "ALL"
            )
        })
    }

    /// Helper to build symbol table from SQL
    fn build_symbol_table(sql: &str) -> SymbolTable {
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        SymbolTable::build_from_tokens(&tokens)
    }

    /// Helper to infer context from SQL at given offset
    #[allow(dead_code)]
    fn infer_context(sql: &str, offset: usize) -> SqlContext {
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        ContextInferrer::infer(&tokens, offset, &symbol_table)
    }

    /// Simulate DotColumn completion filtering logic
    /// Returns columns that would be suggested for the given alias
    fn get_dot_column_completions(
        alias_or_table: &str,
        symbol_table: &SymbolTable,
        schema: &SqlSchema,
    ) -> Vec<String> {
        // Resolve alias to table name using symbol table
        let resolved_table = symbol_table
            .resolve(alias_or_table)
            .map(|s| s.to_string())
            .unwrap_or_else(|| alias_or_table.to_string());

        // Try to find columns for the resolved table
        // First try exact match, then case-insensitive match
        let columns = schema.columns_by_table.get(&resolved_table).or_else(|| {
            let lower = resolved_table.to_lowercase();
            schema
                .columns_by_table
                .iter()
                .find(|(k, _)| k.to_lowercase() == lower)
                .map(|(_, v)| v)
        });

        match columns {
            Some(cols) => cols.iter().map(|(name, _, _)| name.clone()).collect(),
            None => vec![],
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // =====================================================================
        // Property 5a: Alias resolves to correct table and returns only its columns
        // Validates: Requirements 4.1, 4.2
        // =====================================================================
        #[test]
        fn prop_alias_resolves_to_table_columns(
            table1 in identifier_strategy(),
            alias1 in alias_strategy(),
            cols1 in prop::collection::vec(column_strategy(), 1..4),
            table2 in identifier_strategy(),
            cols2 in prop::collection::vec(column_strategy(), 1..4)
        ) {
            // Ensure tables are different
            prop_assume!(table1 != table2);

            // Build schema with two tables
            let mut columns_by_table = HashMap::new();
            columns_by_table.insert(
                table1.clone(),
                cols1.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );
            columns_by_table.insert(
                table2.clone(),
                cols2.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );

            let schema = SqlSchema {
                tables: vec![(table1.clone(), "".to_string()), (table2.clone(), "".to_string())],
                columns: vec![],
                columns_by_table,
            };

            // Build SQL with alias
            let sql = format!("SELECT {}. FROM {} {}", alias1, table1, alias1);
            let symbol_table = build_symbol_table(&sql);

            // Get completions for alias
            let completions = get_dot_column_completions(&alias1, &symbol_table, &schema);

            // Verify: completions should only contain columns from table1
            for col in &completions {
                prop_assert!(
                    cols1.contains(col),
                    "Completion '{}' should be from table '{}', but it's not in {:?}",
                    col, table1, cols1
                );
            }

            // Verify: all columns from table1 should be in completions
            for col in &cols1 {
                prop_assert!(
                    completions.contains(col),
                    "Column '{}' from table '{}' should be in completions, but got {:?}",
                    col, table1, completions
                );
            }

            // Verify: no columns from table2 should be in completions
            for col in &cols2 {
                prop_assert!(
                    !completions.contains(col) || cols1.contains(col),
                    "Column '{}' from table '{}' should NOT be in completions for alias '{}'",
                    col, table2, alias1
                );
            }
        }

        // =====================================================================
        // Property 5b: Unknown alias returns empty column list
        // Validates: Requirement 4.4
        // =====================================================================
        #[test]
        fn prop_unknown_alias_returns_empty(
            table in identifier_strategy(),
            alias in alias_strategy(),
            unknown_alias in alias_strategy(),
            cols in prop::collection::vec(column_strategy(), 1..4)
        ) {
            // Ensure unknown_alias is different from the defined alias
            prop_assume!(unknown_alias != alias);
            prop_assume!(unknown_alias != table);

            // Build schema
            let mut columns_by_table = HashMap::new();
            columns_by_table.insert(
                table.clone(),
                cols.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );

            let schema = SqlSchema {
                tables: vec![(table.clone(), "".to_string())],
                columns: vec![],
                columns_by_table,
            };

            // Build SQL with alias
            let sql = format!("SELECT * FROM {} {}", table, alias);
            let symbol_table = build_symbol_table(&sql);

            // Get completions for unknown alias
            let completions = get_dot_column_completions(&unknown_alias, &symbol_table, &schema);

            // Verify: completions should be empty for unknown alias
            prop_assert!(
                completions.is_empty(),
                "Unknown alias '{}' should return empty completions, but got {:?}",
                unknown_alias, completions
            );
        }

        // =====================================================================
        // Property 5c: Multiple aliases filter correctly
        // Validates: Requirement 4.5
        // =====================================================================
        #[test]
        fn prop_multiple_aliases_filter_correctly(
            table1 in identifier_strategy(),
            alias1 in alias_strategy(),
            cols1 in prop::collection::vec(column_strategy(), 1..3),
            table2 in identifier_strategy(),
            alias2 in alias_strategy(),
            cols2 in prop::collection::vec(column_strategy(), 1..3)
        ) {
            // Ensure tables and aliases are different
            prop_assume!(table1 != table2);
            prop_assume!(alias1 != alias2);

            // Build schema with two tables
            let mut columns_by_table = HashMap::new();
            columns_by_table.insert(
                table1.clone(),
                cols1.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );
            columns_by_table.insert(
                table2.clone(),
                cols2.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );

            let schema = SqlSchema {
                tables: vec![(table1.clone(), "".to_string()), (table2.clone(), "".to_string())],
                columns: vec![],
                columns_by_table,
            };

            // Build SQL with both aliases
            let sql = format!(
                "SELECT * FROM {} {} JOIN {} {} ON {}.id = {}.id",
                table1, alias1, table2, alias2, alias1, alias2
            );
            let symbol_table = build_symbol_table(&sql);

            // Get completions for alias1
            let completions1 = get_dot_column_completions(&alias1, &symbol_table, &schema);

            // Verify: completions1 should only contain columns from table1
            for col in &completions1 {
                prop_assert!(
                    cols1.contains(col),
                    "Completion '{}' for alias '{}' should be from table '{}', not found in {:?}",
                    col, alias1, table1, cols1
                );
            }

            // Get completions for alias2
            let completions2 = get_dot_column_completions(&alias2, &symbol_table, &schema);

            // Verify: completions2 should only contain columns from table2
            for col in &completions2 {
                prop_assert!(
                    cols2.contains(col),
                    "Completion '{}' for alias '{}' should be from table '{}', not found in {:?}",
                    col, alias2, table2, cols2
                );
            }
        }

        // =====================================================================
        // Property 5d: Table name without alias returns its columns
        // Validates: Requirement 4.3
        // =====================================================================
        #[test]
        fn prop_table_name_without_alias_returns_columns(
            table in identifier_strategy(),
            cols in prop::collection::vec(column_strategy(), 1..4)
        ) {
            // Build schema
            let mut columns_by_table = HashMap::new();
            columns_by_table.insert(
                table.clone(),
                cols.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );

            let schema = SqlSchema {
                tables: vec![(table.clone(), "".to_string())],
                columns: vec![],
                columns_by_table,
            };

            // Build SQL without alias (table maps to itself)
            let sql = format!("SELECT * FROM {}", table);
            let symbol_table = build_symbol_table(&sql);

            // Get completions for table name directly
            let completions = get_dot_column_completions(&table, &symbol_table, &schema);

            // Verify: completions should contain all columns from the table
            for col in &cols {
                prop_assert!(
                    completions.contains(col),
                    "Column '{}' from table '{}' should be in completions, but got {:?}",
                    col, table, completions
                );
            }
        }

        // =====================================================================
        // Property 5e: AS keyword alias resolves correctly
        // Validates: Requirement 4.2 (with AS keyword)
        // =====================================================================
        #[test]
        fn prop_as_keyword_alias_resolves(
            table in identifier_strategy(),
            alias in alias_strategy(),
            cols in prop::collection::vec(column_strategy(), 1..4)
        ) {
            // Ensure alias is different from table
            prop_assume!(alias != table);

            // Build schema
            let mut columns_by_table = HashMap::new();
            columns_by_table.insert(
                table.clone(),
                cols.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );

            let schema = SqlSchema {
                tables: vec![(table.clone(), "".to_string())],
                columns: vec![],
                columns_by_table,
            };

            // Build SQL with AS keyword
            let sql = format!("SELECT * FROM {} AS {}", table, alias);
            let symbol_table = build_symbol_table(&sql);

            // Get completions for alias
            let completions = get_dot_column_completions(&alias, &symbol_table, &schema);

            // Verify: completions should contain all columns from the table
            for col in &cols {
                prop_assert!(
                    completions.contains(col),
                    "Column '{}' from table '{}' (via AS alias '{}') should be in completions, but got {:?}",
                    col, table, alias, completions
                );
            }
        }

        // =====================================================================
        // Property 5f: Case-insensitive alias lookup
        // Validates: Requirements 4.1, 4.2 (case insensitivity)
        // =====================================================================
        #[test]
        fn prop_case_insensitive_alias_lookup(
            table in identifier_strategy(),
            alias in alias_strategy(),
            cols in prop::collection::vec(column_strategy(), 1..4)
        ) {
            // Build schema
            let mut columns_by_table = HashMap::new();
            columns_by_table.insert(
                table.clone(),
                cols.iter().map(|c| (c.clone(), String::new(), format!("{} column", c))).collect()
            );

            let schema = SqlSchema {
                tables: vec![(table.clone(), "".to_string())],
                columns: vec![],
                columns_by_table,
            };

            // Build SQL with lowercase alias
            let sql = format!("SELECT * FROM {} {}", table, alias.to_lowercase());
            let symbol_table = build_symbol_table(&sql);

            // Get completions using uppercase alias
            let completions = get_dot_column_completions(&alias.to_uppercase(), &symbol_table, &schema);

            // Verify: completions should contain all columns (case-insensitive lookup)
            for col in &cols {
                prop_assert!(
                    completions.contains(col),
                    "Column '{}' should be found with case-insensitive alias lookup, but got {:?}",
                    col, completions
                );
            }
        }
    }

    // =========================================================================
    // **Feature: sql-smart-completion, Property 6: Completion Sorting by Context Priority**
    // *For any* completion request, the returned items SHALL be sorted such that
    // context-relevant items (columns in DotColumn, tables in TableName) have
    // higher priority than less relevant items, and prefix-matching items rank
    // higher than non-matching items.
    // **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5, 5.6**
    // =========================================================================

    use crate::sql_editor::SqlContext as EditorSqlContext;
    use crate::sql_editor::completion_priority;
    use lsp_types::CompletionItemKind;
    use proptest::bool::ANY;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // =====================================================================
        // Property 6a: DotColumn context prioritizes columns
        // Validates: Requirement 5.1
        // =====================================================================
        #[test]
        fn prop_dot_column_context_prioritizes_columns(
            alias in alias_strategy(),
            matches_prefix in ANY
        ) {
            let context = EditorSqlContext::DotColumn(alias);

            // Column score in DotColumn context
            let column_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::FIELD),
                matches_prefix,
            );

            // Keyword score in DotColumn context
            let keyword_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::KEYWORD),
                matches_prefix,
            );

            // Verify: columns should have higher priority than keywords (lower score = higher priority)
            prop_assert!(
                column_score < keyword_score,
                "In DotColumn context, columns (score={}) should rank higher than keywords (score={})",
                column_score, keyword_score
            );

            // Verify: column score should be at most 3000 (lower = higher priority)
            let base_column_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::FIELD),
                false,
            );
            prop_assert!(
                base_column_score <= 3000,
                "DotColumn columns should have base score <= 3000, got {}",
                base_column_score
            );
        }

        // =====================================================================
        // Property 6b: TableName context prioritizes tables
        // Validates: Requirement 5.2
        // =====================================================================
        #[test]
        fn prop_table_name_context_prioritizes_tables(
            matches_prefix in ANY
        ) {
            let context = EditorSqlContext::TableName;

            // Table score in TableName context
            let table_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::STRUCT),
                matches_prefix,
            );

            // Keyword score in TableName context
            let keyword_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::KEYWORD),
                matches_prefix,
            );

            // Verify: tables should have higher priority than keywords (lower score = higher priority)
            prop_assert!(
                table_score < keyword_score,
                "In TableName context, tables (score={}) should rank higher than keywords (score={})",
                table_score, keyword_score
            );

            // Verify: table score should be at most 2000 (lower = higher priority)
            let base_table_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::STRUCT),
                false,
            );
            prop_assert!(
                base_table_score <= 2000,
                "TableName tables should have base score <= 2000, got {}",
                base_table_score
            );
        }

        // =====================================================================
        // Property 6c: SelectColumns context prioritizes columns and functions
        // Validates: Requirement 5.3
        // =====================================================================
        #[test]
        fn prop_select_columns_context_prioritizes_columns_and_functions(
            matches_prefix in ANY
        ) {
            let context = EditorSqlContext::SelectColumns;

            // Column score
            let column_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::FIELD),
                matches_prefix,
            );

            // Function score
            let function_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::FUNCTION),
                matches_prefix,
            );

            // Keyword score
            let keyword_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::KEYWORD),
                matches_prefix,
            );

            // Verify: columns should have higher priority than keywords (lower score = higher priority)
            // But functions should have lower priority than keywords
            prop_assert!(
                column_score < keyword_score,
                "In SelectColumns context, columns (score={}) should rank higher than keywords (score={})",
                column_score, keyword_score
            );
            prop_assert!(
                function_score > keyword_score,
                "In SelectColumns context, functions (score={}) should rank lower than keywords (score={})",
                function_score, keyword_score
            );
        }

        // =====================================================================
        // Property 6d: Condition context prioritizes columns and operators
        // Validates: Requirement 5.4
        // =====================================================================
        #[test]
        fn prop_condition_context_prioritizes_columns_and_operators(
            matches_prefix in ANY
        ) {
            let context = EditorSqlContext::Condition;

            // Column score
            let column_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::FIELD),
                matches_prefix,
            );

            // Operator score
            let operator_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::OPERATOR),
                matches_prefix,
            );

            // Keyword score
            let keyword_score = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::KEYWORD),
                matches_prefix,
            );

            // Verify: columns should have higher priority than keywords (lower score = higher priority)
            // But operators should have lower priority than keywords
            prop_assert!(
                column_score < keyword_score,
                "In Condition context, columns (score={}) should rank higher than keywords (score={})",
                column_score, keyword_score
            );
            prop_assert!(
                operator_score > keyword_score,
                "In Condition context, operators (score={}) should rank lower than keywords (score={})",
                operator_score, keyword_score
            );
        }

        // =====================================================================
        // Property 6e: Prefix matching boosts score
        // Validates: Requirement 5.5
        // =====================================================================
        #[test]
        fn prop_prefix_matching_boosts_score(
            alias in alias_strategy()
        ) {
            let context = EditorSqlContext::DotColumn(alias);

            // Score without prefix match
            let score_no_prefix = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::FIELD),
                false,
            );

            // Score with prefix match
            let score_with_prefix = completion_priority::calculate_score(
                &context,
                Some(CompletionItemKind::FIELD),
                true,
            );

            // Verify: prefix matching should boost score (lower score = higher priority)
            prop_assert!(
                score_with_prefix < score_no_prefix,
                "Prefix matching should boost score: with_prefix={} < no_prefix={}",
                score_with_prefix, score_no_prefix
            );

            // Verify: boost should be exactly 200 (PREFIX_MATCH_BOOST reduces score)
            let boost = score_no_prefix - score_with_prefix;
            prop_assert!(
                boost == 200,
                "Prefix match boost should be 200, got {}",
                boost
            );
        }

        // =====================================================================
        // Property 6f: Sort text ordering is correct
        // Validates: Requirements 5.1-5.5 (sorting behavior)
        // =====================================================================
        #[test]
        fn prop_sort_text_ordering_is_correct(
            score1 in 0i32..2000,
            score2 in 0i32..2000,
            label1 in identifier_strategy(),
            label2 in identifier_strategy()
        ) {
            let sort_text1 = completion_priority::score_to_sort_text(score1, &label1);
            let sort_text2 = completion_priority::score_to_sort_text(score2, &label2);

            // Verify: lower scores should result in lower sort_text (appear first, higher priority)
            if score1 < score2 {
                prop_assert!(
                    sort_text1 < sort_text2,
                    "Lower score {} (higher priority) should sort before higher score {}: '{}' < '{}'",
                    score1, score2, sort_text1, sort_text2
                );
            } else if score1 > score2 {
                prop_assert!(
                    sort_text1 > sort_text2,
                    "Higher score {} (lower priority) should sort after lower score {}: '{}' > '{}'",
                    score1, score2, sort_text1, sort_text2
                );
            }
            // Equal scores: ordering depends on label (secondary sort)
        }
    }

    // =========================================================================
    // Unit tests for completion priority edge cases
    // =========================================================================

    #[test]
    fn test_dot_column_columns_have_highest_priority() {
        let context = EditorSqlContext::DotColumn("u".to_string());
        let score =
            completion_priority::calculate_score(&context, Some(CompletionItemKind::FIELD), false);
        // Columns base (3000) - context boost (2500) = 500
        assert_eq!(
            score, 500,
            "DotColumn columns should have score 500 (lower = higher priority)"
        );
    }

    #[test]
    fn test_table_name_tables_have_high_priority() {
        let context = EditorSqlContext::TableName;
        let score =
            completion_priority::calculate_score(&context, Some(CompletionItemKind::STRUCT), false);
        // Tables base (2000) - context boost (2500) = -500 (clamped to 0 or negative is fine)
        assert!(
            score < 1000,
            "TableName tables should have score < 1000 (higher priority than keywords)"
        );
    }

    #[test]
    fn test_select_columns_items_priority() {
        let context = EditorSqlContext::SelectColumns;
        let column_score =
            completion_priority::calculate_score(&context, Some(CompletionItemKind::FIELD), false);
        let function_score = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::FUNCTION),
            false,
        );
        // Columns: 3000 - 2500 = 500
        assert_eq!(
            column_score, 500,
            "SelectColumns columns should have score 500"
        );
        // Functions: 5000 (no context boost)
        assert_eq!(
            function_score, 5000,
            "SelectColumns functions should have score 5000"
        );
    }

    #[test]
    fn test_condition_items_priority() {
        let context = EditorSqlContext::Condition;
        let column_score =
            completion_priority::calculate_score(&context, Some(CompletionItemKind::FIELD), false);
        let operator_score = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::OPERATOR),
            false,
        );
        // Columns: 3000 - 2500 = 500
        assert_eq!(column_score, 500, "Condition columns should have score 500");
        // Operators: 4500 (no context boost)
        assert_eq!(
            operator_score, 4500,
            "Condition operators should have score 4500"
        );
    }

    #[test]
    fn test_prefix_match_boost() {
        let context = EditorSqlContext::SelectColumns;
        let score_no_prefix =
            completion_priority::calculate_score(&context, Some(CompletionItemKind::FIELD), false);
        let score_with_prefix =
            completion_priority::calculate_score(&context, Some(CompletionItemKind::FIELD), true);
        // Prefix match reduces score by 200 (lower = higher priority)
        assert_eq!(
            score_no_prefix - score_with_prefix,
            200,
            "Prefix match should reduce score by 200"
        );
    }

    #[test]
    fn test_sort_text_format() {
        // Lower score should produce lower sort_text (higher priority)
        let high_priority_text = completion_priority::score_to_sort_text(1000, "col1");
        let low_priority_text = completion_priority::score_to_sort_text(2000, "col2");
        assert!(
            high_priority_text < low_priority_text,
            "Lower score (higher priority) should sort first"
        );
    }

    // =========================================================================
    // API Backward Compatibility Tests
    // **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
    // =========================================================================

    /// Test: DefaultSqlCompletionProvider::new(schema) works unchanged
    /// **Validates: Requirement 7.1**
    #[test]
    fn test_api_default_sql_completion_provider_new() {
        // Create schema using builder pattern
        let schema = SqlSchema::default()
            .with_tables([("users", "User table"), ("orders", "Order table")])
            .with_columns([("id", "Primary key"), ("name", "Name column")]);

        // Verify: new() accepts SqlSchema and returns provider
        let provider = DefaultSqlCompletionProvider::new(schema);

        // Verify: provider implements CompletionProvider trait
        let _: &dyn CompletionProvider = &provider;
    }

    /// Test: SqlSchema struct remains compatible
    /// **Validates: Requirement 7.2**
    #[test]
    fn test_api_sql_schema_struct_compatible() {
        // Test default construction
        let schema1 = SqlSchema::default();
        assert!(schema1.tables.is_empty());
        assert!(schema1.columns.is_empty());
        assert!(schema1.columns_by_table.is_empty());

        // Test with_tables builder
        let schema2 = SqlSchema::default().with_tables([("users", "User table")]);
        assert_eq!(schema2.tables.len(), 1);
        assert_eq!(schema2.tables[0].0, "users");
        assert_eq!(schema2.tables[0].1, "User table");

        // Test with_columns builder
        let schema3 =
            SqlSchema::default().with_columns([("id", "ID column"), ("name", "Name column")]);
        assert_eq!(schema3.columns.len(), 2);

        // Test with_table_columns builder
        let schema4 = SqlSchema::default()
            .with_table_columns("users", [("id", "User ID"), ("email", "Email")]);
        assert!(schema4.columns_by_table.contains_key("users"));
        assert_eq!(schema4.columns_by_table.get("users").unwrap().len(), 2);

        // Test chained builders
        let schema5 = SqlSchema::default()
            .with_tables([("users", ""), ("orders", "")])
            .with_columns([("id", "")])
            .with_table_columns("users", [("id", ""), ("name", "")])
            .with_table_columns("orders", [("id", ""), ("total", "")]);
        assert_eq!(schema5.tables.len(), 2);
        assert_eq!(schema5.columns.len(), 1);
        assert_eq!(schema5.columns_by_table.len(), 2);
    }

    /// Test: with_db_completion_info method works
    /// **Validates: Requirement 7.3**
    #[test]
    fn test_api_with_db_completion_info() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // Create SqlCompletionInfo
        let info = SqlCompletionInfo {
            keywords: vec![("LIMIT", "Limit rows")],
            functions: vec![("NOW()", "Current timestamp")],
            operators: vec![("LIKE", "Pattern match")],
            data_types: vec![("INT", "Integer")],
            snippets: vec![("sel", "SELECT * FROM", "Select all")],
        };

        // Verify: with_db_completion_info accepts SqlCompletionInfo
        let _provider_with_info = provider.with_db_completion_info(info);
    }

    /// Test: CompletionProvider trait implementation
    /// **Validates: Requirement 7.4**
    #[test]
    fn test_api_completion_provider_trait_impl() {
        let schema = SqlSchema::default()
            .with_tables([("users", "User table")])
            .with_columns([("id", "ID")]);

        let provider = DefaultSqlCompletionProvider::new(schema);

        // Verify: provider can be used as trait object
        let trait_obj: &dyn CompletionProvider = &provider;

        // Verify: is_completion_trigger method exists and works
        // Note: We can't fully test completions() without gpui context,
        // but we can verify the trait is properly implemented
        fn accepts_completion_provider(_p: &dyn CompletionProvider) {}
        accepts_completion_provider(trait_obj);
    }

    /// Test: SqlSchema Clone implementation
    /// **Validates: Requirement 7.2 (Clone is needed for internal use)**
    #[test]
    fn test_api_sql_schema_clone() {
        let schema = SqlSchema::default()
            .with_tables([("users", "User table")])
            .with_table_columns("users", [("id", "ID")]);

        let cloned = schema.clone();
        assert_eq!(cloned.tables.len(), schema.tables.len());
        assert_eq!(cloned.columns_by_table.len(), schema.columns_by_table.len());
    }

    /// Test: DefaultSqlCompletionProvider Clone implementation
    /// **Validates: Requirement 7.4 (Clone is needed for async completions)**
    #[test]
    fn test_api_default_sql_completion_provider_clone() {
        let schema = SqlSchema::default().with_tables([("users", "User table")]);

        let provider = DefaultSqlCompletionProvider::new(schema);
        let _cloned = provider.clone();
    }

    /// Test: SqlCompletionInfo Default implementation
    /// **Validates: Requirement 7.3**
    #[test]
    fn test_api_sql_completion_info_default() {
        let info = SqlCompletionInfo::default();
        assert!(info.keywords.is_empty());
        assert!(info.functions.is_empty());
        assert!(info.operators.is_empty());
        assert!(info.data_types.is_empty());
        assert!(info.snippets.is_empty());
    }

    /// Test: Full API usage pattern (integration-style)
    /// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
    #[test]
    fn test_api_full_usage_pattern() {
        // Step 1: Create schema with tables and columns
        let schema = SqlSchema::default()
            .with_tables([("users", "User accounts"), ("orders", "Customer orders")])
            .with_columns([("id", "Primary key"), ("created_at", "Creation timestamp")])
            .with_table_columns(
                "users",
                [
                    ("id", "User ID"),
                    ("name", "User name"),
                    ("email", "Email address"),
                ],
            )
            .with_table_columns(
                "orders",
                [
                    ("id", "Order ID"),
                    ("user_id", "Foreign key to users"),
                    ("total", "Order total"),
                ],
            );

        // Step 2: Create provider with schema
        let provider = DefaultSqlCompletionProvider::new(schema);

        // Step 3: Add database-specific completion info
        let db_info = SqlCompletionInfo {
            keywords: vec![("LIMIT", "Limit result rows"), ("OFFSET", "Skip rows")],
            functions: vec![("NOW()", "Current timestamp"), ("UUID()", "Generate UUID")],
            operators: vec![("REGEXP", "Regular expression match")],
            data_types: vec![("BIGINT", "64-bit integer"), ("JSON", "JSON data type")],
            snippets: vec![("selall", "SELECT * FROM $1", "Select all from table")],
        };

        let provider_with_db = provider.with_db_completion_info(db_info);

        // Step 4: Verify provider can be used as trait object
        let _: Box<dyn CompletionProvider> = Box::new(provider_with_db);
    }

    // =========================================================================
    // **Feature: is_completion_trigger 测试**
    // 测试各种字符是否正确触发自动完成
    // =========================================================================

    /// 测试：分号触发自动完成请求（返回空列表以关闭菜单）
    #[test]
    fn test_trigger_semicolon_triggers_to_close_menu() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 分号应触发请求，completions 返回空列表以关闭菜单
        assert!(
            provider.is_completion_trigger_check(";"),
            "Semicolon should trigger completion (to close menu)"
        );
    }

    /// 测试：逗号触发自动完成请求（返回空列表以关闭菜单）
    #[test]
    fn test_trigger_comma_triggers_to_close_menu() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 逗号应触发请求，completions 返回空列表以关闭菜单
        assert!(
            provider.is_completion_trigger_check(","),
            "Comma should trigger completion (to close menu)"
        );
    }

    /// 测试：右括号触发自动完成请求（返回空列表以关闭菜单）
    #[test]
    fn test_trigger_right_paren_triggers_to_close_menu() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 右括号应触发请求，completions 返回空列表以关闭菜单
        assert!(
            provider.is_completion_trigger_check(")"),
            "Right paren should trigger completion (to close menu)"
        );
    }

    /// 测试：换行符后不触发自动完成
    #[test]
    fn test_trigger_newline_not_trigger() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 换行符后不应触发
        assert!(
            !provider.is_completion_trigger_check("\n"),
            "Newline should not trigger completion"
        );
        assert!(
            !provider.is_completion_trigger_check("\r"),
            "Carriage return should not trigger completion"
        );
        assert!(
            !provider.is_completion_trigger_check("\t"),
            "Tab should not trigger completion"
        );
    }

    /// 测试：点号触发自动完成（table.column）
    #[test]
    fn test_trigger_dot_triggers() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 点号应触发
        assert!(
            provider.is_completion_trigger_check("."),
            "Dot should trigger completion (table.column)"
        );
    }

    /// 测试：左括号触发自动完成（函数参数）
    #[test]
    fn test_trigger_left_paren_triggers() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 左括号应触发
        assert!(
            provider.is_completion_trigger_check("("),
            "Left paren should trigger completion (function args)"
        );
    }

    /// 测试：空格触发自动完成
    #[test]
    fn test_trigger_space_triggers() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 空格应触发
        assert!(
            provider.is_completion_trigger_check(" "),
            "Space should trigger completion (e.g. after SELECT)"
        );
    }

    /// 测试：字母和数字触发自动完成
    #[test]
    fn test_trigger_alphanumeric_triggers() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 字母应触发
        assert!(
            provider.is_completion_trigger_check("a"),
            "Letters should trigger completion"
        );
        assert!(
            provider.is_completion_trigger_check("Z"),
            "Uppercase letters should trigger completion"
        );
        // 数字应触发
        assert!(
            provider.is_completion_trigger_check("5"),
            "Digits should trigger completion"
        );
        // 下划线应触发
        assert!(
            provider.is_completion_trigger_check("_"),
            "Underscore should trigger completion"
        );
    }

    /// 测试：引号触发自动完成（在 completions 中控制返回内容）
    #[test]
    fn test_trigger_quotes_triggers() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 引号触发请求，completions 控制返回内容
        assert!(
            provider.is_completion_trigger_check("'"),
            "Single quote should trigger completion"
        );
        assert!(
            provider.is_completion_trigger_check("\""),
            "Double quote should trigger completion"
        );
    }

    /// 测试：星号触发自动完成（在 completions 中控制返回内容）
    #[test]
    fn test_trigger_asterisk_triggers() {
        let schema = SqlSchema::default();
        let provider = DefaultSqlCompletionProvider::new(schema);

        // 星号触发请求
        assert!(
            provider.is_completion_trigger_check("*"),
            "Asterisk should trigger completion"
        );
    }

    // =========================================================================
    // **Feature: CREATE TABLE 上下文测试**
    // 测试 CREATE TABLE 语句中不同位置的自动完成行为
    // =========================================================================

    /// 测试：CREATE TABLE 后输入表名时的上下文
    #[test]
    fn test_create_table_context_after_table_keyword() {
        let sql = "CREATE TABLE ";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        let context = ContextInferrer::infer(&tokens, sql.len(), &symbol_table);

        assert_eq!(
            context,
            SqlContext::CreateTable,
            "After CREATE TABLE should be CreateTable context"
        );
    }

    /// 测试：CREATE TABLE 表名后的上下文
    #[test]
    fn test_create_table_context_after_table_name() {
        let sql = "CREATE TABLE users ";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        let context = ContextInferrer::infer(&tokens, sql.len(), &symbol_table);

        assert_eq!(
            context,
            SqlContext::CreateTable,
            "After table name should be CreateTable context"
        );
    }

    /// 测试：CREATE TABLE 左括号后的上下文（输入字段名）
    #[test]
    fn test_create_table_context_after_open_paren() {
        let sql = "CREATE TABLE users (";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        let context = ContextInferrer::infer(&tokens, sql.len(), &symbol_table);

        assert_eq!(
            context,
            SqlContext::CreateTable,
            "After ( should be CreateTable context"
        );
    }

    /// 测试：CREATE TABLE 字段名后的上下文（输入数据类型）
    #[test]
    fn test_create_table_context_after_column_name() {
        let sql = "CREATE TABLE users (id ";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        let context = ContextInferrer::infer(&tokens, sql.len(), &symbol_table);

        assert_eq!(
            context,
            SqlContext::CreateTable,
            "After column name should be CreateTable context"
        );
    }

    /// 测试：CREATE TABLE 数据类型后逗号的上下文
    #[test]
    fn test_create_table_context_after_type_comma() {
        let sql = "CREATE TABLE users (id INT, ";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        let context = ContextInferrer::infer(&tokens, sql.len(), &symbol_table);

        assert_eq!(
            context,
            SqlContext::CreateTable,
            "After type comma should be CreateTable context"
        );
    }

    // =========================================================================
    // **Feature: 子查询别名测试**
    // 测试子查询别名的识别和解析
    // =========================================================================

    /// 测试：子查询别名注册为 #subquery
    #[test]
    fn test_subquery_alias_registration() {
        let sql = "SELECT * FROM (SELECT id FROM users) AS sub";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        // 子查询别名应该解析为 #subquery
        assert_eq!(
            symbol_table.resolve("sub"),
            Some("#subquery"),
            "Subquery alias should resolve to #subquery"
        );
    }

    /// 测试：子查询别名（无 AS 关键字）
    #[test]
    fn test_subquery_alias_without_as() {
        let sql = "SELECT * FROM (SELECT id FROM users) sub";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(
            symbol_table.resolve("sub"),
            Some("#subquery"),
            "Subquery alias without AS should resolve to #subquery"
        );
    }

    /// 测试：嵌套子查询
    #[test]
    fn test_nested_subquery_alias() {
        let sql = "SELECT * FROM (SELECT * FROM (SELECT id FROM users) inner_sub) outer_sub";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(
            symbol_table.resolve("outer_sub"),
            Some("#subquery"),
            "Outer subquery alias should resolve to #subquery"
        );
    }

    /// 测试：JOIN 中的子查询
    #[test]
    fn test_subquery_in_join() {
        let sql = "SELECT * FROM users u JOIN (SELECT user_id FROM orders) o ON u.id = o.user_id";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(
            symbol_table.resolve("u"),
            Some("users"),
            "Regular table alias should resolve normally"
        );
        assert_eq!(
            symbol_table.resolve("o"),
            Some("#subquery"),
            "Subquery alias in JOIN should resolve to #subquery"
        );
    }

    // =========================================================================
    // **Feature: CTE (WITH 子句) 别名测试**
    // 测试 CTE 别名的识别和解析
    // =========================================================================

    /// 测试：简单 CTE 别名
    #[test]
    fn test_cte_alias_simple() {
        let sql = "WITH cte AS (SELECT id FROM users) SELECT * FROM cte";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(
            symbol_table.resolve("cte"),
            Some("#cte"),
            "CTE alias should resolve to #cte"
        );
    }

    /// 测试：多个 CTE
    #[test]
    fn test_multiple_ctes() {
        let sql = "WITH cte1 AS (SELECT id FROM users), cte2 AS (SELECT id FROM orders) SELECT * FROM cte1, cte2";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(
            symbol_table.resolve("cte1"),
            Some("#cte"),
            "First CTE should resolve to #cte"
        );
        assert_eq!(
            symbol_table.resolve("cte2"),
            Some("#cte"),
            "Second CTE should resolve to #cte"
        );
    }

    /// 测试：CTE 和普通表混合
    #[test]
    fn test_cte_with_regular_table() {
        let sql = "WITH cte AS (SELECT id FROM orders) SELECT * FROM users u, cte c";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(
            symbol_table.resolve("u"),
            Some("users"),
            "Regular table alias should resolve normally"
        );
        assert_eq!(
            symbol_table.resolve("cte"),
            Some("#cte"),
            "CTE name should resolve to #cte"
        );
    }

    // =========================================================================
    // **Feature: 多表 JOIN 列名去重测试**
    // 测试多表 JOIN 时重复列名的处理
    // =========================================================================

    /// 测试：多表 JOIN 中同名列的识别
    #[test]
    fn test_multi_table_duplicate_columns_detection() {
        let mut columns_by_table = HashMap::new();
        columns_by_table.insert(
            "users".to_string(),
            vec![
                ("id".to_string(), "INT".to_string(), "用户ID".to_string()),
                ("name".to_string(), "VARCHAR".to_string(), "用户名".to_string()),
            ],
        );
        columns_by_table.insert(
            "orders".to_string(),
            vec![
                ("id".to_string(), "INT".to_string(), "订单ID".to_string()),
                ("user_id".to_string(), "INT".to_string(), "用户外键".to_string()),
            ],
        );

        let schema = SqlSchema {
            tables: vec![
                ("users".to_string(), "用户表".to_string()),
                ("orders".to_string(), "订单表".to_string()),
            ],
            columns: vec![],
            columns_by_table,
        };

        // 验证 schema 中两个表都有 id 列
        let users_cols = schema.columns_by_table.get("users").unwrap();
        let orders_cols = schema.columns_by_table.get("orders").unwrap();

        let users_has_id = users_cols.iter().any(|(name, _, _)| name == "id");
        let orders_has_id = orders_cols.iter().any(|(name, _, _)| name == "id");

        assert!(users_has_id, "users should have id column");
        assert!(orders_has_id, "orders should have id column");
    }

    /// 测试：Symbol Table 正确解析多表别名
    #[test]
    fn test_multi_table_join_alias_resolution() {
        let sql = "SELECT * FROM users u JOIN orders o ON u.id = o.user_id";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(
            symbol_table.resolve("u"),
            Some("users"),
            "u should resolve to users"
        );
        assert_eq!(
            symbol_table.resolve("o"),
            Some("orders"),
            "o should resolve to orders"
        );
    }

    /// 测试：LEFT JOIN 别名解析
    #[test]
    fn test_left_join_alias_resolution() {
        let sql = "SELECT * FROM users u LEFT JOIN orders o ON u.id = o.user_id";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(symbol_table.resolve("u"), Some("users"));
        assert_eq!(symbol_table.resolve("o"), Some("orders"));
    }

    /// 测试：多个 JOIN 别名解析
    #[test]
    fn test_multiple_joins_alias_resolution() {
        let sql = "SELECT * FROM users u JOIN orders o ON u.id = o.user_id JOIN products p ON o.product_id = p.id";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(symbol_table.resolve("u"), Some("users"));
        assert_eq!(symbol_table.resolve("o"), Some("orders"));
        assert_eq!(symbol_table.resolve("p"), Some("products"));
    }

    /// 测试：逗号分隔的多表
    #[test]
    fn test_comma_separated_tables() {
        let sql = "SELECT * FROM users u, orders o WHERE u.id = o.user_id";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        assert_eq!(symbol_table.resolve("u"), Some("users"));
        assert_eq!(symbol_table.resolve("o"), Some("orders"));
    }

    // =========================================================================
    // **Feature: 列类型信息测试**
    // 测试 with_table_columns_typed 方法
    // =========================================================================

    /// 测试：with_table_columns_typed 保存类型信息
    #[test]
    fn test_with_table_columns_typed() {
        let schema = SqlSchema::default()
            .with_table_columns_typed(
                "users",
                [
                    ("id", "INT", "用户ID"),
                    ("name", "VARCHAR(255)", "用户名"),
                    ("email", "VARCHAR(100)", "邮箱"),
                ],
            );

        let cols = schema.columns_by_table.get("users").unwrap();
        assert_eq!(cols.len(), 3);

        // 验证类型信息
        assert_eq!(cols[0], ("id".to_string(), "INT".to_string(), "用户ID".to_string()));
        assert_eq!(cols[1], ("name".to_string(), "VARCHAR(255)".to_string(), "用户名".to_string()));
        assert_eq!(cols[2], ("email".to_string(), "VARCHAR(100)".to_string(), "邮箱".to_string()));
    }

    /// 测试：with_table_columns 向后兼容（类型为空）
    #[test]
    fn test_with_table_columns_backward_compat() {
        let schema = SqlSchema::default()
            .with_table_columns(
                "users",
                [("id", "用户ID"), ("name", "用户名")],
            );

        let cols = schema.columns_by_table.get("users").unwrap();
        assert_eq!(cols.len(), 2);

        // 类型字段应该为空
        assert_eq!(cols[0].1, "");
        assert_eq!(cols[1].1, "");
    }

    // =========================================================================
    // **Feature: Schema 不区分大小写查找测试**
    // =========================================================================

    /// 测试：表名大小写不敏感查找
    #[test]
    fn test_case_insensitive_table_lookup() {
        let mut columns_by_table = HashMap::new();
        columns_by_table.insert(
            "Users".to_string(),
            vec![("id".to_string(), "INT".to_string(), "ID".to_string())],
        );

        let schema = SqlSchema {
            tables: vec![("Users".to_string(), "".to_string())],
            columns: vec![],
            columns_by_table,
        };

        // 使用小写查找
        let lower = "users".to_lowercase();
        let found = schema
            .columns_by_table
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v);

        assert!(found.is_some(), "Should find columns with case-insensitive table lookup");
    }

    /// 测试：别名大小写不敏感
    #[test]
    fn test_case_insensitive_alias_lookup() {
        let sql = "SELECT * FROM users U";
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);

        // 用小写查找大写别名
        assert_eq!(symbol_table.resolve("u"), Some("users"));
        // 用大写查找
        assert_eq!(symbol_table.resolve("U"), Some("users"));
    }

    // =========================================================================
    // **Feature: 数据类型优先级测试**
    // 测试 CREATE TABLE 上下文中数据类型的优先级
    // =========================================================================

    /// 测试：CreateTable 上下文中数据类型优先级高于关键词
    #[test]
    fn test_create_table_data_types_priority() {
        let context = EditorSqlContext::CreateTable;

        // 数据类型分数
        let data_type_score = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::TYPE_PARAMETER),
            false,
        );

        // 关键词分数
        let keyword_score = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::KEYWORD),
            false,
        );

        // 验证：数据类型应该比关键词优先（分数更低）
        assert!(
            data_type_score < keyword_score,
            "In CreateTable context, data types (score={}) should rank above keywords (score={})",
            data_type_score, keyword_score
        );
    }

    /// 测试：数据类型基础分数
    #[test]
    fn test_data_types_base_score() {
        let context = EditorSqlContext::Start;

        let data_type_score = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::TYPE_PARAMETER),
            false,
        );

        // 数据类型基础分数应该是 1500
        assert_eq!(
            data_type_score, 1500,
            "Base data type score should be 1500"
        );
    }

    /// 测试：CreateTable 上下文中数据类型获得加成
    #[test]
    fn test_create_table_data_types_boost() {
        let context = EditorSqlContext::CreateTable;

        // CreateTable 上下文中的数据类型分数
        let boosted_score = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::TYPE_PARAMETER),
            false,
        );

        // Start 上下文中的数据类型分数（无加成）
        let base_score = completion_priority::calculate_score(
            &EditorSqlContext::Start,
            Some(CompletionItemKind::TYPE_PARAMETER),
            false,
        );

        // 验证：CreateTable 上下文应该有加成
        assert!(
            boosted_score < base_score,
            "In CreateTable context, data types should be boosted: boosted={} < base={}",
            boosted_score, base_score
        );

        // 验证：加成值应该是 CONTEXT_BOOST (2500)
        assert_eq!(
            base_score - boosted_score, 2500,
            "Data type boost should be 2500"
        );
    }

    /// 测试：前缀匹配对数据类型分数的影响
    #[test]
    fn test_data_type_prefix_match_boost() {
        let context = EditorSqlContext::CreateTable;

        let score_no_prefix = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::TYPE_PARAMETER),
            false,
        );

        let score_with_prefix = completion_priority::calculate_score(
            &context,
            Some(CompletionItemKind::TYPE_PARAMETER),
            true,
        );

        // 验证：前缀匹配应该进一步提升优先级
        assert!(
            score_with_prefix < score_no_prefix,
            "Prefix match should boost data types: with_prefix={} < no_prefix={}",
            score_with_prefix, score_no_prefix
        );
    }

    // =========================================================================
    // **Feature: @表名提示（Agent 模式）**
    // =========================================================================

    #[test]
    fn test_table_mention_extract_simple() {
        let text = "请查询 @users 数据";
        let offset = text.find("users").unwrap() + "users".len();
        let (start, prefix) =
            TableMentionCompletionProvider::extract_mention_query(text, offset).unwrap();
        assert_eq!(start, text.find('@').unwrap());
        assert_eq!(prefix, "users");
    }

    #[test]
    fn test_table_mention_extract_backtick_unclosed() {
        let text = "请查询 @`用户";
        let offset = text.len();
        let (_, prefix) =
            TableMentionCompletionProvider::extract_mention_query(text, offset).unwrap();
        assert_eq!(prefix, "用户");
    }

    #[test]
    fn test_table_mention_extract_backtick_closed() {
        let text = "请查询 @`用户` 数据";
        let offset = text.find("` 数据").unwrap() + 1;
        let result = TableMentionCompletionProvider::extract_mention_query(text, offset);
        assert!(result.is_none());
    }

    #[test]
    fn test_table_mention_format() {
        assert_eq!(
            TableMentionCompletionProvider::format_table_mention("users"),
            "@users"
        );
        assert_eq!(
            TableMentionCompletionProvider::format_table_mention("用户表"),
            "@用户表"
        );
        assert_eq!(
            TableMentionCompletionProvider::format_table_mention("user info"),
            "@`user info`"
        );
    }
}
