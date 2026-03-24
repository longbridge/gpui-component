use std::time::Instant;

use anyhow::{Result, anyhow};
use async_trait::async_trait;

use super::format_import_table_reference;
use crate::DatabasePlugin;
use crate::connection::DbConnection;
use crate::executor::{ExecOptions, SqlResult};
use crate::import_export::{
    ExportConfig, ExportProgressEvent, ExportProgressSender, ExportResult, FormatHandler,
    ImportConfig, ImportResult,
};

pub struct TxtFormatHandler;

impl TxtFormatHandler {
    fn escape_txt_field(field: &str, delimiter: char, qualifier: Option<char>) -> String {
        let needs_quote = field.contains(delimiter)
            || field.contains('\n')
            || field.contains('\r')
            || qualifier.map(|q| field.contains(q)).unwrap_or(false);

        if needs_quote {
            if let Some(q) = qualifier {
                let escaped = field.replace(q, &format!("{}{}", q, q));
                format!("{}{}{}", q, escaped, q)
            } else {
                field.to_string()
            }
        } else {
            field.to_string()
        }
    }
}

#[async_trait]
impl FormatHandler for TxtFormatHandler {
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
            .ok_or_else(|| anyhow!("Table name required for TXT import"))?;
        let table_ref = format_import_table_reference(plugin, config, table);

        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() {
            return Ok(ImportResult {
                success: true,
                rows_imported: 0,
                errors,
                elapsed_ms: start.elapsed().as_millis(),
            });
        }

        let columns: Vec<String> = lines[0].split('\t').map(|s| s.to_string()).collect();
        if columns.is_empty() {
            return Err(anyhow!("TXT header is empty"));
        }

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

        for (line_num, line) in lines.iter().skip(1).enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let values: Vec<&str> = line.split('\t').collect();
            if values.len() != columns.len() {
                errors.push(format!("Line {}: column count mismatch", line_num + 2));
                if config.stop_on_error {
                    break;
                }
                continue;
            }

            let mut insert_sql = format!("INSERT INTO {} (", table_ref);
            for (i, col) in columns.iter().enumerate() {
                if i > 0 {
                    insert_sql.push_str(", ");
                }
                insert_sql.push_str(&plugin.quote_identifier(col));
            }
            insert_sql.push_str(") VALUES (");

            for (i, val) in values.iter().enumerate() {
                if i > 0 {
                    insert_sql.push_str(", ");
                }
                if val.is_empty() || val.eq_ignore_ascii_case("null") {
                    insert_sql.push_str("NULL");
                } else {
                    insert_sql.push('\'');
                    insert_sql.push_str(&val.replace('\'', "''"));
                    insert_sql.push('\'');
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
                                errors.push(format!("Line {}: {}", line_num + 2, err.message));
                                if config.stop_on_error {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Line {}: {}", line_num + 2, e));
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
        let mut output = String::new();
        let mut total_rows = 0u64;
        let total_tables = config.tables.len();
        let is_streaming = progress_tx.is_some();

        let csv_config =
            config
                .csv_config
                .clone()
                .unwrap_or_else(|| crate::import_export::CsvExportConfig {
                    field_delimiter: '\t',
                    text_qualifier: Some('"'),
                    include_header: true,
                    record_terminator: "\n".to_string(),
                });
        let delimiter = csv_config.field_delimiter;
        let qualifier = csv_config.text_qualifier;
        let include_header = csv_config.include_header;
        let record_terminator = csv_config.record_terminator;

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
                let mut table_output = String::new();

                if include_header {
                    for (i, col) in query_result.columns.iter().enumerate() {
                        if i > 0 {
                            table_output.push(delimiter);
                        }
                        table_output.push_str(&Self::escape_txt_field(col, delimiter, qualifier));
                    }
                    table_output.push_str(&record_terminator);
                }

                let rows_count = query_result.rows.len() as u64;
                for row in &query_result.rows {
                    for (i, val) in row.iter().enumerate() {
                        if i > 0 {
                            table_output.push(delimiter);
                        }
                        if let Some(v) = val {
                            table_output.push_str(&Self::escape_txt_field(v, delimiter, qualifier));
                        }
                    }
                    table_output.push_str(&record_terminator);
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
