//! Events for the WebView.

use wef::{Frame, LogSeverity};

/// Emitted when the browser is created.
#[derive(Debug)]
pub struct CreatedEvent;

/// Emitted when the address of the frame changes.
#[derive(Debug)]
pub struct AddressChangedEvent {
    /// The frame object.
    pub frame: Frame,
    /// The new URL.
    pub url: String,
}

/// Emitted when the title changes.
#[derive(Debug)]
pub struct TitleChangedEvent {
    /// The new title.
    pub title: String,
}

/// Emitted when the browser is about to display a tooltip.
#[derive(Debug)]
pub struct TooltipEvent {
    /// The tooltip text.
    pub text: String,
}

/// Emitted when the browser receives a status message.
#[derive(Debug)]
pub struct StatusMessageEvent {
    /// The status message text.
    pub text: String,
}

/// Emitted when the browser receives a console message.
#[derive(Debug)]
pub struct ConsoleMessageEvent {
    /// The console message text.
    pub message: String,
    /// The log level.
    pub level: LogSeverity,
    /// The source code file where the message is sent.
    pub source: String,
    /// The line number in the source code file.
    pub line_number: i32,
}

/// Emitted when preparing to open a popup browser window.
#[derive(Debug)]
pub struct BeforePopupEvent {
    /// The URL of the popup window.
    pub url: String,
}

/// Emitted when the overall page loading progress changes.
#[derive(Debug)]
pub struct LoadingProgressChangedEvent {
    /// Ranges from 0.0 to 1.0.
    pub progress: f32,
}

/// Emitted when the loading state changes.
#[derive(Debug)]
pub struct LoadingStateChangedEvent {
    /// Whether the browser is loading a page.
    pub is_loading: bool,
    /// Whether the browser can go back in history.
    pub can_go_back: bool,
    /// Whether the browser can go forward in history.
    pub can_go_forward: bool,
}

/// Emitted when the browser starts loading a page.
#[derive(Debug)]
pub struct LoadStartEvent {
    /// The frame object.
    pub frame: Frame,
}

/// Emitted when the browser finishes loading a page.
#[derive(Debug)]
pub struct LoadEndEvent {
    /// The frame object.
    pub frame: Frame,
}

/// Emitted when the browser fails to load a page.
#[derive(Debug)]
pub struct LoadErrorEvent {
    /// The frame object.
    pub frame: Frame,
    /// The error text.
    pub error_text: String,
    /// The uRL that failed to load.
    pub failed_url: String,
}
