use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
    prelude::FluentBuilder as _, relative, rems,
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
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        v_flex()
            .min_w_0()
            .gap_2()
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
    stack_style: StyleRefinement,
    alignment: MessageAlignment,
    avatar: Option<MessageAvatar>,
    header: Option<MessageHeader>,
    content: Option<MessageContent>,
    footer: Option<MessageFooter>,
}

impl Message {
    /// Create a leading-aligned message.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            stack_style: StyleRefinement::default(),
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

    /// Refine the inner vertical stack that contains the named slots.
    pub fn with_stack_style(mut self, style: StyleRefinement) -> Self {
        self.stack_style = style;
        self
    }

    /// Set an optional avatar or other sender identity element.
    pub fn avatar(mut self, avatar: impl IntoElement) -> Self {
        self.avatar = Some(MessageAvatar::new().child(avatar));
        self
    }

    /// Set a fully configured avatar slot.
    pub fn avatar_slot(mut self, avatar: MessageAvatar) -> Self {
        self.avatar = Some(avatar);
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
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let alignment = self.alignment;
        let has_footer = self.footer.is_some();
        let stack_style = self.stack_style;

        h_flex()
            .relative()
            .w_full()
            .min_w_0()
            .items_end()
            .gap_2()
            .text_sm()
            .line_height(relative(1.25))
            .when(alignment == MessageAlignment::End, |this| {
                this.flex_row_reverse()
            })
            .refine_style(&self.style)
            .when_some(self.avatar, |this, avatar| {
                this.child(avatar.with_footer_offset(has_footer))
            })
            .child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(rems(0.625))
                    .map(|this| match alignment {
                        MessageAlignment::Start => this.items_start(),
                        MessageAlignment::End => this.items_end(),
                    })
                    .refine_style(&stack_style)
                    .when_some(self.header, |this, header| this.child(header))
                    .when_some(self.content, |this, content| {
                        this.child(content.aligned(alignment))
                    })
                    .when_some(self.footer, |this, footer| this.child(footer)),
            )
    }
}

/// The sender identity slot rendered beside a [`Message`].
///
/// The slot reserves a 32 pixel baseline and moves above a message footer so
/// the avatar remains aligned with the visible message surface.
#[derive(IntoElement)]
pub struct MessageAvatar {
    style: StyleRefinement,
    footer_offset: bool,
    children: Vec<AnyElement>,
}

impl MessageAvatar {
    /// Create an empty avatar slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            footer_offset: false,
            children: Vec::new(),
        }
    }

    fn with_footer_offset(mut self, footer_offset: bool) -> Self {
        self.footer_offset = footer_offset;
        self
    }
}

impl Default for MessageAvatar {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MessageAvatar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MessageAvatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageAvatar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        h_flex()
            .relative()
            .min_w_8()
            .flex_none()
            .items_center()
            .justify_center()
            .self_end()
            .overflow_hidden()
            .rounded(cx.theme().radius_full())
            .bg(tokens.colors.muted)
            .when(self.footer_offset, |this| this.bottom_8())
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Header content such as a sender name and timestamp.
#[derive(IntoElement)]
pub struct MessageHeader {
    style: StyleRefinement,
    content_inset: bool,
    children: Vec<AnyElement>,
}

impl MessageHeader {
    /// Create an empty message header.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            content_inset: true,
            children: Vec::new(),
        }
    }

    /// Set whether the header keeps its default horizontal content inset.
    pub fn content_inset(mut self, content_inset: bool) -> Self {
        self.content_inset = content_inset;
        self
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
            .gap_1()
            .text_xs()
            .line_height(relative(1.25))
            .font_medium()
            .text_color(tokens.colors.muted_foreground)
            .when(self.content_inset, |this| this.px_3())
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
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            .max_w_full()
            .min_w_0()
            .gap(rems(0.625))
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
    content_inset: bool,
    children: Vec<AnyElement>,
}

impl MessageFooter {
    /// Create an empty message footer.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            content_inset: true,
            children: Vec::new(),
        }
    }

    /// Set whether the footer keeps its default horizontal content inset.
    pub fn content_inset(mut self, content_inset: bool) -> Self {
        self.content_inset = content_inset;
        self
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
            .gap_1()
            .text_xs()
            .line_height(relative(1.25))
            .font_medium()
            .text_color(tokens.colors.muted_foreground)
            .when(self.content_inset, |this| this.px_3())
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder() {
        let stack_style = StyleRefinement::default().gap_1();
        let message = Message::new()
            .alignment(MessageAlignment::End)
            .with_stack_style(stack_style.clone())
            .avatar_slot(MessageAvatar::new().child(gpui::div()))
            .header(MessageHeader::new().content_inset(false).child("Alice"))
            .content(MessageContent::new().child("Hello"))
            .footer(MessageFooter::new().content_inset(false).child("Delivered"));

        assert_eq!(message.alignment, MessageAlignment::End);
        assert_eq!(message.stack_style, stack_style);
        assert!(message.avatar.is_some());
        assert!(message.header.is_some());
        assert!(message.content.is_some());
        assert!(message.footer.is_some());
        assert!(!message.avatar.as_ref().unwrap().footer_offset);
        assert!(!message.header.as_ref().unwrap().content_inset);
        assert!(!message.footer.as_ref().unwrap().content_inset);

        let group = MessageGroup::new().child("First").child("Second");
        assert_eq!(group.children.len(), 2);

        let content = MessageContent::new().aligned(MessageAlignment::End);
        assert_eq!(content.alignment, MessageAlignment::End);

        let avatar = MessageAvatar::new().with_footer_offset(true).child("ME");
        assert!(avatar.footer_offset);
        assert_eq!(avatar.children.len(), 1);
    }
}
