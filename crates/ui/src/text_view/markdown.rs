use gpui::{div, prelude::FluentBuilder as _, IntoElement, ParentElement, Render, Styled, Window};
use markdown::{
    mdast::{self, Node},
    ParseOptions,
};

use crate::v_flex;

use super::ast::{self, TextNode};

pub struct MarkdownView {
    root: Result<ast::Node, markdown::message::Message>,
}

impl MarkdownView {
    pub fn new(source: &str) -> Self {
        let node = markdown::to_mdast(source, &ParseOptions::default());
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

impl From<mdast::Node> for ast::Node {
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
                ast::Node::Root(children)
            }
            Node::Paragraph(val) => {
                let children = val.children.into_iter().map(|c| c.into()).collect();
                ast::Node::Paragraph(children)
            }
            Node::Blockquote(val) => {
                let children = val.children.into_iter().map(|c| c.into()).collect();
                ast::Node::Blockquote(children)
            }
            Node::Text(val) => {
                return ast::Node::Text(TextNode {
                    text: val.value.into(),
                    ..Default::default()
                })
            }
            Node::List(list) => {
                let children = list.children.into_iter().map(|c| c.into()).collect();
                ast::Node::List {
                    ordered: list.ordered,
                    children,
                }
            }
            Node::Break(_) => ast::Node::Break,
            Node::InlineCode(code) => ast::Node::Text(TextNode {
                text: code.value.into(),
                code: true,
                ..Default::default()
            }),
            Node::Delete(raw) => {
                let mut text = String::new();
                for child in raw.children {
                    text.push_str(parse_text(&child));
                }

                ast::Node::Text(TextNode {
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

                ast::Node::Text(TextNode {
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

                ast::Node::Text(TextNode {
                    text: text.into(),
                    bold: true,
                    ..Default::default()
                })
            }
            Node::Image(image) => ast::Node::Image {
                url: image.url.into(),
                title: image.title.map(|t| t.into()),
                alt: Some(image.alt.into()),
                width: None,
                height: None,
            },
            Node::Link(link) => {
                let children = link.children.into_iter().map(|c| c.into()).collect();
                ast::Node::Link {
                    url: link.url.into(),
                    title: link.title.map(|t| t.into()),
                    children,
                }
            }
            Node::Code(raw) => ast::Node::CodeBlock {
                code: raw.value.into(),
                lang: raw.lang.map(|s| s.into()),
            },
            Node::Heading(heading) => {
                let children = heading.children.into_iter().map(|c| c.into()).collect();
                ast::Node::Heading {
                    level: heading.depth,
                    children,
                }
            }
            _ => ast::Node::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MarkdownView;

    #[test]
    fn test_parse() {
        let source = include_str!("../../../story/examples/markdown.md");
        let _ = MarkdownView::parse(source).unwrap();
        // println!("{:#?}", renderer.root);
    }
}
