// 2. 外部 crate 导入（按字母顺序）
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, AsyncApp, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, UniformListScrollHandle, Window, div, px, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, StyledExt,
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    h_flex,
    popover::Popover,
    progress::Progress,
    scroll::ScrollableElement,
    spinner::Spinner,
    tab::{Tab, TabBar},
    v_flex,
};
use one_ui::edit_table::Column;
use smol::Timer;
use tracing::log::error;

use crate::table_data::data_grid::{DataGrid, DataGridConfig, DataGridUsage};
use one_core::ai_chat::ask_ai::AskAiButton;
// 3. 当前 crate 导入（按模块分组）
use db::{GlobalDbState, SqlResult, SqlSource};
use gpui_component::checkbox::Checkbox;
use rust_i18n::t;

// Structure to hold a single SQL result with its metadata
#[derive(Clone)]
pub struct SqlResultTab {
    pub sql: String,
    pub result: SqlResult,
    pub execution_time: String,
    pub rows_count: String,
    pub data_grid: Option<Entity<DataGrid>>,
}

/// 执行状态
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionState {
    Idle,
    Executing { current: usize, total: usize },
    Completed,
}

/// 语句列表项 - 用于虚拟滚动列表
#[derive(Clone)]
pub struct StatementListItem {
    pub sql: String,
    pub elapsed_ms: u128,
    pub is_error: bool,
    pub status_text: String,
    pub full_error_message: Option<String>,
    pub truncated_sql: Option<String>,
}

/// 语句列表数据
pub struct StatementListData {
    items: Vec<StatementListItem>,
    show_errors_only: bool,
    cached_filtered_items: Vec<StatementListItem>,
}

impl StatementListData {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            show_errors_only: false,
            cached_filtered_items: Vec::new(),
        }
    }

    fn truncate_text(text: &str, max_length: usize, clean_whitespace: bool) -> String {
        let text = if clean_whitespace {
            text.replace('\n', " ").replace('\r', "")
        } else {
            text.to_string()
        };
        if text.chars().count() <= max_length {
            text
        } else {
            let truncated: String = text.chars().take(max_length).collect();
            format!("{}...", truncated)
        }
    }

    pub fn set_items(&mut self, results: &[SqlResult]) {
        self.items = results
            .iter()
            .map(|result| match result {
                SqlResult::Query(q) => StatementListItem {
                    sql: q.sql.clone(),
                    elapsed_ms: q.elapsed_ms,
                    is_error: false,
                    status_text: format!("{} rows", q.rows.len()),
                    full_error_message: None,
                    truncated_sql: Some(Self::truncate_text(&q.sql, 50, true)),
                },
                SqlResult::Exec(e) => StatementListItem {
                    sql: e.sql.clone(),
                    elapsed_ms: e.elapsed_ms,
                    is_error: false,
                    status_text: format!("{} rows affected", e.rows_affected),
                    full_error_message: None,
                    truncated_sql: Some(Self::truncate_text(&e.sql, 50, true)),
                },
                SqlResult::Error(e) => StatementListItem {
                    sql: e.sql.clone(),
                    elapsed_ms: 0,
                    is_error: true,
                    status_text: Self::truncate_text(&e.message, 50, false),
                    full_error_message: Some(e.message.clone()),
                    truncated_sql: Some(Self::truncate_text(&e.sql, 50, true)),
                },
            })
            .collect();
        self.invalidate_cache();
    }

    pub fn set_show_errors_only(&mut self, show_errors_only: bool) {
        self.show_errors_only = show_errors_only;
        self.invalidate_cache();
    }

    fn invalidate_cache(&mut self) {
        self.cached_filtered_items = self
            .items
            .iter()
            .filter(|item| !self.show_errors_only || item.is_error)
            .cloned()
            .collect();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.cached_filtered_items.clear();
        self.show_errors_only = false;
    }

    pub fn filtered_items(&self) -> &[StatementListItem] {
        &self.cached_filtered_items
    }
}

#[derive(Clone)]
pub struct SqlResultTabContainer {
    pub result_tabs: Entity<Vec<SqlResultTab>>,
    pub active_result_tab: Entity<usize>,
    pub all_results: Entity<Vec<SqlResult>>,
    pub is_visible: Entity<bool>,
    pub execution_state: Entity<ExecutionState>,
    pub statement_list: Entity<StatementListData>,
    pub scroll_handle: UniformListScrollHandle,
    pub show_errors_only: Entity<bool>,
    pub total_elapsed_ms: Entity<f64>,
    /// 执行开始时间，用于实时更新运行时间
    pub execution_start: Entity<Option<Instant>>,
    /// 停止计时器的标志
    timer_stop_flag: Arc<AtomicBool>,
}

