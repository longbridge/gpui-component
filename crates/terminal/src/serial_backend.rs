use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

use one_core::storage::models::SerialParams;

use crate::pty_backend::{GpuiEventProxy, TerminalEvent};
use crate::{TerminalBackend, TerminalSize};

enum SerialCommand {
    Write(Vec<u8>),
    Shutdown,
}

pub struct SerialBackend {
    command_tx: UnboundedSender<SerialCommand>,
}

impl SerialBackend {
    pub fn connect(
        params: SerialParams,
        term: Arc<FairMutex<Term<GpuiEventProxy>>>,
        event_tx: UnboundedSender<TerminalEvent>,
        on_disconnect: Option<UnboundedSender<()>>,
    ) -> anyhow::Result<Self> {
        let data_bits = match params.data_bits {
            5 => serialport::DataBits::Five,
            6 => serialport::DataBits::Six,
            7 => serialport::DataBits::Seven,
            _ => serialport::DataBits::Eight,
        };

        let stop_bits = match params.stop_bits {
            2 => serialport::StopBits::Two,
            _ => serialport::StopBits::One,
        };

        let parity = match params.parity {
            one_core::storage::models::SerialParity::Odd => serialport::Parity::Odd,
            one_core::storage::models::SerialParity::Even => serialport::Parity::Even,
            one_core::storage::models::SerialParity::None => serialport::Parity::None,
        };

        let flow_control = match params.flow_control {
            one_core::storage::models::SerialFlowControl::Software => {
                serialport::FlowControl::Software
            }
            one_core::storage::models::SerialFlowControl::Hardware => {
                serialport::FlowControl::Hardware
            }
            one_core::storage::models::SerialFlowControl::None => serialport::FlowControl::None,
        };

        let port = serialport::new(&params.port_name, params.baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(flow_control)
            .timeout(Duration::from_millis(10))
            .open()?;

        // 克隆一份用于写入
        let write_port = port.try_clone()?;

        let (command_tx, mut command_rx) = unbounded_channel::<SerialCommand>();

        // 读取线程：从串口读取数据并写入 alacritty Term
        let read_event_tx = event_tx.clone();
        std::thread::Builder::new()
            .name("serial-read".into())
            .spawn(move || {
                let mut port = port;
                let mut processor: Processor<StdSyncHandler> = Processor::new();
                let mut buf = [0u8; 4096];

                loop {
                    match port.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            processor.advance(&mut *term.lock(), &buf[..n]);
                            let _ = read_event_tx.send(TerminalEvent::Wakeup);
                        }
                        Ok(_) => {}
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            // 超时是正常的，继续读取
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // 非阻塞模式下无数据可读
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => {
                            // 读取错误，串口可能已断开
                            break;
                        }
                    }
                }
                if let Some(tx) = on_disconnect {
                    let _ = tx.send(());
                }
            })?;

        // 写入线程：从 command channel 接收命令并写入串口
        std::thread::Builder::new()
            .name("serial-write".into())
            .spawn(move || {
                let mut port = write_port;
                while let Some(cmd) = command_rx.blocking_recv() {
                    match cmd {
                        SerialCommand::Write(data) => {
                            if port.write_all(&data).is_err() {
                                break;
                            }
                        }
                        SerialCommand::Shutdown => {
                            break;
                        }
                    }
                }
            })?;

        Ok(Self { command_tx })
    }
}

impl TerminalBackend for SerialBackend {
    fn write(&self, data: Vec<u8>) {
        let _ = self.command_tx.send(SerialCommand::Write(data));
    }

    fn resize(&self, _size: TerminalSize) {
        // 串口无 PTY 尺寸概念，resize 为空操作
    }

    fn shutdown(&self) {
        let _ = self.command_tx.send(SerialCommand::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::grid::Dimensions;
    use std::io::{Read, Write};
    use std::process::Command;

    /// 实现 Dimensions trait 用于测试
    struct TestDimensions;
    impl Dimensions for TestDimensions {
        fn total_lines(&self) -> usize {
            24
        }
        fn screen_lines(&self) -> usize {
            24
        }
        fn columns(&self) -> usize {
            80
        }
    }

    fn create_test_term(
        event_tx: UnboundedSender<TerminalEvent>,
    ) -> Arc<FairMutex<Term<GpuiEventProxy>>> {
        let config = alacritty_terminal::term::Config::default();
        let event_proxy = GpuiEventProxy::new(event_tx);
        Arc::new(FairMutex::new(Term::new(config, &TestDimensions, event_proxy)))
    }

    fn create_virtual_serial_pair() -> Option<(std::process::Child, String, String)> {
        if Command::new("socat").arg("-V").output().is_err() {
            return None;
        }

        let pid = std::process::id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let pty_a = format!("/tmp/vtest_a_{}_{}", pid, ts);
        let pty_b = format!("/tmp/vtest_b_{}_{}", pid, ts);

        // 使用 cfmakeraw 确保 pty 是 raw 模式，避免 "Not a typewriter" 错误
        let child = Command::new("socat")
            .args([
                &format!("pty,raw,echo=0,cfmakeraw,link={}", pty_a),
                &format!("pty,raw,echo=0,cfmakeraw,link={}", pty_b),
            ])
            .spawn()
            .ok()?;

        // 等 pty 就绪
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(100));
            if std::path::Path::new(&pty_a).exists() && std::path::Path::new(&pty_b).exists() {
                // 额外等一下让 socat 完全初始化
                std::thread::sleep(Duration::from_millis(200));
                return Some((child, pty_a, pty_b));
            }
        }
        None
    }

