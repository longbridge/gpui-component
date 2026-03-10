//! 终端侧边栏文件管理器面板
//!
//! 仅针对 SSH 终端，通过独立的 SFTP 连接浏览远程文件系统。
//! UI 参考 `sftp_view` 的 `FileListPanel`，但为侧边栏场景做了精简和适配。

use chrono::{DateTime, Local};
use gpui::{
    div, prelude::*, px, uniform_list, App, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, IntoElement, ListSizingBehavior, MouseButton, MouseDownEvent,
    ParentElement, Render, SharedString, Styled, UniformListScrollHandle, Window,
};
use gpui_component::{
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    spinner::Spinner,
    tooltip::Tooltip,
    v_flex, ActiveTheme, Icon, IconName, InteractiveElementExt, Sizable, Size,
};
use one_core::gpui_tokio::Tokio;
use one_core::storage::models::{ProxyType as StorageProxyType, SshAuthMethod, StoredConnection};
use rust_i18n::t;
use sftp::{RusshSftpClient, SftpClient};
use ssh::{JumpServerConnectConfig, ProxyConnectConfig, ProxyType, SshAuth, SshConnectConfig};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

/// SFTP 连接状态
#[derive(Debug, Clone, PartialEq, Eq)]
enum ConnectionState {
    /// 初始状态，尚未连接
    Idle,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 连接失败
    Error(String),
}

/// 排序列
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SortColumn {
    Name,
    Size,
    Modified,
}

/// 排序方向
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SortOrder {
    Ascending,
    Descending,
}

/// 远程文件项
#[derive(Clone, Debug)]
struct RemoteFileItem {
    name: String,
    size: u64,
    modified: SystemTime,
    is_dir: bool,
}

/// 文件管理器面板事件
#[derive(Clone, Debug)]
pub enum FileManagerPanelEvent {
    /// 关闭面板
    Close,
    /// 在终端中 cd 到指定路径
    CdToTerminal(String),
}

/// 格式化文件大小（紧凑格式，适合侧边栏窄列）
fn format_file_size(size: u64) -> String {
    if size == 0 {
        return "-".to_string();
    }
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}

/// 格式化修改时间（短格式，适合侧边栏）
fn format_modified_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    let now = Local::now();
    // 同年使用 M/D HH:MM，不同年使用 YYYY/M/D
    if datetime.format("%Y").to_string() == now.format("%Y").to_string() {
        datetime.format("%-m/%-d %H:%M").to_string()
    } else {
        datetime.format("%Y/%-m/%-d").to_string()
    }
}

/// 从 StoredConnection 构建 SshConnectConfig
fn build_ssh_config(conn: &StoredConnection) -> anyhow::Result<SshConnectConfig> {
    let ssh_params = conn.to_ssh_params().map_err(|e| anyhow::anyhow!("{}", e))?;

    let auth = match ssh_params.auth_method {
        SshAuthMethod::Password { password } => SshAuth::Password(password),
        SshAuthMethod::PrivateKey {
            key_path,
            passphrase,
        } => SshAuth::PrivateKey {
            key_path,
            passphrase,
            certificate_path: None,
        },
    };

    Ok(SshConnectConfig {
        host: ssh_params.host,
        port: ssh_params.port,
        username: ssh_params.username,
        auth,
        timeout: ssh_params.connect_timeout.map(Duration::from_secs),
        keepalive_interval: ssh_params.keepalive_interval.map(Duration::from_secs),
        keepalive_max: ssh_params.keepalive_max,
        jump_server: ssh_params.jump_server.map(|jump| {
            let jump_auth = match jump.auth_method {
                SshAuthMethod::Password { password } => SshAuth::Password(password),
                SshAuthMethod::PrivateKey {
                    key_path,
                    passphrase,
                } => SshAuth::PrivateKey {
                    key_path,
                    passphrase,
                    certificate_path: None,
                },
            };
            JumpServerConnectConfig {
                host: jump.host,
                port: jump.port,
                username: jump.username,
                auth: jump_auth,
            }
        }),
        proxy: ssh_params.proxy.map(|p| {
            let proxy_type = match p.proxy_type {
                StorageProxyType::Socks5 => ProxyType::Socks5,
                StorageProxyType::Http => ProxyType::Http,
            };
            ProxyConnectConfig {
                proxy_type,
                host: p.host,
                port: p.port,
                username: p.username,
                password: p.password,
            }
        }),
    })
}

