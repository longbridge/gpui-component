use gpui::{div, prelude::FluentBuilder as _, IntoElement, ParentElement, Render, Styled, Window};
use markdown::{
    mdast::{self, Node},
    ParseOptions,
};

use crate::v_flex;

use super::element::{self, InlineTextStyle, LinkMark, Paragraph, Span};

/// Markdown GFM renderer
pub struct MarkdownView {
    root: Result<element::Node, markdown::message::Message>,
}

impl MarkdownView {
    pub fn new(source: &str) -> Self {
        let node = markdown::to_mdast(source, &ParseOptions::gfm());
        Self {
            root: node.map(|n| n.into()),
        }
    }
}

impl Render for MarkdownView {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<'_, Self>) -> impl IntoElement {
        div().map(|this| match self.root.clone() {
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

fn parse_paragraph(paragraph: &mut Paragraph, node: &mdast::Node) -> String {
    paragraph.span = node.position().map(|pos| Span {
        start: pos.start.offset,
        end: pos.end.offset,
    });

    let mut text = String::new();

    match node {
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
        Node::Code(val) => {
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
        _ => {}
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
                let mut paragraph = Paragraph::default();
                val.children.iter().for_each(|c| {
                    parse_paragraph(&mut paragraph, c);
                });
                element::Node::ListItem {
                    children: paragraph,
                    checked: val.checked,
                }
            }
            Node::Break(_) => element::Node::Break,
            Node::Image(image) => element::Node::Image {
                url: image.url.into(),
                title: image.title.map(|t| t.into()),
                alt: Some(image.alt.into()),
                width: None,
                height: None,
            },
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
            _ => element::Node::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MarkdownView;

    #[test]
    fn test_parse() {
        let source = include_str!("../../../story/examples/markdown.md");
        let _renderer = MarkdownView::new(source);
        // println!("{:#?}", _renderer.root);
    }
}
