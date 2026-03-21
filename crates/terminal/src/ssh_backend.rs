use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

use ssh::{ChannelEvent, PtyConfig, RusshClient, SshChannel, SshClient, SshConnectConfig};

use crate::pty_backend::{GpuiEventProxy, TerminalEvent};
use crate::{TerminalBackend, TerminalSize};

/// 从原始数据中提取 OSC 7 路径
///
/// OSC 7 格式: `\x1b]7;file://hostname/path\x07` 或 `\x1b]7;file://hostname/path\x1b\\`
/// 提取 `file://` URI 中的路径部分（跳过 hostname），并对 %XX 编码进行解码。
fn extract_osc7_path(data: &[u8]) -> Option<String> {
    // 在数据中查找 OSC 7 序列起始标记 "\x1b]7;"
    let start_marker = b"\x1b]7;";
    let start_pos = data
        .windows(start_marker.len())
        .position(|w| w == start_marker)?;
    let uri_start = start_pos + start_marker.len();

    if uri_start >= data.len() {
        return None;
    }

    // 查找终止符: BEL (\x07) 或 ST (\x1b\\)
    let remaining = &data[uri_start..];
    let uri_end = remaining
        .iter()
        .position(|&b| b == 0x07)
        .or_else(|| remaining.windows(2).position(|w| w == b"\x1b\\"))?;

    let uri_bytes = &remaining[..uri_end];
    let uri = std::str::from_utf8(uri_bytes).ok()?;

    // 解析 file:// URI，提取路径部分（跳过 hostname）
    let path = if let Some(rest) = uri.strip_prefix("file://") {
        // file://hostname/path → 找到第一个 '/' 即路径开始
        if let Some(slash_pos) = rest.find('/') {
            &rest[slash_pos..]
        } else {
            return None;
        }
    } else {
        // 不是 file:// URI，忽略
        return None;
    };

    // URL decode: 将 %XX 编码转为实际字符
    let decoded = percent_decode(path)?;
    if decoded.is_empty() || !decoded.starts_with('/') {
        return None;
    }
    Some(decoded)
}

/// 简单的 percent-decode 实现，将 %XX 编码转为实际字节
fn percent_decode(input: &str) -> Option<String> {
    let mut result = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_digit(bytes[i + 1])?;
            let lo = hex_digit(bytes[i + 2])?;
            result.push(hi << 4 | lo);
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(result).ok()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

enum SshCommand {
    Write(Vec<u8>),
    Resize(TerminalSize),
    Shutdown,
}

pub struct SshBackend {
    command_tx: UnboundedSender<SshCommand>,
}

impl SshBackend {
    pub async fn connect(
        config: SshConnectConfig,
        pty_config: PtyConfig,
        term: Arc<FairMutex<Term<GpuiEventProxy>>>,
        event_proxy: GpuiEventProxy,
        event_tx: UnboundedSender<TerminalEvent>,
        notify_tx: UnboundedSender<()>,
        on_disconnect: Option<UnboundedSender<()>>,
        init_commands: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut client = RusshClient::connect(config).await?;
        let mut channel = client.open_channel().await?;

        channel.request_pty(&pty_config).await?;
        channel.request_shell().await?;

        // 有初始化命令时直接写入 shell
        if let Some(ref commands) = init_commands {
            for line in commands.lines() {
                if !line.trim().is_empty() {
                    let mut data = line.as_bytes().to_vec();
                    data.push(b'\n');
                    channel.send_data(&data).await?;
                }
            }
        }

        let (command_tx, mut command_rx) = unbounded_channel::<SshCommand>();

        // 创建 PtyWrite 回写通道
        let (pty_write_tx, mut pty_write_rx) = unbounded_channel::<Vec<u8>>();
        event_proxy.set_ssh_write_back(pty_write_tx);

        tokio::spawn(async move {
            let mut shutdown = false;
            let mut processor: Processor<StdSyncHandler> = Processor::new();

            loop {
                tokio::select! {
                    biased;
                    Some(cmd) = command_rx.recv() => {
                        match cmd {
                            SshCommand::Write(data) => {
                                let send_result = tokio::time::timeout(
                                    Duration::from_secs(30),
                                    channel.send_data(&data)
                                ).await;
                                if send_result.is_err() || send_result.is_ok_and(|r| r.is_err()) {
                                    break;
                                }
                            }
                            SshCommand::Resize(size) => {
                                let _ = channel.resize_pty(size.cols as u32, size.rows as u32).await;
                            }
                            SshCommand::Shutdown => {
                                shutdown = true;
                                let _ = channel.close().await;
                                break;
                            }
                        }
                    }
                    Some(data) = pty_write_rx.recv() => {
                        // DA 响应等回写数据
                        let send_result = tokio::time::timeout(
                            Duration::from_secs(30),
                            channel.send_data(&data)
                        ).await;
                        if send_result.is_err() || send_result.is_ok_and(|r| r.is_err()) {
                            break;
                        }
                    }
                    event = channel.recv() => {
                        match event {
                            Some(ChannelEvent::Data(data)) => {
                                if let Some(path) = extract_osc7_path(&data) {
                                    let _ = event_tx.send(TerminalEvent::WorkingDirChanged(path));
                                }
                                processor.advance(&mut *term.lock(), &data);
                                let _ = notify_tx.send(());
                            }
                            Some(ChannelEvent::ExtendedData { data, .. }) => {
                                if let Some(path) = extract_osc7_path(&data) {
                                    let _ = event_tx.send(TerminalEvent::WorkingDirChanged(path));
                                }
                                processor.advance(&mut *term.lock(), &data);
                                let _ = notify_tx.send(());
                            }
                            Some(ChannelEvent::Eof) | Some(ChannelEvent::Close) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            if !shutdown {
                let _ = client.disconnect().await;
            }
            if let Some(tx) = on_disconnect {
                let _ = tx.send(());
            }
        });

        Ok(Self { command_tx })
    }
}

impl TerminalBackend for SshBackend {
    fn write(&self, data: Vec<u8>) {
        let _ = self.command_tx.send(SshCommand::Write(data));
    }

    fn resize(&self, size: TerminalSize) {
        tracing::info!(
            "SshBackend::resize: 发送 resize 命令到远程 PTY: {}x{}",
            size.cols,
            size.rows
        );
        let _ = self.command_tx.send(SshCommand::Resize(size));
    }

    fn shutdown(&self) {
        let _ = self.command_tx.send(SshCommand::Shutdown);
    }
}
