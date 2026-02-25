//! 查询工作流 - SQL 生成相关的数据类型和工具函数
//!
//! 提供 @表名 解析、AI 选表 prompt 生成、SQL 生成 prompt 等核心能力。
//! 被 SqlWorkflowAgent 和 ChatPanel 共同使用。

use one_core::storage::DatabaseType;
use regex::Regex;
use std::sync::LazyLock;
use rust_i18n::t;

// ============================================================================
// 配置常量
// ============================================================================

/// 表数量阈值，超过此值时显示警告
pub const TABLE_COUNT_THRESHOLD: usize = 30;

// ============================================================================
// 数据结构
// ============================================================================

/// 表元信息（用于AI选表阶段，只包含名称和注释）
#[derive(Clone, Debug)]
pub struct TableBrief {
    pub name: String,
    pub comment: Option<String>,
}

/// 表完整元信息（包含列信息）
#[derive(Clone, Debug)]
pub struct TableMeta {
    pub name: String,
    pub comment: Option<String>,
    pub columns: Vec<ColumnMeta>,
}

/// 列元信息
#[derive(Clone, Debug)]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub comment: Option<String>,
    pub is_primary_key: bool,
}

/// 查询上下文（传给AI生成SQL）
#[derive(Clone, Debug)]
pub struct QueryContext {
    pub user_question: String,
    pub database_type: DatabaseType,
    pub tables: Vec<TableMeta>,
    /// 选中的表名列表（用于显示）
    pub selected_table_names: Vec<String>,
    /// 是否用户主动@表
    pub is_user_mentioned: bool,
    /// 警告信息（如表数量超过阈值）
    pub warning: Option<String>,
}

impl QueryContext {
    /// 生成工作流摘要（用于显示和存储）
    pub fn to_workflow_summary(&self) -> String {
        let mut summary = String::new();

        // 警告信息
        if let Some(warning) = &self.warning {
            summary.push_str(
                &t!("QueryWorkflow.warning_block", warning = warning).to_string()
            );
        }

        // 选择的表
        if !self.selected_table_names.is_empty() {
            let source = if self.is_user_mentioned {
                t!("QueryWorkflow.source_user").to_string()
            } else {
                t!("QueryWorkflow.source_ai").to_string()
            };
            summary.push_str(
                &t!(
                    "QueryWorkflow.related_tables_header",
                    source = source
                )
                .to_string()
            );
            summary.push_str("```json\n");
            let json_array = serde_json::to_string(&self.selected_table_names)
                .unwrap_or_else(|_| format!("{:?}", self.selected_table_names));
            summary.push_str(&json_array);
            summary.push_str("\n```\n\n");
        }

        summary
    }
}

/// 解析后的用户输入
#[derive(Clone, Debug)]
pub struct ParsedInput {
    /// 原始问题
    pub raw_question: String,
    /// 清理后的问题（移除@表标记）
    pub clean_question: String,
    /// @的表名列表
    pub mentioned_tables: Vec<String>,
}

// ============================================================================
// 输入解析
// ============================================================================

/// 解析用户输入中的@表标记
/// 支持格式：
/// - @table_name （支持中英文、数字、下划线）
/// - @`表名` （反引号内可包含中文和空格）
/// - @"表名" （双引号内可包含中文和空格）
pub fn parse_user_input(input: &str) -> ParsedInput {
    static TABLE_MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"@(?:`([^`]+)`|"([^"]+)"|([\p{L}_][\p{L}\p{N}_]*))"#).unwrap()
    });

    let mut mentioned_tables = Vec::new();
    let mut clean_question = input.to_string();

    for cap in TABLE_MENTION_RE.captures_iter(input) {
        let table_name: Option<String> = cap
            .get(1)
            .or_else(|| cap.get(2))
            .or_else(|| cap.get(3))
            .map(|m: regex::Match<'_>| m.as_str().to_string());

        if let Some(name) = table_name {
            mentioned_tables.push(name);
        }

        if let Some(full_match) = cap.get(0) {
            let match_str: &str = full_match.as_str();
            clean_question = clean_question.replace(match_str, "");
        }
    }

    let clean_question = clean_question
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    ParsedInput {
        raw_question: input.to_string(),
        clean_question,
        mentioned_tables,
    }
}

// ============================================================================
// Prompt 生成
// ============================================================================

