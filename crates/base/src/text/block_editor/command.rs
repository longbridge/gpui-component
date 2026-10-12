use super::{
    document::{BlockContent, EditorDocument},
    editor::BlockEditorModel,
    id::BlockId,
    inline::InlineText,
    kind::{BlockKind, BlockKindData},
    marks::{MarkChange, MarkKind, MarkState},
    position::{BlockPosition, PositionError},
    selection::{BlockSelection, ResolvedSelection, SelectionError},
    step::{DetachedBlock, DetachedContent, StepError},
    transaction::{EditOrigin, TransactionBuilder},
};
use crate::text::node::TextMark;

#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
pub enum EditorCommand {
    ToggleMark(MarkKind),
    ApplyMark(MarkChange),
    SetBlockKind(BlockKind),
    ToggleList { ordered: bool },
    ToggleTask,
    Indent,
    Outdent,
    MoveBlocks { down: bool },
    InsertBlock(BlockKind),
    SelectAll,
    DeleteSelection,
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
pub enum CommandError {
    Position(PositionError),
    Selection(SelectionError),
    /// The caret is in a block the spike cannot edit in place (code block,
    /// atom); Phase 4.
    NotEditable(BlockId),
    Unsupported(&'static str),
    /// A step failed; the transaction was rolled back.
    Internal(String),
}

impl From<StepError> for CommandError {
    fn from(error: StepError) -> Self {
        Self::Internal(format!("{error:?}"))
    }
}

impl From<SelectionError> for CommandError {
    fn from(error: SelectionError) -> Self {
        Self::Selection(error)
    }
}

impl From<PositionError> for CommandError {
    fn from(error: PositionError) -> Self {
        Self::Position(error)
    }
}

/// A position plus its path from the root, which commands walk to find
/// their target ("the nearest `ListItem`") at any nesting depth (R7).
pub(crate) struct ResolvedPosition {
    pub(crate) pos: BlockPosition,
    pub(crate) path: Vec<BlockId>,
}

impl ResolvedPosition {
    pub(crate) fn resolve(doc: &EditorDocument, pos: BlockPosition) -> Result<Self, PositionError> {
        let pos = doc.validate(pos)?;
        Ok(Self {
            pos,
            path: doc.path(pos.block),
        })
    }

