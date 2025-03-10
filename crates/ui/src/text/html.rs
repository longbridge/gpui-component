extern crate markup5ever_rcdom as rcdom;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, relative, AnyElement, DefiniteLength, Element, ElementId, IntoElement,
    ParentElement as _, SharedString, Styled as _, Window,
};
use html5ever::tendril::TendrilSink;
use html5ever::{local_name, parse_document, LocalName, ParseOpts};
use markup5ever_rcdom::{Node, NodeData, RcDom};

use crate::v_flex;

use super::element::{
    self, ImageNode, InlineTextStyle, LinkMark, Paragraph, Table, TableRow, TextNode,
};
use super::TextViewStyle;

const BLOCK_ELEMENTS: [&str; 33] = [
    "html",
    "body",
    "head",
    "address",
    "article",
    "aside",
    "blockquote",
    "details",
    "summary",
    "dialog",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "ul",
];

pub(super) fn parse_html(source: &str) -> Result<element::Node, SharedString> {
    let opts = ParseOpts {
        ..Default::default()
    };

    let bytes = cleanup_html(&source);
    let mut cursor = std::io::Cursor::new(bytes);
    // Ref
    // https://github.com/servo/html5ever/blob/main/rcdom/examples/print-rcdom.rs
    let dom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut cursor)
        .map_err(|e| SharedString::from(format!("{:?}", e)))?;

    let mut paragraph = Paragraph::default();
    // NOTE: The outer paragraph is not used.
    let node: element::Node = parse_node(&dom.document, &mut paragraph);
    let node = node.compact();

    Ok(node)
}

fn cleanup_html(source: &str) -> Vec<u8> {
    let mut cfg = minify_html::Cfg::default();
    cfg.keep_closing_tags = true;
    minify_html::minify(source.as_bytes(), &cfg)
}

#[derive(Clone)]
pub(super) struct HtmlElement {
    id: ElementId,
    pub(super) text: SharedString,
    style: TextViewStyle,
}

impl HtmlElement {
    pub(super) fn new(id: impl Into<ElementId>, raw: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: raw.into(),
            style: TextViewStyle::default(),
        }
    }

    /// Set the source of the markdown view.
    pub(crate) fn text(mut self, raw: impl Into<SharedString>) -> Self {
        self.text = raw.into();
        self
    }

    /// Set TextViewStyle.
    pub(crate) fn style(mut self, style: impl Into<TextViewStyle>) -> Self {
        self.style = style.into();
        self
    }
}

#[derive(Default)]
pub struct HtmlState {
    raw: SharedString,
    root: Option<Result<element::Node, SharedString>>,
}

impl HtmlState {
    fn parse_if_needed(&mut self, new_text: SharedString) {
        let is_changed = self.raw != new_text;

        if self.root.is_some() && !is_changed {
            return;
        }

        self.raw = new_text;
        self.root = Some(parse_html(&self.raw));
    }
}

impl IntoElement for HtmlElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for HtmlElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        Some(self.id.clone())
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        window.with_element_state(id.unwrap(), |state, window| {
            let mut state: HtmlState = state.unwrap_or_default();
            state.parse_if_needed(self.text.clone());

            let root = state
                .root
                .clone()
                .expect("BUG: root should not None, maybe parse_if_needed issue.");

            let mut el = div()
                .map(|this| match root {
                    Ok(node) => this.child(node.render(None, true, &self.style, window, cx)),
                    Err(err) => this.child(
                        v_flex()
                            .gap_1()
                            .child("Error parsing HTML")
                            .child(err.to_string()),
                    ),
                })
                .into_any_element();

            let layout_id = el.request_layout(window, cx);

            ((layout_id, el), state)
        })
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> Self::PrepaintState {
        request_layout.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        request_layout.paint(window, cx);
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

/// Trim text but leave at least one space.
///
/// - Before: " \r\n Hello world \t "
/// - After: " Hello world "
#[allow(dead_code)]
fn trim_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for (i, c) in text.chars().enumerate() {
        if c.is_whitespace() {
            if i > 0 && out.ends_with(' ') {
                continue;
            }
        }
        out.push(c);
    }

    out
}

