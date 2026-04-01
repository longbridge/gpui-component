//! 终端侧边栏模块
//!
//! 提供终端视图的侧边栏功能，包括：
//! - 设置面板（搜索、字体、主题）
//! - 快捷命令面板
//! - AI 聊天面板
//! - 文件管理器面板（仅 SSH 终端）

pub mod file_manager_panel;
mod quick_command_panel;
mod server_monitor_panel;
mod settings_panel;

pub use file_manager_panel::{FileManagerPanel, FileManagerPanelEvent};
pub use quick_command_panel::QuickCommandPanel;
pub use server_monitor_panel::{ServerMonitorPanel, ServerMonitorPanelEvent};
pub use settings_panel::SettingsPanel;

use crate::theme::{TerminalColors, TerminalTheme};
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window,
};
use gpui_component::{v_flex, ActiveTheme, Icon, IconName, Sizable, Size};
use one_core::layout::TOOLBAR_WIDTH;
use one_core::storage::models::StoredConnection;
use one_core::{AiChatPanel, AiChatPanelEvent, CodeBlockAction, LanguageMatcher};
use rust_i18n::t;
use terminal::terminal::SshTerminalConfig;

const TERMINAL_AI_SYSTEM_INSTRUCTION: &str = r#"你是终端侧边栏中的 Linux 命令助手，默认面向 Linux shell 环境回答。
请严格遵循以下规则：
1. 当用户请求安装、配置、排查、运维或执行命令时，优先返回可以直接在 Linux 终端执行的命令。
2. 所有命令都必须放在 Markdown 代码块中，代码块语言使用 bash。
3. 每个代码块只能包含一条命令，不要在同一个代码块中放多条命令，不要使用 &&、; 或换行把多个命令塞进同一个代码块，除非用户明确要求组合命令。
4. 如果任务需要多步骤，请拆成多个独立代码块，每个代码块只对应一步的一条命令。
5. 解释、注意事项、风险提示、步骤标题必须写在代码块外面，保持简洁。
6. 如果命令依赖 sudo、包管理器或发行版差异，请先简短说明再给命令。
7. 如果用户明确要求非 Linux 平台、非命令答案或更详细的解释，再按用户要求调整。"#;

/// 侧边栏面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    /// 设置面板（搜索 + 字体 + 主题）
    Settings,
    /// 快捷命令面板
    QuickCommand,
    /// AI 聊天面板
    AiChat,
    /// 文件管理器面板（仅 SSH 终端）
    FileManager,
    /// 服务器监控面板（仅 SSH 终端）
    ServerMonitor,
}

impl SidebarPanel {
    /// 获取面板图标
    pub fn icon(&self) -> Icon {
        match self {
            SidebarPanel::Settings => IconName::Settings.mono(),
            SidebarPanel::QuickCommand => IconName::SquareTerminal.mono(),
            SidebarPanel::AiChat => IconName::AI.color(),
            SidebarPanel::FileManager => IconName::FolderOpen.mono(),
            SidebarPanel::ServerMonitor => IconName::Monitor.color(),
        }
    }

    /// 获取面板标题
    pub fn title(&self) -> &'static str {
        match self {
            SidebarPanel::Settings => "Settings",
            SidebarPanel::QuickCommand => "Quick Commands",
            SidebarPanel::AiChat => "AI Chat",
            SidebarPanel::FileManager => "File Manager",
            SidebarPanel::ServerMonitor => "Server Monitor",
        }
    }
}

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
    /// 粘贴命令到终端输入区（不自动回车）
    ExecuteCommand(String),
    /// 请求询问 AI
    AskAi,
    /// 粘贴代码到终端（用于AI生成的代码块）
    PasteCodeToTerminal(String),
    /// 光标闪烁变更
    CursorBlinkChanged(bool),
    /// 非 bracketed 模式下，多行粘贴确认开关
    ConfirmMultilinePasteChanged(bool),
    /// 高危命令确认开关
    ConfirmHighRiskCommandChanged(bool),
    /// 选中自动复制开关
    AutoCopyChanged(bool),
    /// 中键粘贴开关
    MiddleClickPasteChanged(bool),
    /// 路径与终端同步开关
    SyncPathChanged(bool),
    /// 在终端中 cd 到指定路径
    CdToTerminal(String),
    /// 请求将终端当前工作目录同步到文件管理器
    SyncWorkingDir,
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
    /// 文件管理器面板（仅 SSH 终端时创建）
    file_manager_panel: Option<Entity<FileManagerPanel>>,
    /// 服务器监控面板（仅 SSH 终端时创建）
    server_monitor_panel: Option<Entity<ServerMonitorPanel>>,
    /// 路径与终端同步开关（默认开启）
    sync_path_enabled: bool,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 终端主题配色（用于侧边栏工具栏）
    colors: TerminalColors,
    /// 订阅句柄
    _subs: Vec<Subscription>,
}

