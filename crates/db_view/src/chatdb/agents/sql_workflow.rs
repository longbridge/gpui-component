//! SqlWorkflowAgent - SQL 工作流 Agent
//!
//! 将 AI 选表、元数据获取、SQL 生成整合为单一 Agent，
//! 通过 AgentEvent 协议与 ChatPanel 通信。

use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{info, warn};
use rust_i18n::t;

use one_core::agent::types::{Agent, AgentContext, AgentDescriptor, AgentEvent, AgentResult};
use one_core::llm::{ChatRequest, Message, Role};

use crate::chatdb::query_workflow::{
    build_table_selection_prompt, parse_table_selection_response, parse_user_input,
    QueryContext, TableBrief, TABLE_COUNT_THRESHOLD,
};

use super::db_metadata::{CAP_DB_METADATA, DatabaseMetadataProvider};

static DESCRIPTOR: AgentDescriptor = AgentDescriptor {
    id: "sql_workflow",
    display_name: "SQL Query Assistant",
    description: "Generates SQL queries by analyzing database schema. Handles table selection, \
                  metadata fetching, and SQL generation. Use when the user wants to query, \
                  analyze, or manipulate database data.",
    keywords: &[
        "@", "sql", "query", "select", "insert", "update", "delete",
        "查询", "统计", "表", "数据库", "筛选", "排序", "分组",
    ],
    command_prefix: Some("/sql"),
    examples: &[
        "查询最近7天注册用户",
        "@orders 统计订单金额",
        "SELECT * FROM users WHERE active = true",
    ],
    required_capabilities: &[CAP_DB_METADATA],
    priority: 10,
};

/// SQL workflow agent that integrates table selection, metadata fetching, and SQL generation.
pub struct SqlWorkflowAgent;

#[async_trait]
impl Agent for SqlWorkflowAgent {
    fn descriptor(&self) -> &AgentDescriptor {
        &DESCRIPTOR
    }

    async fn execute(&self, ctx: AgentContext, tx: mpsc::Sender<AgentEvent>) {
        if let Err(e) = self.run(ctx, &tx).await {
            let _ = tx.send(AgentEvent::Error(e)).await;
        }
    }
}

