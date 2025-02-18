use std::ops::Range;

use gpui::{
    div, img, prelude::FluentBuilder as _, rems, App, ElementId, FontStyle, FontWeight,
    HighlightStyle, InteractiveText, IntoElement, ParentElement as _, Pixels, RenderOnce,
    SharedString, SharedUri, Styled, StyledText, Window,
};

use crate::{h_flex, link::Link, v_flex, ActiveTheme as _, StyledExt};

#[derive(Debug, Default, Clone)]
pub struct LinkMark {
    pub url: SharedUri,
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

#[derive(Debug, Default, Clone)]
pub struct TextNode {
    pub text: String,
    pub marks: Vec<(Range<usize>, InlineTextStyle)>,
}

#[derive(Debug, Default, Clone, IntoElement)]
pub struct Paragraph {
    pub span: Option<Span>,
    children: Vec<TextNode>,
}

impl Paragraph {
    pub fn push_str(&mut self, text: &str) {
        self.children.push(TextNode {
            text: text.to_string(),
            marks: vec![(0..text.len(), InlineTextStyle::default())],
        });
    }

    pub fn push(&mut self, text: TextNode) {
        self.children.push(text);
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
        children: Paragraph,
        /// Whether the list item is checked, if None, it's not a checkbox
        checked: Option<bool>,
    },
    Image {
        url: SharedUri,
        title: Option<SharedString>,
        alt: Option<SharedString>,
        width: Option<Pixels>,
        height: Option<Pixels>,
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
        let mut text = String::new();
        let mut highlights: Vec<(Range<usize>, HighlightStyle)> = vec![];
        let mut offset = 0;

        for text_node in self.children.into_iter() {
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
                // if let Some(link) = style.link {
                //     highlight = highlight
                //         .text_color(cx.theme().accent)
                //         .hover_text_color(cx.theme().accent)
                //         .hover_bg_color(cx.theme().bg)
                //         .cursor_pointer()
                //         .on_click(move |_, _| {
                //             if let Some(url) = link.url.as_ref() {
                //                 open_url(url);
                //             }
                //         });
                // }
                let new_range = ((range.start + offset)..(range.end + offset));
                offset += range.end - range.start;

                highlights.push((new_range, highlight));
            }
        }

        let text_style = window.text_style();
        let element_id: ElementId = self.span.unwrap_or_default().into();
        let styled_text = StyledText::new(text).with_highlights(&text_style, highlights);

        div()
            .w_auto()
            .mb_4()
            .whitespace_normal()
            .child(InteractiveText::new(element_id, styled_text))
    }
}

/// Ref:
/// https://ui.shadcn.com/docs/components/typography
impl RenderOnce for Node {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            Node::Root(children) => v_flex().w_full().children(children).into_any_element(),
            Node::Paragraph(paragraph) => paragraph.into_any_element(),
            Node::Heading { level, children } => {
                let (text_size, font_weight) = match level {
                    1 => (rems(3.), FontWeight::BOLD),
                    2 => (rems(1.875), FontWeight::SEMIBOLD),
                    3 => (rems(1.5), FontWeight::SEMIBOLD),
                    4 => (rems(1.25), FontWeight::SEMIBOLD),
                    5 => (rems(1.125), FontWeight::MEDIUM),
                    6 => (rems(1.), FontWeight::MEDIUM),
                    _ => (rems(1.), FontWeight::NORMAL),
                };

                h_flex()
                    .whitespace_normal()
                    .mb_2()
                    .text_size(text_size)
                    .font_weight(font_weight)
                    .child(children)
                    .into_any_element()
            }
            Node::Blockquote(children) => div()
                .w_full()
                .mb_4()
                .bg(cx.theme().accent)
                .border_l_2()
                .border_color(cx.theme().border)
                .px_1()
                .py_1()
                .child(children)
                .into_any_element(),
            Node::List { children, ordered } => v_flex()
                .mb_4()
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    for (ix, item) in children.into_iter().enumerate() {
                        items.push(h_flex().flex_wrap().w_full().child(match ordered {
                            true => div().pl_4().child(format!("{}. ", ix + 1)).child(item),
                            false => div().pl_4().child("• ").child(item),
                        }))
                    }
                    items
                })
                .into_any_element(),
            Node::CodeBlock { code, .. } => div()
                .mb_4()
                .rounded(cx.theme().radius)
                .bg(cx.theme().accent)
                .p_3()
                .text_size(rems(0.875))
                .relative()
                .child(code)
                .into_any_element(),
            Node::ListItem { children, .. } => children.into_any_element(),
            Node::Image {
                url, width, height, ..
            } => img(url)
                .when_some(width, |this, width| this.w(width))
                .when_some(height, |this, height| this.w(height))
                .into_any_element(),
            // Node::Link { children, url, .. } => Link::new("link")
            //     .href(url)
            //     .children(children)
            //     .into_any_element(),
            Node::Break => div().into_any_element(),
            _ => div().into_any_element(),
        }
    }
}
