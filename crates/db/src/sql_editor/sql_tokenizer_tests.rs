/// Property-based tests for SQL Tokenizer
///
/// **Feature: sql-smart-completion**

#[cfg(test)]
mod tests {
    use crate::sql_editor::sql_tokenizer::{SqlTokenKind, SqlTokenizer};
    use proptest::prelude::*;

    // =========================================================================
    // **Feature: sql-smart-completion, Property 1: Tokenizer Special Region Handling**
    // *For any* SQL text containing strings (single-quoted) or comments
    // (line `--` or block `/* */`), tokenizing the text SHALL produce exactly
    // one token for each special region, and the token text SHALL match the
    // original region exactly.
    // **Validates: Requirements 1.1, 1.2, 1.3, 1.4**
    // =========================================================================

    /// Generate a valid single-quoted string (no internal single quotes for simplicity)
    fn string_content_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 _.,!?@#$%^&*()-+=]{0,50}".prop_map(|s| s)
    }

    /// Generate a valid double-quoted identifier content
    fn quoted_ident_content_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z_][a-zA-Z0-9_]{0,30}".prop_map(|s| s)
    }

    /// Generate line comment content (no newlines)
    fn line_comment_content_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 _.,!?@#$%^&*()-+=]{0,50}".prop_map(|s| s)
    }

    /// Generate block comment content (no */ sequence)
    fn block_comment_content_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 _.,!?@#$%^&()-+=\n]{0,50}".prop_map(|s| s.replace("*/", ""))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 1a: Single-quoted strings are tokenized as single STRING token
        #[test]
        fn prop_single_quoted_string_is_single_token(content in string_content_strategy()) {
            let sql = format!("'{}'", content);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            // Filter out EOF
            let non_eof: Vec<_> = tokens.into_iter().filter(|t| !matches!(t.kind, SqlTokenKind::Eof)).collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token, got {:?}", non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::String), "Expected String token, got {:?}", non_eof[0].kind);
            prop_assert_eq!(&non_eof[0].text, &sql, "Token text should match original");
        }

        /// Property 1b: Double-quoted identifiers are tokenized as single QUOTED_IDENT token
        #[test]
        fn prop_double_quoted_ident_is_single_token(content in quoted_ident_content_strategy()) {
            let sql = format!("\"{}\"", content);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter().filter(|t| !matches!(t.kind, SqlTokenKind::Eof)).collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token, got {:?}", non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::QuotedIdent), "Expected QuotedIdent token, got {:?}", non_eof[0].kind);
            prop_assert_eq!(&non_eof[0].text, &sql, "Token text should match original");
        }

        /// Property 1c: Line comments are tokenized as single LINE_COMMENT token
        #[test]
        fn prop_line_comment_is_single_token(content in line_comment_content_strategy()) {
            let sql = format!("-- {}", content);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter().filter(|t| !matches!(t.kind, SqlTokenKind::Eof)).collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token, got {:?}", non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::LineComment), "Expected LineComment token, got {:?}", non_eof[0].kind);
            prop_assert_eq!(&non_eof[0].text, &sql, "Token text should match original");
        }

        /// Property 1d: Block comments are tokenized as single BLOCK_COMMENT token
        #[test]
        fn prop_block_comment_is_single_token(content in block_comment_content_strategy()) {
            let sql = format!("/* {} */", content);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter().filter(|t| !matches!(t.kind, SqlTokenKind::Eof)).collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token, got {:?}", non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::BlockComment), "Expected BlockComment token, got {:?}", non_eof[0].kind);
            prop_assert_eq!(&non_eof[0].text, &sql, "Token text should match original");
        }

        /// Property 1e: Escaped single quotes ('') within strings are handled correctly
        #[test]
        fn prop_escaped_single_quote_in_string(
            prefix in "[a-zA-Z0-9 ]{0,10}",
            suffix in "[a-zA-Z0-9 ]{0,10}"
        ) {
            let sql = format!("'{}''{}'" , prefix, suffix);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter().filter(|t| !matches!(t.kind, SqlTokenKind::Eof)).collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token for escaped quote string, got {:?}", non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::String), "Expected String token, got {:?}", non_eof[0].kind);
            prop_assert_eq!(&non_eof[0].text, &sql, "Token text should match original");
        }

        /// Property 1f: Escaped double quotes ("") within identifiers are handled correctly
        #[test]
        fn prop_escaped_double_quote_in_ident(
            prefix in "[a-zA-Z_][a-zA-Z0-9_]{0,10}",
            suffix in "[a-zA-Z0-9_]{0,10}"
        ) {
            let sql = format!("\"{}\"\"{}\"", prefix, suffix);
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter().filter(|t| !matches!(t.kind, SqlTokenKind::Eof)).collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token for escaped quote ident, got {:?}", non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::QuotedIdent), "Expected QuotedIdent token, got {:?}", non_eof[0].kind);
            prop_assert_eq!(&non_eof[0].text, &sql, "Token text should match original");
        }
    }
}

