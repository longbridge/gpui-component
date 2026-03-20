//! ChatBiAgent - BI 分析 Agent
//!
//! 流程：识别问题 -> 选表/生成 SQL -> 取数 -> 让模型基于结果产出分析与图表 JSON。

use async_trait::async_trait;
use db::is_query_statement_fallback;
use regex::Regex;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use rust_i18n::t;

use one_core::agent::types::{Agent, AgentContext, AgentDescriptor, AgentEvent, AgentResult};
use one_core::llm::{ChatRequest, Message, Role};

use crate::chatdb::agents::query_workflow::{
    TableBrief, TableSelectionSource, build_table_selection_prompt, extract_tables_from_history,
    is_followup_question, parse_table_selection_response, parse_user_input,
};

use super::db_metadata::{CAP_DB_METADATA, DatabaseMetadataProvider};

const PREVIEW_MAX_ROWS: usize = 200;
const CHART_MAX_ROWS: usize = 20;

static DESCRIPTOR: AgentDescriptor = AgentDescriptor {
    id: "chat_bi",
    display_name: "Chat BI",
    description: "Data analysis agent that queries database, analyzes result, and returns chart JSON code block for rendering.",
    keywords: &[
        "bi",
        "chart",
        "dashboard",
        "trend",
        "analysis",
        "同比",
        "环比",
        "趋势",
        "图表",
        "分析",
    ],
    command_prefix: Some("/bi"),
    examples: &[
        "按月份分析订单金额趋势并画图",
        "近30天活跃用户变化，给出图表",
        "/bi 按渠道统计营收占比",
    ],
    required_capabilities: &[CAP_DB_METADATA],
    priority: 8,
};

pub struct ChatBiAgent;

#[async_trait]
impl Agent for ChatBiAgent {
    fn descriptor(&self) -> &AgentDescriptor {
        &DESCRIPTOR
    }

    async fn execute(&self, ctx: AgentContext, tx: mpsc::Sender<AgentEvent>) {
        if let Err(err) = self.run(ctx, &tx).await {
            let _ = tx.send(AgentEvent::Error(err)).await;
        }
    }
}

impl ChatBiAgent {
    async fn run(&self, ctx: AgentContext, tx: &mpsc::Sender<AgentEvent>) -> Result<(), String> {
        if ctx.cancel_token.is_cancelled() {
            let _ = tx.send(AgentEvent::Cancelled).await;
            return Ok(());
        }

        let db_meta = ctx
            .get_capability::<DatabaseMetadataProvider>(CAP_DB_METADATA)
            .ok_or_else(|| "缺少数据库连接能力，无法执行 BI 分析".to_string())?
            .clone();

        let _ = tx
            .send(AgentEvent::Progress(
                "正在识别分析问题与候选表...".to_string(),
            ))
            .await;

        let parsed = parse_user_input(&ctx.user_input);
        let question = if parsed.clean_question.is_empty() {
            ctx.user_input.clone()
        } else {
            parsed.clean_question
        };

        let (selected_tables, table_source) = if !parsed.mentioned_tables.is_empty() {
            // 优先级1: 用户显式 @表名
            (parsed.mentioned_tables, TableSelectionSource::UserMentioned)
        } else if is_followup_question(&question)
            && let Some(history_tables) = extract_tables_from_history(&ctx.chat_history)
        {
            // 优先级2: 追问场景 + 历史中有已选表 → 复用
            (history_tables, TableSelectionSource::HistoryReused)
        } else {
            // 优先级3: AI 选表
            let tables = db_meta
                .list_tables()
                .await
                .map_err(|e| format!("获取表列表失败: {}", e))?;
            let selected = self.ai_select_tables(&ctx, &tables, &question).await?;
            (selected, TableSelectionSource::AiSelected)
        };

        if selected_tables.is_empty() {
            return Err("未能识别可用数据表，请尝试在问题中使用 @表名".to_string());
        }

        // 发送选表信息（类似 SqlWorkflowAgent 的 workflow_summary）
        let source = match table_source {
            TableSelectionSource::UserMentioned => t!("QueryWorkflow.source_user").to_string(),
            TableSelectionSource::AiSelected => t!("QueryWorkflow.source_ai").to_string(),
            TableSelectionSource::HistoryReused => t!("QueryWorkflow.source_history").to_string(),
        };
        let mut workflow_prefix = String::new();
        workflow_prefix
            .push_str(t!("QueryWorkflow.related_tables_header", source = source).as_ref());
        workflow_prefix.push_str("```json\n");
        let json_array = serde_json::to_string(&selected_tables)
            .unwrap_or_else(|_| format!("{:?}", selected_tables));
        workflow_prefix.push_str(&json_array);
        workflow_prefix.push_str("\n```\n\n");

        let _ = tx
            .send(AgentEvent::TextDelta(workflow_prefix.clone()))
            .await;

        let _ = tx
            .send(AgentEvent::Progress("正在生成分析 SQL...".to_string()))
            .await;

        let sql = self
            .generate_analysis_sql(&ctx, &db_meta, &question, &selected_tables)
            .await?;

        if !is_query_statement_fallback(&sql) {
            return Err(
                "检测到非查询语句，已拒绝执行。请调整问题，仅生成 SELECT/SHOW/WITH/EXPLAIN 等查询 SQL。"
                    .to_string(),
            );
        }

        // 发送生成的 SQL
        let sql_block = format!("```sql\n{}\n```\n\n", sql);
        let _ = tx.send(AgentEvent::TextDelta(sql_block.clone())).await;

        let _ = tx
            .send(AgentEvent::Progress(
                "正在执行 SQL 获取分析数据...".to_string(),
            ))
            .await;

        let query_result = db_meta
            .execute_query_preview(&sql, PREVIEW_MAX_ROWS)
            .await
            .map_err(|e| format!("执行分析 SQL 失败: {}", e))?;

        let _ = tx
            .send(AgentEvent::Progress(
                "正在生成 BI 分析与图表配置...".to_string(),
            ))
            .await;

        let analysis = self
            .generate_analysis_report(&ctx, &question, &sql, &query_result)
            .await?;

        let analysis_with_chart = if contains_chart_json_code_block(&analysis) {
            analysis
        } else {
            format!(
                "{}\n\n```json\n{}\n```",
                analysis,
                self.build_fallback_chart_json(&query_result)
            )
        };

        // 通过 TextDelta 发送分析结论与图表，chat_panel 的 full_content 会累积所有 TextDelta，
        // Completed 时优先使用 full_content，因此必须把全部内容都通过 TextDelta 发出
        let _ = tx.send(AgentEvent::TextDelta(analysis_with_chart)).await;

        tx.send(AgentEvent::Completed(AgentResult::default()))
            .await
            .map_err(|e| format!("发送结果失败: {}", e))?;

        Ok(())
    }

