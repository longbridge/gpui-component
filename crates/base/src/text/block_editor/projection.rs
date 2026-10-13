//! Projection of caret leaves into an immutable render snapshot (§7.3).
//!
//! The snapshot is rebuilt only when the document revision changes. Per
//! leaf, the previous projection is reused when its content is unchanged,
//! so a keystroke re-projects exactly one leaf.

use std::{collections::HashMap, rc::Rc};

use gpui::SharedString;

use super::{
    document::{BlockContent, EditorDocument},
    id::BlockId,
    kind::BlockKindData,
};
use crate::text::node::TextMark;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LeafStyle {
    Paragraph,
    Heading(u8),
    Code,
    Rule,
    Atom,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct LeafProjection {
    pub(crate) block: BlockId,
    pub(crate) style: LeafStyle,
    /// Shaped text. For atoms this is a display label, not caret text.
    pub(crate) text: SharedString,
    pub(crate) runs: Vec<(usize, TextMark)>,
    /// List and blockquote nesting depth.
    pub(crate) depth: u8,
    pub(crate) quote: bool,
    /// List marker for the first leaf of a list item.
    pub(crate) marker: Option<SharedString>,
}

#[derive(Default)]
pub(crate) struct ProjectionCache {
    revision: Option<u64>,
    leaves: Rc<Vec<Rc<LeafProjection>>>,
    by_block: HashMap<BlockId, Rc<LeafProjection>>,
    /// Number of leaf projections built (not reused); a `cfg(test)` perf
    /// counter (D9).
    pub(crate) built: usize,
    pub(crate) rebuilds: usize,
}

impl ProjectionCache {
    pub(crate) fn snapshot(&mut self, doc: &EditorDocument) -> Rc<Vec<Rc<LeafProjection>>> {
        if self.revision == Some(doc.revision) {
            return self.leaves.clone();
        }
        self.rebuilds += 1;
        let mut by_block = HashMap::with_capacity(doc.leaves().len());
        let mut leaves = Vec::with_capacity(doc.leaves().len());
        for id in doc.leaves() {
            let fresh = project(doc, *id);
            let leaf = match self.by_block.get(id) {
                Some(prev) if **prev == fresh => prev.clone(),
                _ => {
                    self.built += 1;
                    Rc::new(fresh)
                }
            };
            by_block.insert(*id, leaf.clone());
            leaves.push(leaf);
        }
        self.by_block = by_block;
        self.leaves = Rc::new(leaves);
        self.revision = Some(doc.revision);
        self.leaves.clone()
    }

    pub(crate) fn invalidate(&mut self) {
        self.revision = None;
        self.by_block.clear();
    }
}

fn project(doc: &EditorDocument, id: BlockId) -> LeafProjection {
    let entry = doc.get(id);
    let kind = entry.map(|e| &e.kind);
    let (style, text, runs) = match (kind, entry.map(|e| &e.content)) {
        (_, Some(BlockContent::Inline(inline))) => {
            let style = match kind {
                Some(BlockKindData::Heading { level }) => LeafStyle::Heading(*level),
                _ => LeafStyle::Paragraph,
            };
            let runs = inline
                .runs()
                .iter()
                .map(|run| (run.len, run.marks.clone()))
                .collect();
            (style, SharedString::from(inline.text().to_string()), runs)
        }
        (_, Some(BlockContent::Plain(code))) => {
            let text = code.trim_end_matches('\n').to_string();
            let len = text.len();
            (
                LeafStyle::Code,
                SharedString::from(text),
                vec![(len, TextMark::default().code())],
            )
        }
        (Some(BlockKindData::HorizontalRule), _) => (
            LeafStyle::Rule,
            SharedString::from(" "),
            vec![(1, TextMark::default())],
        ),
        (kind, _) => {
            let label = format!("[{:?}]", kind.map(|k| k.public()));
            let len = label.len();
            (
                LeafStyle::Atom,
                SharedString::from(label),
                vec![(len, TextMark::default())],
            )
        }
    };

    let path = doc.path(id);
    let mut depth = 0u8;
    let mut quote = false;
    for ancestor in &path {
        match doc.kind(*ancestor) {
            Some(BlockKindData::List { .. }) => depth += 1,
            Some(BlockKindData::Blockquote) => {
                depth += 1;
                quote = true;
            }
            _ => {}
        }
    }
    let marker = list_marker(doc, id);
    LeafProjection {
        block: id,
        style,
        text,
        runs,
        depth,
        quote,
        marker,
    }
}

fn list_marker(doc: &EditorDocument, id: BlockId) -> Option<SharedString> {
    let (item, index) = doc.index_in_parent(id)?;
    if index != 0 {
        return None;
    }
    let Some(BlockKindData::ListItem { checked, .. }) = doc.kind(item) else {
        return None;
    };
    let (list, item_ix) = doc.index_in_parent(item)?;
    let marker = match (checked, doc.kind(list)) {
        (Some(true), _) => "☑".to_string(),
        (Some(false), _) => "☐".to_string(),
        (
            None,
            Some(BlockKindData::List {
                ordered: true,
                start,
            }),
        ) => {
            format!("{}.", start.unwrap_or(1) as usize + item_ix)
        }
        _ => "•".to_string(),
    };
    Some(marker.into())
}