fn parse_paragraph(
    paragraph: &mut Paragraph,
    node: &Rc<Node>,
) -> (String, Vec<(Range<usize>, InlineTextStyle)>) {
    let mut text = String::new();
    let mut marks = vec![];

    /// Append new_text and new_marks to text and marks.
    fn merge_child_text(
        text: &mut String,
        marks: &mut Vec<(Range<usize>, InlineTextStyle)>,
        new_text: &str,
        new_marks: &[(Range<usize>, InlineTextStyle)],
    ) {
        let offset = text.len();
        text.push_str(new_text);
        for (range, style) in new_marks {
            marks.push((range.start + offset..new_text.len() + offset, style.clone()));
        }
    }

    match &node.data {
        NodeData::Text { ref contents } => {
            let part = &contents.borrow();
            text.push_str(&part);
            paragraph.push_str(&text);
        }
        NodeData::Element { name, attrs, .. } => match name.local {
            local_name!("em") | local_name!("i") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    merge_child_text(&mut text, &mut marks, &child_text, &child_marks);
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
                    merge_child_text(&mut text, &mut marks, &child_text, &child_marks);
                }

                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        bold: true,
                        ..Default::default()
                    },
                ));
                paragraph.push(TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("del") | local_name!("s") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    merge_child_text(&mut text, &mut marks, &child_text, &child_marks);
                }
                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        strikethrough: true,
                        ..Default::default()
                    },
                ));
                paragraph.push(TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("code") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    merge_child_text(&mut text, &mut marks, &child_text, &child_marks);
                }
                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        code: true,
                        ..Default::default()
                    },
                ));
                paragraph.push(TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("a") => {
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    merge_child_text(&mut text, &mut marks, &child_text, &child_marks);
                }

                marks.push((
                    0..text.len(),
                    InlineTextStyle {
                        link: Some(LinkMark {
                            url: attr_value(&attrs, local_name!("href")).unwrap().into(),
                            title: attr_value(&attrs, local_name!("title")).map(Into::into),
                        }),
                        ..Default::default()
                    },
                ));
                paragraph.push(TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
            local_name!("img") => {
                let Some(src) = attr_value(attrs, local_name!("src")) else {
                    if cfg!(debug_assertions) {
                        eprintln!("[html] Image node missing src attribute");
                    }
                    return (text, marks);
                };

                let alt = attr_value(attrs, local_name!("alt"));
                let title = attr_value(attrs, local_name!("title"));
                let (width, height) = attr_width_height(attrs);

                paragraph.set_image(ImageNode {
                    url: src.into(),
                    alt: alt.map(Into::into),
                    width,
                    height,
                    title: title.map(Into::into),
                });
            }

            _ => {
                // All unknown tags to as text
                let mut child_paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                    merge_child_text(&mut text, &mut marks, &child_text, &child_marks);
                }
                paragraph.push(element::TextNode {
                    text: text.clone(),
                    marks: marks.clone(),
                });
            }
        },
        _ => {
            let mut child_paragraph = Paragraph::default();
            for child in node.children.borrow().iter() {
                let (child_text, child_marks) = parse_paragraph(&mut child_paragraph, &child);
                merge_child_text(&mut text, &mut marks, &child_text, &child_marks);
            }
            paragraph.push(element::TextNode {
                text: text.clone(),
                marks: marks.clone(),
            });
        }
    }

    (text, marks)
}