impl QueryContext {
    /// 生成SQL生成的system prompt
    pub fn to_sql_generation_prompt(&self) -> String {
        let db_type = format!("{:?}", self.database_type);
        let mut prompt =
            t!("QueryWorkflow.sql_prompt_intro", db_type = db_type).to_string();

        prompt.push_str(t!("QueryWorkflow.sql_prompt_tables_header").as_ref());

        for table in &self.tables {
            prompt.push_str(
                &t!("QueryWorkflow.sql_prompt_table_header", table = table.name).to_string()
            );
            if let Some(comment) = &table.comment {
                if !comment.is_empty() {
                    prompt.push_str(&format!(" - {}", comment));
                }
            }
            prompt.push_str("\n\n");

            if table.columns.is_empty() {
                prompt.push_str(t!("QueryWorkflow.sql_prompt_no_columns").as_ref());
                continue;
            }

            prompt.push_str(t!("QueryWorkflow.sql_prompt_columns_header").as_ref());
            prompt.push_str(t!("QueryWorkflow.sql_prompt_columns_divider").as_ref());

            for col in &table.columns {
                prompt.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} |\n",
                    col.name,
                    col.data_type,
                    if col.nullable {
                        t!("Common.yes")
                    } else {
                        t!("Common.no")
                    },
                    if col.is_primary_key { "🔑" } else { "" },
                    col.comment.as_deref().unwrap_or("-")
                ));
            }
            prompt.push_str("\n");
        }

        prompt.push_str(t!("QueryWorkflow.sql_prompt_requirements_header").as_ref());
        prompt.push_str(t!("QueryWorkflow.sql_prompt_requirement_1").as_ref());
        prompt.push_str(t!("QueryWorkflow.sql_prompt_requirement_2").as_ref());
        prompt.push_str(t!("QueryWorkflow.sql_prompt_requirement_3").as_ref());
        prompt.push_str(t!("QueryWorkflow.sql_prompt_requirement_4").as_ref());

        prompt
    }
}

/// 生成AI选表的prompt
pub fn build_table_selection_prompt(tables: &[TableBrief], user_question: &str) -> String {
    let mut prompt = t!("QueryWorkflow.table_select_intro").to_string();

    prompt.push_str(t!("QueryWorkflow.table_select_tables_header").as_ref());
    prompt.push_str(t!("QueryWorkflow.table_select_table_header_row").as_ref());
    prompt.push_str(t!("QueryWorkflow.table_select_table_divider").as_ref());

    for table in tables {
        let comment = table.comment.as_deref().unwrap_or("-");
        prompt.push_str(&format!("| `{}` | {} |\n", table.name, comment));
    }

    prompt.push_str(t!("QueryWorkflow.table_select_user_question_header").as_ref());
    prompt.push_str(user_question);

    prompt.push_str(t!("QueryWorkflow.table_select_requirements_header").as_ref());
    prompt.push_str(t!("QueryWorkflow.table_select_requirement_1").as_ref());
    prompt.push_str(t!("QueryWorkflow.table_select_requirement_2").as_ref());
    prompt.push_str(t!("QueryWorkflow.table_select_example").as_ref());

    prompt
}

/// 解析AI选表的响应
pub fn parse_table_selection_response(response: &str) -> Vec<String> {
    static JSON_ARRAY_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"\[[\s\S]*?\]"#).unwrap());

    if let Some(json_match) = JSON_ARRAY_RE.find(response) {
        let json_str: &str = json_match.as_str();
        if let Ok(tables) = serde_json::from_str::<Vec<String>>(json_str) {
            return tables;
        }
    }

    // 备选：尝试提取反引号中的表名
    static BACKTICK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`]+)`").unwrap());

    BACKTICK_RE
        .captures_iter(response)
        .filter_map(|cap: regex::Captures<'_>| {
            cap.get(1).map(|m: regex::Match<'_>| m.as_str().to_string())
        })
        .collect()
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_input_simple() {
        let input = "查询 @users 表中的所有用户";
        let result = parse_user_input(input);
        assert_eq!(result.mentioned_tables, vec!["users"]);
        assert_eq!(result.clean_question, "查询 表中的所有用户");
    }

    #[test]
    fn test_parse_user_input_multiple() {
        let input = "统计 @orders 和 @order_items 的销售额";
        let result = parse_user_input(input);
        assert_eq!(result.mentioned_tables, vec!["orders", "order_items"]);
    }

    #[test]
    fn test_parse_user_input_backtick() {
        let input = "查询 @`user info` 表";
        let result = parse_user_input(input);
        assert_eq!(result.mentioned_tables, vec!["user info"]);
    }

    #[test]
    fn test_parse_user_input_quoted() {
        let input = r#"查询 @"order details" 表"#;
        let result = parse_user_input(input);
        assert_eq!(result.mentioned_tables, vec!["order details"]);
    }

    #[test]
    fn test_parse_user_input_chinese_unquoted() {
        let input = "查询 @用户表 的数据";
        let result = parse_user_input(input);
        assert_eq!(result.mentioned_tables, vec!["用户表"]);
        assert_eq!(result.clean_question, "查询 的数据");
    }

    #[test]
    fn test_parse_user_input_no_mention() {
        let input = "查询所有用户";
        let result = parse_user_input(input);
        assert!(result.mentioned_tables.is_empty());
        assert_eq!(result.clean_question, "查询所有用户");
    }

    #[test]
    fn test_parse_table_selection_response_json() {
        let response = r#"根据问题，相关的表是：
```json
["users", "orders"]
```"#;
        let tables = parse_table_selection_response(response);
        assert_eq!(tables, vec!["users", "orders"]);
    }

    #[test]
    fn test_parse_table_selection_response_backticks() {
        let response = "相关的表是 `users` 和 `orders`";
        let tables = parse_table_selection_response(response);
        assert_eq!(tables, vec!["users", "orders"]);
    }
}
