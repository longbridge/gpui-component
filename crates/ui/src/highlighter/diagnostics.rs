use std::ops::Range;

use gpui::{px, App, HighlightStyle, Hsla, SharedString, UnderlineStyle};
use rope::Rope;

use crate::{
    input::{Position, RopeExt as _},
    ActiveTheme,
};

pub type DiagnosticRelatedInformation = lsp_types::DiagnosticRelatedInformation;
pub type CodeDescription = lsp_types::CodeDescription;
pub type RelatedInformation = lsp_types::DiagnosticRelatedInformation;
pub type DiagnosticTag = lsp_types::DiagnosticTag;

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Diagnostic {
    /// The range [`Position`] at which the message applies.
    ///
    /// This is the column, character range within a single line.
    pub range: Range<Position>,

    pub(crate) byte_range: Range<usize>,

    /// The diagnostic's severity. Can be omitted. If omitted it is up to the
    /// client to interpret diagnostics as error, warning, info or hint.
    pub severity: DiagnosticSeverity,

    /// The diagnostic's code. Can be omitted.
    pub code: Option<SharedString>,

    pub code_description: Option<CodeDescription>,

    /// A human-readable string describing the source of this
    /// diagnostic, e.g. 'typescript' or 'super lint'.
    pub source: Option<SharedString>,

    /// The diagnostic's message.
    pub message: SharedString,

    /// An array of related diagnostic information, e.g. when symbol-names within
    /// a scope collide all definitions can be marked via this property.
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,

    /// Additional metadata about the diagnostic.
    pub tags: Option<Vec<DiagnosticTag>>,

    /// A data entry field that is preserved between a `textDocument/publishDiagnostics`
    /// notification and `textDocument/codeAction` request.
    ///
    /// @since 3.16.0
    pub data: Option<serde_json::Value>,
}

impl From<lsp_types::Diagnostic> for Diagnostic {
    fn from(value: lsp_types::Diagnostic) -> Self {
        Self {
            range: Position::from(value.range.start)..Position::from(value.range.end),
            byte_range: 0..0,
            severity: value
                .severity
                .map(Into::into)
                .unwrap_or(DiagnosticSeverity::Info),
            code: value.code.map(|c| match c {
                lsp_types::NumberOrString::Number(n) => SharedString::from(n.to_string()),
                lsp_types::NumberOrString::String(s) => SharedString::from(s),
            }),
            code_description: value.code_description,
            source: value.source.map(|s| s.into()),
            message: value.message.into(),
            related_information: value.related_information,
            tags: value.tags,
            data: value.data,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticSeverity {
    #[default]
    Hint,
    Error,
    Warning,
    Info,
}

impl From<lsp_types::DiagnosticSeverity> for DiagnosticSeverity {
    fn from(value: lsp_types::DiagnosticSeverity) -> Self {
        match value {
            lsp_types::DiagnosticSeverity::ERROR => Self::Error,
            lsp_types::DiagnosticSeverity::WARNING => Self::Warning,
            lsp_types::DiagnosticSeverity::INFORMATION => Self::Info,
            lsp_types::DiagnosticSeverity::HINT => Self::Hint,
            _ => Self::Info, // Default to Info if unknown
        }
    }
}

impl DiagnosticSeverity {
    pub(crate) fn bg(&self, cx: &App) -> Hsla {
        let theme = &cx.theme().highlight_theme;

        match self {
            Self::Error => theme.style.status.error_background(cx),
            Self::Warning => theme.style.status.warning_background(cx),
            Self::Info => theme.style.status.info_background(cx),
            Self::Hint => theme.style.status.hint_background(cx),
        }
    }

    pub(crate) fn fg(&self, cx: &App) -> Hsla {
        let theme = &cx.theme().highlight_theme;

        match self {
            Self::Error => theme.style.status.error(cx),
            Self::Warning => theme.style.status.warning(cx),
            Self::Info => theme.style.status.info(cx),
            Self::Hint => theme.style.status.hint(cx),
        }
    }

    pub(crate) fn border(&self, cx: &App) -> Hsla {
        let theme = &cx.theme().highlight_theme;
        match self {
            Self::Error => theme.style.status.error_border(cx),
            Self::Warning => theme.style.status.warning_border(cx),
            Self::Info => theme.style.status.info_border(cx),
            Self::Hint => theme.style.status.hint_border(cx),
        }
    }

    pub(crate) fn highlight_style(&self, cx: &App) -> HighlightStyle {
        let theme = &cx.theme().highlight_theme;

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

impl Diagnostic {
    pub fn new(range: Range<impl Into<Position>>, message: impl Into<SharedString>) -> Self {
        Self {
            range: range.start.into()..range.end.into(),
            message: message.into(),
            ..Default::default()
        }
    }

    pub fn with_severity(mut self, severity: impl Into<DiagnosticSeverity>) -> Self {
        self.severity = severity.into();
        self
    }

    pub fn with_code(mut self, code: impl Into<SharedString>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Prepares the byte range of the diagnostic within the given text.
    pub(crate) fn prepare(&mut self, text: &Rope) {
        let start = text.position_to_offset(&self.range.start);
        let end = text.position_to_offset(&self.range.end);

        self.byte_range = start..end;
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticSet {
    text: Rope,
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSet {
    pub fn new(text: &Rope) -> Self {
        Self {
            text: text.clone(),
            diagnostics: Vec::new(),
        }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        let mut diagnostic = diagnostic;
        diagnostic.prepare(&self.text);

        self.diagnostics.push(diagnostic);
    }

    pub fn extend<I>(&mut self, diagnostics: I)
    where
        I: IntoIterator<Item = Diagnostic>,
    {
        for diagnostic in diagnostics {
            self.push(diagnostic);
        }
    }

    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Todo impl for_range
    pub(crate) fn for_offset(&self, offset: usize) -> Option<&Diagnostic> {
        for diagnostic in self.diagnostics.iter() {
            if diagnostic.byte_range.contains(&offset) {
                return Some(diagnostic);
            }
        }

        None
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }
}
