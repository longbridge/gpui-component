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

    /// Insert plain text at the selection (replacing it); `\n` splits blocks.
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

    /// Backspace (`forward == false`) or Delete: a grapheme, the selection, a join with the neighbouring block, or (next to a table) select the table.
    pub fn delete(&mut self, forward: bool) -> Result<(), CommandError> {
        if self.selected_blocks().is_some() {
            return self.delete_blocks().map(|_| ());
        }
        let range = self.resolved()?;
        if range.is_collapsed() && self.select_adjacent_table(range.start, forward)? {
            return Ok(());
        }
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

    /// Enter: split the text block (in a list, a new item; in an empty item, exit or outdent).
    pub fn split_block(&mut self) -> Result<(), CommandError> {
        let range = self.resolved()?;
        if range.is_collapsed() && self.exit_empty_item(range.start)? {
            return Ok(());
        }
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

    /// Enter in an empty list item whose only child is the caret's empty
    /// paragraph: a nested item outdents one level, a top-level item becomes
    /// a paragraph after the list (splitting it when mid-list). One
    /// transaction, so one undo step. Returns `false` when it does not apply.
    fn exit_empty_item(&mut self, pos: BlockPosition) -> Result<bool, CommandError> {
        let doc = &self.doc;
        if doc.kind(pos.block) != Some(&BlockKindData::Paragraph)
            || doc.inline(pos.block).is_none_or(|inline| inline.len() != 0)
        {
            return Ok(false);
        }
        let Some((item, 0)) = doc.index_in_parent(pos.block) else {
            return Ok(false);
        };
        if !doc.kind(item).is_some_and(is_list_item) || doc.children(item).len() != 1 {
            return Ok(false);
        }
        let Some((list, index)) = doc.index_in_parent(item) else {
            return Ok(false);
        };
        let Some(BlockKindData::List { ordered, .. }) = doc.kind(list).cloned() else {
            return Ok(false);
        };
        let Some((outer, list_index)) = doc.index_in_parent(list) else {
            return Ok(false);
        };
        if doc.kind(outer).is_some_and(is_list_item) {
            self.outdent()?;
            return Ok(true);
        }
        let leaf = pos.block;
        self.transact(EditOrigin::Command("exit_list"), |tx| {
            let count = tx.doc.children(list).len();
            let mut removed = tx.remove(list, index..count)?;
            let rest: Vec<_> = removed.drain(1..).collect();
            let item = removed.pop().ok_or(StepError::OutOfRange)?;
            let DetachedContent::Children(content) = item.content else {
                return Err(StepError::KindMismatch.into());
            };
            let mut at = list_index + 1;
            if index == 0 {
                tx.remove(outer, list_index..list_index + 1)?;
                at = list_index;
            }
            let inserted = content.len();
            tx.insert(outer, at, content)?;
            if !rest.is_empty() {
                let tail = DetachedBlock::container(
                    BlockKindData::List {
                        ordered,
                        start: None,
                    },
                    rest,
                );
                tx.insert(outer, at + inserted, vec![tail])?;
            }
            Ok(caret(BlockPosition::new(leaf, 0)))
        })?;
        Ok(true)
    }

    /// Paste Markdown as blocks, as one undo step. Text without a line
    /// break, or that parses to a single paragraph, is left to plain
    /// insertion (`insert_text`). Otherwise the text is parsed: a
    /// leading paragraph merges into the text before the caret, a trailing
    /// one into the text after it, and the blocks between are inserted
    /// between the two halves. Returns whether it applied (`false` for
    /// single-line text or when the caret is not in an editable text block;
    /// the caller then inserts plain text).
    pub fn paste_blocks(&mut self, text: &str) -> Result<bool, CommandError> {
        if !text.trim_end_matches('\n').contains('\n') {
            return Ok(false);
        }
        let mut cx = crate::text::node::NodeContext {
            markdown_extensions: self.doc.extensions.clone(),
            ..Default::default()
        };
        let Ok(parsed) = crate::text::format::markdown::parse(text, &mut cx) else {
            return Ok(false);
        };
        let mut defs = super::import::Defs::new();
        super::import::collect_definitions(&parsed.blocks, &mut defs);
        let mut blocks = Vec::new();
        for node in parsed.blocks.iter() {
            super::import::import_block(node, &defs, &mut blocks);
        }
        // Nothing structural (one paragraph, soft line breaks only): plain
        // text, where each line becomes a block as typed.
        if blocks.is_empty() || (blocks.len() == 1 && blocks[0].kind == BlockKindData::Paragraph) {
            return Ok(false);
        }
        if self.selected_blocks().is_some() {
            self.delete_blocks()?;
        }
        let range = self.resolved()?;
        let editable = |doc: &EditorDocument, pos: BlockPosition| {
            doc.inline(pos.block).is_some()
                && doc.kind(pos.block) != Some(&BlockKindData::TableCell)
        };
        if !editable(&self.doc, range.start) || !editable(&self.doc, range.end) {
            return Ok(false);
        }
        let paragraph_inline = |block: &DetachedBlock| match (&block.kind, &block.content) {
            (BlockKindData::Paragraph, DetachedContent::Leaf(BlockContent::Inline(inline))) => {
                Some(inline.clone())
            }
            _ => None,
        };
        self.transact(EditOrigin::Paste, |tx| {
            let pos = if range.is_collapsed() {
                range.start
            } else {
                delete_range(tx, range)?
            };
            let mut blocks = blocks;
            let head = blocks.first().and_then(paragraph_inline);
            if head.is_some() {
                blocks.remove(0);
            }
            let tail = if blocks.is_empty() {
                None
            } else {
                blocks.last().and_then(paragraph_inline)
            };
            if tail.is_some() {
                blocks.pop();
            }
            // Only one paragraph: plain inline insertion with its marks.
            if blocks.is_empty() && tail.is_none() {
                let inline = head.unwrap_or_default();
                let len = inline.len();
                tx.replace_text(pos.block, pos.offset..pos.offset, inline)?;
                return Ok(caret(BlockPosition::new(pos.block, pos.offset + len)));
            }
            let left = pos.block;
            let right = split_at(tx, pos)?.block;
            let merged_head = head.is_some();
            if let Some(head) = head {
                let at = tx.doc.leaf_len(left).unwrap_or(0);
                tx.replace_text(left, at..at, head)?;
            }
            let mut caret_at = BlockPosition::new(right, 0);
            if let Some(tail) = tail.clone() {
                caret_at.offset = tail.len();
                tx.replace_text(right, 0..0, tail)?;
            }
            let (parent, index) = tx
                .doc
                .index_in_parent(left)
                .ok_or(PositionError::UnknownBlock(left))?;
            let last = blocks.last().map(|b| b.id);
            tx.insert(parent, index + 1, blocks)?;
            // Halves that stayed empty go away.
            if !merged_head && tx.doc.leaf_len(left) == Some(0) {
                tx.remove_block(left)?;
            }
            if tail.is_none() && tx.doc.leaf_len(right) == Some(0) {
                if let Some(last) = last {
                    tx.remove_block(right)?;
                    caret_at = tx.doc.last_leaf_in(last);
                }
            }
            Ok(caret(caret_at))
        })?;
        Ok(true)
    }

    /// A `BlockSelection::Blocks` as its parent and child index range.
    pub(crate) fn selected_blocks(&self) -> Option<(BlockId, std::ops::Range<usize>)> {
        let BlockSelection::Blocks { anchor, head } = &self.selection else {
            return None;
        };
        let (parent, a) = self.doc.index_in_parent(*anchor)?;
        let (_, b) = self.doc.index_in_parent(*head)?;
        Some((parent, a.min(b)..a.max(b) + 1))
    }

    /// Remove the selected blocks; the caret goes to the block that took
    /// their place (an empty paragraph when nothing is left).
    fn delete_blocks(&mut self) -> Result<bool, CommandError> {
        let Some((parent, range)) = self.selected_blocks() else {
            return Ok(false);
        };
        self.transact(EditOrigin::Command("delete_blocks"), |tx| {
            let start = range.start;
            tx.remove(parent, range)?;
            let children = tx.doc.children(parent).to_vec();
            let pos = if let Some(next) = children.get(start) {
                tx.doc.first_leaf_in(*next)
            } else if let Some(prev) = start.checked_sub(1).and_then(|i| children.get(i)) {
                tx.doc.last_leaf_in(*prev)
            } else {
                let paragraph = DetachedBlock::paragraph(InlineText::new());
                let id = paragraph.id;
                tx.insert(parent, start, vec![paragraph])?;
                BlockPosition::new(id, 0)
            };
            Ok(caret(pos))
        })
    }

    /// Backspace at the start of a block after a table (or Delete at the end
    /// of one before it) selects the table as a node instead of editing its
    /// cells (no in-place cell editing yet).
    fn select_adjacent_table(
        &mut self,
        pos: BlockPosition,
        forward: bool,
    ) -> Result<bool, CommandError> {
        let Some(len) = self.doc.leaf_len(pos.block) else {
            return Ok(false);
        };
        if self.doc.kind(pos.block) == Some(&BlockKindData::TableCell)
            || (forward && pos.offset < len)
            || (!forward && pos.offset > 0)
        {
            return Ok(false);
        }
        let leaves = self.doc.walk_leaves();
        let Some(ix) = leaves.iter().position(|id| *id == pos.block) else {
            return Ok(false);
        };
        let neighbour = if forward {
            leaves.get(ix + 1)
        } else {
            ix.checked_sub(1).and_then(|i| leaves.get(i))
        };
        let Some(&cell) = neighbour else {
            return Ok(false);
        };
        let Some(table) = self.table_of(cell) else {
            return Ok(false);
        };
        self.set_selection(BlockSelection::Blocks {
            anchor: table,
            head: table,
        })?;
        Ok(true)
    }

    /// The table containing a cell.
    pub(crate) fn table_of(&self, cell: BlockId) -> Option<BlockId> {
        if self.doc.kind(cell) != Some(&BlockKindData::TableCell) {
            return None;
        }
        self.doc
            .path(cell)
            .into_iter()
            .rev()
            .find(|id| matches!(self.doc.kind(*id), Some(BlockKindData::Table { .. })))
    }

    /// Run an [`EditorCommand`]; returns whether the document or selection changed.
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
                if self.selected_blocks().is_some() {
                    return self.delete_blocks();
                }
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
        let target = match self.selected_blocks() {
            Some((parent, range)) if range.len() == 1 => self.doc.children(parent)[range.start],
            Some(_) => return Err(CommandError::Unsupported("move several blocks")),
            None => {
                let resolved = self.caret_resolved()?;
                resolved
                    .nearest(&self.doc, is_list_item)
                    .unwrap_or(resolved.pos.block)
            }
        };
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
