//! 终端侧边栏模块
//!
//! 提供终端视图的侧边栏功能，包括：
//! - 设置面板（搜索、字体、主题）
//! - 快捷命令面板
//! - AI 聊天面板

mod quick_command_panel;
mod settings_panel;

pub use quick_command_panel::QuickCommandPanel;
pub use settings_panel::SettingsPanel;

use gpui::{px, App, AppContext, AnyElement, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Pixels, Render, SharedString, Styled, Window, InteractiveElement, StatefulInteractiveElement, div, Subscription};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    v_flex, ActiveTheme, Icon, IconName, Sizable, Size,
};
use one_core::{AiChatPanel, AiChatPanelEvent, CodeBlockAction, LanguageMatcher};
use crate::theme::{TerminalColors, TerminalTheme};
use rust_i18n::t;

/// 侧边栏面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    /// 设置面板（搜索 + 字体 + 主题）
    Settings,
    /// 快捷命令面板
    QuickCommand,
    /// AI 聊天面板
    AiChat,
}

impl SidebarPanel {
    /// 获取面板图标
    pub fn icon(&self) -> IconName {
        match self {
            SidebarPanel::Settings => IconName::Settings,
            SidebarPanel::QuickCommand => IconName::SquareTerminal,
            SidebarPanel::AiChat => IconName::Bot,
        }
    }

    /// 获取面板标题
    pub fn title(&self) -> &'static str {
        match self {
            SidebarPanel::Settings => "Settings",
            SidebarPanel::QuickCommand => "Quick Commands",
            SidebarPanel::AiChat => "AI Chat",
        }
    }
}

/// 侧边栏默认宽度
pub const SIDEBAR_DEFAULT_WIDTH: Pixels = px(380.0);
/// 侧边栏最小宽度
pub const SIDEBAR_MIN_WIDTH: Pixels = px(220.0);
/// 侧边栏最大宽度
pub const SIDEBAR_MAX_WIDTH: Pixels = px(500.0);
/// 工具栏宽度
pub const TOOLBAR_WIDTH: Pixels = px(44.0);

/// 终端侧边栏事件
#[derive(Clone, Debug)]
pub enum TerminalSidebarEvent {
    /// 面板切换
    PanelChanged(Option<SidebarPanel>),
    /// 搜索模式变化
    SearchPatternChanged(String),
    /// 搜索前一个
    SearchPrevious,
    /// 搜索下一个
    SearchNext,
    /// 字体大小变更
    FontSizeChanged(f32),
    /// 字体变更
    FontFamilyChanged(String),
    /// 主题变更
    ThemeChanged(TerminalTheme),
    /// 执行历史命令
    ExecuteCommand(String),
    /// 请求询问 AI
    AskAi,
    /// 粘贴代码到终端（用于AI生成的代码块）
    PasteCodeToTerminal(String),
    /// 光标闪烁变更
    CursorBlinkChanged(bool),
}

/// 终端侧边栏组件
pub struct TerminalSidebar {
    /// 当前激活的面板
    active_panel: Option<SidebarPanel>,
    /// 设置面板
    settings_panel: Entity<SettingsPanel>,
    /// 快捷命令面板
    quick_command_panel: Entity<QuickCommandPanel>,
    /// AI 聊天面板
    ai_chat_panel: Entity<AiChatPanel>,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 终端主题配色（用于侧边栏工具栏）
    colors: TerminalColors,
    /// 订阅句柄
    _subs: Vec<Subscription>
}

impl TerminalSidebar {
    pub fn new(
        connection_id: Option<i64>,
        initial_theme: &TerminalTheme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let colors = initial_theme.colors();
        let settings_panel = cx.new(|cx| SettingsPanel::new(initial_theme, window, cx));
        let quick_command_panel = cx.new(|cx| QuickCommandPanel::new(connection_id, window, cx));
        let ai_chat_panel = cx.new(|cx| AiChatPanel::new(window, cx));

        // 注册 bash/sh 代码块操作
        let sidebar_entity = cx.entity();
        ai_chat_panel.update(cx, |panel, cx| {
            // 注册复制操作（默认已有，这里只是确保）
            // 注册粘贴到终端操作
            if let Some(paste_action) = CodeBlockAction::new("paste-to-terminal")
                .icon(IconName::SquareTerminal)
                .label(t!("TerminalSidebar.paste_to_terminal").to_string())
                .matcher(LanguageMatcher::shell())
                .on_click({
                    let sidebar = sidebar_entity.clone();
                    move |code, _lang, _window, cx| {
                        sidebar.update(cx, |_this, cx| {
                            cx.emit(TerminalSidebarEvent::PasteCodeToTerminal(code.clone()));
                        });
                    }
                })
                .build()
            {
                panel.register_code_block_action(paste_action, cx);
            }
        });

        // 订阅设置面板事件
        let set_sub = cx.subscribe(&settings_panel, |this, _, event: &settings_panel::SettingsPanelEvent, cx| {
            match event {
                settings_panel::SettingsPanelEvent::Close => {
                    this.set_active_panel(None, cx);
                }
                settings_panel::SettingsPanelEvent::SearchPatternChanged(pattern) => {
                    cx.emit(TerminalSidebarEvent::SearchPatternChanged(pattern.clone()));
                }
                settings_panel::SettingsPanelEvent::SearchPrevious => {
                    cx.emit(TerminalSidebarEvent::SearchPrevious);
                }
                settings_panel::SettingsPanelEvent::SearchNext => {
                    cx.emit(TerminalSidebarEvent::SearchNext);
                }
                settings_panel::SettingsPanelEvent::FontSizeChanged(size) => {
                    cx.emit(TerminalSidebarEvent::FontSizeChanged(*size));
                }
                settings_panel::SettingsPanelEvent::FontFamilyChanged(family) => {
                    cx.emit(TerminalSidebarEvent::FontFamilyChanged(family.clone()));
                }
                settings_panel::SettingsPanelEvent::ThemeChanged(theme) => {
                    this.colors = theme.colors();
                    cx.emit(TerminalSidebarEvent::ThemeChanged(theme.clone()));
                }
                settings_panel::SettingsPanelEvent::CursorBlinkChanged(enabled) => {
                    cx.emit(TerminalSidebarEvent::CursorBlinkChanged(*enabled));
                }
            }
        });

        // 订阅快捷命令面板事件
        let quick_sub = cx.subscribe(&quick_command_panel, |this, _, event: &quick_command_panel::QuickCommandPanelEvent, cx| {
            match event {
                quick_command_panel::QuickCommandPanelEvent::Close => {
                    this.set_active_panel(None, cx);
                }
                quick_command_panel::QuickCommandPanelEvent::ExecuteCommand(cmd) => {
                    cx.emit(TerminalSidebarEvent::ExecuteCommand(cmd.clone()));
                }
            }
        });

        // 订阅 AI 聊天面板关闭事件
        let ai_chat_sub = cx.subscribe(&ai_chat_panel, |this, _, event: &AiChatPanelEvent, cx| {
            if let AiChatPanelEvent::Close = event {
                this.active_panel = None;
                cx.emit(TerminalSidebarEvent::PanelChanged(None));
                cx.notify();
            }
        });

        Self {
            active_panel: None,
            settings_panel,
            quick_command_panel,
            ai_chat_panel,
            focus_handle: cx.focus_handle(),
            colors,
            _subs: vec![set_sub, quick_sub, ai_chat_sub],
        }
    }

