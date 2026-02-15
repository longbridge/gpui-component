use crate::executor::SqlSource;
use one_core::storage::DatabaseType;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read};
use std::path::PathBuf;

/// 统一的 SQL 读取器，支持字符串和文件两种来源
enum SqlReader {
    Memory(Cursor<Vec<u8>>),
    File(BufReader<File>),
}

impl Read for SqlReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            SqlReader::Memory(cursor) => cursor.read(buf),
            SqlReader::File(reader) => reader.read(buf),
        }
    }
}

impl BufRead for SqlReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            SqlReader::Memory(cursor) => cursor.fill_buf(),
            SqlReader::File(reader) => reader.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            SqlReader::Memory(cursor) => cursor.consume(amt),
            SqlReader::File(reader) => reader.consume(amt),
        }
    }
}

/// 流式 SQL 解析器
/// 从 BufRead 流中按需读取并解析 SQL 语句，避免一次性加载整个文件
pub struct StreamingSqlParser {
    reader: SqlReader,
    db_type: DatabaseType,
    buffer: String,
    bytes_read: u64,
    total_size: u64,

    in_string: bool,
    string_char: char,
    escape_next: bool,
    prev_was_string_char: bool,
    in_line_comment: bool,
    in_block_comment: bool,
    dollar_quote: Option<String>,

    paren_depth: i32,
    begin_depth: i32,
    last_checked_len: usize,
    delimiter: String,

    pending_chars: Vec<char>,
    eof: bool,
}

impl StreamingSqlParser {
    /// 从 SqlSource 创建解析器
    pub fn from_source(source: SqlSource, db_type: DatabaseType) -> io::Result<Self> {
        let (reader, total_size) = match source {
            SqlSource::Script(script) => {
                let size = script.len() as u64;
                (SqlReader::Memory(Cursor::new(script.into_bytes())), size)
            }
            SqlSource::File(path) => {
                let file = File::open(&path)?;
                let size = file.metadata()?.len();
                (SqlReader::File(BufReader::new(file)), size)
            }
        };

        Ok(Self {
            reader,
            db_type,
            buffer: String::new(),
            bytes_read: 0,
            total_size,
            in_string: false,
            string_char: '\0',
            escape_next: false,
            prev_was_string_char: false,
            in_line_comment: false,
            in_block_comment: false,
            dollar_quote: None,
            paren_depth: 0,
            begin_depth: 0,
            last_checked_len: 0,
            delimiter: ";".to_string(),
            pending_chars: Vec::new(),
            eof: false,
        })
    }

    /// 从文件路径创建解析器
    pub fn from_file(path: PathBuf, db_type: DatabaseType) -> io::Result<Self> {
        Self::from_source(SqlSource::File(path), db_type)
    }

    /// 从脚本字符串创建解析器
    pub fn from_script(script: String, db_type: DatabaseType) -> io::Result<Self> {
        Self::from_source(SqlSource::Script(script), db_type)
    }

    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// 进度百分比
    pub fn progress_percent(&self) -> f32 {
        if self.total_size > 0 {
            (self.bytes_read as f64 / self.total_size as f64 * 100.0) as f32
        } else {
            0.0
        }
    }

    fn read_next_statement(&mut self) -> io::Result<Option<String>> {
        if self.eof && self.buffer.is_empty() && self.pending_chars.is_empty() {
            return Ok(None);
        }

        let mut line_buf = String::new();

        loop {
            // First process any pending characters from previous line
            if !self.pending_chars.is_empty() {
                let chars: Vec<char> = self.pending_chars.drain(..).collect();
                for ch in chars {
                    if let Some(stmt) = self.process_char(ch) {
                        return Ok(Some(stmt));
                    }
                }
            }

            // Then read new line if not EOF
            if !self.eof {
                line_buf.clear();
                match self.reader.read_line(&mut line_buf) {
                    Ok(0) => {
                        self.eof = true;
                    }
                    Ok(n) => {
                        self.bytes_read += n as u64;
                    }
                    Err(e) => return Err(e),
                }
            }

            if !line_buf.is_empty() {
                let chars = line_buf.chars().collect::<Vec<char>>();
                let mut i = 0;
                while i < chars.len() {
                    if let Some(stmt) = self.process_char(chars[i]) {
                        // Save remaining characters for next call
                        self.pending_chars.extend_from_slice(&chars[i + 1..]);
                        return Ok(Some(stmt));
                    }
                    i += 1;
                }
            }

            if self.eof {
                let trimmed = self.buffer.trim();
                if !trimmed.is_empty()
                    && !trimmed.to_uppercase().starts_with("DELIMITER")
                    && !self.is_pure_comment(trimmed)
                {
                    let stmt = trimmed.to_string();
                    self.buffer.clear();
                    self.last_checked_len = 0;
                    return Ok(Some(stmt));
                }
                self.buffer.clear();
                self.last_checked_len = 0;
                return Ok(None);
            }
        }
    }