    /// Nearest ancestor-or-self matching `pred`, innermost first.
    pub(crate) fn nearest(
        &self,
        doc: &EditorDocument,
        pred: impl Fn(&BlockKindData) -> bool,
    ) -> Option<BlockId> {
        self.path
            .iter()
            .rev()
            .copied()
            .find(|id| doc.kind(*id).is_some_and(&pred))
    }
}

fn is_list_item(kind: &BlockKindData) -> bool {
    matches!(kind, BlockKindData::ListItem { .. })
}

fn caret(pos: BlockPosition) -> Option<BlockSelection> {
    Some(BlockSelection::caret(pos))
}

/// Delete a resolved range and return where the caret goes. Atoms and code
/// blocks between (or at fully covered ends of) the range are removed.
fn delete_range(
    tx: &mut TransactionBuilder<'_>,
    range: ResolvedSelection,
) -> Result<BlockPosition, CommandError> {
    let (start, end) = (range.start, range.end);
    let doc = &*tx.doc;
    if start.block == end.block {
        if doc.inline(start.block).is_some() {
            tx.replace_text(start.block, start.offset..end.offset, InlineText::new())?;
        } else if start.offset == 0 && end.offset == 1 {
            tx.remove_block(start.block)?;
        }
        return Ok(start);
    }
    // Select-all followed by an edit starts a fresh document (R2 repro):
    // every leaf is fully covered, including the code block.
    if start == doc.first_caret()
        && end.block == doc.last_caret().block
        && end.offset == doc.last_caret().offset
    {
        let root = doc.root;
        let len = doc.children(root).len();
        tx.remove(root, 0..len)?;
        let paragraph = DetachedBlock::paragraph(InlineText::new());
        let id = paragraph.id;
        tx.insert(root, 0, vec![paragraph])?;
        return Ok(BlockPosition::new(id, 0));
    }
    let leaves = doc.walk_leaves();
    let (Some(si), Some(ei)) = (
        leaves.iter().position(|id| *id == start.block),
        leaves.iter().position(|id| *id == end.block),
    ) else {
        return Err(CommandError::Position(PositionError::UnknownBlock(
            start.block,
        )));
    };
    let is_cell =
        |doc: &EditorDocument, id: BlockId| doc.kind(id) == Some(&BlockKindData::TableCell);

    // Start leaf.
    let start_kept = if let Some(inline) = doc.inline(start.block) {
        let len = inline.len();
        tx.replace_text(start.block, start.offset..len, InlineText::new())?;
        true
    } else if start.offset == 0 {
        tx.remove_block(start.block)?;
        false
    } else {
        false
    };
    let start_inline = start_kept;
    // Leaves in between.
    for id in &leaves[si + 1..ei] {
        if is_cell(tx.doc, *id) {
            let len = tx.doc.leaf_len(*id).unwrap_or(0);
            tx.replace_text(*id, 0..len, InlineText::new())?;
        } else if tx.doc.get(*id).is_some() {
            tx.remove_block(*id)?;
        }
    }
    // End leaf.
    if let Some(inline) = tx.doc.inline(end.block) {
        let tail = inline.slice(end.offset..inline.len());
        let joinable = start_inline && !is_cell(tx.doc, end.block) && !is_cell(tx.doc, start.block);
        if joinable {
            tx.remove_block(end.block)?;
            tx.replace_text(start.block, start.offset..start.offset, tail)?;
        } else {
            tx.replace_text(end.block, 0..end.offset, InlineText::new())?;
            if !start_kept {
                return Ok(BlockPosition::new(end.block, 0));
            }
        }
    } else if end.offset == 1 {
        tx.remove_block(end.block)?;
    }
    if start_kept {
        Ok(start)
    } else {
        Ok(tx.doc.first_caret())
    }
}

impl BlockEditorModel {
    /// Marks for text typed at `pos` (D6): bold/italic/strike/underline/
    /// highlight extend at their trailing edge; link and code only apply
    /// strictly inside.
    pub(crate) fn inherited_marks(&self, pos: BlockPosition) -> TextMark {
        let Some(inline) = self.doc.inline(pos.block) else {
            return TextMark::default();
        };
        let before = inline.mark_before(pos.offset);
        let after = inline.mark_after(pos.offset);
        match (before, after) {
            (Some(b), Some(a)) if b == a => b.clone(),
            (Some(b), _) => {
                let mut mark = b.clone();
                for kind in [MarkKind::Link, MarkKind::Code] {
                    if !kind.extends_at_boundary() {
                        mark = MarkChange::new().clear(kind).apply(&mark);
                    }
                }
                mark
            }
            _ => TextMark::default(),
        }
    }

    /// Per-leaf `(block, start, end)` covered by a text range.
    pub(crate) fn text_ranges(&self, range: ResolvedSelection) -> Vec<(BlockId, usize, usize)> {
        let leaves = self.doc.leaves();
        let (Some(si), Some(ei)) = (
            leaves.iter().position(|id| *id == range.start.block),
            leaves.iter().position(|id| *id == range.end.block),
        ) else {
            return Vec::new();
        };
        leaves[si..=ei]
            .iter()
            .map(|id| {
                let len = self.doc.leaf_len(*id).unwrap_or(0);
                let lo = if *id == range.start.block {
                    range.start.offset
                } else {
                    0
                };
                let hi = if *id == range.end.block {
                    range.end.offset
                } else {
                    len
                };
                (*id, lo, hi)
            })
            .collect()
    }

    fn resolved(&self) -> Result<ResolvedSelection, CommandError> {
        Ok(self.doc.resolve_selection(&self.selection)?)
    }

