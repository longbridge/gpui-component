//! 查询工作流 - 智能SQL生成的多阶段工作流
//!
//! 工作流程：
//! 1. 解析用户输入中的 @表名
//! 2. 根据是否有@表和表数量决定流程：
//!    - 有@表：直接获取元数据 → AI生成SQL
//!    - 无@表且表数 ≤ 阈值：AI选表 → 获取元数据 → AI生成SQL
//!    - 无@表且表数 > 阈值：提示用户@表

use db::GlobalDbState;
use gpui::AsyncApp;
use one_core::storage::DatabaseType;
use regex::Regex;
use std::sync::LazyLock;

// ============================================================================
// 配置常量
// ============================================================================

/// 表数量阈值，超过此值需要用户手动@表
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
            summary.push_str(&format!("> ⚠️ {}\n\n", warning));
        }

        // 选择的表
        if !self.selected_table_names.is_empty() {
            let source = if self.is_user_mentioned {
                "用户指定"
            } else {
                "AI 分析"
            };
            summary.push_str(&format!("**相关表**（{}）：\n", source));
            summary.push_str("```json\n");
            let json_array = serde_json::to_string(&self.selected_table_names)
                .unwrap_or_else(|_| format!("{:?}", self.selected_table_names));
            summary.push_str(&json_array);
            summary.push_str("\n```\n\n");
        }

        summary
    }
}

/// 工作流阶段
#[derive(Clone, Debug)]
pub enum WorkflowStage {
    /// 空闲
    Idle,
    /// 获取表列表中
    FetchingTables,
    /// AI选表中
    SelectingTables {
        user_question: String,
        tables: Vec<TableBrief>,
    },
    /// 获取元数据中
    FetchingMetadata {
        user_question: String,
        selected_tables: Vec<String>,
    },
    /// AI生成SQL中
    GeneratingSql {
        context: QueryContext,
    },
    /// 需要用户@表
    NeedUserSelectTables {
        user_question: String,
        table_count: usize,
    },
}

/// 工作流结果
#[derive(Clone, Debug)]
pub enum WorkflowAction {
    /// 继续下一阶段
    Continue(WorkflowStage),
    /// 需要AI选表（返回选表prompt）
    NeedAiSelectTables {
        prompt: String,
        tables: Vec<TableBrief>,
        /// 可选的警告信息（比如表数量超过阈值时）
        warning: Option<String>,
    },
    /// 准备好生成SQL（返回完整上下文）
    ReadyToGenerate {
        context: QueryContext,
    },
    /// 需要用户手动@表（目前保留但不使用）
    RequireUserMention {
        message: String,
        table_count: usize,
    },
    /// 错误
    Error(String),
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
        // @`表名` 或 @"表名" 可以包含任意字符
        // @table_name 支持中英文、数字、下划线
        Regex::new(r#"@(?:`([^`]+)`|"([^"]+)"|([\p{L}_][\p{L}\p{N}_]*))"#).unwrap()
    });

    let mut mentioned_tables = Vec::new();
    let mut clean_question = input.to_string();

    for cap in TABLE_MENTION_RE.captures_iter(input) {
        // 提取表名（可能在不同的捕获组中）
        let table_name: Option<String> = cap
            .get(1)
            .or_else(|| cap.get(2))
            .or_else(|| cap.get(3))
            .map(|m: regex::Match<'_>| m.as_str().to_string());

        if let Some(name) = table_name {
            mentioned_tables.push(name);
        }

        // 从清理后的问题中移除@标记
        if let Some(full_match) = cap.get(0) {
            let match_str: &str = full_match.as_str();
            clean_question = clean_question.replace(match_str, "");
        }
    }

    // 清理多余空格
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
        let mut prompt = format!(
            "你是一个 {db_type} 数据库专家。请根据用户问题生成准确的 SQL 查询语句。\n\n"
        );

        prompt.push_str("## 可用表结构\n\n");

        for table in &self.tables {
            prompt.push_str(&format!("### 表: `{}`", table.name));
            if let Some(comment) = &table.comment {
                if !comment.is_empty() {
                    prompt.push_str(&format!(" - {}", comment));
                }
            }
            prompt.push_str("\n\n");

            if table.columns.is_empty() {
                prompt.push_str("（列信息暂无）\n\n");
                continue;
            }

            prompt.push_str("| 列名 | 类型 | 可空 | 主键 | 说明 |\n");
            prompt.push_str("|------|------|------|------|------|\n");

            for col in &table.columns {
                prompt.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} |\n",
                    col.name,
                    col.data_type,
                    if col.nullable { "是" } else { "否" },
                    if col.is_primary_key { "🔑" } else { "" },
                    col.comment.as_deref().unwrap_or("-")
                ));
            }
            prompt.push_str("\n");
        }

        prompt.push_str("## 要求\n\n");
        prompt.push_str("1. 生成的 SQL 必须放在 ```sql 代码块中\n");
        prompt.push_str("2. 只使用上述表中存在的列名\n");
        prompt.push_str("3. 简要解释 SQL 的作用和逻辑\n");
        prompt.push_str("4. 如果需要多个查询，分别用代码块包裹\n");

        prompt
    }
}

