extern crate markup5ever_rcdom as rcdom;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::{
    div, px, relative, Context, DefiniteLength, IntoElement, ParentElement as _, Render,
    SharedString,
};
use html5ever::tendril::TendrilSink;
use html5ever::{local_name, parse_document, LocalName, ParseOpts};
use markup5ever_rcdom::{Node, NodeData, RcDom};

use super::element::{self, ImageNode, InlineTextStyle, Paragraph, Table, TableRow};

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

    let mut inline_text_buf = String::new();
    let node: element::Node = parse_node(&dom.document, &mut inline_text_buf);
    let node = node.compact();
    // println!("----- dom: {:?}", node);

    Ok(node)
}

pub struct HtmlView {
    source: SharedString,
    parsed: bool,
    node: Option<element::Node>,
}

impl HtmlView {
    pub fn new(source: impl Into<SharedString>) -> Self {
        Self {
            source: source.into(),
            parsed: false,
            node: None,
        }
    }

    pub fn set_source(&mut self, source: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.source = source.into();
        self.parsed = false;
        self.node = None;
        cx.notify();
    }

    fn parse_if_needed(&mut self) {
        if !self.parsed {
            self.node = parse_html(&self.source).ok();
            self.parsed = true;
        }
    }
}

impl Render for HtmlView {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        self.parse_if_needed();

        if let Some(node) = &self.node {
            div().child(node.clone())
        } else {
            div()
        }
    }
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

fn parse_table_row(table: &mut Table, node: &Rc<Node>) {
    let mut row = TableRow::default();
    let mut count = 0;
    for child in node.children.borrow().iter() {
        match child.data {
            NodeData::Element {
                ref name,
                ref attrs,
                ..
            } if name.local == local_name!("td") || name.local == local_name!("th") => {
                if child.children.borrow().is_empty() {
                    continue;
                }

                count += 1;
                parse_table_cell(&mut row, child, attrs);
            }
            _ => {}
        }
    }

    if count > 0 {
        table.children.push(row);
    }
}

fn parse_table_cell(
    row: &mut element::TableRow,
    node: &Rc<Node>,
    attrs: &RefCell<Vec<html5ever::Attribute>>,
) {
    let mut paragraph = Paragraph::default();
    for child in node.children.borrow().iter() {
        parse_paragraph(&mut paragraph, child);
    }
    let width = attr_width_height(attrs).0;
    let table_cell = element::TableCell {
        children: paragraph,
        width,
    };
    row.children.push(table_cell);
}

fn parse_paragraph(
    paragraph: &mut Paragraph,
    node: &Rc<Node>,
) -> (String, Vec<(Range<usize>, InlineTextStyle)>) {
    let mut text = String::new();
    let mut marks = vec![];

    match &node.data {
        NodeData::Text { ref contents } => {
            text.push_str(&contents.borrow().trim());
        }
        NodeData::Element { name, attrs, .. } => match name.local {
            local_name!("em") | local_name!("i") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    text.push_str(&child_text);
                    marks.extend(child_marks);
                }
                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        italic: true,
                        ..Default::default()
                    },
                ));
                paragraph.push(element::TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("strong") | local_name!("b") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    text.push_str(&child_text);
                    marks.extend(child_marks);
                }
                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        bold: true,
                        ..Default::default()
                    },
                ));
                paragraph.push(element::TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("del") | local_name!("s") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    text.push_str(&child_text);
                    marks.extend(child_marks);
                }
                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        strikethrough: true,
                        ..Default::default()
                    },
                ));
                paragraph.push(element::TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("code") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    text.push_str(&child_text);
                    marks.extend(child_marks);
                }
                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        code: true,
                        ..Default::default()
                    },
                ));
                paragraph.push(element::TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("a") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    text.push_str(&child_text);
                    marks.extend(child_marks);
                }
                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        link: Some(element::LinkMark {
                            url: attr_value(&attrs, local_name!("href")).unwrap().into(),
                            title: attr_value(&attrs, local_name!("title")).map(Into::into),
                        }),
                        ..Default::default()
                    },
                ));
                paragraph.push(element::TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("img") => {
                let src = attr_value(&attrs, local_name!("src")).unwrap();
                let alt = attr_value(&attrs, local_name!("alt"));
                let title = attr_value(&attrs, local_name!("title"));
                let (width, height) = attr_width_height(&attrs);

                paragraph.set_image(ImageNode {
                    url: src.into(),
                    alt: alt.map(Into::into),
                    width,
                    height,
                    title: title.map(Into::into),
                });
            }
            _ => {
                if cfg!(debug_assertions) {
                    eprintln!("[html] unsupported node: {:#?}", node);
                }
            }
        },
        _ => {
            if cfg!(debug_assertions) {
                eprintln!("[html] unsupported node: {:#?}", node);
            }
        }
    }

    (text, marks)
}

fn parse_node(node: &Rc<Node>, inline_text_buf: &mut String) -> element::Node {
    match node.data {
        NodeData::Text { ref contents } => {
            let text = contents.borrow().trim().to_string();
            inline_text_buf.push_str(&text);

            element::Node::Ignore
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
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
                    children.push(parse_node(child, inline_text_buf));
                }
                let text = inline_text_buf.clone();
                inline_text_buf.clear();

                element::Node::Root {
                    children,
                    text: if text.is_empty() { None } else { Some(text) },
                }
            }
            local_name!("table") => {
                let mut table = Table::default();
                for child in node.children.borrow().iter() {
                    match child.data {
                        NodeData::Element { ref name, .. }
                            if name.local == local_name!("tbody")
                                || name.local == local_name!("thead") =>
                        {
                            for sub_child in child.children.borrow().iter() {
                                parse_table_row(&mut table, &sub_child);
                            }
                        }
                        _ => {
                            parse_table_row(&mut table, &child);
                        }
                    }
                }

                element::Node::Table(table)
            }
            _ => {
                let mut children: Vec<element::Node> = vec![];
                for child in node.children.borrow().iter() {
                    children.push(parse_node(child, inline_text_buf));
                }
                if children.is_empty() {
                    element::Node::Ignore
                } else {
                    let text = inline_text_buf.clone();
                    inline_text_buf.clear();
                    element::Node::Root {
                        children,
                        text: if text.is_empty() { None } else { Some(text) },
                    }
                }
            }
        },
        NodeData::Document => {
            let mut children = vec![];
            for child in node.children.borrow().iter() {
                children.push(parse_node(child, inline_text_buf));
            }
            let text = inline_text_buf.clone();
            inline_text_buf.clear();
            element::Node::Root {
                children,
                text: if text.is_empty() { None } else { Some(text) },
            }
        }
        NodeData::Doctype { .. } => element::Node::Ignore,
        NodeData::Comment { .. } => element::Node::Ignore,
        NodeData::ProcessingInstruction { .. } => element::Node::Ignore,
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