    fn process_char(&mut self, ch: char) -> Option<String> {
        if self.in_line_comment {
            self.buffer.push(ch);
            if ch == '\n' {
                self.in_line_comment = false;
            }
            return None;
        }

        if self.in_block_comment {
            self.buffer.push(ch);
            if ch == '/' && self.buffer.ends_with("*/") {
                self.in_block_comment = false;
            }
            return None;
        }

        if let Some(ref tag) = self.dollar_quote.clone() {
            self.buffer.push(ch);
            if ch == '$' {
                let end_pos = self.buffer.len();
                let start_pos = end_pos.saturating_sub(tag.len());
                if self.buffer[start_pos..].ends_with(tag.as_str()) {
                    self.dollar_quote = None;
                }
            }
            return None;
        }

        if self.in_string {
            if self.escape_next {
                self.buffer.push(ch);
                self.escape_next = false;
                self.prev_was_string_char = false;
                return None;
            }

            if ch == '\\' && self.db_type == DatabaseType::MySQL {
                self.buffer.push(ch);
                self.escape_next = true;
                self.prev_was_string_char = false;
                return None;
            }

            if ch == self.string_char {
                self.buffer.push(ch);
                if self.prev_was_string_char {
                    // This is '' escape - two quotes represent one escaped quote
                    self.prev_was_string_char = false;
                } else {
                    // Might be end of string or start of '' escape
                    self.prev_was_string_char = true;
                }
                return None;
            }

            // Non-quote, non-escape character
            if self.prev_was_string_char {
                // Previous quote was end of string, process this char normally
                self.in_string = false;
                self.prev_was_string_char = false;
                // Fall through to normal character processing
            } else {
                self.buffer.push(ch);
                return None;
            }
        }

        if ch == '-' && self.buffer.ends_with('-') {
            self.buffer.push(ch);
            self.in_line_comment = true;
            return None;
        }

        if ch == '-' {
            self.buffer.push(ch);
            return None;
        }

        if ch == '#' && self.db_type == DatabaseType::MySQL {
            self.buffer.push(ch);
            self.in_line_comment = true;
            return None;
        }

        if ch == '*' && self.buffer.ends_with('/') {
            self.buffer.push(ch);
            self.in_block_comment = true;
            return None;
        }

        if ch == '$' && self.db_type == DatabaseType::PostgreSQL {
            self.buffer.push(ch);
            if let Some(tag) = self.try_extract_dollar_quote() {
                self.dollar_quote = Some(tag);
            }
            return None;
        }

        if ch == '\'' || ch == '"' {
            self.in_string = true;
            self.string_char = ch;
            self.buffer.push(ch);
            return None;
        }

        if ch == '`' && self.db_type == DatabaseType::MySQL {
            self.in_string = true;
            self.string_char = ch;
            self.buffer.push(ch);
            return None;
        }

        if ch == '(' {
            self.paren_depth += 1;
            self.buffer.push(ch);
            return None;
        }

        if ch == ')' {
            self.paren_depth = (self.paren_depth - 1).max(0);
            self.buffer.push(ch);
            return None;
        }

        self.buffer.push(ch);

        if ch.is_whitespace() || ch == ';' || ch == '$' {
            self.update_begin_depth();
        }

        if self.db_type == DatabaseType::MySQL && ch == '\n' {
            if let Some(new_delim) = self.try_parse_delimiter() {
                self.delimiter = new_delim;
                let lines: Vec<&str> = self.buffer.lines().collect();
                if lines.len() > 1 {
                    self.buffer = lines[..lines.len() - 1].join("\n");
                    self.last_checked_len = 0;
                } else {
                    self.buffer.clear();
                    self.last_checked_len = 0;
                }
                return None;
            }
        }

        if self.db_type == DatabaseType::MSSQL && ch == '\n' {
            let lines: Vec<&str> = self.buffer.lines().collect();
            if let Some(last_line) = lines.last() {
                if last_line.trim().to_uppercase() == "GO" {
                    let stmt_lines: Vec<&str> = lines[..lines.len() - 1].to_vec();
                    let stmt = stmt_lines.join("\n").trim().to_string();
                    self.buffer.clear();
                    self.last_checked_len = 0;
                    if !stmt.is_empty() {
                        return Some(stmt);
                    }
                    return None;
                }
            }
        }

        if self.paren_depth == 0 && self.begin_depth == 0 {
            let trimmed_current = self.buffer.trim_end();
            if trimmed_current.ends_with(&self.delimiter) {
                let stmt = trimmed_current
                    .strip_suffix(&self.delimiter)
                    .unwrap_or(trimmed_current)
                    .trim();

                if !stmt.is_empty()
                    && !stmt.to_uppercase().starts_with("DELIMITER")
                    && !self.is_pure_comment(stmt)
                {
                    let result = stmt.to_string();
                    self.buffer.clear();
                    self.last_checked_len = 0;
                    return Some(result);
                }
                self.buffer.clear();
                self.last_checked_len = 0;
            } else if self.db_type == DatabaseType::Oracle
                && self.buffer.trim().ends_with('\n')
                && self.buffer.trim_end().ends_with('/')
            {
                let stmt = self
                    .buffer
                    .trim()
                    .strip_suffix('/')
                    .unwrap_or(&self.buffer)
                    .trim();
                if !stmt.is_empty() {
                    let result = stmt.to_string();
                    self.buffer.clear();
                    self.last_checked_len = 0;
                    return Some(result);
                }
                self.buffer.clear();
                self.last_checked_len = 0;
            }
        }

        None
    }

