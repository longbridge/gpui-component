//! Terminal 模型层
//!
//! 独立的 Terminal Entity，负责：
//! - PTY/SSH 后端通信
//! - 终端状态管理（Term grid、选择、滚动）
//! - 事件发送（Title、Bell、ChildExit 等）
//!
//! 与 TerminalView 分离，TerminalView 只负责视图逻辑。

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point as AlacPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config as TermConfig, Term, TermMode};
use alacritty_terminal::tty::{self, Options as PtyOptions};
use futures::StreamExt;
use gpui::*;
use one_core::gpui_tokio::Tokio;
use one_core::storage::models::{
    ActiveConnections, ProxyType as StorageProxyType, SerialParams, SshAuthMethod, StoredConnection,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::interval;

#[cfg(any(test, target_os = "windows"))]
use std::env;
#[cfg(any(test, target_os = "windows"))]
use std::ffi::OsStr;
#[cfg(any(test, target_os = "windows"))]
use std::path::{Path, PathBuf};

use crate::pty_backend::{GpuiEventProxy, LocalPtyBackend};

use crate::{LocalConfig, SerialBackend, SshBackend, TerminalBackend, TerminalEvent, TerminalSize};
pub use ssh::{
    JumpServerConnectConfig, ProxyConnectConfig, ProxyType, PtyConfig, SshAuth, SshConnectConfig,
};

/// Terminal 发出的事件，供 TerminalView 订阅
#[derive(Debug, Clone)]
pub enum TerminalModelEvent {
    /// 终端内容已更新，需要重新渲染
    Wakeup,
    /// 终端标题已更改
    TitleChanged(String),
    /// 终端响铃
    Bell,
    /// 子进程已退出
    ChildExit(i32),
    /// 终端程序请求存储到剪贴板
    ClipboardStore(String),
    /// 远程工作目录变更（OSC 7）
    WorkingDirChanged(String),
}

/// 终端连接状态
#[derive(Clone, PartialEq, Debug)]
pub enum ConnectionState {
    Connected,
    Connecting,
    Disconnected { error: Option<String> },
}

/// 终端连接类型
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerminalConnectionKind {
    Local,
    Ssh,
    Serial,
}

/// SSH 终端配置
#[derive(Clone)]
pub struct SshTerminalConfig {
    pub ssh_config: SshConnectConfig,
    pub pty_config: PtyConfig,
}

const DEFAULT_COLS: usize = 80;
const DEFAULT_ROWS: usize = 24;

/// 将路径安全地转为 POSIX shell 单参数，避免命令注入。
pub(crate) fn shell_escape_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }

    let mut escaped = String::with_capacity(arg.len() + 2);
    escaped.push('\'');
    for ch in arg.chars() {
        if ch == '\'' {
            escaped.push_str("'\"'\"'");
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\'');
    escaped
}

fn build_cd_command(dir: &str) -> String {
    format!("cd -- {}", shell_escape_arg(dir))
}

const OSC7_PROMPT_COMMAND: &str = r#"export PROMPT_COMMAND='printf "\033]7;file://%s%s\007" "$HOSTNAME" "$PWD"'${PROMPT_COMMAND:+";$PROMPT_COMMAND"}"#;

fn build_ssh_base_init_commands(
    working_dir: Option<&str>,
    default_directory: Option<&str>,
    init_script: Option<&str>,
) -> Option<String> {
    let mut commands = Vec::new();

    if let Some(work_dir) = working_dir {
        commands.push(build_cd_command(work_dir));
    } else {
        if let Some(dir) = default_directory.filter(|dir| !dir.is_empty()) {
            commands.push(build_cd_command(dir));
        }
        if let Some(script) = init_script.filter(|script| !script.is_empty()) {
            commands.push(script.to_string());
        }
    }

    (!commands.is_empty()).then(|| commands.join("\n"))
}

fn compose_ssh_init_commands(
    base_init_commands: Option<&str>,
    sync_path_with_terminal: bool,
) -> Option<String> {
    let mut commands = Vec::new();

    if let Some(base_commands) = base_init_commands.filter(|commands| !commands.is_empty()) {
        commands.push(base_commands.to_string());
    }

    if sync_path_with_terminal {
        commands.push(OSC7_PROMPT_COMMAND.to_string());
    }

    (!commands.is_empty()).then(|| commands.join("\n"))
}

