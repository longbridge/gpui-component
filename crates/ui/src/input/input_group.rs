use gpui::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseUpEvent, ParentElement, Pixels, RenderOnce, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use smallvec::SmallVec;

use crate::{ActiveTheme, Disableable, StyledExt, h_flex, input::InputState};

/// Alignment options for [`InputGroupAddon`].
///
/// Determines where the addon is positioned relative to the input element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InputGroupAlign {
    /// Align to the left (inline start) - default
    #[default]
    InlineStart,
    /// Align to the right (inline end)
    InlineEnd,
    /// Align to the top (block start)
    BlockStart,
    /// Align to the bottom (block end)
    BlockEnd,
}

impl InputGroupAlign {
    /// Returns padding configuration for this alignment.
    ///
    /// Returns (left, right, top, bottom) padding in pixels.
        #[inline]
    const fn padding(&self) -> (Pixels, Pixels, Pixels, Pixels) {
        let pading: Pixels = px(12.);
        match self {
            Self::InlineStart => (pading, px(0.), px(0.), px(0.)),
            Self::InlineEnd => (px(0.), pading, px(0.), px(0.)),
            Self::BlockStart => (
                pading,
                pading,
                pading,
                px(0.),
            ),
            Self::BlockEnd => (
                pading,
                pading,
                px(0.),
                pading,
            ),
        }
    }

    /// Returns whether this alignment should use full width.
    #[inline]
    const fn is_full_width(&self) -> bool {
        matches!(self, Self::BlockStart | Self::BlockEnd)
    }
}

/// A container that groups input elements with addons, text, and buttons.
///
/// `InputGroup` provides a flexible way to combine input fields with additional
/// elements like icons, buttons, or text. It supports disabled state and
/// flexible layouts (horizontal/vertical).
///
/// # Examples
///
/// ```ignore
/// // Basic search input with icon
/// InputGroup::new()
///     .child(InputGroupAddon::new().child(Icon::new(IconName::Search)))
///     .child(InputGroupInput::new(&input_state))
///     .child(
///         InputGroupAddon::new()
///             .inline_end()
///             .child(InputGroupText::new().child("12 results"))
///     )
/// ```
///
/// ```ignore
/// // Chat input with toolbar
/// InputGroup::new()
///     .flex_col()
///     .h_auto()
///     .child(InputGroupTextarea::new(&chat_state))
///     .child(
///         InputGroupAddon::new()
///             .block_end()
///             .child(Button::new("send").icon(IconName::ArrowUp))
///     )
/// ```
#[derive(IntoElement)]
pub struct InputGroup {
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 2]>,
    disabled: bool,
}

impl InputGroup {
    /// Creates a new `InputGroup`.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: SmallVec::new(),
            disabled: false,
        }
    }
}

impl ParentElement for InputGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InputGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for InputGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for InputGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.disabled;

        div()
            .w_full()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().input)
            .bg(if disabled {
                cx.theme().muted
            } else {
                cx.theme().background
            })
            .when(cx.theme().shadow, |this| this.shadow_xs())
            .when(disabled, |this| this.cursor_not_allowed())
            .refine_style(&self.style)
            .map(|this| {
                if self.style.flex_direction.is_none() {
                    this.flex().items_center().h_9()
                } else {
                    this
                }
            })
            .children(self.children)
            .when(disabled, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .cursor_not_allowed()
                        .on_mouse_down(
                            MouseButton::Left,
                            |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                            },
                        )
                        .on_mouse_down(
                            MouseButton::Right,
                            |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                            },
                        )
                        .on_mouse_down(
                            MouseButton::Middle,
                            |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                            },
                        )
                        .on_mouse_up(
                            MouseButton::Left,
                            |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                            },
                        )
                        .on_mouse_up(
                            MouseButton::Right,
                            |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                            },
                        )
                        .on_mouse_up(
                            MouseButton::Middle,
                            |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                            },
                        )
                        .on_scroll_wheel(|_: &gpui::ScrollWheelEvent, _: &mut Window, cx| {
                            cx.stop_propagation();
                        })
                        .on_key_down(|_: &gpui::KeyDownEvent, _: &mut Window, cx| {
                            cx.stop_propagation();
                        }),
                )
            })
    }
}

