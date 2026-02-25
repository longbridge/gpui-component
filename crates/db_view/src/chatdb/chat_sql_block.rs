//! SQL 代码块结果状态 - 复用 SQLResultTabContainer 并保存执行信息

use gpui::{App, Entity};
use rust_i18n::t;

use crate::sql_result_tab::SqlResultTabContainer;

#[derive(Clone)]
pub struct SqlBlockResultState {
    pub sql: String,
    pub container: Entity<SqlResultTabContainer>,
    pub last_run_sql: Option<String>,
    pub error: Option<String>,
    /// 是否折叠结果区域
    pub collapsed: bool,
}

impl SqlBlockResultState {
    pub fn new(sql: String, container: Entity<SqlResultTabContainer>) -> Self {
        Self {
            sql,
            container,
            last_run_sql: None,
            error: None,
            collapsed: false, // 默认展开
        }
    }

    pub fn should_run(&self, sql: &str) -> bool {
        self.last_run_sql.as_deref() != Some(sql)
    }

    pub fn mark_run(&mut self, sql: String) {
        self.last_run_sql = Some(sql);
    }

    pub fn set_error(&mut self, message: String) {
        self.error = Some(message);
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// 切换折叠状态
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// 获取结果摘要信息（用于折叠状态显示）
    pub fn get_summary(&self, cx: &App) -> Option<String> {
        let container = self.container.read(cx);
        if !container.has_results(cx) {
            return None;
        }
        // 从容器获取结果统计
        let results = container.all_results.read(cx);
        if results.is_empty() {
            return None;
        }

        let mut total_rows = 0usize;
        let mut success_count = 0usize;
        let mut error_count = 0usize;

        for result in results.iter() {
            match result {
                db::SqlResult::Query(q) => {
                    success_count += 1;
                    total_rows += q.rows.len();
                }
                db::SqlResult::Exec(_) => {
                    success_count += 1;
                }
                db::SqlResult::Error(_) => {
                    error_count += 1;
                }
            }
        }

        if error_count > 0 {
            Some(
                t!(
                    "ChatSqlBlock.summary_success_error",
                    success = success_count,
                    error = error_count
                )
                .to_string(),
            )
        } else {
            Some(t!("ChatSqlBlock.summary_rows", rows = total_rows).to_string())
        }
    }

    pub fn has_visible_result(&self, cx: &App) -> bool {
        let container = self.container.read(cx);
        if !container.is_visible(cx) {
            return false;
        }
        container.has_results(cx) || container.is_executing(cx)
    }
}