impl SqlWorkflowAgent {
    async fn run(
        &self,
        ctx: AgentContext,
        tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<(), String> {
        // Check cancellation
        if ctx.cancel_token.is_cancelled() {
            let _ = tx.send(AgentEvent::Cancelled).await;
            return Ok(());
        }

        let db_meta = ctx
            .get_capability::<DatabaseMetadataProvider>(CAP_DB_METADATA)
            .ok_or_else(|| "Database metadata capability not available".to_string())?
            .clone();

        // Step 1: Parse user input for @table mentions
        let parsed = parse_user_input(&ctx.user_input);
        let user_question = if parsed.clean_question.is_empty() {
            ctx.user_input.clone()
        } else {
            parsed.clean_question.clone()
        };

        // Step 2: Determine table selection strategy
        let (selected_tables, is_user_mentioned, warning) = if !parsed.mentioned_tables.is_empty() {
            // User explicitly mentioned tables
            (parsed.mentioned_tables.clone(), true, None)
        } else {
            // Need to discover tables
            let _ = tx
                .send(AgentEvent::Progress(t!("SqlWorkflow.fetch_tables").to_string()))
                .await;

            if ctx.cancel_token.is_cancelled() {
                let _ = tx.send(AgentEvent::Cancelled).await;
                return Ok(());
            }

            let tables = db_meta
                .list_tables()
                .await
                .map_err(|e| t!("SqlWorkflow.fetch_tables_failed", error = e).to_string())?;

            let table_count = tables.len();
            let warning = if table_count > TABLE_COUNT_THRESHOLD {
                Some(
                    t!(
                        "SqlWorkflow.table_count_warning",
                        count = table_count,
                        threshold = TABLE_COUNT_THRESHOLD
                    )
                    .to_string()
                )
            } else {
                None
            };

            // AI selects relevant tables
            let _ = tx
                .send(AgentEvent::Progress(t!("SqlWorkflow.ai_select_tables").to_string()))
                .await;

            let selected = self
                .ai_select_tables(&ctx, &tables, &user_question)
                .await?;

            if selected.is_empty() {
                return Err(t!("SqlWorkflow.no_tables_selected").to_string());
            }

            info!(tables = ?selected, "AI selected tables");
            (selected, false, warning)
        };

        // Step 3: Fetch metadata for selected tables
        let total = selected_tables.len();
        let mut table_metas = Vec::new();

        for (i, table_name) in selected_tables.iter().enumerate() {
            if ctx.cancel_token.is_cancelled() {
                let _ = tx.send(AgentEvent::Cancelled).await;
                return Ok(());
            }

            let _ = tx
                .send(AgentEvent::Progress(
                    t!(
                        "SqlWorkflow.fetch_table_schema",
                        current = i + 1,
                        total = total
                    )
                    .to_string()
                ))
                .await;

            match db_meta.fetch_table_metadata(table_name).await {
                Ok(meta) => table_metas.push(meta),
                Err(e) => {
                    warn!(
                        "{}",
                        t!(
                            "SqlWorkflow.fetch_table_metadata_failed",
                            table = table_name,
                            error = e
                        )
                    );
                }
            }
        }

        if table_metas.is_empty() {
            return Err(t!("SqlWorkflow.no_table_metadata").to_string());
        }

        // Step 4: Build QueryContext
        let context = QueryContext {
            user_question: user_question.clone(),
            database_type: db_meta.database_type,
            tables: table_metas,
            selected_table_names: selected_tables,
            is_user_mentioned,
            warning,
        };

        // Step 5: Send workflow summary as prefix, then stream SQL generation
        let workflow_summary = context.to_workflow_summary();
        if !workflow_summary.is_empty() {
            let _ = tx.send(AgentEvent::TextDelta(workflow_summary.clone())).await;
        }

        self.stream_sql_generation(&ctx, &context, &workflow_summary, tx)
            .await
    }

    /// Non-streaming LLM call to select relevant tables.
    async fn ai_select_tables(
        &self,
        ctx: &AgentContext,
        tables: &[TableBrief],
        user_question: &str,
    ) -> Result<Vec<String>, String> {
        let prompt = build_table_selection_prompt(tables, user_question);

        let provider = ctx
            .provider_state
            .manager()
            .get_provider(&ctx.provider_config)
            .await
            .map_err(|e| t!("SqlWorkflow.get_ai_provider_failed", error = e).to_string())?;

        let request = ChatRequest {
            model: ctx.provider_config.model.clone(),
            messages: vec![Message::text(Role::User, prompt)],
            max_tokens: Some(500),
            temperature: Some(0.3),
            stream: Some(false),
            ..Default::default()
        };

        let response = tokio::select! {
            _ = ctx.cancel_token.cancelled() => {
                return Err(t!("SqlWorkflow.operation_cancelled").to_string());
            }
            result = provider.chat(&request) => {
                result.map_err(|e| t!("SqlWorkflow.ai_select_failed", error = e).to_string())?
            }
        };

        Ok(parse_table_selection_response(&response))
    }

    /// Streaming LLM call to generate SQL, emitting TextDelta events with throttle.
    async fn stream_sql_generation(
        &self,
        ctx: &AgentContext,
        context: &QueryContext,
        workflow_summary: &str,
        tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<(), String> {
        let system_prompt = context.to_sql_generation_prompt();

        // Build messages: system prompt + recent history + user question
        let mut messages = vec![Message::text(Role::System, system_prompt)];
        // Add recent chat history
        for msg in &ctx.chat_history {
            messages.push(msg.clone());
        }
        messages.push(Message::text(Role::User, &context.user_question));

        let request = ChatRequest {
            model: ctx.provider_config.model.clone(),
            messages,
            max_tokens: ctx
                .provider_config
                .max_tokens
                .map(|v| v as u32)
                .or(Some(4096)),
            temperature: ctx.provider_config.temperature.or(Some(0.7)),
            stream: Some(true),
            ..Default::default()
        };

        let provider = ctx
            .provider_state
            .manager()
            .get_provider(&ctx.provider_config)
            .await
            .map_err(|e| t!("SqlWorkflow.get_ai_provider_failed", error = e).to_string())?;

        let mut stream = provider
            .chat_stream(&request)
            .await
            .map_err(|e| t!("SqlWorkflow.start_stream_failed", error = e).to_string())?;

        let mut full_content = workflow_summary.to_string();
        let mut pending_delta = String::new();
        let mut last_emit = Instant::now();
        let throttle = Duration::from_millis(50);

        loop {
            tokio::select! {
                _ = ctx.cancel_token.cancelled() => {
                    let _ = tx.send(AgentEvent::Cancelled).await;
                    return Ok(());
                }
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(response)) => {
                            if let Some(content) = response.get_content() {
                                full_content.push_str(content);
                                pending_delta.push_str(content);

                                if last_emit.elapsed() >= throttle {
                                    let delta = std::mem::take(&mut pending_delta);
                                    let _ = tx.send(AgentEvent::TextDelta(delta)).await;
                                    last_emit = Instant::now();
                                }
                            }

                            let is_done = response.choices.iter().any(|c| {
                                c.finish_reason
                                    .as_ref()
                                    .map(|r| r != "null")
                                    .unwrap_or(false)
                            });

                            if is_done {
                                if !pending_delta.is_empty() {
                                    let _ = tx.send(AgentEvent::TextDelta(pending_delta)).await;
                                }
                                let _ = tx
                                    .send(AgentEvent::Completed(AgentResult {
                                        content: full_content,
                                        artifacts: vec![],
                                        ..Default::default()
                                    }))
                                    .await;
                                return Ok(());
                            }
                        }
                        Some(Err(e)) => {
                            return Err(t!("SqlWorkflow.stream_error", error = e).to_string());
                        }
                        None => {
                            if !pending_delta.is_empty() {
                                let _ = tx.send(AgentEvent::TextDelta(pending_delta)).await;
                            }
                            let _ = tx
                                .send(AgentEvent::Completed(AgentResult {
                                    content: full_content,
                                    artifacts: vec![],
                                    ..Default::default()
                                }))
                                .await;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}