#[cfg(test)]
mod keyword_ident_tests {
    use crate::sql_editor::sql_tokenizer::{SqlKeyword, SqlTokenKind, SqlTokenizer};
    use proptest::bool::ANY;
    use proptest::prelude::*;
    // =========================================================================
    // **Feature: sql-smart-completion, Property 2: Tokenizer Keyword/Identifier Recognition**
    // *For any* SQL text containing keywords and identifiers, the tokenizer
    // SHALL correctly classify keywords as KEYWORD tokens and identifiers as
    // IDENT tokens, preserving the original case of identifiers.
    // **Validates: Requirements 1.5, 1.6, 1.7**
    // =========================================================================

    /// All SQL keywords that should be recognized
    const KEYWORDS: &[&str] = &[
        "SELECT",
        "FROM",
        "WHERE",
        "JOIN",
        "INNER",
        "LEFT",
        "RIGHT",
        "FULL",
        "CROSS",
        "ON",
        "AS",
        "AND",
        "OR",
        "NOT",
        "IN",
        "BETWEEN",
        "LIKE",
        "IS",
        "NULL",
        "ORDER",
        "GROUP",
        "BY",
        "HAVING",
        "LIMIT",
        "OFFSET",
        "INSERT",
        "INTO",
        "VALUES",
        "UPDATE",
        "SET",
        "DELETE",
        "CREATE",
        "ALTER",
        "DROP",
        "TABLE",
        "INDEX",
        "VIEW",
        "UNION",
        "INTERSECT",
        "EXCEPT",
        "ALL",
        "DISTINCT",
        "CASE",
        "WHEN",
        "THEN",
        "ELSE",
        "END",
        "WITH",
        "ASC",
        "DESC",
        "USING",
        "EXISTS",
        "PRIMARY",
        "FOREIGN",
        "KEY",
        "REFERENCES",
        "UNIQUE",
        "CHECK",
        "DEFAULT",
        "TRUNCATE",
        "TRUE",
        "FALSE",
    ];

    /// Generate a valid identifier (not a keyword)
    fn identifier_strategy() -> impl Strategy<Value = String> {
        "[a-z_][a-z0-9_]{0,20}".prop_filter("must not be a keyword", |s| {
            SqlKeyword::from_str(s).is_none()
        })
    }

    /// Generate a keyword with random case
    fn keyword_with_case_strategy() -> impl Strategy<Value = (String, String)> {
        prop::sample::select(KEYWORDS).prop_flat_map(|kw| {
            let kw_str = kw.to_string();
            prop::collection::vec(prop::bool::ANY, kw.len()).prop_map(move |cases| {
                let mixed: String = kw_str
                    .chars()
                    .zip(cases.iter())
                    .map(|(c, &upper)| {
                        if upper {
                            c.to_ascii_uppercase()
                        } else {
                            c.to_ascii_lowercase()
                        }
                    })
                    .collect();
                (kw_str.clone(), mixed)
            })
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 2a: Keywords are recognized regardless of case
        #[test]
        fn prop_keywords_recognized_any_case((_original, mixed) in keyword_with_case_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&mixed);
            let tokens = tokenizer.tokenize();

            let non_ws_eof: Vec<_> = tokens.into_iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof | SqlTokenKind::Whitespace))
                .collect();

            prop_assert_eq!(non_ws_eof.len(), 1, "Expected exactly 1 token for keyword '{}', got {:?}", mixed, non_ws_eof);
            prop_assert!(matches!(&non_ws_eof[0].kind, SqlTokenKind::Keyword(_)),
                "Expected Keyword token for '{}', got {:?}", mixed, non_ws_eof[0].kind);
        }