fn build_ssh_init_commands(
    working_dir: Option<&str>,
    default_directory: Option<&str>,
    init_script: Option<&str>,
    sync_path_with_terminal: bool,
) -> Option<String> {
    let base_init_commands =
        build_ssh_base_init_commands(working_dir, default_directory, init_script);
    compose_ssh_init_commands(base_init_commands.as_deref(), sync_path_with_terminal)
}

#[cfg(any(test, target_os = "windows"))]
fn path_if_file(path: impl Into<PathBuf>) -> Option<String> {
    let path = path.into();
    path.is_file().then(|| path.to_string_lossy().into_owned())
}

#[cfg(any(test, target_os = "windows"))]
fn find_executable_in_path(path_env: Option<&OsStr>, program: &str) -> Option<String> {
    let path_env = path_env?;
    env::split_paths(path_env)
        .map(|dir| dir.join(program))
        .find_map(path_if_file)
}

#[cfg(any(test, target_os = "windows"))]
fn resolve_default_windows_shell_from_env(
    path_env: Option<&OsStr>,
    system_root: Option<&OsStr>,
    comspec: Option<&OsStr>,
) -> String {
    if let Some(pwsh) = find_executable_in_path(path_env, "pwsh.exe") {
        return pwsh;
    }

    if let Some(system_root) = system_root {
        let powershell = Path::new(system_root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if let Some(powershell) = path_if_file(powershell) {
            return powershell;
        }
    }

    if let Some(powershell) = find_executable_in_path(path_env, "powershell.exe") {
        return powershell;
    }

    if let Some(comspec) = comspec.and_then(path_if_file) {
        return comspec;
    }

    if let Some(system_root) = system_root {
        let cmd = Path::new(system_root).join("System32").join("cmd.exe");
        if let Some(cmd) = path_if_file(cmd) {
            return cmd;
        }
    }

    "cmd.exe".to_string()
}

#[cfg(target_os = "windows")]
fn build_local_shell(shell: Option<String>) -> Option<tty::Shell> {
    let program = shell.unwrap_or_else(|| {
        resolve_default_windows_shell_from_env(
            env::var_os("PATH").as_deref(),
            env::var_os("SystemRoot")
                .or_else(|| env::var_os("SYSTEMROOT"))
                .as_deref(),
            env::var_os("COMSPEC").as_deref(),
        )
    });
    Some(tty::Shell::new(program, vec![]))
}

#[cfg(not(target_os = "windows"))]
fn build_local_shell(shell: Option<String>) -> Option<tty::Shell> {
    shell.map(|program| tty::Shell::new(program, vec![]))
}

/// 终端模型 Entity
///
/// 负责管理终端的核心状态，包括：
/// - alacritty Term grid
/// - PTY/SSH 后端
/// - 连接状态
/// - 标题
pub struct Terminal {
    /// alacritty 终端状态
    term: Arc<FairMutex<Term<GpuiEventProxy>>>,
    /// PTY/SSH 后端
    backend: Option<Box<dyn TerminalBackend>>,

    /// 终端标题
    title: String,
    /// 当前工作目录（由 OSC 7 更新，仅 SSH 终端）
    current_working_dir: Option<String>,
    /// 子进程退出码
    child_exited: Option<i32>,
    /// 连接状态
    connection_state: ConnectionState,

    /// 终端尺寸
    cols: usize,
    rows: usize,

    /// SSH 配置（用于重连）
    ssh_config: Option<SshTerminalConfig>,
    /// 串口参数（用于重连）
    serial_params: Option<SerialParams>,
    /// 事件发送器（用于 SSH 重连）
    event_tx: Option<UnboundedSender<TerminalEvent>>,
    /// 事件代理（用于设置 PtyWrite 回写通道）
    event_proxy: Option<GpuiEventProxy>,
    /// 连接 ID
    connection_id: Option<i64>,
    /// 连接名称
    connection_name: Option<String>,
    /// SSH 基础初始化命令（不含 OSC7，用于运行时重建）
    ssh_base_init_commands: Option<String>,
    /// 初始化命令（连接成功后执行）
    init_commands: Option<String>,

    /// 连接类型
    connection_kind: TerminalConnectionKind,
}

#[derive(Clone)]
pub struct TerminalScrollProxy {
    term: Arc<FairMutex<Term<GpuiEventProxy>>>,
    event_tx: Option<UnboundedSender<TerminalEvent>>,
}

/// Snapshot of terminal scroll state, captured in a single lock acquisition
/// to ensure consistency.
#[derive(Clone, Debug)]
pub struct TerminalScrollSnapshot {
    pub display_offset: usize,
    pub history_size: usize,
    pub screen_lines: usize,
    pub columns: usize,
}

impl TerminalScrollProxy {
    /// Snapshot all scroll-related state in a single lock acquisition
    /// to avoid inconsistency from multiple separate locks.
    pub fn snapshot(&self) -> TerminalScrollSnapshot {
        let term = self.term.lock();
        TerminalScrollSnapshot {
            display_offset: term.grid().display_offset(),
            history_size: term.history_size(),
            screen_lines: term.screen_lines(),
            columns: term.columns(),
        }
    }

    pub fn display_offset(&self) -> usize {
        self.term.lock().grid().display_offset()
    }

    pub fn history_size(&self) -> usize {
        self.term.lock().history_size()
    }

    pub fn screen_lines(&self) -> usize {
        self.term.lock().screen_lines()
    }

    pub fn columns(&self) -> usize {
        self.term.lock().columns()
    }

    pub fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    pub fn scroll_display_delta(&self, delta: i32) {
        if delta == 0 {
            return;
        }
        self.term
            .lock()
            .scroll_display(alacritty_terminal::grid::Scroll::Delta(delta));
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(TerminalEvent::Wakeup);
        }
    }
}

