use std::ops::Range;

use gpui::{
    div, img, prelude::FluentBuilder as _, px, relative, rems, AnyElement, App, DefiniteLength,
    ElementId, FontStyle, FontWeight, Half, HighlightStyle, InteractiveElement as _,
    InteractiveText, IntoElement, Length, ObjectFit, ParentElement, Rems, RenderOnce, SharedString,
    SharedUri, Styled, StyledImage as _, StyledText, Window,
};
use markdown::mdast;

use crate::{
    h_flex,
    highlighter::{LanguageRegistry, SyntaxHighlighter},
    v_flex, ActiveTheme as _, Icon, IconName,
};

use super::{utils::list_item_prefix, TextViewStyle};

#[allow(unused)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LinkMark {
    pub url: SharedString,
    pub title: Option<SharedString>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct InlineTextStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub link: Option<LinkMark>,
}

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
    pub title: Option<SharedString>,
    pub alt: Option<SharedString>,
    pub width: Option<DefiniteLength>,
    pub height: Option<DefiniteLength>,
}

impl PartialEq for ImageNode {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url && self.title == other.title && self.alt == other.alt
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextNode {
    /// The text content.
    pub text: String,
    /// The text styles, each tuple contains the range of the text and the style.
    pub marks: Vec<(Range<usize>, InlineTextStyle)>,
}

#[derive(Debug, Clone, PartialEq, IntoElement)]
pub enum Paragraph {
    Texts {
        span: Option<Span>,
        children: Vec<TextNode>,
    },
    Image {
        span: Option<Span>,
        image: ImageNode,
    },
}

impl Default for Paragraph {
    fn default() -> Self {
        Self::Texts {
            span: None,
            children: vec![],
        }
    }
}

impl From<String> for Paragraph {
    fn from(value: String) -> Self {
        Self::Texts {
            span: None,
            children: vec![TextNode {
                text: value.clone(),
                marks: vec![],
            }],
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Table {
    pub children: Vec<TableRow>,
    pub column_aligns: Vec<TableColumnAlign>,
}

impl Table {
    pub(crate) fn column_align(&self, index: usize) -> TableColumnAlign {
        self.column_aligns.get(index).copied().unwrap_or_default()
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum TableColumnAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl From<mdast::AlignKind> for TableColumnAlign {
    fn from(value: mdast::AlignKind) -> Self {
        match value {
            mdast::AlignKind::None => TableColumnAlign::Left,
            mdast::AlignKind::Left => TableColumnAlign::Left,
            mdast::AlignKind::Center => TableColumnAlign::Center,
            mdast::AlignKind::Right => TableColumnAlign::Right,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TableRow {
    pub children: Vec<TableCell>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TableCell {
    pub children: Paragraph,
    pub width: Option<DefiniteLength>,
}

impl Paragraph {
    pub fn clear(&mut self) {
        match self {
            Self::Texts { children, .. } => children.clear(),
            Self::Image { .. } => *self = Self::default(),
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image { .. })
    }

    pub fn set_span(&mut self, span: Span) {
        match self {
            Self::Texts { span: s, .. } => *s = Some(span),
            Self::Image { span: s, .. } => *s = Some(span),
        }
    }

    pub fn push_str(&mut self, text: &str) {
        if let Self::Texts { children, .. } = self {
            children.push(TextNode {
                text: text.to_string(),
                marks: vec![(0..text.len(), InlineTextStyle::default())],
            });
        }
    }

    pub fn push(&mut self, text: TextNode) {
        if let Self::Texts { children, .. } = self {
            children.push(text);
        }
    }

    pub fn set_image(&mut self, image: ImageNode) {
        *self = Self::Image { span: None, image };
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Texts { .. } => self.text_len() == 0,
            Self::Image { .. } => false,
        }
    }

    /// Return length of children text.
    pub fn text_len(&self) -> usize {
        match self {
            Self::Texts { children, .. } => {
                let mut len = 0;
                for text_node in children.iter() {
                    len = text_node.text.len().max(len);
                }
                len
            }
            Self::Image { .. } => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock {
    code: SharedString,
    lang: Option<SharedString>,
    styles: Vec<(Range<usize>, HighlightStyle)>,
}

impl CodeBlock {
    pub fn new(
        code: SharedString,
        lang: Option<SharedString>,
        _: &TextViewStyle,
        cx: &mut App,
    ) -> Self {
        let theme = LanguageRegistry::global(cx)
            .theme(cx.theme().is_dark())
            .clone();
        let mut styles = vec![];
        if let Some(lang) = &lang {
            let mut highlighter = SyntaxHighlighter::new(&lang, cx);
            highlighter.update(&(0..0), code.clone(), "", cx);
            styles = highlighter.styles(&(0..code.len()), &theme);
        };

        Self { code, lang, styles }
    }
}

/// Ref:
/// https://ui.shadcn.com/docs/components/typography
#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Root {
        children: Vec<Node>,
    },
    Paragraph(Paragraph),
    Heading {
        level: u8,
        children: Paragraph,
    },
    Blockquote(Paragraph),
    List {
        /// Only contains ListItem, others will be ignored
        children: Vec<Node>,
        ordered: bool,
    },
    ListItem {
        children: Vec<Node>,
        spread: bool,
        /// Whether the list item is checked, if None, it's not a checkbox
        checked: Option<bool>,
    },
    CodeBlock(CodeBlock),
    Table(Table),
    Break {
        html: bool,
    },
    Divider,
    Ignore,
    Unknown,
}

impl Node {
    pub(super) fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignore)
    }

    pub(super) fn is_list_item(&self) -> bool {
        matches!(self, Self::ListItem { .. })
    }

    pub(super) fn is_break(&self) -> bool {
        matches!(self, Self::Break { .. })
    }

    /// Combine all children, omitting the empt parent nodes.
    pub(super) fn compact(&self) -> Node {
        match self {
            Self::Root { children } => {
                let children = children
                    .iter()
                    .map(|c| c.compact())
                    .filter(|c| !c.is_ignore())
                    .collect::<Vec<_>>();
                if children.len() == 1 {
                    children.first().unwrap().compact()
                } else {
                    self.clone()
                }
            }
            _ => self.clone(),
        }
    }
}

impl RenderOnce for Paragraph {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            Self::Texts { span, children } => {
                let mut text = String::new();
                let mut highlights: Vec<(Range<usize>, HighlightStyle)> = vec![];
                let mut links: Vec<(Range<usize>, LinkMark)> = vec![];
                let mut offset = 0;

                for text_node in children.into_iter() {
                    let text_len = text_node.text.len();
                    let part = if text.len() == 0 {
                        // trim start for first text
                        text_node.text.trim_start()
                    } else {
                        text_node.text.as_str()
                    };

                    text.push_str(part);

                    let mut node_highlights = vec![];
                    for (range, style) in text_node.marks {
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

                        if let Some(link_mark) = style.link {
                            highlight.color = Some(cx.theme().link);
                            highlight.underline = Some(gpui::UnderlineStyle {
                                thickness: gpui::px(1.),
                                ..Default::default()
                            });

                            links.push((inner_range.clone(), link_mark));
                        }

                        node_highlights.push((inner_range, highlight));
                    }

                    highlights = gpui::combine_highlights(highlights, node_highlights).collect();

                    offset += text_len;
                }

                let text_style = window.text_style();
                let element_id: ElementId = span.unwrap_or_default().into();
                let styled_text =
                    StyledText::new(text).with_default_highlights(&text_style, highlights);
                let link_ranges = links
                    .iter()
                    .map(|(range, _)| range.clone())
                    .collect::<Vec<_>>();

                InteractiveText::new(element_id, styled_text)
                    .on_click(link_ranges, {
                        let links = links.clone();
                        move |ix, _, cx| {
                            if let Some((_, link)) = &links.get(ix) {
                                // Stop propagation to prevent the parent element from handling the event.
                                //
                                // For example the text in a checkbox label, click link need avoid toggle check state.
                                cx.stop_propagation();
                                cx.open_url(&link.url);
                            }
                        }
                    })
                    .into_any_element()
            }
            Self::Image { image, .. } => img(image.url)
                .object_fit(ObjectFit::Contain)
                .max_w(relative(1.))
                .when_some(image.width, |this, width| this.w(width))
                .into_any_element(),
        }
    }
}

#[derive(Default)]
pub(crate) struct ListState {
    todo: bool,
    ordered: bool,
    depth: usize,
}

impl Node {
    fn render_list_item(
        item: Node,
        ix: usize,
        state: ListState,
        text_view_style: &TextViewStyle,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        match item {
            Node::ListItem {
                children,
                spread,
                checked,
            } => v_flex()
                .when(spread, |this| this.child(div()))
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    for child in children.into_iter() {
                        match &child {
                            Node::Paragraph(_) => {
                                items.push(
                                    h_flex()
                                        .relative()
                                        .items_start()
                                        .content_start()
                                        .flex_1()
                                        .when(!state.todo && checked.is_none(), |this| {
                                            this.child(list_item_prefix(
                                                ix,
                                                state.ordered,
                                                state.depth,
                                            ))
                                        })
                                        .when_some(checked, |this, checked| {
                                            // Checkmark
                                            this.child(
                                                div()
                                                    .flex()
                                                    .mt(rems(0.4))
                                                    .mr_1p5()
                                                    .size(rems(0.875))
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(cx.theme().radius.half())
                                                    .bg(cx.theme().primary)
                                                    .text_color(cx.theme().primary_foreground)
                                                    .when(checked, |this| {
                                                        this.child(
                                                            Icon::new(IconName::Check)
                                                                .size_2()
                                                                .text_xs(),
                                                        )
                                                    }),
                                            )
                                        })
                                        .child(div().flex_1().overflow_hidden().child(
                                            child.render(
                                                Some(ListState {
                                                    depth: state.depth + 1,
                                                    ordered: state.ordered,
                                                    todo: checked.is_some(),
                                                }),
                                                true,
                                                text_view_style,
                                                window,
                                                cx,
                                            ),
                                        )),
                                );
                            }
                            Node::List { .. } => {
                                items.push(div().ml(rems(1.)).child(child.render(
                                    Some(ListState {
                                        depth: state.depth + 1,
                                        ordered: state.ordered,
                                        todo: checked.is_some(),
                                    }),
                                    true,
                                    text_view_style,
                                    window,
                                    cx,
                                )))
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

    fn render_table(item: &Node, _: &mut Window, cx: &mut App) -> impl IntoElement {
        const DEFAULT_LENGTH: usize = 5;
        const MAX_LENGTH: usize = 150;
        let col_lens = match item {
            Node::Table(table) => {
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
            Node::Table(table) => div()
                .id("table")
                .mb(rems(1.))
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
                                .when(row_ix < table.children.len() - 1, |this| this.border_b_1())
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
                                                .when(align == TableColumnAlign::Center, |this| {
                                                    this.justify_center()
                                                })
                                                .when(align == TableColumnAlign::Right, |this| {
                                                    this.justify_end()
                                                })
                                                .w(Length::Definite(relative(len as f32)))
                                                .px_2()
                                                .py_1()
                                                .when(!is_last_col, |this| {
                                                    this.border_r_1()
                                                        .border_color(cx.theme().border)
                                                })
                                                .truncate()
                                                .child(cell.children.clone()),
                                        )
                                    }
                                    cells
                                }),
                        )
                    }
                    rows
                })
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }

    fn render_codeblock(
        code_block: CodeBlock,
        mb: Rems,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .mb(mb)
            .p_3()
            .rounded(cx.theme().radius)
            .bg(cx.theme().secondary)
            .font_family("Menlo, Monaco, Consolas, monospace")
            .text_size(rems(0.875))
            .relative()
            .child(StyledText::new(code_block.code.clone()).with_highlights(code_block.styles))
            .into_any_element()
    }

    pub(crate) fn render(
        self,
        list_state: Option<ListState>,
        is_last_child: bool,
        text_view_style: &TextViewStyle,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let in_list = list_state.is_some();
        let mb = if in_list || is_last_child {
            rems(0.)
        } else {
            text_view_style.paragraph_gap
        };

        match self {
            Node::Root { children } => div()
                .children({
                    let children_len = children.len();
                    children.into_iter().enumerate().map(move |(index, c)| {
                        let is_last_child = index == children_len - 1;
                        c.render(None, is_last_child, text_view_style, window, cx)
                    })
                })
                .into_any_element(),
            Node::Paragraph(paragraph) => div().mb(mb).child(paragraph).into_any_element(),
            Node::Heading { level, children } => {
                let (text_size, font_weight) = match level {
                    1 => (rems(2.), FontWeight::BOLD),
                    2 => (rems(1.5), FontWeight::SEMIBOLD),
                    3 => (rems(1.25), FontWeight::SEMIBOLD),
                    4 => (rems(1.125), FontWeight::SEMIBOLD),
                    5 => (rems(1.), FontWeight::SEMIBOLD),
                    6 => (rems(1.), FontWeight::MEDIUM),
                    _ => (rems(1.), FontWeight::NORMAL),
                };

                let text_size = text_size.to_pixels(text_view_style.heading_base_font_size);

                h_flex()
                    .mb(rems(0.3))
                    .whitespace_normal()
                    .text_size(text_size)
                    .font_weight(font_weight)
                    .child(children)
                    .into_any_element()
            }
            Node::Blockquote(children) => div()
                .w_full()
                .mb(mb)
                .text_color(cx.theme().muted_foreground)
                .border_l_3()
                .border_color(cx.theme().secondary_active)
                .px_4()
                .child(children)
                .into_any_element(),
            Node::List { children, ordered } => v_flex()
                .mb(mb)
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    let list_state = list_state.unwrap_or_default();
                    let mut ix = 0;
                    for item in children.into_iter() {
                        let is_item = item.is_list_item();

                        items.push(Self::render_list_item(
                            item,
                            ix,
                            ListState {
                                ordered,
                                todo: list_state.todo,
                                depth: list_state.depth,
                            },
                            text_view_style,
                            window,
                            cx,
                        ));

                        if is_item {
                            ix += 1;
                        }
                    }
                    items
                })
                .into_any_element(),
            Node::CodeBlock(code_block) => Self::render_codeblock(code_block, mb, window, cx),
            Node::Table { .. } => Self::render_table(&self, window, cx).into_any_element(),
            Node::Divider => div()
                .bg(cx.theme().border)
                .h(px(2.))
                .mb(mb)
                .into_any_element(),
            Node::Break { .. } => div().into_any_element(),
            Node::Ignore => div().into_any_element(),
            _ => {
                if cfg!(debug_assertions) {
                    eprintln!("Unknown implementation: {:?}", self);
                }

                div().into_any_element()
            }
        }
    }
}

impl Paragraph {
    fn to_markdown(&self) -> String {
        let mut text = match self {
            Paragraph::Texts { children, .. } => children
                .iter()
                .map(|text_node| {
                    let mut text = text_node.text.clone();
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
                    text
                })
                .collect::<Vec<_>>()
                .join(""),
            Paragraph::Image { image, .. } => {
                let alt = image.alt.clone().unwrap_or_default();
                let title = image
                    .title
                    .clone()
                    .map_or(String::new(), |t| format!(" \"{}\"", t));
                format!("![{}]({}{})", alt, image.url, title)
            }
        };

        text.push_str("\n\n");
        text
    }
}

impl Node {
    /// Converts the node to markdown format.
    ///
    /// This is used to generate markdown for test.
    #[allow(dead_code)]
    pub(crate) fn to_markdown(&self) -> String {
        match self {
            Node::Root { children } => children
                .iter()
                .map(|child| child.to_markdown())
                .collect::<Vec<_>>()
                .join("\n\n"),
            Node::Paragraph(paragraph) => paragraph.to_markdown(),
            Node::Heading { level, children } => {
                let hashes = "#".repeat(*level as usize);
                format!("{} {}", hashes, children.to_markdown())
            }
            Node::Blockquote(paragraph) => {
                let content = paragraph.to_markdown();
                content
                    .lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Node::List { children, ordered } => children
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
            Node::ListItem {
                children, checked, ..
            } => {
                let checkbox = if let Some(checked) = checked {
                    if *checked {
                        "[x] "
                    } else {
                        "[ ] "
                    }
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
            Node::CodeBlock(code_block) => {
                format!(
                    "```{}\n{}\n```",
                    code_block.lang.clone().unwrap_or_default(),
                    code_block.code
                )
            }
            Node::Table(table) => {
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
                            TableColumnAlign::Left => ":--",
                            TableColumnAlign::Center => ":-:",
                            TableColumnAlign::Right => "--:",
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
            Node::Break { html } => {
                if *html {
                    "<br>".to_string()
                } else {
                    "\n".to_string()
                }
            }
            Node::Divider => "---".to_string(),
            Node::Ignore => "".to_string(),
            Node::Unknown => "".to_string(),
        }
        .trim()
        .to_string()
    }
}
