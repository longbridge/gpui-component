use crate::{
    highlighter::HighlightTheme,
    input::{InputState, LineColumn},
};
use gpui::{px, App, HighlightStyle, Hsla, SharedString, UnderlineStyle};
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

        let start_line_str = state.text.slice(start_line.range.clone());

        let Some(end_line) = state
            .text_wrapper
            .lines
            .get(self.end.line.saturating_sub(1))
        else {
            return;
        };
        let end_line_str = state.text.slice(end_line.range.clone());

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
    pub(super) fn bg(&self, theme: &HighlightTheme, cx: &App) -> Hsla {
        match self {
            Self::Error => theme.style.status.error_background(cx),
            Self::Warning => theme.style.status.warning_background(cx),
            Self::Info => theme.style.status.info_background(cx),
            Self::Hint => theme.style.status.hint_background(cx),
        }
    }

    pub(super) fn fg(&self, theme: &HighlightTheme, cx: &App) -> Hsla {
        match self {
            Self::Error => theme.style.status.error(cx),
            Self::Warning => theme.style.status.warning(cx),
            Self::Info => theme.style.status.info(cx),
            Self::Hint => theme.style.status.hint(cx),
        }
    }

    pub(super) fn border(&self, theme: &HighlightTheme, cx: &App) -> Hsla {
        match self {
            Self::Error => theme.style.status.error_border(cx),
            Self::Warning => theme.style.status.warning_border(cx),
            Self::Info => theme.style.status.info_border(cx),
            Self::Hint => theme.style.status.hint_border(cx),
        }
    }

    pub(super) fn highlight_style(&self, theme: &HighlightTheme, cx: &App) -> HighlightStyle {
        let color = match self {
            Self::Error => Some(theme.style.status.error(cx)),
            Self::Warning => Some(theme.style.status.warning(cx)),
            Self::Info => Some(theme.style.status.info(cx)),
            Self::Hint => Some(theme.style.status.hint(cx)),
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
