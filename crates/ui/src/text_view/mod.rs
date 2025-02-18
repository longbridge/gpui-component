use std::ops::Deref;

use gpui::{
    div, IntoElement, ParentElement as _, RenderOnce, SharedString, StyledText, TextAlign, Window,
};

mod ast;
mod markdown;

pub use markdown::MarkdownView;

#[derive(Default)]
struct Link {
    url: SharedString,
    title: Option<SharedString>,
}

#[derive(Default)]
struct TextBlockStyle {
    align: TextAlign,
    wrap: bool,
}

#[derive(IntoElement)]
struct TextBlock {
    style: TextBlockStyle,
    elements: Vec<TextElement>,
}

impl Default for TextBlock {
    fn default() -> Self {
        Self {
            style: TextBlockStyle {
                align: TextAlign::Left,
                wrap: true,
            },
            elements: vec![],
        }
    }
}

#[derive(IntoElement)]
struct TextElement {
    base: StyledText,
    link: Option<Link>,
    code: bool,
}

impl TextElement {
    pub fn new() -> Self {
        Self {
            base: StyledText::new(""),
            link: None,
            code: false,
        }
    }
}

impl Deref for TextElement {
    type Target = StyledText;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(IntoElement)]
pub enum Block {
    Root(Vec<Block>),
    Heading1(TextBlock),
    Heading2(TextBlock),
    Heading3(TextBlock),
    Heading4(TextBlock),
    Heading5(TextBlock),
    Heading6(TextBlock),
    Paragraph(Vec<TextBlock>),
    BulletList(Vec<Block>),
    OrderedList(Vec<Block>),
    Blockquote(Vec<TextBlock>),
    Text(TextBlock),
    Divider,
}

impl RenderOnce for TextElement {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl gpui::IntoElement {
        div().child(self.base)
    }
}

impl RenderOnce for TextBlock {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl gpui::IntoElement {
        div().children(self.elements)
    }
}

impl RenderOnce for Block {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl gpui::IntoElement {
        match self {
            Block::Heading1(block) => div().child(block),
            Block::Heading2(block) => div().child(block),
            Block::Heading3(block) => div().child(block),
            Block::Heading4(block) => div().child(block),
            Block::Heading5(block) => div().child(block),
            Block::Heading6(block) => div().child(block),
            Block::Paragraph(blocks) => div().children(blocks),
            Block::BulletList(blocks) => div().children(blocks),
            Block::OrderedList(blocks) => div().children(blocks),
            Block::Blockquote(blocks) => div().children(blocks),
            Block::Divider => div(),
            _ => div(),
        }
    }
}