    pub fn insert_text(&mut self, text: &str, origin: EditOrigin) -> Result<(), CommandError> {
        let range = self.resolved()?;
        let stored = self.stored_marks.clone();
        let marks = match &stored {
            Some(marks) => marks.clone(),
            None => self.inherited_marks(range.start),
        };
        self.transact(origin, |tx| {
            let mut pos = if range.is_collapsed() {
                range.start
            } else {
                delete_range(tx, range)?
            };
            for (ix, line) in text.split('\n').enumerate() {
                if ix > 0 {
                    pos = split_at(tx, pos)?;
                }
                let Some(inline) = tx.doc.inline(pos.block) else {
                    return Err(CommandError::NotEditable(pos.block));
                };
                let _ = inline;
                let insert = InlineText::plain(line, marks.clone());
                let len = insert.len();
                tx.replace_text(pos.block, pos.offset..pos.offset, insert)?;
                pos.offset += len;
            }
            Ok(caret(pos))
        })?;
        Ok(())
    }

    pub fn delete(&mut self, forward: bool) -> Result<(), CommandError> {
        let range = self.resolved()?;
        let origin = if forward {
            EditOrigin::DeleteForward
        } else {
            EditOrigin::Backspace
        };
        self.transact(origin, |tx| {
            if !range.is_collapsed() {
                return Ok(caret(delete_range(tx, range)?));
            }
            let pos = range.start;
            let doc = &*tx.doc;
            let Some(inline) = doc.inline(pos.block) else {
                // Caret on an atom or code block: delete the block.
                tx.remove_block(pos.block)?;
                return Ok(None);
            };
            if !forward && pos.offset > 0 {
                let prev = inline.prev_grapheme(pos.offset).unwrap_or(0);
                tx.replace_text(pos.block, prev..pos.offset, InlineText::new())?;
                return Ok(caret(BlockPosition::new(pos.block, prev)));
            }
            if forward && pos.offset < inline.len() {
                let next = inline.next_grapheme(pos.offset).unwrap_or(inline.len());
                tx.replace_text(pos.block, pos.offset..next, InlineText::new())?;
                return Ok(caret(pos));
            }
            if doc.kind(pos.block) == Some(&BlockKindData::TableCell) {
                return Ok(None);
            }
            // At a block edge: join with the neighbouring leaf.
            let leaves = doc.walk_leaves();
            let Some(ix) = leaves.iter().position(|id| *id == pos.block) else {
                return Ok(None);
            };
            let (first, second) = if forward {
                match leaves.get(ix + 1) {
                    Some(next) => (pos.block, *next),
                    None => return Ok(None),
                }
            } else {
                match ix.checked_sub(1).and_then(|i| leaves.get(i)) {
                    Some(prev) => (*prev, pos.block),
                    None => return Ok(None),
                }
            };
            if doc.kind(first) == Some(&BlockKindData::TableCell)
                || doc.kind(second) == Some(&BlockKindData::TableCell)
            {
                return Ok(None);
            }
            match (doc.inline(first), doc.inline(second)) {
                (Some(a), Some(b)) => {
                    let at = a.len();
                    let moved = b.clone();
                    tx.remove_block(second)?;
                    tx.replace_text(first, at..at, moved)?;
                    Ok(caret(BlockPosition::new(first, at)))
                }
                // The neighbour is an atom or code block: remove it.
                _ => {
                    let other = if forward { second } else { first };
                    tx.remove_block(other)?;
                    Ok(caret(pos))
                }
            }
        })?;
        Ok(())
    }

    pub fn split_block(&mut self) -> Result<(), CommandError> {
        let range = self.resolved()?;
        self.transact(EditOrigin::Command("split_block"), |tx| {
            let pos = if range.is_collapsed() {
                range.start
            } else {
                delete_range(tx, range)?
            };
            Ok(caret(split_at(tx, pos)?))
        })?;
        Ok(())
    }

