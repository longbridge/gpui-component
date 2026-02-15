//! 聊天 Markdown 解析器 - 复用 TextView 的 Markdown 解析结果

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use gpui_component::highlighter::HighlightTheme;
use gpui_component::text::{CodeBlock, MarkdownCodeBlock, Span, parse_markdown_code_blocks};

#[derive(Clone, Debug)]
pub struct SqlCodeBlock {
    pub key: usize,
    pub code: String,
    pub language: Option<String>,
    pub is_sql: bool,
}

impl SqlCodeBlock {
    pub fn from_code_block(code_block: &CodeBlock, fallback_index: usize) -> Self {
        let code = code_block.code().to_string();
        let language = code_block.lang().map(|lang| lang.to_string());
        let is_sql = is_sql_language(language.as_deref()) || is_sql_statement(&code);
        let key = sql_block_key(code_block.span, &code, fallback_index);

        Self {
            key,
            code,
            language,
            is_sql,
        }
    }

    fn from_markdown_block(block: MarkdownCodeBlock) -> Self {
        let code = block.code.to_string();
        let language = block.language.map(|lang| lang.to_string());
        let is_sql = is_sql_language(language.as_deref()) || is_sql_statement(&code);
        let key = sql_block_key(block.span, &code, block.index);

        Self {
            key,
            code,
            language,
            is_sql,
        }
    }
}

pub fn parse_sql_code_blocks(source: &str, highlight_theme: &HighlightTheme) -> Vec<SqlCodeBlock> {
    let blocks = parse_markdown_code_blocks(source, highlight_theme).unwrap_or_default();
    blocks
        .into_iter()
        .map(SqlCodeBlock::from_markdown_block)
        .collect()
}

fn sql_block_key(span: Option<Span>, code: &str, fallback_index: usize) -> usize {
    if let Some(span) = span {
        return span.start;
    }
    if fallback_index > 0 {
        return fallback_index;
    }

    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    hasher.finish() as usize
}

fn is_sql_language(language: Option<&str>) -> bool {
    let Some(language) = language else {
        return false;
    };
    let lang = language.to_lowercase();
    matches!(
        lang.as_str(),
        "sql" | "mysql" | "postgresql" | "sqlite" | "mssql" | "oracle"
    )
}

fn is_sql_statement(code: &str) -> bool {
    let mut first_token: Option<String> = None;
    for line in code.lines() {
        let line = line.trim_start();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("--") || line.starts_with('#') || line.starts_with("/*") {
            continue;
        }
        if let Some(token) = line.split_whitespace().next() {
            let cleaned = token.trim_matches(|c: char| !c.is_alphanumeric());
            if !cleaned.is_empty() {
                first_token = Some(cleaned.to_uppercase());
                break;
            }
        }
    }

    let Some(token) = first_token else {
        return false;
    };

    matches!(
        token.as_str(),
        "SELECT"
            | "WITH"
            | "SHOW"
            | "DESCRIBE"
            | "DESC"
            | "EXPLAIN"
            | "INSERT"
            | "UPDATE"
            | "DELETE"
            | "CREATE"
            | "ALTER"
            | "DROP"
            | "TRUNCATE"
            | "REPLACE"
            | "MERGE"
            | "CALL"
            | "EXEC"
            | "EXECUTE"
            | "GRANT"
            | "REVOKE"
            | "USE"
            | "SET"
    )
}
