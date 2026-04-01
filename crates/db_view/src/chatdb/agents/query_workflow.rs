//! 查询工作流 - SQL 生成相关的数据类型和工具函数
//!
//! 提供 @表名 解析、AI 选表 prompt 生成、SQL 生成 prompt 等核心能力。
//! 被 SqlWorkflowAgent 和 ChatPanel 共同使用。

use one_core::llm::{Message, Role};
use one_core::storage::DatabaseType;
use regex::Regex;
use rust_i18n::t;
use std::sync::LazyLock;

// ============================================================================
// 配置常量
// ============================================================================

/// 表数量阈值，超过此值时显示警告
pub const TABLE_COUNT_THRESHOLD: usize = 30;

/// 匹配 JSON 数组的正则（模块级共享）
static JSON_ARRAY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\[[\s\S]*?\]"#).unwrap());

// ============================================================================
// 数据结构
// ============================================================================

/// 选表来源
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableSelectionSource {
    /// 用户主动 @表名
    UserMentioned,
    /// AI 分析选表
    AiSelected,
    /// 从对话历史中复用
    HistoryReused,
}

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
    /// 选表来源
    pub table_source: TableSelectionSource,
    /// 警告信息（如表数量超过阈值）
    pub warning: Option<String>,
}

impl QueryContext {
    /// 生成工作流摘要（用于显示和存储）
    pub fn to_workflow_summary(&self) -> String {
        let mut summary = String::new();

        // 警告信息
        if let Some(warning) = &self.warning {
            summary.push_str(&t!("QueryWorkflow.warning_block", warning = warning).to_string());
        }

        // 选择的表
        if !self.selected_table_names.is_empty() {
            let source = match self.table_source {
                TableSelectionSource::UserMentioned => t!("QueryWorkflow.source_user").to_string(),
                TableSelectionSource::AiSelected => t!("QueryWorkflow.source_ai").to_string(),
                TableSelectionSource::HistoryReused => {
                    t!("QueryWorkflow.source_history").to_string()
                }
            };
            summary
                .push_str(&t!("QueryWorkflow.related_tables_header", source = source).to_string());
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
        let mut prompt = t!("QueryWorkflow.sql_prompt_intro", db_type = db_type).to_string();

        prompt.push_str(t!("QueryWorkflow.sql_prompt_tables_header").as_ref());

        for table in &self.tables {
            prompt.push_str(
                &t!("QueryWorkflow.sql_prompt_table_header", table = table.name).to_string(),
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
///
/// `chat_history` 用于在追问场景下让 AI 感知之前的对话上下文，从而选择与先前一致的表。
pub fn build_table_selection_prompt(
    tables: &[TableBrief],
    user_question: &str,
    chat_history: &[Message],
) -> String {
    let mut prompt = t!("QueryWorkflow.table_select_intro").to_string();

    prompt.push_str(t!("QueryWorkflow.table_select_tables_header").as_ref());
    prompt.push_str(t!("QueryWorkflow.table_select_table_header_row").as_ref());
    prompt.push_str(t!("QueryWorkflow.table_select_table_divider").as_ref());

    for table in tables {
        let comment = table.comment.as_deref().unwrap_or("-");
        prompt.push_str(&format!("| `{}` | {} |\n", table.name, comment));
    }

    // 附加最近的对话历史，帮助 AI 在追问场景下选择正确的表
    let recent: Vec<&Message> = chat_history.iter().rev().take(4).collect::<Vec<_>>();
    if !recent.is_empty() {
        prompt.push_str("\n\n## 对话历史（最近几轮，用于理解上下文）\n\n");
        prompt.push_str("如果用户是在追问或补充上一个问题，请选择与之前对话相同的表。\n\n");
        for msg in recent.iter().rev() {
            let role_label = match msg.role {
                Role::User => "用户",
                Role::Assistant => "助手",
                _ => "系统",
            };
            let text = msg.content_as_text();
            if !text.is_empty() {
                // 截断过长的历史消息，避免 prompt 膨胀
                let truncated = if text.len() > 300 {
                    format!("{}...", &text[..text.floor_char_boundary(300)])
                } else {
                    text
                };
                prompt.push_str(&format!("**{}**: {}\n\n", role_label, truncated));
            }
        }
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
// 历史复用
// ============================================================================

/// 判断用户问题是否像追问（延续上一轮话题）
///
/// 通过检测常见追问关键词来判断。只有当检测到追问意图时，
/// 才应复用历史中的选表结果；否则应走 AI 选表。
pub fn is_followup_question(question: &str) -> bool {
    static FOLLOWUP_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)继续|再[查加改删建]|还[要需]|补充|加[上个]|修改|改[成为下]|上[面一]|刚才|之前|同样|同一|那[个张]表|这[个张]表|上述|前面|接着|然后|顺便|continue|also|additionally|same\s+table|previous|above|moreover"
        ).unwrap()
    });
    FOLLOWUP_RE.is_match(question)
}

/// 从对话历史中提取之前选过的表名
///
/// 查找最近一条 Assistant 消息中的 `**相关表**` 或 `**Related Tables**` 标记，
/// 并解析后续的 JSON 数组，返回已选表名列表。
pub fn extract_tables_from_history(chat_history: &[Message]) -> Option<Vec<String>> {
    // 匹配 "**相关表**" 或 "**Related Tables**" 标记
    static RELATED_TABLES_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\*\*相关表\*\*|\*\*Related Tables\*\*)").unwrap());

    // 从后往前找最近一条 Assistant 消息
    let last_assistant = chat_history
        .iter()
        .rev()
        .find(|m| m.role == Role::Assistant)?;

    let text = last_assistant.content_as_text();
    if text.is_empty() {
        return None;
    }

    // 检查是否包含相关表标记
    if !RELATED_TABLES_RE.is_match(&text) {
        return None;
    }

    // 提取标记之后的 JSON 数组
    let marker_end = RELATED_TABLES_RE.find(&text)?.end();
    let after_marker = &text[marker_end..];

    if let Some(json_match) = JSON_ARRAY_RE.find(after_marker) {
        let json_str = json_match.as_str();
        if let Ok(tables) = serde_json::from_str::<Vec<String>>(json_str) {
            if !tables.is_empty() {
                return Some(tables);
            }
        }
    }

    None
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

    // ---- extract_tables_from_history 测试 ----

    #[test]
    fn test_extract_tables_empty_history() {
        let history: Vec<Message> = vec![];
        assert_eq!(extract_tables_from_history(&history), None);
    }

    #[test]
    fn test_extract_tables_from_assistant_message() {
        let history = vec![
            Message::text(Role::User, "查询所有用户"),
            Message::text(
                Role::Assistant,
                "**相关表**（用户指定）：\n```json\n[\"login_user\", \"user_role\"]\n```\n\n这是SQL...",
            ),
        ];
        let result = extract_tables_from_history(&history);
        assert_eq!(
            result,
            Some(vec!["login_user".to_string(), "user_role".to_string()])
        );
    }

    #[test]
    fn test_extract_tables_multi_turn_uses_latest() {
        let history = vec![
            Message::text(Role::User, "查询用户"),
            Message::text(
                Role::Assistant,
                "**相关表**（AI 分析）：\n```json\n[\"old_table\"]\n```\n\nSQL...",
            ),
            Message::text(Role::User, "查询订单"),
            Message::text(
                Role::Assistant,
                "**相关表**（AI 分析）：\n```json\n[\"orders\", \"order_items\"]\n```\n\nSQL...",
            ),
        ];
        let result = extract_tables_from_history(&history);
        assert_eq!(
            result,
            Some(vec!["orders".to_string(), "order_items".to_string()])
        );
    }

    #[test]
    fn test_extract_tables_english_marker() {
        let history = vec![Message::text(
            Role::Assistant,
            "**Related Tables** (User specified):\n```json\n[\"users\"]\n```\n",
        )];
        let result = extract_tables_from_history(&history);
        assert_eq!(result, Some(vec!["users".to_string()]));
    }

    #[test]
    fn test_extract_tables_no_table_info() {
        let history = vec![Message::text(
            Role::Assistant,
            "这是一段普通回复，没有表信息。",
        )];
        let result = extract_tables_from_history(&history);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_tables_user_mentioned_source() {
        // 即使来源是"用户指定"，也应该能提取表名
        let history = vec![Message::text(
            Role::Assistant,
            "**相关表**（用户指定）：\n```json\n[\"my_table\"]\n```\n",
        )];
        let result = extract_tables_from_history(&history);
        assert_eq!(result, Some(vec!["my_table".to_string()]));
    }

    // ---- is_followup_question 测试 ----

    #[test]
    fn test_followup_question_chinese_keywords() {
        assert!(is_followup_question("继续新增备注字段"));
        assert!(is_followup_question("再查一下年龄分布"));
        assert!(is_followup_question("还要加上排序"));
        assert!(is_followup_question("修改上面的SQL"));
        assert!(is_followup_question("接着加个索引"));
        assert!(is_followup_question("补充一个条件"));
    }

    #[test]
    fn test_followup_question_english_keywords() {
        assert!(is_followup_question("continue with the same query"));
        assert!(is_followup_question("also add a WHERE clause"));
        assert!(is_followup_question("additionally filter by date"));
    }

    #[test]
    fn test_not_followup_new_topic() {
        assert!(!is_followup_question("帮我统计有多少报告"));
        assert!(!is_followup_question("查询所有用户"));
        assert!(!is_followup_question("统计订单金额"));
        assert!(!is_followup_question("show me the sales data"));
    }
}
