use std::ops::Range;

use gpui::SharedString;

/// A consumer-supplied highlighter that contributes additional named token
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
/// - Layering decorative highlights (search-match emphasis, scope-aware
///   accent colors) without rebuilding the diagnostic pipeline.
///
/// # Token-name vocabulary
///
/// Returned token names are resolved against the active
/// [`HighlightTheme`](crate::highlighter::HighlightTheme) — the same
/// vocabulary the tree-sitter highlighter emits (`"keyword"`, `"string"`,
/// `"comment"`, `"variable.special"`, …). Names with a `.`-namespace fall
/// back to their prefix (`"keyword.modifier"` → `"keyword"`). Unrecognized
/// names render with the default style.
///
/// This decouples token classification from styling: the theme is the
/// single source of color, so theme switches propagate without implementor
/// cooperation. It also lets a third-party highlighter share a vocabulary
/// with the built-in tree-sitter path so multiple sources colour the same
/// logical token consistently.
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
/// [`tokens`](Self::tokens) is called from the render thread on every
/// frame the input is visible. Implementations should be `Send + Sync` and
/// inexpensive — typically a read of pre-computed state. Heavy parsing
/// should happen off-thread (for example, in response to a text-change
/// event the implementor subscribes to) and cache its output for the
/// render thread to consume.
///
/// The viewport-clamping that the built-in tree-sitter path applies for
/// long-line skipping does **not** apply to custom-highlighter output;
/// implementations are responsible for their own performance
/// characteristics.
pub trait CustomHighlighter: Send + Sync {
    /// Return token-name-tagged byte ranges within the requested viewport
    /// range.
    ///
    /// Returned ranges should be a subset of `range`; ranges outside
    /// `range` are silently dropped during composition. Token names are
    /// resolved against the active theme — see the type-level docs for
    /// the vocabulary.
    fn tokens(&self, range: Range<usize>) -> Vec<(Range<usize>, SharedString)>;
}