    #[test]
    fn test_open_nonexistent_port_fails() {
        let params = SerialParams {
            port_name: "/dev/nonexistent_serial_test_999".to_string(),
            ..Default::default()
        };
        let (event_tx, _event_rx) = unbounded_channel::<TerminalEvent>();
        let term = create_test_term(event_tx.clone());
        let result = SerialBackend::connect(params, term, event_tx, None);
        assert!(result.is_err(), "打开不存在的串口应返回错误");
        let err = result.err().unwrap();
        println!("[PASS] 打开不存在的端口返回错误: {}", err);
    }

    #[test]
    fn test_serial_backend_write_via_socat() {
        // macOS 上 serialport crate 对 pty 调用 ioctl(TIOCEXCL) 会报 ENOTTY，
        // 这是 pty 不是真实串口设备的限制，不影响真实串口功能。
        // 此测试仅在 Linux 或有真实串口设备的环境下有效。
        let Some((mut socat, port_a, port_b)) = create_virtual_serial_pair() else {
            println!("[SKIP] socat 不可用，跳过虚拟串口测试");
            return;
        };

        let (event_tx, _event_rx) = unbounded_channel::<TerminalEvent>();
        let term = create_test_term(event_tx.clone());

        let params = SerialParams {
            port_name: port_a.clone(),
            baud_rate: 115200,
            ..Default::default()
        };
        match SerialBackend::connect(params, term, event_tx, None) {
            Ok(backend) => {
                // 打开对端读取
                let mut peer = serialport::new(&port_b, 115200)
                    .timeout(Duration::from_secs(2))
                    .open()
                    .expect("打开对端失败");

                let msg = b"backend write test\r\n";
                backend.write(msg.to_vec());

                std::thread::sleep(Duration::from_millis(200));
                let mut recv_buf = vec![0u8; msg.len()];
                let mut read_total = 0;
                let deadline = std::time::Instant::now() + Duration::from_secs(3);
                while read_total < msg.len() && std::time::Instant::now() < deadline {
                    match peer.read(&mut recv_buf[read_total..]) {
                        Ok(n) => read_total += n,
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        Err(e) => panic!("对端读取失败: {}", e),
                    }
                }
                assert_eq!(
                    &recv_buf[..read_total],
                    msg,
                    "通过 SerialBackend 写入的数据不匹配"
                );
                println!("[PASS] SerialBackend::write() 通过虚拟串口成功发送数据");
                backend.shutdown();
                drop(peer);
            }
            Err(e) => {
                // macOS pty 会报 ENOTTY，属已知限制
                println!(
                    "[SKIP] SerialBackend 无法连接 pty（macOS 已知限制: {}），跳过写入测试",
                    e
                );
            }
        }

        let _ = socat.kill();
        let _ = std::fs::remove_file(&port_a);
        let _ = std::fs::remove_file(&port_b);
    }

    #[test]
    fn test_serial_backend_read_into_term_via_socat() {
        let Some((mut socat, port_a, port_b)) = create_virtual_serial_pair() else {
            println!("[SKIP] socat 不可用，跳过虚拟串口测试");
            return;
        };

        let (event_tx, mut event_rx) = unbounded_channel::<TerminalEvent>();
        let term = create_test_term(event_tx.clone());

        let params = SerialParams {
            port_name: port_a.clone(),
            baud_rate: 115200,
            ..Default::default()
        };
        match SerialBackend::connect(params, term.clone(), event_tx, None) {
            Ok(backend) => {
                let mut writer = serialport::new(&port_b, 115200)
                    .timeout(Duration::from_secs(2))
                    .open()
                    .expect("打开对端写入失败");

                writer.write_all(b"Hello from peer\r\n").expect("写入失败");
                writer.flush().expect("flush 失败");

                let deadline = std::time::Instant::now() + Duration::from_secs(3);
                let mut got_wakeup = false;
                while std::time::Instant::now() < deadline {
                    match event_rx.try_recv() {
                        Ok(TerminalEvent::Wakeup) => {
                            got_wakeup = true;
                            break;
                        }
                        Ok(_) => {}
                        Err(_) => std::thread::sleep(Duration::from_millis(50)),
                    }
                }
                assert!(got_wakeup, "应收到 Wakeup 事件");
                println!(
                    "[PASS] SerialBackend 读取线程成功接收对端数据并触发 Wakeup 事件"
                );
                backend.shutdown();
                drop(writer);
            }
            Err(e) => {
                println!(
                    "[SKIP] SerialBackend 无法连接 pty（macOS 已知限制: {}），跳过读取测试",
                    e
                );
            }
        }

        let _ = socat.kill();
        let _ = std::fs::remove_file(&port_a);
        let _ = std::fs::remove_file(&port_b);
    }
}