impl Terminal {
    fn new_local_disconnected(error: String, cx: &mut Context<Self>) -> Self {
        let (event_tx, event_rx) = unbounded_channel::<TerminalEvent>();
        let (term, _event_proxy, _colors) =
            Self::create_term(DEFAULT_COLS, DEFAULT_ROWS, event_tx.clone());

        Self::spawn_event_loop(event_rx, cx);

        Self {
            term,
            backend: None,
            title: String::new(),
            current_working_dir: None,
            child_exited: None,
            connection_state: ConnectionState::Disconnected { error: Some(error) },
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            ssh_config: None,
            serial_params: None,
            event_tx: Some(event_tx),
            event_proxy: None,
            connection_id: None,
            connection_name: None,
            ssh_base_init_commands: None,
            init_commands: None,
            connection_kind: TerminalConnectionKind::Local,
        }
    }

    pub fn new_local_or_disconnected(
        config: LocalConfig,
        cx: &mut Context<Self>,
    ) -> (Self, Option<String>) {
        match Self::new_local(config, cx) {
            Ok(terminal) => (terminal, None),
            Err(error) => {
                let message = error.to_string();
                (
                    Self::new_local_disconnected(message.clone(), cx),
                    Some(message),
                )
            }
        }
    }

    /// 创建本地终端
    pub fn new_local(config: LocalConfig, cx: &mut Context<Self>) -> Result<Self> {
        let (event_tx, event_rx) = unbounded_channel::<TerminalEvent>();
        let (term, event_proxy, _colors) =
            Self::create_term(DEFAULT_COLS, DEFAULT_ROWS, event_tx.clone());
        let LocalConfig {
            shell,
            working_dir,
            env,
        } = config;

        let pty_options = PtyOptions {
            shell: build_local_shell(shell),
            working_directory: working_dir.map(Into::into),
            env: env.into_iter().collect(),
            drain_on_exit: true,
            #[cfg(target_os = "windows")]
            escape_args: true,
        };
        let local_backend = LocalPtyBackend::new(term.clone(), event_proxy, pty_options)?;

        Self::spawn_event_loop(event_rx, cx);

        Ok(Self {
            term,
            backend: Some(Box::new(local_backend)),
            title: String::new(),
            current_working_dir: None,
            child_exited: None,
            connection_state: ConnectionState::Connected,
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            ssh_config: None,
            serial_params: None,
            event_tx: Some(event_tx),
            event_proxy: None, // 本地终端的 event_proxy 已在 LocalPtyBackend 中设置
            connection_id: None,
            connection_name: None,
            ssh_base_init_commands: None,
            init_commands: None,
            connection_kind: TerminalConnectionKind::Local,
        })
    }

