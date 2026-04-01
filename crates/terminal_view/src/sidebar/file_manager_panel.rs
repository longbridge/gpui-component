//! 终端侧边栏文件管理器面板
//!
//! 仅针对 SSH 终端，通过独立的 SFTP 连接浏览远程文件系统。
//! UI 参考 `sftp_view` 的 `FileListPanel`，但为侧边栏场景做了精简和适配。
//! 支持文件传输（上传/下载/拖拽），使用独立的传输连接避免阻塞浏览。

use chrono::{DateTime, Local};
use gpui::{
    div, prelude::*, px, uniform_list, App, ClipboardItem, Context, Entity, EventEmitter,
    ExternalPaths, FocusHandle, Focusable, IntoElement, ListSizingBehavior, MouseButton,
    MouseDownEvent, ParentElement, PathPromptOptions, Render, SharedString, Styled,
    UniformListScrollHandle, Window,
};
use gpui_component::{
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    button::{Button, ButtonVariants},
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    notification::Notification,
    progress::Progress,
    spinner::Spinner,
    tooltip::Tooltip,
    v_flex, ActiveTheme, Icon, IconName, InteractiveElementExt, Sizable, Size, WindowExt,
};
use one_core::gpui_tokio::Tokio;
use one_core::storage::models::{ProxyType as StorageProxyType, SshAuthMethod, StoredConnection};
use rust_i18n::t;
use sftp::{RusshSftpClient, SftpClient, TransferCancelled, TransferProgress};
use ssh::{JumpServerConnectConfig, ProxyConnectConfig, ProxyType, SshAuth, SshConnectConfig};
use std::collections::{HashSet, VecDeque};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

// ── 传输相关类型 ──────────────────────────────────────────────

/// 传输操作类型
#[derive(Clone)]
enum TransferOperation {
    Upload {
        local_path: PathBuf,
        remote_path: String,
        is_dir: bool,
    },
    Download {
        remote_path: String,
        local_path: PathBuf,
        is_dir: bool,
    },
}

/// 传输任务状态
#[derive(Clone, PartialEq)]
enum TransferTaskState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 跨线程共享的进度数据（原子操作，无需加锁）
struct SharedProgress {
    transferred: AtomicU64,
    total: AtomicU64,
    /// 存储 f64::to_bits() 的速度值
    speed: AtomicU64,
    cancelled: Arc<AtomicBool>,
    current_file: std::sync::RwLock<Option<String>>,
}

impl SharedProgress {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            transferred: AtomicU64::new(0),
            total: AtomicU64::new(0),
            speed: AtomicU64::new(0),
            cancelled: Arc::new(AtomicBool::new(false)),
            current_file: std::sync::RwLock::new(None),
        })
    }
}

/// 传输任务
#[derive(Clone)]
struct TransferTask {
    id: usize,
    operation: TransferOperation,
    state: TransferTaskState,
    shared_progress: Arc<SharedProgress>,
    error: Option<String>,
}

#[derive(Clone)]
struct PendingUpload {
    name: String,
    local_path: PathBuf,
    remote_path: String,
    is_dir: bool,
    has_conflict: bool,
}

/// 传输队列（单任务串行执行）
struct TransferQueue {
    tasks: Vec<TransferTask>,
    pending: VecDeque<usize>,
}

impl TransferQueue {
    fn new() -> Self {
        Self {
            tasks: Vec::new(),
            pending: VecDeque::new(),
        }
    }

    fn has_active(&self) -> bool {
        self.tasks.iter().any(|task| {
            task.state == TransferTaskState::Running || task.state == TransferTaskState::Pending
        })
    }

    fn running_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.state == TransferTaskState::Running)
            .count()
    }

    fn enqueue(&mut self, task: TransferTask) {
        self.pending.push_back(task.id);
        self.tasks.push(task);
    }

    /// 取出下一个可执行的任务（串行：仅当没有 Running 时才启动）
    fn next_startable(&mut self) -> Option<TransferTask> {
        if self.running_count() > 0 {
            return None;
        }

        while let Some(task_id) = self.pending.pop_front() {
            let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) else {
                continue;
            };

            if task.state != TransferTaskState::Pending {
                continue;
            }

            task.state = TransferTaskState::Running;
            return Some(task.clone());
        }

        None
    }

    /// 获取当前活跃任务（用于进度显示）
    fn active_task(&self) -> Option<&TransferTask> {
        self.tasks
            .iter()
            .find(|task| task.state == TransferTaskState::Running)
            .or_else(|| {
                self.tasks
                    .iter()
                    .find(|task| task.state == TransferTaskState::Pending)
            })
    }

    /// 排队中的任务数（不含正在执行的）
    fn pending_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == TransferTaskState::Pending)
            .count()
    }
}

// ── 基础类型 ──────────────────────────────────────────────────

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetryResetPlan {
    next_state: ConnectionState,
    initial_working_dir: Option<String>,
    clear_listing: bool,
}

fn build_retry_reset_plan(current_path: &str, working_dir: Option<String>) -> RetryResetPlan {
    RetryResetPlan {
        next_state: ConnectionState::Idle,
        initial_working_dir: working_dir.or_else(|| Some(current_path.to_string())),
        clear_listing: true,
    }
}

fn build_refresh_error_plan(current_path: &str, message: String) -> RetryResetPlan {
    RetryResetPlan {
        next_state: ConnectionState::Error(message),
        initial_working_dir: Some(current_path.to_string()),
        clear_listing: true,
    }
}

fn clear_remote_listing_state<T>(
    items: &mut Vec<T>,
    filtered_indices: &mut Vec<usize>,
    selected_indices: &mut HashSet<usize>,
) {
    items.clear();
    filtered_indices.clear();
    selected_indices.clear();
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
    /// 请求将终端当前工作目录同步到文件管理器
    SyncWorkingDir,
}

// ── 工具函数 ──────────────────────────────────────────────────

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

/// 格式化传输速度
fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

/// 拼接远程路径
fn join_remote_path(base: &str, name: &str) -> String {
    if base == "/" {
        format!("/{}", name)
    } else {
        format!("{}/{}", base, name)
    }
}

fn is_valid_entry_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

fn breadcrumb_item(label: impl Into<SharedString>) -> BreadcrumbItem {
    const BREADCRUMB_ITEM_MAX_WIDTH: f32 = 180.;

    BreadcrumbItem::new(label)
        .flex_shrink()
        .min_w(px(0.))
        .max_w(px(BREADCRUMB_ITEM_MAX_WIDTH))
        .overflow_hidden()
        .text_ellipsis()
}

/// 判断传输错误是否为取消
fn is_transfer_cancelled(error: &anyhow::Error) -> bool {
    error.downcast_ref::<TransferCancelled>().is_some()
}

