use std::{
    collections::HashMap,
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::{
    AnyElement, App, DefiniteLength, Div, ElementId, FontStyle, FontWeight, Half, HighlightStyle,
    InteractiveElement as _, IntoElement, Length, ObjectFit, ParentElement, SharedString,
    SharedUri, StatefulInteractiveElement, Styled, StyledImage as _, Window, div, img,
    prelude::FluentBuilder as _, px, relative, rems,
};
use markdown::mdast;
use ropey::Rope;

use crate::{
    ActiveTheme as _, Icon, IconName, StyledExt, h_flex,
    highlighter::{HighlightTheme, SyntaxHighlighter},
    text::{
        CodeBlockActionsFn,
        document::NodeRenderOptions,
        inline::{Inline, InlineState},
    },
    tooltip::Tooltip,
    v_flex,
};

use super::{TextViewStyle, utils::list_item_prefix};

/// The block-level nodes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BlockNode {
    /// Something like a Div container in HTML.
    Root {
        children: Vec<BlockNode>,
        span: Option<Span>,
    },
    Paragraph(Paragraph),
    Heading {
        level: u8,
        children: Paragraph,
        span: Option<Span>,
    },
    Blockquote {
        children: Vec<BlockNode>,
        span: Option<Span>,
    },
    List {
        /// Only contains ListItem, others will be ignored
        children: Vec<BlockNode>,
        ordered: bool,
        span: Option<Span>,
    },
    ListItem {
        children: Vec<BlockNode>,
        spread: bool,
        /// Whether the list item is checked, if None, it's not a checkbox
        checked: Option<bool>,
        span: Option<Span>,
    },
    CodeBlock(CodeBlock),
    Table(Table),
    Break {
        html: bool,
        span: Option<Span>,
    },
    Divider {
        span: Option<Span>,
    },
    /// Use for to_markdown get raw definition
    Definition {
        identifier: SharedString,
        url: SharedString,
        title: Option<SharedString>,
        span: Option<Span>,
    },
    Unknown,
}

impl BlockNode {
    pub(super) fn is_list_item(&self) -> bool {
        matches!(self, Self::ListItem { .. })
    }

    pub(super) fn is_break(&self) -> bool {
        matches!(self, Self::Break { .. })
    }

    /// Combine all children, omitting the empt parent nodes.
    pub(super) fn compact(self) -> BlockNode {
        match self {
            Self::Root { mut children, .. } if children.len() == 1 => children.remove(0).compact(),
            _ => self,
        }
    }