    fn try_extract_dollar_quote(&self) -> Option<String> {
        let last_dollar_pos = self.buffer.rfind('$')?;
        if last_dollar_pos == 0 {
            return None;
        }

        let before_last = &self.buffer[..last_dollar_pos];
        let prev_dollar_pos = before_last.rfind('$')?;

        let tag = &self.buffer[prev_dollar_pos..=last_dollar_pos];
        let inner = &tag[1..tag.len() - 1];
        if inner.is_empty() || inner.chars().all(|c| c.is_alphanumeric() || c == '_') {
            Some(tag.to_string())
        } else {
            None
        }
    }

    fn try_parse_delimiter(&self) -> Option<String> {
        let lines: Vec<&str> = self.buffer.lines().collect();
        if let Some(last_line) = lines.last() {
            let trimmed = last_line.trim();
            if trimmed.to_uppercase().starts_with("DELIMITER") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Some(parts[1].to_string());
                }
            }
        }
        None
    }

    /// 检查字符串是否为纯注释（只包含注释和空白字符）
    fn is_pure_comment(&self, s: &str) -> bool {
        let mut chars = s.trim().chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                // 空白字符跳过
                c if c.is_whitespace() => continue,

                // 行注释 --
                '-' => {
                    if chars.peek() == Some(&'-') {
                        chars.next();
                        // 跳过直到换行
                        for c in chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                    } else {
                        return false;
                    }
                }

                // MySQL 的 # 注释
                '#' if self.db_type == DatabaseType::MySQL => {
                    // 跳过直到换行
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                }

                // 块注释 /* */
                '/' => {
                    if chars.peek() == Some(&'*') {
                        chars.next();
                        // 跳过直到 */
                        let mut prev = ' ';
                        for c in chars.by_ref() {
                            if prev == '*' && c == '/' {
                                break;
                            }
                            prev = c;
                        }
                    } else {
                        return false;
                    }
                }

                // 其他非空白字符表示不是纯注释
                _ => return false,
            }
        }

        true
    }

    fn update_begin_depth(&mut self) {
        let buffer_len = self.buffer.len();

        if buffer_len <= self.last_checked_len {
            return;
        }

        let buffer_bytes = self.buffer.as_bytes();
        let mut end = buffer_len;

        while end > 0 {
            let ch = buffer_bytes[end - 1];
            if ch.is_ascii_whitespace() || ch == b';' || ch == b',' || ch == b'$' {
                end -= 1;
            } else {
                break;
            }
        }

        if end == 0 {
            self.last_checked_len = buffer_len;
            return;
        }

        if end <= self.last_checked_len {
            self.last_checked_len = buffer_len;
            return;
        }

        let mut start = end;
        while start > 0 && buffer_bytes[start - 1].is_ascii_alphabetic() {
            start -= 1;
        }

        let last_word = &self.buffer[start..end];
        let last_word_upper = last_word.to_uppercase();

        if last_word_upper == "BEGIN" {
            self.begin_depth += 1;
        } else if last_word_upper == "END" {
            self.begin_depth = (self.begin_depth - 1).max(0);
        }

        self.last_checked_len = end;
    }
}