impl SqlResultTabContainer {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> SqlResultTabContainer {
        let result_tabs = cx.new(|_| vec![]);
        let active_result_tab = cx.new(|_| 0);
        let all_results = cx.new(|_| vec![]);
        let is_visible = cx.new(|_| false);
        let execution_state = cx.new(|_| ExecutionState::Idle);
        let statement_list = cx.new(|_| StatementListData::new());
        let scroll_handle = UniformListScrollHandle::new();
        let show_errors_only = cx.new(|_| false);
        let total_elapsed_ms = cx.new(|_| 0.0);
        let execution_start = cx.new(|_| None);
        let timer_stop_flag = Arc::new(AtomicBool::new(false));
        SqlResultTabContainer {
            result_tabs,
            active_result_tab,
            all_results,
            is_visible,
            execution_state,
            statement_list,
            scroll_handle,
            show_errors_only,
            total_elapsed_ms,
            execution_start,
            timer_stop_flag,
        }
    }
}

impl SqlResultTabContainer {
    pub fn handle_run_query(
        &mut self,
        sql: String,
        connection_id: String,
        current_database_value: Option<String>,
        current_schema_value: Option<String>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let clone_self = self.clone();
        let connection_id_clone = connection_id.clone();
        let database_clone = current_database_value.clone();

        self.clear_results(cx);

        self.execution_state.update(cx, |state, cx| {
            *state = ExecutionState::Executing {
                current: 0,
                total: 0,
            };
            cx.notify();
        });

        self.active_result_tab.update(cx, |active, cx| {
            *active = 0;
            cx.notify();
        });

        self.is_visible.update(cx, |visible, cx| {
            *visible = true;
            cx.notify();
        });

        // 记录执行开始时间
        let execution_start = Instant::now();
        self.execution_start.update(cx, |start, cx| {
            *start = Some(execution_start);
            cx.notify();
        });

        // 重置运行时间
        self.total_elapsed_ms.update(cx, |elapsed, cx| {
            *elapsed = 0.0;
            cx.notify();
        });

        // 停止之前的计时器
        self.timer_stop_flag.store(true, Ordering::SeqCst);
        // 创建新的停止标志
        let new_stop_flag = Arc::new(AtomicBool::new(false));
        self.timer_stop_flag = new_stop_flag.clone();

        // 启动定时器，每100ms更新运行时间
        let timer_self = self.clone();
        let timer_stop = new_stop_flag.clone();
        cx.spawn(async move |cx: &mut AsyncApp| {
            loop {
                Timer::after(Duration::from_millis(100)).await;

                // 检查是否应该停止
                if timer_stop.load(Ordering::SeqCst) {
                    break;
                }

                // 更新运行时间
                let should_continue = cx.update(|cx| {
                    let start_time = timer_self.execution_start.read(cx);
                    if let Some(start) = *start_time {
                        let elapsed = start.elapsed().as_millis() as f64;
                        timer_self.total_elapsed_ms.update(cx, |ms, cx| {
                            *ms = elapsed;
                            cx.notify();
                        });
                        true
                    } else {
                        false
                    }
                });

                if !should_continue {
                    break;
                }
            }
        })
        .detach();

        cx.spawn(async move |cx: &mut AsyncApp| {
            let (global_state, database_type) = cx.update(|cx| {
                let global_state = cx.global::<GlobalDbState>();
                let config = global_state.get_config(&connection_id);
                let database_type = config
                    .map(|c| c.database_type)
                    .unwrap_or(one_core::storage::DatabaseType::MySQL);
                (global_state.clone(), database_type)
            });

            let exec_opts = db::ExecOptions {
                stop_on_error: false,
                ..Default::default()
            };
            let mut rx = match global_state.execute_streaming(
                cx,
                connection_id_clone.clone(),
                SqlSource::Script(sql.clone()),
                current_database_value,
                current_schema_value,
                Some(exec_opts),
            ) {
                Ok(receiver) => receiver,
                Err(e) => {
                    error!("Error starting streaming execution: {:?}", e);
                    cx.update(|cx| {
                        // 停止计时器
                        clone_self.timer_stop_flag.store(true, Ordering::SeqCst);
                        // 清除开始时间
                        clone_self.execution_start.update(cx, |start, cx| {
                            *start = None;
                            cx.notify();
                        });
                        clone_self.execution_state.update(cx, |state, cx| {
                            *state = ExecutionState::Idle;
                            cx.notify();
                        });
                    });
                    return None;
                }
            };

            let mut has_query_result = false;
            let mut first_query_index: Option<usize> = None;
            let mut query_count = 0usize;

            let mut pending_results: Vec<SqlResult> = Vec::new();
            let mut last_ui_update = std::time::Instant::now();
            const UI_UPDATE_INTERVAL_MS: u128 = 100;
            const BATCH_SIZE: usize = 50;

            loop {
                let progress = match rx.recv().await {
                    Some(p) => p,
                    None => break,
                };

                let (current, total) = (progress.current, progress.total);
                let result = progress.result;

                let is_query = matches!(&result, SqlResult::Query(_));
                if is_query {
                    if first_query_index.is_none() {
                        first_query_index = Some(query_count);
                    }
                    has_query_result = true;
                    query_count += 1;
                }

                pending_results.push(result);

                let should_update_list = pending_results.len() >= BATCH_SIZE
                    || last_ui_update.elapsed().as_millis() >= UI_UPDATE_INTERVAL_MS;

                if should_update_list && !pending_results.is_empty() {
                    let results_to_send = std::mem::take(&mut pending_results);
                    clone_self.update_results_batch(
                        results_to_send,
                        current,
                        total,
                        connection_id_clone.clone(),
                        database_clone.clone(),
                        database_type,
                        cx,
                    );
                    last_ui_update = Instant::now();
                }
            }

            if !pending_results.is_empty() {
                let results_to_send = pending_results;
                cx.update(|cx| {
                    if let Some(window_id) = cx.active_window() {
                        if let Err(err) = cx.update_window(window_id, |_entity, window, cx| {
                            clone_self.add_streaming_results_batch(
                                results_to_send,
                                connection_id_clone.clone(),
                                database_clone.clone(),
                                database_type,
                                window,
                                cx,
                            );
                        }) {
                            error!("Failed to update window after streaming: {:?}", err);
                        }
                    }
                });
            }

            // 最终状态更新
            let total_elapsed = execution_start.elapsed().as_secs_f64();
            cx.update(|cx| {
                // 停止计时器
                clone_self.timer_stop_flag.store(true, Ordering::SeqCst);

                // 清除开始时间
                clone_self.execution_start.update(cx, |start, cx| {
                    *start = None;
                    cx.notify();
                });

                clone_self.execution_state.update(cx, |state, cx| {
                    *state = ExecutionState::Completed;
                    cx.notify();
                });

                clone_self.total_elapsed_ms.update(cx, |t, cx| {
                    *t = total_elapsed * 1000.0;
                    cx.notify();
                });

                if has_query_result {
                    if let Some(idx) = first_query_index {
                        clone_self.active_result_tab.update(cx, |active, cx| {
                            *active = idx + 1;
                            cx.notify();
                        });
                    }
                }
            });
            Some(())
        })
        .detach();
    }

