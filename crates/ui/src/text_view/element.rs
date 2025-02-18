use std::ops::Range;

use gpui::{
    div, img, prelude::FluentBuilder as _, rems, App, ElementId, FontStyle, FontWeight,
    HighlightStyle, InteractiveText, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    SharedUri, Styled, StyledText, Window,
};

use crate::{h_flex, v_flex, ActiveTheme as _, IconName};

use super::utils::list_item_prefix;

#[allow(unused)]
#[derive(Debug, Default, Clone)]
pub struct LinkMark {
    pub url: SharedString,
    pub title: Option<SharedString>,
}

#[derive(Debug, Default, Clone)]
pub struct InlineTextStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub link: Option<LinkMark>,
}

#[derive(Debug, Default, Copy, Clone)]
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
    pub width: Option<Pixels>,
    pub height: Option<Pixels>,
}

#[derive(Debug, Default, Clone)]
pub struct TextNode {
    pub text: String,
    pub marks: Vec<(Range<usize>, InlineTextStyle)>,
}

#[derive(Debug, Clone, IntoElement)]
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

impl Paragraph {
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
}

#[allow(unused)]
#[derive(Debug, Clone, IntoElement)]
pub enum Node {
    Root(Vec<Node>),
    Paragraph(Paragraph),
    Heading {
        level: u8,
        children: Paragraph,
    },
    Blockquote(Paragraph),
    List {
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
    // <br>
    Break,
    Unknown,
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
                    text.push_str(&text_node.text);

                    for (range, style) in text_node.marks {
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

                        let new_range = (range.start + offset)..(range.end + offset);

                        if let Some(link_mark) = style.link {
                            highlight.color = Some(cx.theme().link);
                            highlight.underline = Some(gpui::UnderlineStyle {
                                thickness: gpui::px(1.),
                                ..Default::default()
                            });
                            links.push((new_range.clone(), link_mark));
                        }

                        offset += range.end - range.start;

                        highlights.push((new_range, highlight));
                    }
                }

                let text_style = window.text_style();
                let element_id: ElementId = span.unwrap_or_default().into();
                let styled_text = StyledText::new(text).with_highlights(&text_style, highlights);
                let link_ranges = links
                    .iter()
                    .map(|(range, _)| range.clone())
                    .collect::<Vec<_>>();

                div()
                    .w_auto()
                    .whitespace_normal()
                    .child(
                        InteractiveText::new(element_id, styled_text).on_click(link_ranges, {
                            let links = links.clone();
                            move |ix, _, cx| {
                                if let Some((_, link)) = &links.get(ix) {
                                    cx.open_url(&link.url);
                                }
                            }
                        }),
                    )
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
                                                    .size(rems(0.875))
                                                    .mr_1()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(cx.theme().radius)
                                                    .border_1()
                                                    .border_color(cx.theme().border)
                                                    .bg(cx.theme().accent)
                                                    .when(checked, |this| {
                                                        this.child(
                                                            div()
                                                                .items_center()
                                                                .text_xs()
                                                                .child(IconName::Check),
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

    fn render_node(
        self,
        list_state: Option<ListState>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let in_list = list_state.is_some();
        let mb = if in_list { rems(0.0) } else { rems(1.) };

        match self {
            Node::Root(children) => v_flex().w_full().children(children).into_any_element(),
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
                    .whitespace_normal()
                    .text_size(text_size)
                    .font_weight(font_weight)
                    .child(children)
                    .into_any_element()
            }
            Node::Blockquote(children) => div()
                .w_full()
                .mb(mb)
                .bg(cx.theme().accent)
                .border_l_2()
                .border_color(cx.theme().border)
                .px_1()
                .py_1()
                .child(children)
                .into_any_element(),
            Node::List { children, ordered } => v_flex()
                .mb(mb)
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    let list_state = list_state.unwrap_or_default();
                    for (ix, item) in children.into_iter().enumerate() {
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
                        ))
                    }
                    items
                })
                .into_any_element(),
            Node::CodeBlock { code, .. } => div()
                .mb(mb)
                .rounded(cx.theme().radius)
                .bg(cx.theme().accent)
                .p_3()
                .text_size(rems(0.875))
                .relative()
                .child(code)
                .into_any_element(),
            Node::Break => div().into_any_element(),
            _ => div().into_any_element(),
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