        /// Property 2b: Identifiers preserve original case
        #[test]
        fn prop_identifiers_preserve_case(ident in identifier_strategy()) {
            // Create mixed case version
            let mixed: String = ident.chars().enumerate()
                .map(|(i, c)| if i % 2 == 0 { c.to_ascii_uppercase() } else { c })
                .collect();

            let mut tokenizer = SqlTokenizer::new(&mixed);
            let tokens = tokenizer.tokenize();

            let non_ws_eof: Vec<_> = tokens.into_iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof | SqlTokenKind::Whitespace))
                .collect();

            prop_assert_eq!(non_ws_eof.len(), 1, "Expected exactly 1 token, got {:?}", non_ws_eof);
            prop_assert!(matches!(non_ws_eof[0].kind, SqlTokenKind::Ident),
                "Expected Ident token for '{}', got {:?}", mixed, non_ws_eof[0].kind);
            prop_assert_eq!(&non_ws_eof[0].text, &mixed, "Identifier text should preserve original case");
        }

        /// Property 2c: Operators are correctly tokenized
        #[test]
        fn prop_operators_tokenized(op in prop::sample::select(&["=", "<", ">", "+", "-", "*", "/", "<=", ">=", "<>", "!="])) {
            let mut tokenizer = SqlTokenizer::new(op);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof))
                .collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token for operator '{}', got {:?}", op, non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::Operator),
                "Expected Operator token for '{}', got {:?}", op, non_eof[0].kind);
        }

        /// Property 2d: Punctuation is correctly tokenized
        #[test]
        fn prop_punctuation_tokenized(punct in prop::sample::select(&[".", ",", ";", "(", ")"])) {
            let mut tokenizer = SqlTokenizer::new(punct);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof))
                .collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token for punctuation '{}', got {:?}", punct, non_eof);

            let expected_kind = match punct {
                "." => SqlTokenKind::Dot,
                "," => SqlTokenKind::Comma,
                ";" => SqlTokenKind::Semicolon,
                "(" => SqlTokenKind::LParen,
                ")" => SqlTokenKind::RParen,
                _ => unreachable!(),
            };
            prop_assert_eq!(&non_eof[0].kind, &expected_kind, "Unexpected token kind for '{}'", punct);
        }

        /// Property 2e: Numbers are correctly tokenized
        #[test]
        fn prop_numbers_tokenized(
            int_part in "[1-9][0-9]{0,5}",
            has_decimal in ANY,
            decimal_part in "[0-9]{1,3}"
        ) {
            let num = if has_decimal {
                format!("{}.{}", int_part, decimal_part)
            } else {
                int_part.clone()
            };

            let mut tokenizer = SqlTokenizer::new(&num);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.into_iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof))
                .collect();

            prop_assert_eq!(non_eof.len(), 1, "Expected exactly 1 token for number '{}', got {:?}", num, non_eof);
            prop_assert!(matches!(non_eof[0].kind, SqlTokenKind::Number),
                "Expected Number token for '{}', got {:?}", num, non_eof[0].kind);
        }
    }
}

#[cfg(test)]
mod token_position_tests {
    use crate::sql_editor::sql_tokenizer::{SqlTokenKind, SqlTokenizer, TokenAtResult};
    use proptest::prelude::*;

    // =========================================================================
    // **Feature: sql-smart-completion, Property 7: Token Position Tracking**
    // *For any* SQL text and any valid offset within that text, the tokenizer
    // SHALL be able to identify which token contains that offset, and the
    // token's start/end offsets SHALL correctly bound the token text.
    // **Validates: Requirements 6.1, 6.2, 6.3**
    // =========================================================================

    /// Generate simple SQL fragments for position testing
    fn sql_fragment_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("SELECT".to_string()),
            Just("SELECT id FROM users".to_string()),
            Just("SELECT u.id FROM users u".to_string()),
            Just("SELECT * FROM t WHERE x = 1".to_string()),
            "[a-zA-Z_][a-zA-Z0-9_]{0,10}".prop_map(|s| s),
            "[a-zA-Z_][a-zA-Z0-9_]{0,5} [a-zA-Z_][a-zA-Z0-9_]{0,5}".prop_map(|s| s),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 7a: Every token has valid start/end bounds
        #[test]
        fn prop_token_bounds_are_valid(sql in sql_fragment_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            for token in tokens.iter() {
                if matches!(token.kind, SqlTokenKind::Eof) {
                    continue;
                }
                // start < end
                prop_assert!(token.start < token.end,
                    "Token {:?} has invalid bounds: start={}, end={}", token.kind, token.start, token.end);
                // end <= input length
                prop_assert!(token.end <= sql.len(),
                    "Token {:?} end {} exceeds input length {}", token.kind, token.end, sql.len());
                // text matches slice
                prop_assert_eq!(&token.text, &sql[token.start..token.end],
                    "Token text doesn't match input slice");
            }
        }

        /// Property 7b: token_at returns InToken for any offset within a token
        #[test]
        fn prop_token_at_returns_correct_token(sql in sql_fragment_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            for token in tokens.iter() {
                if matches!(token.kind, SqlTokenKind::Eof) {
                    continue;
                }
                // Test offset at token start
                let result = SqlTokenizer::token_at(&tokens, token.start);
                match result {
                    TokenAtResult::InToken(t) => {
                        prop_assert_eq!(t.start, token.start,
                            "token_at({}) returned wrong token", token.start);
                    }
                    _ => prop_assert!(false, "Expected InToken at offset {}, got {:?}", token.start, result),
                }

                // Test offset in middle of token (if token has length > 1)
                if token.end - token.start > 1 {
                    let mid = token.start + (token.end - token.start) / 2;
                    let result = SqlTokenizer::token_at(&tokens, mid);
                    match result {
                        TokenAtResult::InToken(t) => {
                            prop_assert_eq!(t.start, token.start,
                                "token_at({}) returned wrong token", mid);
                        }
                        _ => prop_assert!(false, "Expected InToken at offset {}, got {:?}", mid, result),
                    }
                }
            }
        }