fn parse_node(node: &Rc<Node>, paragraph: &mut Paragraph) -> element::Node {
    match node.data {
        NodeData::Text { ref contents } => {
            let text = contents.borrow().to_string();
            if text.len() > 0 {
                paragraph.push_str(&text);
            }

            element::Node::Ignore
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => match name.local {
            local_name!("br") => element::Node::Break { html: true },
            local_name!("h1")
            | local_name!("h2")
            | local_name!("h3")
            | local_name!("h4")
            | local_name!("h5")
            | local_name!("h6") => {
                let mut children = vec![];
                if !paragraph.is_empty() {
                    children.push(element::Node::Paragraph(paragraph.clone()));
                    paragraph.clear();
                }

                let level = name
                    .local
                    .chars()
                    .last()
                    .unwrap_or('6')
                    .to_digit(10)
                    .unwrap_or(6) as u8;

                let mut paragraph = Paragraph::default();
                for child in node.children.borrow().iter() {
                    parse_paragraph(&mut paragraph, child);
                }

                let heading = element::Node::Heading {
                    level,
                    children: paragraph,
                };
                if children.len() > 0 {
                    children.push(heading);

                    element::Node::Root { children }
                } else {
                    heading
                }
            }
            local_name!("img") => {
                let mut children = vec![];
                if !paragraph.is_empty() {
                    children.push(element::Node::Paragraph(paragraph.clone()));
                    paragraph.clear();
                }

                let Some(src) = attr_value(attrs, local_name!("src")) else {
                    if cfg!(debug_assertions) {
                        eprintln!("[html] Image node missing src attribute");
                    }
                    return element::Node::Ignore;
                };

                let alt = attr_value(&attrs, local_name!("alt"));
                let title = attr_value(&attrs, local_name!("title"));
                let (width, height) = attr_width_height(&attrs);

                let image = Paragraph::Image {
                    span: None,
                    image: ImageNode {
                        url: src.into(),
                        title: title.map(Into::into),
                        alt: alt.map(Into::into),
                        width,
                        height,
                    },
                };

                if children.len() > 0 {
                    children.push(element::Node::Paragraph(image));
                    element::Node::Root { children }
                } else {
                    element::Node::Paragraph(image)
                }
            }
            local_name!("ul") | local_name!("ol") => {
                let mut children = vec![];
                if !paragraph.is_empty() {
                    children.push(element::Node::Paragraph(paragraph.clone()));
                    paragraph.clear();
                }

                let ordered = name.local == local_name!("ol");

                let mut list_children = vec![];
                for child in node.children.borrow().iter() {
                    let mut child_paragraph = Paragraph::default();
                    list_children.push(parse_node(child, &mut child_paragraph));
                }

                let list = element::Node::List {
                    children: list_children,
                    ordered,
                };
                if children.len() > 0 {
                    children.push(list);
                    element::Node::Root { children }
                } else {
                    list
                }
            }
            local_name!("li") => {
                let mut children = vec![];
                for child in node.children.borrow().iter() {
                    let mut child_paragraph = Paragraph::default();
                    children.push(parse_node(child, &mut child_paragraph));
                    if child_paragraph.text_len() > 0 {
                        children.push(element::Node::Paragraph(child_paragraph.clone()));
                        child_paragraph.clear();
                    }
                }

                if !paragraph.is_empty() {
                    children.push(element::Node::Paragraph(paragraph.clone()));
                    paragraph.clear();
                }

                element::Node::ListItem {
                    children,
                    spread: false,
                    checked: None,
                }
            }
            local_name!("table") => {
                let mut children = vec![];
                if !paragraph.is_empty() {
                    children.push(element::Node::Paragraph(paragraph.clone()));
                    paragraph.clear();
                }

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

                let table = element::Node::Table(table);
                if children.len() > 0 {
                    children.push(table);
                    element::Node::Root { children }
                } else {
                    table
                }
            }
            local_name!("blockquote") => {
                let mut children = vec![];
                if !paragraph.is_empty() {
                    children.push(element::Node::Paragraph(paragraph.clone()));
                    paragraph.clear();
                }

                let mut blockquote = Paragraph::default();
                for (i, child) in node.children.borrow().iter().enumerate() {
                    if i > 0 {
                        blockquote.push_str("\n");
                    }
                    parse_paragraph(&mut blockquote, child);
                }
                children.push(element::Node::Blockquote(blockquote));

                element::Node::Root { children: children }
            }
            _ => {
                if BLOCK_ELEMENTS.contains(&name.local.trim()) {
                    let mut children: Vec<element::Node> = vec![];

                    // Case:
                    //
                    // Hello <p>Inner text of block element</p> World

                    // Insert before text as a node -- The "Hello"
                    if !paragraph.is_empty() {
                        children.push(element::Node::Paragraph(paragraph.clone()));
                        paragraph.clear();
                    }

                    // Inner of the block element -- The "Inner text of block element"
                    for child in node.children.borrow().iter() {
                        children.push(parse_node(child, paragraph));
                    }

                    // if !paragraph.is_empty() {
                    //     children.push(element::Node::Paragraph(paragraph.clone()));
                    //     paragraph.clear();
                    // }

                    if children.is_empty() {
                        element::Node::Ignore
                    } else {
                        element::Node::Root { children }
                    }
                } else {
                    // Others to as Inline
                    parse_paragraph(paragraph, node);

                    if paragraph.is_image() {
                        let image = paragraph.clone();
                        paragraph.clear();
                        element::Node::Paragraph(image)
                    } else {
                        element::Node::Ignore
                    }
                }
            }
        },
        NodeData::Document => {
            let mut children = vec![];
            for child in node.children.borrow().iter() {
                children.push(parse_node(child, paragraph));
            }

            if !paragraph.is_empty() {
                children.push(element::Node::Paragraph(paragraph.clone()));
                paragraph.clear();
            }

            element::Node::Root { children }
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

    use super::trim_text;

    #[test]
    fn test_cleanup_html() {
        let html = r#"<p>
            and
            <code>code</code>
            text
        </p>"#;
        let cleaned = super::cleanup_html(html);
        assert_eq!(
            String::from_utf8(cleaned).unwrap(),
            "<p>and <code>code</code> text</p>"
        );

        let html = r#"<p>
            and
            <em>   <code>code</code>   <i>italic</i>   </em>
            text
        </p>"#;
        let cleaned = super::cleanup_html(html);
        assert_eq!(
            String::from_utf8(cleaned).unwrap(),
            "<p>and <em> <code>code</code> <i>italic</i> </em> text</p>"
        );
    }

    #[test]
    fn test_trim_text() {
        assert_eq!(trim_text("  \n\tHello world \t\r "), " Hello world ",);
    }

    #[test]
    fn test_keep_spaces() {
        let html = r#"<p>and <code>code</code> text</p>"#;
        let node = super::parse_html(html).unwrap();
        assert_eq!(node.to_markdown(), "and `code` text");

        let html = r#"
            <div>
            <p>
                and
                <em>   <code>code</code>   <i>italic</i>   </em>
                text
            </p>
            <p>
                <img src="https://example.com/image.png" alt="Example" width="100" height="200" title="Example Image" />
            </p>
            <ul>
                <li>Item 1</li>
                <li>Item 2
                </li>
            </ul>
            </div>
        "#;
        let node = super::parse_html(html).unwrap();
        assert_eq!(
            node.to_markdown(),
            indoc::indoc! {r#"
            and * code italic * text

            ![Example](https://example.com/image.png "Example Image")

            - Item 1
            - Item 2
            "#}
            .trim()
        );
    }

    #[test]
    fn test_value_to_length() {
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
