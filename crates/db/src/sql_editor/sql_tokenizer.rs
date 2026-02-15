/// SQL Tokenizer module for smart completion
///
/// This module provides lexical analysis for SQL text, correctly handling
/// strings, comments, keywords, and identifiers.

/// SQL keyword enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SqlKeyword {
    Select,
    From,
    Where,
    Join,
    Inner,
    Left,
    Right,
    Full,
    Cross,
    On,
    As,
    And,
    Or,
    Not,
    In,
    Between,
    Like,
    Is,
    Null,
    Order,
    Group,
    By,
    Having,
    Limit,
    Offset,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    Create,
    Alter,
    Drop,
    Table,
    Index,
    View,
    Union,
    Intersect,
    Except,
    All,
    Distinct,
    Case,
    When,
    Then,
    Else,
    End,
    With,
    Asc,
    Desc,
    Using,
    Exists,
    Primary,
    Foreign,
    Key,
    References,
    Unique,
    Check,
    Default,
    Truncate,
    True,
    False,
}

impl SqlKeyword {
    /// Try to parse a keyword from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<SqlKeyword> {
        match s.to_uppercase().as_str() {
            "SELECT" => Some(SqlKeyword::Select),
            "FROM" => Some(SqlKeyword::From),
            "WHERE" => Some(SqlKeyword::Where),
            "JOIN" => Some(SqlKeyword::Join),
            "INNER" => Some(SqlKeyword::Inner),
            "LEFT" => Some(SqlKeyword::Left),
            "RIGHT" => Some(SqlKeyword::Right),
            "FULL" => Some(SqlKeyword::Full),
            "CROSS" => Some(SqlKeyword::Cross),
            "ON" => Some(SqlKeyword::On),
            "AS" => Some(SqlKeyword::As),
            "AND" => Some(SqlKeyword::And),
            "OR" => Some(SqlKeyword::Or),
            "NOT" => Some(SqlKeyword::Not),
            "IN" => Some(SqlKeyword::In),
            "BETWEEN" => Some(SqlKeyword::Between),
            "LIKE" => Some(SqlKeyword::Like),
            "IS" => Some(SqlKeyword::Is),
            "NULL" => Some(SqlKeyword::Null),
            "ORDER" => Some(SqlKeyword::Order),
            "GROUP" => Some(SqlKeyword::Group),
            "BY" => Some(SqlKeyword::By),
            "HAVING" => Some(SqlKeyword::Having),
            "LIMIT" => Some(SqlKeyword::Limit),
            "OFFSET" => Some(SqlKeyword::Offset),
            "INSERT" => Some(SqlKeyword::Insert),
            "INTO" => Some(SqlKeyword::Into),
            "VALUES" => Some(SqlKeyword::Values),
            "UPDATE" => Some(SqlKeyword::Update),
            "SET" => Some(SqlKeyword::Set),
            "DELETE" => Some(SqlKeyword::Delete),
            "CREATE" => Some(SqlKeyword::Create),
            "ALTER" => Some(SqlKeyword::Alter),
            "DROP" => Some(SqlKeyword::Drop),
            "TABLE" => Some(SqlKeyword::Table),
            "INDEX" => Some(SqlKeyword::Index),
            "VIEW" => Some(SqlKeyword::View),
            "UNION" => Some(SqlKeyword::Union),
            "INTERSECT" => Some(SqlKeyword::Intersect),
            "EXCEPT" => Some(SqlKeyword::Except),
            "ALL" => Some(SqlKeyword::All),
            "DISTINCT" => Some(SqlKeyword::Distinct),
            "CASE" => Some(SqlKeyword::Case),
            "WHEN" => Some(SqlKeyword::When),
            "THEN" => Some(SqlKeyword::Then),
            "ELSE" => Some(SqlKeyword::Else),
            "END" => Some(SqlKeyword::End),
            "WITH" => Some(SqlKeyword::With),
            "ASC" => Some(SqlKeyword::Asc),
            "DESC" => Some(SqlKeyword::Desc),
            "USING" => Some(SqlKeyword::Using),
            "EXISTS" => Some(SqlKeyword::Exists),
            "PRIMARY" => Some(SqlKeyword::Primary),
            "FOREIGN" => Some(SqlKeyword::Foreign),
            "KEY" => Some(SqlKeyword::Key),
            "REFERENCES" => Some(SqlKeyword::References),
            "UNIQUE" => Some(SqlKeyword::Unique),
            "CHECK" => Some(SqlKeyword::Check),
            "DEFAULT" => Some(SqlKeyword::Default),
            "TRUNCATE" => Some(SqlKeyword::Truncate),
            "TRUE" => Some(SqlKeyword::True),
            "FALSE" => Some(SqlKeyword::False),
            _ => None,
        }
    }
}

