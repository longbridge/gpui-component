use gpui::{
    div, prelude::FluentBuilder as _, Context, IntoElement, ParentElement, Render, SharedString,
    Styled, Window,
};
use markdown::{
    mdast::{self, Node},
    ParseOptions,
};

use crate::v_flex;

use super::{
    element::{self, ImageNode, InlineTextStyle, LinkMark, Paragraph, Span, Table, TableRow},
    html::parse_html,
};

/// Markdown GFM renderer
///
/// This is design goal is to be able to most common Markdown (GFM) features
/// to let us to display rich text in our application.
///
/// The goal:
///
/// - For used to help message.
/// - For used to display like about page.
/// - Some general style customization (Like base text size, line-height...).
///
/// Not in goal:
///
/// - As a markdown editor.
/// - Add custom markdown syntax.
/// - Complex styles cumstomization.
pub(super) struct MarkdownView {
    source: SharedString,
    parsed: bool,
    root: Option<Result<element::Node, markdown::message::Message>>,
}

impl MarkdownView {
    pub(super) fn new(source: impl Into<SharedString>) -> Self {
        Self {
            source: source.into(),
            parsed: false,
            root: None,
        }
    }

    /// Set the source of the markdown view.
    pub(crate) fn set_source(&mut self, source: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.source = source.into();
        self.parsed = false;
        cx.notify();
    }

    fn parse_if_needed(&mut self) {
        if self.parsed {
            return;
        }

        self.root = Some(markdown::to_mdast(&self.source, &ParseOptions::gfm()).map(|n| n.into()));
    }
}

impl Render for MarkdownView {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<'_, Self>) -> impl IntoElement {
        self.parse_if_needed();

        let Some(root) = self.root.clone() else {
            return div();
        };

        div().map(|this| match root {
            Ok(node) => this.child(node),
            Err(err) => this.child(
                v_flex()
                    .gap_1()
                    .child("Error parsing markdown")
                    .child(err.to_string()),
            ),
        })
    }
}

fn parse_table_row(table: &mut Table, node: &mdast::TableRow) {
    let mut row = TableRow::default();
    node.children.iter().for_each(|c| {
        match c {
            Node::TableCell(cell) => {
                parse_table_cell(&mut row, cell);
            }
            _ => {}
        };
    });
    table.children.push(row);
}

fn parse_table_cell(row: &mut element::TableRow, node: &mdast::TableCell) {
    let mut paragraph = Paragraph::default();
    node.children.iter().for_each(|c| {
        parse_paragraph(&mut paragraph, c);
    });
    let table_cell = element::TableCell {
        children: paragraph,
        ..Default::default()
    };
    row.children.push(table_cell);
}

fn parse_paragraph(paragraph: &mut Paragraph, node: &mdast::Node) -> String {
    let span = node.position().map(|pos| Span {
        start: pos.start.offset,
        end: pos.end.offset,
    });
    if let Some(span) = span {
        paragraph.set_span(span);
    }

    let mut text = String::new();

    match node {
        Node::Paragraph(val) => {
            val.children.iter().for_each(|c| {
                text.push_str(&parse_paragraph(paragraph, c));
            });
        }
        Node::Text(val) => {
            text = val.value.clone();
            paragraph.push_str(&val.value)
        }
        Node::Emphasis(val) => {
            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child));
            }
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: vec![(
                    0..text.len(),
                    InlineTextStyle {
                        italic: true,
                        ..Default::default()
                    },
                )],
            });
        }
        Node::Strong(val) => {
            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child));
            }
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: vec![(
                    0..text.len(),
                    InlineTextStyle {
                        bold: true,
                        ..Default::default()
                    },
                )],
            });
        }
        Node::Delete(val) => {
            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child));
            }
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: vec![(
                    0..text.len(),
                    InlineTextStyle {
                        strikethrough: true,
                        ..Default::default()
                    },
                )],
            });
        }
        Node::InlineCode(val) => {
            text = val.value.clone();
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: vec![(
                    0..text.len(),
                    InlineTextStyle {
                        code: true,
                        ..Default::default()
                    },
                )],
            });
        }
        Node::Link(val) => {
            let mut child_paragraph = Paragraph::default();
            for child in val.children.iter() {
                text.push_str(&parse_paragraph(&mut child_paragraph, &child));
            }
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: vec![(
                    0..text.len(),
                    InlineTextStyle {
                        link: Some(LinkMark {
                            url: val.url.clone().into(),
                            title: val.title.clone().map(|s| s.into()),
                        }),
                        ..Default::default()
                    },
                )],
            });
        }
        Node::Image(raw) => {
            paragraph.set_image(ImageNode {
                url: raw.url.clone().into(),
                title: raw.title.clone().map(|t| t.into()),
                alt: Some(raw.alt.clone().into()),
                ..Default::default()
            });
        }
        Node::InlineMath(raw) => {
            text = raw.value.clone();
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: vec![(
                    0..text.len(),
                    InlineTextStyle {
                        code: true,
                        ..Default::default()
                    },
                )],
            });
        }
        Node::MdxTextExpression(raw) => {
            text = raw.value.clone();
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: vec![(0..text.len(), InlineTextStyle::default())],
            });
        }
        Node::Html(val) => match parse_html(&val.value) {
            Ok(el) => {
                if el == element::Node::Break {
                    text.push_str("\n");
                } else {
                    if cfg!(debug_assertions) {
                        eprintln!("[markdown] unsupprted inline html tag: {:#?}", el);
                    }
                }
            }
            Err(err) => {
                if cfg!(debug_assertions) {
                    eprintln!("[markdown] error parsing html: {:#?}", err);
                }

                text.push_str(&val.value);
            }
        },
        _ => {
            if cfg!(debug_assertions) {
                eprintln!("[markdown] unsupported inline node: {:#?}", node);
            }
        }
    }

    text
}

