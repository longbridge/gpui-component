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