    /// 创建 SSH 终端
    pub fn new_ssh(
        conn: StoredConnection,
        cx: &mut Context<Self>,
        working_dir: Option<&str>,
        sync_path_with_terminal: bool,
    ) -> Self {
        let ssh_params = conn
            .to_ssh_params()
            .expect("StoredConnection should contain valid SSH params");

        let auth = match ssh_params.auth_method.clone() {
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

        // 构建初始化命令
        let ssh_base_init_commands = build_ssh_base_init_commands(
            working_dir,
            ssh_params.default_directory.as_deref(),
            ssh_params.init_script.as_deref(),
        );
        let init_commands = build_ssh_init_commands(
            working_dir,
            ssh_params.default_directory.as_deref(),
            ssh_params.init_script.as_deref(),
            sync_path_with_terminal,
        );

        let ssh_config = SshConnectConfig {
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
        };

        let pty_config = PtyConfig::default();
        let config = SshTerminalConfig {
            ssh_config,
            pty_config,
        };

        let cols = config.pty_config.width as usize;
        let rows = config.pty_config.height as usize;

        let (event_tx, event_rx) = unbounded_channel::<TerminalEvent>();
        let (term, event_proxy, _colors) = Self::create_term(cols, rows, event_tx.clone());
        let (disconnect_tx, disconnect_rx) = tokio::sync::oneshot::channel::<()>();

        Self::spawn_disconnect_handler(disconnect_rx, cx);
        Self::spawn_event_loop(event_rx, cx);
        Self::spawn_ssh_connect(
            config.clone(),
            term.clone(),
            event_proxy.clone(),
            event_tx.clone(),
            Some(disconnect_tx),
            init_commands.clone(),
            cx,
        );

        Self {
            term,
            backend: None,
            title: String::new(),
            current_working_dir: None,
            child_exited: None,
            connection_state: ConnectionState::Connecting,
            cols,
            rows,
            ssh_config: Some(config),
            serial_params: None,
            event_tx: Some(event_tx),
            event_proxy: Some(event_proxy),
            connection_id: conn.id,
            connection_name: Some(conn.name),
            ssh_base_init_commands,
            init_commands,
            connection_kind: TerminalConnectionKind::Ssh,
        }
    }

    /// 创建串口终端
    pub fn new_serial(conn: StoredConnection, cx: &mut Context<Self>) -> Self {
        let serial_params = conn
            .to_serial_params()
            .expect("StoredConnection 应包含有效的 SerialParams");

        let (event_tx, event_rx) = unbounded_channel::<TerminalEvent>();
        let (term, _event_proxy, _colors) =
            Self::create_term(DEFAULT_COLS, DEFAULT_ROWS, event_tx.clone());
        let (disconnect_tx, disconnect_rx) = tokio::sync::oneshot::channel::<()>();

        Self::spawn_disconnect_handler(disconnect_rx, cx);
        Self::spawn_event_loop(event_rx, cx);
        Self::spawn_serial_connect(
            serial_params.clone(),
            term.clone(),
            event_tx.clone(),
            Some(disconnect_tx),
            cx,
        );

        Self {
            term,
            backend: None,
            title: String::new(),
            current_working_dir: None,
            child_exited: None,
            connection_state: ConnectionState::Connecting,
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            ssh_config: None,
            serial_params: Some(serial_params),
            event_tx: Some(event_tx),
            event_proxy: None,
            connection_id: conn.id,
            connection_name: Some(conn.name),
            ssh_base_init_commands: None,
            init_commands: None,
            connection_kind: TerminalConnectionKind::Serial,
        }
    }

    fn create_term(
        cols: usize,
        rows: usize,
        event_tx: UnboundedSender<TerminalEvent>,
    ) -> (
        Arc<FairMutex<Term<GpuiEventProxy>>>,
        GpuiEventProxy,
        alacritty_terminal::term::color::Colors,
    ) {
        let term_config = TermConfig {
            scrolling_history: 10000,
            ..Default::default()
        };
        let event_proxy = GpuiEventProxy::new(event_tx);
        let term = Term::new(
            term_config,
            &TermDimensions { cols, rows },
            event_proxy.clone(),
        );
        let colors = term.colors().clone();
        (Arc::new(FairMutex::new(term)), event_proxy, colors)
    }

    fn spawn_event_loop(mut event_rx: UnboundedReceiver<TerminalEvent>, cx: &mut Context<Self>) {
        let _entity = cx.entity().downgrade();
        let (render_tx, mut render_rx) = futures::channel::mpsc::unbounded::<TerminalEvent>();

        // 后台事件聚合任务 - 8ms 节流
        Tokio::spawn(cx, async move {
            let mut render_interval = interval(Duration::from_millis(8));
            let mut pending_wakeup = false;
            let mut pending_events: Vec<TerminalEvent> = Vec::new();

            loop {
                tokio::select! {
                    result = event_rx.recv() => {
                        match result {
                            None => break,
                            Some(event) => {
                                match &event {
                                    TerminalEvent::Wakeup => pending_wakeup = true,
                                    _ => pending_events.push(event),
                                }
                            }
                        }
                    }
                    _ = render_interval.tick() => {
                        // 先发送非 Wakeup 事件
                        for event in pending_events.drain(..) {
                            if render_tx.unbounded_send(event).is_err() {
                                return;
                            }
                        }
                        // 最后发送 Wakeup
                        if pending_wakeup {
                            pending_wakeup = false;
                            if render_tx.unbounded_send(TerminalEvent::Wakeup).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        })
        .detach();

        // GPUI 线程事件处理
        cx.spawn(async move |this, cx| {
            while let Some(event) = render_rx.next().await {
                if this
                    .update(cx, |this, cx| {
                        this.handle_terminal_event(event, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn spawn_disconnect_handler(
        disconnect_rx: tokio::sync::oneshot::Receiver<()>,
        cx: &mut Context<Self>,
    ) {
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let _ = disconnect_rx.await;
            let _ = entity.update(cx, |this, cx| {
                this.connection_state = ConnectionState::Disconnected { error: None };
                this.backend = None;
                this.set_connection_active(false, cx);
                cx.emit(TerminalModelEvent::Wakeup);
            });
        })
        .detach();
    }

    fn spawn_ssh_connect(
        config: SshTerminalConfig,
        term: Arc<FairMutex<Term<GpuiEventProxy>>>,
        event_proxy: GpuiEventProxy,
        event_tx: UnboundedSender<TerminalEvent>,
        on_disconnect: Option<tokio::sync::oneshot::Sender<()>>,
        init_commands: Option<String>,
        cx: &mut Context<Self>,
    ) {
        // 创建 SSH 后端需要的通知通道（UnboundedSender<()>）
        let (notify_tx, mut notify_rx) = unbounded_channel::<()>();

        let task = Tokio::spawn(cx, async move {
            // 转发 SSH 通知到事件通道（必须在 tokio runtime 内部）
            let event_tx_clone = event_tx.clone();
            tokio::spawn(async move {
                while notify_rx.recv().await.is_some() {
                    let _ = event_tx_clone.send(TerminalEvent::Wakeup);
                }
            });

            let disconnect_tx = on_disconnect.map(|tx| {
                let (sender, mut receiver) = unbounded_channel::<()>();
                tokio::spawn(async move {
                    if receiver.recv().await.is_some() {
                        let _ = tx.send(());
                    }
                });
                sender
            });
            SshBackend::connect(
                config.ssh_config,
                config.pty_config,
                term,
                event_proxy,
                event_tx,
                notify_tx,
                disconnect_tx,
                init_commands,
            )
            .await
        });

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.handle_ssh_result(result, cx);
            });
        })
        .detach();
    }

    fn handle_ssh_result(
        &mut self,
        result: Result<Result<SshBackend, anyhow::Error>, tokio::task::JoinError>,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(Ok(backend)) => {
                self.connection_state = ConnectionState::Connected;
                self.set_connection_active(true, cx);
                // 连接后重新调整终端大小
                self.term.lock().resize(TermDimensions {
                    cols: self.cols,
                    rows: self.rows,
                });
                // 重要：将当前终端尺寸同步到新连接的 SSH 后端
                // 因为远程 PTY 是用 PtyConfig 默认尺寸（80x24）创建的，
                // 需要调整到当前实际尺寸
                tracing::info!(
                    "SSH 连接成功，同步终端尺寸到远程: {}x{}",
                    self.cols,
                    self.rows
                );
                backend.resize(TerminalSize {
                    rows: self.rows as u16,
                    cols: self.cols as u16,
                    pixel_width: 0,
                    pixel_height: 0,
                });
                self.backend = Some(Box::new(backend));
            }
            Ok(Err(e)) => {
                self.connection_state = ConnectionState::Disconnected {
                    error: Some(e.to_string()),
                };
                self.set_connection_active(false, cx);
            }
            Err(e) => {
                self.connection_state = ConnectionState::Disconnected {
                    error: Some(e.to_string()),
                };
                self.set_connection_active(false, cx);
            }
        }
        cx.emit(TerminalModelEvent::Wakeup);
    }

    fn spawn_serial_connect(
        params: SerialParams,
        term: Arc<FairMutex<Term<GpuiEventProxy>>>,
        event_tx: UnboundedSender<TerminalEvent>,
        on_disconnect: Option<tokio::sync::oneshot::Sender<()>>,
        cx: &mut Context<Self>,
    ) {
        let disconnect_tx = on_disconnect.map(|tx| {
            let (sender, mut receiver) = unbounded_channel::<()>();
            Tokio::spawn(cx, async move {
                if receiver.recv().await.is_some() {
                    let _ = tx.send(());
                }
            })
            .detach();
            sender
        });

        let result = SerialBackend::connect(params, term, event_tx, disconnect_tx);

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let _ = this.update(cx, |this, cx| {
                this.handle_serial_result(result, cx);
            });
        })
        .detach();
    }

    fn handle_serial_result(
        &mut self,
        result: anyhow::Result<SerialBackend>,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(backend) => {
                self.connection_state = ConnectionState::Connected;
                self.set_connection_active(true, cx);
                self.backend = Some(Box::new(backend));
                tracing::info!("串口连接成功");
            }
            Err(e) => {
                self.connection_state = ConnectionState::Disconnected {
                    error: Some(e.to_string()),
                };
                self.set_connection_active(false, cx);
            }
        }
        cx.emit(TerminalModelEvent::Wakeup);
    }

    fn set_connection_active(&self, active: bool, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id else {
            return;
        };

        let global_state = cx.global_mut::<ActiveConnections>();
        if active {
            global_state.add(connection_id);
        } else {
            global_state.remove(connection_id);
        }
    }

    fn handle_terminal_event(&mut self, event: TerminalEvent, cx: &mut Context<Self>) {
        match event {
            TerminalEvent::Wakeup => {
                cx.emit(TerminalModelEvent::Wakeup);
            }
            TerminalEvent::TitleChanged(title) => {
                self.title = title.clone();
                cx.emit(TerminalModelEvent::TitleChanged(title));
            }
            TerminalEvent::Bell => {
                cx.emit(TerminalModelEvent::Bell);
            }
            TerminalEvent::ChildExit(code) => {
                self.child_exited = Some(code);
                cx.emit(TerminalModelEvent::ChildExit(code));
            }
            TerminalEvent::ClipboardStore(_ty, data) => {
                cx.emit(TerminalModelEvent::ClipboardStore(data));
            }
            TerminalEvent::ClipboardLoad(_ty) => {
                // 剪贴板加载由 TerminalView 处理
            }
            TerminalEvent::WorkingDirChanged(path) => {
                self.current_working_dir = Some(path.clone());
                cx.emit(TerminalModelEvent::WorkingDirChanged(path));
            }
        }
    }

    // ========== 公共 API ==========

    /// 获取 Term 的共享引用
    pub fn term(&self) -> &Arc<FairMutex<Term<GpuiEventProxy>>> {
        &self.term
    }

    /// 获取终端标题
    pub fn title(&self) -> &str {
        &self.title
    }

    /// 获取子进程退出码
    pub fn child_exited(&self) -> Option<i32> {
        self.child_exited
    }

    /// 获取连接状态
    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }

    /// 获取连接名称
    pub fn connection_name(&self) -> Option<&str> {
        self.connection_name.as_deref()
    }

    /// 获取连接 ID
    pub fn connection_id(&self) -> Option<i64> {
        self.connection_id
    }

    /// 获取当前工作目录（由 OSC 7 更新，仅 SSH 终端）
    pub fn current_working_dir(&self) -> Option<&str> {
        self.current_working_dir.as_deref()
    }

    /// 获取 SSH 连接配置（仅 SSH 终端）
    pub fn ssh_config(&self) -> Option<&SshTerminalConfig> {
        self.ssh_config.as_ref()
    }

    /// 获取连接类型
    pub fn connection_kind(&self) -> TerminalConnectionKind {
        self.connection_kind
    }

    /// 是否可以重连
    pub fn can_reconnect(&self) -> bool {
        self.ssh_config.is_some() || self.serial_params.is_some()
    }

    /// 写入数据到终端
    pub fn write(&self, data: &[u8]) {
        if let Some(ref backend) = self.backend {
            backend.write(data.to_vec());
        }
    }

    /// 调整终端大小
    pub fn resize(&mut self, cols: usize, rows: usize, pixel_width: u16, pixel_height: u16) {
        if self.cols == cols && self.rows == rows {
            return;
        }

        tracing::info!(
            "Terminal::resize: {}x{} -> {}x{}, pixel={}x{}",
            self.cols,
            self.rows,
            cols,
            rows,
            pixel_width,
            pixel_height
        );

        self.cols = cols;
        self.rows = rows;

        self.term.lock().resize(TermDimensions { cols, rows });

        if let Some(ref backend) = self.backend {
            backend.resize(TerminalSize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width,
                pixel_height,
            });
        }
    }