    pub fn run(&mut self, command: EditorCommand) -> Result<bool, CommandError> {
        match command {
            EditorCommand::SelectAll => {
                let selection = BlockSelection::Text {
                    anchor: self.doc.first_caret(),
                    head: self.doc.last_caret(),
                };
                self.set_selection(selection)?;
                Ok(true)
            }
            EditorCommand::DeleteSelection => {
                let range = self.resolved()?;
                if range.is_collapsed() {
                    return Ok(false);
                }
                self.transact(EditOrigin::Command("delete_selection"), |tx| {
                    Ok(caret(delete_range(tx, range)?))
                })
            }
            EditorCommand::ToggleMark(kind) => {
                if kind == MarkKind::Link {
                    return Err(CommandError::Unsupported("toggle link: use ApplyMark"));
                }
                let change = if self.mark_state(kind) == MarkState::On {
                    MarkChange::new().clear(kind)
                } else {
                    MarkChange::new().set(kind)
                };
                self.apply_mark(change)
            }
            EditorCommand::ApplyMark(change) => self.apply_mark(change),
            EditorCommand::SetBlockKind(kind) => self.set_block_kind(kind),
            EditorCommand::ToggleList { ordered } => self.toggle_list(ordered),
            EditorCommand::ToggleTask => self.with_item("toggle_task", |tx, item| {
                if let Some(BlockKindData::ListItem { checked, spread }) =
                    tx.doc.kind(item).cloned()
                {
                    let checked = if checked.is_some() { None } else { Some(false) };
                    tx.set_kind(item, BlockKindData::ListItem { checked, spread })?;
                }
                Ok(())
            }),
            EditorCommand::Indent => self.indent(),
            EditorCommand::Outdent => self.outdent(),
            EditorCommand::MoveBlocks { down } => self.move_blocks(down),
            EditorCommand::InsertBlock(BlockKind::HorizontalRule) => {
                let pos = self
                    .caret()
                    .ok_or(CommandError::Unsupported("block selection"))?;
                self.transact(EditOrigin::Command("insert_block"), |tx| {
                    let (parent, index) = tx
                        .doc
                        .index_in_parent(pos.block)
                        .ok_or(PositionError::UnknownBlock(pos.block))?;
                    let rule = DetachedBlock::leaf(
                        BlockKindData::HorizontalRule,
                        BlockContent::Atom(super::document::AtomData::None),
                    );
                    tx.insert(parent, index + 1, vec![rule])?;
                    Ok(None)
                })
            }
            EditorCommand::InsertBlock(_) => Err(CommandError::Unsupported("insert_block kind")),
        }
    }

    fn apply_mark(&mut self, change: MarkChange) -> Result<bool, CommandError> {
        let range = self.resolved()?;
        if range.is_collapsed() {
            let current = self.active_marks().0;
            self.stored_marks = Some(change.apply(&current));
            return Ok(false);
        }
        let ranges = self.text_ranges(range);
        let selection = self.selection.clone();
        self.transact(EditOrigin::Command("toggle_mark"), |tx| {
            for (block, lo, hi) in ranges {
                let Some(inline) = tx.doc.inline(block) else {
                    continue;
                };
                if lo >= hi {
                    continue;
                }
                let mut marked = inline.slice(lo..hi);
                let len = marked.len();
                marked.map_marks(0..len, |mark| change.apply(mark));
                if marked != inline.slice(lo..hi) {
                    tx.replace_text(block, lo..hi, marked)?;
                }
            }
            Ok(Some(selection))
        })
    }

    fn set_block_kind(&mut self, kind: BlockKind) -> Result<bool, CommandError> {
        let data = match kind {
            BlockKind::Paragraph => BlockKindData::Paragraph,
            BlockKind::Heading { level } if (1..=6).contains(&level) => {
                BlockKindData::Heading { level }
            }
            _ => return Err(CommandError::Unsupported("set_block_kind target")),
        };
        let range = self.resolved()?;
        let blocks: Vec<BlockId> = self
            .text_ranges(range)
            .into_iter()
            .map(|(id, _, _)| id)
            .filter(|id| {
                matches!(
                    self.doc.kind(*id),
                    Some(BlockKindData::Paragraph | BlockKindData::Heading { .. })
                )
            })
            .collect();
        self.transact(EditOrigin::Command("set_block_kind"), |tx| {
            for id in blocks {
                tx.set_kind(id, data.clone())?;
            }
            Ok(None)
        })
    }

