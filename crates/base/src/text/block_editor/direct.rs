//! Direct projection of the editing model into the shared renderer's
//! `BlockNode`s (design §7.3): no Markdown round trip, so no render-mode
//! placeholders and no re-parse of a whole list for an edit inside it.
//! Every text leaf (paragraph, heading, code block, table cell) gets its
//! `BlockId` as `editor_key`.

use std::sync::{Arc, Mutex};

use super::{
    document::{AtomData, BlockContent, EditorDocument},
    id::BlockId,
    inline::{InlineAtom, InlineText},
    kind::BlockKindData,
};
use crate::text::{
    inline::InlineState,
    node::{BlockNode, BlockState, CodeBlock, InlineNode, Paragraph, Table, TableCell, TableRow},
};

fn keyed(state: &Arc<Mutex<InlineState>>, id: BlockId) {
    if let Ok(mut state) = state.lock() {
        state.editor_key = Some(id.as_u64());
    }
}

/// A paragraph holding `inline`, keyed by `id`.
pub(crate) fn paragraph(inline: &InlineText, id: BlockId) -> Paragraph {
    let mut paragraph = Paragraph::default();
    for (range, mark, atom) in inline.pieces() {
        match atom {
            Some(InlineAtom::Image(image)) => paragraph.push(InlineNode::image(image.clone())),
            Some(InlineAtom::Custom(node)) => {
                let node = InlineNode::custom(node.clone());
                let len = node.text.len();
                paragraph.push(node.marks(vec![(0..len, mark.clone())]));
            }
            None => {
                let text = &inline.text()[range];
                paragraph.push(
                    InlineNode::new(text.to_string()).marks(vec![(0..text.len(), mark.clone())]),
                );
            }
        }
    }
    keyed(&paragraph.state, id);
    paragraph
}

/// Project one block (and its subtree).
pub(crate) fn project_block(doc: &EditorDocument, id: BlockId) -> Option<BlockNode> {
    let entry = doc.get(id)?;
    let children = |doc: &EditorDocument| -> Vec<BlockNode> {
        doc.children(id)
            .iter()
            .filter_map(|child| project_block(doc, *child))
            .collect()
    };
    let inline = || match &entry.content {
        BlockContent::Inline(inline) => paragraph(inline, id),
        _ => paragraph(&InlineText::new(), id),
    };
    Some(match &entry.kind {
        BlockKindData::Root => BlockNode::Root {
            children: children(doc),
            span: None,
        },
        BlockKindData::Paragraph => BlockNode::Paragraph(inline()),
        BlockKindData::Heading { level } => BlockNode::Heading {
            level: *level,
            children: inline(),
            span: None,
        },
        BlockKindData::Blockquote => BlockNode::Blockquote {
            children: children(doc),
            span: None,
        },
        BlockKindData::List { ordered, start } => BlockNode::List {
            children: children(doc),
            ordered: *ordered,
            start: *start,
            span: None,
        },
        BlockKindData::ListItem { checked, spread } => BlockNode::ListItem {
            children: children(doc),
            spread: *spread,
            checked: *checked,
            span: None,
        },
        BlockKindData::CodeBlock { lang } => {
            let code = match &entry.content {
                BlockContent::Plain(code) => code.clone(),
                _ => String::new(),
            };
            let code = CodeBlock::new(code.into(), lang.clone(), None::<crate::text::node::Span>);
            keyed(code.inline_state(), id);
            BlockNode::CodeBlock(code)
        }
        BlockKindData::Table { aligns } => BlockNode::Table(Table {
            children: doc
                .children(id)
                .iter()
                .map(|row| TableRow {
                    children: doc
                        .children(*row)
                        .iter()
                        .map(|cell| TableCell {
                            children: match doc.inline(*cell) {
                                Some(inline) => paragraph(inline, *cell),
                                None => paragraph(&InlineText::new(), *cell),
                            },
                            width: None,
                        })
                        .collect(),
                })
                .collect(),
            column_aligns: aligns.clone(),
            span: None,
            table_data_cache: Default::default(),
            column_widths_cache: Default::default(),
        }),
        BlockKindData::TableRow | BlockKindData::TableCell => return None,
        BlockKindData::HorizontalRule => BlockNode::HorizontalRule { span: None },
        BlockKindData::Break { html } => BlockNode::Break {
            html: *html,
            span: None,
        },
        BlockKindData::Custom => match &entry.content {
            BlockContent::Atom(AtomData::Custom(node)) => BlockNode::Custom {
                node: node.clone(),
                state: BlockState::default(),
            },
            _ => BlockNode::Unknown,
        },
        BlockKindData::Definition {
            identifier,
            url,
            title,
        } => BlockNode::Definition {
            identifier: identifier.clone(),
            url: url.clone(),
            title: title.clone(),
            span: None,
        },
        BlockKindData::Unknown => BlockNode::Unknown,
    })
}
