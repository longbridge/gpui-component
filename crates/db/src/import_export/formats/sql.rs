use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;

use super::format_import_table_reference;
use crate::connection::DbConnection;
use crate::executor::{ExecOptions, SqlResult};
use crate::import_export::{
    ExportConfig, ExportProgressEvent, ExportProgressSender, ExportResult, FormatHandler,
    ImportConfig, ImportProgressEvent, ImportProgressSender, ImportResult,
};
use crate::{DatabasePlugin, DbError, SqlSource};

pub struct SqlFormatHandler;

#[async_trait]
impl FormatHandler for SqlFormatHandler {
    async fn import(
        &self,
        plugin: &dyn DatabasePlugin,
        connection: &dyn DbConnection,
        config: &ImportConfig,
        data: &str,
    ) -> Result<ImportResult> {
        self.import_with_progress(plugin, connection, config, data, "", None)
            .await
    }

    async fn import_with_progress(
        &self,
        plugin: &dyn DatabasePlugin,
        connection: &dyn DbConnection,
        config: &ImportConfig,
        data: &str,
        file_name: &str,
        progress_tx: Option<ImportProgressSender>,
    ) -> Result<ImportResult> {
        let start = Instant::now();
        let mut errors = Vec::new();
        let mut total_rows = 0u64;

        let send_progress = |event: ImportProgressEvent| {
            if let Some(tx) = &progress_tx {
                let _ = tx.send(event);
            }
        };

        send_progress(ImportProgressEvent::ParsingFile {
            file: file_name.to_string(),
        });

        if config.truncate_before_import {
            if let Some(table) = &config.table {
                let table_ref = format_import_table_reference(plugin, config, table);
                let truncate_sql = format!("TRUNCATE TABLE {}", table_ref);
                send_progress(ImportProgressEvent::ExecutingStatement {
                    file: file_name.to_string(),
                    statement_index: 0,
                    total_statements: 1,
                });

                let results = connection
                    .execute(plugin, &truncate_sql, ExecOptions::default())
                    .await
                    .map_err(|e| anyhow::anyhow!("Truncate failed: {}", e))?;

                for result in results {
                    if let SqlResult::Error(err) = result {
                        let error_msg = format!("Truncate failed: {}", err.message);
                        errors.push(error_msg.clone());
                        send_progress(ImportProgressEvent::Error {
                            file: file_name.to_string(),
                            message: error_msg,
                        });
                        if config.stop_on_error {
                            send_progress(ImportProgressEvent::Finished {
                                total_rows: 0,
                                elapsed_ms: start.elapsed().as_millis(),
                            });
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
        }

        let parser = plugin
            .create_parser(SqlSource::Script(data.to_string()))
            .map_err(|e| DbError::query(format!("Failed to create parser: {}", e)))?;
        let statements: Vec<String> = parser
            .filter_map(|r| r.ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let total_statements = statements.len();

        for (idx, stmt) in statements.iter().enumerate() {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }

            send_progress(ImportProgressEvent::ExecutingStatement {
                file: file_name.to_string(),
                statement_index: idx,
                total_statements,
            });

            let exec_options = ExecOptions {
                stop_on_error: config.stop_on_error,
                transactional: false,
                max_rows: None,
                streaming: false,
            };

            match connection.execute(plugin, stmt, exec_options).await {
                Ok(results) => {
                    for result in results {
                        match result {
                            SqlResult::Exec(exec_result) => {
                                total_rows += exec_result.rows_affected;
                                send_progress(ImportProgressEvent::StatementExecuted {
                                    file: file_name.to_string(),
                                    rows_affected: exec_result.rows_affected,
                                });
                            }
                            SqlResult::Error(err) => {
                                let error_msg = err.message.clone();
                                errors.push(error_msg.clone());
                                send_progress(ImportProgressEvent::Error {
                                    file: file_name.to_string(),
                                    message: error_msg,
                                });
                                if config.stop_on_error {
                                    send_progress(ImportProgressEvent::Finished {
                                        total_rows,
                                        elapsed_ms: start.elapsed().as_millis(),
                                    });
                                    return Ok(ImportResult {
                                        success: false,
                                        rows_imported: total_rows,
                                        errors,
                                        elapsed_ms: start.elapsed().as_millis(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    errors.push(error_msg.clone());
                    send_progress(ImportProgressEvent::Error {
                        file: file_name.to_string(),
                        message: error_msg,
                    });
                    if config.stop_on_error {
                        send_progress(ImportProgressEvent::Finished {
                            total_rows,
                            elapsed_ms: start.elapsed().as_millis(),
                        });
                        return Ok(ImportResult {
                            success: false,
                            rows_imported: total_rows,
                            errors,
                            elapsed_ms: start.elapsed().as_millis(),
                        });
                    }
                }
            }
        }

        let elapsed_ms = start.elapsed().as_millis();
        send_progress(ImportProgressEvent::FileFinished {
            file: file_name.to_string(),
            rows_imported: total_rows,
        });

        Ok(ImportResult {
            success: errors.is_empty(),
            rows_imported: total_rows,
            errors,
            elapsed_ms,
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

            if config.include_schema {
                send_progress(ExportProgressEvent::GettingStructure {
                    table: table.clone(),
                });

                match plugin
                    .export_table_create_sql(
                        connection,
                        &config.database,
                        config.schema.as_deref(),
                        table,
                    )
                    .await
                {
                    Ok(schema_sql) => {
                        let mut schema_output = String::new();
                        if !schema_sql.is_empty() {
                            schema_output.push_str("-- Table structure for ");
                            schema_output.push_str(table);
                            schema_output.push('\n');
                            schema_output.push_str(&schema_sql);
                            schema_output.push_str(";\n\n");
                        }
                        send_progress(ExportProgressEvent::StructureExported {
                            table: table.clone(),
                            data: schema_output.clone(),
                        });
                        if !is_streaming {
                            output.push_str(&schema_output);
                        }
                    }
                    Err(e) => {
                        let error_output =
                            format!("-- Failed to export structure for {}: {}\n\n", table, e);
                        if !is_streaming {
                            output.push_str(&error_output);
                        }
                        send_progress(ExportProgressEvent::Error {
                            table: table.clone(),
                            message: format!("Failed to export structure: {}", e),
                        });
                    }
                }
            }

            if config.include_data {
                send_progress(ExportProgressEvent::FetchingData {
                    table: table.clone(),
                });

                match plugin
                    .export_table_data_sql(
                        connection,
                        &config.database,
                        config.schema.as_deref(),
                        table,
                        config.where_clause.as_deref(),
                        config.limit,
                    )
                    .await
                {
                    Ok(data_sql) => {
                        let mut data_output = String::new();
                        let rows_count = if !data_sql.is_empty() {
                            data_output.push_str("-- Data for table ");
                            data_output.push_str(table);
                            data_output.push('\n');
                            data_output.push_str(&data_sql);
                            data_output.push('\n');
                            data_sql.lines().filter(|l| l.starts_with("INSERT")).count() as u64
                        } else {
                            0
                        };
                        total_rows += rows_count;
                        send_progress(ExportProgressEvent::DataExported {
                            table: table.clone(),
                            rows: rows_count,
                            data: data_output.clone(),
                        });
                        if !is_streaming {
                            output.push_str(&data_output);
                        }
                    }
                    Err(e) => {
                        let error_output =
                            format!("-- Failed to export data for {}: {}\n\n", table, e);
                        if !is_streaming {
                            output.push_str(&error_output);
                        }
                        send_progress(ExportProgressEvent::Error {
                            table: table.clone(),
                            message: format!("Failed to export data: {}", e),
                        });
                    }
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
