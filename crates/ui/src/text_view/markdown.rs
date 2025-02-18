use gpui::{
    div, prelude::FluentBuilder as _, Context, IntoElement, ParentElement, Render, SharedString,
    Styled, Window,
};
use markdown::{
    mdast::{self, Node},
    ParseOptions,
};

use crate::v_flex;

use super::element::{self, ImageNode, InlineTextStyle, LinkMark, Paragraph, Span};

/// Markdown GFM renderer
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
        println!("{:#?}", _renderer.root);
    }
}