/// An addon container for [`InputGroup`] that can hold icons, text, or buttons.
///
/// Addons provide additional context or functionality to input fields.
/// They can be aligned to different positions using [`InputGroupAlign`].
///
/// # Examples
///
/// ```ignore
/// // Left-aligned icon
/// InputGroupAddon::new()
///     .child(Icon::new(IconName::Search).small())
///
/// // Right-aligned button
/// InputGroupAddon::new()
///     .inline_end()
///     .child(Button::new("clear").icon(IconName::Close))
/// ```
#[derive(IntoElement)]
pub struct InputGroupAddon {
    align: InputGroupAlign,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 1]>,
}

impl InputGroupAddon {
    /// Creates a new `InputGroupAddon`.
    pub fn new() -> Self {
        Self {
            align: InputGroupAlign::default(),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }

    /// Sets the alignment of the addon.
    pub fn align(mut self, align: InputGroupAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets the alignment to BlockEnd (bottom).
    pub fn block_end(mut self) -> Self {
        self.align = InputGroupAlign::BlockEnd;
        self
    }

    /// Sets the alignment to BlockStart (top).
    pub fn block_start(mut self) -> Self {
        self.align = InputGroupAlign::BlockStart;
        self
    }

    /// Sets the alignment to InlineEnd (right).
    pub fn inline_end(mut self) -> Self {
        self.align = InputGroupAlign::InlineEnd;
        self
    }

    /// Sets the alignment to InlineStart (left).
    pub fn inline_start(mut self) -> Self {
        self.align = InputGroupAlign::InlineStart;
        self
    }
}

impl Default for InputGroupAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for InputGroupAddon {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InputGroupAddon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupAddon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (pl, pr, pt, pb) = self.align.padding();

        h_flex()
            .items_center()
            .gap_2()
            .text_color(cx.theme().muted_foreground)
            .text_sm()
            .font_medium()
            .pl(pl)
            .pr(pr)
            .pt(pt)
            .pb(pb)
            .when(self.align.is_full_width(), |this| this.w_full())
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A text element for use within [`InputGroupAddon`].
///
/// Provides a simple way to display text content alongside input fields.
///
/// # Examples
///
/// ```ignore
/// // Simple text
/// InputGroupText::new().child("https://")
///
/// // Custom styled text
/// InputGroupText::new()
///     .text_color(theme.primary)
///     .child("Custom text")
/// ```
#[derive(IntoElement)]
pub struct InputGroupText {
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 1]>,
}

impl InputGroupText {
    /// Creates a new `InputGroupText`.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }
}

impl Default for InputGroupText {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for InputGroupText {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InputGroupText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupText {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .text_sm()
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A simplified input element for use within [`InputGroup`].
///
/// This component wraps [`Input`](crate::input::Input) with appearance
/// and border removed to integrate seamlessly with the group container.
///
/// # Examples
///
/// ```ignore
/// InputGroupInput::new(&input_state)
///     .placeholder("Enter text...")
///     .flex_1()
/// ```
#[derive(IntoElement)]
pub struct InputGroupInput {
    state: Entity<InputState>,
    style: StyleRefinement,
}

impl InputGroupInput {
    /// Creates a new `InputGroupInput` bound to the given state.
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for InputGroupInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupInput {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        crate::input::Input::new(&self.state)
            .appearance(false)
            .bordered(false)
            .refine_style(&self.style)
    }
}

/// A simplified textarea element for use within [`InputGroup`].
///
/// Similar to [`InputGroupInput`] but for multi-line text input.
///
/// # Examples
///
/// ```ignore
/// InputGroupTextarea::new(&textarea_state)
///     .h(px(120.))
///     .flex_1()
/// ```
#[derive(IntoElement)]
pub struct InputGroupTextarea {
    state: Entity<InputState>,
    style: StyleRefinement,
}

impl InputGroupTextarea {
    /// Creates a new `InputGroupTextarea` bound to the given state.
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for InputGroupTextarea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupTextarea {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        crate::input::Input::new(&self.state)
            .appearance(false)
            .bordered(false)
            .refine_style(&self.style)
    }
}