    fn clear_results(&mut self, cx: &mut App) {
        self.result_tabs.update(cx, |tabs, cx| {
            tabs.clear();
            cx.notify();
        });
        self.all_results.update(cx, |results, cx| {
            results.clear();
            cx.notify();
        });
        self.statement_list.update(cx, |list, cx| {
            list.clear();
            cx.notify();
        });
        self.total_elapsed_ms.update(cx, |t, cx| {
            *t = 0.0;
            cx.notify();
        });
        self.show_errors_only.update(cx, |s, cx| {
            *s = false;
            cx.notify();
        });
    }

    fn update_results_batch(
        &self,
        results: Vec<SqlResult>,
        current: usize,
        total: usize,
        connection_id: String,
        database: Option<String>,
        database_type: one_core::storage::DatabaseType,
        cx: &mut AsyncApp,
    ) {
        cx.update(|cx| {
            if let Some(window_id) = cx.active_window() {
                if let Err(err) = cx.update_window(window_id, |_entity, window, cx| {
                    self.execution_state.update(cx, |state, cx| {
                        *state = ExecutionState::Executing { current, total };
                        cx.notify();
                    });

                    self.add_streaming_results_batch(
                        results,
                        connection_id,
                        database,
                        database_type,
                        window,
                        cx,
                    );
                }) {
                    error!("Failed to update window during batch update: {:?}", err);
                }
            }
        });
    }

