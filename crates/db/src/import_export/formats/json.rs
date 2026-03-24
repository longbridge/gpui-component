use std::time::Instant;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::Value;

use super::format_import_table_reference;
use crate::DatabasePlugin;
use crate::connection::DbConnection;
use crate::executor::{ExecOptions, SqlResult};
use crate::import_export::{
    ExportConfig, ExportProgressEvent, ExportProgressSender, ExportResult, FormatHandler,
    ImportConfig, ImportResult,
};

pub struct JsonFormatHandler;

#[async_trait]
impl FormatHandler for JsonFormatHandler {
    async fn import(
        &self,
        plugin: &dyn DatabasePlugin,
        connection: &dyn DbConnection,
        config: &ImportConfig,
        data: &str,
    ) -> Result<ImportResult> {
        let start = Instant::now();
        let mut errors = Vec::new();
        let mut total_rows = 0u64;

        let table = config
            .table
            .as_ref()
            .ok_or_else(|| anyhow!("Table name required for JSON import"))?;
        let table_ref = format_import_table_reference(plugin, config, table);

        // 解析JSON
        let json_value: Value = serde_json::from_str(data)?;
        let rows = match json_value {
            Value::Array(arr) => arr,
            Value::Object(_) => vec![json_value],
            _ => return Err(anyhow!("JSON must be array or object")),
        };

        if rows.is_empty() {
            return Ok(ImportResult {
                success: true,
                rows_imported: 0,
                errors,
                elapsed_ms: start.elapsed().as_millis(),
            });
        }

        // TRUNCATE表
        if config.truncate_before_import {
            let truncate_sql = format!("TRUNCATE TABLE {}", table_ref);
            let results = connection
                .execute(plugin, &truncate_sql, ExecOptions::default())
                .await
                .map_err(|e| anyhow!("Truncate failed: {}", e))?;

            for result in results {
                if let SqlResult::Error(err) = result {
                    errors.push(format!("Truncate failed: {}", err.message));
                    if config.stop_on_error {
                        return Ok(ImportResult {
                            success: false,
                            rows_imported: 0,
                            errors,
                            elapsed_ms: start.elapsed().as_millis(),
                        });
                    }
                }
            }
        }

        // 获取第一行的字段
        let first_obj = rows[0]
            .as_object()
            .ok_or_else(|| anyhow!("JSON array must contain objects"))?;
        let columns: Vec<String> = first_obj.keys().cloned().collect();

        // 批量插入
        for row_obj in rows {
            let obj = match row_obj.as_object() {
                Some(o) => o,
                None => {
                    errors.push("Row is not an object".to_string());
                    if config.stop_on_error {
                        break;
                    }
                    continue;
                }
            };

            let mut insert_sql = format!("INSERT INTO {} (", table_ref);
            for (i, col) in columns.iter().enumerate() {
                if i > 0 {
                    insert_sql.push_str(", ");
                }
                insert_sql.push_str(&plugin.quote_identifier(col));
            }
            insert_sql.push_str(") VALUES (");

            for (i, col) in columns.iter().enumerate() {
                if i > 0 {
                    insert_sql.push_str(", ");
                }
                match obj.get(col) {
                    Some(Value::Null) | None => insert_sql.push_str("NULL"),
                    Some(Value::String(s)) => {
                        insert_sql.push('\'');
                        insert_sql.push_str(&s.replace('\'', "''"));
                        insert_sql.push('\'');
                    }
                    Some(Value::Number(n)) => insert_sql.push_str(&n.to_string()),
                    Some(Value::Bool(b)) => insert_sql.push_str(if *b { "1" } else { "0" }),
                    Some(v) => {
                        insert_sql.push('\'');
                        insert_sql.push_str(&v.to_string().replace('\'', "''"));
                        insert_sql.push('\'');
                    }
                }
            }
            insert_sql.push(')');

            match connection
                .execute(plugin, &insert_sql, ExecOptions::default())
                .await
            {
                Ok(results) => {
                    for result in results {
                        match result {
                            SqlResult::Exec(exec_result) => {
                                total_rows += exec_result.rows_affected;
                            }
                            SqlResult::Error(err) => {
                                errors.push(format!("Insert failed: {}", err.message));
                                if config.stop_on_error {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Insert failed: {}", e));
                    if config.stop_on_error {
                        break;
                    }
                }
            }
        }

        Ok(ImportResult {
            success: errors.is_empty(),
            rows_imported: total_rows,
            errors,
            elapsed_ms: start.elapsed().as_millis(),
        })
    }

    async fn export(
        &self,
        plugin: &dyn DatabasePlugin,
        connection: &dyn DbConnection,
        config: &ExportConfig,
    ) -> Result<ExportResult> {
        self.export_with_progress(plugin, connection, config, None)
            .await
    }

    async fn export_with_progress(
        &self,
        plugin: &dyn DatabasePlugin,
        connection: &dyn DbConnection,
        config: &ExportConfig,
        progress_tx: Option<ExportProgressSender>,
    ) -> Result<ExportResult> {
        let start = Instant::now();
        let mut all_data = Vec::new();
        let mut total_rows = 0u64;
        let total_tables = config.tables.len();
        let is_streaming = progress_tx.is_some();

        let send_progress = |event: ExportProgressEvent| {
            if let Some(tx) = &progress_tx {
                let _ = tx.send(event);
            }
        };

        for (index, table) in config.tables.iter().enumerate() {
            send_progress(ExportProgressEvent::TableStart {
                table: table.clone(),
                table_index: index,
                total_tables,
            });

            send_progress(ExportProgressEvent::FetchingData {
                table: table.clone(),
            });

            let table_ref = plugin.format_table_reference(&config.database, None, table);
            let columns_str = if let Some(cols) = &config.columns {
                cols.iter()
                    .map(|c| plugin.quote_identifier(c))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "*".to_string()
            };

            let mut select_sql = format!("SELECT {} FROM {}", columns_str, table_ref);
            if let Some(where_clause) = &config.where_clause {
                select_sql.push_str(" WHERE ");
                select_sql.push_str(where_clause);
            }
            if let Some(limit) = config.limit {
                let pagination = plugin.format_pagination(limit, 0, "");
                select_sql.push_str(&pagination);
            }

            let result = connection
                .query(&select_sql)
                .await
                .map_err(|e| anyhow!("Query failed: {}", e))?;

            if let SqlResult::Query(query_result) = result {
                let mut table_data = Vec::new();
                let rows_count = query_result.rows.len() as u64;

                for row in &query_result.rows {
                    let mut obj = serde_json::Map::new();
                    for (i, col_name) in query_result.columns.iter().enumerate() {
                        let value = match &row[i] {
                            Some(v) => Value::String(v.clone()),
                            None => Value::Null,
                        };
                        obj.insert(col_name.clone(), value);
                    }
                    table_data.push(Value::Object(obj));
                }

                total_rows += rows_count;

                let table_output = if is_streaming {
                    if index == 0 {
                        format!(
                            "[\n{}",
                            serde_json::to_string_pretty(&table_data)?
                                .trim_start_matches('[')
                                .trim_end_matches(']')
                        )
                    } else if index == total_tables - 1 {
                        format!(
                            ",\n{}\n]",
                            serde_json::to_string_pretty(&table_data)?
                                .trim_start_matches('[')
                                .trim_end_matches(']')
                        )
                    } else {
                        format!(
                            ",\n{}",
                            serde_json::to_string_pretty(&table_data)?
                                .trim_start_matches('[')
                                .trim_end_matches(']')
                        )
                    }
                } else {
                    String::new()
                };

                send_progress(ExportProgressEvent::DataExported {
                    table: table.clone(),
                    rows: rows_count,
                    data: table_output,
                });

                if !is_streaming {
                    all_data.extend(table_data);
                }
            }

            send_progress(ExportProgressEvent::TableFinished {
                table: table.clone(),
            });
        }

        let output = if !is_streaming {
            serde_json::to_string_pretty(&all_data)?
        } else {
            String::new()
        };

        let elapsed_ms = start.elapsed().as_millis();
        send_progress(ExportProgressEvent::Finished {
            total_rows,
            elapsed_ms,
        });

        Ok(ExportResult {
            success: true,
            output,
            rows_exported: total_rows,
            elapsed_ms,
        })
    }
}