    async fn ai_select_tables(
        &self,
        ctx: &AgentContext,
        tables: &[TableBrief],
        user_question: &str,
    ) -> Result<Vec<String>, String> {
        let prompt = build_table_selection_prompt(tables, user_question, &ctx.chat_history);
        let provider = ctx
            .provider_state
            .manager()
            .get_provider(&ctx.provider_config)
            .await
            .map_err(|e| format!("获取模型失败: {}", e))?;

        let request = ChatRequest {
            model: ctx.provider_config.model.clone(),
            messages: vec![Message::text(Role::User, prompt)],
            max_tokens: Some(500),
            temperature: Some(0.2),
            stream: Some(false),
            ..Default::default()
        };

        let response = provider
            .chat(&request)
            .await
            .map_err(|e| format!("AI 选表失败: {}", e))?;
        Ok(parse_table_selection_response(&response))
    }

    async fn generate_analysis_sql(
        &self,
        ctx: &AgentContext,
        db_meta: &DatabaseMetadataProvider,
        question: &str,
        selected_tables: &[String],
    ) -> Result<String, String> {
        let mut table_meta_text = String::new();
        for table in selected_tables {
            if let Ok(meta) = db_meta.fetch_table_metadata(table).await {
                table_meta_text.push_str(&format!("表: {}\n", meta.name));
                for col in meta.columns {
                    table_meta_text.push_str(&format!(
                        "- {} ({}){}\n",
                        col.name,
                        col.data_type,
                        if col.is_primary_key { " [PK]" } else { "" }
                    ));
                }
                table_meta_text.push('\n');
            }
        }

        let prompt = format!(
            "你是数据分析 SQL 专家。请根据用户问题和表结构生成一条可执行 SQL。\n\
             约束：\n\
             1. 只返回一个 ```sql 代码块\n\
             2. 优先聚合统计并包含时间/类别维度\n\
             3. LIMIT 200\n\n\
             用户问题:\n{}\n\n\
             可用表:\n{}",
            question, table_meta_text
        );

        let provider = ctx
            .provider_state
            .manager()
            .get_provider(&ctx.provider_config)
            .await
            .map_err(|e| format!("获取模型失败: {}", e))?;

        let request = ChatRequest {
            model: ctx.provider_config.model.clone(),
            messages: vec![Message::text(Role::User, prompt)],
            max_tokens: Some(900),
            temperature: Some(0.2),
            stream: Some(false),
            ..Default::default()
        };

        let response = provider
            .chat(&request)
            .await
            .map_err(|e| format!("生成 SQL 失败: {}", e))?;

        extract_sql_from_response(&response)
            .ok_or_else(|| "未从模型返回中提取到 SQL，请重试或明确说明分析维度".to_string())
    }

