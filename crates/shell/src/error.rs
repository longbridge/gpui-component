//! The engine-neutral error type.
//!
//! Style resolution, value coercion, and the spec arena all produce errors that
//! must reach the script author as a script-level exception. They must not name
//! a particular VM, or the whole layer above `engine/` would stop being
//! engine-independent. Each engine converts this into its own error at the
//! boundary.

/// An error that will surface to the script author.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellError(String);

impl ShellError {
    pub fn runtime(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ShellError {}

pub type Result<T> = std::result::Result<T, ShellError>;
