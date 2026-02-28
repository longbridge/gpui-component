//! SqlBlockRenderer - SQL 代码块渲染器
//!
//! 渲染 SQL 代码块及其执行结果

use gpui::prelude::FluentBuilder;
use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, px};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    h_flex, v_flex,
};
use rust_i18n::t;
use std::collections::HashMap;

use crate::chatdb::chat_markdown::SqlCodeBlock;
use crate::chatdb::chat_sql_block::SqlBlockResultState;

// ============================================================================
// SqlBlockRenderer
// ============================================================================

/// SQL 代码块渲染器
pub struct SqlBlockRenderer;

impl SqlBlockRenderer {
    /// 渲染 SQL 代码块操作按钮
    pub fn render_actions<F>(code: &str, is_sql: bool, on_run: F) -> impl IntoElement
    where
        F: Fn() + 'static,
    {
        h_flex()
            .gap_1()
            .child(Clipboard::new("copy").value(code.to_string()))
            .when(is_sql, move |this| {
                this.child(
                    Button::new("run-sql")
                        .icon(IconName::SquareTerminal)
                        .ghost()
                        .xsmall()
                        .label(t!("ChatSqlBlock.run").to_string())
                        .on_click(move |_, _, _| on_run()),
                )
            })
    }

    /// 渲染 SQL 代码块容器（包含结果）
    pub fn render_container(
        message_id: &str,
        block: &SqlCodeBlock,
        default_element: AnyElement,
        sql_block_results: &HashMap<String, HashMap<usize, SqlBlockResultState>>,
        cx: &App,
    ) -> AnyElement {
        let result_state = sql_block_results
            .get(message_id)
            .and_then(|map| map.get(&block.key));

        let (error, container, has_visible_result) = result_state
            .map(|state| {
                (
                    state.error.clone(),
                    Some(state.container.clone()),
                    state.has_visible_result(cx),
                )
            })
            .unwrap_or((None, None, false));

        let error_element = error.map(|error| {
            div()
                .text_sm()
                .text_color(cx.theme().danger)
                .child(error)
                .into_any_element()
        });

        let result_element = if has_visible_result {
            container.map(|container| {
                div()
                    .w_full()
                    .h(px(280.0))
                    .child(container)
                    .into_any_element()
            })
        } else {
            None
        };

        v_flex()
            .w_full()
            .gap_2()
            .child(default_element)
            .when_some(error_element, |this, error| this.child(error))
            .when_some(result_element, |this, result| this.child(result))
            .into_any_element()
    }
}
