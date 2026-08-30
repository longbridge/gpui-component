use std::sync::Arc;

use gpui::{HighlightStyle, Pixels, Rems, StyleRefinement, px, rems};

use crate::highlighter::HighlightTheme;

/// TextViewStyle used to customize the style for [`super::TextView`].
#[derive(Clone)]
pub struct TextViewStyle {
    /// Gap between paragraphs. Defaults to 1 rem.
    pub paragraph_gap: Rems,
    /// Base font size used to derive heading sizes.
    pub heading_base_font_size: Pixels,
    /// Optional heading-size resolver for levels 1 through 6.
    pub heading_font_size: Option<Arc<dyn Fn(u8, Pixels) -> Pixels + Send + Sync + 'static>>,
    /// Syntax-highlighting theme for fenced code blocks.
    pub highlight_theme: Arc<HighlightTheme>,
    /// Style refinement applied to code blocks.
    pub code_block: StyleRefinement,
    /// Style refinement applied to the table container.
    pub table: StyleRefinement,
    /// Style refinement applied to table header rows.
    pub table_head: StyleRefinement,
    /// Style refinement applied to table cells.
    pub table_cell: StyleRefinement,
    /// Highlight style applied to inline code.
    pub inline_code: HighlightStyle,
    /// Whether content-specific rendering should use dark-mode assets.
    pub is_dark: bool,
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self {
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            heading_font_size: None,
            highlight_theme: HighlightTheme::default_light().clone(),
            code_block: StyleRefinement::default(),
            table: StyleRefinement::default(),
            table_head: StyleRefinement::default(),
            table_cell: StyleRefinement::default(),
            inline_code: HighlightStyle::default(),
            is_dark: false,
        }
    }
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.heading_base_font_size == other.heading_base_font_size
            && match (&self.heading_font_size, &other.heading_font_size) {
                (Some(left), Some(right)) => (1..=6).all(|level| {
                    left(level, self.heading_base_font_size)
                        == right(level, other.heading_base_font_size)
                }),
                (None, None) => true,
                _ => false,
            }
            && self.highlight_theme == other.highlight_theme
            && self.code_block == other.code_block
            && self.table == other.table
            && self.table_head == other.table_head
            && self.table_cell == other.table_cell
            && self.inline_code == other.inline_code
            && self.is_dark == other.is_dark
    }
}

impl TextViewStyle {
    /// Sets the gap between paragraphs.
    pub fn paragraph_gap(mut self, gap: Rems) -> Self {
        self.paragraph_gap = gap;
        self
    }
    /// Sets the heading-size resolver for levels 1 through 6.
    pub fn heading_font_size<F>(mut self, f: F) -> Self
    where
        F: Fn(u8, Pixels) -> Pixels + Send + Sync + 'static,
    {
        self.heading_font_size = Some(Arc::new(f));
        self
    }
    /// Sets the code-block style refinement.
    pub fn code_block(mut self, style: StyleRefinement) -> Self {
        self.code_block = style;
        self
    }
    /// Sets the inline-code highlight style.
    pub fn inline_code(mut self, style: HighlightStyle) -> Self {
        self.inline_code = style;
        self
    }
    /// Sets the table-container style refinement.
    pub fn table(mut self, style: StyleRefinement) -> Self {
        self.table = style;
        self
    }
    /// Sets the table-header style refinement.
    pub fn table_head(mut self, style: StyleRefinement) -> Self {
        self.table_head = style;
        self
    }
    /// Sets the table-cell style refinement.
    pub fn table_cell(mut self, style: StyleRefinement) -> Self {
        self.table_cell = style;
        self
    }
}