impl From<mdast::Node> for element::Node {
    fn from(value: Node) -> Self {
        match value {
            Node::Root(val) => {
                let children = val.children.into_iter().map(|c| c.into()).collect();
                element::Node::Root(children)
            }
            Node::Paragraph(val) => {
                let mut paragraph = Paragraph::default();
                val.children.iter().for_each(|c| {
                    parse_paragraph(&mut paragraph, c);
                });

                element::Node::Paragraph(paragraph)
            }
            Node::Blockquote(val) => {
                let mut paragraph = Paragraph::default();
                val.children.iter().for_each(|c| {
                    parse_paragraph(&mut paragraph, c);
                });

                element::Node::Blockquote(paragraph)
            }
            Node::List(list) => {
                let children = list.children.into_iter().map(|c| c.into()).collect();
                element::Node::List {
                    ordered: list.ordered,
                    children,
                }
            }
            Node::ListItem(val) => {
                let children = val.children.into_iter().map(|c| c.into()).collect();
                element::Node::ListItem {
                    children,
                    spread: val.spread,
                    checked: val.checked,
                }
            }
            Node::Break(_) => element::Node::Break,
            Node::Code(raw) => element::Node::CodeBlock {
                code: raw.value.into(),
                lang: raw.lang.map(|s| s.into()),
            },
            Node::Heading(val) => {
                let mut paragraph = Paragraph::default();
                val.children.iter().for_each(|c| {
                    parse_paragraph(&mut paragraph, c);
                });

                element::Node::Heading {
                    level: val.depth,
                    children: paragraph,
                }
            }
            Node::Math(val) => element::Node::CodeBlock {
                code: val.value.into(),
                lang: Some("math".into()),
            },
            Node::Html(val) => match parse_html(&val.value) {
                Ok(el) => el,
                Err(err) => {
                    if cfg!(debug_assertions) {
                        eprintln!("[markdown] error parsing html: {:#?}", err);
                    }

                    element::Node::Paragraph(val.value.into())
                }
            },
            Node::MdxFlowExpression(val) => element::Node::CodeBlock {
                code: val.value.into(),
                lang: Some("mdx".into()),
            },
            Node::Yaml(val) => element::Node::CodeBlock {
                code: val.value.into(),
                lang: Some("yaml".into()),
            },
            Node::Toml(val) => element::Node::CodeBlock {
                code: val.value.into(),
                lang: Some("toml".into()),
            },
            Node::MdxJsxTextElement(val) => {
                println!("MdxJsxTextElement: {:#?}", val);
                let mut paragraph = Paragraph::default();
                val.children.iter().for_each(|c| {
                    parse_paragraph(&mut paragraph, c);
                });
                element::Node::Paragraph(paragraph)
            }
            Node::MdxJsxFlowElement(val) => {
                println!("MdxJsxFlowElement: {:#?}", val);
                let mut paragraph = Paragraph::default();
                val.children.iter().for_each(|c| {
                    parse_paragraph(&mut paragraph, c);
                });
                element::Node::Paragraph(paragraph)
            }
            Node::ThematicBreak(_) => element::Node::Divider,
            Node::Table(val) => {
                let mut table = Table::default();
                val.children.iter().for_each(|c| {
                    if let Node::TableRow(row) = c {
                        parse_table_row(&mut table, row);
                    }
                });

                element::Node::Table(table)
            }
            _ => {
                if cfg!(debug_assertions) {
                    eprintln!("[markdown] unsupported node: {:#?}", value);
                }
                element::Node::Unknown
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MarkdownView;

    #[test]
    fn test_parse() {
        let source = include_str!("../../../story/examples/markdown.md");
        let mut renderer = MarkdownView::new(source);
        renderer.parse_if_needed();
        // println!("{:#?}", renderer.root);
    }
}
