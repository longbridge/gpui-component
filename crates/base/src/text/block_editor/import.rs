//! `markdown::parse` output (`BlockNode`) into the editor model.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::SharedString;

use super::{
    document::{AtomData, BlockContent, EditorDocument},
    inline::{InlineAtom, InlineText},
    kind::BlockKindData,
    step::DetachedBlock,
    transaction::TransactionBuilder,
};
use crate::text::{
    format::markdown,
    markdown_ext::MarkdownExtensions,
    node::{BlockNode, InlineNode, LinkMark, NodeContext, Paragraph, TextMark},
};

pub(crate) fn parse_document(
    source: &str,
    extensions: Arc<MarkdownExtensions>,
) -> Result<EditorDocument, SharedString> {
    let mut cx = NodeContext {
        markdown_extensions: extensions.clone(),
        ..Default::default()
    };
    let parsed = markdown::parse(source, &mut cx)?;
    let mut doc = EditorDocument::empty(extensions);
    let root = doc.root;
    // Drop the placeholder paragraph of the empty document.
    doc.detach(root, 0);
    let mut defs = Defs::new();
    collect_definitions(&parsed.blocks, &mut defs);
    let mut blocks = Vec::new();
    for node in parsed.blocks.iter() {
        import_block(node, &defs, &mut blocks);
    }
    for (ix, block) in blocks.into_iter().enumerate() {
        doc.attach(root, ix, block);
    }
    // Same structural rules as after every transaction (empty list items
    // get a paragraph, an empty document gets one leaf). The steps of this
    // pass are not history.
    let mut tx = TransactionBuilder::new(&mut doc);
    tx.normalize()
        .map_err(|e| SharedString::from(format!("{e:?}")))?;
    drop(tx);
    doc.structure_dirty = true;
    doc.refresh_order();
    doc.revision = 0;
    Ok(doc)
}

fn leaf(kind: BlockKindData, content: BlockContent) -> DetachedBlock {
    DetachedBlock::leaf(kind, content)
}

fn import_children(nodes: &[BlockNode], defs: &Defs) -> Vec<DetachedBlock> {
    let mut out = Vec::new();
    for node in nodes {
        import_block(node, defs, &mut out);
    }
    out
}

pub(crate) fn import_block(node: &BlockNode, defs: &Defs, out: &mut Vec<DetachedBlock>) {
    let block = match node {
        // An HTML container: splice its children in place.
        BlockNode::Root { children, .. } => {
            for child in children {
                import_block(child, defs, out);
            }
            return;
        }
        BlockNode::Paragraph(paragraph) => leaf(
            BlockKindData::Paragraph,
            BlockContent::Inline(import_inline(paragraph, defs)),
        ),
        BlockNode::Heading {
            level, children, ..
        } => leaf(
            BlockKindData::Heading { level: *level },
            BlockContent::Inline(import_inline(children, defs)),
        ),
        BlockNode::Blockquote { children, .. } => {
            DetachedBlock::container(BlockKindData::Blockquote, import_children(children, defs))
        }
        BlockNode::List {
            children,
            ordered,
            start,
            ..
        } => DetachedBlock::container(
            BlockKindData::List {
                ordered: *ordered,
                start: *start,
            },
            import_children(children, defs),
        ),
        BlockNode::ListItem {
            children,
            spread,
            checked,
            ..
        } => DetachedBlock::container(
            BlockKindData::ListItem {
                checked: *checked,
                spread: *spread,
            },
            import_children(children, defs),
        ),
        BlockNode::CodeBlock(code) => {
            let text = code.code().to_string();
            let text = text.strip_suffix('\n').unwrap_or(&text).to_string();
            leaf(
                BlockKindData::CodeBlock { lang: code.lang() },
                BlockContent::Plain(text),
            )
        }
        BlockNode::Custom { node, .. } => leaf(
            BlockKindData::Custom,
            BlockContent::Atom(AtomData::Custom(node.clone())),
        ),
        BlockNode::Table(table) => {
            let rows = table
                .children
                .iter()
                .map(|row| {
                    DetachedBlock::container(
                        BlockKindData::TableRow,
                        row.children
                            .iter()
                            .map(|cell| {
                                leaf(
                                    BlockKindData::TableCell,
                                    BlockContent::Inline(import_inline(&cell.children, defs)),
                                )
                            })
                            .collect(),
                    )
                })
                .collect();
            DetachedBlock::container(
                BlockKindData::Table {
                    aligns: table.column_aligns.clone(),
                },
                rows,
            )
        }
        BlockNode::Break { html, .. } => leaf(
            BlockKindData::Break { html: *html },
            BlockContent::Atom(AtomData::None),
        ),
        BlockNode::HorizontalRule { .. } => leaf(
            BlockKindData::HorizontalRule,
            BlockContent::Atom(AtomData::None),
        ),
        BlockNode::Definition {
            identifier,
            url,
            title,
            ..
        } => leaf(
            BlockKindData::Definition {
                identifier: identifier.clone(),
                url: url.clone(),
                title: title.clone(),
            },
            BlockContent::Atom(AtomData::None),
        ),
        BlockNode::Unknown => leaf(BlockKindData::Unknown, BlockContent::Atom(AtomData::None)),
    };
    out.push(block);
}

