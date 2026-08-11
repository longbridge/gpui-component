//! Compatibility exports for the presentation-independent Base editor LSP core.
//!
//! Menus and popovers remain in `input::popovers`; provider contracts,
//! requests, caches, edits, debounce and overlay models live in `gpui-base`.

pub use gpui_base::input::{
    CodeActionItem, CodeActionProvider, CodeActionSession, CompletionMenuOptions,
    CompletionProvider, CompletionSession, DefinitionProvider, DocumentColorProvider,
    DocumentRangeSemanticTokensProvider, HoverProvider, HoverSession, Lsp, ShowDocumentHandler,
};
