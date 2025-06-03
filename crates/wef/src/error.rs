/// Error type.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    /// Failed to load the CEF framework.
    ///
    /// Only used on macOS.
    #[error("failed to load CEF library")]
    LoadLibrary,
    /// Failed to initialize the CEF browser process.
    #[error("failed to initialize the CEF browser process")]
    InitializeBrowserProcess,
}
