use gpui::SharedString;
use markdown::{
    ParseOptions,
    mdast::{self, Node},
};

use crate::{
    highlighter::HighlightTheme,
    text::{
        document::ParsedDocument,
        node::{
            self, BlockNode, CodeBlock, ImageNode, InlineNode, LinkMark, NodeContext, Paragraph,
            Span, Table, TableRow, TextMark,
        },
    },
};

/// Parse Markdown into a tree of nodes.
///
/// TODO: Remove `highlight_theme` option, this should in render stage.
pub(crate) fn parse(
    source: &str,
    cx: &mut NodeContext,
    highlight_theme: &HighlightTheme,
) -> Result<ParsedDocument, SharedString> {
    markdown::to_mdast(&source, &ParseOptions::gfm())
        .map(|n| ast_to_document(source, n, cx, highlight_theme))
        .map_err(|e| e.to_string().into())
}

fn parse_table_row(table: &mut Table, node: &mdast::TableRow, cx: &mut NodeContext) {
    let mut row = TableRow::default();
    node.children.iter().for_each(|c| {
        match c {
            Node::TableCell(cell) => {
                parse_table_cell(&mut row, cell, cx);
            }
            _ => {}
        };
    });
    table.children.push(row);
}

fn parse_table_cell(row: &mut node::TableRow, node: &mdast::TableCell, cx: &mut NodeContext) {
    let mut paragraph = Paragraph::default();
    node.children.iter().for_each(|c| {
        parse_paragraph(&mut paragraph, c, cx);
    });
    let table_cell = node::TableCell {
        children: paragraph,
        ..Default::default()
    };
    row.children.push(table_cell);
}

fn parse_paragraph(paragraph: &mut Paragraph, node: &mdast::Node, cx: &mut NodeContext) -> String {
    let span = node.position().map(|pos| Span {
        start: cx.offset + pos.start.offset,
        end: cx.offset + pos.end.offset,
    });
    if let Some(span) = span {
        paragraph.set_span(span);
    }

    let mut text = String::new();

    match node {
        Node::Paragraph(val) => {
            val.children.iter().for_each(|c| {
                text.push_str(&parse_paragraph(paragraph, c, cx));
            });
        }
        Node::Text(val) => {
            text = val.value.clone();
            paragraph.push_str(&val.value)
        }
        Node::Emphasis(val) => {
            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child, cx));
            }
            paragraph.push(
                InlineNode::new(&text).marks(vec![(0..text.len(), TextMark::default().italic())]),
            );
        }
        Node::Strong(val) => {
            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child, cx));
            }
            paragraph.push(
                InlineNode::new(&text).marks(vec![(0..text.len(), TextMark::default().bold())]),
            );
        }
        Node::Delete(val) => {
            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child, cx));
            }
            paragraph.push(
                InlineNode::new(&text)
                    .marks(vec![(0..text.len(), TextMark::default().strikethrough())]),
            );
        }
        Node::InlineCode(val) => {
            text = val.value.clone();
            paragraph.push(
                InlineNode::new(&text).marks(vec![(0..text.len(), TextMark::default().code())]),
            );
        }
        Node::Link(val) => {
            let link_mark = Some(LinkMark {
                url: val.url.clone().into(),
                title: val.title.clone().map(|s| s.into()),
                ..Default::default()
            });

            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child, cx));
            }

            // FIXME: GPUI InteractiveText does not support inline images yet.
            // So here we push images to the paragraph directly.
            for child in child_paragraph.children.iter_mut() {
                if let Some(image) = child.image.as_mut() {
                    image.link = link_mark.clone();
                }

                child.marks.push((
                    0..child.text.len(),
                    TextMark {
                        link: link_mark.clone(),
                        ..Default::default()
                    },
                ));
            }

            paragraph.merge(child_paragraph);
        }
        Node::Image(raw) => {
            paragraph.push_image(ImageNode {
                url: raw.url.clone().into(),
                title: raw.title.clone().map(|t| t.into()),
                alt: Some(raw.alt.clone().into()),
                ..Default::default()
            });
        }
        Node::InlineMath(raw) => {
            text = raw.value.clone();
            paragraph.push(
                InlineNode::new(&text).marks(vec![(0..text.len(), TextMark::default().code())]),
            );
        }
        Node::MdxTextExpression(raw) => {
            text = raw.value.clone();
            paragraph
                .push(InlineNode::new(&text).marks(vec![(0..text.len(), TextMark::default())]));
        }
        Node::Html(val) => match super::html::parse(&val.value, cx) {
            Ok(el) => {
                if el
                    .blocks
                    .first()
                    .map(|node| node.is_break())
                    .unwrap_or(false)
                {
                    text = "\n".to_owned();
                    paragraph.push(InlineNode::new(&text));
                } else {
                    if cfg!(debug_assertions) {
                        tracing::warn!("unsupported inline html tag: {:#?}", el);
                    }
                }
            }
            Err(err) => {
                if cfg!(debug_assertions) {
                    tracing::warn!("failed parsing html: {:#?}", err);
                }

                text.push_str(&val.value);
            }
        },
        Node::FootnoteReference(foot) => {
            let prefix = format!("[{}]", foot.identifier);
            paragraph.push(InlineNode::new(&prefix).marks(vec![(
                0..prefix.len(),
                TextMark {
                    italic: true,
                    ..Default::default()
                },
            )]));
        }
        Node::LinkReference(link) => {
            let mut child_paragraph = Paragraph::default();
            let mut child_text = String::new();
            for child in link.children.iter() {
                child_text.push_str(&parse_paragraph(&mut child_paragraph, child, cx));
            }

            let link_mark = LinkMark {
                url: "".into(),
                title: link.label.clone().map(Into::into),
                identifier: Some(link.identifier.clone().into()),
            };

            paragraph.push(InlineNode::new(&child_text).marks(vec![(
                0..child_text.len(),
                TextMark {
                    link: Some(link_mark),
                    ..Default::default()
                },
            )]));
        }
        _ => {
            if cfg!(debug_assertions) {
                tracing::warn!("unsupported inline node: {:#?}", node);
            }
        }
    }

    text
}

