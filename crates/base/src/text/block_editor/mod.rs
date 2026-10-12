//! Headless block editor model: spike for the design on
//! [longbridge/gpui-kit#3049](https://github.com/longbridge/gpui-kit/pull/3049).
//!
//! Model only: stable [`BlockId`]s, validated [`BlockPosition`]s, normalized
//! inline mark runs, invertible steps grouped into transactions, intent-based
//! undo coalescing, commands on nested blocks, and a `MarkdownSource`-based
//! serializer. No rendering, input handling or entity yet (design Phase 3).
//! Everything is behind the off-by-default `block-editor` feature, and
//! [`BlockEditorModel`] is a spike name.
#![allow(dead_code)] // removed once the facade uses everything

mod id;
mod inline;
mod kind;
mod marks;


pub use id::BlockId;
pub use kind::BlockKind;
pub use marks::{MarkChange, MarkKind, MarkSet, MarkState};