    fn caret_resolved(&self) -> Result<ResolvedPosition, CommandError> {
        let pos = self
            .caret()
            .ok_or(CommandError::Unsupported("block selection"))?;
        Ok(ResolvedPosition::resolve(&self.doc, pos)?)
    }

    fn with_item(
        &mut self,
        name: &'static str,
        f: impl FnOnce(&mut TransactionBuilder<'_>, BlockId) -> Result<(), CommandError>,
    ) -> Result<bool, CommandError> {
        let resolved = self.caret_resolved()?;
        let Some(item) = resolved.nearest(&self.doc, is_list_item) else {
            return Ok(false);
        };
        self.transact(EditOrigin::Command(name), |tx| {
            f(tx, item)?;
            Ok(None)
        })
    }

    fn toggle_list(&mut self, ordered: bool) -> Result<bool, CommandError> {
        let resolved = self.caret_resolved()?;
        let leaf = resolved.pos.block;
        let item = resolved.nearest(&self.doc, is_list_item);
        self.transact(EditOrigin::Command("toggle_list"), |tx| {
            match item {
                Some(item) => {
                    let (list, index) = tx
                        .doc
                        .index_in_parent(item)
                        .ok_or(StepError::UnknownBlock(item))?;
                    let Some(BlockKindData::List {
                        ordered: was,
                        start,
                    }) = tx.doc.kind(list).cloned()
                    else {
                        return Ok(None);
                    };
                    if was != ordered {
                        tx.set_kind(list, BlockKindData::List { ordered, start })?;
                        return Ok(None);
                    }
                    // Lift the item out of its list, splitting the list.
                    let (outer, list_index) = tx
                        .doc
                        .index_in_parent(list)
                        .ok_or(StepError::UnknownBlock(list))?;
                    let count = tx.doc.children(list).len();
                    let after = tx.remove(list, index + 1..count)?;
                    if !after.is_empty() {
                        let rest = DetachedBlock::container(
                            BlockKindData::List {
                                ordered,
                                start: None,
                            },
                            after,
                        );
                        tx.insert(outer, list_index + 1, vec![rest])?;
                    }
                    let content_len = tx.doc.children(item).len();
                    let content = tx.remove(item, 0..content_len)?;
                    tx.insert(outer, list_index + 1, content)?;
                }
                None => {
                    let (parent, index) = tx
                        .doc
                        .index_in_parent(leaf)
                        .ok_or(StepError::UnknownBlock(leaf))?;
                    let block = tx.remove_block(leaf)?;
                    let list = DetachedBlock::container(
                        BlockKindData::List {
                            ordered,
                            start: None,
                        },
                        vec![DetachedBlock::container(
                            BlockKindData::ListItem {
                                checked: None,
                                spread: false,
                            },
                            vec![block],
                        )],
                    );
                    tx.insert(parent, index, vec![list])?;
                }
            }
            Ok(None)
        })
    }

    fn indent(&mut self) -> Result<bool, CommandError> {
        let resolved = self.caret_resolved()?;
        let Some(item) = resolved.nearest(&self.doc, is_list_item) else {
            return Ok(false);
        };
        let Some((list, index)) = self.doc.index_in_parent(item) else {
            return Ok(false);
        };
        if index == 0 {
            return Ok(false);
        }
        let prev = self.doc.children(list)[index - 1];
        let ordered = matches!(
            self.doc.kind(list),
            Some(BlockKindData::List { ordered: true, .. })
        );
        self.transact(EditOrigin::Command("indent"), |tx| {
            let nested = tx
                .doc
                .children(prev)
                .last()
                .copied()
                .filter(|id| matches!(tx.doc.kind(*id), Some(BlockKindData::List { .. })));
            let block = tx.remove_block(item)?;
            match nested {
                Some(nested) => {
                    let len = tx.doc.children(nested).len();
                    tx.insert(nested, len, vec![block])?;
                }
                None => {
                    let len = tx.doc.children(prev).len();
                    let list = DetachedBlock::container(
                        BlockKindData::List {
                            ordered,
                            start: None,
                        },
                        vec![block],
                    );
                    tx.insert(prev, len, vec![list])?;
                }
            }
            Ok(None)
        })
    }

    fn outdent(&mut self) -> Result<bool, CommandError> {
        let resolved = self.caret_resolved()?;
        let Some(item) = resolved.nearest(&self.doc, is_list_item) else {
            return Ok(false);
        };
        let Some((list, index)) = self.doc.index_in_parent(item) else {
            return Ok(false);
        };
        let Some((outer_item, _)) = self.doc.index_in_parent(list) else {
            return Ok(false);
        };
        if !self.doc.kind(outer_item).is_some_and(is_list_item) {
            // Lifting a top-level item out of its list is toggle_list.
            return Ok(false);
        }
        let list_kind = self.doc.kind(list).cloned();
        self.transact(EditOrigin::Command("outdent"), |tx| {
            let (outer_list, outer_index) = tx
                .doc
                .index_in_parent(outer_item)
                .ok_or(StepError::UnknownBlock(outer_item))?;
            // Following siblings become children of the outdented item.
            let count = tx.doc.children(list).len();
            let following = tx.remove(list, index + 1..count)?;
            let mut block = tx.remove_block(item)?;
            if !following.is_empty()
                && let (DetachedContent::Children(children), Some(kind)) =
                    (&mut block.content, list_kind.clone())
            {
                children.push(DetachedBlock::container(kind, following));
            }
            tx.insert(outer_list, outer_index + 1, vec![block])?;
            Ok(None)
        })
    }

    fn move_blocks(&mut self, down: bool) -> Result<bool, CommandError> {
        let resolved = self.caret_resolved()?;
        let target = resolved
            .nearest(&self.doc, is_list_item)
            .unwrap_or(resolved.pos.block);
        let Some((parent, index)) = self.doc.index_in_parent(target) else {
            return Ok(false);
        };
        let len = self.doc.children(parent).len();
        let new_index = if down {
            if index + 1 >= len {
                return Ok(false);
            }
            index + 1
        } else {
            if index == 0 {
                return Ok(false);
            }
            index - 1
        };
        let selection = self.selection.clone();
        self.transact(EditOrigin::Command("move_blocks"), |tx| {
            tx.move_block(target, parent, new_index)?;
            Ok(Some(selection))
        })
    }
}

/// Split the text block at `pos`; returns the caret at the start of the new
/// block. In a list item's first paragraph this creates a new list item.
fn split_at(
    tx: &mut TransactionBuilder<'_>,
    pos: BlockPosition,
) -> Result<BlockPosition, CommandError> {
    let doc = &*tx.doc;
    let Some(inline) = doc.inline(pos.block) else {
        return Err(CommandError::NotEditable(pos.block));
    };
    let kind = doc.kind(pos.block).cloned();
    if kind == Some(BlockKindData::TableCell) {
        return Err(CommandError::NotEditable(pos.block));
    }
    let tail = inline.slice(pos.offset..inline.len());
    let len = inline.len();
    let (parent, index) = doc
        .index_in_parent(pos.block)
        .ok_or(PositionError::UnknownBlock(pos.block))?;
    let new_kind = match kind {
        Some(BlockKindData::Heading { level }) if pos.offset < len => {
            BlockKindData::Heading { level }
        }
        _ => BlockKindData::Paragraph,
    };
    let parent_kind = doc.kind(parent).cloned();
    tx.replace_text(pos.block, pos.offset..len, InlineText::new())?;
    let block = DetachedBlock::leaf(new_kind, BlockContent::Inline(tail));
    let id = block.id;
    match parent_kind {
        Some(BlockKindData::ListItem { checked, spread }) if index == 0 => {
            let item = DetachedBlock::container(
                BlockKindData::ListItem {
                    checked: checked.map(|_| false),
                    spread,
                },
                vec![block],
            );
            let (list, item_index) = tx
                .doc
                .index_in_parent(parent)
                .ok_or(StepError::UnknownBlock(parent))?;
            tx.insert(list, item_index + 1, vec![item])?;
        }
        _ => tx.insert(parent, index + 1, vec![block])?,
    }
    Ok(BlockPosition::new(id, 0))
}