    async fn generate_analysis_report(
        &self,
        ctx: &AgentContext,
        question: &str,
        sql: &str,
        query_result: &db::QueryResult,
    ) -> Result<String, String> {
        let preview = query_result_to_json_preview(query_result, CHART_MAX_ROWS);

        let prompt = format!(
            "你是 BI 数据分析助手。请基于用户问题、SQL和查询结果给出结论，并输出图表 JSON。\n\
             输出要求：\n\
             1. 先给简短中文分析结论（3-6条）\n\
             2. 再输出一个 ```json 代码块，必须符合以下结构之一：\n\
                - line/bar: {{\"chart_type\":\"line|bar\",\"title\":\"...\",\"x_key\":\"...\",\"y_key\":\"...\",\"data\":[{{...}}]}}\n\
                - pie: {{\"chart_type\":\"pie\",\"title\":\"...\",\"category_key\":\"...\",\"value_key\":\"...\",\"data\":[{{...}}]}}\n\
             3. JSON 的 data 必须来源于查询结果，不得虚构字段。\n\n\
             用户问题:\n{}\n\nSQL:\n{}\n\n查询结果预览(JSON):\n{}",
            question, sql, preview
        );

        let provider = ctx
            .provider_state
            .manager()
            .get_provider(&ctx.provider_config)
            .await
            .map_err(|e| format!("获取模型失败: {}", e))?;

        let request = ChatRequest {
            model: ctx.provider_config.model.clone(),
            messages: vec![Message::text(Role::User, prompt)],
            max_tokens: Some(1800),
            temperature: Some(0.4),
            stream: Some(false),
            ..Default::default()
        };

        provider
            .chat(&request)
            .await
            .map_err(|e| format!("生成 BI 报告失败: {}", e))
    }

    fn build_fallback_chart_json(&self, query_result: &db::QueryResult) -> String {
        let columns = &query_result.columns;
        let mut x_idx = None;
        let mut y_idx = None;

        for (idx, name) in columns.iter().enumerate() {
            if x_idx.is_none() && !is_numeric_column(query_result, idx) {
                x_idx = Some((idx, name.clone()));
            }
            if y_idx.is_none() && is_numeric_column(query_result, idx) {
                y_idx = Some((idx, name.clone()));
            }
        }

        let (x_idx, x_key) = x_idx.unwrap_or((0, columns.first().cloned().unwrap_or("x".into())));
        let (y_idx, y_key) = y_idx.unwrap_or((
            usize::min(1, columns.len().saturating_sub(1)),
            columns
                .get(usize::min(1, columns.len().saturating_sub(1)))
                .cloned()
                .unwrap_or("y".into()),
        ));

        let mut data = Vec::new();
        for row in query_result.rows.iter().take(CHART_MAX_ROWS) {
            let x = row
                .get(x_idx)
                .and_then(|v| v.as_ref())
                .cloned()
                .unwrap_or_default();
            let y = row
                .get(y_idx)
                .and_then(|v| v.as_ref())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            data.push(json!({
                x_key.clone(): x,
                y_key.clone(): y,
            }));
        }

        json!({
            "chart_type": "bar",
            "title": "自动生成图表",
            "x_key": x_key,
            "y_key": y_key,
            "data": data,
        })
        .to_string()
    }
}

fn contains_chart_json_code_block(content: &str) -> bool {
    let re = Regex::new(r"(?s)```json\s*\{.*?\}\s*```").expect("valid regex");
    re.is_match(content) && content.contains("\"chart_type\"")
}

fn extract_sql_from_response(response: &str) -> Option<String> {
    let re = Regex::new(r"(?s)```sql\s*(.*?)\s*```").ok()?;
    if let Some(caps) = re.captures(response) {
        return caps.get(1).map(|m| m.as_str().trim().to_string());
    }

    // 兜底：直接使用整段文本
    let trimmed = response.trim();
    if trimmed.to_lowercase().starts_with("select") || trimmed.to_lowercase().starts_with("with") {
        return Some(trimmed.to_string());
    }
    None
}

fn is_numeric_column(result: &db::QueryResult, idx: usize) -> bool {
    result
        .rows
        .iter()
        .filter_map(|row| row.get(idx).and_then(|v| v.as_ref()))
        .take(10)
        .all(|v| v.parse::<f64>().is_ok())
}

fn query_result_to_json_preview(result: &db::QueryResult, max_rows: usize) -> String {
    let mut rows = Vec::new();
    for row in result.rows.iter().take(max_rows) {
        let mut obj = serde_json::Map::new();
        for (idx, col) in result.columns.iter().enumerate() {
            let value = row.get(idx).and_then(|v| v.clone());
            match value {
                Some(v) => {
                    if let Ok(n) = v.parse::<f64>() {
                        obj.insert(col.clone(), json!(n));
                    } else {
                        obj.insert(col.clone(), json!(v));
                    }
                }
                None => {
                    obj.insert(col.clone(), Value::Null);
                }
            }
        }
        rows.push(Value::Object(obj));
    }

    json!({
        "columns": result.columns,
        "rows": rows,
        "row_count": result.rows.len(),
    })
    .to_string()
}
