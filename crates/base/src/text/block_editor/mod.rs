//! Headless block editor model: spike for the design on
//! [longbridge/gpui-kit#3049](https://github.com/longbridge/gpui-kit/pull/3049).
//!
//! Model only: stable [`BlockId`]s, validated [`BlockPosition`]s, normalized
//! inline mark runs, invertible steps grouped into transactions, intent-based
//! undo coalescing, commands on nested blocks, and a `MarkdownSource`-based
//! serializer. No rendering, input handling or entity yet (design Phase 3).
//! Everything is behind the off-by-default `block-editor` feature, and
//! [`BlockEditorModel`] is a spike name.

mod command;
mod document;
mod editor;
mod id;
mod import;
mod inline;
mod kind;
mod marks;
mod order;
mod position;
mod selection;
mod serialize;
mod step;
mod transaction;
mod undo;

#[cfg(test)]

pub use command::{CommandError, EditorCommand};
pub use document::{BlockRef, DocumentRef};
pub use editor::BlockEditorModel;
pub use id::BlockId;
pub use kind::BlockKind;
pub use marks::{MarkChange, MarkKind, MarkSet, MarkState};
pub use position::{Affinity, BlockPosition, PositionError};
pub use selection::{BlockSelection, SelectionError};
pub use transaction::{EditOrigin, Transaction};