/// Token kind enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum SqlTokenKind {
    /// SQL keyword (SELECT, FROM, WHERE, etc.)
    Keyword(SqlKeyword),
    /// Regular identifier (table name, column name, etc.)
    Ident,
    /// Double-quoted identifier ("column name")
    QuotedIdent,
    /// Single-quoted string literal ('value')
    String,
    /// Numeric literal (123, 3.14, etc.)
    Number,
    /// Line comment (-- comment)
    LineComment,
    /// Block comment (/* comment */)
    BlockComment,
    /// Dot operator (.)
    Dot,
    /// Comma (,)
    Comma,
    /// Semicolon (;)
    Semicolon,
    /// Left parenthesis (()
    LParen,
    /// Right parenthesis ())
    RParen,
    /// Other operators (=, <, >, +, -, *, /, etc.)
    Operator,
    /// Whitespace (space, tab, newline)
    Whitespace,
    /// Unknown/unrecognized character
    Unknown,
    /// End of file
    Eof,
}

/// A single token with position information
#[derive(Debug, Clone)]
pub struct SqlToken {
    /// The kind of token
    pub kind: SqlTokenKind,
    /// The original text of the token
    pub text: String,
    /// Start offset in the source (byte offset)
    pub start: usize,
    /// End offset in the source (byte offset, exclusive)
    pub end: usize,
}

impl SqlToken {
    /// Create a new token
    pub fn new(kind: SqlTokenKind, text: String, start: usize, end: usize) -> Self {
        Self {
            kind,
            text,
            start,
            end,
        }
    }

    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(self.kind, SqlTokenKind::Keyword(_))
    }

    /// Check if this token is a specific keyword
    pub fn is_keyword_of(&self, kw: SqlKeyword) -> bool {
        matches!(&self.kind, SqlTokenKind::Keyword(k) if *k == kw)
    }

    /// Check if this token is whitespace
    pub fn is_whitespace(&self) -> bool {
        matches!(self.kind, SqlTokenKind::Whitespace)
    }

    /// Check if this token is a comment
    pub fn is_comment(&self) -> bool {
        matches!(
            self.kind,
            SqlTokenKind::LineComment | SqlTokenKind::BlockComment
        )
    }
}

