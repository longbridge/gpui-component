use std::rc::Rc;
use std::time::Instant;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, PathPromptOptions, Render, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Disableable, IconName, IndexPath, VirtualListScrollHandle,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    select::{Select, SelectItem, SelectState},
    switch::Switch,
    v_flex, v_virtual_list,
};
use tokio::sync::mpsc;

use db::{CsvImportConfig, DataFormat, GlobalDbState, ImportConfig, ImportProgressEvent};
use gpui_component::tooltip::Tooltip;
use rust_i18n::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStep {
    Config,
    Execute,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RecordSeparator {
    Lf,
    CrLf,
}

impl RecordSeparator {
    pub fn to_separator_string(&self) -> String {
        match self {
            RecordSeparator::Lf => "\n".to_string(),
            RecordSeparator::CrLf => "\r\n".to_string(),
        }
    }

    fn all() -> Vec<Self> {
        vec![RecordSeparator::Lf, RecordSeparator::CrLf]
    }
}

impl SelectItem for RecordSeparator {
    type Value = RecordSeparator;

    fn title(&self) -> gpui::SharedString {
        match self {
            RecordSeparator::Lf => "LF (\\n)".into(),
            RecordSeparator::CrLf => "CRLF (\\r\\n)".into(),
        }
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldSeparator {
    Comma,
    Tab,
    Semicolon,
    Pipe,
}

impl FieldSeparator {
    pub fn to_separator_char(&self) -> char {
        match self {
            FieldSeparator::Comma => ',',
            FieldSeparator::Tab => '\t',
            FieldSeparator::Semicolon => ';',
            FieldSeparator::Pipe => '|',
        }
    }

    fn all() -> Vec<Self> {
        vec![
            FieldSeparator::Comma,
            FieldSeparator::Tab,
            FieldSeparator::Semicolon,
            FieldSeparator::Pipe,
        ]
    }
}

impl SelectItem for FieldSeparator {
    type Value = FieldSeparator;

    fn title(&self) -> gpui::SharedString {
        match self {
            FieldSeparator::Comma => t!("ImportExport.delimiter_comma").into(),
            FieldSeparator::Tab => t!("ImportExport.delimiter_tab").into(),
            FieldSeparator::Semicolon => t!("ImportExport.delimiter_semicolon").into(),
            FieldSeparator::Pipe => t!("ImportExport.delimiter_pipe").into(),
        }
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

#[derive(Clone, Debug)]
pub struct TextQualifierItem {
    name: String,
    value: String,
}

impl TextQualifierItem {
    fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

impl SelectItem for TextQualifierItem {
    type Value = String;

    fn title(&self) -> gpui::SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataImportFormat {
    Txt,
    Csv,
    Json,
    Xml,
}

impl DataImportFormat {
    pub fn to_data_format(self) -> DataFormat {
        match self {
            Self::Txt => DataFormat::Txt,
            Self::Csv => DataFormat::Csv,
            Self::Json => DataFormat::Json,
            Self::Xml => DataFormat::Xml,
        }
    }

    pub fn needs_delimiter_config(self) -> bool {
        matches!(self, Self::Txt | Self::Csv)
    }
}

#[derive(Debug, Clone)]
struct LogEntry {
    table: String,
    message: String,
}

pub struct TableImportView {
    connection_id: String,
    server_info: String,
    database: String,
    schema: Option<String>,
    table: String,

    current_step: ImportStep,
    format: Entity<DataImportFormat>,

    file_path: Entity<InputState>,
    pending_file_path: Entity<Option<String>>,

    record_separator: Entity<SelectState<Vec<RecordSeparator>>>,
    field_separator: Entity<SelectState<Vec<FieldSeparator>>>,
    text_qualifier: Entity<SelectState<Vec<TextQualifierItem>>>,

    has_header: Entity<bool>,
    stop_on_error: Entity<bool>,
    use_transaction: Entity<bool>,
    truncate_before: Entity<bool>,

    logs: Entity<Vec<LogEntry>>,
    scroll_handle: VirtualListScrollHandle,

    processed_records: Entity<u64>,
    error_count: Entity<u32>,
    elapsed_time: Entity<String>,
    progress: Entity<f32>,

    is_running: Entity<bool>,
    is_finished: Entity<bool>,
    start_time: Option<Instant>,

    validation_error: Entity<Option<String>>,

    focus_handle: FocusHandle,
}

impl TableImportView {
    pub fn new(
        connection_id: impl Into<String>,
        database: impl Into<String>,
        schema: Option<String>,
        table: Option<String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let global_db_state = cx.global::<GlobalDbState>();
        let connection_id = connection_id.into();
        let config = global_db_state.get_config(&connection_id);

        let text_qualifier_items = vec![
            TextQualifierItem::new(t!("ImportExport.qualifier_none"), ""),
            TextQualifierItem::new(t!("ImportExport.qualifier_double_quote"), "\""),
            TextQualifierItem::new(t!("ImportExport.qualifier_single_quote"), "'"),
        ];

        cx.new(|cx| Self {
            connection_id: connection_id.clone(),
            server_info: config.map(|c| c.server_info()).unwrap_or_default(),
            database: database.into(),
            schema,
            table: table.unwrap_or_default(),

            current_step: ImportStep::Config,
            format: cx.new(|_| DataImportFormat::Csv),

            file_path: cx
                .new(|cx| InputState::new(window, cx).placeholder(t!("ImportExport.select_import_file"))),
            pending_file_path: cx.new(|_| None),

            record_separator: cx.new(|cx| {
                SelectState::new(
                    RecordSeparator::all(),
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            }),
            field_separator: cx.new(|cx| {
                SelectState::new(
                    FieldSeparator::all(),
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            }),
            text_qualifier: cx.new(|cx| {
                SelectState::new(text_qualifier_items, Some(IndexPath::new(1)), window, cx)
            }),

            has_header: cx.new(|_| true),
            stop_on_error: cx.new(|_| true),
            use_transaction: cx.new(|_| true),
            truncate_before: cx.new(|_| false),

            logs: cx.new(|_| Vec::new()),
            scroll_handle: VirtualListScrollHandle::new(),

            processed_records: cx.new(|_| 0),
            error_count: cx.new(|_| 0),
            elapsed_time: cx.new(|_| "0.00s".to_string()),
            progress: cx.new(|_| 0.0),

            is_running: cx.new(|_| false),
            is_finished: cx.new(|_| false),
            start_time: None,

            validation_error: cx.new(|_| None),

            focus_handle: cx.focus_handle(),
        })
    }

    fn add_log(
        cx: &gpui::AsyncApp,
        logs: &Entity<Vec<LogEntry>>,
        scroll_handle: &VirtualListScrollHandle,
        table: String,
        message: String,
    ) {
        let logs_clone = logs.clone();
        let scroll_handle_clone = scroll_handle.clone();
        let _ = cx.update(|cx| {
            logs_clone.update(cx, |l, cx| {
                l.push(LogEntry { table, message });
                cx.notify();
            });
            scroll_handle_clone.scroll_to_bottom();
        });
    }

    fn select_file(&mut self, _window: &mut Window, cx: &mut App) {
        let pending = self.pending_file_path.clone();
        let future = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            multiple: false,
            directories: false,
            prompt: Some(t!("ImportExport.select_import_file").into()),
        });

        cx.spawn(async move |cx| {
            if let Ok(Ok(Some(paths))) = future.await {
                if let Some(path_buf) = paths.first() {
                    let path = path_buf.to_string_lossy().to_string();
                    let _ = cx.update(|cx| {
                        pending.update(cx, |p, cx| {
                            *p = Some(path);
                            cx.notify();
                        });
                    });
                }
            }
        })
        .detach();
    }

    fn start_import(&mut self, _window: &mut Window, cx: &mut App) {
        if *self.is_running.read(cx) {
            return;
        }

        let file_path_str = self.file_path.read(cx).text().to_string();
        if file_path_str.is_empty() {
            self.logs.update(cx, |l, cx| {
                l.push(LogEntry {
                    table: "".to_string(),
                    message: t!("ImportExport.please_select_file").to_string(),
                });
                cx.notify();
            });
            self.scroll_handle.scroll_to_bottom();
            return;
        }

        if self.table.is_empty() {
            self.logs.update(cx, |l, cx| {
                l.push(LogEntry {
                    table: "".to_string(),
                    message: t!("ImportExport.please_enter_table_name").to_string(),
                });
                cx.notify();
            });
            self.scroll_handle.scroll_to_bottom();
            return;
        }

        self.is_running.update(cx, |r, cx| {
            *r = true;
            cx.notify();
        });

        self.start_time = Some(Instant::now());

        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = self.connection_id.clone();
        let database = self.database.clone();
        let schema = self.schema.clone();
        let table = self.table.clone();
        let format = *self.format.read(cx);

        let field_delimiter = self
            .field_separator
            .read(cx)
            .selected_value()
            .map(|v| v.to_separator_char())
            .unwrap_or(',');

        let record_terminator = self
            .record_separator
            .read(cx)
            .selected_value()
            .map(|v| v.to_separator_string())
            .unwrap_or_else(|| "\n".to_string());

        let text_qualifier = self
            .text_qualifier
            .read(cx)
            .selected_value()
            .and_then(|s| s.chars().next());

        let has_header = *self.has_header.read(cx);
        let stop_on_error = *self.stop_on_error.read(cx);
        let use_transaction = *self.use_transaction.read(cx);
        let truncate_before = *self.truncate_before.read(cx);

        let csv_config = if format.needs_delimiter_config() {
            Some(CsvImportConfig {
                field_delimiter,
                text_qualifier,
                has_header,
                record_terminator,
            })
        } else {
            None
        };

        let logs = self.logs.clone();
        let scroll_handle = self.scroll_handle.clone();
        let processed_records = self.processed_records.clone();
        let error_count = self.error_count.clone();
        let elapsed_time = self.elapsed_time.clone();
        let progress = self.progress.clone();
        let is_running = self.is_running.clone();
        let is_finished = self.is_finished.clone();
        let start_time = self.start_time;

        cx.spawn(async move |cx| {
            Self::add_log(
                &cx,
                &logs,
                &scroll_handle,
                "".to_string(),
                t!("ImportExport.import_table_log", table = table).to_string(),
            );

            let data = match std::fs::read_to_string(&file_path_str) {
                Ok(d) => d,
                Err(e) => {
                    Self::add_log(
                        &cx,
                        &logs,
                        &scroll_handle,
                        "".to_string(),
                        t!("ImportExport.file_read_error_with_message", error = e).to_string(),
                    );
                    let _ = cx.update(|cx| {
                        is_running.update(cx, |r, cx| {
                            *r = false;
                            cx.notify();
                        });
                        is_finished.update(cx, |f, cx| {
                            *f = true;
                            cx.notify();
                        });
                        error_count.update(cx, |e, cx| {
                            *e += 1;
                            cx.notify();
                        });
                    });
                    return;
                }
            };

            let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ImportProgressEvent>();

            let import_config = ImportConfig {
                format: format.to_data_format(),
                database: database.clone(),
                schema: schema.clone(),
                table: Some(table.clone()),
                stop_on_error,
                use_transaction,
                truncate_before_import: truncate_before,
                csv_config,
            };

            let global_state_clone = global_state.clone();
            let connection_id_clone = connection_id.clone();
            let file_name = file_path_str.clone();

            let import_handle = cx.background_spawn(async move {
                global_state_clone
                    .import_data_with_progress_sync(
                        connection_id_clone,
                        import_config,
                        data,
                        &file_name,
                        Some(progress_tx),
                    )
                    .await
            });

            while let Some(event) = progress_rx.recv().await {
                let event_clone = event.clone();
                let logs_clone = logs.clone();
                let scroll_handle_clone = scroll_handle.clone();
                let processed_records_clone = processed_records.clone();
                let error_count_clone = error_count.clone();
                let elapsed_time_clone = elapsed_time.clone();
                let progress_clone = progress.clone();

                let _ = cx.update(|cx| {
                    let elapsed = start_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);

                    elapsed_time_clone.update(cx, |t, cx| {
                        *t = format!("{:.2}s", elapsed);
                        cx.notify();
                    });

                    match event_clone {
                        ImportProgressEvent::FileStart {
                            file,
                            file_index,
                            total_files,
                        } => {
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: format!(
                                        "{}",
                                        t!(
                                            "ImportExport.file_start",
                                            file = file,
                                            current = file_index + 1,
                                            total = total_files
                                        )
                                    ),
                                });
                                cx.notify();
                            });
                        }
                        ImportProgressEvent::ReadingFile { file } => {
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!("ImportExport.reading_file", file = file)
                                        .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ImportProgressEvent::ParsingFile { file } => {
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!("ImportExport.parsing_file", file = file).to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ImportProgressEvent::ExecutingStatement {
                            file: _,
                            statement_index,
                            total_statements,
                        } => {
                            if total_statements > 0 {
                                let p = ((statement_index + 1) as f32 / total_statements as f32)
                                    * 100.0;
                                progress_clone.update(cx, |pr, cx| {
                                    *pr = p;
                                    cx.notify();
                                });
                            }
                            processed_records_clone.update(cx, |r, cx| {
                                *r = (statement_index + 1) as u64;
                                cx.notify();
                            });
                        }
                        ImportProgressEvent::StatementExecuted {
                            file: _,
                            rows_affected,
                        } => {
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!(
                                        "ImportExport.execution_done_rows",
                                        rows = rows_affected
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ImportProgressEvent::FileFinished {
                            file,
                            rows_imported,
                        } => {
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!(
                                        "ImportExport.file_finished_rows",
                                        file = file,
                                        rows = rows_imported
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ImportProgressEvent::Error { file: _, message } => {
                            error_count_clone.update(cx, |e, cx| {
                                *e += 1;
                                cx.notify();
                            });
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!(
                                        "ImportExport.import_error_with_message",
                                        message = message
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ImportProgressEvent::Finished {
                            total_rows,
                            elapsed_ms,
                        } => {
                            progress_clone.update(cx, |p, cx| {
                                *p = 100.0;
                                cx.notify();
                            });
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!(
                                        "ImportExport.import_complete_summary",
                                        rows = total_rows,
                                        elapsed_ms = elapsed_ms
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                    }

                    scroll_handle_clone.scroll_to_bottom();
                });
            }

            let result = import_handle.await;

            let _ = cx.update(|cx| {
                is_running.update(cx, |r, cx| {
                    *r = false;
                    cx.notify();
                });
                is_finished.update(cx, |f, cx| {
                    *f = true;
                    cx.notify();
                });

                match result {
                    Ok(import_result) => {
                        if import_result.success {
                            logs.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!(
                                        "ImportExport.import_success_summary",
                                        rows = import_result.rows_imported,
                                        elapsed_ms = import_result.elapsed_ms
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        } else {
                            logs.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!(
                                        "ImportExport.import_partial_summary",
                                        rows = import_result.rows_imported,
                                        errors = import_result.errors.len()
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                            error_count.update(cx, |e, cx| {
                                *e += import_result.errors.len() as u32;
                                cx.notify();
                            });
                        }
                    }
                    Err(e) => {
                        logs.update(cx, |l, cx| {
                            l.push(LogEntry {
                                table: "".to_string(),
                                message: t!("ImportExport.import_failed", error = e).to_string(),
                            });
                            cx.notify();
                        });
                        error_count.update(cx, |e, cx| {
                            *e += 1;
                            cx.notify();
                        });
                    }
                }

                scroll_handle.scroll_to_bottom();
            });
        })
        .detach();
    }
}

impl Focusable for TableImportView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Clone for TableImportView {
    fn clone(&self) -> Self {
        Self {
            connection_id: self.connection_id.clone(),
            server_info: self.server_info.clone(),
            database: self.database.clone(),
            schema: self.schema.clone(),
            table: self.table.clone(),
            current_step: self.current_step,
            format: self.format.clone(),
            file_path: self.file_path.clone(),
            pending_file_path: self.pending_file_path.clone(),
            record_separator: self.record_separator.clone(),
            field_separator: self.field_separator.clone(),
            text_qualifier: self.text_qualifier.clone(),
            has_header: self.has_header.clone(),
            stop_on_error: self.stop_on_error.clone(),
            use_transaction: self.use_transaction.clone(),
            truncate_before: self.truncate_before.clone(),
            logs: self.logs.clone(),
            scroll_handle: self.scroll_handle.clone(),
            processed_records: self.processed_records.clone(),
            error_count: self.error_count.clone(),
            elapsed_time: self.elapsed_time.clone(),
            progress: self.progress.clone(),
            is_running: self.is_running.clone(),
            is_finished: self.is_finished.clone(),
            start_time: self.start_time,
            validation_error: self.validation_error.clone(),
            focus_handle: self.focus_handle.clone(),
        }
    }
}

impl Render for TableImportView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(path) = self.pending_file_path.read(cx).clone() {
            self.file_path.update(cx, |state, cx| {
                state.replace(path, window, cx);
            });
            self.pending_file_path.update(cx, |p, _| *p = None);
        }

        let is_running = *self.is_running.read(cx);
        let is_finished = *self.is_finished.read(cx);
        let current_format = *self.format.read(cx);
        let progress_value = *self.progress.read(cx);
        let processed = *self.processed_records.read(cx);
        let errors = *self.error_count.read(cx);
        let elapsed = self.elapsed_time.read(cx).clone();
        let logs = self.logs.read(cx).clone();
        let current_step = self.current_step;
        let validation_error = self.validation_error.read(cx).clone();

        v_flex()
            .w_full()
            .h(px(540.0))
            .gap_3()
            .p_4()
            .pt_8()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if current_step == ImportStep::Config {
                        t!("ImportExport.import_hint")
                    } else {
                        t!("ImportExport.import_ready_hint")
                    }),
            )
            .when(current_step == ImportStep::Config, |this| {
                this.child(
                    v_flex()
                        .flex_1()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_6()
                                .p_2()
                                .rounded_md()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{}:", t!("TreeView.server"))),
                                        )
                                        .child(
                                            div()
                                                .text_ellipsis()
                                                .overflow_hidden()
                                                .child(self.server_info.clone()),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{}:", t!("Database.database"))),
                                        )
                                        .child(
                                            div()
                                                .text_ellipsis()
                                                .overflow_hidden()
                                                .child(self.database.clone()),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{}:", t!("Common.table"))),
                                        )
                                        .child(
                                            div()
                                                .text_ellipsis()
                                                .overflow_hidden()
                                                .child(self.table.clone()),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .p_2()
                                .rounded_md()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(Input::new(&self.file_path).flex_1())
                                        .child(
                                            Button::new("browse").icon(IconName::Folder).on_click(
                                                window.listener_for(
                                                    &cx.entity(),
                                                    |view, _, window, cx| {
                                                        view.validation_error
                                                            .update(cx, |e, _| *e = None);
                                                        view.select_file(window, cx);
                                                    },
                                                ),
                                            ),
                                        ),
                                )
                                .when_some(validation_error.clone(), |this, err| {
                                    this.child(
                                        div().text_sm().text_color(cx.theme().danger).child(err),
                                    )
                                }),
                        )
                        .child(div().h(px(1.)).w_full().bg(cx.theme().border))
                        .child(
                            v_flex()
                                .gap_3()
                                .p_3()
                                .rounded_md()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(t!("ImportExport.file_format")),
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .child({
                                            let is_selected =
                                                current_format == DataImportFormat::Txt;
                                            let mut btn = Button::new("format_txt").child("TXT");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(
                                                &cx.entity(),
                                                |view, _, _, cx| {
                                                    view.format.update(cx, |f, cx| {
                                                        *f = DataImportFormat::Txt;
                                                        cx.notify();
                                                    });
                                                },
                                            ))
                                        })
                                        .child({
                                            let is_selected =
                                                current_format == DataImportFormat::Csv;
                                            let mut btn = Button::new("format_csv").child("CSV");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(
                                                &cx.entity(),
                                                |view, _, _, cx| {
                                                    view.format.update(cx, |f, cx| {
                                                        *f = DataImportFormat::Csv;
                                                        cx.notify();
                                                    });
                                                },
                                            ))
                                        })
                                        .child({
                                            let is_selected =
                                                current_format == DataImportFormat::Json;
                                            let mut btn = Button::new("format_json").child("JSON");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(
                                                &cx.entity(),
                                                |view, _, _, cx| {
                                                    view.format.update(cx, |f, cx| {
                                                        *f = DataImportFormat::Json;
                                                        cx.notify();
                                                    });
                                                },
                                            ))
                                        })
                                        .child({
                                            let is_selected =
                                                current_format == DataImportFormat::Xml;
                                            let mut btn = Button::new("format_xml").child("XML");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(
                                                &cx.entity(),
                                                |view, _, _, cx| {
                                                    view.format.update(cx, |f, cx| {
                                                        *f = DataImportFormat::Xml;
                                                        cx.notify();
                                                    });
                                                },
                                            ))
                                        }),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_3()
                                .p_3()
                                .rounded_md()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(t!("ImportExport.import_options")),
                                )
                                .child(
                                    v_flex().gap_2().child(
                                        h_flex()
                                            .gap_8()
                                            .child(
                                                h_flex()
                                                    .w(px(140.))
                                                    .gap_2()
                                                    .items_center()
                                                    .child(
                                                        Switch::new("has_header")
                                                            .checked(*self.has_header.read(cx))
                                                            .on_click(cx.listener(
                                                                |view, checked, _, cx| {
                                                                    view.has_header.update(
                                                                        cx,
                                                                        |state, cx| {
                                                                            *state = *checked;
                                                                            cx.notify();
                                                                        },
                                                                    );
                                                                },
                                                            )),
                                                    )
                                                    .child(t!("ImportExport.has_header")),
                                            )
                                            .child(
                                                h_flex()
                                                    .w(px(120.))
                                                    .gap_2()
                                                    .items_center()
                                                    .child(
                                                        Switch::new("stop_on_error")
                                                            .checked(*self.stop_on_error.read(cx))
                                                            .on_click(cx.listener(
                                                                |view, checked, _, cx| {
                                                                    view.stop_on_error.update(
                                                                        cx,
                                                                        |state, cx| {
                                                                            *state = *checked;
                                                                            cx.notify();
                                                                        },
                                                                    );
                                                                },
                                                            )),
                                                    )
                                                    .child(t!("ImportExport.stop_on_error")),
                                            )
                                            .child(
                                                h_flex()
                                                    .w(px(120.))
                                                    .gap_2()
                                                    .items_center()
                                                    .child(
                                                        Switch::new("use_transaction")
                                                            .checked(*self.use_transaction.read(cx))
                                                            .on_click(cx.listener(
                                                                |view, checked, _, cx| {
                                                                    view.use_transaction.update(
                                                                        cx,
                                                                        |state, cx| {
                                                                            *state = *checked;
                                                                            cx.notify();
                                                                        },
                                                                    );
                                                                },
                                                            )),
                                                    )
                                                    .child(t!("ImportExport.use_transaction")),
                                            )
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .items_center()
                                                    .child(
                                                        Switch::new("truncate_before")
                                                            .checked(*self.truncate_before.read(cx))
                                                            .on_click(cx.listener(
                                                                |view, checked, _, cx| {
                                                                    view.truncate_before.update(
                                                                        cx,
                                                                        |state, cx| {
                                                                            *state = *checked;
                                                                            cx.notify();
                                                                        },
                                                                    );
                                                                },
                                                            )),
                                                    )
                                                    .child(t!("ImportExport.truncate_before_import")),
                                            ),
                                    ),
                                )
                                .when(current_format.needs_delimiter_config(), |this| {
                                    this.child(div().h(px(1.)).w_full().bg(cx.theme().border))
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .child(t!("ImportExport.delimiter_config")),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_4()
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                            .text_color(
                                                                cx.theme().muted_foreground,
                                                            )
                                                            .child(t!("ImportExport.record_delimiter")),
                                                        )
                                                        .child(
                                                            Select::new(&self.record_separator)
                                                                .w_full(),
                                                        ),
                                                )
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                            .text_color(
                                                                cx.theme().muted_foreground,
                                                            )
                                                            .child(t!("ImportExport.field_delimiter")),
                                                        )
                                                        .child(
                                                            Select::new(&self.field_separator)
                                                                .w_full(),
                                                        ),
                                                )
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                            .text_color(
                                                                cx.theme().muted_foreground,
                                                            )
                                                            .child(t!("ImportExport.text_qualifier")),
                                                        )
                                                        .child(
                                                            Select::new(&self.text_qualifier)
                                                                .w_full(),
                                                        ),
                                                ),
                                        )
                                }),
                        ),
                )
            })
            .when(current_step == ImportStep::Execute, |this| {
                let file_path = self.file_path.read(cx).text().to_string();
                this.child(
                    v_flex()
                        .flex_1()
                        .gap_2()
                        .child(
                            h_flex()
                                .justify_between()
                                .gap_6()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{}:", t!("ImportExport.file"))),
                                        )
                                        .child(
                                            div()
                                                .id(SharedString::from("table_import_file_path"))
                                                .max_w(px(500.))
                                                .text_ellipsis()
                                                .overflow_hidden()
                                                .child(file_path.clone())
                                                .tooltip(move |_w, cx| {
                                                    cx.new(|_| Tooltip::new(file_path.clone()))
                                                        .into()
                                                }),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child(format!("{}:", t!("ImportExport.processed"))),
                                                )
                                                .child(div().child(processed.to_string())),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child(format!("{}:", t!("ImportExport.time"))),
                                                )
                                                .child(div().child(elapsed)),
                                        ),
                                ),
                        )
                        .when(errors > 0, |this| {
                            this.child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_color(cx.theme().danger)
                                            .child(format!("{}:", t!("ImportExport.errors"))),
                                    )
                                    .child(
                                        div()
                                            .text_color(cx.theme().danger)
                                            .child(errors.to_string()),
                                    ),
                            )
                        })
                        .child({
                            let chars_per_line = 100;
                            let line_height = 20.0_f32;
                            let min_height = line_height;
                            let max_height = 80.0_f32;

                            let item_sizes = Rc::new(
                                logs.iter()
                                    .map(|entry| {
                                        let text_len = if entry.table.is_empty() {
                                            entry.message.len() + 6
                                        } else {
                                            entry.table.len() + entry.message.len() + 8
                                        };
                                        let lines = ((text_len as f32 / chars_per_line as f32)
                                            .ceil()
                                            as i32)
                                            .max(1);
                                        let height = (lines as f32 * line_height)
                                            .clamp(min_height, max_height);
                                        gpui::size(px(0.), px(height))
                                    })
                                    .collect::<Vec<_>>(),
                            );

                            div()
                                .flex_1()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_md()
                                .overflow_hidden()
                                .bg(cx.theme().background)
                                .p_2()
                                .child(
                                    v_virtual_list(
                                        cx.entity().clone(),
                                        "logs-virtual-list",
                                        item_sizes.clone(),
                                        move |view, visible_range, _window, cx| {
                                            let logs = view.logs.read(cx);
                                            visible_range
                                                .into_iter()
                                                .filter_map(|idx| {
                                                    logs.get(idx).map(|entry| {
                                                        let text = if entry.table.is_empty() {
                                                            format!("[IMP] {}", entry.message)
                                                        } else {
                                                            format!(
                                                                "[IMP] {}> {}",
                                                                entry.table, entry.message
                                                            )
                                                        };
                                                        let item_height = item_sizes
                                                            .get(idx)
                                                            .map(|s| s.height)
                                                            .unwrap_or(px(20.));
                                                        div()
                                                            .id(("log-entry", idx))
                                                            .w_full()
                                                            .text_xs()
                                                            .h(item_height)
                                                            .child(text)
                                                    })
                                                })
                                                .collect()
                                        },
                                    )
                                    .size_full()
                                    .track_scroll(&self.scroll_handle),
                                )
                        })
                        .child(
                            div()
                                .h_2()
                                .w_full()
                                .rounded_full()
                                .bg(cx.theme().primary.opacity(0.2))
                                .child(
                                    div()
                                        .h_full()
                                        .rounded_full()
                                        .bg(cx.theme().primary)
                                        .w(gpui::relative(progress_value / 100.0)),
                                ),
                        ),
                )
            })
            .child(
                h_flex()
                    .gap_2()
                    .pt_3()
                    .mt_1()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .justify_end()
                    .child(
                        Button::new("cancel")
                            .child(t!("Common.cancel"))
                            .on_click(|_, window, _cx| {
                                window.remove_window();
                            }),
                    )
                    .when(current_step == ImportStep::Execute && !is_running, |this| {
                        this.child(
                            Button::new("prev")
                                .child(t!("Common.previous"))
                                .disabled(is_finished)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.current_step = ImportStep::Config;
                                    cx.notify();
                                })),
                        )
                    })
                    .when(current_step == ImportStep::Config, |this| {
                        this.child(Button::new("next").primary().child(t!("Common.next")).on_click(
                            cx.listener(|view, _, _, cx| {
                                let file_path = view.file_path.read(cx).text().to_string();
                                if file_path.is_empty() {
                                    view.validation_error.update(cx, |e, cx| {
                                        *e = Some(t!("ImportExport.please_select_file").to_string());
                                        cx.notify();
                                    });
                                    return;
                                }
                                view.validation_error.update(cx, |e, _| *e = None);
                                view.current_step = ImportStep::Execute;
                                cx.notify();
                            }),
                        ))
                    })
                    .when(
                        current_step == ImportStep::Execute && !is_running && !is_finished,
                        |this| {
                            this.child(Button::new("start").primary().child(t!("ImportExport.start_import")).on_click(
                                window.listener_for(&cx.entity(), |view, _, window, cx| {
                                    view.start_import(window, cx);
                                }),
                            ))
                        },
                    )
                    .when(is_running, |this| {
                        this.child(Button::new("running").loading(true).child(t!("ImportExport.importing")))
                    })
                    .when(is_finished, |this| {
                        this.child(Button::new("close").primary().child(t!("Common.finish")).on_click(
                            |_, window, cx| {
                                gpui_component::WindowExt::close_dialog(window, cx);
                            },
                        ))
                    }),
            )
    }
}
