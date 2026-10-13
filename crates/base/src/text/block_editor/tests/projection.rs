//! The direct projection (`direct.rs`) against `parse(serialize(doc))`.

use super::props::{Rng, document};
use super::*;
use crate::text::{
    block_editor::{
        direct::project_block,
        document::BlockContent,
        import::{Defs, collect_definitions, import_block},
        step::{DetachedBlock, DetachedContent},
    },
    format::markdown,
    node::{BlockNode, NodeContext},
};

/// Structure and content without ids.
fn shape(block: &DetachedBlock, out: &mut String) {
    out.push_str(&format!("{:?}(", block.kind));
    match &block.content {
        DetachedContent::Children(children) => {
            for child in children {
                shape(child, out);
            }
        }
        DetachedContent::Leaf(content) => out.push_str(&format!("{content:?}")),
    }
    out.push(')');
}

fn shapes(nodes: &[BlockNode]) -> String {
    let mut defs = Defs::new();
    collect_definitions(nodes, &mut defs);
    let mut blocks = Vec::new();
    for node in nodes {
        import_block(node, &defs, &mut blocks);
    }
    let mut out = String::new();
    for block in &blocks {
        shape(block, &mut out);
    }
    out
}

fn keys(node: &BlockNode, out: &mut Vec<u64>) {
    let mut push = |state: &std::sync::Arc<std::sync::Mutex<crate::text::inline::InlineState>>| {
        if let Some(key) = state.lock().unwrap().editor_key {
            out.push(key);
        }
    };
    match node {
        BlockNode::Paragraph(p) => push(&p.state),
        BlockNode::Heading { children, .. } => push(&children.state),
        BlockNode::CodeBlock(code) => push(code.inline_state()),
        BlockNode::Table(table) => {
            for row in &table.children {
                for cell in &row.children {
                    push(&cell.children.state);
                }
            }
        }
        BlockNode::Root { children, .. }
        | BlockNode::Blockquote { children, .. }
        | BlockNode::List { children, .. }
        | BlockNode::ListItem { children, .. } => {
            for child in children {
                keys(child, out);
            }
        }
        _ => {}
    }
}

/// Direct projection equals parsing the serialized document, and every text
/// leaf is keyed exactly once, in document order.
fn assert_equivalent(m: &BlockEditorModel) {
    let doc = &m.doc;
    let direct: Vec<BlockNode> = doc
        .children(doc.root)
        .iter()
        .filter_map(|id| project_block(doc, *id))
        .collect();
    let mut cx = NodeContext {
        markdown_extensions: doc.extensions.clone(),
        ..Default::default()
    };
    let source = m.source();
    let parsed = markdown::parse(&source, &mut cx).unwrap();
    assert_eq!(
        shapes(&direct),
        shapes(&parsed.blocks),
        "direct projection differs from parse(serialize) for {source:?}"
    );
    let mut got = Vec::new();
    for node in &direct {
        keys(node, &mut got);
    }
    let want: Vec<u64> = doc
        .leaves()
        .iter()
        .filter(|id| {
            matches!(
                doc.get(**id).map(|e| &e.content),
                Some(BlockContent::Inline(_) | BlockContent::Plain(_))
            )
        })
        .map(|id| id.as_u64())
        .collect();
    assert_eq!(got, want, "{source:?}");
}

#[test]
fn direct_projection_matches_parse_on_fixtures() {
    for source in [
        "# H1\n\n###### H6",
        "> quote\n>\n> - item",
        "1. one\n2. two",
        "3. three\n4. four",
        "- [ ] todo\n- [x] done",
        "- a\n  - b\n    1. c\n- d",
        "```rust\nfn main() {}\n```",
        "````\n```\n````",
        "---",
        "[id]: https://example.com \"Title\"",
        "a [link](https://example.com) b",
        "![alt](https://example.com/a.png)",
        "`code` and ~~strike~~ **b*bi*b** <u>u</u> <mark>m</mark>",
        "| **a** | <u>b</u> |\n| :-- | --: |\n| *c* | d |",
        "lead  and trail ",
    ] {
        assert_equivalent(&model(source));
    }
}

#[test]
fn direct_projection_matches_parse_on_random_documents() {
    for seed in 1..=300u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        assert_equivalent(&model(&document(&mut rng)));
    }
}