    /// 批量添加streaming结果并滚动到最新位置
    fn add_streaming_results_batch(
        &self,
        results: Vec<SqlResult>,
        connection_id: String,
        database: Option<String>,
        database_type: one_core::storage::DatabaseType,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let mut new_all_results = Vec::new();
        let mut new_tabs = Vec::new();

        let global_state = cx.global::<GlobalDbState>().clone();
        let plugin = global_state.db_manager.get_plugin(&database_type).ok();
        let db_name = database.as_ref().map_or(String::new(), |s| s.clone());

        for result in results {
            new_all_results.push(result.clone());

            if let SqlResult::Query(query_result) = result {
                let (editable, table_name) = if let Some(ref plugin) = plugin {
                    match plugin.analyze_select_editability(&query_result.sql) {
                        Some(parsed_table_name) => (true, parsed_table_name),
                        None => (false, "".to_string()),
                    }
                } else {
                    (false, "".to_string())
                };

                let config = DataGridConfig::new(
                    db_name.clone(),
                    table_name.clone(),
                    &connection_id,
                    database_type,
                )
                .editable(editable)
                .show_toolbar(true)
                .usage(DataGridUsage::SqlResult)
                .rows_count(query_result.rows.len())
                .execution_time(query_result.elapsed_ms)
                .sql(query_result.sql.clone());

                let data_grid = cx.new(|cx| DataGrid::new(config, _window, cx));

                let columns = query_result
                    .columns
                    .iter()
                    .map(|h| Column::new(h.clone(), h.clone()))
                    .collect();
                let rows: Vec<Vec<Option<String>>> = query_result
                    .rows
                    .iter()
                    .map(|row| row.iter().cloned().collect())
                    .collect();

                data_grid.update(cx, |this, cx| {
                    this.update_data(columns, rows, vec![], cx);
                    this.load_column_meta_if_editable(cx);
                });

                if editable
                    && database_type == one_core::storage::DatabaseType::ClickHouse
                    && !table_name.is_empty()
                {
                    let global_state = global_state.clone();
                    let connection_id = connection_id.clone();
                    let database_name = db_name.clone();
                    let table_name = table_name.clone();
                    let data_grid = data_grid.clone();
                    cx.spawn(async move |cx: &mut AsyncApp| {
                        let result = global_state
                            .list_views_view(cx, connection_id, database_name)
                            .await;
                        let table_name = table_name
                            .split('.')
                            .last()
                            .unwrap_or(&table_name)
                            .to_string();
                        let is_view = result
                            .ok()
                            .map(|view| {
                                view.rows.iter().any(|row| {
                                    row.first().map(|name| name == &table_name).unwrap_or(false)
                                })
                            })
                            .unwrap_or(false);
                        if is_view {
                            cx.update(|cx| {
                                data_grid.update(cx, |grid, cx| {
                                    grid.set_editable(false, cx);
                                });
                            });
                        }
                    })
                    .detach();
                }

                let tab = SqlResultTab {
                    sql: query_result.sql.clone(),
                    result: SqlResult::Query(query_result.clone()),
                    execution_time: format!("{}ms", query_result.elapsed_ms),
                    rows_count: format!("{} rows", query_result.rows.len()),
                    data_grid: Some(data_grid),
                };

                new_tabs.push(tab);
            }
        }

        self.all_results.update(cx, |all_results, _cx| {
            all_results.extend(new_all_results);
        });

        self.result_tabs.update(cx, |tabs, _cx| {
            tabs.extend(new_tabs);
        });

        let all_results_for_update: Vec<SqlResult> = self.all_results.read(cx).clone();
        let statement_list = self.statement_list.clone();
        statement_list.update(cx, |list, cx| {
            list.set_items(&all_results_for_update);
            cx.notify();
        });
        self.scroll_handle.scroll_to_bottom();
    }

    /// 切换结果面板的显示/隐藏状态
    pub fn toggle_visibility(&mut self, cx: &mut App) {
        self.is_visible.update(cx, |visible, cx| {
            *visible = !*visible;
            cx.notify();
        });
    }

