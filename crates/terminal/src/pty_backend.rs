use alacritty_terminal::event::{Event as AlacTermEvent, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{ClipboardType, Term};
use alacritty_terminal::tty::{self, Options as PtyOptions};
use std::borrow::Cow;
use std::sync::Arc;
use std::thread;
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
    /// 远程工作目录变更（OSC 7）
    WorkingDirChanged(String),
}

/// Commands from UI layer to PTY backend
pub enum PtyCommand {
    Write(Vec<u8>),
    Resize(TerminalSize),
    Shutdown,
}

/// 用于将数据写回 PTY/SSH 通道的回写通道
///
/// 当 alacritty_terminal 处理 DA 查询等序列时，会生成 PtyWrite 事件，
/// 需要通过此通道将响应写回终端。
#[derive(Clone)]
enum PtyWriteBack {
    /// 本地 PTY：通过 EventLoopSender 写回
    Local(EventLoopSender),
    /// SSH：通过 UnboundedSender 写回
    Ssh(UnboundedSender<Vec<u8>>),
}

impl PtyWriteBack {
    fn write(&self, data: Vec<u8>) {
        match self {
            PtyWriteBack::Local(sender) => {
                let _ = sender.send(Msg::Input(Cow::Owned(data)));
            }
            PtyWriteBack::Ssh(sender) => {
                let _ = sender.send(data);
            }
        }
    }
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
    pub fn new(
        term: Arc<FairMutex<Term<GpuiEventProxy>>>,
        event_proxy: GpuiEventProxy,
        pty_options: PtyOptions,
    ) -> anyhow::Result<Self> {
        let window_size = WindowSize {
            num_lines: 24,
            num_cols: 80,
            cell_width: 8,
            cell_height: 18,
        };

        tracing::debug!(
            "LocalPtyBackend::new: 初始尺寸 {}x{}, cell={}x{}",
            window_size.num_cols,
            window_size.num_lines,
            window_size.cell_width,
            window_size.cell_height
        );

        let pty = tty::new(&pty_options, window_size, 0)?;
        let event_loop = EventLoop::new(term, event_proxy.clone(), pty, true, false)?;
        let event_loop_sender = event_loop.channel();

        // 设置 PtyWrite 回写通道，使 DA 等终端响应能写回 PTY
        event_proxy.set_write_back(PtyWriteBack::Local(event_loop_sender.clone()));

        let handle = thread::spawn(move || {
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
        tracing::debug!(
            "LocalPtyBackend::resize: {}x{}, cell={}x{}, pixel={}x{}",
            window_size.num_cols,
            window_size.num_lines,
            window_size.cell_width,
            window_size.cell_height,
            size.pixel_width,
            size.pixel_height
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
/// 将 alacritty 事件转换为 TerminalEvent 并发送，
/// 同时处理 PtyWrite 等需要回写 PTY 的事件
#[derive(Clone)]
pub struct GpuiEventProxy {
    event_tx: UnboundedSender<TerminalEvent>,
    /// PtyWrite 回写通道（在后端创建后设置）
    write_back: Arc<std::sync::Mutex<Option<PtyWriteBack>>>,
}

impl GpuiEventProxy {
    pub fn new(event_tx: UnboundedSender<TerminalEvent>) -> Self {
        Self {
            event_tx,
            write_back: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// 设置回写通道
    fn set_write_back(&self, wb: PtyWriteBack) {
        *self.write_back.lock().unwrap() = Some(wb);
    }

    /// 设置 SSH 回写通道
    pub(crate) fn set_ssh_write_back(&self, sender: UnboundedSender<Vec<u8>>) {
        self.set_write_back(PtyWriteBack::Ssh(sender));
    }

    fn write_back(&self, data: Vec<u8>) {
        if let Some(wb) = self.write_back.lock().unwrap().as_ref() {
            wb.write(data);
        }
    }
}

impl EventListener for GpuiEventProxy {
    fn send_event(&self, event: AlacTermEvent) {
        let terminal_event = match event {
            AlacTermEvent::PtyWrite(text) => {
                self.write_back(text.into_bytes());
                return;
            }
            AlacTermEvent::ColorRequest(_index, format_fn) => {
                let text = format_fn(alacritty_terminal::vte::ansi::Rgb { r: 0, g: 0, b: 0 });
                self.write_back(text.into_bytes());
                return;
            }
            AlacTermEvent::TextAreaSizeRequest(format_fn) => {
                let text = format_fn(WindowSize {
                    num_lines: 24,
                    num_cols: 80,
                    cell_width: 8,
                    cell_height: 18,
                });
                self.write_back(text.into_bytes());
                return;
            }
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
