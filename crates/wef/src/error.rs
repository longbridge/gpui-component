/// Error type.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    /// Failed to load the CEF framework.
    #[cfg(target_os = "macos")]
    #[error("failed to load CEF library")]
    LoadLibrary,
    /// Failed to create the sandbox context.
    #[cfg(target_os = "macos")]
    #[error("failed to create the sandbox context")]
    SandboxContextCreate,
    /// Failed to initialize the CEF browser process.
    #[error("failed to initialize the CEF browser process")]
    InitializeBrowserProcess,
}