    /// 显示结果面板
    pub fn show(&mut self, cx: &mut App) {
        self.is_visible.update(cx, |visible, cx| {
            *visible = true;
            cx.notify();
        });
    }

    /// 隐藏结果面板
    pub fn hide(&mut self, cx: &mut App) {
        self.is_visible.update(cx, |visible, cx| {
            *visible = false;
            cx.notify();
        });
    }

    /// 检查是否有结果数据
    pub fn has_results(&self, cx: &App) -> bool {
        !self.all_results.read(cx).is_empty()
    }

    /// 检查面板是否可见
    pub fn is_visible(&self, cx: &App) -> bool {
        *self.is_visible.read(cx)
    }

    /// 检查是否正在执行查询
    pub fn is_executing(&self, cx: &App) -> bool {
        matches!(
            *self.execution_state.read(cx),
            ExecutionState::Executing { .. }
        )
    }

    fn render_stats_row(
        &self,
        all_results: &[SqlResult],
        success_count: usize,
        error_count: usize,
        total_elapsed_ms: f64,
        show_errors_only: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let clone_self = self.clone();
        h_flex()
            .w_full()
            .p_4()
            .gap_8()
            .justify_between()
            .child(
                h_flex()
                    .gap_8()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlResultTab.processed_queries_label")),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .child(format!("{}", all_results.len())),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlResultTab.success_label")),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().success)
                                    .child(format!("{}", success_count)),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlResultTab.error_label")),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().danger)
                                    .child(format!("{}", error_count)),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlResultTab.execution_time_label")),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .child(format!("{:.3}s", total_elapsed_ms / 1000.0)),
                            ),
                    ),
            )
            .child(
                h_flex().gap_2().items_center().child(
                    Checkbox::new("show-errors-only")
                        .label(t!("SqlResultTab.show_errors_only").to_string())
                        .checked(show_errors_only)
                        .on_click({
                            let container = clone_self.clone();
                            move |checked, _, cx| {
                                container.show_errors_only.update(cx, |s, cx| {
                                    *s = *checked;
                                    cx.notify();
                                });
                                container.statement_list.update(cx, |list, cx| {
                                    list.set_show_errors_only(*checked);
                                    cx.notify();
                                });
                            }
                        }),
                ),
            )
    }

    fn render_progress_bar(
        &self,
        is_executing: bool,
        current: usize,
        total: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        if !is_executing {
            return div();
        }

        // 当查询执行中但尚未解析出总数时，显示不确定状态的加载指示器
        if total == 0 {
            return div().px_4().py_2().child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Spinner::new().with_size(Size::Small))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("SqlResultTab.parsing_and_executing")),
                    ),
            );
        }

        let progress_percent = (current as f32 / total as f32) * 100.0;
        div().px_4().child(
            Progress::new("query-progress")
                .h(px(4.))
                .value(progress_percent),
        )
    }

    fn render_table_header(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .px_4()
            .py_2()
            .gap_4()
            .bg(cx.theme().muted)
            .child(
                div()
                    .w(px(300.))
                    .flex_shrink_0()
                    .text_sm()
                    .font_semibold()
                    .child(t!("SqlResultTab.query_header")),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_semibold()
                    .child(t!("SqlResultTab.message_header")),
            )
            .child(
                div()
                    .w(px(80.))
                    .flex_shrink_0()
                    .text_sm()
                    .font_semibold()
                    .child(t!("SqlResultTab.execution_time_header")),
            )
    }

    fn render_statement_list(
        &self,
        filtered_items: Vec<StatementListItem>,
        is_executing: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let item_count = filtered_items.len();

        // 当执行中且列表为空时，显示加载占位符
        if is_executing && item_count == 0 {
            return div().flex_1().w_full().px_4().py_8().child(
                v_flex()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .child(Spinner::new().with_size(Size::Large))
                    .child(
                        div()
                            .text_base()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("SqlResultTab.query_executing_wait")),
                    ),
            );
        }

        div().flex_1().w_full().px_4().child(
            uniform_list(
                "statement-list",
                item_count,
                cx.processor(
                    move |view: &mut Self, visible_range: Range<usize>, _window, cx| {
                        let items = view.statement_list.read(cx).filtered_items().to_vec();
                        visible_range
                            .into_iter()
                            .filter_map(|idx| {
                                items.get(idx).map(|item| {
                                    let status_color = if item.is_error {
                                        cx.theme().danger
                                    } else {
                                        cx.theme().success
                                    };
                                    let sql_display =
                                        item.truncated_sql.as_ref().cloned().unwrap_or_else(|| {
                                            item.sql.replace('\n', " ").replace('\r', "")
                                        });

                                    h_flex()
                                        .id(("statement-item", idx))
                                        .w_full()
                                        .h(px(40.))
                                        .items_center()
                                        .gap_4()
                                        .child(Self::render_sql_column(
                                            item,
                                            sql_display,
                                            status_color,
                                            idx,
                                            cx,
                                        ))
                                        .child(view.render_message_column(
                                            item,
                                            status_color,
                                            idx,
                                            cx,
                                        ))
                                        .child(
                                            div()
                                                .w(px(80.))
                                                .flex_shrink_0()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!(
                                                    "{:.3}s",
                                                    item.elapsed_ms as f64 / 1000.0
                                                )),
                                        )
                                })
                            })
                            .collect()
                    },
                ),
            )
            .size_full()
            .track_scroll(&self.scroll_handle),
        )
    }

    fn render_sql_column(
        item: &StatementListItem,
        sql_display: String,
        status_color: impl Into<gpui::Hsla>,
        idx: usize,
        _cx: &Context<Self>,
    ) -> impl IntoElement {
        if item.truncated_sql.is_some() && item.sql.len() > 50 {
            let full_sql = item.sql.clone();
            Popover::new(("sql-popover", idx))
                .trigger(
                    Button::new(("sql-btn", idx))
                        .ghost()
                        .with_size(Size::XSmall)
                        .w(px(300.))
                        .flex_shrink_0()
                        .justify_start()
                        .text_sm()
                        .text_color(status_color)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(sql_display),
                )
                .content(move |_state, _window, _cx| {
                    Self::render_sql_popover_content(full_sql.clone(), idx)
                })
                .max_w(px(600.))
                .into_any_element()
        } else {
            div()
                .w(px(300.))
                .flex_shrink_0()
                .text_sm()
                .text_color(status_color)
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(sql_display)
                .into_any_element()
        }
    }

    fn render_sql_popover_content(full_sql: String, idx: usize) -> impl IntoElement {
        let sql_for_copy = full_sql.clone();
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(Icon::new(IconName::File).with_size(Size::Small))
                            .child(t!("SqlResultTab.sql_statement")),
                    )
                    .child(Clipboard::new(("copy-sql", idx)).value(sql_for_copy)),
            )
            .child(
                div()
                    .max_w(px(500.))
                    .max_h(px(200.))
                    .overflow_y_scrollbar()
                    .text_sm()
                    .child(full_sql),
            )
    }

    fn render_message_column(
        &self,
        item: &StatementListItem,
        status_color: impl Into<gpui::Hsla>,
        idx: usize,
        _cx: &Context<Self>,
    ) -> impl IntoElement {
        if item.is_error && item.full_error_message.is_some() {
            let error_msg = item.full_error_message.as_ref().unwrap().clone();
            let sql = item.sql.clone();

            Popover::new(("error-popover", idx))
                .trigger(
                    Button::new(("error-btn", idx))
                        .ghost()
                        .with_size(Size::XSmall)
                        .flex_1()
                        .min_w(px(0.))
                        .justify_start()
                        .text_sm()
                        .text_color(status_color)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(item.status_text.clone()),
                )
                .content(move |_state, _window, _cx| {
                    Self::render_error_popover_content(error_msg.clone(), sql.clone(), idx)
                })
                .max_w(px(400.))
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_w(px(0.))
                .text_sm()
                .text_color(status_color)
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(item.status_text.clone())
                .into_any_element()
        }
    }

    fn render_error_popover_content(
        error_msg: String,
        sql: String,
        idx: usize,
    ) -> impl IntoElement {
        let error_for_copy = error_msg.clone();
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(Icon::new(IconName::TriangleAlert).with_size(Size::Small))
                            .child(t!("SqlResultTab.error_info")),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(Clipboard::new(("copy-error", idx)).value(error_for_copy))
                            .child(
                                AskAiButton::new(
                                    format!("ask-ai-error-{}", idx),
                                    sql,
                                    error_msg.clone(),
                                )
                                .with_size(Size::XSmall),
                            ),
                    ),
            )
            .child(
                div()
                    .max_w(px(400.))
                    .max_h(px(200.))
                    .overflow_y_scrollbar()
                    .text_sm()
                    .child(error_msg),
            )
    }

    fn render_summary_panel(
        &self,
        all_results: &[SqlResult],
        execution_state: ExecutionState,
        show_errors_only: bool,
        total_elapsed_ms: f64,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let filtered_items: Vec<StatementListItem> =
            self.statement_list.read(cx).filtered_items().to_vec();

        let mut success_count = 0;
        let mut error_count = 0;
        for result in all_results.iter() {
            match result {
                SqlResult::Query(_) | SqlResult::Exec(_) => success_count += 1,
                SqlResult::Error(_) => error_count += 1,
            }
        }

        let is_executing = matches!(execution_state, ExecutionState::Executing { .. });
        let (current, total) = match &execution_state {
            ExecutionState::Executing { current, total } => (*current, *total),
            _ => (0, 0),
        };

        div()
            .flex_1()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .overflow_hidden()
            .child(
                v_flex()
                    .size_full()
                    .child(self.render_stats_row(
                        all_results,
                        success_count,
                        error_count,
                        total_elapsed_ms,
                        show_errors_only,
                        cx,
                    ))
                    .child(self.render_progress_bar(is_executing, current, total, cx))
                    .child(div().mx_4().h(px(1.)).w_full().bg(cx.theme().border))
                    .child(self.render_table_header(cx))
                    .child(self.render_statement_list(filtered_items, is_executing, cx)),
            )
    }

    fn render_result_tab(&self, active_idx: usize, cx: &Context<Self>) -> impl IntoElement {
        let query_tabs = self.result_tabs.read(cx);
        query_tabs
            .get(active_idx - 1)
            .and_then(|tab| tab.data_grid.as_ref())
            .map(|data_grid| data_grid.clone().into_any_element())
            .unwrap_or_else(|| {
                div()
                    .flex_1()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_md()
                    .into_any_element()
            })
    }
}