        /// Property 7c: tokens_before returns only tokens ending before offset
        #[test]
        fn prop_tokens_before_correct(sql in sql_fragment_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            // Test at various offsets
            for offset in (0..=sql.len()).step_by(1.max(sql.len() / 10)) {
                let before = SqlTokenizer::tokens_before(&tokens, offset);
                for t in before.iter() {
                    prop_assert!(t.end <= offset,
                        "tokens_before({}) returned token ending at {}", offset, t.end);
                    prop_assert!(!matches!(t.kind, SqlTokenKind::Whitespace | SqlTokenKind::Eof),
                        "tokens_before should not return whitespace or EOF");
                }
            }
        }

        /// Property 7d: Consecutive tokens don't overlap
        #[test]
        fn prop_tokens_dont_overlap(sql in sql_fragment_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof))
                .collect();

            for window in non_eof.windows(2) {
                let prev = window[0];
                let next = window[1];
                prop_assert!(prev.end <= next.start,
                    "Tokens overlap: {:?} ends at {}, {:?} starts at {}",
                    prev.kind, prev.end, next.kind, next.start);
            }
        }
    }
}

#[cfg(test)]
mod tokenizer_reconstruction_tests {
    use crate::sql_editor::sql_tokenizer::{SqlTokenKind, SqlTokenizer};
    use proptest::prelude::*;

    // =========================================================================
    // **Feature: sql-smart-completion, Property 8: Tokenizer Reconstruction**
    // *For any* SQL text, concatenating all non-whitespace token texts in order
    // SHALL produce a string that is semantically equivalent to the original
    // (ignoring whitespace differences).
    // **Validates: Requirements 1.1-1.7**
    // =========================================================================

    /// Generate SQL-like text for reconstruction testing
    fn sql_text_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("SELECT id FROM users".to_string()),
            Just("SELECT u.id, u.name FROM users u WHERE u.active = 1".to_string()),
            Just("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id".to_string()),
            Just("INSERT INTO users (name) VALUES ('test')".to_string()),
            Just("UPDATE users SET name = 'new' WHERE id = 1".to_string()),
            // Simple identifier sequences
            "[a-zA-Z_][a-zA-Z0-9_]{0,8}( [a-zA-Z_][a-zA-Z0-9_]{0,8}){0,5}".prop_map(|s| s),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8a: Concatenating all token texts reproduces the original input
        #[test]
        fn prop_token_texts_reconstruct_input(sql in sql_text_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            // Concatenate all token texts (including whitespace, excluding EOF)
            let reconstructed: String = tokens.iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof))
                .map(|t| t.text.as_str())
                .collect();

            prop_assert_eq!(&reconstructed, &sql,
                "Reconstructed text doesn't match original");
        }

        /// Property 8b: Token positions cover the entire input without gaps
        #[test]
        fn prop_tokens_cover_input(sql in sql_text_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            let non_eof: Vec<_> = tokens.iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof))
                .collect();

            if non_eof.is_empty() {
                prop_assert!(sql.is_empty(), "No tokens but input is not empty");
                return Ok(());
            }

            // First token should start at 0
            prop_assert_eq!(non_eof[0].start, 0,
                "First token doesn't start at 0");

            // Last token should end at input length
            prop_assert_eq!(non_eof.last().unwrap().end, sql.len(),
                "Last token doesn't end at input length");

            // No gaps between consecutive tokens
            for window in non_eof.windows(2) {
                prop_assert_eq!(window[0].end, window[1].start,
                    "Gap between tokens: {:?} ends at {}, {:?} starts at {}",
                    window[0].kind, window[0].end, window[1].kind, window[1].start);
            }
        }

        /// Property 8c: Removing whitespace from tokens equals removing whitespace from input
        #[test]
        fn prop_non_whitespace_content_preserved(sql in sql_text_strategy()) {
            let mut tokenizer = SqlTokenizer::new(&sql);
            let tokens = tokenizer.tokenize();

            // Get non-whitespace content from tokens
            let token_content: String = tokens.iter()
                .filter(|t| !matches!(t.kind, SqlTokenKind::Eof | SqlTokenKind::Whitespace))
                .map(|t| t.text.as_str())
                .collect();

            // Get non-whitespace content from original
            let original_content: String = sql.chars()
                .filter(|c| !c.is_whitespace())
                .collect();

            prop_assert_eq!(&token_content, &original_content,
                "Non-whitespace content differs");
        }
    }
}