    /// Get the span of the node.
    pub(super) fn span(&self) -> Option<Span> {
        match self {
            BlockNode::Root { span, .. } => *span,
            BlockNode::Paragraph(paragraph) => paragraph.span,
            BlockNode::Heading { span, .. } => *span,
            BlockNode::Blockquote { span, .. } => *span,
            BlockNode::List { span, .. } => *span,
            BlockNode::ListItem { span, .. } => *span,
            BlockNode::CodeBlock(code_block) => code_block.span,
            BlockNode::Table(table) => table.span,
            BlockNode::Break { span, .. } => *span,
            BlockNode::Divider { span, .. } => *span,
            BlockNode::Definition { span, .. } => *span,
            BlockNode::Unknown { .. } => None,
        }
    }

    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();
        match self {
            BlockNode::Root { children, .. } => {
                let mut block_text = String::new();
                for c in children.iter() {
                    block_text.push_str(&c.selected_text());
                }
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Paragraph(paragraph) => {
                let mut block_text = String::new();
                block_text.push_str(&paragraph.selected_text());
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Heading { children, .. } => {
                let mut block_text = String::new();
                block_text.push_str(&children.selected_text());
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::List { children, .. } => {
                for c in children.iter() {
                    text.push_str(&c.selected_text());
                }
            }
            BlockNode::ListItem { children, .. } => {
                for c in children.iter() {
                    text.push_str(&c.selected_text());
                }
            }
            BlockNode::Blockquote { children, .. } => {
                let mut block_text = String::new();
                for c in children.iter() {
                    block_text.push_str(&c.selected_text());
                }

                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Table(table) => {
                let mut block_text = String::new();
                for row in table.children.iter() {
                    let mut row_texts = vec![];
                    for cell in row.children.iter() {
                        row_texts.push(cell.children.selected_text());
                    }
                    if !row_texts.is_empty() {
                        block_text.push_str(&row_texts.join(" "));
                        block_text.push('\n');
                    }
                }

                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::CodeBlock(code_block) => {
                let block_text = code_block.selected_text();
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Definition { .. }
            | BlockNode::Break { .. }
            | BlockNode::Divider { .. }
            | BlockNode::Unknown { .. } => {}
        }

        text
    }
}

#[allow(unused)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LinkMark {
    pub url: SharedString,
    /// Optional identifier for footnotes.
    pub identifier: Option<SharedString>,
    pub title: Option<SharedString>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextMark {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub link: Option<LinkMark>,
}

impl TextMark {
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    pub fn code(mut self) -> Self {
        self.code = true;
        self
    }

    pub fn link(mut self, link: impl Into<LinkMark>) -> Self {
        self.link = Some(link.into());
        self
    }
}

/// The bytes
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for ElementId {
    fn from(value: Span) -> Self {
        ElementId::Name(format!("md-{}:{}", value.start, value.end).into())
    }
}

#[allow(unused)]
#[derive(Debug, Default, Clone)]
pub struct ImageNode {
    pub url: SharedUri,
    pub link: Option<LinkMark>,
    pub title: Option<SharedString>,
    pub alt: Option<SharedString>,
    pub width: Option<DefiniteLength>,
    pub height: Option<DefiniteLength>,
}

impl ImageNode {
    pub fn title(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| self.alt.clone().unwrap_or_default())
            .to_string()
    }
}

impl PartialEq for ImageNode {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
            && self.link == other.link
            && self.title == other.title
            && self.alt == other.alt
            && self.width == other.width
            && self.height == other.height
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct InlineNode {
    /// The text content.
    pub(crate) text: SharedString,
    pub(crate) image: Option<ImageNode>,
    /// The text styles, each tuple contains the range of the text and the style.
    pub(crate) marks: Vec<(Range<usize>, TextMark)>,

    state: Arc<Mutex<InlineState>>,
}

impl PartialEq for InlineNode {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text && self.image == other.image && self.marks == other.marks
    }
}

impl InlineNode {
    pub(crate) fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            image: None,
            marks: vec![],
            state: Arc::new(Mutex::new(InlineState::default())),
        }
    }

    pub(crate) fn image(image: ImageNode) -> Self {
        let mut this = Self::new("");
        this.image = Some(image);
        this
    }

    pub(crate) fn marks(mut self, marks: Vec<(Range<usize>, TextMark)>) -> Self {
        self.marks = marks;
        self
    }
}

/// The paragraph element, contains multiple text nodes.
///
/// Unlike other Element, this is cloneable, because it is used in the Node AST.
/// We are keep the selection state inside this AST Nodes.
#[derive(Debug, Clone, Default)]
pub(crate) struct Paragraph {
    pub(super) span: Option<Span>,
    pub(super) children: Vec<InlineNode>,
    /// The link references in this paragraph, used for reference links.
    ///
    /// The key is the identifier, the value is the url.
    pub(super) link_refs: HashMap<SharedString, SharedString>,

    pub(crate) state: Arc<Mutex<InlineState>>,
}

impl PartialEq for Paragraph {
    fn eq(&self, other: &Self) -> bool {
        self.span == other.span
            && self.children == other.children
            && self.link_refs == other.link_refs
    }
}

impl Paragraph {
    pub(crate) fn new(text: String) -> Self {
        Self {
            span: None,
            children: vec![InlineNode::new(&text)],
            link_refs: HashMap::new(),
            state: Arc::new(Mutex::new(InlineState::default())),
        }
    }

    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();

        for c in self.children.iter() {
            let state = c.state.lock().unwrap();
            if let Some(selection) = &state.selection {
                let part_text = state.text.clone();
                text.push_str(&part_text[selection.start..selection.end]);
            }
        }

        let state = self.state.lock().unwrap();
        if let Some(selection) = &state.selection {
            let all_text = state.text.clone();
            text.push_str(&all_text[selection.start..selection.end]);
        }

        text
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Table {
    pub(crate) children: Vec<TableRow>,
    pub(crate) column_aligns: Vec<ColumnumnAlign>,
    pub(crate) span: Option<Span>,
}

impl Table {
    pub(crate) fn column_align(&self, index: usize) -> ColumnumnAlign {
        self.column_aligns.get(index).copied().unwrap_or_default()
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub(crate) enum ColumnumnAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl From<mdast::AlignKind> for ColumnumnAlign {
    fn from(value: mdast::AlignKind) -> Self {
        match value {
            mdast::AlignKind::None => ColumnumnAlign::Left,
            mdast::AlignKind::Left => ColumnumnAlign::Left,
            mdast::AlignKind::Center => ColumnumnAlign::Center,
            mdast::AlignKind::Right => ColumnumnAlign::Right,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TableRow {
    pub children: Vec<TableCell>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TableCell {
    pub children: Paragraph,
    pub width: Option<DefiniteLength>,
}

impl Paragraph {
    pub(crate) fn take(&mut self) -> Paragraph {
        std::mem::replace(
            self,
            Paragraph {
                span: None,
                children: vec![],
                link_refs: Default::default(),
                state: Arc::new(Mutex::new(InlineState::default())),
            },
        )
    }

    pub(crate) fn is_image(&self) -> bool {
        false
    }

    pub(crate) fn set_span(&mut self, span: Span) {
        self.span = Some(span);
    }

    pub(crate) fn push_str(&mut self, text: &str) {
        self.children.push(
            InlineNode::new(text.to_string()).marks(vec![(0..text.len(), TextMark::default())]),
        );
    }

    pub(crate) fn push(&mut self, text: InlineNode) {
        self.children.push(text);
    }

    pub(crate) fn push_image(&mut self, image: ImageNode) {
        self.children.push(InlineNode::image(image));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.children.is_empty()
            || self
                .children
                .iter()
                .all(|node| node.text.is_empty() && node.image.is_none())
    }

    /// Return length of children text.
    pub(crate) fn text_len(&self) -> usize {
        self.children
            .iter()
            .map(|node| node.text.len())
            .sum::<usize>()
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.children.extend(other.children);
    }
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    lang: Option<SharedString>,
    styles: Vec<(Range<usize>, HighlightStyle)>,
    state: Arc<Mutex<InlineState>>,
    pub span: Option<Span>,
}

impl PartialEq for CodeBlock {
    fn eq(&self, other: &Self) -> bool {
        self.lang == other.lang && self.styles == other.styles
    }
}

impl CodeBlock {
    /// Get the language of the code block.
    pub fn lang(&self) -> Option<SharedString> {
        self.lang.clone()
    }

    /// Get the code content of the code block.
    pub fn code(&self) -> SharedString {
        self.state.lock().unwrap().text.clone()
    }

    pub(crate) fn new(
        code: SharedString,
        lang: Option<SharedString>,
        highlight_theme: &HighlightTheme,
        span: Option<impl Into<Span>>,
    ) -> Self {
        let mut styles = vec![];
        if let Some(lang) = &lang {
            let mut highlighter = SyntaxHighlighter::new(&lang);
            highlighter.update(None, &Rope::from_str(code.as_str()));
            styles = highlighter.styles(&(0..code.len()), highlight_theme);
        };

        let state = Arc::new(Mutex::new(InlineState::default()));
        state.lock().unwrap().set_text(code);

        Self {
            lang,
            styles,
            state,
            span: span.map(|s| s.into()),
        }
    }

    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();
        let state = self.state.lock().unwrap();
        if let Some(selection) = &state.selection {
            let part_text = state.text.clone();
            text.push_str(&part_text[selection.start..selection.end]);
        }
        text
    }

    fn render(
        &self,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let style = &node_cx.style;

        div()
            .when(!options.is_last, |this| this.pb(style.paragraph_gap))
            .child(
                div()
                    .id(("codeblock", options.ix))
                    .p_3()
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().muted)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .relative()
                    .refine_style(&style.code_block)
                    .child(Inline::new(
                        "code",
                        self.state.clone(),
                        vec![],
                        self.styles.clone(),
                    ))
                    .when_some(node_cx.code_block_actions.clone(), |this, actions| {
                        this.child(
                            div()
                                .id("actions")
                                .absolute()
                                .top_2()
                                .right_2()
                                .bg(cx.theme().muted)
                                .rounded(cx.theme().radius)
                                .child(actions(&self, window, cx)),
                        )
                    }),
            )
            .into_any_element()
    }
}

/// A context for rendering nodes, contains link references.
#[derive(Default, Clone)]
pub(crate) struct NodeContext {
    /// The byte offset of the node in the original markdown text.
    /// Used for incremental updates.
    pub(crate) offset: usize,
    pub(crate) link_refs: HashMap<SharedString, LinkMark>,
    pub(crate) style: TextViewStyle,
    pub(crate) code_block_actions: Option<Arc<CodeBlockActionsFn>>,
}

impl NodeContext {
    pub(super) fn add_ref(&mut self, identifier: SharedString, link: LinkMark) {
        self.link_refs.insert(identifier, link);
    }
}

impl PartialEq for NodeContext {
    fn eq(&self, other: &Self) -> bool {
        self.link_refs == other.link_refs && self.style == other.style
        // Note: code_block_buttons is intentionally not compared (closures can't be compared)
    }
}

impl Paragraph {
    fn render(
        &self,
        node_cx: &NodeContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let span = self.span;
        let children = &self.children;

        let mut child_nodes: Vec<AnyElement> = vec![];

        let mut text = String::new();
        let mut highlights: Vec<(Range<usize>, HighlightStyle)> = vec![];
        let mut links: Vec<(Range<usize>, LinkMark)> = vec![];
        let mut offset = 0;

        let mut ix = 0;
        for inline_node in children {
            let text_len = inline_node.text.len();
            text.push_str(&inline_node.text);

            if let Some(image) = &inline_node.image {
                if text.len() > 0 {
                    inline_node
                        .state
                        .lock()
                        .unwrap()
                        .set_text(text.clone().into());
                    child_nodes.push(
                        Inline::new(
                            ix,
                            inline_node.state.clone(),
                            links.clone(),
                            highlights.clone(),
                        )
                        .into_any_element(),
                    );
                }
                child_nodes.push(
                    img(image.url.clone())
                        .id(ix)
                        .object_fit(ObjectFit::Contain)
                        .max_w(relative(1.))
                        .when_some(image.width, |this, width| this.w(width))
                        .when_some(image.link.clone(), |this, link| {
                            let title = image.title();
                            this.cursor_pointer()
                                .tooltip(move |window, cx| {
                                    Tooltip::new(title.clone()).build(window, cx)
                                })
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    cx.open_url(&link.url);
                                })
                        })
                        .into_any_element(),
                );

                text.clear();
                links.clear();
                highlights.clear();
                offset = 0;
            } else {
                let mut node_highlights = vec![];
                for (range, style) in &inline_node.marks {
                    let inner_range = (offset + range.start)..(offset + range.end);

                    let mut highlight = HighlightStyle::default();
                    if style.bold {
                        highlight.font_weight = Some(FontWeight::BOLD);
                    }
                    if style.italic {
                        highlight.font_style = Some(FontStyle::Italic);
                    }
                    if style.strikethrough {
                        highlight.strikethrough = Some(gpui::StrikethroughStyle {
                            thickness: gpui::px(1.),
                            ..Default::default()
                        });
                    }
                    if style.code {
                        highlight.background_color = Some(cx.theme().accent);
                    }

                    if let Some(mut link_mark) = style.link.clone() {
                        highlight.color = Some(cx.theme().link);
                        highlight.underline = Some(gpui::UnderlineStyle {
                            thickness: gpui::px(1.),
                            ..Default::default()
                        });

                        // convert link references, replace link
                        if let Some(identifier) = link_mark.identifier.as_ref() {
                            if let Some(mark) = node_cx.link_refs.get(identifier) {
                                link_mark = mark.clone();
                            }
                        }

                        links.push((inner_range.clone(), link_mark));
                    }

                    node_highlights.push((inner_range, highlight));
                }

                highlights = gpui::combine_highlights(highlights, node_highlights).collect();
                offset += text_len;
            }
            ix += 1;
        }

        // Add the last text node
        if text.len() > 0 {
            self.state.lock().unwrap().set_text(text.into());
            child_nodes
                .push(Inline::new(ix, self.state.clone(), links, highlights).into_any_element());
        }

        div().id(span.unwrap_or_default()).children(child_nodes)
    }
}

impl Paragraph {
    fn to_markdown(&self) -> String {
        let mut text = self
            .children
            .iter()
            .map(|text_node| {
                let mut text = text_node.text.to_string();
                for (range, style) in &text_node.marks {
                    if style.bold {
                        text = format!("**{}**", &text_node.text[range.clone()]);
                    }
                    if style.italic {
                        text = format!("*{}*", &text_node.text[range.clone()]);
                    }
                    if style.strikethrough {
                        text = format!("~~{}~~", &text_node.text[range.clone()]);
                    }
                    if style.code {
                        text = format!("`{}`", &text_node.text[range.clone()]);
                    }
                    if let Some(link) = &style.link {
                        text = format!("[{}]({})", &text_node.text[range.clone()], link.url);
                    }
                }

                if let Some(image) = &text_node.image {
                    let alt = image.alt.clone().unwrap_or_default();
                    let title = image
                        .title
                        .clone()
                        .map_or(String::new(), |t| format!(" \"{}\"", t));
                    text.push_str(&format!("![{}]({}{})", alt, image.url, title))
                }

                text
            })
            .collect::<Vec<_>>()
            .join("");

        text.push_str("\n\n");
        text
    }
}

impl BlockNode {
    /// Converts the node to markdown format.
    ///
    /// This is used to generate markdown for test.
    #[allow(dead_code)]
    pub(crate) fn to_markdown(&self) -> String {
        match self {
            BlockNode::Root { children, .. } => children
                .iter()
                .map(|child| child.to_markdown())
                .collect::<Vec<_>>()
                .join("\n\n"),
            BlockNode::Paragraph(paragraph) => paragraph.to_markdown(),
            BlockNode::Heading {
                level, children, ..
            } => {
                let hashes = "#".repeat(*level as usize);
                format!("{} {}", hashes, children.to_markdown())
            }
            BlockNode::Blockquote { children, .. } => {
                let content = children
                    .iter()
                    .map(|child| child.to_markdown())
                    .collect::<Vec<_>>()
                    .join("\n\n");

                content
                    .lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            BlockNode::List {
                children, ordered, ..
            } => children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let prefix = if *ordered {
                        format!("{}. ", i + 1)
                    } else {
                        "- ".to_string()
                    };
                    format!("{}{}", prefix, child.to_markdown())
                })
                .collect::<Vec<_>>()
                .join("\n"),
            BlockNode::ListItem {
                children, checked, ..
            } => {
                let checkbox = if let Some(checked) = checked {
                    if *checked { "[x] " } else { "[ ] " }
                } else {
                    ""
                };
                format!(
                    "{}{}",
                    checkbox,
                    children
                        .iter()
                        .map(|child| child.to_markdown())
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            BlockNode::CodeBlock(code_block) => {
                format!(
                    "```{}\n{}\n```",
                    code_block.lang.clone().unwrap_or_default(),
                    code_block.code()
                )
            }
            BlockNode::Table(table) => {
                let header = table
                    .children
                    .first()
                    .map(|row| {
                        row.children
                            .iter()
                            .map(|cell| cell.children.to_markdown())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
                    .unwrap_or_default();
                let alignments = table
                    .column_aligns
                    .iter()
                    .map(|align| {
                        match align {
                            ColumnumnAlign::Left => ":--",
                            ColumnumnAlign::Center => ":-:",
                            ColumnumnAlign::Right => "--:",
                        }
                        .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                let rows = table
                    .children
                    .iter()
                    .skip(1)
                    .map(|row| {
                        row.children
                            .iter()
                            .map(|cell| cell.children.to_markdown())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n{}\n{}", header, alignments, rows)
            }
            BlockNode::Break { html, .. } => {
                if *html {
                    "<br>".to_string()
                } else {
                    "\n".to_string()
                }
            }
            BlockNode::Divider { .. } => "---".to_string(),
            BlockNode::Definition {
                identifier,
                url,
                title,
                ..
            } => {
                if let Some(title) = title {
                    format!("[{}]: {} \"{}\"", identifier, url, title)
                } else {
                    format!("[{}]: {}", identifier, url)
                }
            }
            BlockNode::Unknown { .. } => "".to_string(),
        }
        .trim()
        .to_string()
    }
}

impl BlockNode {
    fn render_list_item(
        item: &BlockNode,
        ix: usize,
        options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        match item {
            BlockNode::ListItem {
                children,
                spread,
                checked,
                ..
            } => v_flex()
                .id(("li", options.ix))
                .when(*spread, |this| this.child(div()))
                .children({
                    let mut items: Vec<Div> = Vec::with_capacity(children.len());

                    for (child_ix, child) in children.iter().enumerate() {
                        match child {
                            BlockNode::Paragraph { .. } => {
                                let last_not_list = child_ix > 0
                                    && !matches!(children[child_ix - 1], BlockNode::List { .. });

                                let text = child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                );

                                // merge content into last item.
                                if last_not_list {
                                    if let Some(item_item) = items.last_mut() {
                                        item_item.extend(vec![
                                            div().overflow_hidden().child(text).into_any_element(),
                                        ]);
                                        continue;
                                    }
                                }

                                items.push(
                                    h_flex()
                                        .flex_1()
                                        .relative()
                                        .items_start()
                                        .content_start()
                                        .when(!options.todo && checked.is_none(), |this| {
                                            this.child(list_item_prefix(
                                                ix,
                                                options.ordered,
                                                options.depth,
                                            ))
                                        })
                                        .when_some(*checked, |this, checked| {
                                            // Todo list checkbox
                                            this.child(
                                                div()
                                                    .flex()
                                                    .mt(rems(0.4))
                                                    .mr_1p5()
                                                    .size(rems(0.875))
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(cx.theme().radius.half())
                                                    .border_1()
                                                    .border_color(cx.theme().primary)
                                                    .text_color(cx.theme().primary_foreground)
                                                    .when(checked, |this| {
                                                        this.bg(cx.theme().primary).child(
                                                            Icon::new(IconName::Check)
                                                                .size_2()
                                                                .text_xs(),
                                                        )
                                                    }),
                                            )
                                        })
                                        .child(div().overflow_hidden().child(text)),
                                );
                            }
                            BlockNode::List { .. } => {
                                items.push(div().ml(rems(1.)).child(child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                )));
                            }
                            _ => {}
                        }
                    }
                    items
                })
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }

    fn render_table(
        item: &BlockNode,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        const DEFAULT_LENGTH: usize = 5;
        const MAX_LENGTH: usize = 150;
        let col_lens = match item {
            BlockNode::Table(table) => {
                let mut col_lens = vec![];
                for row in table.children.iter() {
                    for (ix, cell) in row.children.iter().enumerate() {
                        if col_lens.len() <= ix {
                            col_lens.push(DEFAULT_LENGTH);
                        }

                        let len = cell.children.text_len();
                        if len > col_lens[ix] {
                            col_lens[ix] = len;
                        }
                    }
                }
                col_lens
            }
            _ => vec![],
        };

        match item {
            BlockNode::Table(table) => div()
                .pb(rems(1.))
                .w_full()
                .child(
                    div()
                        .id(("table", options.ix))
                        .w_full()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius)
                        .children({
                            let mut rows = Vec::with_capacity(table.children.len());
                            for (row_ix, row) in table.children.iter().enumerate() {
                                rows.push(
                                    div()
                                        .id("row")
                                        .w_full()
                                        .when(row_ix < table.children.len() - 1, |this| {
                                            this.border_b_1()
                                        })
                                        .border_color(cx.theme().border)
                                        .flex()
                                        .flex_row()
                                        .children({
                                            let mut cells = Vec::with_capacity(row.children.len());
                                            for (ix, cell) in row.children.iter().enumerate() {
                                                let align = table.column_align(ix);
                                                let is_last_col = ix == row.children.len() - 1;
                                                let len = col_lens
                                                    .get(ix)
                                                    .copied()
                                                    .unwrap_or(MAX_LENGTH)
                                                    .min(MAX_LENGTH);

                                                cells.push(
                                                    div()
                                                        .id("cell")
                                                        .flex()
                                                        .when(
                                                            align == ColumnumnAlign::Center,
                                                            |this| this.justify_center(),
                                                        )
                                                        .when(
                                                            align == ColumnumnAlign::Right,
                                                            |this| this.justify_end(),
                                                        )
                                                        .w(Length::Definite(relative(len as f32)))
                                                        .px_2()
                                                        .py_1()
                                                        .when(!is_last_col, |this| {
                                                            this.border_r_1()
                                                                .border_color(cx.theme().border)
                                                        })
                                                        .truncate()
                                                        .child(
                                                            cell.children
                                                                .render(node_cx, window, cx),
                                                        ),
                                                )
                                            }
                                            cells
                                        }),
                                )
                            }
                            rows
                        }),
                )
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }

    pub(crate) fn render_block(
        &self,
        options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let ix = options.ix;
        let mb = if options.in_list || options.is_last {
            rems(0.)
        } else {
            node_cx.style.paragraph_gap
        };

        match self {
            BlockNode::Root { children, .. } => div()
                .id(("div", ix))
                .children(children.into_iter().enumerate().map(move |(ix, node)| {
                    node.render_block(NodeRenderOptions { ix, ..options }, node_cx, window, cx)
                }))
                .into_any_element(),
            BlockNode::Paragraph(paragraph) => div()
                .id(("p", ix))
                .pb(mb)
                .child(paragraph.render(node_cx, window, cx))
                .into_any_element(),
            BlockNode::Heading {
                level, children, ..
            } => {
                let (text_size, font_weight) = match level {
                    1 => (rems(2.), FontWeight::BOLD),
                    2 => (rems(1.5), FontWeight::SEMIBOLD),
                    3 => (rems(1.25), FontWeight::SEMIBOLD),
                    4 => (rems(1.125), FontWeight::SEMIBOLD),
                    5 => (rems(1.), FontWeight::SEMIBOLD),
                    6 => (rems(1.), FontWeight::MEDIUM),
                    _ => (rems(1.), FontWeight::NORMAL),
                };

                let mut text_size = text_size.to_pixels(node_cx.style.heading_base_font_size);
                if let Some(f) = node_cx.style.heading_font_size.as_ref() {
                    text_size = (f)(*level, node_cx.style.heading_base_font_size);
                }

                h_flex()
                    .id(SharedString::from(format!("h{}-{}", level, ix)))
                    .pb(rems(0.3))
                    .whitespace_normal()
                    .text_size(text_size)
                    .font_weight(font_weight)
                    .child(children.render(node_cx, window, cx))
                    .into_any_element()
            }
            BlockNode::Blockquote { children, .. } => div()
                .w_full()
                .pb(mb)
                .child(
                    div()
                        .id(("blockquote", ix))
                        .w_full()
                        .text_color(cx.theme().muted_foreground)
                        .border_l_3()
                        .border_color(cx.theme().secondary_active)
                        .px_4()
                        .children({
                            let children_len = children.len();
                            children.into_iter().enumerate().map(move |(index, c)| {
                                let is_last = index == children_len - 1;
                                c.render_block(options.is_last(is_last), node_cx, window, cx)
                            })
                        }),
                )
                .into_any_element(),
            BlockNode::List {
                children, ordered, ..
            } => v_flex()
                .id((if *ordered { "ol" } else { "ul" }, ix))
                .pb(mb)
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    let mut item_index = 0;
                    for (ix, item) in children.into_iter().enumerate() {
                        let is_item = item.is_list_item();

                        items.push(Self::render_list_item(
                            item,
                            item_index,
                            NodeRenderOptions {
                                ix,
                                ordered: *ordered,
                                ..options
                            },
                            node_cx,
                            window,
                            cx,
                        ));

                        if is_item {
                            item_index += 1;
                        }
                    }
                    items
                })
                .into_any_element(),
            BlockNode::CodeBlock(code_block) => code_block.render(&options, node_cx, window, cx),
            BlockNode::Table { .. } => {
                Self::render_table(self, &options, node_cx, window, cx).into_any_element()
            }
            BlockNode::Divider { .. } => div()
                .pb(mb)
                .child(div().id("divider").bg(cx.theme().border).h(px(2.)))
                .into_any_element(),
            BlockNode::Break { .. } => div().id("break").into_any_element(),
            BlockNode::Unknown { .. } | BlockNode::Definition { .. } => div().into_any_element(),
            _ => {
                if cfg!(debug_assertions) {
                    tracing::warn!("unknown implementation: {:?}", self);
                }

                div().into_any_element()
            }
        }
    }
}
