use gpui::{
    div, prelude::FluentBuilder as _, relative, rems, App, Empty, FontWeight, IntoElement,
    ParentElement as _, Pixels, RenderOnce, SharedString, Styled, Window,
};

use crate::{v_flex, ActiveTheme as _, StyledExt};

#[derive(Debug, Default, Clone, IntoElement)]
pub struct TextNode {
    pub text: SharedString,
    pub children: Vec<TextNode>,
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
}

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
    Text(TextNode),
    Image {
        url: SharedString,
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
            .when(self.bold, |this| this.font_bold())
            .when(self.italic, |this| this.italic())
            .when(self.strikethrough, |this| this.line_through())
            .when(self.code, |this| this.px_0p5().bg(cx.theme().accent))
            .child(self.text)
    }
}

/// Ref:
/// https://ui.shadcn.com/docs/components/typography
impl RenderOnce for Node {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            Node::Root(children) => v_flex().w_full().children(children),
            Node::Paragraph(children) => div().w_full().whitespace_normal().children(children),
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

                div()
                    .w_full()
                    .whitespace_normal()
                    .text_size(text_size)
                    .font_weight(font_weight)
                    .children(children)
            }
            Node::Blockquote(children) => div()
                .w_full()
                .bg(cx.theme().accent)
                .border_l_2()
                .border_color(cx.theme().border)
                .px_1()
                .py_1()
                .children(children),
            Node::List { children, ordered } => v_flex().children({
                let mut items = Vec::with_capacity(children.len());
                for (ix, item) in children.into_iter().enumerate() {
                    items.push(div().w_full().child(match ordered {
                        true => div().pl_4().child(format!("{}. ", ix + 1)).child(item),
                        false => div().pl_4().child("• ").child(item),
                    }))
                }
                items
            }),
            _ => div(),
        }
    }
}
