use std::ops::Range;

use gpui::{
    div, img, prelude::FluentBuilder as _, px, relative, rems, App, DefiniteLength, ElementId,
    FontStyle, FontWeight, Half, HighlightStyle, InteractiveElement as _, InteractiveText,
    IntoElement, Length, ParentElement, RenderOnce, SharedString, SharedUri, Styled, StyledText,
    Window,
};

use crate::{h_flex, v_flex, ActiveTheme as _, Icon, IconName};

use super::utils::list_item_prefix;

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

#[allow(unused)]
#[derive(Debug, Clone, IntoElement, PartialEq)]
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
    CodeBlock {
        code: SharedString,
        lang: Option<SharedString>,
    },
    Table(Table),
    // <br>
    Break,
    Divider,
    Ignore,
    Unknown,
}

impl Node {
    fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignore)
    }

    fn is_list_item(&self) -> bool {
        matches!(self, Self::ListItem { .. })
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
                    text.push_str(&text_node.text);

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

                        highlights.push((inner_range, highlight));
                    }

                    offset += text_len;
                }

                let text_style = window.text_style();
                let element_id: ElementId = span.unwrap_or_default().into();
                let styled_text = StyledText::new(text).with_highlights(&text_style, highlights);
                let link_ranges = links
                    .iter()
                    .map(|(range, _)| range.clone())
                    .collect::<Vec<_>>();

                InteractiveText::new(element_id, styled_text)
                    .on_click(link_ranges, {
                        let links = links.clone();
                        move |ix, _, cx| {
                            if let Some((_, link)) = &links.get(ix) {
                                cx.open_url(&link.url);
                            }
                        }
                    })
                    .into_any_element()
            }
            Self::Image { image, .. } => img(image.url)
                .when_some(image.width, |this, width| this.w(width))
                .when_some(image.height, |this, height| this.h(height))
                .into_any_element(),
        }
    }
}

#[derive(Default)]
struct ListState {
    todo: bool,
    ordered: bool,
    depth: usize,
}

impl Node {
    fn render_list_item(
        item: Node,
        ix: usize,
        state: ListState,
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
                                        .items_center()
                                        .when(!state.todo && checked.is_none(), |this| {
                                            this.child(list_item_prefix(
                                                ix,
                                                state.ordered,
                                                state.depth,
                                            ))
                                        })
                                        .when_some(checked, |this, checked| {
                                            this.child(
                                                div()
                                                    .flex()
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
                                        .child(child.render_node(
                                            Some(ListState {
                                                depth: state.depth + 1,
                                                ordered: state.ordered,
                                                todo: checked.is_some(),
                                            }),
                                            window,
                                            cx,
                                        )),
                                );
                            }
                            Node::List { .. } => {
                                items.push(div().ml(rems(1.)).child(child.render_node(
                                    Some(ListState {
                                        depth: state.depth + 1,
                                        ordered: state.ordered,
                                        todo: checked.is_some(),
                                    }),
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
                                        let len = col_lens
                                            .get(ix)
                                            .copied()
                                            .unwrap_or(MAX_LENGTH)
                                            .min(MAX_LENGTH);

                                        cells.push(
                                            div()
                                                .id("cell")
                                                .w(Length::Definite(relative(len as f32)))
                                                .px_2()
                                                .py_1()
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

    fn render_node(
        self,
        list_state: Option<ListState>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let in_list = list_state.is_some();
        let mb = if in_list { rems(0.0) } else { rems(1.) };

        match self {
            Node::Root { children } => div().children(children).into_any_element(),
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

                h_flex()
                    .mb(rems(0.5))
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
                .py_1()
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
            Node::CodeBlock { code, .. } => div()
                .mb(mb)
                .rounded(cx.theme().radius)
                .bg(cx.theme().secondary)
                .p_3()
                .text_size(rems(0.875))
                .relative()
                .child(code)
                .into_any_element(),
            Node::Table { .. } => Self::render_table(&self, window, cx).into_any_element(),
            Node::Divider => div()
                .bg(cx.theme().border)
                .h(px(2.))
                .mb(mb)
                .into_any_element(),
            Node::Break => div().into_any_element(),
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

/// Ref:
/// https://ui.shadcn.com/docs/components/typography
impl RenderOnce for Node {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_node(None, window, cx)
    }
}
