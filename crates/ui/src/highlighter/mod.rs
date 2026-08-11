// Diagnostics are editor behavior/data and live in Base. Keep this re-export
// so existing `gpui_component::highlighter` paths remain source-compatible.
pub use gpui_base::highlighter::*;

use gpui::{App, Hsla};

use crate::ActiveTheme as _;

pub(crate) fn diagnostic_background(severity: DiagnosticSeverity, cx: &App) -> Hsla {
    let status = &cx.theme().highlight_theme.style.status;
    match severity {
        DiagnosticSeverity::Error => status.error_background(cx),
        DiagnosticSeverity::Warning => status.warning_background(cx),
        DiagnosticSeverity::Info => status.info_background(cx),
        DiagnosticSeverity::Hint => status.hint_background(cx),
    }
}

pub(crate) fn diagnostic_foreground(severity: DiagnosticSeverity, cx: &App) -> Hsla {
    let status = &cx.theme().highlight_theme.style.status;
    match severity {
        DiagnosticSeverity::Error => status.error(cx),
        DiagnosticSeverity::Warning => status.warning(cx),
        DiagnosticSeverity::Info => status.info(cx),
        DiagnosticSeverity::Hint => status.hint(cx),
    }
}

pub(crate) fn diagnostic_border(severity: DiagnosticSeverity, cx: &App) -> Hsla {
    let status = &cx.theme().highlight_theme.style.status;
    match severity {
        DiagnosticSeverity::Error => status.error_border(cx),
        DiagnosticSeverity::Warning => status.warning_border(cx),
        DiagnosticSeverity::Info => status.info_border(cx),
        DiagnosticSeverity::Hint => status.hint_border(cx),
    }
}
