use crate::{highlighter::HighlightTheme, input::InputState};
use gpui::{px, HighlightStyle, SharedString, UnderlineStyle};
use itertools::Itertools;
use std::ops::Range;

/// Marker represents a diagnostic message, such as an error or warning, in the code editor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Marker {
    pub severity: MarkerSeverity,
    pub start: LineColumn,
    pub end: LineColumn,
    /// The message associated with the marker, typically a description of the issue.
    pub message: SharedString,
}

impl Marker {
    /// Creates a new marker with the specified severity, start and end positions, and message.
    pub fn new(
        severity: impl Into<MarkerSeverity>,
        start: impl Into<LineColumn>,
        end: impl Into<LineColumn>,
        message: impl Into<SharedString>,
    ) -> Self {
        Self {
            severity: severity.into(),
            start: start.into(),
            end: end.into(),
            message: message.into(),
        }
    }

    /// Returns the range (zero-based) of bytes in the source code that this marker covers.
    pub(super) fn byte_range(&self, state: &InputState) -> Option<Range<usize>> {
        let start_line = state
            .text_wrapper
            .lines
            .get(self.start.line.saturating_sub(1))?;
        let start_line_str = state.text.get(start_line.range.clone())?;

        let end_line = state
            .text_wrapper
            .lines
            .get(self.end.line.saturating_sub(1))?;
        let end_line_str = state.text.get(end_line.range.clone())?;

        let start_byte = start_line.range.start
            + start_line_str
                .chars()
                .take(self.start.column.saturating_sub(1))
                .counts_by(|c| c.len_utf8())
                .values()
                .sum::<usize>();
        let end_byte = end_line.range.start
            + end_line_str
                .chars()
                .take(self.end.column.saturating_sub(1))
                .counts_by(|c| c.len_utf8())
                .values()
                .sum::<usize>();

        Some(start_byte..end_byte)
    }
}

/// Line and column position (1-based) in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineColumn {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
}

impl From<(usize, usize)> for LineColumn {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0.max(1),
            column: value.1.max(1),
        }
    }
}

/// Severity of the marker.
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarkerSeverity {
    #[default]
    Hint,
    Error,
    Warning,
    Info,
}

impl From<&str> for MarkerSeverity {
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

impl MarkerSeverity {
    /// Returns the [`HighlightStyle`] for the marker severity with the given theme style.
    pub(super) fn highlight_style(&self, theme: &HighlightTheme) -> HighlightStyle {
        let color = match self {
            Self::Error => Some(theme.style.status.error()),
            Self::Warning => Some(theme.style.status.warning()),
            Self::Info => Some(theme.style.status.info()),
            Self::Hint => Some(theme.style.status.hint()),
        };

        let mut style = HighlightStyle::default();
        style.underline = Some(UnderlineStyle {
            color: color,
            thickness: px(1.),
            wavy: true,
        });

        style
    }
}
