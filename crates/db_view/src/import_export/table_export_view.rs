use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, PathPromptOptions, Render, Styled, Task, Window, div, prelude::FluentBuilder,
    px,
};
use gpui_component::{
    ActiveTheme, Disableable, IconName, IndexPath, Sizable, VirtualListScrollHandle,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
    select::{Select, SelectItem, SelectState},
    switch::Switch,
    v_flex, v_virtual_list,
};
use tokio::sync::mpsc;

use db::{
    ColumnInfo, CsvExportConfig, DataFormat, ExportConfig, ExportProgressEvent, GlobalDbState,
};
use one_core::storage::get_download_dir;
use rust_i18n::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataExportFormat {
    Txt,
    Csv,
    Json,
    Xml,
}

impl DataExportFormat {
    pub fn to_data_format(self) -> DataFormat {
        match self {
            Self::Txt => DataFormat::Txt,
            Self::Csv => DataFormat::Csv,
            Self::Json => DataFormat::Json,
            Self::Xml => DataFormat::Xml,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Xml => "xml",
        }
    }

    pub fn needs_delimiter_config(self) -> bool {
        matches!(self, Self::Txt | Self::Csv)
    }
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
pub enum ExportStep {
    Config,
    Execute,
}

#[derive(Debug, Clone)]
pub struct ColumnItem {
    pub name: String,
    pub selected: bool,
}

#[derive(Debug, Clone)]
struct LogEntry {
    table: String,
    message: String,
}

pub struct ColumnListDelegate {
    columns: Vec<ColumnItem>,
    selected_index: Option<IndexPath>,
}

impl ColumnListDelegate {
    pub fn new(columns: Vec<ColumnItem>) -> Self {
        Self {
            columns,
            selected_index: None,
        }
    }

    pub fn select_all(&mut self) {
        for col in &mut self.columns {
            col.selected = true;
        }
    }

    pub fn deselect_all(&mut self) {
        for col in &mut self.columns {
            col.selected = false;
        }
    }

    pub fn toggle(&mut self, index: usize) {
        if let Some(col) = self.columns.get_mut(index) {
            col.selected = !col.selected;
        }
    }

    pub fn selected_columns(&self) -> Vec<String> {
        self.columns
            .iter()
            .filter(|c| c.selected)
            .map(|c| c.name.clone())
            .collect()
    }

    pub fn set_columns(&mut self, columns: Vec<ColumnItem>) {
        self.columns = columns;
    }
}

impl ListDelegate for ColumnListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.columns.len()
    }

    fn perform_search(
        &mut self,
        _query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        Task::ready(())
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let index = ix.row;
        let col = self.columns.get(index)?;
        let col_name = col.name.clone();
        let checked = col.selected;

        Some(
            ListItem::new(("col-item", index)).py_1().child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new(("col-check", index))
                            .checked(checked)
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.delegate_mut().toggle(index);
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().child(col_name)),
            ),
        )
    }
}

pub struct DataExportView {
    connection_id: String,
    server_info: String,
    database: String,
    schema: Option<String>,
    table: String,
    output_path: Entity<InputState>,

    current_step: ExportStep,
    format: Entity<DataExportFormat>,
    column_list: Entity<ListState<ColumnListDelegate>>,

    record_separator: Entity<SelectState<Vec<RecordSeparator>>>,
    field_separator: Entity<SelectState<Vec<FieldSeparator>>>,
    text_qualifier: Entity<SelectState<Vec<TextQualifierItem>>>,
    include_header: Entity<bool>,

    logs: Entity<Vec<LogEntry>>,
    scroll_handle: VirtualListScrollHandle,

    processed_records: Entity<u64>,
    error_count: Entity<u32>,
    transferred_records: Entity<u64>,
    elapsed_time: Entity<String>,
    progress: Entity<f32>,

    is_running: Entity<bool>,
    is_finished: Entity<bool>,
    start_time: Option<Instant>,