    /// 重新连接 SSH 或串口
    pub fn reconnect(&mut self, cx: &mut Context<Self>) {
        if let Some(config) = self.ssh_config.clone() {
            let Some(event_tx) = self.event_tx.clone() else {
                return;
            };
            let Some(event_proxy) = self.event_proxy.clone() else {
                return;
            };

            self.connection_state = ConnectionState::Connecting;

            let (disconnect_tx, disconnect_rx) = tokio::sync::oneshot::channel::<()>();
            Self::spawn_disconnect_handler(disconnect_rx, cx);
            Self::spawn_ssh_connect(
                config,
                self.term.clone(),
                event_proxy,
                event_tx,
                Some(disconnect_tx),
                self.init_commands.clone(),
                cx,
            );
        } else if let Some(params) = self.serial_params.clone() {
            let Some(event_tx) = self.event_tx.clone() else {
                return;
            };

            self.connection_state = ConnectionState::Connecting;

            let (disconnect_tx, disconnect_rx) = tokio::sync::oneshot::channel::<()>();
            Self::spawn_disconnect_handler(disconnect_rx, cx);
            Self::spawn_serial_connect(
                params,
                self.term.clone(),
                event_tx,
                Some(disconnect_tx),
                cx,
            );
        } else {
            return;
        }

        cx.emit(TerminalModelEvent::Wakeup);
    }

