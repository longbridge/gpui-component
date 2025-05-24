/// Error type for function calls.
#[derive(Debug, thiserror::Error)]
pub enum CallFunctionError {
    /// Invalid number of arguments.
    #[error("Invalid number of arguments, expected {expected}, got {actual}")]
    InvalidNumberOfArguments {
        /// Expected number of arguments.
        expected: usize,
        /// Actual number of arguments.
        actual: usize,
    },
    /// Invalid argument type.
    #[error("Invalid argument {arg_name}: {error}")]
    InvalidArgument {
        /// Argument name.
        arg_name: String,
        /// Error message.
        error: String,
    },
    /// Function not found.
    #[error("Function not found: {0}")]
    NotFound(String),
    /// Other errors.
    #[error("{0}")]
    Other(String),
}
