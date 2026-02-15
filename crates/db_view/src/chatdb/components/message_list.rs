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
    SharedString, StatefulInteractiveElement, Styled,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    highlighter::HighlightTheme,
    scroll::Scrollbar,
    text::TextView,
    v_flex, ActiveTheme, Icon, IconName, Sizable, Size,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::chatdb::chat_markdown::{parse_sql_code_blocks, SqlCodeBlock};
use crate::chatdb::chat_sql_block::SqlBlockResultState;
use crate::chatdb::chat_sql_result::ChatSqlResultView;

// 复用核心库的类型和常量
pub use one_core::ai_chat::{
    ChatRole, MessageVariant, MESSAGE_RENDER_LIMIT, MESSAGE_RENDER_STEP,
};

// ============================================================================
// 消息类型（SQL 扩展版本）
// ============================================================================

/// UI 消息结构（SQL 扩展版本）
///
/// 继承核心库的基础功能，添加 SQL 代码块缓存支持。
#[derive(Clone, Debug)]
pub struct ChatMessageUI {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub variant: MessageVariant,
    pub is_streaming: bool,
    /// SQL 代码块解析缓存（内容哈希, 解析结果）
    cached_sql_blocks: Option<Arc<(u64, Vec<SqlCodeBlock>)>>,
}

impl ChatMessageUI {
    /// 创建用户消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: ChatRole::User,
            content: content.into(),
            variant: MessageVariant::Text,
            is_streaming: false,
            cached_sql_blocks: None,
        }
    }

    /// 创建助手消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: content.into(),
            variant: MessageVariant::Text,
            is_streaming: false,
            cached_sql_blocks: None,
        }
    }

    /// 创建状态消息
    pub fn status(title: impl Into<String>, is_done: bool) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: String::new(),
            variant: MessageVariant::Status {
                title: title.into(),
                is_done,
            },
            is_streaming: !is_done,
            cached_sql_blocks: None,
        }
    }

    /// 创建流式助手消息
    pub fn streaming_assistant() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: String::new(),
            variant: MessageVariant::Text,
            is_streaming: true,
            cached_sql_blocks: None,
        }
    }

    /// 从历史消息创建（用于加载会话历史）
    pub fn from_history(id: impl Into<String>, role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role,
            content: content.into(),
            variant: MessageVariant::Text,
            is_streaming: false,
            cached_sql_blocks: None,
        }
    }

    /// 设置 ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// 设置变体
    pub fn with_variant(mut self, variant: MessageVariant) -> Self {
        self.variant = variant;
        self
    }

    /// 设置流式状态
    pub fn with_streaming(mut self, is_streaming: bool) -> Self {
        self.is_streaming = is_streaming;
        self
    }

    /// 设置内容
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self.cached_sql_blocks = None;
        self
    }

    /// 计算内容哈希
    fn content_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        hasher.finish()
    }

    /// 获取 SQL 代码块（带缓存）
    pub fn get_sql_blocks(&mut self, highlight_theme: &HighlightTheme) -> Vec<SqlCodeBlock> {
        let content_hash = self.content_hash();

        // 检查缓存是否有效
        if let Some(cached) = &self.cached_sql_blocks {
            if cached.0 == content_hash {
                return cached.1.clone();
            }
        }

        // 重新解析并缓存
        let blocks = parse_sql_code_blocks(&self.content, highlight_theme);
        self.cached_sql_blocks = Some(Arc::new((content_hash, blocks.clone())));
        blocks
    }

    /// 流式消息结束时刷新缓存
    pub fn finalize_streaming(&mut self) {
        self.is_streaming = false;
        self.cached_sql_blocks = None;
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.cached_sql_blocks = None;
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
    fn render_message(
        msg: &ChatMessageUI,
        ctx: &MessageListContext,
        cx: &App,
    ) -> AnyElement {
        match msg.role {
            ChatRole::User => Self::render_user_message(msg, cx),
            ChatRole::Assistant => match &msg.variant {
                MessageVariant::Status { title, is_done } => {
                    Self::render_status_message(&msg.id, title, *is_done, cx)
                }
                MessageVariant::Text => Self::render_assistant_text_message(msg, cx),
                MessageVariant::SqlResult => Self::render_sql_result_message(&msg.id, ctx, cx),
            },
            ChatRole::System => {
                // 系统消息渲染为居中的灰色文本
                h_flex()
                    .w_full()
                    .justify_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(msg.content.clone()),
                    )
                    .into_any_element()
            }
        }
    }

    /// 渲染用户消息
    fn render_user_message(msg: &ChatMessageUI, cx: &App) -> AnyElement {
        div()
            .w_full()
            .px_3()
            .py_2()
            .bg(cx.theme().accent)
            .text_color(cx.theme().accent_foreground)
            .rounded_lg()
            .child(
                TextView::markdown(
                    SharedString::from(format!("user-msg-{}", msg.id)),
                    msg.content.clone(),
                )
                .selectable(true),
            )
            .into_any_element()
    }

    /// 渲染状态消息
    fn render_status_message(id: &str, title: &str, is_done: bool, cx: &App) -> AnyElement {
        let icon = if is_done {
            IconName::Check
        } else {
            IconName::Loader
        };

        h_flex()
            .id(SharedString::from(id.to_string()))
            .w_full()
            .items_center()
            .gap_2()
            .py_1()
            .child(
                Icon::new(icon).with_size(Size::Small).text_color(if is_done {
                    cx.theme().success
                } else {
                    cx.theme().muted_foreground
                }),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(title.to_string()),
            )
            .into_any_element()
    }

    /// 渲染助手文本消息
    fn render_assistant_text_message(msg: &ChatMessageUI, cx: &App) -> AnyElement {
        if msg.is_streaming && msg.content.is_empty() {
            return div()
                .w_full()
                .py_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("思考中..."),
                )
                .into_any_element();
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
