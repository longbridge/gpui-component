use std::sync::Arc;

use gpui::{HighlightStyle, Hsla, Pixels, Rems, StyleRefinement, hsla, px, rems};

/// TextViewStyle used to customize the style for [`TextView`].
#[derive(Clone)]
pub struct TextViewStyle {
    /// Default body-text color.
    pub foreground: Hsla,
    /// Secondary text color.
    pub muted_foreground: Hsla,
    /// Link text color.
    pub link: Hsla,
    /// Painted selection color.
    pub selection: Hsla,
    /// Background for fenced code blocks.
    pub code_background: Hsla,
    /// Neutral border and horizontal-rule color.
    pub border: Hsla,
    /// Gap of each paragraphs, default is 1 rem.
    pub paragraph_gap: Rems,
    /// Base font size for headings, default is 14px.
    pub heading_base_font_size: Pixels,
    /// Function to calculate heading font size based on heading level (1-6).
    ///
    /// The first parameter is the heading level (1-6), the second parameter is the base font size.
    /// The second parameter is the base font size.
    pub heading_font_size: Option<Arc<dyn Fn(u8, Pixels) -> Pixels + Send + Sync + 'static>>,
    /// The style refinement for code blocks.
    pub code_block: StyleRefinement,
    /// Style refinement applied to the table container (the bordered wrapper
    /// in wrap mode, the scroll viewport in horizontal-scroll mode).
    ///
    /// Set `overflow_x: scroll` here for adaptive table layout: columns fit
    /// their content when space allows, shrink (wrapping cell text) down to a
    /// per-column floor when the frame is narrower, and below that the table
    /// scrolls horizontally instead of squeezing further, e.g.
    /// `TextViewStyle::default().table({ let mut s = StyleRefinement::default(); s.overflow.x = Some(Overflow::Scroll); s })`.
    pub table: StyleRefinement,
    /// Style refinement applied to the header row (the first row) of a table,
    /// on top of the `table_head` background and foreground from the theme.
    pub table_head: StyleRefinement,
    /// Style refinement applied to each table cell.
    ///
    /// With the scroll layout, set `white_space: nowrap` here to keep cells
    /// on a single line — columns then never shrink and the table scrolls as
    /// soon as the content is wider than the frame.
    pub table_cell: StyleRefinement,
    /// The highlight style for inline code.
    ///
    /// When `background_color` is `None`, the neutral Base code background is
    /// used. This keeps [`TextViewStyle::default`] usable without a theme.
    pub inline_code: HighlightStyle,
    pub is_dark: bool,
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.foreground == other.foreground
            && self.muted_foreground == other.muted_foreground
            && self.link == other.link
            && self.selection == other.selection
            && self.code_background == other.code_background
            && self.border == other.border
            && self.heading_base_font_size == other.heading_base_font_size
            && match (&self.heading_font_size, &other.heading_font_size) {
                (Some(left), Some(right)) => (1..=6).all(|level| {
                    left(level, self.heading_base_font_size)
                        == right(level, other.heading_base_font_size)
                }),
                (None, None) => true,
                _ => false,
            }
            && self.code_block == other.code_block
            && self.table == other.table
            && self.table_head == other.table_head
            && self.table_cell == other.table_cell
            && self.inline_code == other.inline_code
            && self.is_dark == other.is_dark
    }
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self {
            foreground: hsla(0.62, 0.20, 0.16, 1.0),
            muted_foreground: hsla(0.62, 0.10, 0.46, 1.0),
            link: hsla(0.60, 0.75, 0.48, 1.0),
            selection: hsla(0.58, 0.85, 0.62, 0.35),
            code_background: hsla(0.62, 0.12, 0.95, 1.0),
            border: hsla(0.62, 0.10, 0.86, 1.0),
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            heading_font_size: None,
            code_block: StyleRefinement::default(),
            table: StyleRefinement::default(),
            table_head: StyleRefinement::default(),
            table_cell: StyleRefinement::default(),
            inline_code: HighlightStyle {
                background_color: Some(hsla(0.62, 0.12, 0.95, 1.0)),
                ..Default::default()
            },
            is_dark: false,
        }
    }
}