fn generate_unique_name(original_name: &str, existing_names: &HashSet<String>) -> String {
    let (stem, ext) = if let Some(dot_pos) = original_name.rfind('.') {
        if dot_pos > 0 {
            (
                original_name[..dot_pos].to_string(),
                Some(original_name[dot_pos..].to_string()),
            )
        } else {
            (original_name.to_string(), None)
        }
    } else {
        (original_name.to_string(), None)
    };

    let mut counter = 1;
    loop {
        let new_name = if counter == 1 {
            if let Some(ref ext) = ext {
                format!("{} (copy){}", stem, ext)
            } else {
                format!("{} (copy)", stem)
            }
        } else if let Some(ref ext) = ext {
            format!("{} (copy {}){}", stem, counter, ext)
        } else {
            format!("{} (copy {})", stem, counter)
        };

        if !existing_names.contains(&new_name) {
            return new_name;
        }
        counter += 1;
    }
}

fn rename_conflicting_uploads(
    mut uploads: Vec<PendingUpload>,
    existing_names: HashSet<String>,
) -> Vec<PendingUpload> {
    let mut used_names = existing_names;

    for upload in &mut uploads {
        if upload.has_conflict {
            let new_name = generate_unique_name(&upload.name, &used_names);
            used_names.insert(new_name.clone());

            let dir_part = if let Some(slash_pos) = upload.remote_path.rfind('/') {
                Some(upload.remote_path[..=slash_pos].to_string())
            } else {
                None
            };

            upload.remote_path = if let Some(dir) = dir_part {
                format!("{}{}", dir, new_name)
            } else {
                new_name.clone()
            };
            upload.name = new_name;
        }
    }

    uploads
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
        SshAuthMethod::Agent => SshAuth::Agent,
        SshAuthMethod::AutoPublicKey => SshAuth::AutoPublicKey,
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
                SshAuthMethod::Agent => SshAuth::Agent,
                SshAuthMethod::AutoPublicKey => SshAuth::AutoPublicKey,
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

// ── FileManagerPanel ──────────────────────────────────────────

/// 终端侧边栏文件管理器面板
pub struct FileManagerPanel {
    /// 存储的连接信息
    stored_connection: StoredConnection,
    /// SFTP 客户端（浏览用）
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
    /// 路径输入框
    path_input: Entity<InputState>,
    /// 搜索关键词
    search_query: String,
    /// 是否正在编辑路径
    path_editing: bool,
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

    // ── 传输相关字段 ──
    /// 独立的传输 SFTP 连接（懒创建）
    transfer_client: Option<Arc<Mutex<RusshSftpClient>>>,
    /// 传输队列
    transfer_queue: TransferQueue,
    /// 下一个任务 ID
    next_task_id: usize,
    /// 进度刷新定时器
    progress_refresh_task: Option<gpui::Task<()>>,
    /// 是否有外部文件拖入
    is_dragging_over: bool,
    /// 终端当前工作目录（连接前由外部设置，连接时作为初始路径）
    initial_working_dir: Option<String>,
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
        let path_input = cx
            .new(|cx| InputState::new(window, cx).placeholder(t!("FileManager.path_placeholder")));

        let mut subscriptions = Vec::new();
        subscriptions.push(
            cx.subscribe(&search_input, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = input.read(cx).text().to_string();
                    this.search_query = text;
                    this.apply_filter();
                    this.selected_indices.clear();
                    cx.notify();
                }
            }),
        );
        subscriptions.push(cx.subscribe_in(
            &path_input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => {
                    this.confirm_path(window, cx);
                }
                InputEvent::Blur => {
                    this.cancel_path_editing(cx);
                }
                _ => {}
            },
        ));

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
            path_input,
            search_query: String::new(),
            path_editing: false,
            history: vec!["/".to_string()],
            history_index: 0,
            scroll_handle: UniformListScrollHandle::new(),
            focus_handle,
            loading: false,
            _subscriptions: subscriptions,
            transfer_client: None,
            transfer_queue: TransferQueue::new(),
            next_task_id: 0,
            progress_refresh_task: None,
            is_dragging_over: false,
            initial_working_dir: None,
        }
    }

    // ── 连接管理 ──────────────────────────────────────────────

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

        let initial_dir = self.initial_working_dir.take();
        let task = Tokio::spawn(cx, async move {
            let mut client = RusshSftpClient::connect(config).await?;
            // 优先使用终端当前工作目录，否则回退到 realpath(".")
            let real_path = if let Some(dir) = initial_dir {
                dir
            } else {
                client
                    .realpath(".")
                    .await
                    .unwrap_or_else(|_| "/".to_string())
            };
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

    fn apply_retry_reset_plan(&mut self, plan: RetryResetPlan) {
        self.connection_state = plan.next_state;
        self.initial_working_dir = plan.initial_working_dir;
        self.sftp_client = None;
        self.transfer_client = None;
        self.loading = false;

        if plan.clear_listing {
            clear_remote_listing_state(
                &mut self.items,
                &mut self.filtered_indices,
                &mut self.selected_indices,
            );
        }
    }

    fn reset_connection_for_retry(&mut self, working_dir: Option<String>) {
        let plan = build_retry_reset_plan(&self.current_path, working_dir);
        self.apply_retry_reset_plan(plan);
    }

    fn handle_refresh_error(&mut self, message: String) {
        let plan = build_refresh_error_plan(&self.current_path, message);
        self.apply_retry_reset_plan(plan);
    }

    pub fn reconnect_with_working_dir(
        &mut self,
        working_dir: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let should_reconnect = self.connection_state != ConnectionState::Idle
            || self.sftp_client.is_some()
            || !self.items.is_empty();
        if !should_reconnect {
            return;
        }

        self.reset_connection_for_retry(working_dir);
        self.connect(cx);
    }

    /// 设置初始工作目录（连接前由终端 OSC 7 提供）
    ///
    /// 仅在尚未连接时有效，连接后应使用 `sync_navigate_to`。
    pub fn set_initial_working_dir(&mut self, path: String) {
        if self.connection_state == ConnectionState::Idle {
            self.initial_working_dir = Some(path);
        }
    }

    /// 从终端 OSC 7 同步导航到指定路径
    ///
    /// 仅在已连接且路径不同时才导航，避免不必要的刷新。
    pub fn sync_navigate_to(&mut self, path: String, cx: &mut Context<Self>) {
        if self.connection_state != ConnectionState::Connected {
            return;
        }
        if path == self.current_path {
            return;
        }
        self.navigate_to(path, cx);
    }

    fn start_path_editing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.path_editing = true;
        let path = self.current_path.clone();
        self.path_input.update(cx, |state, cx| {
            state.set_value(&path, window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    fn confirm_path(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let new_path = self.path_input.read(cx).text().to_string();
        let new_path = new_path.trim().to_string();
        self.path_editing = false;

        if !new_path.is_empty() && new_path != self.current_path {
            self.navigate_to(new_path, cx);
        } else {
            cx.notify();
        }
    }

    fn cancel_path_editing(&mut self, cx: &mut Context<Self>) {
        if self.path_editing {
            self.path_editing = false;
            cx.notify();
        }
    }

    fn render_path_breadcrumb(&self, cx: &mut Context<Self>) -> Breadcrumb {
        let mut breadcrumb = Breadcrumb::new();
        const MAX_VISIBLE: usize = 4;

        if self.current_path == "." {
            return breadcrumb.child(breadcrumb_item("."));
        }

        let parts: Vec<&str> = self
            .current_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let starts_with_slash = self.current_path.starts_with('/');
        let total = parts.len() + if starts_with_slash { 1 } else { 0 };

        if total <= MAX_VISIBLE {
            if starts_with_slash {
                breadcrumb = breadcrumb.child(breadcrumb_item("/").on_click(cx.listener(
                    |this, _, _window, cx| {
                        cx.stop_propagation();
                        this.navigate_to("/".to_string(), cx);
                    },
                )));
            }

            for (idx, part) in parts.iter().enumerate() {
                let path_so_far = if starts_with_slash {
                    format!("/{}", parts[..=idx].join("/"))
                } else {
                    parts[..=idx].join("/")
                };

                breadcrumb = breadcrumb.child(breadcrumb_item(part.to_string()).on_click(
                    cx.listener(move |this, _, _window, cx| {
                        cx.stop_propagation();
                        this.navigate_to(path_so_far.clone(), cx);
                    }),
                ));
            }
        } else {
            if starts_with_slash {
                breadcrumb = breadcrumb.child(breadcrumb_item("/").on_click(cx.listener(
                    |this, _, _window, cx| {
                        cx.stop_propagation();
                        this.navigate_to("/".to_string(), cx);
                    },
                )));
            }

            breadcrumb = breadcrumb.child(breadcrumb_item("...").disabled(true));

            let visible_count = MAX_VISIBLE - 2;
            let visible_start = parts.len().saturating_sub(visible_count);
            for idx in visible_start..parts.len() {
                let path_so_far = if starts_with_slash {
                    format!("/{}", parts[..=idx].join("/"))
                } else {
                    parts[..=idx].join("/")
                };

                breadcrumb = breadcrumb.child(breadcrumb_item(parts[idx].to_string()).on_click(
                    cx.listener(move |this, _, _window, cx| {
                        cx.stop_propagation();
                        this.navigate_to(path_so_far.clone(), cx);
                    }),
                ));
            }
        }

        breadcrumb
    }

    // ── 目录浏览 ──────────────────────────────────────────────

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
                        this.handle_refresh_error(
                            t!("FileManager.read_dir_failed", error = e).to_string(),
                        );
                    }
                    Err(e) => {
                        tracing::error!("SFTP 任务失败: {}", e);
                        this.handle_refresh_error(
                            t!("FileManager.read_dir_failed", error = e).to_string(),
                        );
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

    // ── 排序和过滤 ───────────────────────────────────────────

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

    // ── 传输调度 ──────────────────────────────────────────────

    /// 分配下一个任务 ID
    fn alloc_task_id(&mut self) -> usize {
        let id = self.next_task_id;
        self.next_task_id += 1;
        id
    }

    /// 创建传输专用连接（首次传输时懒创建），然后执行排队任务
    fn ensure_transfer_client_and_schedule(&mut self, cx: &mut Context<Self>) {
        if self.transfer_client.is_some() {
            self.schedule_transfers(cx);
            return;
        }

        let config = match build_ssh_config(&self.stored_connection) {
            Ok(config) => config,
            Err(e) => {
                tracing::error!("{}: {}", t!("FileManager.transfer_connect_failed"), e);
                // 将所有排队任务标记为失败
                let error_msg = format!("{}: {}", t!("FileManager.transfer_connect_failed"), e);
                for task in &mut self.transfer_queue.tasks {
                    if task.state == TransferTaskState::Pending {
                        task.state = TransferTaskState::Failed;
                        task.error = Some(error_msg.clone());
                    }
                }
                self.transfer_queue.pending.clear();
                cx.notify();
                return;
            }
        };

        let connect_task = Tokio::spawn(cx, async move {
            let client = RusshSftpClient::connect(config).await?;
            Ok::<_, anyhow::Error>(client)
        });

        cx.spawn(async move |this, cx| {
            let result = match connect_task.await {
                Ok(Ok(client)) => Ok(client),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(anyhow::Error::new(e)),
            };

            match result {
                Ok(client) => {
                    let _ = this.update(cx, |this, cx| {
                        this.transfer_client = Some(Arc::new(Mutex::new(client)));
                        this.schedule_transfers(cx);
                    });
                }
                Err(e) => {
                    let _ = this.update(cx, |this, cx| {
                        let error_msg =
                            format!("{}: {}", t!("FileManager.transfer_connect_failed"), e);
                        tracing::error!("{}", error_msg);
                        for task in &mut this.transfer_queue.tasks {
                            if task.state == TransferTaskState::Pending {
                                task.state = TransferTaskState::Failed;
                                task.error = Some(error_msg.clone());
                            }
                        }
                        this.transfer_queue.pending.clear();
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// 调度下一个待执行的传输任务
    fn schedule_transfers(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.transfer_queue.next_startable() else {
            return;
        };

        match task.operation.clone() {
            TransferOperation::Upload {
                local_path,
                remote_path,
                is_dir,
            } => {
                self.start_upload_task(
                    task.id,
                    local_path,
                    remote_path,
                    is_dir,
                    task.shared_progress,
                    cx,
                );
            }
            TransferOperation::Download {
                remote_path,
                local_path,
                is_dir,
            } => {
                self.start_download_task(
                    task.id,
                    remote_path,
                    local_path,
                    is_dir,
                    task.shared_progress,
                    cx,
                );
            }
        }

        self.start_progress_refresh(cx);
        cx.notify();
    }

    /// 执行上传任务
    fn start_upload_task(
        &mut self,
        task_id: usize,
        local_path: PathBuf,
        remote_path: String,
        is_dir: bool,
        shared_progress: Arc<SharedProgress>,
        cx: &mut Context<Self>,
    ) {
        let Some(client) = self.transfer_client.clone() else {
            return;
        };

        let cancelled = shared_progress.cancelled.clone();
        let progress_for_callback = shared_progress.clone();
        let current_remote_path = self.current_path.clone();
        let local_path_for_refresh = local_path.clone();
        let remote_path_for_refresh = remote_path.clone();

        let upload_task = Tokio::spawn(cx, async move {
            let mut client_guard = client.lock().await;
            if is_dir {
                client_guard
                    .upload_dir_with_progress(
                        local_path.to_string_lossy().as_ref(),
                        &remote_path,
                        cancelled,
                        Box::new(move |progress: TransferProgress| {
                            progress_for_callback
                                .transferred
                                .store(progress.transferred, Ordering::Relaxed);
                            progress_for_callback
                                .total
                                .store(progress.total, Ordering::Relaxed);
                            progress_for_callback
                                .speed
                                .store(progress.speed.to_bits(), Ordering::Relaxed);
                            if let Some(file) = progress.current_file {
                                if let Ok(mut guard) = progress_for_callback.current_file.write() {
                                    *guard = Some(file);
                                }
                            }
                        }),
                    )
                    .await
            } else {
                client_guard
                    .upload_with_progress(
                        local_path.to_string_lossy().as_ref(),
                        &remote_path,
                        cancelled,
                        Box::new(move |progress: TransferProgress| {
                            progress_for_callback
                                .transferred
                                .store(progress.transferred, Ordering::Relaxed);
                            progress_for_callback
                                .total
                                .store(progress.total, Ordering::Relaxed);
                            progress_for_callback
                                .speed
                                .store(progress.speed.to_bits(), Ordering::Relaxed);
                        }),
                    )
                    .await
            }
        });

        cx.spawn(async move |this, cx| {
            let result = match upload_task.await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(anyhow::Error::new(e)),
            };

            let should_refresh = result.is_ok();

            let _ = this.update(cx, |this, cx| {
                this.update_task_state(task_id, result);
                this.schedule_transfers(cx);

                if should_refresh {
                    // 上传完成后，如果当前目录与上传目标同级，刷新目录
                    let remote_parent = remote_path_parent(&current_remote_path);
                    let upload_parent = remote_path_parent(&remote_path_parent_of_upload(
                        &local_path_for_refresh,
                        &remote_path_for_refresh,
                    ));
                    if current_remote_path == remote_parent
                        || current_remote_path == upload_parent
                        || remote_path_for_refresh.starts_with(&current_remote_path)
                    {
                        this.refresh_dir(cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 执行下载任务
    fn start_download_task(
        &mut self,
        task_id: usize,
        remote_path: String,
        local_path: PathBuf,
        is_dir: bool,
        shared_progress: Arc<SharedProgress>,
        cx: &mut Context<Self>,
    ) {
        let Some(client) = self.transfer_client.clone() else {
            return;
        };

        let cancelled = shared_progress.cancelled.clone();
        let progress_for_callback = shared_progress.clone();

        let download_task = Tokio::spawn(cx, async move {
            let mut client_guard = client.lock().await;
            if is_dir {
                client_guard
                    .download_dir_with_progress(
                        &remote_path,
                        local_path.to_string_lossy().as_ref(),
                        cancelled,
                        Box::new(move |progress: TransferProgress| {
                            progress_for_callback
                                .transferred
                                .store(progress.transferred, Ordering::Relaxed);
                            progress_for_callback
                                .total
                                .store(progress.total, Ordering::Relaxed);
                            progress_for_callback
                                .speed
                                .store(progress.speed.to_bits(), Ordering::Relaxed);
                            if let Some(file) = progress.current_file {
                                if let Ok(mut guard) = progress_for_callback.current_file.write() {
                                    *guard = Some(file);
                                }
                            }
                        }),
                    )
                    .await
            } else {
                client_guard
                    .download_with_progress(
                        &remote_path,
                        local_path.to_string_lossy().as_ref(),
                        cancelled,
                        Box::new(move |progress: TransferProgress| {
                            progress_for_callback
                                .transferred
                                .store(progress.transferred, Ordering::Relaxed);
                            progress_for_callback
                                .total
                                .store(progress.total, Ordering::Relaxed);
                            progress_for_callback
                                .speed
                                .store(progress.speed.to_bits(), Ordering::Relaxed);
                        }),
                    )
                    .await
            }
        });

        cx.spawn(async move |this, cx| {
            let result = match download_task.await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(anyhow::Error::new(e)),
            };

            let _ = this.update(cx, |this, cx| {
                this.update_task_state(task_id, result);
                this.schedule_transfers(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// 更新任务状态
    fn update_task_state(&mut self, task_id: usize, result: Result<(), anyhow::Error>) {
        if let Some(task) = self
            .transfer_queue
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
        {
            match result {
                Ok(()) => {
                    task.state = TransferTaskState::Completed;
                    task.error = None;
                }
                Err(error) => {
                    if is_transfer_cancelled(&error) {
                        task.state = TransferTaskState::Cancelled;
                        task.error = None;
                    } else {
                        task.state = TransferTaskState::Failed;
                        task.error = Some(error.to_string());
                    }
                }
            }
        }
    }

    /// 取消传输
    fn cancel_transfer(&mut self, task_id: usize, cx: &mut Context<Self>) {
        if let Some(task) = self
            .transfer_queue
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
        {
            match task.state {
                TransferTaskState::Pending => {
                    task.state = TransferTaskState::Cancelled;
                    task.error = None;
                }
                TransferTaskState::Running => {
                    task.shared_progress
                        .cancelled
                        .store(true, Ordering::Relaxed);
                }
                TransferTaskState::Completed
                | TransferTaskState::Failed
                | TransferTaskState::Cancelled => {}
            }
        }
        self.schedule_transfers(cx);
        cx.notify();
    }

    /// 100ms 定时刷新进度
    fn start_progress_refresh(&mut self, cx: &mut Context<Self>) {
        if self.progress_refresh_task.is_some() {
            cx.notify();
            return;
        }

        self.progress_refresh_task = Some(cx.spawn(async move |this, cx| loop {
            let should_continue = this
                .update(cx, |this, cx| {
                    let has_active = this.transfer_queue.has_active();
                    if has_active {
                        cx.notify();
                        true
                    } else {
                        this.progress_refresh_task = None;
                        false
                    }
                })
                .unwrap_or(false);

            if !should_continue {
                break;
            }

            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
        }));
    }

    // ── 传输入口 ──────────────────────────────────────────────

    /// 将待上传项加入传输队列
    fn enqueue_pending_uploads(&mut self, uploads: Vec<PendingUpload>, cx: &mut Context<Self>) {
        for upload in uploads {
            let task = TransferTask {
                id: self.alloc_task_id(),
                operation: TransferOperation::Upload {
                    local_path: upload.local_path,
                    remote_path: upload.remote_path,
                    is_dir: upload.is_dir,
                },
                state: TransferTaskState::Pending,
                shared_progress: SharedProgress::new(),
                error: None,
            };
            self.transfer_queue.enqueue(task);
        }

        self.ensure_transfer_client_and_schedule(cx);
    }

    /// 上传前先检测目标目录中的重名项，必要时弹出冲突提示
    fn prepare_uploads(
        &mut self,
        paths: Vec<PathBuf>,
        remote_dir: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }

        let Some(client) = self.sftp_client.clone() else {
            let uploads: Vec<_> = paths
                .into_iter()
                .map(|path| {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    PendingUpload {
                        remote_path: join_remote_path(remote_dir, &name),
                        is_dir: path.is_dir(),
                        local_path: path,
                        name,
                        has_conflict: false,
                    }
                })
                .collect();
            self.enqueue_pending_uploads(uploads, cx);
            return;
        };

        let remote_dir = remote_dir.to_string();
        let view = cx.entity().clone();
        let list_task = Tokio::spawn(cx, {
            let remote_dir = remote_dir.clone();
            async move {
                let mut client_guard = client.lock().await;
                client_guard.list_dir(&remote_dir).await
            }
        });

        window
            .spawn(cx, async move |cx| {
                let remote_names: HashSet<String> = match list_task.await {
                    Ok(Ok(entries)) => entries
                        .into_iter()
                        .filter(|entry| entry.name != "." && entry.name != "..")
                        .map(|entry| entry.name)
                        .collect(),
                    Ok(Err(e)) => {
                        tracing::error!("读取远程目录失败: {}", e);
                        let error_msg = t!("FileManager.read_dir_failed", error = e).to_string();
                        let _ = view.update_in(cx, |_this, window, cx| {
                            window.push_notification(Notification::error(error_msg.clone()), cx);
                        });
                        return;
                    }
                    Err(e) => {
                        tracing::error!("远程目录检查任务失败: {}", e);
                        let error_msg = t!("FileManager.read_dir_failed", error = e).to_string();
                        let _ = view.update_in(cx, |_this, window, cx| {
                            window.push_notification(Notification::error(error_msg.clone()), cx);
                        });
                        return;
                    }
                };

                let _ = view.update_in(cx, |this, window, cx| {
                    let mut pending_uploads = Vec::new();
                    let mut has_conflict = false;

                    for path in paths {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let has_name_conflict = remote_names.contains(&name);
                        if has_name_conflict {
                            has_conflict = true;
                        }
                        pending_uploads.push(PendingUpload {
                            remote_path: join_remote_path(&remote_dir, &name),
                            is_dir: path.is_dir(),
                            local_path: path,
                            name,
                            has_conflict: has_name_conflict,
                        });
                    }

                    if pending_uploads.is_empty() {
                        return;
                    }

                    if has_conflict {
                        let conflict_names = pending_uploads
                            .iter()
                            .filter(|upload| upload.has_conflict)
                            .map(|upload| upload.name.clone())
                            .collect();
                        this.show_upload_conflict_dialog(
                            conflict_names,
                            pending_uploads,
                            remote_names,
                            window,
                            cx,
                        );
                    } else {
                        this.enqueue_pending_uploads(pending_uploads, cx);
                    }
                });
            })
            .detach();
    }

    fn show_upload_conflict_dialog(
        &mut self,
        conflict_names: Vec<String>,
        pending_uploads: Vec<PendingUpload>,
        existing_names: HashSet<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();
        let conflict_count = conflict_names.len();
        let conflict_list = if conflict_count <= 3 {
            conflict_names.join(", ")
        } else {
            t!(
                "Conflict.n_files",
                name = conflict_names[..3].join(", "),
                count = conflict_count
            )
            .to_string()
        };
        let has_dir_conflict = pending_uploads
            .iter()
            .any(|upload| upload.has_conflict && upload.is_dir);

        window.open_dialog(cx, move |dialog, _window, cx| {
            let view_overwrite = view.clone();
            let view_keep = view.clone();
            let view_skip = view.clone();
            let view_merge = view.clone();

            let uploads_overwrite = pending_uploads.clone();
            let uploads_keep = pending_uploads.clone();
            let uploads_skip = pending_uploads.clone();
            let uploads_merge = pending_uploads.clone();
            let existing_names_keep = existing_names.clone();

            dialog
                .title(t!("Dialog.file_conflict").to_string())
                .w(px(450.))
                .child(
                    v_flex()
                        .gap_2()
                        .child(t!("Conflict.files_exist").to_string())
                        .child(
                            div()
                                .p_2()
                                .bg(cx.theme().secondary)
                                .rounded_md()
                                .text_sm()
                                .child(conflict_list.clone()),
                        )
                        .child(t!("Conflict.choose_action").to_string()),
                )
                .footer(move |_, _, _window, _cx| {
                    let mut buttons: Vec<gpui::AnyElement> = vec![
                        Button::new("skip")
                            .label(t!("Conflict.skip").to_string())
                            .ghost()
                            .on_click({
                                let view = view_skip.clone();
                                let uploads = uploads_skip.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let uploads: Vec<_> = uploads
                                        .iter()
                                        .filter(|upload| !upload.has_conflict)
                                        .cloned()
                                        .collect();
                                    if !uploads.is_empty() {
                                        view.update(cx, |this, cx| {
                                            this.enqueue_pending_uploads(uploads, cx);
                                        });
                                    }
                                }
                            })
                            .into_any_element(),
                        Button::new("keep_both")
                            .label(t!("Conflict.keep_both").to_string())
                            .ghost()
                            .on_click({
                                let view = view_keep.clone();
                                let uploads = uploads_keep.clone();
                                let existing = existing_names_keep.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let uploads = rename_conflicting_uploads(
                                        uploads.clone(),
                                        existing.clone(),
                                    );
                                    view.update(cx, |this, cx| {
                                        this.enqueue_pending_uploads(uploads, cx);
                                    });
                                }
                            })
                            .into_any_element(),
                    ];

                    if has_dir_conflict {
                        buttons.push(
                            Button::new("merge")
                                .label(t!("Conflict.merge").to_string())
                                .ghost()
                                .on_click({
                                    let view = view_merge.clone();
                                    let uploads = uploads_merge.clone();
                                    move |_, window, cx| {
                                        window.close_dialog(cx);
                                        let uploads: Vec<_> = uploads
                                            .iter()
                                            .filter(|upload| !upload.has_conflict || upload.is_dir)
                                            .cloned()
                                            .collect();
                                        if !uploads.is_empty() {
                                            view.update(cx, |this, cx| {
                                                this.enqueue_pending_uploads(uploads, cx);
                                            });
                                        }
                                    }
                                })
                                .into_any_element(),
                        );
                    }

                    buttons.push(
                        Button::new("overwrite")
                            .label(t!("Conflict.overwrite").to_string())
                            .primary()
                            .on_click({
                                let view = view_overwrite.clone();
                                let uploads = uploads_overwrite.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    view.update(cx, |this, cx| {
                                        this.enqueue_pending_uploads(uploads.clone(), cx);
                                    });
                                }
                            })
                            .into_any_element(),
                    );

                    buttons
                })
                .overlay_closable(false)
                .close_button(true)
        });
    }

    /// 入队下载任务
    fn enqueue_download(
        &mut self,
        remote_path: String,
        local_path: PathBuf,
        is_dir: bool,
        cx: &mut Context<Self>,
    ) {
        let task = TransferTask {
            id: self.alloc_task_id(),
            operation: TransferOperation::Download {
                remote_path,
                local_path,
                is_dir,
            },
            state: TransferTaskState::Pending,
            shared_progress: SharedProgress::new(),
            error: None,
        };
        self.transfer_queue.enqueue(task);
        self.ensure_transfer_client_and_schedule(cx);
    }

    /// 通过文件选择器上传文件
    fn select_and_upload_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote_dir = self.current_path.clone();
        let view = cx.entity().clone();

        let future = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            multiple: true,
            directories: false,
            prompt: Some(t!("FileManager.select_upload_files").to_string().into()),
        });

        window
            .spawn(cx, async move |cx| {
                if let Ok(Ok(Some(paths))) = future.await {
                    if paths.is_empty() {
                        return;
                    }
                    let _ = view.update_in(cx, |this, window, cx| {
                        this.prepare_uploads(paths, &remote_dir, window, cx);
                    });
                }
            })
            .detach();
    }

    /// 通过文件夹选择器上传文件夹
    fn select_and_upload_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote_dir = self.current_path.clone();
        let view = cx.entity().clone();

        let future = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            multiple: true,
            directories: true,
            prompt: Some(t!("FileManager.select_upload_folder").to_string().into()),
        });

        window
            .spawn(cx, async move |cx| {
                if let Ok(Ok(Some(paths))) = future.await {
                    if paths.is_empty() {
                        return;
                    }
                    let _ = view.update_in(cx, |this, window, cx| {
                        this.prepare_uploads(paths, &remote_dir, window, cx);
                    });
                }
            })
            .detach();
    }

    fn show_new_folder_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("FileManager.new_folder_placeholder"))
        });
        let view = cx.entity().downgrade();

        input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let view_clone = view.clone();
            let input_for_callback = input.clone();

            dialog
                .title(t!("FileManager.new_folder").to_string())
                .w(px(360.))
                .child(Input::new(&input))
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.create").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let folder_name = input_for_callback.read(cx).text().to_string();
                    let folder_name = folder_name.trim().to_string();
                    if folder_name.is_empty() {
                        return false;
                    }
                    if !is_valid_entry_name(&folder_name) {
                        window.push_notification(
                            Notification::error(t!("FileManager.invalid_name")),
                            cx,
                        );
                        return false;
                    }

                    let _ = view_clone.update(cx, |this, cx| {
                        let Some(client) = this.sftp_client.clone() else {
                            return;
                        };

                        let remote_path = join_remote_path(&this.current_path, &folder_name);
                        let task = Tokio::spawn(cx, async move {
                            let mut client = client.lock().await;
                            client.mkdir(&remote_path).await
                        });

                        let view = cx.entity().clone();
                        window
                            .spawn(cx, async move |cx| match task.await {
                                Ok(Ok(_)) => {
                                    let _ = view.update_in(cx, |this, window, cx| {
                                        window.close_dialog(cx);
                                        this.refresh_dir(cx);
                                    });
                                }
                                Ok(Err(e)) => {
                                    tracing::error!("创建远程文件夹失败: {}", e);
                                    let error_msg =
                                        t!("FileManager.create_folder_failed", error = e)
                                            .to_string();
                                    let _ = view.update_in(cx, |_this, window, cx| {
                                        window.push_notification(
                                            Notification::error(error_msg.clone()),
                                            cx,
                                        );
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("远程创建文件夹任务失败: {}", e);
                                    let error_msg =
                                        t!("FileManager.create_folder_failed", error = e)
                                            .to_string();
                                    let _ = view.update_in(cx, |_this, window, cx| {
                                        window.push_notification(
                                            Notification::error(error_msg.clone()),
                                            cx,
                                        );
                                    });
                                }
                            })
                            .detach();
                    });
                    false
                })
        });
    }

    /// 通过保存目录选择器下载远程文件/文件夹
    fn download_item(
        &mut self,
        remote_path: String,
        is_dir: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().clone();
        let remote_name = remote_path
            .rsplit('/')
            .next()
            .unwrap_or(&remote_path)
            .to_string();

        let future = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            multiple: false,
            directories: true,
            prompt: Some(t!("FileManager.select_download_dir").to_string().into()),
        });

        cx.spawn(async move |_this, cx| {
            if let Ok(Ok(Some(paths))) = future.await {
                if let Some(dir) = paths.first() {
                    let local_path = dir.join(&remote_name);
                    view.update(cx, |this, cx| {
                        this.enqueue_download(remote_path, local_path, is_dir, cx);
                    });
                }
            }
        })
        .detach();
    }

    // ── 渲染方法 ──────────────────────────────────────────────

    /// 渲染工具栏
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_go_back = self.history_index > 0;
        let breadcrumb = self.render_path_breadcrumb(cx);

        v_flex()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar)
            .child(
                h_flex()
                    .h_9()
                    .px_2()
                    .gap_1()
                    .items_center()
                    // 后退按钮
                    .child(
                        div()
                            .id("fm-back")
                            .cursor_pointer()
                            .rounded_md()
                            .p(px(5.))
                            .when(!can_go_back, |el| el.opacity(0.4))
                            .when(can_go_back, |el| el.hover(|s| s.bg(cx.theme().list_active)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _window, cx| {
                                    this.go_back(cx);
                                }),
                            )
                            .tooltip(move |window, cx| {
                                Tooltip::new(t!("FileManager.go_back").to_string())
                                    .build(window, cx)
                            })
                            .child(
                                Icon::new(IconName::ArrowLeft)
                                    .small()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    // Home 按钮
                    .child(
                        div()
                            .id("fm-home")
                            .cursor_pointer()
                            .rounded_md()
                            .p(px(5.))
                            .hover(|s| s.bg(cx.theme().list_active))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _window, cx| {
                                    this.go_home(cx);
                                }),
                            )
                            .tooltip(move |window, cx| {
                                Tooltip::new(t!("FileManager.go_home").to_string())
                                    .build(window, cx)
                            })
                            .child(
                                Icon::new(IconName::Home)
                                    .small()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    // 上级目录按钮
                    .child(
                        div()
                            .id("fm-parent")
                            .cursor_pointer()
                            .rounded_md()
                            .p(px(5.))
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
                            .tooltip(move |window, cx| {
                                Tooltip::new(t!("FileManager.go_parent").to_string())
                                    .build(window, cx)
                            })
                            .child(
                                Icon::new(IconName::ArrowUp)
                                    .small()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(
                        Button::new("fm-upload-file")
                            .ghost()
                            .small()
                            .icon(IconName::Upload)
                            .tooltip(t!("FileManager.upload_file"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.select_and_upload_files(window, cx);
                            })),
                    )
                    .child(
                        Button::new("fm-new-folder")
                            .ghost()
                            .small()
                            .icon(IconName::NewFolder)
                            .tooltip(t!("FileManager.new_folder"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.show_new_folder_dialog(window, cx);
                            })),
                    )
                    .child(div().flex_1())
                    // 同步终端工作目录按钮
                    .child(
                        div()
                            .id("fm-sync-terminal")
                            .cursor_pointer()
                            .rounded_md()
                            .p(px(5.))
                            .hover(|s| s.bg(cx.theme().list_active))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_this, _, _window, cx| {
                                    cx.emit(FileManagerPanelEvent::SyncWorkingDir);
                                }),
                            )
                            .tooltip(move |window, cx| {
                                Tooltip::new(t!("FileManager.sync_terminal_dir").to_string())
                                    .build(window, cx)
                            })
                            .child(
                                Icon::new(IconName::Sync)
                                    .small()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    // 刷新按钮
                    .child(
                        div()
                            .id("fm-refresh")
                            .cursor_pointer()
                            .rounded_md()
                            .p(px(5.))
                            .hover(|s| s.bg(cx.theme().list_active))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _window, cx| {
                                    this.refresh_dir(cx);
                                }),
                            )
                            .tooltip(move |window, cx| {
                                Tooltip::new(t!("FileManager.refresh").to_string())
                                    .build(window, cx)
                            })
                            .child(
                                Icon::new(IconName::Refresh)
                                    .small()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    // 隐藏文件开关
                    .child(
                        div()
                            .id("fm-hidden")
                            .cursor_pointer()
                            .rounded_md()
                            .p(px(5.))
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
                            .tooltip(move |window, cx| {
                                Tooltip::new(t!("FileManager.toggle_hidden").to_string())
                                    .build(window, cx)
                            })
                            .child(
                                Icon::new(IconName::Eye)
                                    .small()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    // 关闭按钮
                    .child(
                        div()
                            .id("fm-close")
                            .cursor_pointer()
                            .rounded_md()
                            .p(px(5.))
                            .hover(|s| s.bg(cx.theme().list_active))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_this, _, _window, cx| {
                                    cx.emit(FileManagerPanelEvent::Close);
                                }),
                            )
                            .child(
                                Icon::new(IconName::Close)
                                    .small()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .h_8()
                    .px_2()
                    .pb_2()
                    .items_center()
                    .child(if self.path_editing {
                        h_flex()
                            .id("fm-path-editor")
                            .flex_1()
                            .min_w(px(0.))
                            .h_7()
                            .px_2()
                            .items_center()
                            .bg(cx.theme().secondary)
                            .rounded_md()
                            .child(
                                Input::new(&self.path_input)
                                    .small()
                                    .appearance(false)
                                    .cleanable(false)
                                    .w_full(),
                            )
                            .into_any_element()
                    } else {
                        h_flex()
                            .id("fm-path")
                            .flex_1()
                            .min_w(px(0.))
                            .h_7()
                            .px_2()
                            .items_center()
                            .bg(cx.theme().secondary)
                            .cursor_text()
                            .rounded_md()
                            .hover(|style| style.bg(cx.theme().list_active))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.start_path_editing(window, cx);
                            }))
                            .child(breadcrumb.flex_1().min_w(px(0.)).overflow_hidden())
                            .tooltip(move |window, cx| {
                                Tooltip::new(t!("FileManager.edit_path").to_string())
                                    .build(window, cx)
                            })
                            .into_any_element()
                    }),
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
        let path_for_download = full_path.to_string();
        let is_dir_for_download = is_dir;

        let mut menu = menu;

        // 下载
        let view_download = view.clone();
        menu = menu.item(
            PopupMenuItem::new(t!("FileManager.download"))
                .icon(IconName::ArrowDown)
                .on_click(
                    window.listener_for(&view_download, move |this, _, window, cx| {
                        this.download_item(
                            path_for_download.clone(),
                            is_dir_for_download,
                            window,
                            cx,
                        );
                    }),
                ),
        );

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

        // 分隔线 + 上传文件 + 上传文件夹 + 刷新
        let view_upload_files = view.clone();
        let view_upload_folder = view.clone();
        let view_refresh = view.clone();
        menu = menu
            .separator()
            .item(
                PopupMenuItem::new(t!("FileManager.upload_file"))
                    .icon(IconName::Upload)
                    .on_click(window.listener_for(
                        &view_upload_files,
                        move |this, _, window, cx| {
                            this.select_and_upload_files(window, cx);
                        },
                    )),
            )
            .item(
                PopupMenuItem::new(t!("FileManager.upload_folder"))
                    .icon(IconName::Upload)
                    .on_click(window.listener_for(
                        &view_upload_folder,
                        move |this, _, window, cx| {
                            this.select_and_upload_folder(window, cx);
                        },
                    )),
            )
            .separator()
            .item(
                PopupMenuItem::new(t!("FileManager.refresh"))
                    .icon(IconName::Refresh)
                    .on_click(window.listener_for(&view_refresh, move |this, _, _, cx| {
                        this.refresh_dir(cx);
                    })),
            );

        menu
    }

    /// 渲染底部传输进度条（紧凑型，适合侧边栏窄宽度）
    fn render_transfer_progress(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(task) = self.transfer_queue.active_task() else {
            return div().into_any_element();
        };

        let (icon, label) = match &task.operation {
            TransferOperation::Upload { local_path, .. } => (
                IconName::ArrowUp,
                local_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
            TransferOperation::Download { remote_path, .. } => {
                let name = remote_path.rsplit('/').next().unwrap_or(remote_path);
                (IconName::ArrowDown, name.to_string())
            }
        };

        let transferred = task.shared_progress.transferred.load(Ordering::Relaxed);
        let total = task.shared_progress.total.load(Ordering::Relaxed);
        let speed_bits = task.shared_progress.speed.load(Ordering::Relaxed);
        let speed = f64::from_bits(speed_bits);

        let progress_pct = if total > 0 {
            (transferred as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };

        let task_id = task.id;
        let is_running = task.state == TransferTaskState::Running;
        let pending_count = self.transfer_queue.pending_count();

        let status_text = match task.state {
            TransferTaskState::Pending => t!("FileManager.transfer_pending").to_string(),
            TransferTaskState::Running => {
                if is_running && speed > 0.0 {
                    format!("{}% {}", progress_pct, format_speed(speed))
                } else {
                    format!("{}%", progress_pct)
                }
            }
            TransferTaskState::Completed => t!("FileManager.transfer_done").to_string(),
            TransferTaskState::Failed => t!("FileManager.transfer_failed").to_string(),
            TransferTaskState::Cancelled => t!("FileManager.transfer_cancelled").to_string(),
        };

        let tooltip_label = label.clone();

        v_flex()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar)
            .px_2()
            .py_1()
            .gap_1()
            // 第一行：图标 + 文件名 + 状态文本 + 取消按钮
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Icon::new(icon)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        div()
                            .id("fm-transfer-name")
                            .flex_1()
                            .text_xs()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(label)
                            .tooltip(move |window, cx| {
                                Tooltip::new(tooltip_label.clone()).build(window, cx)
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(status_text),
                    )
                    .when(
                        is_running || task.state == TransferTaskState::Pending,
                        |el| {
                            el.child(
                                div()
                                    .id("fm-cancel-transfer")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .p(px(2.))
                                    .hover(|s| s.bg(cx.theme().list_active))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _, _window, cx| {
                                            this.cancel_transfer(task_id, cx);
                                        }),
                                    )
                                    .child(
                                        Icon::new(IconName::Close)
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground),
                                    ),
                            )
                        },
                    ),
            )
            // 第二行：进度条 + 排队数
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        div().flex_1().child(
                            Progress::new("fm-transfer-progress").value(progress_pct as f32),
                        ),
                    )
                    .when(pending_count > 0, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("+{}", pending_count)),
                        )
                    }),
            )
            .into_any_element()
    }

    /// 渲染拖拽覆盖层
    fn render_drop_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(gpui::rgba(0x3b82f630))
            .border_2()
            .border_color(gpui::rgba(0x3b82f6ff))
            .rounded_md()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex().items_center().gap_2().child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(cx.theme().foreground)
                        .child(t!("FileManager.drop_files_here")),
                ),
            )
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
        let has_active_transfer = self.transfer_queue.has_active();
        let is_dragging = self.is_dragging_over;

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
                    div()
                        .id("fm-file-list-drop-zone")
                        .flex_1()
                        .relative()
                        // 拖拽上传支持
                        .drag_over::<ExternalPaths>(|el, _, _, _cx| el.bg(gpui::rgba(0x3b82f620)))
                        .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                            this.is_dragging_over = false;
                            let file_paths = paths.paths().to_vec();
                            if !file_paths.is_empty() {
                                let remote_dir = this.current_path.clone();
                                this.prepare_uploads(file_paths, &remote_dir, window, cx);
                            }
                        }))
                        .child(
                            uniform_list("fm-file-list", total_count, {
                                cx.processor(
                                    move |state: &mut Self, range: Range<usize>, _window, cx| {
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
                                                let is_selected =
                                                    state.selected_indices.contains(&filtered_ix);
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
                                                            move |this,
                                                                  event: &MouseDownEvent,
                                                                  _window,
                                                                  cx| {
                                                                let multi_select =
                                                                    event.modifiers.secondary();
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
                                                                this.navigate_to(
                                                                    fp.clone(),
                                                                    cx,
                                                                );
                                                            } else {
                                                                cx.write_to_clipboard(
                                                                    ClipboardItem::new_string(
                                                                        name.clone(),
                                                                    ),
                                                                );
                                                            }
                                                        }
                                                    }))
                                                    .context_menu(
                                                        move |menu, window, cx| {
                                                            Self::build_context_menu(
                                                                menu,
                                                                &ctx_name,
                                                                &ctx_full_path,
                                                                ctx_is_dir,
                                                                &ctx_view,
                                                                window,
                                                                cx,
                                                            )
                                                        },
                                                    )
                                                    .child(state.render_file_row(
                                                        item,
                                                        is_selected,
                                                        cx,
                                                    ))
                                                    .into_any_element()
                                            })
                                            .collect()
                                    },
                                )
                            })
                            .flex_1()
                            .size_full()
                            .track_scroll(&scroll_handle)
                            .with_sizing_behavior(ListSizingBehavior::Auto),
                        )
                        .when(is_dragging, |el| el.child(self.render_drop_overlay(cx))),
                )
            })
            // 底部传输进度条
            .when(has_active_transfer, |el| {
                el.child(self.render_transfer_progress(cx))
            })
    }
}