/// SQL Tokenizer - lexical analyzer for SQL text
pub struct SqlTokenizer<'a> {
    input: &'a str,
    pos: usize,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl<'a> SqlTokenizer<'a> {
    /// Create a new tokenizer for the given input
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            chars: input.char_indices().peekable(),
        }
    }

    /// Tokenize the entire input and return all tokens
    pub fn tokenize(&mut self) -> Vec<SqlToken> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = matches!(token.kind, SqlTokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Get the next token from the input
    fn next_token(&mut self) -> SqlToken {
        // Skip to next character
        let (start, ch) = match self.chars.peek().copied() {
            Some((pos, ch)) => (pos, ch),
            None => {
                return SqlToken::new(
                    SqlTokenKind::Eof,
                    String::new(),
                    self.input.len(),
                    self.input.len(),
                )
            }
        };

        // Whitespace
        if ch.is_whitespace() {
            return self.scan_whitespace(start);
        }

        // Line comment: --
        if ch == '-' {
            if let Some((_, '-')) = self.peek_next() {
                return self.scan_line_comment(start);
            }
        }

        // Block comment: /* */
        if ch == '/' {
            if let Some((_, '*')) = self.peek_next() {
                return self.scan_block_comment(start);
            }
        }

        // Single-quoted string: 'text'
        if ch == '\'' {
            return self.scan_string(start);
        }

        // Double-quoted identifier: "identifier"
        if ch == '"' {
            return self.scan_quoted_ident(start);
        }

        // Number
        if ch.is_ascii_digit() {
            return self.scan_number(start);
        }

        // Identifier or keyword
        if ch.is_alphabetic() || ch == '_' {
            return self.scan_identifier(start);
        }

        // Punctuation and operators
        self.advance();
        match ch {
            '.' => SqlToken::new(SqlTokenKind::Dot, ".".to_string(), start, start + 1),
            ',' => SqlToken::new(SqlTokenKind::Comma, ",".to_string(), start, start + 1),
            ';' => SqlToken::new(SqlTokenKind::Semicolon, ";".to_string(), start, start + 1),
            '(' => SqlToken::new(SqlTokenKind::LParen, "(".to_string(), start, start + 1),
            ')' => SqlToken::new(SqlTokenKind::RParen, ")".to_string(), start, start + 1),
            '=' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '!' | '&' | '|' | '^' | '~' => {
                self.scan_operator(start, ch)
            }
            _ => SqlToken::new(
                SqlTokenKind::Unknown,
                ch.to_string(),
                start,
                start + ch.len_utf8(),
            ),
        }
    }

    /// Advance to the next character
    fn advance(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((pos, ch)) = result {
            self.pos = pos + ch.len_utf8();
        }
        result
    }

    /// Peek at the next character (after current)
    fn peek_next(&mut self) -> Option<(usize, char)> {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next()
    }

    /// Scan whitespace
    fn scan_whitespace(&mut self, start: usize) -> SqlToken {
        while let Some(&(_, ch)) = self.chars.peek() {
            if !ch.is_whitespace() {
                break;
            }
            self.advance();
        }
        let text = &self.input[start..self.pos];
        SqlToken::new(SqlTokenKind::Whitespace, text.to_string(), start, self.pos)
    }

    /// Scan identifier or keyword
    fn scan_identifier(&mut self, start: usize) -> SqlToken {
        while let Some(&(_, ch)) = self.chars.peek() {
            if !(ch.is_alphanumeric() || ch == '_') {
                break;
            }
            self.advance();
        }
        let text = &self.input[start..self.pos];
        let kind = if let Some(kw) = SqlKeyword::from_str(text) {
            SqlTokenKind::Keyword(kw)
        } else {
            SqlTokenKind::Ident
        };
        SqlToken::new(kind, text.to_string(), start, self.pos)
    }

    /// Scan number literal
    fn scan_number(&mut self, start: usize) -> SqlToken {
        // Integer part
        while let Some(&(_, ch)) = self.chars.peek() {
            if !ch.is_ascii_digit() {
                break;
            }
            self.advance();
        }
        // Decimal part
        if let Some(&(_, '.')) = self.chars.peek() {
            let mut iter = self.chars.clone();
            iter.next();
            if let Some((_, ch)) = iter.next() {
                if ch.is_ascii_digit() {
                    self.advance(); // consume '.'
                    while let Some(&(_, ch)) = self.chars.peek() {
                        if !ch.is_ascii_digit() {
                            break;
                        }
                        self.advance();
                    }
                }
            }
        }
        // Exponent part (e.g., 1e10, 1E-5)
        if let Some(&(_, ch)) = self.chars.peek() {
            if ch == 'e' || ch == 'E' {
                let mut iter = self.chars.clone();
                iter.next();
                if let Some((_, next_ch)) = iter.next() {
                    if next_ch.is_ascii_digit() || next_ch == '+' || next_ch == '-' {
                        self.advance(); // consume 'e' or 'E'
                        if let Some(&(_, sign)) = self.chars.peek() {
                            if sign == '+' || sign == '-' {
                                self.advance();
                            }
                        }
                        while let Some(&(_, ch)) = self.chars.peek() {
                            if !ch.is_ascii_digit() {
                                break;
                            }
                            self.advance();
                        }
                    }
                }
            }
        }
        let text = &self.input[start..self.pos];
        SqlToken::new(SqlTokenKind::Number, text.to_string(), start, self.pos)
    }

    /// Scan operator (may be multi-character like <=, >=, <>, !=, ||, etc.)
    fn scan_operator(&mut self, start: usize, first_ch: char) -> SqlToken {
        let mut text = first_ch.to_string();
        // Check for two-character operators
        if let Some(&(_, next_ch)) = self.chars.peek() {
            let two_char = format!("{}{}", first_ch, next_ch);
            if matches!(
                two_char.as_str(),
                "<=" | ">=" | "<>" | "!=" | "||" | "&&" | "::" | "->"
            ) {
                self.advance();
                text = two_char;
            }
        }
        SqlToken::new(
            SqlTokenKind::Operator,
            text.clone(),
            start,
            start + text.len(),
        )
    }
}

