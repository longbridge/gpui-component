//! 会话列表组件 - 通用的历史会话列表
//!
//! 提供可复用的会话列表组件，支持：
//! - 会话列表显示和搜索
//! - 可选的编辑和删除按钮
//! - 通过 trait 自定义回调处理

use gpui::{
    div, prelude::FluentBuilder, App, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Task, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex,
    list::{ListDelegate, ListState},
    ActiveTheme, IconName, IndexPath, Selectable, Sizable,
};
use rust_i18n::t;

// ============================================================================
// 会话数据
// ============================================================================

/// 会话数据
#[derive(Clone, Debug)]
pub struct SessionData {
    /// 会话 ID
    pub id: i64,
    /// 会话名称
    pub name: SharedString,
    /// 更新时间戳
    pub updated_at: i64,
}

impl SessionData {
    /// 创建新的会话数据
    pub fn new(id: i64, name: impl Into<SharedString>, updated_at: i64) -> Self {
        Self {
            id,
            name: name.into(),
            updated_at,
        }
    }
}

// ============================================================================
// 会话列表宿主 Trait
// ============================================================================

/// 会话列表宿主接口
///
/// 实现此 trait 的类型可以作为会话列表的宿主，接收会话操作的回调。
pub trait SessionListHost: 'static + Sized {
    /// 处理会话选择
    fn on_session_select(&mut self, session_id: i64, cx: &mut Context<Self>);

    /// 处理会话编辑（可选）
    fn on_session_edit(&mut self, _session_id: i64, _name: String, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// 处理会话删除（可选）
    fn on_session_delete(&mut self, _session_id: i64, _cx: &mut Context<Self>) {}

    /// 判断是否为当前会话
    fn is_current_session(&self, session_id: i64) -> bool;

    /// 处理列表确认后的操作（如关闭弹出框）
    fn on_session_list_confirm(&mut self, _cx: &mut Context<Self>) {}

    /// 处理列表取消（如关闭弹出框）
    fn on_session_list_cancel(&mut self, _cx: &mut Context<Self>) {}
}

// ============================================================================
// 会话列表配置
// ============================================================================

/// 会话列表配置
#[derive(Clone, Debug)]
pub struct SessionListConfig {
    /// 是否显示编辑按钮
    pub show_edit_button: bool,
    /// 是否显示删除按钮
    pub show_delete_button: bool,
}

impl Default for SessionListConfig {
    fn default() -> Self {
        Self {
            show_edit_button: true,
            show_delete_button: true,
        }
    }
}

impl SessionListConfig {
    /// 创建不显示任何操作按钮的配置
    pub fn no_actions() -> Self {
        Self {
            show_edit_button: false,
            show_delete_button: false,
        }
    }

    /// 设置是否显示编辑按钮
    pub fn with_edit_button(mut self, show: bool) -> Self {
        self.show_edit_button = show;
        self
    }

    /// 设置是否显示删除按钮
    pub fn with_delete_button(mut self, show: bool) -> Self {
        self.show_delete_button = show;
        self
    }
}

// ============================================================================
// 会话列表项
// ============================================================================

/// 会话列表项
#[derive(IntoElement)]
pub struct SessionListItem<H: SessionListHost> {
    session: SessionData,
    selected: bool,
    config: SessionListConfig,
    host: Entity<H>,
}

impl<H: SessionListHost> SessionListItem<H> {
    /// 创建新的会话列表项
    pub fn new(session: SessionData, config: SessionListConfig, host: Entity<H>) -> Self {
        Self {
            session,
            selected: false,
            config,
            host,
        }
    }
}

impl<H: SessionListHost> Selectable for SessionListItem<H> {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl<H: SessionListHost> RenderOnce for SessionListItem<H> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let session_id = self.session.id;
        let session_name = self.session.name.clone();
        let is_current = self.host.read(cx).is_current_session(session_id);
        let host = self.host.clone();

        let container_id = SharedString::from(format!("session-item-{}", session_id));

        let group_name = SharedString::from(format!("session-group-{}", session_id));

