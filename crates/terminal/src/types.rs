use serde::{Deserialize, Serialize};

/// Terminal backend trait - abstracts local PTY and SSH backends
pub trait TerminalBackend: Send {
    fn write(&self, data: Vec<u8>);
    fn resize(&self, size: TerminalSize);
    fn shutdown(&self);
}

/// Local terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    /// Shell command (default: system default shell)
    pub shell: Option<String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            shell: None,
            working_dir: None,
            env: vec![
                ("TERM".to_string(), "xterm-256color".to_string()),
                ("COLORTERM".to_string(), "truecolor".to_string()),
                ("CLICOLOR".to_string(), "1".to_string()),
                ("CLICOLOR_FORCE".to_string(), "1".to_string()),
            ],
        }
    }
}

/// Terminal dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}