/// 获取远程路径的父目录
fn remote_path_parent(path: &str) -> String {
    if path == "/" || path.is_empty() {
        "/".to_string()
    } else {
        let trimmed = path.trim_end_matches('/');
        match trimmed.rfind('/') {
            Some(0) => "/".to_string(),
            Some(pos) => trimmed[..pos].to_string(),
            None => "/".to_string(),
        }
    }
}

/// 从上传操作中推断远程目标的父目录
fn remote_path_parent_of_upload(_local_path: &PathBuf, remote_path: &str) -> String {
    remote_path.to_string()
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

#[cfg(test)]
mod tests {
    use super::{
        build_refresh_error_plan, build_retry_reset_plan, clear_remote_listing_state,
        ConnectionState, RetryResetPlan,
    };
    use std::collections::HashSet;

    #[test]
    fn build_retry_reset_plan_prefers_explicit_working_dir() {
        let plan = build_retry_reset_plan("/srv/project", Some("/srv/override".to_string()));

        assert_eq!(plan.next_state, ConnectionState::Idle);
        assert_eq!(plan.initial_working_dir.as_deref(), Some("/srv/override"));
        assert!(plan.clear_listing);
    }

    #[test]
    fn build_refresh_error_plan_preserves_current_path_for_retry() {
        let plan = build_refresh_error_plan("/srv/project", "连接已断开".to_string());

        assert_eq!(
            plan,
            RetryResetPlan {
                next_state: ConnectionState::Error("连接已断开".to_string()),
                initial_working_dir: Some("/srv/project".to_string()),
                clear_listing: true,
            }
        );
    }

    #[test]
    fn clear_remote_listing_state_clears_items_and_selection() {
        let mut items = vec![1, 2, 3];
        let mut filtered_indices = vec![0, 2];
        let mut selected_indices = HashSet::from([0usize, 1usize]);

        clear_remote_listing_state(&mut items, &mut filtered_indices, &mut selected_indices);

        assert!(items.is_empty());
        assert!(filtered_indices.is_empty());
        assert!(selected_indices.is_empty());
    }
}