impl<'a> SqlTokenizer<'a> {
    /// Scan single-quoted string literal with escape handling ('text', 'it''s')
    fn scan_string(&mut self, start: usize) -> SqlToken {
        self.advance(); // consume opening quote
        loop {
            match self.chars.peek().copied() {
                Some((_, '\'')) => {
                    self.advance(); // consume quote
                                    // Check for escaped quote ('')
                    if let Some(&(_, '\'')) = self.chars.peek() {
                        self.advance(); // consume second quote, continue string
                    } else {
                        break; // end of string
                    }
                }
                Some((_, _)) => {
                    self.advance();
                }
                None => break, // unterminated string
            }
        }
        let text = &self.input[start..self.pos];
        SqlToken::new(SqlTokenKind::String, text.to_string(), start, self.pos)
    }

    /// Scan double-quoted identifier with escape handling ("col", "col""name")
    fn scan_quoted_ident(&mut self, start: usize) -> SqlToken {
        self.advance(); // consume opening quote
        loop {
            match self.chars.peek().copied() {
                Some((_, '"')) => {
                    self.advance(); // consume quote
                                    // Check for escaped quote ("")
                    if let Some(&(_, '"')) = self.chars.peek() {
                        self.advance(); // consume second quote, continue identifier
                    } else {
                        break; // end of identifier
                    }
                }
                Some((_, _)) => {
                    self.advance();
                }
                None => break, // unterminated identifier
            }
        }
        let text = &self.input[start..self.pos];
        SqlToken::new(SqlTokenKind::QuotedIdent, text.to_string(), start, self.pos)
    }
}

impl<'a> SqlTokenizer<'a> {
    /// Scan line comment (-- to end of line)
    fn scan_line_comment(&mut self, start: usize) -> SqlToken {
        // Consume --
        self.advance();
        self.advance();
        // Consume until end of line
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
        let text = &self.input[start..self.pos];
        SqlToken::new(SqlTokenKind::LineComment, text.to_string(), start, self.pos)
    }

    /// Scan block comment (/* ... */)
    fn scan_block_comment(&mut self, start: usize) -> SqlToken {
        // Consume /*
        self.advance();
        self.advance();
        // Consume until */
        loop {
            match self.chars.peek().copied() {
                Some((_, '*')) => {
                    self.advance();
                    if let Some(&(_, '/')) = self.chars.peek() {
                        self.advance();
                        break;
                    }
                }
                Some((_, _)) => {
                    self.advance();
                }
                None => break, // unterminated comment
            }
        }
        let text = &self.input[start..self.pos];
        SqlToken::new(
            SqlTokenKind::BlockComment,
            text.to_string(),
            start,
            self.pos,
        )
    }
}