    /// 获取当前激活的面板
    pub fn active_panel(&self) -> Option<SidebarPanel> {
        self.active_panel
    }

    /// 设置激活的面板
    pub fn set_active_panel(&mut self, panel: Option<SidebarPanel>, cx: &mut Context<Self>) {
        if self.active_panel != panel {
            self.active_panel = panel;
            cx.emit(TerminalSidebarEvent::PanelChanged(panel));
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

    /// 是否显示侧边栏
    pub fn is_visible(&self) -> bool {
        self.active_panel.is_some()
    }

    /// 更新设置面板的当前主题
    pub fn update_current_theme(&mut self, theme: &TerminalTheme, window: &mut Window, cx: &mut Context<Self>) {
        self.colors = theme.colors();
        // 更新设置面板（会同时更新颜色和主题）
        let theme_clone = theme.clone();
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_current_theme(theme_clone, window, cx);
        });

        cx.notify();
    }

    /// 更新搜索输入框的值
    pub fn set_search_value(&self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_search_value(value, window, cx);
        });
    }

    /// 获取搜索输入框的值
    pub fn search_value(&self, cx: &App) -> String {
        self.settings_panel.read(cx).search_value(cx)
    }

    /// 询问 AI
    pub fn ask_ai(&mut self, message: String, cx: &mut Context<Self>) {
        // 打开 AI 聊天面板
        if self.active_panel != Some(SidebarPanel::AiChat) {
            self.active_panel = Some(SidebarPanel::AiChat);
        }

        // 发送消息到 AI 聊天面板
        self.ai_chat_panel.update(cx, |panel, cx| {
            panel.send_external_message(message, cx);
        });

        cx.emit(TerminalSidebarEvent::AskAi);
        cx.notify();
    }

    /// 添加快捷命令（外部调用）
    pub fn add_quick_command(&self, command: String, cx: &mut Context<Self>) {
        self.quick_command_panel.update(cx, |panel, cx| {
            panel.add_command_external(command, cx);
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
        let accent_color = self.colors.accent;
        let accent_fg = self.colors.accent_foreground;
        let muted_fg = self.colors.muted_foreground;
        let muted_bg = self.colors.muted;

        div()
            .id(SharedString::from(format!("toolbar-btn-{:?}", panel)))
            .w(px(36.0))
            .h(px(36.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .when(is_active, |this| {
                this.bg(accent_color)
            })
            .when(!is_active, |this| {
                this.hover(|s| s.bg(muted_bg))
            })
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.toggle_panel(panel, cx);
            }))
            .child(
                Icon::new(panel.icon())
                    .with_size(Size::Medium)
                    .text_color(if is_active { accent_fg } else { muted_fg })
            )
    }

    /// 渲染工具栏
    pub fn render_toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let border_color = self.colors.border;
        let muted_bg = self.colors.background;

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
            .child(self.render_toolbar_button(SidebarPanel::Settings, window, cx))
            .child(self.render_toolbar_button(SidebarPanel::QuickCommand, window, cx))
            .child(self.render_toolbar_button(SidebarPanel::AiChat, window, cx))
            .into_any_element()
    }

    /// 渲染面板内容
    pub fn render_panel_content(&self, panel: SidebarPanel, _window: &mut Window, _cx: &mut Context<Self>) -> AnyElement {
        match panel {
            SidebarPanel::Settings => {
                self.settings_panel.clone().into_any_element()
            }
            SidebarPanel::QuickCommand => {
                self.quick_command_panel.clone().into_any_element()
            }
            SidebarPanel::AiChat => {
                self.ai_chat_panel.clone().into_any_element()
            }
        }
    }
}

impl EventEmitter<TerminalSidebarEvent> for TerminalSidebar {}

impl Focusable for TerminalSidebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalSidebar {
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
            .when(!self.is_visible(), |this| {
                this.child(self.render_toolbar(window, cx))
            })
    }
}
