extern crate markup5ever_rcdom as rcdom;

use std::cell::RefCell;
use std::collections::HashMap;

use gpui::{px, relative, DefiniteLength};
use html5ever::tendril::TendrilSink;
use html5ever::{local_name, parse_document, LocalName, ParseOpts};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use super::element::{self, ImageNode, Paragraph};

pub(super) fn parse_html(source: &str) -> Result<element::Node, std::io::Error> {
    let opts = ParseOpts {
        ..Default::default()
    };
    let mut bytes = source.as_bytes();
    // Ref
    // https://github.com/servo/html5ever/blob/main/rcdom/examples/print-rcdom.rs
    let dom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut bytes)?;

    let node: element::Node = dom.document.into();

    Ok(node.compact())
}

fn attr_value(attrs: &RefCell<Vec<html5ever::Attribute>>, name: LocalName) -> Option<String> {
    attrs.borrow().iter().find_map(|attr| {
        if attr.name.local == name {
            Some(attr.value.to_string())
        } else {
            None
        }
    })
}

/// Get style properties to HashMap
/// TODO: Use cssparser to parse style attribute.
fn style_attrs(attrs: &RefCell<Vec<html5ever::Attribute>>) -> HashMap<String, String> {
    let mut styles = HashMap::new();
    let Some(css_text) = attr_value(attrs, local_name!("style")) else {
        return styles;
    };

    for decl in css_text.split(';') {
        for rule in decl.split(':') {
            let mut parts = rule.splitn(2, ':');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                styles.insert(
                    key.trim().to_lowercase().to_string(),
                    value.trim().to_string(),
                );
            }
        }
    }

    styles
}

/// Parse length value from style attribute.
///
/// When is percentage, it will be converted to relative length.
/// Else, it will be converted to pixels.
fn value_to_length(value: &str) -> Option<DefiniteLength> {
    if value.ends_with("px") {
        value
            .trim_end_matches("px")
            .parse()
            .ok()
            .map(|v| px(v).into())
    } else if value.ends_with("%") {
        value
            .trim_end_matches("%")
            .parse::<f32>()
            .ok()
            .map(|v| relative(v / 100.))
    } else {
        value
            .trim_end_matches("px")
            .parse()
            .ok()
            .map(|v| px(v).into())
    }
}

/// Get width, height from attributes or parse them from style attribute.
fn attr_width_height(
    attrs: &RefCell<Vec<html5ever::Attribute>>,
) -> (Option<DefiniteLength>, Option<DefiniteLength>) {
    let mut width = None;
    let mut height = None;

    if let Some(value) = attr_value(attrs, local_name!("width")) {
        width = value_to_length(&value);
    }

    if let Some(value) = attr_value(attrs, local_name!("height")) {
        height = value_to_length(&value);
    }

    if width.is_none() || height.is_none() {
        let styles = style_attrs(attrs);
        if width.is_none() {
            width = styles.get("width").and_then(|v| value_to_length(&v));
        }
        if height.is_none() {
            height = styles.get("height").and_then(|v| value_to_length(&v));
        }
    }

    (width, height)
}

impl From<Handle> for element::Node {
    fn from(node: Handle) -> Self {
        match &node.data {
            NodeData::Text { ref contents } => {
                let text = contents.borrow().trim().to_string();
                if text.len() > 0 {
                    element::Node::Paragraph(text.into())
                } else {
                    element::Node::Ignore
                }
            }
            NodeData::Element {
                ref name, attrs, ..
            } => match name.local {
                local_name!("br") => element::Node::Break,
                local_name!("img") => {
                    let Some(src) = attr_value(attrs, local_name!("src")) else {
                        if cfg!(debug_assertions) {
                            eprintln!("[html] Image node missing src attribute");
                        }
                        return element::Node::Ignore;
                    };

                    let alt = attr_value(attrs, local_name!("alt"));
                    let title = attr_value(attrs, local_name!("title"));
                    let (width, height) = attr_width_height(attrs);

                    element::Node::Paragraph(Paragraph::Image {
                        span: None,
                        image: ImageNode {
                            url: src.into(),
                            alt: alt.map(Into::into),
                            width,
                            height,
                            title: title.map(Into::into),
                        },
                    })
                }
                local_name!("div") => {
                    let mut children = vec![];
                    for child in node.children.borrow().iter() {
                        children.push(child.clone().into());
                    }
                    element::Node::Root(children)
                }
                _ => {
                    let mut children: Vec<element::Node> = vec![];
                    for child in node.children.borrow().iter() {
                        children.push(child.clone().into());
                    }
                    if children.is_empty() {
                        element::Node::Ignore
                    } else {
                        element::Node::Root(children)
                    }
                }
            },
            NodeData::Document => {
                let mut children = vec![];
                for child in node.children.borrow().iter() {
                    children.push(child.clone().into());
                }
                element::Node::Root(children)
            }
            NodeData::Doctype { .. } => element::Node::Ignore,
            NodeData::Comment { .. } => element::Node::Ignore,
            NodeData::ProcessingInstruction { .. } => element::Node::Ignore,
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{px, relative};

    use crate::text::element::{Node, Paragraph};

    #[test]
    fn value_to_length() {
        assert_eq!(super::value_to_length("100px"), Some(px(100.).into()));
        assert_eq!(super::value_to_length("100%"), Some(relative(1.)));
        assert_eq!(super::value_to_length("56%"), Some(relative(0.56)));
        assert_eq!(super::value_to_length("240"), Some(px(240.).into()));
    }

    #[test]
    fn test_image() {
        let html = r#"<img src="https://example.com/image.png" alt="Example" width="100" height="200" title="Example Image" />"#;
        let node = super::parse_html(html).unwrap();
        assert_eq!(
            node,
            Node::Paragraph(Paragraph::Image {
                span: None,
                image: super::ImageNode {
                    url: "https://example.com/image.png".to_string().into(),
                    alt: Some("Example".to_string().into()),
                    width: Some(px(100.).into()),
                    height: Some(px(200.).into()),
                    title: Some("Example Image".to_string().into())
                }
            })
        );

        let html = r#"<img src="https://example.com/image.png" alt="Example" style="width: 80%" title="Example Image" />"#;
        let node = super::parse_html(html).unwrap();
        assert_eq!(
            node,
            Node::Paragraph(Paragraph::Image {
                span: None,
                image: super::ImageNode {
                    url: "https://example.com/image.png".to_string().into(),
                    alt: Some("Example".to_string().into()),
                    width: Some(relative(0.8)),
                    height: None,
                    title: Some("Example Image".to_string().into())
                }
            })
        );
    }
}
