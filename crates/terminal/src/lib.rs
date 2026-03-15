pub mod pty_backend;
pub mod serial_backend;
pub mod ssh_backend;
pub mod terminal;
pub mod types;

pub use pty_backend::{GpuiEventProxy, TerminalEvent};
pub use serial_backend::SerialBackend;
pub use ssh_backend::SshBackend;
pub use terminal::TerminalScrollProxy;
pub use types::{LocalConfig, TerminalBackend, TerminalSize};