    /// 更新 SSH 终端的路径同步设置。
    ///
    /// 更新 `init_commands` 以影响后续新建连接或重连。
    pub fn set_sync_path_with_terminal(&mut self, enabled: bool) {
        if self.connection_kind != TerminalConnectionKind::Ssh {
            return;
        }

        self.init_commands =
            compose_ssh_init_commands(self.ssh_base_init_commands.as_deref(), enabled);
    }

    /// 关闭终端
    pub fn shutdown(&self) {
        if let Some(ref backend) = self.backend {
            backend.shutdown();
        }
    }

    // ========== 选择操作 ==========

    /// 获取选中的文本
    pub fn selection_text(&self) -> Option<String> {
        self.term.lock().selection_to_string()
    }

    /// 清除选择
    pub fn clear_selection(&mut self) {
        self.term.lock().selection = None;
    }

    /// 全选
    pub fn select_all(&mut self) {
        let mut term = self.term.lock();
        let start = AlacPoint::new(Line(-(term.history_size() as i32)), Column(0));
        let end = AlacPoint::new(
            Line(term.screen_lines() as i32 - 1),
            Column(term.columns() - 1),
        );
        term.selection = Some(Selection::new(SelectionType::Simple, start, Side::Left));
        if let Some(selection) = &mut term.selection {
            selection.update(end, Side::Right);
        }
    }