impl TextViewStyle {
    /// Derives rich-text colors from Base semantic theme tokens.
    pub fn from_theme(theme: &crate::Theme) -> Self {
        let mut style = Self::default()
            .foreground(theme.tokens.colors.foreground)
            .muted_foreground(theme.tokens.colors.muted_foreground)
            .link(theme.tokens.colors.primary);
        style.code_background = theme.tokens.colors.accent;
        style.border = theme.tokens.colors.border;
        style
    }

    pub fn foreground(mut self, color: Hsla) -> Self {
        self.foreground = color;
        self
    }

    pub fn muted_foreground(mut self, color: Hsla) -> Self {
        self.muted_foreground = color;
        self
    }

    pub fn link(mut self, color: Hsla) -> Self {
        self.link = color;
        self
    }

    pub fn selection(mut self, color: Hsla) -> Self {
        self.selection = color;
        self
    }

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

    /// Set style for inline code spans.
    pub fn inline_code(mut self, style: HighlightStyle) -> Self {
        self.inline_code = style;
        self
    }

    /// Set extra style for the table container.
    ///
    /// Set `overflow_x: scroll` on the refinement for adaptive layout: cells
    /// wrap as the frame narrows, and once columns reach their minimum width
    /// the table scrolls horizontally instead of shrinking further.
    pub fn table(mut self, style: StyleRefinement) -> Self {
        self.table = style;
        self
    }

    /// Set extra style for the table header row.
    pub fn table_head(mut self, style: StyleRefinement) -> Self {
        self.table_head = style;
        self
    }

    /// Set extra style for each table cell.
    ///
    /// With the scroll table layout, `white_space: nowrap` here keeps cells
    /// on a single line and the table scrolls whenever the content is wider
    /// than the frame.
    pub fn table_cell(mut self, style: StyleRefinement) -> Self {
        self.table_cell = style;
        self
    }

    /// Returns the [`HighlightStyle`] to use for inline code, with the neutral
    /// Base code background when no custom background was supplied.
    pub(crate) fn inline_code_highlight(&self) -> HighlightStyle {
        let mut style = self.inline_code;
        if style.background_color.is_none() {
            style.background_color = Some(hsla(0.62, 0.12, 0.95, 1.0));
        }
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_layout_fingerprint_covers_callback_table_and_theme_fields() {
        let base = TextViewStyle::default();
        let heading = base.clone().heading_font_size(|_, size| size);
        assert!(heading == base.clone().heading_font_size(|_, size| size));
        assert!(heading != base.clone().heading_font_size(|_, size| size * 2.));

        let mut table = StyleRefinement::default();
        table.text.white_space = Some(gpui::WhiteSpace::Nowrap);
        assert!(base != base.clone().table_cell(table));

        let mut dark = base.clone();
        dark.is_dark = true;
        assert!(base != dark);
    }

    #[test]
    fn cloning_preserves_the_same_heading_callback_fingerprint() {
        let style = TextViewStyle::default().heading_font_size(|_, size| size);
        assert!(style == style.clone());
    }

    #[test]
    fn default_style_is_readable_without_an_application_theme() {
        let style = TextViewStyle::default();

        assert_eq!(style.foreground.a, 1.0);
        assert_eq!(style.link.a, 1.0);
        assert!(style.selection.a > 0.0);
        assert!(style.inline_code.background_color.is_some());
        assert!(style.code_background.a > 0.0);
        assert!(style.border.a > 0.0);
    }

    #[test]
    fn from_theme_maps_base_semantic_tokens() {
        let mut theme = crate::Theme::default();
        theme.tokens.colors.foreground = gpui::rgb(0x112233).into();
        theme.tokens.colors.muted_foreground = gpui::rgb(0x445566).into();
        theme.tokens.colors.primary = gpui::rgb(0x3366ff).into();
        theme.tokens.colors.accent = gpui::rgb(0xddeeff).into();
        theme.tokens.colors.border = gpui::rgb(0x778899).into();

        let style = TextViewStyle::from_theme(&theme);
        assert_eq!(style.foreground, theme.tokens.colors.foreground);
        assert_eq!(style.link, theme.tokens.colors.primary);
        assert_eq!(style.code_background, theme.tokens.colors.accent);
        assert_eq!(style.border, theme.tokens.colors.border);
    }
}