/// 终端侧边栏文件管理器面板
pub struct FileManagerPanel {
    /// 存储的连接信息
    stored_connection: StoredConnection,
    /// SFTP 客户端
    sftp_client: Option<Arc<Mutex<RusshSftpClient>>>,
    /// 连接状态
    connection_state: ConnectionState,
    /// 当前远程路径
    current_path: String,
    /// 文件列表
    items: Vec<RemoteFileItem>,
    /// 过滤后的索引
    filtered_indices: Vec<usize>,
    /// 选中项索引（基于 filtered_indices 的下标）
    selected_indices: HashSet<usize>,
    /// 排序列
    sort_column: SortColumn,
    /// 排序方向
    sort_order: SortOrder,
    /// 是否显示隐藏文件
    show_hidden: bool,
    /// 搜索输入框
    search_input: Entity<InputState>,
    /// 搜索关键词
    search_query: String,
    /// 导航历史
    history: Vec<String>,
    /// 当前历史位置
    history_index: usize,
    /// 滚动句柄
    scroll_handle: UniformListScrollHandle,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 是否正在加载目录
    loading: bool,
    /// 订阅
    _subscriptions: Vec<gpui::Subscription>,
}

impl FileManagerPanel {
    pub fn new(
        stored_connection: StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("FileManager.search_placeholder"))
        });

        let sub = cx.subscribe(&search_input, |this, input, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let text = input.read(cx).text().to_string();
                this.search_query = text;
                this.apply_filter();
                this.selected_indices.clear();
                cx.notify();
            }
        });

        Self {
            stored_connection,
            sftp_client: None,
            connection_state: ConnectionState::Idle,
            current_path: "/".to_string(),
            items: Vec::new(),
            filtered_indices: Vec::new(),
            selected_indices: HashSet::new(),
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Ascending,
            show_hidden: false,
            search_input,
            search_query: String::new(),
            history: vec!["/".to_string()],
            history_index: 0,
            scroll_handle: UniformListScrollHandle::new(),
            focus_handle,
            loading: false,
            _subscriptions: vec![sub],
        }
    }

    /// 建立 SFTP 连接
    pub fn connect(&mut self, cx: &mut Context<Self>) {
        if self.connection_state == ConnectionState::Connecting {
            return;
        }

        self.connection_state = ConnectionState::Connecting;
        cx.notify();

        let config = match build_ssh_config(&self.stored_connection) {
            Ok(config) => config,
            Err(e) => {
                self.connection_state =
                    ConnectionState::Error(format!("{}: {}", t!("FileManager.connect_failed"), e));
                cx.notify();
                return;
            }
        };

        let task = Tokio::spawn(cx, async move {
            let mut client = RusshSftpClient::connect(config).await?;
            let real_path = client
                .realpath(".")
                .await
                .unwrap_or_else(|_| "/".to_string());
            Ok::<_, anyhow::Error>((client, real_path))
        });

        cx.spawn(async move |this, cx| match task.await {
            Ok(Ok((client, real_path))) => {
                let _ = this.update(cx, |this, cx| {
                    this.sftp_client = Some(Arc::new(Mutex::new(client)));
                    this.connection_state = ConnectionState::Connected;
                    this.current_path = real_path.clone();
                    this.history = vec![real_path];
                    this.history_index = 0;
                    this.refresh_dir(cx);
                });
            }
            Ok(Err(e)) => {
                let _ = this.update(cx, |this, cx| {
                    this.connection_state = ConnectionState::Error(format!(
                        "{}: {}",
                        t!("FileManager.connect_failed"),
                        e
                    ));
                    cx.notify();
                });
            }
            Err(e) => {
                let _ = this.update(cx, |this, cx| {
                    this.connection_state = ConnectionState::Error(format!(
                        "{}: {}",
                        t!("FileManager.connect_failed"),
                        e
                    ));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// 仅在 Idle 状态时自动连接（用于面板首次激活）
    pub fn connect_if_idle(&mut self, cx: &mut Context<Self>) {
        if self.connection_state == ConnectionState::Idle {
            self.connect(cx);
        }
    }

    /// 刷新当前目录
    fn refresh_dir(&mut self, cx: &mut Context<Self>) {
        let Some(client) = self.sftp_client.clone() else {
            return;
        };

        self.loading = true;
        cx.notify();

        let path = self.current_path.clone();
        let task = Tokio::spawn(cx, async move {
            let mut client: tokio::sync::MutexGuard<'_, RusshSftpClient> = client.lock().await;
            client.list_dir(&path).await
        });

        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(Ok(entries)) => {
                        this.items = entries
                            .into_iter()
                            .filter(|e| e.name != "." && e.name != "..")
                            .map(|e| RemoteFileItem {
                                name: e.name,
                                size: e.size,
                                modified: e.modified,
                                is_dir: e.is_dir,
                            })
                            .collect();
                        this.sort_items();
                        this.apply_filter();
                        this.selected_indices.clear();
                    }
                    Ok(Err(e)) => {
                        tracing::error!("列出目录失败: {}", e);
                        this.items.clear();
                        this.filtered_indices.clear();
                    }
                    Err(e) => {
                        tracing::error!("SFTP 任务失败: {}", e);
                        this.items.clear();
                        this.filtered_indices.clear();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 导航到指定路径
    fn navigate_to(&mut self, path: String, cx: &mut Context<Self>) {
        if path == self.current_path {
            self.refresh_dir(cx);
            return;
        }

        self.current_path = path.clone();

        // 截断当前位置之后的历史记录，再追加新路径
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(path);
        self.history_index = self.history.len() - 1;

        self.scroll_handle = UniformListScrollHandle::new();
        self.refresh_dir(cx);
    }

    /// 后退
    fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.history[self.history_index].clone();
            self.scroll_handle = UniformListScrollHandle::new();
            self.refresh_dir(cx);
        }
    }

    /// 导航到 Home（SFTP realpath "." 返回的初始路径）
    fn go_home(&mut self, cx: &mut Context<Self>) {
        let home = self.history.first().cloned().unwrap_or("/".to_string());
        self.navigate_to(home, cx);
    }

    /// 导航到上层目录
    fn go_parent(&mut self, cx: &mut Context<Self>) {
        let parent = if self.current_path == "/" {
            "/".to_string()
        } else {
            let path = self.current_path.trim_end_matches('/');
            match path.rfind('/') {
                Some(0) => "/".to_string(),
                Some(pos) => path[..pos].to_string(),
                None => "/".to_string(),
            }
        };
        self.navigate_to(parent, cx);
    }

    /// 是否在根目录
    fn is_at_root(&self) -> bool {
        self.current_path == "/" || self.current_path.is_empty()
    }

    /// 排序文件列表
    fn sort_items(&mut self) {
        let sort_column = self.sort_column;
        let sort_order = self.sort_order;

        self.items.sort_by(|a, b| {
            // 文件夹始终排在前面
            if a.is_dir != b.is_dir {
                return if a.is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            let cmp = match sort_column {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Size => a.size.cmp(&b.size),
                SortColumn::Modified => a.modified.cmp(&b.modified),
            };

            match sort_order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });
    }

    /// 设置排序
    fn set_sort(&mut self, column: SortColumn, cx: &mut Context<Self>) {
        if self.sort_column == column {
            self.sort_order = match self.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            self.sort_column = column;
            self.sort_order = SortOrder::Ascending;
        }
        self.sort_items();
        self.apply_filter();
        self.selected_indices.clear();
        cx.notify();
    }

    /// 应用过滤
    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        let show_hidden = self.show_hidden;

        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if !show_hidden && item.name.starts_with('.') {
                    return false;
                }
                if query.is_empty() {
                    true
                } else {
                    item.name.to_lowercase().contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();
    }

    /// 切换选中状态
    fn toggle_selection(&mut self, row_ix: usize, multi_select: bool) {
        if multi_select {
            if self.selected_indices.contains(&row_ix) {
                self.selected_indices.remove(&row_ix);
            } else {
                self.selected_indices.insert(row_ix);
            }
        } else if !self.selected_indices.contains(&row_ix) {
            self.selected_indices.clear();
            self.selected_indices.insert(row_ix);
        }
    }

    /// 渲染工具栏
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_go_back = self.history_index > 0;

        h_flex()
            .h_8()
            .px_2()
            .gap_1()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar)
            // 后退按钮
            .child(
                div()
                    .id("fm-back")
                    .cursor_pointer()
                    .rounded_md()
                    .p(px(4.))
                    .when(!can_go_back, |el| el.opacity(0.4))
                    .when(can_go_back, |el| el.hover(|s| s.bg(cx.theme().list_active)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.go_back(cx);
                        }),
                    )
                    .child(
                        Icon::new(IconName::ArrowLeft)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            // Home 按钮
            .child(
                div()
                    .id("fm-home")
                    .cursor_pointer()
                    .rounded_md()
                    .p(px(4.))
                    .hover(|s| s.bg(cx.theme().list_active))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.go_home(cx);
                        }),
                    )
                    .child(
                        Icon::new(IconName::Home)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            // 上级目录按钮
            .child(
                div()
                    .id("fm-parent")
                    .cursor_pointer()
                    .rounded_md()
                    .p(px(4.))
                    .when(self.is_at_root(), |el| el.opacity(0.4))
                    .when(!self.is_at_root(), |el| {
                        el.hover(|s| s.bg(cx.theme().list_active))
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.go_parent(cx);
                        }),
                    )
                    .child(
                        Icon::new(IconName::ArrowUp)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            // 当前路径（flex_1 填充剩余空间）
            .child(
                div()
                    .id("fm-path")
                    .flex_1()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_xs()
                    .whitespace_nowrap()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.current_path.clone())
                    .tooltip(move |window, cx| {
                        Tooltip::new(t!("FileManager.current_path").to_string()).build(window, cx)
                    }),
            )
            // 刷新按钮
            .child(
                div()
                    .id("fm-refresh")
                    .cursor_pointer()
                    .rounded_md()
                    .p(px(4.))
                    .hover(|s| s.bg(cx.theme().list_active))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.refresh_dir(cx);
                        }),
                    )
                    .child(
                        Icon::new(IconName::Refresh)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            // 隐藏文件开关
            .child(
                div()
                    .id("fm-hidden")
                    .cursor_pointer()
                    .rounded_md()
                    .p(px(4.))
                    .hover(|s| s.bg(cx.theme().list_active))
                    .when(self.show_hidden, |el| el.bg(cx.theme().list_active))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.show_hidden = !this.show_hidden;
                            this.apply_filter();
                            this.selected_indices.clear();
                            cx.notify();
                        }),
                    )
                    .child(
                        Icon::new(IconName::Eye)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            // 关闭按钮
            .child(
                div()
                    .id("fm-close")
                    .cursor_pointer()
                    .rounded_md()
                    .p(px(4.))
                    .hover(|s| s.bg(cx.theme().list_active))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_this, _, _window, cx| {
                            cx.emit(FileManagerPanelEvent::Close);
                        }),
                    )
                    .child(
                        Icon::new(IconName::Close)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
    }

    /// 渲染搜索栏
    fn render_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_query = !self.search_query.is_empty();
        let filtered_count = self.filtered_indices.len();
        let total_count = self.items.len();

        h_flex()
            .h_8()
            .px_2()
            .gap_2()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                Icon::new(IconName::Search)
                    .xsmall()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div().flex_1().child(
                    Input::new(&self.search_input)
                        .xsmall()
                        .appearance(false)
                        .cleanable(has_query),
                ),
            )
            .when(has_query, |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{}/{}", filtered_count, total_count)),
                )
            })
    }

    /// 渲染排序表头
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h_7()
            .px_2()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar)
            .child(self.render_header_cell(&t!("FileManager.name"), SortColumn::Name, true, cx))
            .child(self.render_header_cell(&t!("FileManager.size"), SortColumn::Size, false, cx))
            .child(self.render_header_cell(
                &t!("FileManager.time"),
                SortColumn::Modified,
                false,
                cx,
            ))
    }

    /// 渲染单个表头列
    fn render_header_cell(
        &self,
        label: &str,
        column: SortColumn,
        is_flex: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_sorted = self.sort_column == column;
        let sort_order = self.sort_order;
        let label = label.to_string();

        let base = h_flex()
            .h_full()
            .px_1()
            .items_center()
            .gap_0p5()
            .cursor_pointer()
            .hover(|s| s.bg(cx.theme().list_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _window, cx| {
                    this.set_sort(column, cx);
                }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .when(is_sorted, |el| {
                el.child(
                    Icon::new(if sort_order == SortOrder::Ascending {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .xsmall()
                    .text_color(cx.theme().muted_foreground),
                )
            });

        if is_flex {
            base.flex_1()
        } else {
            base.w(px(70.))
        }
    }

    /// 渲染单行文件项
    fn render_file_row(
        &self,
        item: &RemoteFileItem,
        is_selected: bool,
        cx: &App,
    ) -> impl IntoElement {
        let name = item.name.clone();
        let is_dir = item.is_dir;

        h_flex()
            .h(px(36.))
            .px_2()
            .items_center()
            .when(is_selected, |el| el.bg(cx.theme().selection))
            // 名称列
            .child(
                h_flex()
                    .flex_1()
                    .gap_1()
                    .items_center()
                    .overflow_hidden()
                    .child(
                        Icon::new(if is_dir {
                            IconName::Folder1
                        } else {
                            IconName::File
                        })
                        .with_size(Size::Small)
                        .color(),
                    )
                    .child({
                        let tooltip_name = name.clone();
                        div()
                            .id(SharedString::from(name.clone()))
                            .flex_1()
                            .text_sm()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(name)
                            .tooltip(move |window, cx| {
                                Tooltip::new(tooltip_name.clone()).build(window, cx)
                            })
                    }),
            )
            // 大小列
            .child(
                div()
                    .w(px(50.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(if is_dir {
                        "-".to_string()
                    } else {
                        format_file_size(item.size)
                    }),
            )
            // 时间列
            .child(
                div()
                    .w(px(70.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(format_modified_time(item.modified)),
            )
    }

    /// 渲染上级目录行（..）
    fn render_parent_row(&self, _cx: &App) -> impl IntoElement {
        h_flex()
            .h(px(36.))
            .px_2()
            .items_center()
            .child(
                h_flex()
                    .flex_1()
                    .gap_1()
                    .items_center()
                    .child(Icon::new(IconName::Folder1).with_size(Size::Small).color())
                    .child(div().text_sm().child("..")),
            )
            .child(div().w(px(50.)))
            .child(div().w(px(70.)))
    }

    /// 构建文件项右键菜单
    fn build_context_menu(
        menu: PopupMenu,
        name: &str,
        full_path: &str,
        is_dir: bool,
        view: &Entity<Self>,
        window: &mut Window,
        _cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let path_for_cd = full_path.to_string();
        let path_for_copy = full_path.to_string();
        let name_for_copy = name.to_string();

        let mut menu = menu;

        // 文件夹：在终端中 CD
        if is_dir {
            let view_cd = view.clone();
            menu = menu.item(
                PopupMenuItem::new(t!("FileManager.cd_to_terminal"))
                    .icon(IconName::SquareTerminal)
                    .on_click(window.listener_for(&view_cd, move |_this, _, _, cx| {
                        cx.emit(FileManagerPanelEvent::CdToTerminal(path_for_cd.clone()));
                    })),
            );
        }

        // 复制路径
        let view_copy_path = view.clone();
        menu = menu.item(
            PopupMenuItem::new(t!("FileManager.copy_path"))
                .icon(IconName::Copy)
                .on_click(
                    window.listener_for(&view_copy_path, move |_this, _, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(path_for_copy.clone()));
                    }),
                ),
        );

        // 复制名称
        let view_copy_name = view.clone();
        menu = menu.item(
            PopupMenuItem::new(t!("FileManager.copy_name"))
                .icon(IconName::Copy)
                .on_click(
                    window.listener_for(&view_copy_name, move |_this, _, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(name_for_copy.clone()));
                    }),
                ),
        );

        // 分隔线 + 刷新
        let view_refresh = view.clone();
        menu = menu.separator().item(
            PopupMenuItem::new(t!("FileManager.refresh"))
                .icon(IconName::Refresh)
                .on_click(window.listener_for(&view_refresh, move |this, _, _, cx| {
                    this.refresh_dir(cx);
                })),
        );

        menu
    }

    /// 渲染连接中状态
    fn render_connecting(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(Spinner::new().small())
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("FileManager.connecting")),
            )
    }

    /// 渲染错误状态
    fn render_error(&self, error: &str, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_4()
            .child(
                Icon::new(IconName::CircleX)
                    .color()
                    .with_size(Size::Large)
                    .text_color(cx.theme().danger),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .text_center()
                    .max_w(px(200.))
                    .child(error.to_string()),
            )
            .child(
                div()
                    .id("fm-retry")
                    .cursor_pointer()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
                    .text_sm()
                    .hover(|s| s.opacity(0.9))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.connect(cx);
                        }),
                    )
                    .child(t!("FileManager.retry")),
            )
    }

    /// 渲染初始状态（提示连接）
    fn render_idle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_4()
            .child(
                Icon::new(IconName::FolderOpen)
                    .color()
                    .with_size(Size::Large)
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("FileManager.title")),
            )
            .child(
                div()
                    .id("fm-connect")
                    .cursor_pointer()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
                    .text_sm()
                    .hover(|s| s.opacity(0.9))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.connect(cx);
                        }),
                    )
                    .child(t!("FileManager.connect")),
            )
    }

    /// 渲染已连接的文件列表
    fn render_file_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let filtered_count = self.filtered_indices.len();
        let show_parent = !self.is_at_root();
        let total_count = if show_parent {
            filtered_count + 1
        } else {
            filtered_count
        };
        let scroll_handle = self.scroll_handle.clone();
        let is_loading = self.loading;

        v_flex()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(self.render_search_bar(cx))
            .child(self.render_header(cx))
            .when(is_loading, |el| {
                el.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Spinner::new().small()),
                )
            })
            .when(!is_loading, |el| {
                el.child(
                    uniform_list("fm-file-list", total_count, {
                        cx.processor(move |state: &mut Self, range: Range<usize>, _window, cx| {
                            let current_path = state.current_path.clone();
                            let has_parent = !state.is_at_root();
                            let view = cx.entity();

                            range
                                .map(|list_ix| {
                                    // 上级目录行
                                    if has_parent && list_ix == 0 {
                                        return div()
                                            .id(list_ix)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(cx.theme().list_hover))
                                            .on_double_click(cx.listener(
                                                move |this, _, _window, cx| {
                                                    this.go_parent(cx);
                                                },
                                            ))
                                            .child(state.render_parent_row(cx))
                                            .into_any_element();
                                    }

                                    let filtered_ix =
                                        if has_parent { list_ix - 1 } else { list_ix };
                                    let real_ix = state.filtered_indices[filtered_ix];
                                    let item = &state.items[real_ix];
                                    let is_selected = state.selected_indices.contains(&filtered_ix);
                                    let item_name = item.name.clone();
                                    let is_dir = item.is_dir;
                                    let full_path = if current_path.ends_with('/') {
                                        format!("{}{}", current_path, item_name)
                                    } else {
                                        format!("{}/{}", current_path, item_name)
                                    };

                                    // 右键菜单变量
                                    let ctx_name = item_name.clone();
                                    let ctx_full_path = full_path.clone();
                                    let ctx_is_dir = is_dir;
                                    let ctx_view = view.clone();

                                    div()
                                        .id(list_ix)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(cx.theme().list_hover))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                move |this, event: &MouseDownEvent, _window, cx| {
                                                    let multi_select = event.modifiers.secondary();
                                                    this.toggle_selection(
                                                        filtered_ix,
                                                        multi_select,
                                                    );
                                                    cx.notify();
                                                },
                                            ),
                                        )
                                        .on_double_click(cx.listener({
                                            let name = item_name.clone();
                                            let fp = full_path.clone();
                                            move |this, _, _window, cx| {
                                                if is_dir {
                                                    this.navigate_to(fp.clone(), cx);
                                                } else {
                                                    // 文件双击：复制路径到剪贴板
                                                    cx.write_to_clipboard(
                                                        ClipboardItem::new_string(name.clone()),
                                                    );
                                                }
                                            }
                                        }))
                                        .context_menu(move |menu, window, cx| {
                                            Self::build_context_menu(
                                                menu,
                                                &ctx_name,
                                                &ctx_full_path,
                                                ctx_is_dir,
                                                &ctx_view,
                                                window,
                                                cx,
                                            )
                                        })
                                        .child(state.render_file_row(item, is_selected, cx))
                                        .into_any_element()
                                })
                                .collect()
                        })
                    })
                    .flex_1()
                    .size_full()
                    .track_scroll(&scroll_handle)
                    .with_sizing_behavior(ListSizingBehavior::Auto),
                )
            })
    }
}

impl EventEmitter<FileManagerPanelEvent> for FileManagerPanel {}

impl Focusable for FileManagerPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileManagerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.connection_state.clone();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(match state {
                ConnectionState::Idle => self.render_idle(cx).into_any_element(),
                ConnectionState::Connecting => self.render_connecting(cx).into_any_element(),
                ConnectionState::Connected => self.render_file_list(cx).into_any_element(),
                ConnectionState::Error(ref msg) => self.render_error(msg, cx).into_any_element(),
            })
    }
}
