use crate::{highlighter::HighlightTheme, input::InputState};
use gpui::{px, HighlightStyle, Hsla, SharedString, UnderlineStyle};
use itertools::Itertools;
use std::ops::Range;

/// Marker represents a diagnostic message, such as an error or warning, in the code editor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Marker {
    pub severity: MarkerSeverity,
    pub start: LineColumn,
    pub end: LineColumn,
    pub(super) range: Option<Range<usize>>,
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
            range: None,
        }
    }

    /// Prepare the marker to convert line, column to byte offsets.
    pub(super) fn prepare(&mut self, state: &InputState) {
        let Some(start_line) = state
            .text_wrapper
            .lines
            .get(self.start.line.saturating_sub(1))
        else {
            return;
        };

        let Some(start_line_str) = state.text.get(start_line.range.clone()) else {
            return;
        };

        let Some(end_line) = state
            .text_wrapper
            .lines
            .get(self.end.line.saturating_sub(1))
        else {
            return;
        };
        let Some(end_line_str) = state.text.get(end_line.range.clone()) else {
            return;
        };

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

        self.range = Some(start_byte..end_byte);
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
    pub(super) fn bg(&self, theme: &HighlightTheme) -> Hsla {
        match self {
            Self::Error => theme.style.status.error_background(),
            Self::Warning => theme.style.status.warning_background(),
            Self::Info => theme.style.status.info_background(),
            Self::Hint => theme.style.status.hint_background(),
        }
    }

    pub(super) fn fg(&self, theme: &HighlightTheme) -> Hsla {
        match self {
            Self::Error => theme.style.status.error(),
            Self::Warning => theme.style.status.warning(),
            Self::Info => theme.style.status.info(),
            Self::Hint => theme.style.status.hint(),
        }
    }

    pub(super) fn border(&self, theme: &HighlightTheme) -> Hsla {
        match self {
            Self::Error => theme.style.status.error_border(),
            Self::Warning => theme.style.status.warning_border(),
            Self::Info => theme.style.status.info_border(),
            Self::Hint => theme.style.status.hint_border(),
        }
    }

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
