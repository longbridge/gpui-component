/// Property-based tests for SQL Symbol Table
///
/// **Feature: sql-smart-completion**

#[cfg(test)]
mod tests {
    use crate::sql_editor::sql_symbol_table::SymbolTable;
    use crate::sql_editor::sql_tokenizer::SqlTokenizer;
    use proptest::prelude::*;

    // =========================================================================
    // **Feature: sql-smart-completion, Property 3: Alias Resolution Completeness**
    // *For any* SQL query containing FROM or JOIN clauses with table references
    // (with or without aliases, with or without AS keyword), the SymbolTable
    // SHALL correctly map all aliases to their source tables, and tables
    // without aliases SHALL be mapped to themselves.
    // **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**
    // =========================================================================

    /// SQL keywords that should not be used as identifiers in tests
    const SQL_KEYWORDS: &[&str] = &[
        "select",
        "from",
        "where",
        "join",
        "inner",
        "left",
        "right",
        "full",
        "cross",
        "on",
        "as",
        "and",
        "or",
        "not",
        "in",
        "between",
        "like",
        "is",
        "null",
        "order",
        "group",
        "by",
        "having",
        "limit",
        "offset",
        "insert",
        "into",
        "values",
        "update",
        "set",
        "delete",
        "create",
        "alter",
        "drop",
        "table",
        "index",
        "view",
        "union",
        "intersect",
        "except",
        "all",
        "distinct",
        "case",
        "when",
        "then",
        "else",
        "end",
        "with",
        "asc",
        "desc",
        "using",
        "exists",
        "primary",
        "foreign",
        "key",
        "references",
        "unique",
        "check",
        "default",
        "truncate",
        "true",
        "false",
    ];

