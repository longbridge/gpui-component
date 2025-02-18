use gpui::{div, prelude::FluentBuilder as _, IntoElement, ParentElement, Render, Styled, Window};
use markdown::{
    mdast::{self, Node},
    ParseOptions,
};

use crate::v_flex;

use super::element::{self, TextNode};

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

impl From<mdast::Node> for element::Node {
    fn from(value: Node) -> Self {
        fn parse_text(node: &Node) -> &str {
            match node {
                Node::Text(text) => return &text.value,
                Node::Code(code) => return &code.value,
                _ => "",
            }
        }

        match value {
            Node::Root(val) => {
                let children = val.children.into_iter().map(|c| c.into()).collect();
                element::Node::Root(children)
            }
            Node::Paragraph(val) => {
                let children = val.children.into_iter().map(|c| c.into()).collect();
                element::Node::Paragraph(children)
            }
            Node::Blockquote(val) => {
                let children = val.children.into_iter().map(|c| c.into()).collect();
                element::Node::Blockquote(children)
            }
            Node::Text(val) => {
                return element::Node::Text(TextNode {
                    text: val.value.into(),
                    ..Default::default()
                })
            }
            Node::List(list) => {
                let children = list.children.into_iter().map(|c| c.into()).collect();
                element::Node::List {
                    ordered: list.ordered,
                    children,
                }
            }
            Node::ListItem(item) => {
                let children = item.children.into_iter().map(|c| c.into()).collect();
                element::Node::ListItem {
                    children,
                    checked: item.checked,
                }
            }
            Node::Break(_) => element::Node::Break,
            Node::InlineCode(code) => element::Node::Text(TextNode {
                text: code.value.into(),
                code: true,
                ..Default::default()
            }),
            Node::Delete(raw) => {
                let mut text = String::new();
                for child in raw.children {
                    text.push_str(parse_text(&child));
                }

                element::Node::Text(TextNode {
                    text: text.into(),
                    strikethrough: true,
                    ..Default::default()
                })
            }
            Node::Emphasis(raw) => {
                let mut text = String::new();
                for child in raw.children {
                    text.push_str(parse_text(&child));
                }

                element::Node::Text(TextNode {
                    text: text.into(),
                    italic: true,
                    ..Default::default()
                })
            }
            Node::Strong(raw) => {
                let mut text = String::new();
                for child in raw.children {
                    text.push_str(parse_text(&child));
                }

                element::Node::Text(TextNode {
                    text: text.into(),
                    bold: true,
                    ..Default::default()
                })
            }
            Node::Image(image) => element::Node::Image {
                url: image.url.into(),
                title: image.title.map(|t| t.into()),
                alt: Some(image.alt.into()),
                width: None,
                height: None,
            },
            Node::Link(link) => {
                let children = link.children.into_iter().map(|c| c.into()).collect();
                element::Node::Link {
                    url: link.url.into(),
                    title: link.title.map(|t| t.into()),
                    children,
                }
            }
            Node::Code(raw) => element::Node::CodeBlock {
                code: raw.value.into(),
                lang: raw.lang.map(|s| s.into()),
            },
            Node::Heading(heading) => {
                let children = heading.children.into_iter().map(|c| c.into()).collect();
                element::Node::Heading {
                    level: heading.depth,
                    children,
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
        let _ = MarkdownView::new(source);
        // println!("{:#?}", renderer.root);
    }
}