    /// 开始选择
    pub fn start_selection(&mut self, selection_type: SelectionType, point: AlacPoint, side: Side) {
        let mut term = self.term.lock();
        let point_with_offset = AlacPoint::new(
            point.line - term.grid().display_offset() as i32,
            point.column,
        );
        term.selection = Some(Selection::new(selection_type, point_with_offset, side));
    }

    /// 更新选择
    pub fn update_selection(&mut self, point: AlacPoint, side: Side) {
        let mut term = self.term.lock();
        let point_with_offset = AlacPoint::new(
            point.line - term.grid().display_offset() as i32,
            point.column,
        );
        if let Some(selection) = &mut term.selection {
            selection.update(point_with_offset, side);
        }
    }

    // ========== 滚动操作 ==========

    /// 滚动终端
    pub fn scroll(&mut self, delta: i32) {
        self.term
            .lock()
            .scroll_display(alacritty_terminal::grid::Scroll::Delta(delta));
    }

    /// 获取滚动代理（供视图层的滚动条使用）
    pub fn scroll_proxy(&self) -> TerminalScrollProxy {
        TerminalScrollProxy {
            term: self.term.clone(),
            event_tx: self.event_tx.clone(),
        }
    }

    // ========== Vi 模式 ==========

    /// 切换 Vi 模式
    pub fn toggle_vi_mode(&mut self) {
        self.term.lock().toggle_vi_mode();
    }

