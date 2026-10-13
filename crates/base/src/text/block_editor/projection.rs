//! Projection of the editing model into the shared renderer's `BlockNode`s
//! (design §7.3, D3).
//!
//! Each top-level block is written to Markdown in render mode and parsed by
//! gpui-kit's own parser, so the editor renders exactly like `TextView`.
//! Every text leaf's `InlineState` is tagged with its `BlockId` as the
//! `editor_key`, which is how the renderer reports layouts back (the seam
//! in `text::editor_seam`). A top-level block is re-projected only when its
//! per-block revision changes, so one keystroke re-projects one block.

use std::{collections::HashMap, rc::Rc, sync::Arc};

use super::{
    document::{BlockContent, EditorDocument},
    id::BlockId,
    kind::BlockKindData,
    serialize::block_for_render,
};
use crate::text::{
    format::markdown,
    node::{BlockNode, NodeContext},
};

/// One projected top-level block.
pub(crate) struct ProjectedBlock {
    revision: u64,
    pub(crate) nodes: Vec<BlockNode>,
}

/// Geometry facts the frame needs about a caret leaf.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LeafInfo {
    pub(crate) block: BlockId,
    /// Caret offsets are 0/1 (code blocks in the spike).
    pub(crate) atomic: bool,
    /// Model text length (bytes); for atomic leaves the rendered text length.
    pub(crate) len: usize,
}

#[derive(Default)]
pub(crate) struct Snapshot {
    pub(crate) blocks: Vec<Rc<ProjectedBlock>>,
    /// Leaves that have a rendered text layout, in document order.
    pub(crate) leaves: Rc<Vec<LeafInfo>>,
    /// Tables and their keyed cells, for node selection (no caret in cells).
    pub(crate) tables: Rc<Vec<(BlockId, Vec<BlockId>)>>,
}

#[derive(Default)]
pub(crate) struct ProjectionCache {
    revision: Option<u64>,
    snapshot: Rc<Snapshot>,
    by_id: HashMap<BlockId, Rc<ProjectedBlock>>,
    /// `cfg(test)`-checked perf counters (D9): top-level blocks parsed, and
    /// snapshot rebuilds.
    pub(crate) built: usize,
    pub(crate) rebuilds: usize,
}

impl ProjectionCache {
    pub(crate) fn snapshot(&mut self, doc: &EditorDocument) -> Rc<Snapshot> {
        if self.revision == Some(doc.revision) {
            return self.snapshot.clone();
        }
        self.rebuilds += 1;
        let top = doc.children(doc.root);
        let mut blocks = Vec::with_capacity(top.len());
        let mut by_id = HashMap::with_capacity(top.len());
        for id in top {
            let revision = doc.block_revision(*id);
            let block = match self.by_id.get(id) {
                Some(prev) if prev.revision == revision => prev.clone(),
                _ => {
                    self.built += 1;
                    Rc::new(project(doc, *id, revision))
                }
            };
            by_id.insert(*id, block.clone());
            blocks.push(block);
        }
        let leaves = doc
            .leaves()
            .iter()
            .filter_map(|id| leaf_info(doc, *id))
            .collect();
        let mut tables: Vec<(BlockId, Vec<BlockId>)> = Vec::new();
        for id in doc.leaves() {
            if doc.kind(*id) != Some(&BlockKindData::TableCell) {
                continue;
            }
            let Some(table) = doc
                .path(*id)
                .into_iter()
                .rev()
                .find(|b| matches!(doc.kind(*b), Some(BlockKindData::Table { .. })))
            else {
                continue;
            };
            match tables.last_mut() {
                Some((last, cells)) if *last == table => cells.push(*id),
                _ => tables.push((table, vec![*id])),
            }
        }
        self.by_id = by_id;
        self.snapshot = Rc::new(Snapshot {
            blocks,
            leaves: Rc::new(leaves),
            tables: Rc::new(tables),
        });
        self.revision = Some(doc.revision);
        self.snapshot.clone()
    }

    pub(crate) fn invalidate(&mut self) {
        self.revision = None;
        self.by_id.clear();
    }
}

fn leaf_info(doc: &EditorDocument, id: BlockId) -> Option<LeafInfo> {
    let entry = doc.get(id)?;
    match &entry.content {
        BlockContent::Inline(inline) if entry.kind != BlockKindData::TableCell => Some(LeafInfo {
            block: id,
            atomic: false,
            len: inline.len(),
        }),
        BlockContent::Plain(code) => Some(LeafInfo {
            block: id,
            atomic: true,
            len: code.trim_end_matches('\n').len(),
        }),
        _ => None,
    }
}

/// Model leaves under `id` that the renderer draws as one text leaf each.
fn text_leaves(doc: &EditorDocument, id: BlockId, out: &mut Vec<BlockId>) {
    match doc.get(id).map(|e| (&e.kind, &e.content)) {
        Some((BlockKindData::TableCell, BlockContent::Inline(_))) => out.push(id),
        Some((_, BlockContent::Children(children))) => {
            for child in children {
                text_leaves(doc, *child, out);
            }
        }
        Some((_, BlockContent::Inline(_) | BlockContent::Plain(_))) => out.push(id),
        _ => {}
    }
}

/// Rendered text leaves in a parsed tree, in order.
fn node_states(
    node: &BlockNode,
    out: &mut Vec<Arc<std::sync::Mutex<crate::text::inline::InlineState>>>,
) {
    match node {
        BlockNode::Paragraph(paragraph) => out.push(paragraph.state.clone()),
        BlockNode::Heading { children, .. } => out.push(children.state.clone()),
        BlockNode::CodeBlock(code) => out.push(code.inline_state().clone()),
        BlockNode::Table(table) => {
            for row in &table.children {
                for cell in &row.children {
                    out.push(cell.children.state.clone());
                }
            }
        }
        BlockNode::Root { children, .. }
        | BlockNode::Blockquote { children, .. }
        | BlockNode::List { children, .. }
        | BlockNode::ListItem { children, .. } => {
            for child in children {
                node_states(child, out);
            }
        }
        _ => {}
    }
}

fn project(doc: &EditorDocument, id: BlockId, revision: u64) -> ProjectedBlock {
    let source = block_for_render(doc, id);
    let mut node_cx = NodeContext {
        markdown_extensions: doc.extensions.clone(),
        ..Default::default()
    };
    let nodes = markdown::parse(&source, &mut node_cx)
        .map(|parsed| parsed.blocks.as_ref().clone())
        .unwrap_or_default();
    let mut leaves = Vec::new();
    text_leaves(doc, id, &mut leaves);
    let mut states = Vec::new();
    for node in &nodes {
        node_states(node, &mut states);
    }
    // The parser reads back what the serializer wrote, so leaves pair up in
    // order. If they ever don't, leave the block unkeyed (no caret geometry)
    // rather than misplace carets.
    if leaves.len() == states.len() {
        for (leaf, state) in leaves.iter().zip(states) {
            if let Ok(mut state) = state.lock() {
                state.editor_key = Some(leaf.as_u64());
            }
        }
    }
    ProjectedBlock { revision, nodes }
}
