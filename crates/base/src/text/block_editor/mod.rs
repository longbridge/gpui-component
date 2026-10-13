//! Headless block editor model: spike for the design on
//! [longbridge/gpui-kit#3049](https://github.com/longbridge/gpui-kit/pull/3049).
//!
//! Model only: stable [`BlockId`]s, validated [`BlockPosition`]s, normalized
//! inline mark runs, invertible steps grouped into transactions, intent-based
//! undo coalescing, commands on nested blocks, and a `MarkdownSource`-based
//! serializer. The UI slice adds the [`BlockEditorState`] entity and the [`BlockEditor`]
//! view (design Phase 3, spike form).
//! Everything is behind the off-by-default `block-editor` feature, and
//! [`BlockEditorModel`] is a spike name.

mod command;
mod document;
mod editor;
mod element;
mod frame;
mod id;
mod import;
mod inline;
mod kind;
mod marks;
mod order;
mod position;
mod projection;
mod selection;
mod serialize;
mod state;
mod step;
mod style;
mod transaction;
mod undo;

#[cfg(test)]
mod tests;

pub use command::{CommandError, EditorCommand};
pub use document::{BlockRef, DocumentRef};
pub use editor::BlockEditorModel;
pub use element::BlockEditor;
pub use state::{BlockEditorEvent, BlockEditorState};
pub use style::BlockEditorStyle;

pub(crate) fn init(cx: &mut gpui::App) {
    state::init(cx);
}
pub use id::BlockId;
pub use kind::BlockKind;
pub use marks::{MarkChange, MarkKind, MarkSet, MarkState};
pub use position::{Affinity, BlockPosition, PositionError};
pub use selection::{BlockSelection, SelectionError};
pub use transaction::{EditOrigin, Transaction};