    focus_handle: FocusHandle,
}

impl DataExportView {
    pub fn new(
        connection_id: impl Into<String>,
        database: impl Into<String>,
        schema: Option<String>,
        table: impl Into<String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let global_db_state = cx.global::<GlobalDbState>();
        let connection_id = connection_id.into();
        let config = global_db_state.get_config(&connection_id);

        let delegate = ColumnListDelegate::new(vec![]);

        let text_qualifier_items = vec![
            TextQualifierItem::new(t!("ImportExport.qualifier_none"), ""),
            TextQualifierItem::new(t!("ImportExport.qualifier_double_quote"), "\""),
            TextQualifierItem::new(t!("ImportExport.qualifier_single_quote"), "'"),
        ];

        let default_path = get_download_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        Self {
            connection_id: connection_id.into(),
            server_info: if let Some(c) = config {
                c.server_info()
            } else {
                "".to_string()
            },
            database: database.into(),
            schema,
            table: table.into(),
            output_path: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("ImportExport.export_directory_placeholder"))
                    .default_value(default_path)
            }),
            current_step: ExportStep::Config,
            format: cx.new(|_| DataExportFormat::Csv),
            column_list: cx.new(|cx| ListState::new(delegate, window, cx)),

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
            include_header: cx.new(|_| true),

            logs: cx.new(|_| Vec::new()),
            scroll_handle: VirtualListScrollHandle::new(),

            processed_records: cx.new(|_| 0),
            error_count: cx.new(|_| 0),
            transferred_records: cx.new(|_| 0),
            elapsed_time: cx.new(|_| "0.00s".to_string()),
            progress: cx.new(|_| 0.0),

            is_running: cx.new(|_| false),
            is_finished: cx.new(|_| false),
            start_time: None,

            focus_handle: cx.focus_handle(),
        }
    }

    pub fn update_column_list(&mut self, columns: Vec<ColumnInfo>, cx: &mut Context<Self>) {
        let column_items: Vec<ColumnItem> = columns
            .into_iter()
            .map(|col| ColumnItem {
                name: col.name,
                selected: true,
            })
            .collect();
        self.column_list.update(cx, |list, _cx| {
            list.delegate_mut().set_columns(column_items)
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

    fn select_all_columns(&mut self, _window: &mut Window, cx: &mut App) {
        self.column_list.update(cx, |list, cx| {
            list.delegate_mut().select_all();
            cx.notify();
        });
    }

    fn deselect_all_columns(&mut self, _window: &mut Window, cx: &mut App) {
        self.column_list.update(cx, |list, cx| {
            list.delegate_mut().deselect_all();
            cx.notify();
        });
    }

    fn select_directory(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let output_path = self.output_path.clone();
        let future = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t!("ImportExport.select_export_directory").into()),
        });
        cx.spawn(async move |_this, cx| {
            if let Ok(Ok(Some(paths))) = future.await {
                if let Some(selected_path) = paths.into_iter().next() {
                    let path_str = selected_path.display().to_string();
                    let _ = cx.update(|cx: &mut App| {
                        if let Some(window_id) = cx.active_window() {
                            let _ = cx.update_window(window_id, |_, window, cx| {
                                output_path.update(cx, |state, cx| {
                                    state.set_value(path_str, window, cx);
                                });
                            });
                        }
                    });
                }
            }
        })
        .detach();
    }

    fn start_export(&mut self, _window: &mut Window, cx: &mut App) {
        if *self.is_running.read(cx) {
            return;
        }

        let selected_columns: Vec<String> = self.column_list.read(cx).delegate().selected_columns();

        if selected_columns.is_empty() {
            self.logs.update(cx, |l, cx| {
                l.push(LogEntry {
                    table: "".to_string(),
                    message: t!("ImportExport.select_at_least_one_column").to_string(),
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
        let output_path = PathBuf::from(self.output_path.read(cx).text().to_string());
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

        let include_header = *self.include_header.read(cx);

        let csv_config = if format.needs_delimiter_config() {
            Some(CsvExportConfig {
                field_delimiter,
                text_qualifier,
                include_header,
                record_terminator,
            })
        } else {
            None
        };

        let logs = self.logs.clone();
        let scroll_handle = self.scroll_handle.clone();
        let processed_records = self.processed_records.clone();
        let error_count = self.error_count.clone();
        let transferred_records = self.transferred_records.clone();
        let elapsed_time = self.elapsed_time.clone();
        let progress = self.progress.clone();
        let is_running = self.is_running.clone();
        let is_finished = self.is_finished.clone();
        let start_time = self.start_time;

        let now = chrono::Local::now();
        let datetime_str = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let filename = format!(
            "{}_{}_{}.{}",
            database,
            table,
            datetime_str,
            format.extension()
        );
        let full_path = output_path.join(&filename);

        cx.spawn(async move |cx| {
            Self::add_log(
                &cx,
                &logs,
                &scroll_handle,
                "".to_string(),
                t!("ImportExport.export_table_log", table = table).to_string(),
            );

            let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ExportProgressEvent>();

            let export_config = ExportConfig {
                format: format.to_data_format(),
                database: database.clone(),
                schema: schema.clone(),
                tables: vec![table.clone()],
                columns: Some(selected_columns),
                include_schema: false,
                include_data: true,
                where_clause: None,
                limit: None,
                csv_config,
            };

            let global_state_clone = global_state.clone();
            let connection_id_clone = connection_id.clone();

            let export_handle = cx.background_spawn(async move {
                global_state_clone
                    .export_data_with_progress_sync(
                        connection_id_clone,
                        export_config,
                        Some(progress_tx),
                    )
                    .await
            });

            let file_path_for_write = full_path.clone();
            let mut file_created = false;

            while let Some(event) = progress_rx.recv().await {
                let event_clone = event.clone();
                let logs_clone = logs.clone();
                let scroll_handle_clone = scroll_handle.clone();
                let processed_records_clone = processed_records.clone();
                let error_count_clone = error_count.clone();
                let transferred_records_clone = transferred_records.clone();
                let elapsed_time_clone = elapsed_time.clone();
                let progress_clone = progress.clone();

                match &event_clone {
                    ExportProgressEvent::DataExported { data, .. } => {
                        if !data.is_empty() {
                            let write_result = if !file_created {
                                file_created = true;
                                std::fs::write(&file_path_for_write, data)
                            } else {
                                std::fs::OpenOptions::new()
                                    .append(true)
                                    .open(&file_path_for_write)
                                    .and_then(|mut f| f.write_all(data.as_bytes()))
                            };
                            if let Err(e) = write_result {
                                let logs_for_error = logs_clone.clone();
                                let scroll_for_error = scroll_handle_clone.clone();
                                let error_count_for_error = error_count_clone.clone();
                                let _ = cx.update(|cx| {
                                    logs_for_error.update(cx, |l, cx| {
                                        l.push(LogEntry {
                                            table: "".to_string(),
                                            message: t!(
                                                "ImportExport.write_file_failed",
                                                error = e
                                            )
                                            .to_string(),
                                        });
                                        cx.notify();
                                    });
                                    error_count_for_error.update(cx, |e, cx| {
                                        *e += 1;
                                        cx.notify();
                                    });
                                    scroll_for_error.scroll_to_bottom();
                                });
                            }
                        }
                    }
                    _ => {}
                }

                let _ = cx.update(|cx| {
                    let elapsed = start_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);

                    elapsed_time_clone.update(cx, |t, cx| {
                        *t = format!("{:.2}s", elapsed);
                        cx.notify();
                    });

                    match event_clone {
                        ExportProgressEvent::TableStart {
                            table,
                            table_index,
                            total_tables,
                        } => {
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: table.clone(),
                                    message: t!(
                                        "ImportExport.export_start",
                                        current = table_index + 1,
                                        total = total_tables
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                            let p = (table_index as f32 / total_tables as f32) * 100.0;
                            progress_clone.update(cx, |pr, cx| {
                                *pr = p;
                                cx.notify();
                            });
                        }
                        ExportProgressEvent::FetchingData { table } => {
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: table.clone(),
                                    message: t!("ImportExport.fetching_data").to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ExportProgressEvent::DataExported { table, rows, .. } => {
                            transferred_records_clone.update(cx, |r, cx| {
                                *r += rows;
                                cx.notify();
                            });
                            processed_records_clone.update(cx, |r, cx| {
                                *r += rows;
                                cx.notify();
                            });
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: table.clone(),
                                    message: t!(
                                        "ImportExport.transfer_records",
                                        rows = rows
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ExportProgressEvent::TableFinished { table } => {
                            let elapsed =
                                start_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: table.clone(),
                                    message: t!(
                                        "ImportExport.export_finished",
                                        seconds = format!("{:.3}", elapsed)
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ExportProgressEvent::Error { table, message } => {
                            error_count_clone.update(cx, |e, cx| {
                                *e += 1;
                                cx.notify();
                            });
                            logs_clone.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: table.clone(),
                                    message: t!(
                                        "ImportExport.export_error_with_message",
                                        message = message
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        ExportProgressEvent::Finished {
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
                                        "ImportExport.export_complete_summary",
                                        rows = total_rows,
                                        elapsed_ms = elapsed_ms
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                        _ => {}
                    }

                    scroll_handle_clone.scroll_to_bottom();
                });
            }

            let result = export_handle.await;

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
                    Ok(_) => {
                        if file_created {
                            logs.update(cx, |l, cx| {
                                l.push(LogEntry {
                                    table: "".to_string(),
                                    message: t!(
                                        "ImportExport.file_saved",
                                        path = full_path.display()
                                    )
                                    .to_string(),
                                });
                                cx.notify();
                            });
                        }
                    }
                    Err(e) => {
                        logs.update(cx, |l, cx| {
                            l.push(LogEntry {
                                table: "".to_string(),
                                message: t!("ImportExport.export_failed", error = e).to_string(),
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

impl Focusable for DataExportView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Clone for DataExportView {
    fn clone(&self) -> Self {
        Self {
            connection_id: self.connection_id.clone(),
            server_info: self.server_info.clone(),
            database: self.database.clone(),
            schema: self.schema.clone(),
            table: self.table.clone(),
            output_path: self.output_path.clone(),
            current_step: self.current_step,
            format: self.format.clone(),
            column_list: self.column_list.clone(),
            record_separator: self.record_separator.clone(),
            field_separator: self.field_separator.clone(),
            text_qualifier: self.text_qualifier.clone(),
            include_header: self.include_header.clone(),
            logs: self.logs.clone(),
            scroll_handle: self.scroll_handle.clone(),
            processed_records: self.processed_records.clone(),
            error_count: self.error_count.clone(),
            transferred_records: self.transferred_records.clone(),
            elapsed_time: self.elapsed_time.clone(),
            progress: self.progress.clone(),
            is_running: self.is_running.clone(),
            is_finished: self.is_finished.clone(),
            start_time: self.start_time,
            focus_handle: self.focus_handle.clone(),
        }
    }
}

impl Render for DataExportView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_running = *self.is_running.read(cx);
        let is_finished = *self.is_finished.read(cx);
        let current_format = *self.format.read(cx);
        let progress_value = *self.progress.read(cx);
        let processed = *self.processed_records.read(cx);
        let errors = *self.error_count.read(cx);
        let transferred = *self.transferred_records.read(cx);
        let elapsed = self.elapsed_time.read(cx).clone();
        let logs = self.logs.read(cx).clone();
        let current_step = self.current_step;

        v_flex()
            .w_full()
            .h(px(540.0))
            .gap_2()
            .p_4()
            .pt_8()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if current_step == ExportStep::Config {
                        t!("ImportExport.export_hint")
                    } else {
                        t!("ImportExport.export_ready_hint")
                    })
            )
            .when(current_step == ExportStep::Config, |this| {
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
                                .child(div().text_ellipsis().overflow_hidden().child(self.server_info.clone()))
                        )
                                .child(
                                    h_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{}:", t!("Database.database"))),
                                )
                                .child(div().text_ellipsis().overflow_hidden().child(self.database.clone()))
                        )
                                .child(
                                    h_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{}:", t!("Common.table"))),
                                )
                                .child(div().text_ellipsis().overflow_hidden().child(self.table.clone()))
                        )
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
                                        .child(Input::new(&self.output_path).flex_1())
                                        .child(
                                            Button::new("browse")
                                                .icon(IconName::Folder)
                                                .on_click(window.listener_for(&cx.entity(), |view, _, window, cx| {
                                                    view.select_directory(window, cx);
                                                }))
                                        )
                                )
                        )
                        .child(
                            div()
                                .h(px(1.))
                                .w_full()
                                .bg(cx.theme().border)
                        )
                        .child(
                            v_flex()
                                .gap_3()
                                .p_3()
                                .rounded_md()
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .child({
                                            let is_selected = current_format == DataExportFormat::Txt;
                                            let mut btn = Button::new("format_txt").child("TXT");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(&cx.entity(), |view, _, _, cx| {
                                                view.format.update(cx, |f, cx| {
                                                    *f = DataExportFormat::Txt;
                                                    cx.notify();
                                                });
                                            }))
                                        })
                                        .child({
                                            let is_selected = current_format == DataExportFormat::Csv;
                                            let mut btn = Button::new("format_csv").child("CSV");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(&cx.entity(), |view, _, _, cx| {
                                                view.format.update(cx, |f, cx| {
                                                    *f = DataExportFormat::Csv;
                                                    cx.notify();
                                                });
                                            }))
                                        })
                                        .child({
                                            let is_selected = current_format == DataExportFormat::Json;
                                            let mut btn = Button::new("format_json").child("JSON");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(&cx.entity(), |view, _, _, cx| {
                                                view.format.update(cx, |f, cx| {
                                                    *f = DataExportFormat::Json;
                                                    cx.notify();
                                                });
                                            }))
                                        })
                                        .child({
                                            let is_selected = current_format == DataExportFormat::Xml;
                                            let mut btn = Button::new("format_xml").child("XML");
                                            if is_selected {
                                                btn = btn.primary();
                                            }
                                            btn.on_click(window.listener_for(&cx.entity(), |view, _, _, cx| {
                                                view.format.update(cx, |f, cx| {
                                                    *f = DataExportFormat::Xml;
                                                    cx.notify();
                                                });
                                            }))
                                        })
                                )
                        )
                        .child(
                            h_flex()
                                .gap_3()
                                .flex_1()
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .h_full()
                                        .gap_1()
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .child(t!("ImportExport.export_columns")),
                                                )
                                                .child(
                                                    h_flex()
                                                        .gap_1()
                                                        .child(
                                                            Button::new("select_all")
                                                                .small()
                                                                .child(t!("Common.select_all"))
                                                                .on_click(window.listener_for(&cx.entity(), |view, _, window, cx| {
                                                                    view.select_all_columns(window, cx);
                                                                }))
                                                        )
                                                        .child(
                                                            Button::new("deselect_all")
                                                                .small()
                                                                .child(t!("Common.deselect_all"))
                                                                .on_click(window.listener_for(&cx.entity(), |view, _, window, cx| {
                                                                    view.deselect_all_columns(window, cx);
                                                                }))
                                                        )
                                                )
                                        )
                                        .child(
                                            div()
                                                .border_1()
                                                .border_color(cx.theme().border)
                                                .rounded_md()
                                                .flex_1()
                                                .overflow_hidden()
                                                .child(List::new(&self.column_list))
                                        )
                                )
                                .when(current_format.needs_delimiter_config(), |this| {
                                    this.child(
                                        v_flex()
                                            .w(px(200.0))
                                            .h_full()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .child(t!("ImportExport.delimiter_config")),
                                            )
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .gap_2()
                                                    .p_2()
                                                    .border_1()
                                                    .border_color(cx.theme().border)
                                                    .rounded_md()
                                                    .child(
                                                        v_flex()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().muted_foreground)
                                                                    .child(t!("ImportExport.record_delimiter")),
                                                            )
                                                            .child(Select::new(&self.record_separator).w_full())
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().muted_foreground)
                                                                    .child(t!("ImportExport.field_delimiter")),
                                                            )
                                                            .child(Select::new(&self.field_separator).w_full())
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().muted_foreground)
                                                                    .child(t!("ImportExport.text_qualifier")),
                                                            )
                                                            .child(Select::new(&self.text_qualifier).w_full())
                                                    )
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .items_center()
                                                            .child(
                                                                Switch::new("include_header")
                                                                    .checked(*self.include_header.read(cx))
                                                                    .on_click(cx.listener(|view, checked, _, cx| {
                                                                        view.include_header.update(cx, |state, cx| {
                                                                            *state = *checked;
                                                                            cx.notify();
                                                                        });
                                                                    }))
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().muted_foreground)
                                                                    .child(t!("ImportExport.has_header")),
                                                            )
                                                    )
                                            )
                                    )
                                })
                        )
                )
            })
            .when(current_step == ExportStep::Execute, |this| {
                this.child(
                    v_flex()
                        .flex_1()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_6()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{}:", t!("ImportExport.source_object"))),
                                        )
                                        .child(div().child("1")),
                                )
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{}:", t!("ImportExport.total"))),
                                        )
                                        .child(div().child(transferred.to_string())),
                                )
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
                                    .child(div().text_color(cx.theme().danger).child(errors.to_string())),
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
                                        let lines = ((text_len as f32 / chars_per_line as f32).ceil() as i32).max(1);
                                        let height = (lines as f32 * line_height).clamp(min_height, max_height);
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
                                                            format!("[EXP] {}", entry.message)
                                                        } else {
                                                            format!("[EXP] {}> {}", entry.table, entry.message)
                                                        };
                                                        let item_height = item_sizes.get(idx).map(|s| s.height).unwrap_or(px(20.));
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
                                    .track_scroll(&self.scroll_handle)
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
                                        .w(gpui::relative(progress_value / 100.0))
                                ),
                        )
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
                            })
                    )
                    .when(current_step == ExportStep::Execute && !is_running, |this| {
                        this.child(
                            Button::new("prev")
                                .child(t!("Common.previous"))
                                .disabled(is_finished)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.current_step = ExportStep::Config;
                                    cx.notify();
                                }))
                        )
                    })
                    .when(current_step == ExportStep::Config, |this| {
                        this.child(
                            Button::new("next")
                                .primary()
                                .child(t!("Common.next"))
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.current_step = ExportStep::Execute;
                                    cx.notify();
                                }))
                        )
                    })
                    .when(current_step == ExportStep::Execute && !is_running && !is_finished, |this| {
                        this.child(
                            Button::new("start")
                                .primary()
                                .child(t!("ImportExport.start_export"))
                                .on_click(window.listener_for(&cx.entity(), |view, _, window, cx| {
                                    view.start_export(window, cx);
                                }))
                        )
                    })
                    .when(is_running, |this| {
                        this.child(
                            Button::new("running")
                                .loading(true)
                                .child(t!("ImportExport.exporting"))
                        )
                    })
                    .when(is_finished, |this| {
                        this.child(
                            Button::new("close")
                                .primary()
                                .child(t!("Common.finish"))
                                .on_click(|_, window, _cx| {
                                    window.remove_window();
                                })
                        )
                    }),
            )
    }
}