/// Convert a paragraph's inline nodes into normalized runs. `InlineNode.marks`
/// may overlap (`**b*c*d**` gives bold 0..3 and italic 1..2), so split at
/// every boundary and merge the marks covering each piece.
///
/// Reference links (`[text][id]`) arrive with an empty url and an
/// identifier; they are resolved against the document's definitions so the
/// canonical output is an inline link. Unresolved references (and footnote
/// links) lose their link mark: documented lossiness.
pub(crate) fn import_inline(paragraph: &Paragraph, defs: &Defs) -> InlineText {
    let mut out = InlineText::new();
    for node in &paragraph.children {
        out.push(import_inline_node(node));
    }
    let len = out.len();
    out.map_marks(0..len, |mark| match &mark.link {
        Some(link) if link.url.is_empty() => {
            let link = link
                .identifier
                .as_ref()
                .and_then(|id| defs.get(&SharedString::from(id.to_lowercase())))
                .map(|(url, title)| LinkMark {
                    url: url.clone(),
                    identifier: None,
                    title: title.clone(),
                });
            TextMark {
                link,
                ..mark.clone()
            }
        }
        _ => mark.clone(),
    });
    out
}

pub(crate) type Defs = HashMap<SharedString, (SharedString, Option<SharedString>)>;

pub(crate) fn collect_definitions(nodes: &[BlockNode], defs: &mut Defs) {
    for node in nodes {
        match node {
            BlockNode::Definition {
                identifier,
                url,
                title,
                ..
            } => {
                defs.insert(
                    identifier.to_lowercase().into(),
                    (url.clone(), title.clone()),
                );
            }
            BlockNode::Root { children, .. }
            | BlockNode::Blockquote { children, .. }
            | BlockNode::List { children, .. }
            | BlockNode::ListItem { children, .. } => collect_definitions(children, defs),
            _ => {}
        }
    }
}

fn import_inline_node(node: &InlineNode) -> InlineText {
    let merged_marks = || {
        let mut mark = TextMark::default();
        for (_, layer) in &node.marks {
            mark.merge(layer.clone());
        }
        mark
    };
    if let Some(custom) = &node.custom {
        return InlineText::atom(InlineAtom::Custom(custom.clone()), merged_marks());
    }
    if let Some(image) = &node.image {
        return InlineText::atom(InlineAtom::Image(image.clone()), merged_marks());
    }
    let text: &str = &node.text;
    let mut cuts = vec![0, text.len()];
    for (range, _) in &node.marks {
        cuts.push(range.start.min(text.len()));
        cuts.push(range.end.min(text.len()));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut out = InlineText::new();
    for pair in cuts.windows(2) {
        let (lo, hi) = (pair[0], pair[1]);
        if lo >= hi || !text.is_char_boundary(lo) || !text.is_char_boundary(hi) {
            continue;
        }
        let mut mark = TextMark::default();
        for (range, layer) in &node.marks {
            if range.start <= lo && hi <= range.end {
                mark.merge(layer.clone());
            }
        }
        out.push(InlineText::plain(&text[lo..hi], mark));
    }
    out
}
