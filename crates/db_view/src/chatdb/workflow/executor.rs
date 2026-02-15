//! WorkflowExecutor - 工作流执行器
//!
//! 执行智能 SQL 生成的多阶段工作流

use db::GlobalDbState;
use gpui::AsyncApp;
use one_core::storage::DatabaseType;
use tokio_util::sync::CancellationToken;

use super::{
    build_table_selection_prompt, ColumnMeta, ParsedInput, QueryContext, TableBrief, TableMeta,
    WorkflowAction, TABLE_COUNT_THRESHOLD,
};

// ============================================================================
// 错误类型
// ============================================================================

/// 工作流执行错误
#[derive(Debug, Clone)]
pub enum WorkflowError {
    /// 获取表列表失败
    FetchTablesError(String),
    /// 获取元数据失败
    MetadataError(String),
    /// 没有获取到任何元数据
    NoMetadata,
    /// 已取消
    Cancelled,
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::FetchTablesError(msg) => write!(f, "获取表列表失败: {}", msg),
            WorkflowError::MetadataError(msg) => write!(f, "获取元数据失败: {}", msg),
            WorkflowError::NoMetadata => write!(f, "未能获取任何表的元数据"),
            WorkflowError::Cancelled => write!(f, "操作已取消"),
        }
    }
}

impl std::error::Error for WorkflowError {}

// ============================================================================
// 进度回调
// ============================================================================

/// 进度回调函数类型
pub type ProgressCallback = Box<dyn Fn(usize, usize) + Send + Sync>;

// ============================================================================
// WorkflowExecutor
// ============================================================================

/// 工作流执行器
pub struct WorkflowExecutor {
    /// 数据库连接 ID
    connection_id: String,
    /// 数据库名
    database: Option<String>,
    /// Schema 名
    schema: Option<String>,
    /// 数据库类型
    database_type: DatabaseType,
    /// 表数量阈值
    threshold: usize,
}

impl WorkflowExecutor {
    /// 创建新的工作流执行器
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

    /// 设置表数量阈值
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
        cancel_token: &CancellationToken,
    ) -> WorkflowAction {
        // 检查取消
        if cancel_token.is_cancelled() {
            return WorkflowAction::Cancelled;
        }

        // 情况1：用户已 @表，直接获取元数据
        if !input.mentioned_tables.is_empty() {
            return self
                .fetch_metadata_and_prepare(
                    &input.clean_question,
                    &input.mentioned_tables,
                    true, // is_user_mentioned
                    None, // no warning
                    global_state,
                    cx,
                    cancel_token,
                    None,
                )
                .await;
        }

        // 情况2：未 @表，先获取表列表
        let tables = match self.fetch_table_list(global_state, cx, cancel_token).await {
            Ok(t) => t,
            Err(WorkflowError::Cancelled) => return WorkflowAction::Cancelled,
            Err(e) => return WorkflowAction::Error(e.to_string()),
        };

        let table_count = tables.len();

        // 超过阈值时添加警告
        let warning = if table_count > self.threshold {
            Some(format!(
                "表数量 ({}) 超过阈值 ({})，AI 输出结果可能不准确，建议使用 @表名 功能指定目标表",
                table_count, self.threshold
            ))
        } else {
            None
        };

        // 需要 AI 选表
        WorkflowAction::NeedAiSelectTables {
            prompt: build_table_selection_prompt(&tables, &input.clean_question),
            tables,
            warning,
        }
    }

    /// 处理 AI 选表结果
    pub async fn handle_table_selection(
        &self,
        user_question: &str,
        selected_tables: &[String],
        warning: Option<String>,
        global_state: &GlobalDbState,
        cx: &mut AsyncApp,
        cancel_token: &CancellationToken,
        progress_callback: Option<ProgressCallback>,
    ) -> WorkflowAction {
        if selected_tables.is_empty() {
            return WorkflowAction::Error("AI 未能选择任何相关表".to_string());
        }

        self.fetch_metadata_and_prepare(
            user_question,
            selected_tables,
            false, // is_user_mentioned
            warning,
            global_state,
            cx,
            cancel_token,
            progress_callback,
        )
        .await
    }

    /// 获取表列表（只有表名和注释）
    async fn fetch_table_list(
        &self,
        global_state: &GlobalDbState,
        cx: &mut AsyncApp,
        cancel_token: &CancellationToken,
    ) -> Result<Vec<TableBrief>, WorkflowError> {
        // 检查取消
        if cancel_token.is_cancelled() {
            return Err(WorkflowError::Cancelled);
        }

        let tables = global_state
            .list_tables(
                cx,
                self.connection_id.clone(),
                self.database.clone().unwrap_or_default(),
                self.schema.clone(),
            )
            .await
            .map_err(|e| WorkflowError::FetchTablesError(e.to_string()))?;

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
        cancel_token: &CancellationToken,
        progress_callback: Option<ProgressCallback>,
    ) -> WorkflowAction {
        let total = table_names.len();
        let mut table_metas = Vec::new();

        for (i, table_name) in table_names.iter().enumerate() {
            // 检查取消
            if cancel_token.is_cancelled() {
                return WorkflowAction::Cancelled;
            }

            match self
                .fetch_table_metadata(table_name, global_state, cx)
                .await
            {
                Ok(meta) => table_metas.push(meta),
                Err(e) => {
                    tracing::warn!("获取表 {} 元数据失败: {}", table_name, e);
                }
            }

            // 更新进度
            if let Some(ref callback) = progress_callback {
                callback(i + 1, total);
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
            comment: None,
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

    /// 获取连接 ID
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// 获取数据库名
    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    /// 获取 Schema 名
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// 获取数据库类型
    pub fn database_type(&self) -> DatabaseType {
        self.database_type
    }
}

impl Clone for WorkflowExecutor {
    fn clone(&self) -> Self {
        Self {
            connection_id: self.connection_id.clone(),
            database: self.database.clone(),
            schema: self.schema.clone(),
            database_type: self.database_type,
            threshold: self.threshold,
        }
    }
}