impl TerminalSidebar {
    pub fn new(
        connection_id: Option<i64>,
        stored_connection: Option<StoredConnection>,
        ssh_config: Option<SshTerminalConfig>,
        initial_theme: &TerminalTheme,
        sync_path_enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let colors = initial_theme.colors();
        let has_file_manager = stored_connection.is_some();
        let auto_show_server_monitor = ServerMonitorPanel::load_monitor_enabled(connection_id);
        let settings_panel = cx.new(|cx| {
            SettingsPanel::new(
                initial_theme,
                has_file_manager,
                true,
                true,
                sync_path_enabled,
                window,
                cx,
            )
        });
        let quick_command_panel = cx.new(|cx| QuickCommandPanel::new(connection_id, window, cx));
        let ai_chat_panel = cx.new(|cx| AiChatPanel::new(window, cx));

        // 仅 SSH 终端（有 StoredConnection）时创建文件管理器面板
        let file_manager_panel =
            stored_connection.map(|conn| cx.new(|cx| FileManagerPanel::new(conn, window, cx)));
        let server_monitor_panel = ssh_config.map(|config| {
            cx.new(|cx| {
                ServerMonitorPanel::new(
                    connection_id,
                    config.ssh_config.clone(),
                    auto_show_server_monitor,
                    cx,
                )
            })
        });

        // 注册 bash/sh 代码块操作，并注入终端专属提示词
        let sidebar_entity = cx.entity();
        ai_chat_panel.update(cx, |panel, cx| {
            panel.set_system_instruction(Some(TERMINAL_AI_SYSTEM_INSTRUCTION.to_string()), cx);
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
        let set_sub = cx.subscribe(
            &settings_panel,
            |this, _, event: &settings_panel::SettingsPanelEvent, cx| match event {
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
                settings_panel::SettingsPanelEvent::ConfirmMultilinePasteChanged(enabled) => {
                    cx.emit(TerminalSidebarEvent::ConfirmMultilinePasteChanged(*enabled));
                }
                settings_panel::SettingsPanelEvent::ConfirmHighRiskCommandChanged(enabled) => {
                    cx.emit(TerminalSidebarEvent::ConfirmHighRiskCommandChanged(
                        *enabled,
                    ));
                }
                settings_panel::SettingsPanelEvent::AutoCopyChanged(enabled) => {
                    cx.emit(TerminalSidebarEvent::AutoCopyChanged(*enabled));
                }
                settings_panel::SettingsPanelEvent::MiddleClickPasteChanged(enabled) => {
                    cx.emit(TerminalSidebarEvent::MiddleClickPasteChanged(*enabled));
                }
                settings_panel::SettingsPanelEvent::SyncPathChanged(enabled) => {
                    this.sync_path_enabled = *enabled;
                    cx.emit(TerminalSidebarEvent::SyncPathChanged(*enabled));
                }
            },
        );

        // 订阅快捷命令面板事件
        let quick_sub = cx.subscribe(
            &quick_command_panel,
            |this, _, event: &quick_command_panel::QuickCommandPanelEvent, cx| match event {
                quick_command_panel::QuickCommandPanelEvent::Close => {
                    this.set_active_panel(None, cx);
                }
                quick_command_panel::QuickCommandPanelEvent::ExecuteCommand(cmd) => {
                    cx.emit(TerminalSidebarEvent::ExecuteCommand(cmd.clone()));
                }
            },
        );

        // 订阅 AI 聊天面板关闭事件
        let ai_chat_sub = cx.subscribe(&ai_chat_panel, |this, _, event: &AiChatPanelEvent, cx| {
            if let AiChatPanelEvent::Close = event {
                this.active_panel = None;
                cx.emit(TerminalSidebarEvent::PanelChanged(None));
                cx.notify();
            }
        });

        let mut subs = vec![set_sub, quick_sub, ai_chat_sub];

        // 订阅文件管理器面板事件
        if let Some(ref fm_panel) = file_manager_panel {
            let fm_sub =
                cx.subscribe(
                    fm_panel,
                    |this, _, event: &FileManagerPanelEvent, cx| match event {
                        FileManagerPanelEvent::Close => {
                            this.set_active_panel(None, cx);
                        }
                        FileManagerPanelEvent::CdToTerminal(path) => {
                            cx.emit(TerminalSidebarEvent::CdToTerminal(path.clone()));
                        }
                        FileManagerPanelEvent::SyncWorkingDir => {
                            cx.emit(TerminalSidebarEvent::SyncWorkingDir);
                        }
                    },
                );
            subs.push(fm_sub);
        }

        if let Some(ref monitor_panel) = server_monitor_panel {
            let monitor_sub = cx.subscribe(
                monitor_panel,
                |this, _, event: &ServerMonitorPanelEvent, cx| match event {
                    ServerMonitorPanelEvent::Close => {
                        this.set_active_panel(None, cx);
                    }
                },
            );
            subs.push(monitor_sub);
        }

        Self {
            active_panel: None,
            settings_panel,
            quick_command_panel,
            ai_chat_panel,
            file_manager_panel,
            server_monitor_panel,
            sync_path_enabled,
            focus_handle: cx.focus_handle(),
            colors,
            _subs: subs,
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
            // 文件管理器首次激活时自动建立连接
            if panel == SidebarPanel::FileManager {
                if let Some(ref fm_panel) = self.file_manager_panel {
                    fm_panel.update(cx, |panel, cx| {
                        // 仅在 Idle 状态时自动连接
                        panel.connect_if_idle(cx);
                    });
                }
            }
            if panel == SidebarPanel::ServerMonitor {
                if let Some(ref monitor_panel) = self.server_monitor_panel {
                    monitor_panel.update(cx, |panel, cx| {
                        panel.restore_monitoring(cx);
                    });
                }
            }
            self.set_active_panel(Some(panel), cx);
        }
    }

    /// 是否显示侧边栏
    pub fn is_visible(&self) -> bool {
        self.active_panel.is_some()
    }

    /// 更新设置面板的当前主题
    pub fn update_current_theme(
        &mut self,
        theme: &TerminalTheme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.colors = theme.colors();
        // 更新设置面板（会同时更新颜色和主题）
        let theme_clone = theme.clone();
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_current_theme(theme_clone, window, cx);
        });

        cx.notify();
    }