fn ast_to_document(
    source: &str,
    root: mdast::Node,
    cx: &mut NodeContext,
    highlight_theme: &HighlightTheme,
) -> ParsedDocument {
    let root = match root {
        Node::Root(r) => r,
        _ => panic!("expected root node"),
    };

    let blocks = root
        .children
        .into_iter()
        .map(|c| ast_to_node(c, cx, highlight_theme))
        .collect();
    ParsedDocument {
        source: source.to_string().into(),
        blocks,
    }
}

fn new_span(pos: Option<markdown::unist::Position>, cx: &NodeContext) -> Option<Span> {
    let pos = pos?;

    Some(Span {
        start: cx.offset + pos.start.offset,
        end: cx.offset + pos.end.offset,
    })
}

fn ast_to_node(
    value: mdast::Node,
    cx: &mut NodeContext,
    highlight_theme: &HighlightTheme,
) -> BlockNode {
    match value {
        Node::Root(_) => unreachable!("node::Root should be handled separately"),
        Node::Paragraph(val) => {
            let mut paragraph = Paragraph::default();
            val.children.iter().for_each(|c| {
                parse_paragraph(&mut paragraph, c, cx);
            });
            paragraph.span = new_span(val.position, cx);
            BlockNode::Paragraph(paragraph)
        }
        Node::Blockquote(val) => {
            let children = val
                .children
                .into_iter()
                .map(|c| ast_to_node(c, cx, highlight_theme))
                .collect();
            BlockNode::Blockquote {
                children,
                span: new_span(val.position, cx),
            }
        }
        Node::List(list) => {
            let children = list
                .children
                .into_iter()
                .map(|c| ast_to_node(c, cx, highlight_theme))
                .collect();
            BlockNode::List {
                ordered: list.ordered,
                children,
                span: new_span(list.position, cx),
            }
        }
        Node::ListItem(val) => {
            let children = val
                .children
                .into_iter()
                .map(|c| ast_to_node(c, cx, highlight_theme))
                .collect();
            BlockNode::ListItem {
                children,
                spread: val.spread,
                checked: val.checked,
                span: new_span(val.position, cx),
            }
        }
        Node::Break(val) => BlockNode::Break {
            html: false,
            span: new_span(val.position, cx),
        },
        Node::Code(raw) => BlockNode::CodeBlock(CodeBlock::new(
            raw.value.into(),
            raw.lang.map(|s| s.into()),
            highlight_theme,
            new_span(raw.position, cx),
        )),
        Node::Heading(val) => {
            let mut paragraph = Paragraph::default();
            val.children.iter().for_each(|c| {
                parse_paragraph(&mut paragraph, c, cx);
            });

            BlockNode::Heading {
                level: val.depth,
                children: paragraph,
                span: new_span(val.position, cx),
            }
        }
        Node::Math(val) => BlockNode::CodeBlock(CodeBlock::new(
            val.value.into(),
            None,
            highlight_theme,
            new_span(val.position, cx),
        )),
        Node::Html(val) => match super::html::parse(&val.value, cx) {
            Ok(el) => BlockNode::Root {
                children: el.blocks,
                span: new_span(val.position, cx),
            },
            Err(err) => {
                if cfg!(debug_assertions) {
                    tracing::warn!("error parsing html: {:#?}", err);
                }

                BlockNode::Paragraph(Paragraph::new(val.value))
            }
        },
        Node::MdxFlowExpression(val) => BlockNode::CodeBlock(CodeBlock::new(
            val.value.into(),
            Some("mdx".into()),
            highlight_theme,
            new_span(val.position, cx),
        )),
        Node::Yaml(val) => BlockNode::CodeBlock(CodeBlock::new(
            val.value.into(),
            Some("yml".into()),
            highlight_theme,
            new_span(val.position, cx),
        )),
        Node::Toml(val) => BlockNode::CodeBlock(CodeBlock::new(
            val.value.into(),
            Some("toml".into()),
            highlight_theme,
            new_span(val.position, cx),
        )),
        Node::MdxJsxTextElement(val) => {
            let mut paragraph = Paragraph::default();
            val.children.iter().for_each(|c| {
                parse_paragraph(&mut paragraph, c, cx);
            });
            paragraph.span = new_span(val.position, cx);
            BlockNode::Paragraph(paragraph)
        }
        Node::MdxJsxFlowElement(val) => {
            let mut paragraph = Paragraph::default();
            val.children.iter().for_each(|c| {
                parse_paragraph(&mut paragraph, c, cx);
            });
            paragraph.span = new_span(val.position, cx);
            BlockNode::Paragraph(paragraph)
        }
        Node::ThematicBreak(val) => BlockNode::Divider {
            span: new_span(val.position, cx),
        },
        Node::Table(val) => {
            let mut table = Table::default();
            table.column_aligns = val
                .align
                .clone()
                .into_iter()
                .map(|align| align.into())
                .collect();
            val.children.iter().for_each(|c| {
                if let Node::TableRow(row) = c {
                    parse_table_row(&mut table, row, cx);
                }
            });
            table.span = new_span(val.position, cx);

            BlockNode::Table(table)
        }
        Node::FootnoteDefinition(def) => {
            let mut paragraph = Paragraph::default();
            let prefix = format!("[{}]: ", def.identifier);
            paragraph.push(InlineNode::new(&prefix).marks(vec![(
                0..prefix.len(),
                TextMark {
                    italic: true,
                    ..Default::default()
                },
            )]));

            def.children.iter().for_each(|c| {
                parse_paragraph(&mut paragraph, c, cx);
            });
            paragraph.span = new_span(def.position, cx);
            BlockNode::Paragraph(paragraph)
        }
        Node::Definition(def) => {
            cx.add_ref(
                def.identifier.clone().into(),
                LinkMark {
                    url: def.url.clone().into(),
                    identifier: Some(def.identifier.clone().into()),
                    title: def.title.clone().map(Into::into),
                },
            );

            BlockNode::Definition {
                identifier: def.identifier.clone().into(),
                url: def.url.clone().into(),
                title: def.title.clone().map(|s| s.into()),
                span: new_span(def.position, cx),
            }
        }
        _ => {
            if cfg!(debug_assertions) {
                tracing::warn!("unsupported node: {:#?}", value);
            }
            BlockNode::Unknown
        }
    }
}
