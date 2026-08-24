use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, relative,
};

use crate::{ActiveTheme as _, StyledExt as _, message::MessageAlignment};

/// Visual treatment for a chat bubble.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleVariant {
    /// A filled primary surface.
    #[default]
    Filled,
    /// A background surface with a visible border.
    Outline,
    /// No surface, padding, or border.
    Ghost,
}

/// Edge on which reaction feedback is attached.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleReactionSide {
    /// Attach reactions above the bubble.
    Top,
    /// Attach reactions below the bubble.
    #[default]
    Bottom,
}

/// A chat message surface that can contain text or arbitrary rich content.
///
/// The bubble is itself the styled surface. Applications can override every
/// default through [`Styled`] and compose semantic controls as children.
#[derive(IntoElement)]
pub struct Bubble {
    style: StyleRefinement,
    alignment: MessageAlignment,
    variant: BubbleVariant,
    children: Vec<AnyElement>,
}

impl Bubble {
    /// Create a filled, leading-aligned bubble.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            alignment: MessageAlignment::Start,
            variant: BubbleVariant::Filled,
            children: Vec::new(),
        }
    }

    /// Set the bubble alignment.
    pub fn alignment(mut self, alignment: MessageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the visual treatment.
    pub fn with_variant(mut self, variant: BubbleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Default for Bubble {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Bubble {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Bubble {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Bubble {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let variant = self.variant;

        div()
            .relative()
            .flex()
            .min_w_0()
            .flex_col()
            .flex_none()
            .self_start()
            .gap(tokens.spacing.xs)
            .max_w(relative(0.8))
            .text_size(tokens.typography.sm.size)
            .line_height(tokens.typography.sm.line_height)
            .when(self.alignment == MessageAlignment::End, |this| {
                this.self_end()
            })
            .map(|this| match variant {
                BubbleVariant::Filled => this
                    .rounded(tokens.radius.xl)
                    .border_1()
                    .border_color(cx.theme().transparent)
                    .bg(tokens.colors.primary)
                    .text_color(tokens.colors.primary_foreground)
                    .px(tokens.spacing.md)
                    .py(tokens.spacing.sm),
                BubbleVariant::Outline => this
                    .rounded(tokens.radius.xl)
                    .border_1()
                    .border_color(tokens.colors.border)
                    .bg(tokens.colors.background)
                    .text_color(tokens.colors.foreground)
                    .px(tokens.spacing.md)
                    .py(tokens.spacing.sm),
                BubbleVariant::Ghost => this
                    .max_w_full()
                    .border_0()
                    .bg(cx.theme().transparent)
                    .text_color(tokens.colors.foreground)
                    .p_0(),
            })
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A styleable reaction region positioned on a bubble edge.
///
/// Compose existing [`crate::button::Button`] values inside this region to
/// preserve button semantics and keyboard behavior.
#[derive(IntoElement)]
pub struct BubbleReactions {
    style: StyleRefinement,
    side: BubbleReactionSide,
    alignment: MessageAlignment,
    children: Vec<AnyElement>,
}

impl BubbleReactions {
    /// Create a trailing-aligned reaction region on the lower edge.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            side: BubbleReactionSide::Bottom,
            alignment: MessageAlignment::End,
            children: Vec::new(),
        }
    }

    /// Set the edge on which reactions are positioned.
    pub fn side(mut self, side: BubbleReactionSide) -> Self {
        self.side = side;
        self
    }

    /// Set the reaction region alignment along the bubble edge.
    pub fn alignment(mut self, alignment: MessageAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl Default for BubbleReactions {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for BubbleReactions {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for BubbleReactions {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BubbleReactions {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        div()
            .absolute()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(tokens.spacing.xs)
            .rounded(tokens.radius.full)
            .border_2()
            .border_color(tokens.colors.background)
            .bg(tokens.colors.muted)
            .text_color(tokens.colors.foreground)
            .px(tokens.spacing.sm)
            .py(tokens.spacing.xxs)
            .text_size(tokens.typography.xs.size)
            .when(self.side == BubbleReactionSide::Top, |this| {
                this.top(-tokens.spacing.sm)
            })
            .when(self.side == BubbleReactionSide::Bottom, |this| {
                this.bottom(-tokens.spacing.sm)
            })
            .when(self.alignment == MessageAlignment::Start, |this| {
                this.left(tokens.spacing.md)
            })
            .when(self.alignment == MessageAlignment::End, |this| {
                this.right(tokens.spacing.md)
            })
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bubble_builder() {
        let bubble = Bubble::new()
            .alignment(MessageAlignment::End)
            .with_variant(BubbleVariant::Outline)
            .child("Hello");

        assert_eq!(bubble.alignment, MessageAlignment::End);
        assert_eq!(bubble.variant, BubbleVariant::Outline);
        assert_eq!(bubble.children.len(), 1);

        let reactions = BubbleReactions::new()
            .side(BubbleReactionSide::Top)
            .alignment(MessageAlignment::Start)
            .child("👍 2");

        assert_eq!(reactions.side, BubbleReactionSide::Top);
        assert_eq!(reactions.alignment, MessageAlignment::Start);
        assert_eq!(reactions.children.len(), 1);
    }
}