    pub fn set_auto_copy(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_auto_copy(enabled, cx);
        });
    }

    pub fn set_middle_click_paste(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_middle_click_paste(enabled, cx);
        });
    }

    pub fn set_sync_path_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.sync_path_enabled = enabled;
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_sync_path(enabled, cx);
        });
    }

    pub fn set_cursor_blink(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_cursor_blink(enabled, cx);
        });
    }

    pub fn set_confirm_multiline_paste(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_confirm_multiline_paste(enabled, cx);
        });
    }

    pub fn set_confirm_high_risk_command(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings_panel.update(cx, |panel, cx| {
            panel.set_confirm_high_risk_command(enabled, cx);
        });
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

    /// 从终端 OSC 7 同步路径到文件管理器
    ///
    /// 检查 `sync_path_enabled` 且存在文件管理器面板时，导航到指定路径。
    pub fn sync_file_manager_path(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.sync_path_enabled {
            return;
        }
        if let Some(ref fm_panel) = self.file_manager_panel {
            fm_panel.update(cx, |panel, cx| {
                panel.sync_navigate_to(path, cx);
            });
        }
    }

    /// 设置文件管理器的初始工作目录（连接前调用）
    ///
    /// 当终端收到 OSC 7 但文件管理器尚未连接时，缓存路径供首次连接使用。
    pub fn set_file_manager_initial_dir(&mut self, path: String, cx: &mut Context<Self>) {
        if let Some(ref fm_panel) = self.file_manager_panel {
            fm_panel.update(cx, |panel, _cx| {
                panel.set_initial_working_dir(path);
            });
        }
    }

    /// 在终端重连时同步重建文件管理器连接
    pub fn reconnect_file_manager(&mut self, working_dir: Option<String>, cx: &mut Context<Self>) {
        if let Some(ref fm_panel) = self.file_manager_panel {
            fm_panel.update(cx, |panel, cx| {
                panel.reconnect_with_working_dir(working_dir.clone(), cx);
            });
        }
    }

    pub fn reconnect_server_monitor(&mut self, cx: &mut Context<Self>) {
        if let Some(ref monitor_panel) = self.server_monitor_panel {
            monitor_panel.update(cx, |panel, cx| {
                panel.reconnect(cx);
            });
        }
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
        let border_color = self.colors.border;
        let muted_bg = self.colors.background;
        let has_file_manager = self.file_manager_panel.is_some();
        let has_server_monitor = self.server_monitor_panel.is_some();

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
            .when(has_file_manager, |this| {
                this.child(self.render_toolbar_button(SidebarPanel::FileManager, window, cx))
            })
            .when(has_server_monitor, |this| {
                this.child(self.render_toolbar_button(SidebarPanel::ServerMonitor, window, cx))
            })
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
            SidebarPanel::Settings => self.settings_panel.clone().into_any_element(),
            SidebarPanel::QuickCommand => self.quick_command_panel.clone().into_any_element(),
            SidebarPanel::AiChat => self.ai_chat_panel.clone().into_any_element(),
            SidebarPanel::FileManager => {
                if let Some(ref fm_panel) = self.file_manager_panel {
                    fm_panel.clone().into_any_element()
                } else {
                    div().into_any_element()
                }
            }
            SidebarPanel::ServerMonitor => {
                if let Some(ref monitor_panel) = self.server_monitor_panel {
                    monitor_panel.clone().into_any_element()
                } else {
                    div().into_any_element()
                }
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
