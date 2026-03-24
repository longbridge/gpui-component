use std::time::Instant;

use anyhow::{Result, anyhow};
use async_trait::async_trait;

use crate::DatabasePlugin;
use crate::connection::DbConnection;
use crate::executor::SqlResult;
use crate::import_export::{
    ExportConfig, ExportProgressEvent, ExportProgressSender, ExportResult, FormatHandler,
    ImportConfig, ImportResult,
};

pub struct XmlFormatHandler;

impl XmlFormatHandler {
    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn sanitize_tag_name(name: &str) -> String {
        let mut result = String::new();
        for (i, c) in name.chars().enumerate() {
            if i == 0 {
                if c.is_ascii_alphabetic() || c == '_' {
                    result.push(c);
                } else {
                    result.push('_');
                    if c.is_ascii_alphanumeric() {
                        result.push(c);
                    }
                }
            } else if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                result.push(c);
            } else {
                result.push('_');
            }
        }
        if result.is_empty() {
            return "field".to_string();
        }
        result
    }
}

#[async_trait]
impl FormatHandler for XmlFormatHandler {
    async fn import(
        &self,
        _plugin: &dyn DatabasePlugin,
        _connection: &dyn DbConnection,
        _config: &ImportConfig,
        _data: &str,
    ) -> Result<ImportResult> {
        Err(anyhow!("XML import is not supported"))
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
        let mut output = String::new();
        let mut total_rows = 0u64;
        let total_tables = config.tables.len();
        let is_streaming = progress_tx.is_some();

        let send_progress = |event: ExportProgressEvent| {
            if let Some(tx) = &progress_tx {
                let _ = tx.send(event);
            }
        };

        if !is_streaming {
            output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
            output.push_str("<data>\n");
        }

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
                let mut table_output = String::new();
                let table_tag = Self::sanitize_tag_name(table);

                if is_streaming && index == 0 {
                    table_output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
                    table_output.push_str("<data>\n");
                }

                let rows_count = query_result.rows.len() as u64;
                for row in &query_result.rows {
                    table_output.push_str("  <");
                    table_output.push_str(&table_tag);
                    table_output.push_str(">\n");

                    for (i, col_name) in query_result.columns.iter().enumerate() {
                        let tag = Self::sanitize_tag_name(col_name);
                        table_output.push_str("    <");
                        table_output.push_str(&tag);
                        table_output.push('>');

                        if let Some(v) = &row[i] {
                            table_output.push_str(&Self::escape_xml(v));
                        }

                        table_output.push_str("</");
                        table_output.push_str(&tag);
                        table_output.push_str(">\n");
                    }

                    table_output.push_str("  </");
                    table_output.push_str(&table_tag);
                    table_output.push_str(">\n");
                }

                if is_streaming && index == total_tables - 1 {
                    table_output.push_str("</data>\n");
                }

                total_rows += rows_count;
                send_progress(ExportProgressEvent::DataExported {
                    table: table.clone(),
                    rows: rows_count,
                    data: table_output.clone(),
                });

                if !is_streaming {
                    output.push_str(&table_output);
                }
            }

            send_progress(ExportProgressEvent::TableFinished {
                table: table.clone(),
            });
        }

        if !is_streaming {
            output.push_str("</data>\n");
        }

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
