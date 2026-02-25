//! Chat SQL Result - 可复用的 SQL 结果展示组件

use gpui::{div, px, AnyElement, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::Button,
    h_flex,
    tab::{Tab, TabBar},
    v_flex,
    ActiveTheme, Icon, IconName, Sizable, Size,
};
use one_ui::edit_table::Column;

use crate::table_data::data_grid::{DataGrid, DataGridConfig, DataGridUsage};
use db::SqlResult;
use gpui_component::button::ButtonVariants;
use one_core::storage::DatabaseType;
use rust_i18n::t;

/// SQL 结果项
pub struct SqlResultItem {
    pub sql: String,
    pub result: SqlResult,
    pub data_grid: Option<Entity<DataGrid>>,
}

/// 可复用的 SQL 结果视图组件
pub struct ChatSqlResultView {
    results: Vec<SqlResultItem>,
    active_tab: usize,
    success_count: usize,
    error_count: usize,
    total_elapsed_ms: u128,
    /// 是否折叠（默认折叠以提升性能）
    collapsed: bool,
    connection_id: String,
    database: Option<String>,
    db_type: DatabaseType,
}

impl ChatSqlResultView {
    pub fn new(
        sql_results: Vec<SqlResult>,
        connection_id: &str,
        database: Option<String>,
        db_type: DatabaseType,
        initially_collapsed: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;
        let mut total_elapsed_ms = 0u128;

        for result in sql_results {
            match &result {
                SqlResult::Query(q) => {
                    success_count += 1;
                    total_elapsed_ms += q.elapsed_ms;

                    results.push(SqlResultItem {
                        sql: q.sql.clone(),
                        result,
                        data_grid: None,
                    });
                }
                SqlResult::Exec(e) => {
                    success_count += 1;
                    total_elapsed_ms += e.elapsed_ms;
                    results.push(SqlResultItem {
                        sql: e.sql.clone(),
                        result,
                        data_grid: None,
                    });
                }
                SqlResult::Error(e) => {
                    error_count += 1;
                    results.push(SqlResultItem {
                        sql: e.sql.clone(),
                        result,
                        data_grid: None,
                    });
                }
            }
        }

        Self {
            results,
            active_tab: 0,
            success_count,
            error_count,
            total_elapsed_ms,
            collapsed: initially_collapsed,
            connection_id: connection_id.to_string(),
            database,
            db_type,
        }
    }

    /// 设置折叠状态
    pub fn set_collapsed(&mut self, collapsed: bool) {
        self.collapsed = collapsed;
        if collapsed {
            self.release_query_grids();
        }
    }

    /// 获取折叠状态
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    pub fn success_count(&self) -> usize {
        self.success_count
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }

    pub fn total_elapsed_ms(&self) -> u128 {
        self.total_elapsed_ms
    }

    pub fn has_query_results(&self) -> bool {
        self.results.iter().any(|r| matches!(r.result, SqlResult::Query(_)))
    }

    fn release_query_grids(&mut self) {
        for item in &mut self.results {
            if matches!(item.result, SqlResult::Query(_)) {
                item.data_grid = None;
            }
        }
    }

    fn ensure_query_grid(&mut self, result_idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.results.get_mut(result_idx) else {
            return;
        };
        let SqlResult::Query(q) = &item.result else {
            return;
        };
        if item.data_grid.is_some() {
            return;
        }

        let config = DataGridConfig::new(
            self.database.clone().unwrap_or_default(),
            "",
            &self.connection_id,
            self.db_type,
        )
        .editable(false)
        .show_toolbar(false)
        .usage(DataGridUsage::SqlResult)
        .rows_count(q.rows.len())
        .execution_time(q.elapsed_ms)
        .sql(q.sql.clone());

        let data_grid = cx.new(|cx| DataGrid::new(config, window, cx));

        let columns: Vec<Column> = q.columns.iter()
            .map(|h| Column::new(h.clone(), h.clone()))
            .collect();
        let rows: Vec<Vec<Option<String>>> = q.rows.iter()
            .map(|row| row.iter().cloned().collect())
            .collect();

        data_grid.update(cx, |grid: &mut DataGrid, cx| {
            grid.update_data(columns, rows, vec![], cx);
        });

        item.data_grid = Some(data_grid);
    }

