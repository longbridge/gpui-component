//! 数据库视图侧边栏模块
//!
//! 提供数据库视图的侧边栏功能，包括：
//! - AI 聊天面板
//! - 可扩展的其他面板

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, Size, v_flex};
use one_core::ai_chat::ask_ai::{AskAiEvent, get_ask_ai_notifier};
use one_core::ai_chat::{AiChatPanel, AiChatPanelEvent, CodeBlockAction};
use one_core::layout::TOOLBAR_WIDTH;

/// 侧边栏面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    /// AI 聊天面板
    AiChat,
}

impl SidebarPanel {
    /// 获取面板图标
    pub fn icon(&self) -> IconName {
        match self {
            SidebarPanel::AiChat => IconName::Bot,
        }
    }
}

/// 数据库侧边栏事件
#[derive(Clone, Debug)]
pub enum DatabaseSidebarEvent {
    /// 面板切换
    PanelChanged,
    /// 请求询问 AI（由外部触发，内部处理）
    AskAi,
}

/// 数据库侧边栏组件
pub struct DatabaseSidebar {
    /// 当前激活的面板
    active_panel: Option<SidebarPanel>,
    /// AI 聊天面板
    ai_chat_panel: Entity<AiChatPanel>,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 是否处于激活状态（用于控制事件响应）
    is_active: bool,
    /// 订阅句柄
    _subs: Vec<Subscription>,
}

impl DatabaseSidebar {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ai_chat_panel = cx.new(|cx| AiChatPanel::new(window, cx));

        let mut subs = Vec::new();

        // 订阅 AI 聊天面板关闭事件
        subs.push(
            cx.subscribe(&ai_chat_panel, |this, _, event: &AiChatPanelEvent, cx| {
                if let AiChatPanelEvent::Close = event {
                    this.active_panel = None;
                    cx.emit(DatabaseSidebarEvent::PanelChanged);
                    cx.notify();
                }
            }),
        );

        // 订阅全局 AskAi 通知器
        if let Some(notifier) = get_ask_ai_notifier(cx) {
            subs.push(
                cx.subscribe(&notifier, move |this, _, event: &AskAiEvent, cx| {
                    // 只有激活的 tab 才响应事件
                    if this.is_active {
                        let AskAiEvent::Request(message) = event;
                        this.ask_ai(message.clone(), cx);
                    }
                }),
            );
        }

        Self {
            active_panel: None,
            ai_chat_panel,
            focus_handle: cx.focus_handle(),
            is_active: false,
            _subs: subs,
        }
    }

    /// 设置激活状态
    /// 当 tab 被激活时调用 set_active(true)，失活时调用 set_active(false)
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        self.is_active = active;
        cx.notify();
    }

    /// 设置激活的面板
    pub fn set_active_panel(&mut self, panel: Option<SidebarPanel>, cx: &mut Context<Self>) {
        if self.active_panel != panel {
            self.active_panel = panel;
            cx.emit(DatabaseSidebarEvent::PanelChanged);
            cx.notify();
        }
    }

    /// 切换面板
    pub fn toggle_panel(&mut self, panel: SidebarPanel, cx: &mut Context<Self>) {
        if self.active_panel == Some(panel) {
            self.set_active_panel(None, cx);
        } else {
            self.set_active_panel(Some(panel), cx);
        }
    }

    /// 是否显示侧边栏面板
    pub fn is_panel_visible(&self) -> bool {
        self.active_panel.is_some()
    }

    /// 询问 AI
    pub fn ask_ai(&mut self, message: String, cx: &mut Context<Self>) {
        // 显示 AI 面板
        if self.active_panel != Some(SidebarPanel::AiChat) {
            self.active_panel = Some(SidebarPanel::AiChat);
        }

        // 发送消息到 AI 聊天面板
        self.ai_chat_panel.update(cx, |panel, cx| {
            panel.send_external_message(message, cx);
        });

        cx.emit(DatabaseSidebarEvent::AskAi);
        cx.notify();
    }

    /// 注册代码块操作
    pub fn register_code_block_action(&self, action: CodeBlockAction, cx: &mut Context<Self>) {
        self.ai_chat_panel.update(cx, |panel, cx| {
            panel.register_code_block_action(action, cx);
        });
    }

    /// 渲染工具栏按钮
    fn render_toolbar_button(
        &self,
        panel: SidebarPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_panel == Some(panel);
        let accent_color = cx.theme().accent;
        let accent_fg = cx.theme().accent_foreground;
        let muted_fg = cx.theme().muted_foreground;
        let muted_bg = cx.theme().muted;

        div()
            .id(SharedString::from(format!("sidebar-btn-{:?}", panel)))
            .w(px(36.0))
            .h(px(36.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .when(is_active, |this| this.bg(accent_color))
            .when(!is_active, |this| this.hover(|s| s.bg(muted_bg)))
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.toggle_panel(panel, cx);
            }))
            .child(
                Icon::new(panel.icon())
                    .with_size(Size::Medium)
                    .text_color(if is_active { accent_fg } else { muted_fg }),
            )
    }

    /// 渲染工具栏
    pub fn render_toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let border_color = cx.theme().border;
        let muted_bg = cx.theme().muted;

        v_flex()
            .flex_shrink_0()
            .w(TOOLBAR_WIDTH)
            .h_full()
            .bg(muted_bg)
            .border_l_1()
            .border_color(border_color)
            .items_center()
            .py_2()
            .gap_1()
            .child(self.render_toolbar_button(SidebarPanel::AiChat, window, cx))
            .into_any_element()
    }

    /// 渲染面板内容
    pub fn render_panel_content(
        &self,
        panel: SidebarPanel,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        match panel {
            SidebarPanel::AiChat => self.ai_chat_panel.clone().into_any_element(),
        }
    }
}

impl EventEmitter<DatabaseSidebarEvent> for DatabaseSidebar {}

impl Focusable for DatabaseSidebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DatabaseSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border_color = cx.theme().border;
        let bg_color = cx.theme().background;

        div()
            .h_full()
            .flex_shrink_0()
            .when_some(self.active_panel, |this, panel| {
                this.w_full().child(
                    v_flex()
                        .size_full()
                        .border_l_1()
                        .border_color(border_color)
                        .bg(bg_color)
                        .child(self.render_panel_content(panel, window, cx)),
                )
            })
            .when(!self.is_panel_visible(), |this| {
                this.child(self.render_toolbar(window, cx))
            })
    }
}