/// 生成AI选表的prompt
pub fn build_table_selection_prompt(tables: &[TableBrief], user_question: &str) -> String {
    let mut prompt = String::from(
        "你是一个数据库专家。根据用户的问题，从以下表列表中选择最相关的表。\n\n",
    );

    prompt.push_str("## 可用表\n\n");
    prompt.push_str("| 表名 | 说明 |\n");
    prompt.push_str("|------|------|\n");

    for table in tables {
        let comment = table.comment.as_deref().unwrap_or("-");
        prompt.push_str(&format!("| `{}` | {} |\n", table.name, comment));
    }

    prompt.push_str("\n## 用户问题\n\n");
    prompt.push_str(user_question);

    prompt.push_str("\n\n## 要求\n\n");
    prompt.push_str("请返回一个 JSON 数组，包含你认为与问题相关的表名。\n");
    prompt.push_str("只返回 JSON，不要其他解释。\n\n");
    prompt.push_str("示例格式：\n```json\n[\"table1\", \"table2\"]\n```\n");

    prompt
}

/// 解析AI选表的响应
pub fn parse_table_selection_response(response: &str) -> Vec<String> {
    // 尝试从响应中提取JSON数组
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
// 工作流执行器
// ============================================================================

/// 查询工作流执行器
pub struct QueryWorkflow {
    /// 数据库连接ID
    connection_id: String,
    /// 数据库名
    database: Option<String>,
    /// Schema名
    schema: Option<String>,
    /// 数据库类型
    database_type: DatabaseType,
    /// 表数量阈值
    threshold: usize,
}

impl QueryWorkflow {
    pub fn new(
        connection_id: String,
        database: Option<String>,
        schema: Option<String>,
        database_type: DatabaseType,
    ) -> Self {
        Self {
            connection_id,
            database,
            schema,
            database_type,
            threshold: TABLE_COUNT_THRESHOLD,
        }
    }

    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    /// 启动工作流
    pub async fn start(
        &self,
        input: &ParsedInput,
        global_state: &GlobalDbState,
        cx: &mut AsyncApp,
    ) -> WorkflowAction {
        // 情况1：用户已@表
        if !input.mentioned_tables.is_empty() {
            return self
                .fetch_metadata_and_prepare(
                    &input.clean_question,
                    &input.mentioned_tables,
                    true,  // is_user_mentioned
                    None,  // no warning
                    global_state,
                    cx,
                )
                .await;
        }

        // 情况2：未@表，先获取表列表
        let tables = match self.fetch_table_list(global_state, cx).await {
            Ok(t) => t,
            Err(e) => return WorkflowAction::Error(e),
        };

        let table_count = tables.len();

        // 情况2a：表数量超过阈值，显示警告但继续AI选表
        let warning = if table_count > self.threshold {
            Some(format!(
                "表数量大于{}，AI输出结果可能不准确，可以使用@表功能直接针对目标表进行问答",
                self.threshold
            ))
        } else {
            None
        };

        // 情况2b：需要AI选表
        WorkflowAction::NeedAiSelectTables {
            prompt: build_table_selection_prompt(&tables, &input.clean_question),
            tables,
            warning,
        }
    }

    /// 处理AI选表的结果
    pub async fn handle_table_selection(
        &self,
        user_question: &str,
        ai_response: &str,
        warning: Option<String>,
        global_state: &GlobalDbState,
        cx: &mut AsyncApp,
    ) -> WorkflowAction {
        let selected_tables = parse_table_selection_response(ai_response);

        if selected_tables.is_empty() {
            return WorkflowAction::Error("AI未能选择任何相关表".to_string());
        }

        self.fetch_metadata_and_prepare(
            user_question,
            &selected_tables,
            false, // is_user_mentioned
            warning,
            global_state,
            cx,
        )
        .await
    }

    /// 获取表列表（只有表名和注释）
    async fn fetch_table_list(
        &self,
        global_state: &GlobalDbState,
        cx: &mut AsyncApp,
    ) -> Result<Vec<TableBrief>, String> {
        let tables = global_state
            .list_tables(
                cx,
                self.connection_id.clone(),
                self.database.clone().unwrap_or_default(),
                self.schema.clone(),
            )
            .await
            .map_err(|e| format!("获取表列表失败: {}", e))?;

        Ok(tables
            .into_iter()
            .map(|t| TableBrief {
                name: t.name,
                comment: t.comment,
            })
            .collect())
    }

    /// 获取元数据并准备生成上下文
    async fn fetch_metadata_and_prepare(
        &self,
        user_question: &str,
        table_names: &[String],
        is_user_mentioned: bool,
        warning: Option<String>,
        global_state: &GlobalDbState,
        cx: &mut AsyncApp,
    ) -> WorkflowAction {
        let mut table_metas = Vec::new();

        for table_name in table_names {
            match self
                .fetch_table_metadata(table_name, global_state, cx)
                .await
            {
                Ok(meta) => table_metas.push(meta),
                Err(e) => {
                    // 表不存在时记录警告但继续
                    tracing::warn!("获取表 {} 元数据失败: {}", table_name, e);
                }
            }
        }

        if table_metas.is_empty() {
            return WorkflowAction::Error("未能获取任何表的元数据".to_string());
        }

        let context = QueryContext {
            user_question: user_question.to_string(),
            database_type: self.database_type,
            tables: table_metas,
            selected_table_names: table_names.to_vec(),
            is_user_mentioned,
            warning,
        };

        WorkflowAction::ReadyToGenerate { context }
    }

    /// 获取单个表的完整元数据
    async fn fetch_table_metadata(
        &self,
        table_name: &str,
        global_state: &GlobalDbState,
        cx: &mut AsyncApp,
    ) -> Result<TableMeta, String> {
        let columns = global_state
            .list_columns(
                cx,
                self.connection_id.clone(),
                self.database.clone().unwrap_or_default(),
                self.schema.clone(),
                table_name.to_string(),
            )
            .await
            .map_err(|e| format!("获取列信息失败: {}", e))?;

        Ok(TableMeta {
            name: table_name.to_string(),
            comment: None, // 可以从表列表中获取
            columns: columns
                .into_iter()
                .map(|c| ColumnMeta {
                    name: c.name,
                    data_type: c.data_type,
                    nullable: c.is_nullable,
                    comment: c.comment,
                    is_primary_key: c.is_primary_key,
                })
                .collect(),
        })
    }
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