        let mut row = h_flex()
            .id(container_id)
            .group(group_name.clone())
            .w_full()
            .gap_2()
            .items_center()
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .when(self.selected, |this| this.bg(cx.theme().list_active))
            .when(is_current, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .on_click({
                let host = host.clone();
                move |_, _window, cx| {
                    cx.stop_propagation();
                    host.update(cx, |this, cx| {
                        this.on_session_select(session_id, cx);
                    });
                }
            })
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(self.session.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_current {
                                cx.theme().accent_foreground
                            } else {
                                cx.theme().muted_foreground
                            })
                            .child(format_timestamp(self.session.updated_at)),
                    ),
            );

        // 添加操作按钮
        if self.config.show_edit_button || self.config.show_delete_button {
            let mut actions = h_flex()
                .gap_1()
                .items_center()
                .invisible()
                .group_hover(group_name.clone(), |this| this.visible());

            if self.config.show_edit_button {
                let host = host.clone();
                let name = session_name.clone();
                actions = actions.child(
                    Button::new(SharedString::from(format!("edit-{}", session_id)))
                        .icon(IconName::Edit)
                        .ghost()
                        .xsmall()
                        .on_click(move |_, window: &mut Window, cx: &mut App| {
                            cx.stop_propagation();
                            host.update(cx, |this, cx| {
                                this.on_session_edit(session_id, name.to_string(), window, cx);
                            });
                        }),
                );
            }

            if self.config.show_delete_button {
                let host = host.clone();
                actions = actions.child(
                    Button::new(SharedString::from(format!("delete-{}", session_id)))
                        .icon(IconName::Remove)
                        .ghost()
                        .xsmall()
                        .on_click(move |_, _window: &mut Window, cx: &mut App| {
                            cx.stop_propagation();
                            host.update(cx, |this, cx| {
                                this.on_session_delete(session_id, cx);
                            });
                        }),
                );
            }

            row = row.child(actions);
        }

        row.into_any_element()
    }
}

// ============================================================================
// 会话列表代理
// ============================================================================

/// 会话列表代理
pub struct SessionListDelegate<H: SessionListHost> {
    host: Entity<H>,
    sessions: Vec<SessionData>,
    filtered_sessions: Vec<SessionData>,
    selected_index: Option<IndexPath>,
    config: SessionListConfig,
}

impl<H: SessionListHost> SessionListDelegate<H> {
    /// 创建新的会话列表代理
    pub fn new(host: Entity<H>, sessions: Vec<SessionData>, config: SessionListConfig) -> Self {
        let filtered = sessions.clone();
        Self {
            host,
            sessions,
            filtered_sessions: filtered,
            selected_index: None,
            config,
        }
    }

    /// 更新会话列表
    pub fn update_sessions(&mut self, sessions: Vec<SessionData>) {
        self.sessions = sessions.clone();
        self.filtered_sessions = sessions;
    }
}

impl<H: SessionListHost> ListDelegate for SessionListDelegate<H> {
    type Item = SessionListItem<H>;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        if query.is_empty() {
            self.filtered_sessions = self.sessions.clone();
        } else {
            let query_lower = query.to_lowercase();
            self.filtered_sessions = self
                .sessions
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();
        }
        cx.notify();
        Task::ready(())
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
        let session = self.filtered_sessions.get(ix.row)?.clone();
        Some(SessionListItem::new(session, self.config.clone(), self.host.clone()))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, cx: &mut Context<ListState<Self>>) {
        if let Some(ix) = self.selected_index {
            if let Some(session) = self.filtered_sessions.get(ix.row) {
                let session_id = session.id;
                self.host.update(cx, |this, cx| {
                    this.on_session_select(session_id, cx);
                    this.on_session_list_confirm(cx);
                });
            }
        }
    }

    fn cancel(&mut self, _window: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.host.update(cx, |this, cx| {
            this.on_session_list_cancel(cx);
        });
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 格式化时间戳
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{Local, TimeZone};

    let dt = Local.timestamp_opt(timestamp, 0).single();
    match dt {
        Some(dt) => {
            let now = Local::now();
            let diff = now.signed_duration_since(dt);

            if diff.num_days() == 0 {
                dt.format("%H:%M").to_string()
            } else if diff.num_days() < 7 {
                dt.format("%m-%d %H:%M").to_string()
            } else {
                dt.format("%Y-%m-%d").to_string()
            }
        }
        None => t!("AiChat.unknown_time").to_string(),
    }
}
