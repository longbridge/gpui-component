use gpui::{
    div, img, prelude::FluentBuilder as _, rems, App, FontWeight, IntoElement, ParentElement as _,
    Pixels, RenderOnce, SharedString, SharedUri, Styled, Window,
};

use crate::{h_flex, link::Link, v_flex, ActiveTheme as _, StyledExt};

#[derive(Debug, Default, Clone, IntoElement)]
pub struct TextNode {
    pub text: SharedString,
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
}

#[allow(unused)]
#[derive(Debug, Clone, IntoElement)]
pub enum Node {
    Root(Vec<Node>),
    Paragraph(Vec<Node>),
    Heading {
        level: u8,
        children: Vec<Node>,
    },
    Blockquote(Vec<Node>),
    List {
        children: Vec<Node>,
        ordered: bool,
    },
    ListItem {
        children: Vec<Node>,
        /// Whether the list item is checked, if None, it's not a checkbox
        checked: Option<bool>,
    },
    Text(TextNode),
    Image {
        url: SharedUri,
        title: Option<SharedString>,
        alt: Option<SharedString>,
        width: Option<Pixels>,
        height: Option<Pixels>,
    },
    Link {
        children: Vec<Node>,
        url: SharedString,
        title: Option<SharedString>,
    },
    CodeBlock {
        code: SharedString,
        lang: Option<SharedString>,
    },
    // <br>
    Break,
    Unknown,
}

impl RenderOnce for TextNode {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_auto()
            .whitespace_normal()
            .when(self.bold, |this| this.font_bold())
            .when(self.italic, |this| this.italic())
            .when(self.strikethrough, |this| this.line_through())
            .when(self.code, |this| {
                this.px_0p5()
                    .bg(cx.theme().accent)
                    .rounded(cx.theme().radius)
            })
            .child(self.text)
    }
}

/// Ref:
/// https://ui.shadcn.com/docs/components/typography
impl RenderOnce for Node {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            Node::Root(children) => v_flex().w_full().children(children).into_any_element(),

            Node::Paragraph(children) => h_flex()
                .mb_4()
                .whitespace_normal()
                .flex_wrap()
                .children(children)
                .into_any_element(),
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
                    .children(children)
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
                .children(children)
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
            Node::ListItem { children, .. } => h_flex()
                .flex_wrap()
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    for child in children {
                        items.push(child);
                    }
                    items
                })
                .into_any_element(),
            Node::Text(text_node) => text_node.into_any_element(),
            Node::Image {
                url, width, height, ..
            } => img(url)
                .when_some(width, |this, width| this.w(width))
                .when_some(height, |this, height| this.w(height))
                .into_any_element(),
            Node::Link { children, url, .. } => Link::new("link")
                .href(url)
                .children(children)
                .into_any_element(),
            Node::Break => div().into_any_element(),
            _ => div().into_any_element(),
        }
    }
}