    /// 是否处于 Vi 模式
    pub fn in_vi_mode(&self) -> bool {
        self.term.lock().mode().contains(TermMode::VI)
    }

    /// 获取终端模式
    pub fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }
}

impl EventEmitter<TerminalModelEvent> for Terminal {}

#[cfg(test)]
mod tests {
    use super::{
        build_cd_command, build_ssh_base_init_commands, build_ssh_init_commands,
        compose_ssh_init_commands, resolve_default_windows_shell_from_env, shell_escape_arg,
        OSC7_PROMPT_COMMAND,
    };
    use std::fs;

    #[test]
    fn shell_escape_arg_handles_single_quote() {
        let escaped = shell_escape_arg("a'b");
        assert_eq!(escaped, "'a'\"'\"'b'");
    }

    #[test]
    fn build_cd_command_escapes_injection_chars() {
        let cmd = build_cd_command("dir; rm -rf /");
        assert_eq!(cmd, "cd -- 'dir; rm -rf /'");
    }

    #[test]
    fn build_cd_command_escapes_newline() {
        let cmd = build_cd_command("a\nb");
        assert_eq!(cmd, "cd -- 'a\nb'");
    }

    #[test]
    fn build_ssh_init_commands_respects_sync_path_switch() {
        let enabled = build_ssh_init_commands(None, Some("/tmp"), Some("echo ready"), true)
            .expect("启用路径同步时应生成初始化命令");
        assert!(enabled.contains(OSC7_PROMPT_COMMAND));

        let disabled = build_ssh_init_commands(None, Some("/tmp"), Some("echo ready"), false)
            .expect("禁用路径同步时仍应保留其它初始化命令");
        assert!(!disabled.contains(OSC7_PROMPT_COMMAND));
        assert!(disabled.contains("echo ready"));
    }

    #[test]
    fn build_ssh_base_init_commands_prioritizes_explicit_working_dir() {
        let commands =
            build_ssh_base_init_commands(Some("/workspace"), Some("/default"), Some("echo ready"))
                .expect("显式工作目录应生成初始化命令");

        assert!(commands.contains("cd -- '/workspace'"));
        assert!(!commands.contains("/default"));
        assert!(!commands.contains("echo ready"));
    }

    #[test]
    fn compose_ssh_init_commands_supports_sync_only_mode() {
        let commands = compose_ssh_init_commands(None, true).expect("启用同步时应仅注入 OSC7");
        assert_eq!(commands, OSC7_PROMPT_COMMAND);

        assert!(
            compose_ssh_init_commands(None, false).is_none(),
            "无基础命令且关闭同步时不应生成初始化命令"
        );
    }

    #[test]
    fn resolve_default_windows_shell_prefers_pwsh_from_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("onetcli-terminal-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("应创建临时目录");

        let pwsh = temp_dir.join("pwsh.exe");
        let cmd = temp_dir.join("cmd.exe");
        fs::write(&pwsh, b"").expect("应创建 pwsh 占位文件");
        fs::write(&cmd, b"").expect("应创建 cmd 占位文件");

        let path_env = std::ffi::OsString::from(temp_dir.as_os_str());
        let resolved = resolve_default_windows_shell_from_env(
            Some(path_env.as_os_str()),
            None,
            Some(cmd.as_os_str()),
        );

        assert_eq!(resolved, pwsh.to_string_lossy());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn resolve_default_windows_shell_falls_back_to_comspec() {
        let temp_dir = std::env::temp_dir().join(format!(
            "onetcli-terminal-test-comspec-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).expect("应创建临时目录");

        let cmd = temp_dir.join("cmd.exe");
        fs::write(&cmd, b"").expect("应创建 cmd 占位文件");

        let resolved = resolve_default_windows_shell_from_env(None, None, Some(cmd.as_os_str()));

        assert_eq!(resolved, cmd.to_string_lossy());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct TermDimensions {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}
