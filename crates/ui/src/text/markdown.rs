use std::time::Instant;

use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, Element, ElementId, IntoElement,
    ParentElement, SharedString, Styled, Window,
};
use markdown::{
    mdast::{self, Node},
    ParseOptions,
};

use crate::v_flex;

use super::{
    element::{
        self, CodeBlock, ImageNode, InlineTextStyle, LinkMark, Paragraph, Span, Table, TableRow,
    },
    html::parse_html,
    TextViewStyle,
};

/// Markdown GFM renderer
///
/// This is design goal is to be able to most common Markdown (GFM) features
/// to let us to display rich text in our application.
///
/// See also [`super::TextView`]
#[derive(Clone)]
pub(super) struct MarkdownElement {
    id: ElementId,
    pub(super) text: SharedString,
    style: TextViewStyle,
}

impl MarkdownElement {
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
pub struct MarkdownState {
    raw: SharedString,
    root: Option<Result<element::Node, SharedString>>,
    style: TextViewStyle,
    _last_parsed: Option<Instant>,
}

impl MarkdownState {
    fn parse_if_needed(&mut self, new_text: SharedString, style: &TextViewStyle, cx: &mut App) {
        let is_changed = self.raw != new_text || self.style != *style;

        if self.root.is_some() && !is_changed {
            return;
        }

        if let Some(last_parsed) = self._last_parsed {
            if last_parsed.elapsed().as_millis() < 500 {
                return;
            }
        }

        self.raw = new_text;
        // NOTE: About 100ms
        // let measure = crate::Measure::new("parse_markdown");
        self.root = Some(parse_markdown(&self.raw, &style, cx));
        // measure.end();
        self._last_parsed = Some(Instant::now());
        self.style = style.clone();
    }
}

impl IntoElement for MarkdownElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for MarkdownElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        window.with_element_state(id.unwrap(), |state, window| {
            let mut state: MarkdownState = state.unwrap_or_default();
            state.parse_if_needed(self.text.clone(), &self.style, cx);

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
                            .child("Error parsing Markdown")
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
        _: Option<&gpui::InspectorElementId>,
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
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        request_layout.paint(window, cx);
    }
}

/// Parse Markdown into a tree of nodes.
fn parse_markdown(
    raw: &str,
    style: &TextViewStyle,
    cx: &mut App,
) -> Result<element::Node, SharedString> {
    markdown::to_mdast(&raw, &ParseOptions::gfm())
        .map(|n| ast_to_node(n, style, cx))
        .map_err(|e| e.to_string().into())
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
                if el.is_break() {
                    text = "\n".to_owned();
                    paragraph.push(element::TextNode {
                        text: text.clone(),
                        marks: vec![(0..text.len(), InlineTextStyle::default())],
                    });
                } else {
                    if cfg!(debug_assertions) {
                        eprintln!("[markdown] unsupported inline html tag: {:#?}", el);
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

fn ast_to_node(value: mdast::Node, style: &TextViewStyle, cx: &mut App) -> element::Node {
    match value {
        Node::Root(val) => {
            let children = val
                .children
                .into_iter()
                .map(|c| ast_to_node(c, style, cx))
                .collect();
            element::Node::Root { children }
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
            let children = list
                .children
                .into_iter()
                .map(|c| ast_to_node(c, style, cx))
                .collect();
            element::Node::List {
                ordered: list.ordered,
                children,
            }
        }
        Node::ListItem(val) => {
            let children = val
                .children
                .into_iter()
                .map(|c| ast_to_node(c, style, cx))
                .collect();
            element::Node::ListItem {
                children,
                spread: val.spread,
                checked: val.checked,
            }
        }
        Node::Break(_) => element::Node::Break { html: false },
        Node::Code(raw) => element::Node::CodeBlock(CodeBlock::new(
            raw.value.into(),
            raw.lang.map(|s| s.into()),
            style,
            cx,
        )),
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
        Node::Math(val) => {
            element::Node::CodeBlock(CodeBlock::new(val.value.into(), None, style, cx))
        }
        Node::Html(val) => match parse_html(&val.value) {
            Ok(el) => el,
            Err(err) => {
                if cfg!(debug_assertions) {
                    eprintln!("[markdown] error parsing html: {:#?}", err);
                }

                element::Node::Paragraph(val.value.into())
            }
        },
        Node::MdxFlowExpression(val) => element::Node::CodeBlock(CodeBlock::new(
            val.value.into(),
            Some("mdx".into()),
            style,
            cx,
        )),
        Node::Yaml(val) => element::Node::CodeBlock(CodeBlock::new(
            val.value.into(),
            Some("yml".into()),
            style,
            cx,
        )),
        Node::Toml(val) => element::Node::CodeBlock(CodeBlock::new(
            val.value.into(),
            Some("toml".into()),
            style,
            cx,
        )),
        Node::MdxJsxTextElement(val) => {
            let mut paragraph = Paragraph::default();
            val.children.iter().for_each(|c| {
                parse_paragraph(&mut paragraph, c);
            });
            element::Node::Paragraph(paragraph)
        }
        Node::MdxJsxFlowElement(val) => {
            let mut paragraph = Paragraph::default();
            val.children.iter().for_each(|c| {
                parse_paragraph(&mut paragraph, c);
            });
            element::Node::Paragraph(paragraph)
        }
        Node::ThematicBreak(_) => element::Node::Divider,
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
