use gpui::SharedString;
use std::{fmt::Display, ops::Range};

/// Severity of the [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Severity {
    #[default]
    Hint,
    Error,
    Warning,
    Info,
}

impl From<&str> for Severity {
    fn from(value: &str) -> Self {
        match value {
            "error" => Self::Error,
            "warning" => Self::Warning,
            "info" => Self::Info,
            "hint" => Self::Hint,
            _ => Self::Info, // Default to Info if unknown
        }
    }
}

impl Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
            Severity::Hint => write!(f, "hint"),
        }
    }
}

/// Diagnostic represents a single error or warning message with a severity level and a range.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Diagnostic {
    pub message: SharedString,
    pub severity: Severity,
    pub range: Range<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_from_str() {
        assert_eq!(Severity::from("error"), Severity::Error);
        assert_eq!(Severity::from("warning"), Severity::Warning);
        assert_eq!(Severity::from("info"), Severity::Info);
        assert_eq!(Severity::from("hint"), Severity::Hint);
        assert_eq!(Severity::from("unknown"), Severity::Info);

        assert_eq!(format!("{}", Severity::Error), "error");
        assert_eq!(format!("{}", Severity::Warning), "warning");
        assert_eq!(format!("{}", Severity::Info), "info");
        assert_eq!(format!("{}", Severity::Hint), "hint");
    }
}