impl Iterator for StreamingSqlParser {
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_statement() {
            Ok(Some(stmt)) => Some(Ok(stmt)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use one_core::storage::DatabaseType;

    fn parse_all(source: SqlSource, db_type: DatabaseType) -> Vec<String> {
        let mut parser = StreamingSqlParser::from_source(source, db_type).unwrap();
        let mut statements = Vec::new();
        while let Some(Ok(stmt)) = parser.next() {
            statements.push(stmt);
        }
        statements
    }

    #[test]
    fn test_basic_statements() {
        let sql = "SELECT * FROM users;\nINSERT INTO users VALUES (1, 'test');\nUPDATE users SET name = 'new';";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0], "SELECT * FROM users");
        assert_eq!(statements[1], "INSERT INTO users VALUES (1, 'test')");
        assert_eq!(statements[2], "UPDATE users SET name = 'new'");
    }

    #[test]
    fn test_string_with_backslash_escape() {
        let sql =
            "INSERT INTO t VALUES ('it\\'s good');\nINSERT INTO t VALUES ('path\\\\to\\\\file');";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "INSERT INTO t VALUES ('it\\'s good')");
        assert_eq!(statements[1], "INSERT INTO t VALUES ('path\\\\to\\\\file')");
    }

    #[test]
    fn test_string_with_double_quote_escape() {
        let sql =
            "INSERT INTO t VALUES ('it''s good');\nINSERT INTO t VALUES ('quote''test''here');";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "INSERT INTO t VALUES ('it''s good')");
        assert_eq!(statements[1], "INSERT INTO t VALUES ('quote''test''here')");
    }

    #[test]
    fn test_mixed_escapes() {
        let sql = "INSERT INTO t VALUES ('test\\'s', 'he''s', 'path\\\\x');\nSELECT * FROM t;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("'test\\'s'"));
        assert!(statements[0].contains("'he''s'"));
        assert!(statements[0].contains("'path\\\\x'"));
    }

    #[test]
    fn test_line_comments() {
        let sql = "-- This is a comment\nSELECT * FROM users; -- inline comment\n-- Another comment\nINSERT INTO t VALUES (1);";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("SELECT * FROM users"));
        assert!(statements[1].contains("INSERT INTO t VALUES (1)"));
    }

    #[test]
    fn test_mysql_hash_comments() {
        let sql = "# MySQL comment\nSELECT * FROM users; # inline\nINSERT INTO t VALUES (1);";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_block_comments() {
        let sql = "/* This is a block comment */\nSELECT * FROM users; /* inline */ DELETE FROM t;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("SELECT"));
        assert!(statements[1].contains("DELETE"));
    }

    #[test]
    fn test_delimiter_change() {
        let sql =
            "DELIMITER $$\nCREATE PROCEDURE p() BEGIN SELECT 1; END$$\nDELIMITER ;\nSELECT 2;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE PROCEDURE"));
        assert!(statements[0].contains("BEGIN"));
        assert!(statements[0].contains("END"));
        assert_eq!(statements[1], "SELECT 2");
    }

    #[test]
    fn test_begin_end_block() {
        let sql = "BEGIN\n  SELECT 1;\n  SELECT 2;\nEND;\nSELECT 3;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("BEGIN"));
        assert!(statements[0].contains("END"));
        assert_eq!(statements[1], "SELECT 3");
    }

    #[test]
    fn test_nested_parentheses() {
        let sql = "SELECT * FROM t WHERE id IN (SELECT id FROM u WHERE (status = 1 AND (flag = 0)));\nINSERT INTO t VALUES (1);";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("IN (SELECT"));
    }

    #[test]
    fn test_postgresql_dollar_quote() {
        let sql = "CREATE FUNCTION f() RETURNS void AS $$\nBEGIN\n  SELECT 'test;here';\nEND;\n$$ LANGUAGE plpgsql;\nSELECT 1;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::PostgreSQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("$$"));
        assert!(statements[0].contains("'test;here'"));
        assert_eq!(statements[1], "SELECT 1");
    }

    #[test]
    fn test_postgresql_tagged_dollar_quote() {
        let sql = "CREATE FUNCTION f() RETURNS text AS $body$\nSELECT 'test; with semicolon';\n$body$ LANGUAGE sql;\nSELECT 2;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::PostgreSQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("$body$"));
        assert!(statements[0].contains("semicolon"));
    }

    #[test]
    fn test_mssql_go_separator() {
        let sql = "CREATE TABLE t (id INT);\nGO\nINSERT INTO t VALUES (1);\nGO\nSELECT * FROM t;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MSSQL);

        assert_eq!(statements.len(), 3);
        assert!(statements[0].contains("CREATE TABLE"));
        assert!(statements[1].contains("INSERT"));
        assert!(statements[2].contains("SELECT"));
    }

    #[test]
    fn test_oracle_slash_separator() {
        let sql = "CREATE TABLE t (id NUMBER);\n/\nINSERT INTO t VALUES (1);\n/\nSELECT * FROM t;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::Oracle);

        assert!(statements.len() >= 2);
        assert!(statements[0].contains("CREATE TABLE"));
    }

    #[test]
    fn test_unicode_content() {
        let sql = "INSERT INTO t VALUES ('中文测试');\nINSERT INTO t VALUES ('日本語');\nINSERT INTO t VALUES ('한글');";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 3);
        assert!(statements[0].contains("中文测试"));
        assert!(statements[1].contains("日本語"));
        assert!(statements[2].contains("한글"));
    }

    #[test]
    fn test_unicode_with_escapes() {
        let sql = "INSERT INTO t VALUES ('测试\\'引号');\nINSERT INTO t VALUES ('test''测试');";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("测试\\'引号"));
        assert!(statements[1].contains("test''测试"));
    }

    #[test]
    fn test_multiline_statement() {
        let sql = "INSERT INTO users (\n  id,\n  name,\n  email\n)\nVALUES (\n  1,\n  'test',\n  'test@example.com'\n);\nSELECT 1;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("INSERT INTO users"));
        assert!(statements[0].contains("test@example.com"));
    }

    #[test]
    fn test_empty_statements() {
        let sql = ";;;\nSELECT 1;\n;\n;;SELECT 2;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "SELECT 1");
        assert_eq!(statements[1], "SELECT 2");
    }

    #[test]
    fn test_complex_real_world_dump() {
        let sql = r#"
-- MySQL dump example
DROP TABLE IF EXISTS `users`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
CREATE TABLE `users` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `name` varchar(100) DEFAULT NULL,
  `email` varchar(100) DEFAULT 'test@example.com',
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

LOCK TABLES `users` WRITE;
INSERT INTO `users` VALUES (1,'O\'Reilly','test@mail.com'),(2,'It''s fine','user@test.com');
UNLOCK TABLES;
"#;
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert!(statements.len() >= 5);
        assert!(statements.iter().any(|s| s.contains("DROP TABLE")));
        assert!(statements.iter().any(|s| s.contains("CREATE TABLE")));
        assert!(statements.iter().any(|s| s.contains("O\\'Reilly")));
        assert!(statements.iter().any(|s| s.contains("It''s fine")));
    }

    #[test]
    fn test_string_with_semicolon_inside() {
        let sql = "INSERT INTO t VALUES ('SELECT * FROM users; DELETE FROM t;');\nSELECT 1;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("SELECT * FROM users; DELETE FROM t;"));
        assert_eq!(statements[1], "SELECT 1");
    }

    #[test]
    fn test_backtick_identifiers() {
        let sql = "SELECT `id`, `name` FROM `users`;\nINSERT INTO `table` VALUES (1, 'test;here');";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("`id`"));
        assert!(statements[1].contains("'test;here'"));
    }

    #[test]
    fn test_double_quote_identifiers() {
        let sql = r#"SELECT "id", "name" FROM "users";"#;
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::PostgreSQL);

        assert_eq!(statements.len(), 1);
        assert!(statements[0].contains(r#""id""#));
    }

    #[test]
    fn test_mixed_quotes() {
        let sql = r#"SELECT 'single', "double", `backtick` FROM t WHERE x = 'it''s' AND y = "col""name";"#;
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 1);
        assert!(statements[0].contains("'single'"));
        assert!(statements[0].contains("'it''s'"));
    }

    #[test]
    fn test_progress_tracking() {
        let sql = "SELECT 1;\nSELECT 2;\nSELECT 3;";
        let mut parser = StreamingSqlParser::from_source(
            SqlSource::Script(sql.to_string()),
            DatabaseType::MySQL,
        )
        .unwrap();

        assert_eq!(parser.progress_percent(), 0.0);

        let _ = parser.next();
        assert!(parser.progress_percent() > 0.0);
        assert!(parser.progress_percent() < 100.0);

        while parser.next().is_some() {}
        assert_eq!(parser.progress_percent(), 100.0);
    }

    #[test]
    fn test_pure_comment_after_statement() {
        // 测试语句后跟纯注释的情况，纯注释不应该被当作独立语句
        let sql = "SELECT id, username, create_by FROM login_user; -- ❌ 列不存在";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 1);
        assert_eq!(
            statements[0],
            "SELECT id, username, create_by FROM login_user"
        );
    }

    #[test]
    fn test_pure_comment_only() {
        // 测试只有纯注释的情况
        let sql = "-- 这是一个注释";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 0);
    }

    #[test]
    fn test_multiple_pure_comments() {
        // 测试多个纯注释
        let sql = "-- 注释1\n-- 注释2\n/* 块注释 */";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 0);
    }

    #[test]
    fn test_mixed_comments_and_statements() {
        // 测试混合场景
        let sql = "SELECT 1; -- 注释\n-- 纯注释\nSELECT 2; /* 行尾注释 */";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("SELECT 1"));
        assert!(statements[1].contains("SELECT 2"));
    }

    #[test]
    fn test_nested_begin_end() {
        let sql = "BEGIN\n  BEGIN\n    SELECT 1;\n  END;\n  SELECT 2;\nEND;\nSELECT 3;";
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("BEGIN"));
        assert!(statements[0].contains("SELECT 1"));
        assert!(statements[0].contains("SELECT 2"));
    }

    #[test]
    fn test_create_procedure_with_complex_body() {
        let sql = r#"DELIMITER $$
CREATE PROCEDURE complex_proc(IN param INT)
BEGIN
    DECLARE var VARCHAR(100);
    SET var = 'test;value';

    -- Comment with semicolon;
    IF param > 0 THEN
        SELECT * FROM users WHERE name = 'O''Reilly';
    ELSE
        INSERT INTO log VALUES ('Error; occurred');
    END IF;
END$$
DELIMITER ;
SELECT 'done';"#;
        let statements = parse_all(SqlSource::Script(sql.to_string()), DatabaseType::MySQL);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE PROCEDURE"));
        assert!(statements[0].contains("'test;value'"));
        assert!(statements[0].contains("'O''Reilly'"));
        assert_eq!(statements[1], "SELECT 'done'");
    }
}
