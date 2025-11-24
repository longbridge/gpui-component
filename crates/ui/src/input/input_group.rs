use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, DefiniteLength, ElementId,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, RenderOnce, StyleRefinement,
    Styled, Window,
};
use smallvec::SmallVec;

use crate::{h_flex, ActiveTheme, Disableable, StyledExt as _};

// === Constants ===

const DEFAULT_INPUT_GROUP_ID: &str = "input-group";
const DEFAULT_ADDON_PADDING: Pixels = px(12.);
const DEFAULT_TEXTAREA_HEIGHT: Pixels = px(80.);

// === Types ===

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
        match self {
            Self::InlineStart => (DEFAULT_ADDON_PADDING, px(0.), px(0.), px(0.)),
            Self::InlineEnd => (px(0.), DEFAULT_ADDON_PADDING, px(0.), px(0.)),
            Self::BlockStart => (
                DEFAULT_ADDON_PADDING,
                DEFAULT_ADDON_PADDING,
                DEFAULT_ADDON_PADDING,
                px(0.),
            ),
            Self::BlockEnd => (
                DEFAULT_ADDON_PADDING,
                DEFAULT_ADDON_PADDING,
                px(0.),
                DEFAULT_ADDON_PADDING,
            ),
        }
    }

    /// Returns whether this alignment should use full width.
    #[inline]
    const fn is_full_width(&self) -> bool {
        matches!(self, Self::BlockStart | Self::BlockEnd)
    }
}

// === InputGroup ===

/// A container that groups input elements with addons, text, and buttons.
///
/// `InputGroup` provides a flexible way to combine input fields with additional
/// elements like icons, buttons, or text. It supports various states (disabled,
/// invalid) and flexible layouts (horizontal/vertical).
///
/// # Examples
///
/// ```ignore
/// // Basic search input with icon
/// InputGroup::new()
///     .child(InputGroupAddon::new().child(Icon::new(IconName::Search)))
///     .child(InputGroupInput::new(&input_state).placeholder("Search..."))
///     .child(
///         InputGroupAddon::new()
///             .align(InputGroupAlign::InlineEnd)
///             .child(InputGroupText::label("12 results"))
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
///             .align(InputGroupAlign::BlockEnd)
///             .child(Button::new("send").icon(IconName::ArrowUp))
///     )
/// ```
#[derive(IntoElement)]
pub struct InputGroup {
    id: Option<ElementId>,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 2]>,
    disabled: bool,
    invalid: bool,
}

impl InputGroup {
    /// Creates a new `InputGroup`.
    pub fn new() -> Self {
        Self {
            id: None,
            style: StyleRefinement::default(),
            children: SmallVec::new(),
            disabled: false,
            invalid: false,
        }
    }

    /// Sets the element ID.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Marks the input group as invalid/error state.
    ///
    /// When `true`, the border color changes to indicate an error.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Returns the appropriate border color based on state.
    #[inline]
    fn get_border_color(&self, cx: &App) -> gpui::Hsla {
        if self.invalid {
            cx.theme().danger
        } else {
            cx.theme().input
        }
    }
}

impl Default for InputGroup {
    fn default() -> Self {
        Self::new()
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
        let border = self.get_border_color(cx);

        div()
            .id(self.id.unwrap_or_else(|| DEFAULT_INPUT_GROUP_ID.into()))
            .w_full()
            .min_w_0()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(border)
            .bg(cx.theme().background)
            .when(cx.theme().shadow, |this| this.shadow_xs())
            .when(self.disabled, |this| {
                this.opacity(0.5).cursor_not_allowed()
            })
            .refine_style(&self.style)
            .map(|this| {
                // Apply default horizontal layout only if no custom flex-direction is set
                if self.style.flex_direction.is_none() {
                    this.flex().items_center().h_9()
                } else {
                    this
                }
            })
            .children(self.children)
    }
}

// === InputGroupAddon ===

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
///     .align(InputGroupAlign::InlineEnd)
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
        let padding = self.align.padding();

        h_flex()
            .items_center()
            .gap_2()
            .text_color(cx.theme().muted_foreground)
            .text_sm()
            .font_medium()
            .cursor_text()
            .pl(padding.0)
            .pr(padding.1)
            .pt(padding.2)
            .pb(padding.3)
            .when(self.align.is_full_width(), |this| this.w_full())
            .refine_style(&self.style)
            .children(self.children)
    }
}

// === InputGroupText ===

/// A text element for use within [`InputGroupAddon`].
///
/// Provides a simple way to display text content alongside input fields.
///
/// # Examples
///
/// ```ignore
/// // Simple text label
/// InputGroupText::label("https://")
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

    /// Creates a text label with default styling.
    ///
    /// This is a convenience method for quickly creating text elements.
    pub fn label(text: impl Into<String>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .text_sm()
            .child(text.into())
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

// === InputGroupInput ===

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
    state: gpui::Entity<crate::input::InputState>,
    style: StyleRefinement,
}

impl InputGroupInput {
    /// Creates a new `InputGroupInput` bound to the given state.
    pub fn new(state: &gpui::Entity<crate::input::InputState>) -> Self {
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

// === InputGroupTextarea ===

/// A simplified textarea element for use within [`InputGroup`].
///
/// Similar to [`InputGroupInput`] but for multi-line text input.
/// Provides a configurable minimum height.
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
    state: gpui::Entity<crate::input::InputState>,
    style: StyleRefinement,
    height: Option<DefiniteLength>,
}

impl InputGroupTextarea {
    /// Creates a new `InputGroupTextarea` bound to the given state.
    ///
    /// The default height is 80px, which can be overridden using `.h()`.
    pub fn new(state: &gpui::Entity<crate::input::InputState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            height: Some(DEFAULT_TEXTAREA_HEIGHT.into()),
        }
    }

    /// Sets the height of the textarea.
    ///
    /// Pass `None` to remove the default height constraint.
    pub fn height(mut self, height: impl Into<Option<DefiniteLength>>) -> Self {
        self.height = height.into();
        self
    }
}

impl Styled for InputGroupTextarea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupTextarea {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut input = crate::input::Input::new(&self.state)
            .appearance(false)
            .bordered(false);

        if let Some(height) = self.height {
            input = input.h(height);
        }

        input.refine_style(&self.style)
    }
}
