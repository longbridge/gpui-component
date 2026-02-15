//! MessageList - 消息列表组件
//!
//! 渲染聊天消息列表，支持：
//! - 用户/助手消息
//! - 状态消息
//! - SQL 结果消息
//! - 消息分页加载

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, Entity, InteractiveElement, IntoElement, ParentElement, ScrollHandle,
    StatefulInteractiveElement, Styled,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    highlighter::HighlightTheme,
    scroll::Scrollbar,
    v_flex, ActiveTheme,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::chatdb::chat_markdown::{parse_sql_code_blocks, SqlCodeBlock};
use crate::chatdb::chat_sql_block::SqlBlockResultState;
use crate::chatdb::chat_sql_result::ChatSqlResultView;

// 复用核心库的类型和常量
pub use one_core::ai_chat::{
    ChatRole, MessageVariant, MESSAGE_RENDER_LIMIT, MESSAGE_RENDER_STEP,
};

// 使用核心库的泛型消息类型
use one_core::{ChatMessageUIGeneric, MessageExtension};

// ============================================================================
// SQL 扩展
// ============================================================================

/// SQL 消息扩展
///
/// 为消息添加 SQL 代码块解析缓存支持。
#[derive(Clone, Debug, Default)]
pub struct SqlExtension {
    /// SQL 代码块解析缓存（内容哈希, 解析结果）
    pub cached_sql_blocks: Option<Arc<(u64, Vec<SqlCodeBlock>)>>,
}

impl MessageExtension for SqlExtension {
    fn on_finalize_streaming(&mut self) {
        self.cached_sql_blocks = None;
    }

    fn clear_cache(&mut self) {
        self.cached_sql_blocks = None;
    }
}

/// SQL 扩展版本的 ChatMessageUI
pub type ChatMessageUI = ChatMessageUIGeneric<SqlExtension>;

// ============================================================================
// SQL 代码块缓存扩展方法
// ============================================================================

/// SQL 代码块缓存扩展 trait
///
/// 为 `ChatMessageUI`（即 `ChatMessageUIGeneric<SqlExtension>`）添加 SQL 特有的方法。
pub trait SqlBlockCacheExt {
    /// 获取 SQL 代码块（带缓存）
    fn get_sql_blocks(&mut self, highlight_theme: &HighlightTheme) -> Vec<SqlCodeBlock>;
}

impl SqlBlockCacheExt for ChatMessageUI {
    fn get_sql_blocks(&mut self, highlight_theme: &HighlightTheme) -> Vec<SqlCodeBlock> {
        let hash = self.content_hash();

        // 检查缓存是否有效
        if let Some(cached) = &self.extension.cached_sql_blocks {
            if cached.0 == hash {
                return cached.1.clone();
            }
        }

        // 重新解析并缓存
        let blocks = parse_sql_code_blocks(&self.content, highlight_theme);
        self.extension.cached_sql_blocks = Some(Arc::new((hash, blocks.clone())));
        blocks
    }
}

// ============================================================================
// MessageListRenderer
// ============================================================================

/// 消息列表渲染上下文
pub struct MessageListContext<'a> {
    /// 消息列表
    pub messages: &'a [ChatMessageUI],
    /// SQL 结果视图映射
    pub sql_result_views: &'a HashMap<String, Entity<ChatSqlResultView>>,
    /// SQL 代码块结果状态映射
    pub sql_block_results: &'a HashMap<String, HashMap<usize, SqlBlockResultState>>,
    /// 滚动句柄
    pub scroll_handle: &'a ScrollHandle,
    /// 渲染限制
    pub render_limit: usize,
    /// 最新 AI 消息 ID
    pub latest_ai_message_id: Option<&'a str>,
}

/// 消息列表渲染器
pub struct MessageListRenderer;