impl Render for SqlResultTabContainer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clone_self = self.clone();
        let query_tabs = self.result_tabs.read(cx);
        let all_results = self.all_results.read(cx);
        let active_idx = *self.active_result_tab.read(cx);
        let execution_state = self.execution_state.read(cx).clone();
        let show_errors_only = *self.show_errors_only.read(cx);
        let total_elapsed_ms = *self.total_elapsed_ms.read(cx);

        v_flex()
            .size_full()
            .gap_0()
            .child(
                h_flex().w_full().items_center().justify_center().child(
                    TabBar::new("result-tabs")
                        .p_2()
                        .w_full()
                        .underline()
                        .justify_center()
                        .with_size(Size::Small)
                        .selected_index(active_idx)
                        .on_click({
                            let clone_self = clone_self.clone();
                            move |ix: &usize, _w, cx| {
                                clone_self.active_result_tab.update(cx, |active, cx| {
                                    *active = *ix;
                                    cx.notify();
                                });
                            }
                        })
                        .child(
                            Tab::new().label(match &execution_state {
                                ExecutionState::Executing { current, total } => t!(
                                    "SqlResultTab.summary_with_counts",
                                    current = current,
                                    total = total
                                )
                                .to_string(),
                                _ => t!("SqlResultTab.summary").to_string(),
                            }),
                        )
                        .children({
                            let mut tabs = vec![];
                            for idx in 0..query_tabs.len() {
                                tabs.push(Tab::new().label(
                                    t!("SqlResultTab.result_tab", index = idx + 1).to_string(),
                                ))
                            }
                            tabs
                        })
                        .suffix(
                            Button::new("close-results")
                                .with_size(Size::Small)
                                .ghost()
                                .icon(IconName::Close)
                                .tooltip(t!("SqlResultTab.hide_results_panel").to_string())
                                .on_click({
                                    let close_self = clone_self.clone();
                                    move |_, _, cx| {
                                        close_self.clone().hide(cx);
                                    }
                                }),
                        ),
                ),
            )
            .child(if active_idx == 0 {
                self.render_summary_panel(
                    all_results,
                    execution_state,
                    show_errors_only,
                    total_elapsed_ms,
                    cx,
                )
                .into_any_element()
            } else {
                self.render_result_tab(active_idx, cx).into_any_element()
            })
    }
}
