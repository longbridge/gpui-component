rust_i18n::i18n!("locales", fallback = "en");

pub mod addon;
pub mod blink_manager;
pub mod keys;
pub mod sidebar;
pub mod ssh_form_window;
pub mod terminal_element;
pub mod theme;
pub mod view;

pub use addon::{AddonManager, HoveredLink, SearchAddon, TerminalAddon, WebLinksAddon};
pub use blink_manager::BlinkManager;
pub use sidebar::{
    CommandHistoryPanel, SettingsPanel, SidebarPanel, TerminalSidebar, TerminalSidebarEvent,
    SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, TOOLBAR_WIDTH,
};
pub use ssh_form_window::{SshFormWindow, SshFormWindowConfig};
pub use terminal::terminal::{ConnectionState, SshTerminalConfig, Terminal, TerminalModelEvent};
pub use theme::{
    default_font_fallbacks, TerminalTheme, DEFAULT_FONT_SIZE, DEFAULT_LINE_HEIGHT_SCALE,
    MAX_FONT_SIZE, MIN_FONT_SIZE,
};
pub use view::{init, TerminalView};