impl MessageListRenderer {
    /// 渲染消息列表
    pub fn render<F, G>(
        ctx: &MessageListContext,
        on_load_more: F,
        on_collapse: G,
        cx: &App,
    ) -> impl IntoElement
    where
        F: Fn() + 'static,
        G: Fn() + 'static,
    {
        let total = ctx.messages.len();
        let hidden_count = total.saturating_sub(ctx.render_limit);
        let can_collapse = total > MESSAGE_RENDER_LIMIT && ctx.render_limit > MESSAGE_RENDER_LIMIT;

        div()
            .id("chat-messages-list")
            .flex_1()
            .min_h_0()
            .w_full()
            .relative()
            .child(
                div()
                    .id("chat-messages-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(ctx.scroll_handle)
                    .p_4()
                    .pb_8()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            // 加载更多/收起按钮
                            .when(hidden_count > 0 || can_collapse, |this| {
                                this.child(Self::render_pagination_controls(
                                    hidden_count,
                                    can_collapse,
                                    on_load_more,
                                    on_collapse,
                                ))
                            })
                            // 消息列表
                            .children(
                                ctx.messages
                                    .iter()
                                    .skip(hidden_count)
                                    .map(|msg| Self::render_message(msg, ctx, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(px(16.0))
                    .child(Scrollbar::vertical(ctx.scroll_handle)),
            )
    }

    /// 渲染分页控件
    fn render_pagination_controls<F, G>(
        hidden_count: usize,
        can_collapse: bool,
        on_load_more: F,
        on_collapse: G,
    ) -> impl IntoElement
    where
        F: Fn() + 'static,
        G: Fn() + 'static,
    {
        h_flex()
            .w_full()
            .justify_center()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .when(hidden_count > 0, move |this| {
                        this.child(
                            Button::new("chat-load-more")
                                .ghost()
                                .label(format!("加载更早消息（剩余 {} 条）", hidden_count))
                                .on_click(move |_, _, _| on_load_more()),
                        )
                    })
                    .when(can_collapse, move |this| {
                        this.child(
                            Button::new("chat-collapse-history")
                                .ghost()
                                .label("收起历史消息")
                                .on_click(move |_, _, _| on_collapse()),
                        )
                    }),
            )
    }

    /// 渲染单条消息
    /// 委托用户/系统消息到 ChatMessageRenderer，保留 SQL 特有的渲染逻辑
    fn render_message(
        msg: &ChatMessageUI,
        ctx: &MessageListContext,
        cx: &App,
    ) -> AnyElement {
        use one_core::ChatMessageRenderer;

        match msg.role {
            ChatRole::User => ChatMessageRenderer::render_user_message(msg, cx),
            ChatRole::Assistant => match &msg.variant {
                MessageVariant::Status { title, is_done } => {
                    ChatMessageRenderer::render_status_message(&msg.id, title, *is_done, cx)
                }
                MessageVariant::Text => Self::render_assistant_text_message(msg, cx),
                MessageVariant::SqlResult => Self::render_sql_result_message(&msg.id, ctx, cx),
            },
            ChatRole::System => ChatMessageRenderer::render_system_message(msg, cx),
        }
    }

    /// 渲染助手文本消息
    fn render_assistant_text_message(msg: &ChatMessageUI, cx: &App) -> AnyElement {
        use gpui::SharedString;
        use gpui_component::text::TextView;

        if msg.is_streaming && msg.content.is_empty() {
            return one_core::ChatMessageRenderer::render_thinking(cx);
        }

        div()
            .w_full()
            .child(
                div().w_full().p_3().child(
                    TextView::markdown(
                        SharedString::from(format!("ai-sql-msg-{}", msg.id)),
                        msg.content.clone(),
                    )
                    .selectable(true),
                ),
            )
            .into_any_element()
    }

    /// 渲染 SQL 结果消息
    fn render_sql_result_message(msg_id: &str, ctx: &MessageListContext, _cx: &App) -> AnyElement {
        if let Some(result_view) = ctx.sql_result_views.get(msg_id) {
            div()
                .w_full()
                .child(result_view.clone())
                .into_any_element()
        } else {
            div()
                .w_full()
                .text_sm()
                .child("加载中...")
                .into_any_element()
        }
    }

    /// 渲染 SQL 代码块容器
    pub fn render_sql_block_container(
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
