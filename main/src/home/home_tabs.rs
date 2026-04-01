use crate::home_tab::HomePage;
use crate::setting_tab::{AppSettings, DatabaseOpenMode, SettingsPanel};
use db_view::chatdb::chat_panel::ChatPanel;
use db_view::database_tab::DatabaseTabView;
use gpui::AppContext;
use gpui::{App, BorrowAppContext, Context, Entity, Window};
use mongodb_view::MongoTabView;
use one_core::storage::{ConnectionType, StoredConnection, Workspace};
use one_core::tab_container::TabItem;
use redis_view::RedisTabView;
use sftp_view::{SftpView, SftpViewEvent};
use terminal::LocalConfig;
use terminal_view::{TerminalConnectionKind, TerminalTheme, TerminalView, TerminalViewEvent};

impl HomePage {
    fn terminal_sync_path_enabled(cx: &App) -> bool {
        if cx.has_global::<AppSettings>() {
            AppSettings::global(cx).terminal_sync_path_with_terminal
        } else {
            false
        }
    }

    fn register_terminal_view(&mut self, terminal_view: &Entity<TerminalView>) {
        self.terminal_views.retain(|view| view.upgrade().is_some());
        self.terminal_views.push(terminal_view.downgrade());
    }

    /// 注册终端视图：应用当前全局设置 + 绑定事件同步
    ///
    /// 所有创建 TerminalView 的地方都应调用此方法，
    /// 替代之前散落的 register + apply + bind × 2。
    fn setup_terminal_view(
        &mut self,
        terminal_view: &Entity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.register_terminal_view(terminal_view);

        // 从 AppSettings 读取所有终端设置并应用
        if cx.has_global::<AppSettings>() {
            let settings = AppSettings::global(cx);
            let font_size = settings.terminal_font_size as f32;
            let auto_copy = settings.terminal_auto_copy;
            let middle_click_paste = settings.terminal_middle_click_paste;
            let sync_path = settings.terminal_sync_path_with_terminal;
            let cursor_blink = settings.terminal_cursor_blink;
            let confirm_multiline = settings.terminal_confirm_multiline_paste;
            let confirm_high_risk = settings.terminal_confirm_high_risk_command;
            let theme = TerminalTheme::find_by_name(&settings.terminal_theme);

            terminal_view.update(cx, |view, cx| {
                view.apply_terminal_settings(
                    font_size,
                    auto_copy,
                    middle_click_paste,
                    sync_path,
                    window,
                    cx,
                );
                view.apply_cursor_blink(cursor_blink, window, cx);
                view.apply_confirm_multiline_paste(confirm_multiline, cx);
                view.apply_confirm_high_risk_command(confirm_high_risk, cx);
            });
            if let Some(theme) = theme {
                terminal_view.update(cx, |view, cx| {
                    view.apply_theme(&theme, window, cx);
                });
            }
        }

        // 单一订阅处理所有 TerminalViewEvent
        let subscription = cx.subscribe_in(
            terminal_view,
            window,
            |this, _view, event: &TerminalViewEvent, window, cx| {
                match event {
                    // ---- 持久化到 AppSettings 并同步 ----
                    TerminalViewEvent::FontSizeChanged { size } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_font_size = *size as f64;
                            s.save();
                        });
                        let settings = AppSettings::global(cx).clone();
                        this.apply_terminal_settings_to_all(&settings, window, cx);
                    }
                    TerminalViewEvent::AutoCopyChanged { enabled } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_auto_copy = *enabled;
                            s.save();
                        });
                        let settings = AppSettings::global(cx).clone();
                        this.apply_terminal_settings_to_all(&settings, window, cx);
                    }
                    TerminalViewEvent::MiddleClickPasteChanged { enabled } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_middle_click_paste = *enabled;
                            s.save();
                        });
                        let settings = AppSettings::global(cx).clone();
                        this.apply_terminal_settings_to_all(&settings, window, cx);
                    }
                    TerminalViewEvent::SyncPathChanged { enabled } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_sync_path_with_terminal = *enabled;
                            s.save();
                        });
                        let settings = AppSettings::global(cx).clone();
                        this.apply_terminal_settings_to_all(&settings, window, cx);
                    }

                    // ---- 持久化到 AppSettings 并同步 ----
                    TerminalViewEvent::ThemeChanged { theme } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_theme = theme.name.to_string();
                            s.save();
                        });
                        let theme = theme.clone();
                        this.for_each_terminal_view(window, cx, |view, window, cx| {
                            view.apply_theme(&theme, window, cx);
                        });
                    }
                    TerminalViewEvent::CursorBlinkChanged { enabled } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_cursor_blink = *enabled;
                            s.save();
                        });
                        let enabled = *enabled;
                        this.for_each_terminal_view(window, cx, |view, window, cx| {
                            view.apply_cursor_blink(enabled, window, cx);
                        });
                    }
                    TerminalViewEvent::ConfirmMultilinePasteChanged { enabled } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_confirm_multiline_paste = *enabled;
                            s.save();
                        });
                        let enabled = *enabled;
                        this.for_each_terminal_view(window, cx, |view, _window, cx| {
                            view.apply_confirm_multiline_paste(enabled, cx);
                        });
                    }
                    TerminalViewEvent::ConfirmHighRiskCommandChanged { enabled } => {
                        cx.update_global::<AppSettings, _>(|s, _| {
                            s.terminal_confirm_high_risk_command = *enabled;
                            s.save();
                        });
                        let enabled = *enabled;
                        this.for_each_terminal_view(window, cx, |view, _window, cx| {
                            view.apply_confirm_high_risk_command(enabled, cx);
                        });
                    }
                }
                cx.notify();
            },
        );
        self._subscriptions.push(subscription);
    }

    pub(crate) fn apply_terminal_settings_to_all(
        &mut self,
        settings: &AppSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let font_size = settings.terminal_font_size as f32;
        let auto_copy = settings.terminal_auto_copy;
        let middle_click_paste = settings.terminal_middle_click_paste;
        let sync_path = settings.terminal_sync_path_with_terminal;
        self.terminal_views.retain(|weak| {
            if let Some(view) = weak.upgrade() {
                view.update(cx, |view, cx| {
                    view.apply_terminal_settings(
                        font_size,
                        auto_copy,
                        middle_click_paste,
                        sync_path,
                        window,
                        cx,
                    );
                });
                true
            } else {
                false
            }
        });
    }

    /// 遍历所有存活的终端视图并执行回调，同时清理已释放的弱引用
    fn for_each_terminal_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        mut f: impl FnMut(&mut TerminalView, &mut Window, &mut Context<TerminalView>),
    ) {
        self.terminal_views.retain(|weak| {
            if let Some(entity) = weak.upgrade() {
                entity.update(cx, |view, cx| {
                    f(view, window, cx);
                });
                true
            } else {
                false
            }
        });
    }

    pub(crate) fn open_ssh_terminal(
        &mut self,
        conn: StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let conn_id = conn.id.unwrap_or(0);
        // 使用时间戳生成唯一 tab_id，支持同一连接打开多个 SSH 终端
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let tab_id = format!("ssh-terminal-{}-{}", conn_id, timestamp);

        // 统计同一连接的 SSH 终端数量，计算序号
        let prefix = format!("ssh-terminal-{}-", conn_id);
        let existing_count = self
            .tab_container
            .read(cx)
            .tabs()
            .iter()
            .filter(|t| t.id().starts_with(&prefix))
            .count();
        let tab_index = if existing_count > 0 {
            Some(existing_count + 1)
        } else {
            None
        };
        let sync_path = Self::terminal_sync_path_enabled(cx);

        let terminal_view = cx.new(|cx| {
            TerminalView::new_ssh_with_index(conn, tab_index, window, cx, None, sync_path)
        });
        self.setup_terminal_view(&terminal_view, window, cx);
        self.tab_container.update(cx, |tc, cx| {
            let tab = TabItem::new(tab_id, "ssh", terminal_view);
            tc.add_and_activate_tab_with_focus(tab, window, cx);
        });
    }

    pub(crate) fn open_serial_terminal(
        &mut self,
        conn: StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let conn_id = conn.id.unwrap_or(0);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let tab_id = format!("serial-terminal-{}-{}", conn_id, timestamp);

        let prefix = format!("serial-terminal-{}-", conn_id);
        let existing_count = self
            .tab_container
            .read(cx)
            .tabs()
            .iter()
            .filter(|t| t.id().starts_with(&prefix))
            .count();
        let tab_index = if existing_count > 0 {
            Some(existing_count + 1)
        } else {
            None
        };

        let terminal_view =
            cx.new(|cx| TerminalView::new_serial_with_index(conn, tab_index, window, cx));
        self.setup_terminal_view(&terminal_view, window, cx);
        self.tab_container.update(cx, |tc, cx| {
            let tab = TabItem::new(tab_id, "serial", terminal_view);
            tc.add_and_activate_tab_with_focus(tab, window, cx);
        });
    }

    pub(crate) fn open_sftp_view(
        &mut self,
        conn: StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let conn_id = conn.id.unwrap_or(0);
        // 使用时间戳生成唯一 tab_id，支持同一连接打开多个 SFTP 视图
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let tab_id = format!("sftp-{}-{}", conn_id, timestamp);

        // 统计同一连接的 SFTP 视图数量，计算序号
        let prefix = format!("sftp-{}-", conn_id);
        let existing_count = self
            .tab_container
            .read(cx)
            .tabs()
            .iter()
            .filter(|t| t.id().starts_with(&prefix))
            .count();
        let tab_index = if existing_count > 0 {
            Some(existing_count + 1)
        } else {
            None
        };

        // 创建 SftpView 并订阅终端打开事件
        let sftp_view = cx.new(|cx| SftpView::new_with_index(conn, tab_index, window, cx));
        let tab_container = self.tab_container.clone();

        let subscription = cx.subscribe_in(
            &sftp_view,
            window,
            move |this, _sftp, event: &SftpViewEvent, window, cx| {
                match event {
                    SftpViewEvent::OpenLocalTerminal { working_dir } => {
                        // 使用时间戳生成唯一 tab_id，支持打开多个本地终端
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0);
                        let config = LocalConfig {
                            working_dir: Some(working_dir.clone()),
                            ..Default::default()
                        };
                        let tab_id = format!("local-terminal-{}", ts);
                        // 统计已有本地终端数量
                        let existing = tab_container
                            .read(cx)
                            .tabs()
                            .iter()
                            .filter(|t| {
                                t.id().starts_with("local-terminal-")
                                    || t.id().starts_with("terminal-")
                            })
                            .count();
                        let idx = if existing > 0 {
                            Some(existing + 1)
                        } else {
                            None
                        };
                        let terminal_view =
                            cx.new(|cx| TerminalView::new_with_index(config, idx, window, cx));
                        this.setup_terminal_view(&terminal_view, window, cx);
                        tab_container.update(cx, |tc, cx| {
                            let tab = TabItem::new(tab_id, "terminal", terminal_view);
                            tc.add_and_activate_tab_with_focus(tab, window, cx);
                        });
                    }
                    SftpViewEvent::OpenSshTerminal {
                        connection,
                        working_dir,
                    } => {
                        // 使用时间戳生成唯一 tab_id，支持打开多个 SSH 终端
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0);
                        let conn_id = connection.id.unwrap_or(0);
                        let tab_id = format!("ssh-terminal-{}-{}", conn_id, ts);
                        let conn = connection.clone();
                        // 统计同一连接的 SSH 终端数量
                        let prefix = format!("ssh-terminal-{}-", conn_id);
                        let existing = tab_container
                            .read(cx)
                            .tabs()
                            .iter()
                            .filter(|t| t.id().starts_with(&prefix))
                            .count();
                        let idx = if existing > 0 {
                            Some(existing + 1)
                        } else {
                            None
                        };
                        let sync_path = HomePage::terminal_sync_path_enabled(cx);
                        let terminal_view = cx.new(|cx| {
                            TerminalView::new_ssh_with_index(
                                conn,
                                idx,
                                window,
                                cx,
                                Some(working_dir),
                                sync_path,
                            )
                        });
                        this.setup_terminal_view(&terminal_view, window, cx);
                        tab_container.update(cx, |tc, cx| {
                            let tab = TabItem::new(tab_id, "ssh", terminal_view);
                            tc.add_and_activate_tab_with_focus(tab, window, cx);
                        });
                    }
                }
            },
        );
        self._subscriptions.push(subscription);

        // 添加标签页
        let tab = TabItem::new(tab_id, "sftp", sftp_view);
        self.tab_container.update(cx, |tc, cx| {
            tc.add_and_activate_tab_with_focus(tab, window, cx);
        });
    }

    pub(crate) fn open_redis_tab(
        &mut self,
        conn: StoredConnection,
        workspace: Option<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let open_mode = if cx.has_global::<AppSettings>() {
            AppSettings::global(cx).database_open_mode
        } else {
            DatabaseOpenMode::default()
        };

        let workspace_id = workspace.as_ref().and_then(|ws| ws.id);
        let active_conn_id = conn.id;

        let (tab_id, connections, workspace_for_tab) = match open_mode {
            DatabaseOpenMode::Workspace if workspace_id.is_some() => {
                let connections = self
                    .connections
                    .iter()
                    .filter(|connection| connection.workspace_id == workspace_id)
                    .filter(|connection| connection.connection_type == ConnectionType::Redis)
                    .cloned()
                    .collect();
                let tab_id = format!("workspace-redis-tab-{}", workspace_id.unwrap_or(0));
                (tab_id, connections, workspace)
            }
            _ => {
                let conn_id = conn.id.unwrap_or(0);
                let tab_id = format!("redis-{}", conn_id);
                (tab_id, vec![conn.clone()], None)
            }
        };

        let tab_container = self.tab_container.clone();
        window.defer(cx, move |window, cx| {
            let tab_id_for_tab = tab_id.clone();
            tab_container.update(cx, |tc, cx| {
                tc.activate_or_add_tab_lazy(
                    tab_id,
                    move |window, cx| {
                        let redis_view = cx.new(|cx| {
                            RedisTabView::new_with_active_conn(
                                workspace_for_tab,
                                connections,
                                active_conn_id,
                                window,
                                cx,
                            )
                        });
                        TabItem::new(tab_id_for_tab, "redis", redis_view)
                    },
                    window,
                    cx,
                );
            });
        });
    }

    pub(crate) fn open_mongodb_tab(
        &mut self,
        conn: StoredConnection,
        workspace: Option<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let open_mode = if cx.has_global::<AppSettings>() {
            AppSettings::global(cx).database_open_mode
        } else {
            DatabaseOpenMode::default()
        };

        let workspace_id = workspace.as_ref().and_then(|ws| ws.id);
        let active_conn_id = conn.id;

        let (tab_id, connections, workspace_for_tab) = match open_mode {
            DatabaseOpenMode::Workspace if workspace_id.is_some() => {
                let connections = self
                    .connections
                    .iter()
                    .filter(|connection| connection.workspace_id == workspace_id)
                    .filter(|connection| connection.connection_type == ConnectionType::MongoDB)
                    .cloned()
                    .collect();
                let tab_id = format!("workspace-mongodb-tab-{}", workspace_id.unwrap_or(0));
                (tab_id, connections, workspace)
            }
            _ => {
                let conn_id = conn.id.unwrap_or(0);
                let tab_id = format!("mongodb-{}", conn_id);
                (tab_id, vec![conn.clone()], None)
            }
        };

        let tab_container = self.tab_container.clone();
        window.defer(cx, move |window, cx| {
            let tab_id_for_tab = tab_id.clone();
            tab_container.update(cx, |tc, cx| {
                tc.activate_or_add_tab_lazy(
                    tab_id,
                    move |window, cx| {
                        let mongo_view = cx.new(|cx| {
                            MongoTabView::new_with_active_conn(
                                workspace_for_tab,
                                connections,
                                active_conn_id,
                                window,
                                cx,
                            )
                        });
                        TabItem::new(tab_id_for_tab, "mongodb", mongo_view)
                    },
                    window,
                    cx,
                );
            });
        });
    }

    pub(crate) fn add_settings_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_container = self.tab_container.clone();
        window.defer(cx, move |window, cx| {
            tab_container.update(cx, |tc, cx| {
                tc.activate_or_add_tab_lazy(
                    "settings",
                    |win, cx| {
                        let settings = cx.new(|cx| SettingsPanel::new(win, cx));
                        TabItem::new("settings", "home", settings)
                    },
                    window,
                    cx,
                );
            });
        });
    }

    pub(crate) fn add_terminal_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 使用时间戳生成唯一 tab_id，支持打开多个本地终端
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let tab_id = format!("terminal-{}", timestamp);

        // 统计已有本地终端数量，计算序号
        let existing_count = self
            .tab_container
            .read(cx)
            .tabs()
            .iter()
            .filter(|t| t.id().starts_with("terminal-") || t.id().starts_with("local-terminal-"))
            .count();
        let tab_index = if existing_count > 0 {
            Some(existing_count + 1)
        } else {
            None
        };

        let tab_container = self.tab_container.clone();
        let home = cx.entity();
        window.defer(cx, move |window, cx| {
            home.update(cx, |this, cx| {
                let terminal_view = cx.new(|cx| {
                    TerminalView::new_with_index(LocalConfig::default(), tab_index, window, cx)
                });
                this.setup_terminal_view(&terminal_view, window, cx);
                tab_container.update(cx, |tc, cx| {
                    let tab = TabItem::new(tab_id, "home", terminal_view);
                    tc.add_and_activate_tab_with_focus(tab, window, cx);
                });
            });
        });
    }

    pub(crate) fn add_ai_chat_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_container = self.tab_container.clone();
        window.defer(cx, move |window, cx| {
            tab_container.update(cx, |tc, cx| {
                tc.activate_or_add_tab_lazy(
                    "ai-chat",
                    |win, cx| {
                        let ai_chat = cx.new(|x| ChatPanel::new(win, x));
                        TabItem::new("ai-chat", "home", ai_chat)
                    },
                    window,
                    cx,
                );
            });
        });
    }

    pub(crate) fn add_item_to_tab(
        &mut self,
        conn: &StoredConnection,
        workspace: Option<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 根据设置中的数据库打开方式决定如何打开
        let open_mode = if cx.has_global::<AppSettings>() {
            AppSettings::global(cx).database_open_mode
        } else {
            DatabaseOpenMode::default()
        };

        // 在 defer 之前准备所有需要的数据，避免在 HomePage 更新期间
        // 触发 on_deactivate 导致双重借用 panic
        let workspace_id = workspace.as_ref().and_then(|w| w.id);
        let conn_clone = conn.clone();
        let connections: Vec<StoredConnection> = match open_mode {
            DatabaseOpenMode::Workspace if workspace_id.is_some() => self
                .connections
                .iter()
                .filter(|c| c.workspace_id == workspace_id)
                .filter(|c| c.connection_type == ConnectionType::Database)
                .cloned()
                .collect(),
            _ => vec![conn.clone()],
        };

        let tab_container = self.tab_container.clone();
        window.defer(cx, move |window, cx| {
            tab_container.update(cx, |tc, cx| match open_mode {
                DatabaseOpenMode::Single => {
                    let tab_id = format!("database-tab-{}", conn_clone.id.unwrap_or(0));
                    tc.activate_or_add_tab_lazy(
                        tab_id.clone(),
                        move |window, cx| {
                            let db_view = cx.new(|cx| {
                                DatabaseTabView::new_with_active_conn(
                                    None,
                                    vec![conn_clone.clone()],
                                    conn_clone.id,
                                    window,
                                    cx,
                                )
                            });
                            TabItem::new(tab_id.clone(), "home", db_view)
                        },
                        window,
                        cx,
                    );
                }
                DatabaseOpenMode::Workspace => {
                    let tab_id = if workspace_id.is_some() {
                        format!("workspace-database-tab-{}", workspace_id.unwrap_or(0))
                    } else {
                        format!("database-tab-{}", conn_clone.id.unwrap_or(0))
                    };

                    let active_conn_id = conn_clone.id;
                    tc.activate_or_add_tab_lazy(
                        tab_id.clone(),
                        move |window, cx| {
                            let db_view = cx.new(|cx| {
                                DatabaseTabView::new_with_active_conn(
                                    workspace,
                                    connections,
                                    active_conn_id,
                                    window,
                                    cx,
                                )
                            });
                            TabItem::new(tab_id.clone(), "home", db_view)
                        },
                        window,
                        cx,
                    );
                }
            });
        });
    }

    /// 复制当前活动标签并打开
    pub(crate) fn duplicate_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tc = self.tab_container.read(cx);

        // pinned tab 不支持复制
        if tc.is_pinned_tab_active() {
            return;
        }

        let Some(active_tab) = tc.active_tab() else {
            return;
        };

        let content_key = active_tab.content().content_key(cx);

        match content_key {
            "Terminal" => {
                // 获取终端视图的连接信息
                let view = active_tab.content().view();
                let Ok(terminal_view) = view.downcast::<TerminalView>() else {
                    return;
                };

                let kind = terminal_view.read(cx).connection_kind(cx);
                match kind {
                    TerminalConnectionKind::Ssh => {
                        // SSH 终端：通过 connection_id 找到 StoredConnection 并打开新连接
                        let conn_id = terminal_view.read(cx).connection_id(cx);
                        if let Some(conn_id) = conn_id {
                            if let Some(conn) = self
                                .connections
                                .iter()
                                .find(|c| c.id == Some(conn_id))
                                .cloned()
                            {
                                self.open_ssh_terminal(conn, window, cx);
                            }
                        }
                    }
                    TerminalConnectionKind::Serial => {
                        let conn_id = terminal_view.read(cx).connection_id(cx);
                        if let Some(conn_id) = conn_id {
                            if let Some(conn) = self
                                .connections
                                .iter()
                                .find(|c| c.id == Some(conn_id))
                                .cloned()
                            {
                                self.open_serial_terminal(conn, window, cx);
                            }
                        }
                    }
                    TerminalConnectionKind::Local => {
                        // 本地终端：直接新建
                        self.add_terminal_tab(window, cx);
                    }
                }
            }
            _ => {
                // 其他类型暂不支持复制
            }
        }
    }
}
