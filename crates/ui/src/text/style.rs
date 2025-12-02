use std::sync::Arc;

use gpui::{
    AnyElement, App, IntoElement, Pixels, Rems, SharedString, StyleRefinement, Window, px, rems,
};

use crate::highlighter::HighlightTheme;

/// Function signature for rendering custom code block actions (e.g., copy button, run button).
///
/// Parameters:
/// - `code`: The code content of the block
/// - `lang`: Optional language identifier (e.g., "rust", "python")
/// - `window`: Window context
/// - `cx`: App context
pub type CodeBlockActionsFn = dyn Fn(SharedString, Option<SharedString>, &mut Window, &mut App) -> AnyElement
    + Send
    + Sync
    + 'static;

/// TextViewStyle used to customize the style for [`TextView`].
#[derive(Clone)]
pub struct TextViewStyle {
    /// Gap of each paragraphs, default is 1 rem.
    pub paragraph_gap: Rems,
    /// Base font size for headings, default is 14px.
    pub heading_base_font_size: Pixels,
    /// Function to calculate heading font size based on heading level (1-6).
    ///
    /// The first parameter is the heading level (1-6), the second parameter is the base font size.
    /// The second parameter is the base font size.
    pub heading_font_size: Option<Arc<dyn Fn(u8, Pixels) -> Pixels + Send + Sync + 'static>>,
    /// Highlight theme for code blocks. Default: [`HighlightTheme::default_light()`]
    pub highlight_theme: Arc<HighlightTheme>,
    /// The style refinement for code blocks.
    pub code_block: StyleRefinement,
    /// Optional slot for rendering actions on code blocks (copy, run, etc.)
    pub code_block_actions: Option<Arc<CodeBlockActionsFn>>,
    pub is_dark: bool,
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.heading_base_font_size == other.heading_base_font_size
            && self.highlight_theme == other.highlight_theme
    }
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self {
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            heading_font_size: None,
            highlight_theme: HighlightTheme::default_light().clone(),
            code_block: StyleRefinement::default(),
            code_block_actions: None,
            is_dark: false,
        }
    }
}

impl TextViewStyle {
    /// Set paragraph gap, default is 1 rem.
    pub fn paragraph_gap(mut self, gap: Rems) -> Self {
        self.paragraph_gap = gap;
        self
    }

    pub fn heading_font_size<F>(mut self, f: F) -> Self
    where
        F: Fn(u8, Pixels) -> Pixels + Send + Sync + 'static,
    {
        self.heading_font_size = Some(Arc::new(f));
        self
    }

    /// Set style for code blocks.
    pub fn code_block(mut self, style: StyleRefinement) -> Self {
        self.code_block = style;
        self
    }

    /// Set a custom renderer for code block actions (e.g., copy button, run button).
    ///
    /// The function receives the code content and optional language, and should return
    /// an element to render in the top-right corner of the code block.
    pub fn code_block_actions<F, E>(mut self, f: F) -> Self
    where
        F: Fn(SharedString, Option<SharedString>, &mut Window, &mut App) -> E
            + Send
            + Sync
            + 'static,
        E: IntoElement,
    {
        self.code_block_actions = Some(Arc::new(move |code, lang, window, cx| {
            f(code, lang, window, cx).into_any_element()
        }));
        self
    }
}