    fn render_query_result(&mut self, result_idx: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.ensure_query_grid(result_idx, window, cx);
        let Some(item) = self.results.get(result_idx) else {
            return div().into_any_element();
        };
        if let Some(grid) = &item.data_grid {
            return div()
                .w_full()
                .h(px(250.0))
                .border_1()
                .border_color(cx.theme().border)
                .rounded_md()
                .overflow_hidden()
                .child(grid.clone())
                .into_any_element();
        }
        div().into_any_element()
    }
    fn render_summary(&self, cx: &mut Context<Self>) -> AnyElement {
        let collapsed = self.collapsed;
        let has_data = self.has_query_results();
        let rows_info = self.results.iter()
            .filter_map(|r| match &r.result {
                SqlResult::Query(q) => Some(q.rows.len()),
                _ => None,
            })
            .sum::<usize>();

        h_flex()
            .id("sql-result-summary")
            .w_full()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_1()
            .bg(cx.theme().muted)
            .rounded_md()
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Button::new("sql-result-toggle")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(if collapsed { IconName::ChevronRight } else { IconName::ChevronDown })
                                .with_size(Size::Small)
                                .text_color(cx.theme().muted_foreground))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.collapsed = !this.collapsed;
                                if this.collapsed {
                                    this.release_query_grids();
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(Icon::new(IconName::Check).with_size(Size::Small).text_color(cx.theme().success))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().success)
                                            .child(
                                                t!(
                                                    "ChatSqlResult.success_count",
                                                    count = self.success_count
                                                )
                                                .to_string()
                                            )
                            )
                    )
                    .when(self.error_count > 0, |this| {
                        this.child(
                            h_flex()
                                .items_center()
                                .gap_1()
                                .child(Icon::new(IconName::Close).with_size(Size::Small).text_color(cx.theme().danger))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().danger)
                                        .child(
                                            t!(
                                                "ChatSqlResult.error_count",
                                                count = self.error_count
                                            )
                                            .to_string()
                                        )
                                )
                        )
                    })
                    .when(has_data && rows_info > 0, |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("ChatSqlResult.rows_info", rows = rows_info).to_string())
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{:.3}s", self.total_elapsed_ms as f64 / 1000.0))
                    )
            )
            .into_any_element()
    }

    fn render_exec_result(&self, idx: usize, cx: &mut Context<Self>) -> AnyElement {
        let item = &self.results[idx];
        match &item.result {
            SqlResult::Exec(e) => {
                v_flex()
                    .w_full()
                    .p_4()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Check).with_size(Size::Small).text_color(cx.theme().success))
                            .child(
                                div()
                                    .text_sm()
                                    .child(
                                        t!(
                                            "ChatSqlResult.exec_success_rows",
                                            rows = e.rows_affected
                                        )
                                        .to_string()
                                    )
                            )
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                t!("ChatSqlResult.elapsed_ms", ms = e.elapsed_ms).to_string()
                            )
                    )
                    .into_any_element()
            }
            SqlResult::Error(e) => {
                v_flex()
                    .w_full()
                    .p_4()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Close).with_size(Size::Small).text_color(cx.theme().danger))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().danger)
                                    .child(t!("ChatSqlResult.exec_failed"))
                            )
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().danger)
                            .child(e.message.clone())
                    )
                    .into_any_element()
            }
            _ => div().into_any_element(),
        }
    }
}

impl Render for ChatSqlResultView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.collapsed;
        let query_indices: Vec<usize> = self.results.iter()
            .enumerate()
            .filter(|(_, r)| matches!(r.result, SqlResult::Query(_)))
            .map(|(idx, _)| idx)
            .collect();

        v_flex()
            .w_full()
            .gap_2()
            .child(self.render_summary(cx))
            .when(!collapsed, |this| {
                let has_tabs = query_indices.len() > 1;
                let active_tab = self.active_tab.min(query_indices.len().saturating_sub(1));

                this
                    .when(has_tabs, |this| {
                        this.child(
                            TabBar::new("sql-result-tabs")
                                .w_full()
                                .underline()
                                .with_size(Size::Small)
                                .selected_index(active_tab)
                                .children(
                                    query_indices.iter().enumerate().map(|(tab_idx, _)| {
                                        Tab::new().label(
                                            t!(
                                                "ChatSqlResult.result_tab",
                                                index = tab_idx + 1
                                            )
                                            .to_string()
                                        )
                                    })
                                )
                        )
                    })
                    .when(!query_indices.is_empty(), |this| {
                        let result_idx = *query_indices.get(active_tab).unwrap_or(&query_indices[0]);
                        this.child(self.render_query_result(result_idx, window, cx))
                    })
                    .when(query_indices.is_empty() && !self.results.is_empty(), |this| {
                        this.child(self.render_exec_result(0, cx))
                    })
            })
    }
}