// =============================================================================
// Token Position Tracking Methods
// =============================================================================

/// Result of token_at query when offset is between tokens
#[derive(Debug, Clone)]
pub enum TokenAtResult<'a> {
    /// Offset is within a token
    InToken(&'a SqlToken),
    /// Offset is in whitespace/gap between tokens, returns preceding token (if any)
    InGap(Option<&'a SqlToken>),
    /// Offset is at the very start (before any token)
    AtStart,
    /// Offset is beyond the end of input
    BeyondEnd,
}

impl SqlTokenizer<'_> {
    /// Find the token at a specific offset in the tokenized output.
    ///
    /// Returns:
    /// - `TokenAtResult::InToken(&token)` if offset is within a token's [start, end) range
    /// - `TokenAtResult::InGap(Some(&token))` if offset is in whitespace after a token
    /// - `TokenAtResult::InGap(None)` if offset is in leading whitespace
    /// - `TokenAtResult::AtStart` if offset is 0 and there are no tokens
    /// - `TokenAtResult::BeyondEnd` if offset is beyond the input length
    ///
    /// Edge cases:
    /// - Offset at token boundary (start): returns that token
    /// - Offset at token boundary (end): returns next token or gap
    pub fn token_at(tokens: &[SqlToken], offset: usize) -> TokenAtResult<'_> {
        if tokens.is_empty() {
            return if offset == 0 {
                TokenAtResult::AtStart
            } else {
                TokenAtResult::BeyondEnd
            };
        }

        // Find the last non-EOF token to determine input bounds
        let last_content_token = tokens
            .iter()
            .rev()
            .find(|t| !matches!(t.kind, SqlTokenKind::Eof));
        let input_end = match last_content_token {
            Some(t) => t.end,
            None => {
                // Only EOF token exists
                return if offset == 0 {
                    TokenAtResult::AtStart
                } else {
                    TokenAtResult::BeyondEnd
                };
            }
        };

        if offset > input_end {
            return TokenAtResult::BeyondEnd;
        }

        // Binary search for the token containing or preceding the offset
        let mut prev_token: Option<&SqlToken> = None;

        for token in tokens.iter() {
            if matches!(token.kind, SqlTokenKind::Eof) {
                continue;
            }

            // Offset is within this token's range [start, end)
            if offset >= token.start && offset < token.end {
                return TokenAtResult::InToken(token);
            }

            // Offset is before this token starts
            if offset < token.start {
                // We're in a gap before this token
                return TokenAtResult::InGap(prev_token);
            }

            prev_token = Some(token);
        }

        // Offset is at or after the last token's end
        TokenAtResult::InGap(prev_token)
    }

    /// Get all tokens that end before the given offset.
    ///
    /// This is useful for context analysis - determining what SQL constructs
    /// precede the cursor position.
    ///
    /// Returns a slice of tokens where each token's end offset is <= the given offset.
    /// Excludes EOF tokens and whitespace tokens from the result.
    pub fn tokens_before(tokens: &[SqlToken], offset: usize) -> Vec<&SqlToken> {
        tokens
            .iter()
            .filter(|t| {
                t.end <= offset && !matches!(t.kind, SqlTokenKind::Eof | SqlTokenKind::Whitespace)
            })
            .collect()
    }

    /// Get all non-whitespace, non-EOF tokens up to and including the token at offset.
    ///
    /// This includes the token containing the offset (if any).
    pub fn tokens_up_to(tokens: &[SqlToken], offset: usize) -> Vec<&SqlToken> {
        tokens
            .iter()
            .filter(|t| {
                t.start <= offset && !matches!(t.kind, SqlTokenKind::Eof | SqlTokenKind::Whitespace)
            })
            .collect()
    }
}
