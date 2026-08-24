use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{ActiveTheme as _, StyledExt as _, h_flex, v_flex};

/// Horizontal alignment for a message and message-owned chat surfaces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessageAlignment {
    /// Place the message at the leading edge.
    #[default]
    Start,
    /// Place the message at the trailing edge.
    End,
}

/// A vertical stack of consecutive messages from the same sender.
#[derive(IntoElement)]
pub struct MessageGroup {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MessageGroup {
    /// Create an empty message group.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for MessageGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MessageGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MessageGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        v_flex()
            .min_w_0()
            .gap(tokens.spacing.sm)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A composable message row with named avatar, header, content, and footer slots.
///
/// Named slots let the message apply its alignment consistently while every
/// part remains independently styleable.
#[derive(IntoElement)]
pub struct Message {
    style: StyleRefinement,
    alignment: MessageAlignment,
    avatar: Option<AnyElement>,
    header: Option<MessageHeader>,
    content: Option<MessageContent>,
    footer: Option<MessageFooter>,
}

impl Message {
    /// Create a leading-aligned message.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            alignment: MessageAlignment::Start,
            avatar: None,
            header: None,
            content: None,
            footer: None,
        }
    }

    /// Set whether the message is aligned to the leading or trailing edge.
    pub fn alignment(mut self, alignment: MessageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set an optional avatar or other sender identity element.
    pub fn avatar(mut self, avatar: impl IntoElement) -> Self {
        self.avatar = Some(avatar.into_any_element());
        self
    }

    /// Set the message header.
    pub fn header(mut self, header: MessageHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// Set the message body.
    pub fn content(mut self, content: MessageContent) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the message footer.
    pub fn footer(mut self, footer: MessageFooter) -> Self {
        self.footer = Some(footer);
        self
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Message {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Message {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let alignment = self.alignment;

        h_flex()
            .relative()
            .w_full()
            .min_w_0()
            .items_end()
            .gap(tokens.spacing.sm)
            .text_size(tokens.typography.sm.size)
            .line_height(tokens.typography.sm.line_height)
            .when(alignment == MessageAlignment::End, |this| {
                this.flex_row_reverse()
            })
            .refine_style(&self.style)
            .when_some(self.avatar, |this, avatar| {
                this.child(div().flex_none().self_end().child(avatar))
            })
            .child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(tokens.spacing.sm)
                    .map(|this| match alignment {
                        MessageAlignment::Start => this.items_start(),
                        MessageAlignment::End => this.items_end(),
                    })
                    .when_some(self.header, |this, header| this.child(header))
                    .when_some(self.content, |this, content| {
                        this.child(content.aligned(alignment))
                    })
                    .when_some(self.footer, |this, footer| this.child(footer)),
            )
    }
}

/// Header content such as a sender name and timestamp.
#[derive(IntoElement)]
pub struct MessageHeader {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MessageHeader {
    /// Create an empty message header.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for MessageHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MessageHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MessageHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        h_flex()
            .max_w_full()
            .min_w_0()
            .gap(tokens.spacing.xs)
            .text_size(tokens.typography.xs.size)
            .line_height(tokens.typography.xs.line_height)
            .font_medium()
            .text_color(tokens.colors.muted_foreground)
            .px(tokens.spacing.md)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// The message body slot. It can contain bubbles, images, code, or files.
#[derive(IntoElement)]
pub struct MessageContent {
    style: StyleRefinement,
    alignment: MessageAlignment,
    children: Vec<AnyElement>,
}

impl MessageContent {
    /// Create an empty message body.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            alignment: MessageAlignment::Start,
            children: Vec::new(),
        }
    }

    fn aligned(mut self, alignment: MessageAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MessageContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MessageContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageContent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        v_flex()
            .w_full()
            .max_w_full()
            .min_w_0()
            .gap(tokens.spacing.sm)
            .map(|this| match self.alignment {
                MessageAlignment::Start => this.items_start(),
                MessageAlignment::End => this.items_end(),
            })
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Footer content such as delivery state, reactions, or action buttons.
#[derive(IntoElement)]
pub struct MessageFooter {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MessageFooter {
    /// Create an empty message footer.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for MessageFooter {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MessageFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MessageFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        h_flex()
            .max_w_full()
            .min_w_0()
            .gap(tokens.spacing.xs)
            .text_size(tokens.typography.xs.size)
            .line_height(tokens.typography.xs.line_height)
            .font_medium()
            .text_color(tokens.colors.muted_foreground)
            .px(tokens.spacing.md)
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder() {
        let message = Message::new()
            .alignment(MessageAlignment::End)
            .avatar(div())
            .header(MessageHeader::new().child("Alice"))
            .content(MessageContent::new().child("Hello"))
            .footer(MessageFooter::new().child("Delivered"));

        assert_eq!(message.alignment, MessageAlignment::End);
        assert!(message.avatar.is_some());
        assert!(message.header.is_some());
        assert!(message.content.is_some());
        assert!(message.footer.is_some());

        let group = MessageGroup::new().child("First").child("Second");
        assert_eq!(group.children.len(), 2);

        let content = MessageContent::new().aligned(MessageAlignment::End);
        assert_eq!(content.alignment, MessageAlignment::End);
    }
}
