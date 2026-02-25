//! SessionSidebar - 会话侧边栏组件
//!
//! 显示历史会话列表，支持：
//! - 会话搜索
//! - 会话切换
//! - 新建会话

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    list::{List, ListDelegate, ListState},
    v_flex, ActiveTheme, IconName, Selectable, Sizable,
};
use gpui_component::IndexPath;
use one_core::llm::chat_history::ChatSession;
use rust_i18n::t;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// 会话列表项
// ============================================================================

/// 会话列表项组件
#[derive(IntoElement)]
pub struct SessionListItem {
    session_id: i64,
    name: SharedString,
    updated_at: i64,
    selected: bool,
    is_current: bool,
    on_click: Box<dyn Fn(i64) + 'static>,
}

impl SessionListItem {
    pub fn new<F>(
        session_id: i64,
        name: SharedString,
        updated_at: i64,
        is_current: bool,
        on_click: F,
    ) -> Self
    where
        F: Fn(i64) + 'static,
    {
        Self {
            session_id,
            name,
            updated_at,
            selected: false,
            is_current,
            on_click: Box::new(on_click),
        }
    }
}

impl Selectable for SessionListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for SessionListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let session_id = self.session_id;
        let on_click = self.on_click;

        h_flex()
            .id(SharedString::from(format!("sql-session-{}", session_id)))
            .w_full()
            .gap_2()
            .items_center()
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .when(self.selected, |this| this.bg(cx.theme().list_active))
            .when(self.is_current, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .on_click(move |_, _window, cx| {
                cx.stop_propagation();
                on_click(session_id);
            })
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if self.is_current {
                                cx.theme().accent_foreground
                            } else {
                                cx.theme().muted_foreground
                            })
                            .child(format_timestamp(self.updated_at)),
                    ),
            )
            .into_any_element()
    }
}

// ============================================================================
// 会话列表代理
// ============================================================================

/// 会话列表代理
pub struct SessionListDelegate<F>
where
    F: Fn(i64) + Clone + 'static,
{
    sessions: Vec<(i64, SharedString, i64)>,
    filtered_sessions: Vec<(i64, SharedString, i64)>,
    selected_index: Option<IndexPath>,
    current_session_id: Option<i64>,
    on_select: F,
}

impl<F> SessionListDelegate<F>
where
    F: Fn(i64) + Clone + 'static,
{
    pub fn new(current_session_id: Option<i64>, on_select: F) -> Self {
        Self {
            sessions: Vec::new(),
            filtered_sessions: Vec::new(),
            selected_index: None,
            current_session_id,
            on_select,
        }
    }

    pub fn update_sessions(&mut self, sessions: Vec<ChatSession>) {
        self.sessions = sessions
            .into_iter()
            .map(|s| (s.id, SharedString::from(s.name), s.updated_at))
            .collect();
        self.filtered_sessions = self.sessions.clone();
    }

    pub fn set_current_session(&mut self, session_id: Option<i64>) {
        self.current_session_id = session_id;
    }
}

impl<F> ListDelegate for SessionListDelegate<F>
where
    F: Fn(i64) + Clone + 'static,
{
    type Item = SessionListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> gpui::Task<()> {
        if query.is_empty() {
            self.filtered_sessions = self.sessions.clone();
        } else {
            let query_lower = query.to_lowercase();
            self.filtered_sessions = self
                .sessions
                .iter()
                .filter(|(_, name, _)| name.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();
        }
        cx.notify();
        gpui::Task::ready(())
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_sessions.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let (session_id, name, updated_at) = self.filtered_sessions.get(ix.row)?.clone();
        let is_current = self.current_session_id == Some(session_id);
        let on_select = self.on_select.clone();
        Some(SessionListItem::new(
            session_id,
            name,
            updated_at,
            is_current,
            move |id| on_select(id),
        ))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, _cx: &mut Context<ListState<Self>>) {
        if let Some(ix) = self.selected_index {
            if let Some((session_id, _, _)) = self.filtered_sessions.get(ix.row) {
                (self.on_select)(*session_id);
            }
        }
    }
}

// ============================================================================
// SessionSidebar
// ============================================================================

/// 会话侧边栏渲染器
pub struct SessionSidebar;

impl SessionSidebar {
    /// 渲染会话侧边栏
    pub fn render<F, D>(
        session_list: Option<&Entity<ListState<D>>>,
        on_new_session: F,
        cx: &App,
    ) -> impl IntoElement
    where
        F: Fn() + 'static,
        D: ListDelegate + 'static,
    {
        let border = cx.theme().border;
        let muted = cx.theme().muted;

        v_flex()
            .w(px(260.0))
            .h_full()
            .min_h_0()
            .flex_shrink_0()
            .border_r_1()
            .border_color(border)
            .bg(muted)
            // 标题栏
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t!("ChatSession.history_title")),
                    )
                    .child(
                        Button::new("sql-new-session")
                            .icon(IconName::Plus)
                            .ghost()
                            .xsmall()
                            .on_click(move |_, _, _| on_new_session()),
                    ),
            )
            // 会话列表
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .when_some(session_list.cloned(), |this, list| {
                        this.child(
                            List::new(&list)
                                .w_full()
                                .h_full()
                                .border_1()
                                .border_color(border)
                                .rounded(cx.theme().radius),
                        )
                    }),
            )
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 格式化时间戳
pub fn format_timestamp(timestamp: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as i64;

    let diff = now.saturating_sub(timestamp);

    if diff < 60 {
        t!("ChatSession.time_just_now").to_string()
    } else if diff < 3600 {
        t!("ChatSession.time_minutes_ago", minutes = diff / 60).to_string()
    } else if diff < 86400 {
        t!("ChatSession.time_hours_ago", hours = diff / 3600).to_string()
    } else if diff < 604800 {
        t!("ChatSession.time_days_ago", days = diff / 86400).to_string()
    } else {
        t!("ChatSession.time_weeks_ago", weeks = diff / 604800).to_string()
    }
}
