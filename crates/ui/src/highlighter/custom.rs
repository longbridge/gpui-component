use std::ops::Range;

use gpui::{App, HighlightStyle};

/// A consumer-supplied highlighter that contributes additional styled byte
/// ranges to the [`Input`](crate::input::InputState) element's render
/// pipeline, alongside the built-in tree-sitter
/// [`SyntaxHighlighter`](crate::highlighter::SyntaxHighlighter) and
/// [`DiagnosticSet`](crate::highlighter::DiagnosticSet).
///
/// # Use cases
///
/// - Plugging a different parser engine (syntect, regex-based tokenizers,
///   language server semantic tokens) for languages tree-sitter does not
///   cover.
/// - Layering decorative highlights (search match emphasis, code folding
///   indicators, scope-aware accent colors) without rebuilding the
///   diagnostic pipeline.
///
/// # Composition
///
/// Custom-highlighter output is layered between the tree-sitter base layer
/// and the diagnostic overlay: tree-sitter (base) → custom (overlay) →
/// diagnostics (top, wavy underlines). Diagnostics keep highest priority so
/// errors remain visible regardless of language coloring.
///
/// # Threading and performance
///
/// [`styles`](Self::styles) is called from the render thread inside the
/// `Input` element's per-frame highlight pass. Implementations should be
/// `Send + Sync` and inexpensive — caching parsed state across calls is the
/// implementor's responsibility.
///
/// The viewport-clamping that the built-in tree-sitter path applies for
/// long-line skipping does **not** apply to custom highlighter output;
/// implementations are responsible for their own performance characteristics.
pub trait CustomHighlighter: Send + Sync {
    /// Return styled byte ranges within the requested viewport range.
    ///
    /// Returned ranges should be a subset of `range`; ranges outside `range`
    /// are silently dropped during composition.
    fn styles(&self, range: Range<usize>, cx: &App) -> Vec<(Range<usize>, HighlightStyle)>;
}
