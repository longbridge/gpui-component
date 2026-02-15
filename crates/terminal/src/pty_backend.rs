use alacritty_terminal::event::{Event as AlacTermEvent, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{ClipboardType, Term};
use alacritty_terminal::tty::{self, Options as PtyOptions};
use std::borrow::Cow;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::mpsc::UnboundedSender;

use crate::{TerminalBackend, TerminalSize};

/// 终端事件类型
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// 终端内容已更新，需要重新渲染
    Wakeup,
    /// 终端标题已更改
    TitleChanged(String),
    /// 终端响铃
    Bell,
    /// 子进程已退出
    ChildExit(i32),
    /// 终端程序请求存储到剪贴板
    ClipboardStore(ClipboardType, String),
    /// 终端程序请求从剪贴板加载
    ClipboardLoad(ClipboardType),
}

/// Commands from UI layer to PTY backend
pub enum PtyCommand {
    Write(Vec<u8>),
    Resize(TerminalSize),
    Shutdown,
}

/// Local PTY backend using alacritty_terminal's EventLoop
///
/// EventLoop runs in background thread:
/// 1. Reads data from local PTY
/// 2. Parses ANSI sequences and updates Term grid
/// 3. Sends Wakeup event via EventListener
pub struct LocalPtyBackend {
    event_loop_sender: EventLoopSender,
    _event_loop_handle: JoinHandle<()>,
}

impl LocalPtyBackend {
    pub fn new<T: EventListener + Clone + Send + 'static>(
        term: Arc<FairMutex<Term<T>>>,
        event_proxy: T,
        pty_options: PtyOptions,
    ) -> anyhow::Result<Self> {
        let window_size = WindowSize {
            num_lines: 24,
            num_cols: 80,
            cell_width: 8,
            cell_height: 18,
        };

        tracing::info!(
            "LocalPtyBackend::new: 初始尺寸 {}x{}, cell={}x{}",
            window_size.num_cols, window_size.num_lines,
            window_size.cell_width, window_size.cell_height
        );

        let pty = tty::new(&pty_options, window_size, 0)?;
        let event_loop = EventLoop::new(term, event_proxy, pty, true, false)?;
        let event_loop_sender = event_loop.channel();

        let handle = std::thread::spawn(move || {
            let _ = event_loop.spawn().join();
        });

        Ok(Self {
            event_loop_sender,
            _event_loop_handle: handle,
        })
    }

    pub fn write(&self, data: Vec<u8>) {
        let _ = self.event_loop_sender.send(Msg::Input(Cow::Owned(data)));
    }

    pub fn resize(&self, size: TerminalSize) {
        let window_size = WindowSize {
            num_lines: size.rows,
            num_cols: size.cols,
            cell_width: if size.cols > 0 {
                size.pixel_width / size.cols
            } else {
                8
            },
            cell_height: if size.rows > 0 {
                size.pixel_height / size.rows
            } else {
                18
            },
        };
        tracing::info!(
            "LocalPtyBackend::resize: {}x{}, cell={}x{}, pixel={}x{}",
            window_size.num_cols, window_size.num_lines,
            window_size.cell_width, window_size.cell_height,
            size.pixel_width, size.pixel_height
        );
        let _ = self.event_loop_sender.send(Msg::Resize(window_size));
    }

    pub fn shutdown(&self) {
        let _ = self.event_loop_sender.send(Msg::Shutdown);
    }
}

impl TerminalBackend for LocalPtyBackend {
    fn write(&self, data: Vec<u8>) {
        let _ = self.event_loop_sender.send(Msg::Input(Cow::Owned(data)));
    }

    fn resize(&self, size: TerminalSize) {
        let window_size = WindowSize {
            num_lines: size.rows,
            num_cols: size.cols,
            cell_width: if size.cols > 0 {
                size.pixel_width / size.cols
            } else {
                8
            },
            cell_height: if size.rows > 0 {
                size.pixel_height / size.rows
            } else {
                18
            },
        };
        let _ = self.event_loop_sender.send(Msg::Resize(window_size));
    }

    fn shutdown(&self) {
        LocalPtyBackend::shutdown(self);
    }
}

/// GPUI Event proxy for alacritty_terminal
/// 将 alacritty 事件转换为 TerminalEvent 并发送
#[derive(Clone)]
pub struct GpuiEventProxy {
    event_tx: UnboundedSender<TerminalEvent>,
}

impl GpuiEventProxy {
    pub fn new(event_tx: UnboundedSender<TerminalEvent>) -> Self {
        Self { event_tx }
    }
}

impl EventListener for GpuiEventProxy {
    fn send_event(&self, event: AlacTermEvent) {
        let terminal_event = match event {
            AlacTermEvent::Wakeup => TerminalEvent::Wakeup,
            AlacTermEvent::Title(title) => TerminalEvent::TitleChanged(title),
            AlacTermEvent::Bell => TerminalEvent::Bell,
            AlacTermEvent::ClipboardStore(ty, data) => TerminalEvent::ClipboardStore(ty, data),
            AlacTermEvent::ClipboardLoad(ty, _) => TerminalEvent::ClipboardLoad(ty),
            AlacTermEvent::Exit => TerminalEvent::ChildExit(0),
            _ => return,
        };
        let _ = self.event_tx.send(terminal_event);
    }
}
