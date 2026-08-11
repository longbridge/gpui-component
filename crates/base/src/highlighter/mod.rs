//! Syntax-highlighting and diagnostics infrastructure used by Base editors.

mod diagnostics;
pub use diagnostics::*;

#[cfg(feature = "tree-sitter")]
mod highlighter;
#[cfg(feature = "tree-sitter")]
mod languages;
#[cfg(feature = "tree-sitter")]
mod registry;

#[cfg(feature = "tree-sitter")]
pub use highlighter::*;
#[cfg(feature = "tree-sitter")]
pub use languages::*;
#[cfg(feature = "tree-sitter")]
pub use registry::*;

#[cfg(not(feature = "tree-sitter"))]
mod wasm_stub;
#[cfg(not(feature = "tree-sitter"))]
pub use wasm_stub::*;