    /// Generate a valid SQL identifier (table name or alias) that is not a keyword
    fn identifier_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{1,10}".prop_filter("must not be a SQL keyword", |s| {
            !SQL_KEYWORDS.contains(&s.to_lowercase().as_str())
        })
    }

    /// Generate a table reference with optional alias
    #[derive(Debug, Clone)]
    struct TableRef {
        table_name: String,
        alias: Option<String>,
        use_as_keyword: bool,
    }

    impl TableRef {
        fn to_sql(&self) -> String {
            match &self.alias {
                Some(a) if self.use_as_keyword => format!("{} AS {}", self.table_name, a),
                Some(a) => format!("{} {}", self.table_name, a),
                None => self.table_name.clone(),
            }
        }

        fn expected_alias(&self) -> &str {
            self.alias.as_ref().unwrap_or(&self.table_name)
        }

        fn expected_table(&self) -> &str {
            &self.table_name
        }
    }

    fn table_ref_strategy() -> impl Strategy<Value = TableRef> {
        (
            identifier_strategy(),
            prop::option::of(identifier_strategy()),
            prop::bool::ANY,
        )
            .prop_filter("alias must differ from table name", |(table, alias, _)| {
                alias.as_ref().map_or(true, |a| a != table)
            })
            .prop_map(|(table_name, alias, use_as_keyword)| TableRef {
                table_name,
                alias,
                use_as_keyword,
            })
    }

    /// Generate a JOIN type keyword
    fn join_type_strategy() -> impl Strategy<Value = &'static str> {
        prop::sample::select(&[
            "JOIN",
            "INNER JOIN",
            "LEFT JOIN",
            "RIGHT JOIN",
            "CROSS JOIN",
        ])
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 3a: Simple FROM with alias is correctly resolved
        /// `FROM table_name alias` -> maps alias to table_name
        #[test]
        fn prop_from_with_alias_resolved(
            table_name in identifier_strategy(),
            alias in identifier_strategy().prop_filter("alias must differ", |a| a.len() > 0)
        ) {
            prop_assume!(table_name != alias);

            let sql = format!("SELECT * FROM {} {}", table_name, alias);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            prop_assert!(st.is_alias(&alias),
                "Alias '{}' should be registered for SQL: {}", alias, sql);
            prop_assert_eq!(st.resolve(&alias), Some(table_name.as_str()),
                "Alias '{}' should resolve to '{}' for SQL: {}", alias, table_name, sql);
        }

        /// Property 3b: FROM with AS keyword is correctly resolved
        /// `FROM table_name AS alias` -> maps alias to table_name
        #[test]
        fn prop_from_with_as_keyword_resolved(
            table_name in identifier_strategy(),
            alias in identifier_strategy()
        ) {
            prop_assume!(table_name != alias);

            let sql = format!("SELECT * FROM {} AS {}", table_name, alias);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            prop_assert!(st.is_alias(&alias),
                "Alias '{}' should be registered for SQL: {}", alias, sql);
            prop_assert_eq!(st.resolve(&alias), Some(table_name.as_str()),
                "Alias '{}' should resolve to '{}' for SQL: {}", alias, table_name, sql);
        }

        /// Property 3c: FROM without alias maps table to itself
        /// `FROM table_name` -> maps table_name to itself
        #[test]
        fn prop_from_without_alias_self_mapped(table_name in identifier_strategy()) {
            let sql = format!("SELECT * FROM {}", table_name);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            prop_assert!(st.is_alias(&table_name),
                "Table '{}' should be self-mapped for SQL: {}", table_name, sql);
            prop_assert_eq!(st.resolve(&table_name), Some(table_name.as_str()),
                "Table '{}' should resolve to itself for SQL: {}", table_name, sql);
        }

        /// Property 3d: JOIN with alias is correctly resolved
        /// `JOIN table_name alias ON ...` -> maps alias to table_name
        #[test]
        fn prop_join_with_alias_resolved(
            main_table in identifier_strategy(),
            join_table in identifier_strategy(),
            alias in identifier_strategy(),
            join_type in join_type_strategy()
        ) {
            prop_assume!(join_table != alias);
            prop_assume!(main_table != join_table);
            prop_assume!(main_table != alias);

            let sql = format!(
                "SELECT * FROM {} m {} {} {} ON m.id = {}.fk",
                main_table, join_type, join_table, alias, alias
            );
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            prop_assert!(st.is_alias(&alias),
                "Alias '{}' should be registered for SQL: {}", alias, sql);
            prop_assert_eq!(st.resolve(&alias), Some(join_table.as_str()),
                "Alias '{}' should resolve to '{}' for SQL: {}", alias, join_table, sql);
        }

        /// Property 3e: Multiple tables with aliases are all resolved
        #[test]
        fn prop_multiple_tables_all_resolved(
            table1 in identifier_strategy(),
            alias1 in identifier_strategy(),
            table2 in identifier_strategy(),
            alias2 in identifier_strategy()
        ) {
            prop_assume!(table1 != alias1 && table2 != alias2);
            prop_assume!(table1 != table2 && alias1 != alias2);
            prop_assume!(table1 != alias2 && table2 != alias1);

            let sql = format!(
                "SELECT * FROM {} {} JOIN {} {} ON {}.id = {}.fk",
                table1, alias1, table2, alias2, alias1, alias2
            );
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            prop_assert!(st.is_alias(&alias1),
                "Alias '{}' should be registered", alias1);
            prop_assert!(st.is_alias(&alias2),
                "Alias '{}' should be registered", alias2);
            prop_assert_eq!(st.resolve(&alias1), Some(table1.as_str()),
                "Alias '{}' should resolve to '{}'", alias1, table1);
            prop_assert_eq!(st.resolve(&alias2), Some(table2.as_str()),
                "Alias '{}' should resolve to '{}'", alias2, table2);
        }

        /// Property 3f: Alias lookup is case-insensitive
        #[test]
        fn prop_alias_lookup_case_insensitive(
            table_name in identifier_strategy(),
            alias in identifier_strategy()
        ) {
            prop_assume!(table_name != alias);

            let sql = format!("SELECT * FROM {} {}", table_name, alias);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            // Test uppercase lookup
            let upper_alias = alias.to_uppercase();
            prop_assert!(st.is_alias(&upper_alias),
                "Uppercase alias '{}' should be found", upper_alias);
            prop_assert_eq!(st.resolve(&upper_alias), Some(table_name.as_str()),
                "Uppercase alias '{}' should resolve correctly", upper_alias);

            // Test mixed case lookup
            let mixed_alias: String = alias.chars().enumerate()
                .map(|(i, c)| if i % 2 == 0 { c.to_ascii_uppercase() } else { c })
                .collect();
            prop_assert!(st.is_alias(&mixed_alias),
                "Mixed case alias '{}' should be found", mixed_alias);
        }

        /// Property 3g: Comma-separated tables in FROM are all resolved
        #[test]
        fn prop_comma_separated_tables_resolved(
            table1 in identifier_strategy(),
            alias1 in identifier_strategy(),
            table2 in identifier_strategy(),
            alias2 in identifier_strategy()
        ) {
            prop_assume!(table1 != alias1 && table2 != alias2);
            prop_assume!(table1 != table2 && alias1 != alias2);
            prop_assume!(table1 != alias2 && table2 != alias1);

            let sql = format!(
                "SELECT * FROM {} {}, {} {} WHERE {}.id = {}.fk",
                table1, alias1, table2, alias2, alias1, alias2
            );
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            prop_assert!(st.is_alias(&alias1),
                "Alias '{}' should be registered for comma-separated tables", alias1);
            prop_assert!(st.is_alias(&alias2),
                "Alias '{}' should be registered for comma-separated tables", alias2);
            prop_assert_eq!(st.resolve(&alias1), Some(table1.as_str()));
            prop_assert_eq!(st.resolve(&alias2), Some(table2.as_str()));
        }

        /// Property 3h: Generated table references are correctly resolved
        #[test]
        fn prop_generated_table_ref_resolved(table_ref in table_ref_strategy()) {
            let sql = format!("SELECT * FROM {}", table_ref.to_sql());
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();
            let st = SymbolTable::build_from_tokens(&tokens);

            let expected_alias = table_ref.expected_alias();
            let expected_table = table_ref.expected_table();

            prop_assert!(st.is_alias(expected_alias),
                "Expected alias '{}' to be registered for SQL: {}", expected_alias, sql);
            prop_assert_eq!(st.resolve(expected_alias), Some(expected_table),
                "Expected '{}' to resolve to '{}' for SQL: {}", expected_alias, expected_table, sql);
        }
    }
}
