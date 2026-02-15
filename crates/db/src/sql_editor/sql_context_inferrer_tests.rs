/// Property-based tests for SQL Context Inferrer
///
/// **Feature: sql-smart-completion**

#[cfg(test)]
mod tests {
    use crate::sql_editor::sql_context_inferrer::{ContextInferrer, SqlContext};
    use crate::sql_editor::sql_symbol_table::SymbolTable;
    use crate::sql_editor::sql_tokenizer::SqlTokenizer;
    use proptest::prelude::*;
    // =========================================================================
    // **Feature: sql-smart-completion, Property 4: Context Inference Accuracy**
    // *For any* cursor position in a SQL statement, the Context Inferrer SHALL
    // return the correct SqlContext based on the preceding tokens:
    // - SelectColumns after SELECT
    // - TableName after FROM/JOIN
    // - DotColumn after `alias.`
    // - Condition after WHERE/AND/OR
    // - OrderBy after ORDER BY/GROUP BY
    // - FunctionArgs inside parentheses
    // - Start at statement beginning
    // **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7**
    // =========================================================================

    /// Helper to infer context from SQL at given offset
    fn infer_context(sql: &str, offset: usize) -> SqlContext {
        let mut tokenizer = SqlTokenizer::new(sql);
        let tokens = tokenizer.tokenize();
        let symbol_table = SymbolTable::build_from_tokens(&tokens);
        ContextInferrer::infer(&tokens, offset, &symbol_table)
    }

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
                    | "IS"
                    | "IN"
                    | "NOT"
                    | "NULL"
                    | "LIKE"
                    | "BETWEEN"
                    | "EXISTS"
                    | "CASE"
                    | "WHEN"
                    | "THEN"
                    | "ELSE"
                    | "END"
                    | "WITH"
                    | "ASC"
                    | "DESC"
                    | "UNION"
                    | "INTERSECT"
                    | "EXCEPT"
                    | "PRIMARY"
                    | "FOREIGN"
                    | "KEY"
                    | "REFERENCES"
                    | "UNIQUE"
                    | "CHECK"
                    | "DEFAULT"
                    | "TRUNCATE"
                    | "USING"
                    | "INDEX"
                    | "VIEW"
                    | "ALTER"
                    | "DROP"
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

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // =====================================================================
        // Property 4a: After SELECT keyword, context is SelectColumns
        // Validates: Requirement 3.1
        // =====================================================================
        #[test]
        fn prop_select_context(
            columns in prop::collection::vec(identifier_strategy(), 0..3)
        ) {
            let cols_str = if columns.is_empty() {
                String::new()
            } else {
                columns.join(", ") + ", "
            };
            let sql = format!("SELECT {}", cols_str);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::SelectColumns,
                "After 'SELECT {}' at offset {}, expected SelectColumns, got {:?}",
                cols_str, offset, context
            );
        }

        // =====================================================================
        // Property 4b: After FROM keyword, context is TableName
        // Validates: Requirement 3.2
        // =====================================================================
        #[test]
        fn prop_from_context(
            table in identifier_strategy()
        ) {
            let sql = "SELECT * FROM ".to_string();
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::TableName,
                "After 'SELECT * FROM ' at offset {}, expected TableName, got {:?}",
                offset, context
            );

            // Also test after table name with comma (multiple tables)
            let sql2 = format!("SELECT * FROM {}, ", table);
            let offset2 = sql2.len();

            let context2 = infer_context(&sql2, offset2);
            prop_assert!(
                context2 == SqlContext::TableName,
                "After 'SELECT * FROM {}, ' at offset {}, expected TableName, got {:?}",
                table, offset2, context2
            );
        }

        // =====================================================================
        // Property 4c: After JOIN keyword, context is TableName
        // Validates: Requirement 3.2
        // =====================================================================
        #[test]
        fn prop_join_context(
            table in identifier_strategy(),
            alias in alias_strategy(),
            join_type in prop::sample::select(&["JOIN", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN"])
        ) {
            let sql = format!("SELECT * FROM {} {} {} ", table, alias, join_type);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::TableName,
                "After '{}' at offset {}, expected TableName, got {:?}",
                sql.trim(), offset, context
            );
        }

        // =====================================================================
        // Property 4d: After alias followed by dot, context is DotColumn
        // Validates: Requirement 3.3
        // =====================================================================
        #[test]
        fn prop_dot_column_context(
            table in identifier_strategy(),
            alias in alias_strategy()
        ) {
            // SELECT alias. FROM table alias
            let sql = format!("SELECT {}. FROM {} {}", alias, table, alias);
            let offset = 7 + alias.len() + 1; // "SELECT " + alias + "."

            let context = infer_context(&sql, offset);

            match &context {
                SqlContext::DotColumn(resolved) => {
                    prop_assert!(
                        resolved == &table,
                        "DotColumn should resolve alias '{}' to table '{}', got '{}'",
                        alias, table, resolved
                    );
                }
                _ => prop_assert!(false,
                    "After 'SELECT {}.', expected DotColumn, got {:?}", alias, context),
            }
        }

        // =====================================================================
        // Property 4e: After WHERE keyword, context is Condition
        // Validates: Requirement 3.4
        // =====================================================================
        #[test]
        fn prop_where_context(
            table in identifier_strategy()
        ) {
            let sql = format!("SELECT * FROM {} WHERE ", table);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::Condition,
                "After 'WHERE ' at offset {}, expected Condition, got {:?}",
                offset, context
            );
        }

        // =====================================================================
        // Property 4f: After AND/OR keywords, context is Condition
        // Validates: Requirement 3.4
        // =====================================================================
        #[test]
        fn prop_and_or_context(
            table in identifier_strategy(),
            col in identifier_strategy(),
            logical_op in prop::sample::select(&["AND", "OR"])
        ) {
            let sql = format!("SELECT * FROM {} WHERE {} = 1 {} ", table, col, logical_op);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::Condition,
                "After '{} ' at offset {}, expected Condition, got {:?}",
                logical_op, offset, context
            );
        }

        // =====================================================================
        // Property 4g: After ORDER BY, context is OrderBy
        // Validates: Requirement 3.5
        // =====================================================================
        #[test]
        fn prop_order_by_context(
            table in identifier_strategy()
        ) {
            let sql = format!("SELECT * FROM {} ORDER BY ", table);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::OrderBy,
                "After 'ORDER BY ' at offset {}, expected OrderBy, got {:?}",
                offset, context
            );
        }

        // =====================================================================
        // Property 4h: After GROUP BY, context is OrderBy
        // Validates: Requirement 3.5
        // =====================================================================
        #[test]
        fn prop_group_by_context(
            table in identifier_strategy()
        ) {
            let sql = format!("SELECT * FROM {} GROUP BY ", table);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::OrderBy,
                "After 'GROUP BY ' at offset {}, expected OrderBy, got {:?}",
                offset, context
            );
        }

        // =====================================================================
        // Property 4i: Inside function parentheses, context is FunctionArgs
        // Validates: Requirement 3.6
        // =====================================================================
        #[test]
        fn prop_function_args_context(
            func_name in prop::sample::select(&["COUNT", "SUM", "AVG", "MAX", "MIN", "COALESCE"])
        ) {
            let sql = format!("SELECT {}(", func_name);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::FunctionArgs,
                "After '{}(' at offset {}, expected FunctionArgs, got {:?}",
                func_name, offset, context
            );
        }

        // =====================================================================
        // Property 4j: At statement start, context is Start
        // Validates: Requirement 3.7
        // =====================================================================
        #[test]
        fn prop_start_context(
            whitespace in "[ \t\n]{0,5}"
        ) {
            let sql = whitespace.clone();
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::Start,
                "At statement start with whitespace '{}', expected Start, got {:?}",
                sql.escape_debug(), context
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // =====================================================================
        // Property 4k: DotColumn context with multiple aliases resolves correctly
        // Validates: Requirements 3.3, 4.1, 4.2, 4.3
        // =====================================================================
        #[test]
        fn prop_dot_column_multiple_aliases(
            table1 in identifier_strategy(),
            alias1 in alias_strategy(),
            table2 in identifier_strategy(),
            alias2 in alias_strategy()
        ) {
            // Ensure aliases are different
            prop_assume!(alias1 != alias2);

            let sql = format!(
                "SELECT {}. FROM {} {} JOIN {} {} ON {}.id = {}.id",
                alias1, table1, alias1, table2, alias2, alias1, alias2
            );
            let offset = 7 + alias1.len() + 1; // "SELECT " + alias1 + "."

            let context = infer_context(&sql, offset);

            match &context {
                SqlContext::DotColumn(resolved) => {
                    prop_assert!(
                        resolved == &table1,
                        "DotColumn should resolve alias '{}' to table '{}', got '{}'",
                        alias1, table1, resolved
                    );
                }
                _ => prop_assert!(false,
                    "After 'SELECT {}.', expected DotColumn, got {:?}", alias1, context),
            }
        }

        // =====================================================================
        // Property 4l: Context transitions correctly through SQL statement
        // Validates: Requirements 3.1-3.7
        // =====================================================================
        #[test]
        fn prop_context_transitions(
            table in identifier_strategy(),
            alias in alias_strategy(),
            col in identifier_strategy()
        ) {
            let sql = format!(
                "SELECT {}.{} FROM {} {} WHERE {}.{} = 1 ORDER BY {}.{}",
                alias, col, table, alias, alias, col, alias, col
            );

            // After FROM
            let from_pos = sql.find("FROM ").unwrap() + 5;
            let ctx2 = infer_context(&sql, from_pos);
            prop_assert!(
                ctx2 == SqlContext::TableName,
                "After FROM, expected TableName, got {:?}", ctx2
            );

            // After WHERE
            let where_pos = sql.find("WHERE ").unwrap() + 6;
            let ctx3 = infer_context(&sql, where_pos);
            prop_assert!(
                ctx3 == SqlContext::Condition,
                "After WHERE, expected Condition, got {:?}", ctx3
            );

            // After ORDER BY
            let order_pos = sql.find("ORDER BY ").unwrap() + 9;
            let ctx4 = infer_context(&sql, order_pos);
            prop_assert!(
                ctx4 == SqlContext::OrderBy,
                "After ORDER BY, expected OrderBy, got {:?}", ctx4
            );
        }

        // =====================================================================
        // Property 4m: SET clause context in UPDATE statements
        // Validates: Requirement 3.4 (SetClause)
        // =====================================================================
        #[test]
        fn prop_set_clause_context(
            table in identifier_strategy()
        ) {
            let sql = format!("UPDATE {} SET ", table);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::SetClause,
                "After 'UPDATE {} SET ' at offset {}, expected SetClause, got {:?}",
                table, offset, context
            );
        }

        // =====================================================================
        // Property 4n: VALUES context in INSERT statements
        // Validates: Requirement 3.4 (Values)
        // =====================================================================
        #[test]
        fn prop_values_context(
            table in identifier_strategy()
        ) {
            let sql = format!("INSERT INTO {} VALUES ", table);
            let offset = sql.len();

            let context = infer_context(&sql, offset);
            prop_assert!(
                context == SqlContext::Values,
                "After 'INSERT INTO {} VALUES ' at offset {}, expected Values, got {:?}",
                table, offset, context
            );
        }

        // =====================================================================
        // Property 4o: CREATE TABLE context
        // Validates: Requirement 3.4 (CreateTable)
        // =====================================================================
        #[test]
        fn prop_create_table_context(_dummy in Just(())) {
            let sql = "CREATE TABLE ";
            let offset = sql.len();

            let context = infer_context(sql, offset);
            prop_assert!(
                context == SqlContext::CreateTable,
                "After 'CREATE TABLE ' at offset {}, expected CreateTable, got {:?}",
                offset, context
            );
        }
    }
}
