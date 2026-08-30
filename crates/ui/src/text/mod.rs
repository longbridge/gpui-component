//! Compatibility facade for rich text now owned by `gpui-base`.

mod compat;
mod style;

pub use compat::{
    Text, TextView, TextViewLayoutState, TextViewPlugin, TextViewPrepaintState, html, markdown,
};
pub use gpui_base::text::{
    MarkdownBlockParserFn, MarkdownBlockRenderFn, MarkdownExtensions, MarkdownNode,
    MarkdownParseContext, MarkdownPlugin, SelectionFormat, TableData, TextViewState, markdown_ast,
};
pub use style::TextViewStyle;

#[cfg(feature = "tree-sitter")]
use std::{cell::RefCell, collections::HashMap};

use gpui::Styled as _;
#[cfg(feature = "tree-sitter")]
use gpui_base::input::{InputEdit, Point, RopeExt as _};
#[cfg(feature = "tree-sitter")]
use ropey::Rope;

#[cfg(feature = "tree-sitter")]
use crate::highlighter::{LanguageRegistry, SyntaxHighlighter};

#[cfg(test)]
mod window_selection;

/// Derives the Base rich-text style installed by the component theme adapter.
pub(crate) fn base_text_view_style(theme: &crate::Theme) -> gpui_base::TextViewStyle {
    let radius = theme.semantic_tokens().radius.md;
    let mut table = gpui::StyleRefinement::default();
    table.corner_radii.top_left = Some(radius.into());
    table.corner_radii.top_right = Some(radius.into());
    table.corner_radii.bottom_left = Some(radius.into());
    table.corner_radii.bottom_right = Some(radius.into());
    let mut code_block = gpui::StyleRefinement::default();
    code_block.corner_radii = table.corner_radii.clone();
    let table_head = gpui::StyleRefinement::default()
        .bg(theme.table_head)
        .text_color(theme.table_head_foreground);

    let mut style = gpui_base::TextViewStyle::default()
        .foreground(theme.foreground)
        .muted_foreground(theme.muted_foreground)
        .link(theme.link)
        .selection(theme.selection)
        .code_block(code_block)
        .table(table)
        .table_head(table_head);
    style.code_background = theme.muted;
    style.border = theme.border;
    style.inline_code.background_color = Some(theme.accent);
    style.is_dark = theme.is_dark();
    style
}

pub(crate) fn install_text_view_defaults(theme: &crate::Theme, cx: &mut gpui::App) {
    let defaults = gpui_base::TextViewDefaults::new().style(base_text_view_style(theme));

    #[cfg(feature = "tree-sitter")]
    let defaults = defaults.code_block_highlighter(component_code_block_highlighter(
        theme.highlight_theme.clone(),
    ));

    defaults.install(cx);
}

#[cfg(feature = "tree-sitter")]
pub(crate) fn component_code_block_highlighter(
    highlight_theme: std::sync::Arc<crate::highlighter::HighlightTheme>,
) -> impl Fn(&gpui_base::text::CodeBlock) -> Vec<(std::ops::Range<usize>, gpui::HighlightStyle)>
+ Send
+ Sync
+ 'static {
    move |block| {
        thread_local! {
            static HIGHLIGHTERS: RefCell<HashMap<gpui::SharedString, SyntaxHighlighter>> =
                RefCell::new(HashMap::new());
        }

        let Some(lang) = block.lang() else {
            return Vec::new();
        };
        let code = block.code();
        HIGHLIGHTERS.with(|cache| {
            let mut cache = cache.borrow_mut();
            let highlighter = cache
                .entry(lang.clone())
                .or_insert_with(|| SyntaxHighlighter::new(lang.as_ref()));
            if let Some(config) = LanguageRegistry::singleton().language(lang.as_ref())
                && highlighter.language() != &config.name
            {
                *highlighter = SyntaxHighlighter::new(lang.as_ref());
            }

            let old_end_byte = highlighter.text().len();
            let old_end_position = highlighter.text().offset_to_point(old_end_byte);
            let code_rope = Rope::from_str(code.as_ref());
            let edit = InputEdit {
                start_byte: 0,
                old_end_byte,
                new_end_byte: code.len(),
                start_position: Point::new(0, 0),
                old_end_position,
                new_end_position: code_rope.offset_to_point(code.len()),
            };
            highlighter.update_input(Some(edit), &code_rope, None);
            highlighter.styles(&(0..code.len()), highlight_theme.as_ref())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Theme;

    #[test]
    fn component_theme_adapter_maps_text_colors_without_highlighting() {
        let theme = Theme::default();
        let style = super::base_text_view_style(&theme);

        assert_eq!(style.foreground, theme.foreground);
        assert_eq!(style.muted_foreground, theme.muted_foreground);
        assert_eq!(style.link, theme.link);
        assert_eq!(style.selection, theme.selection);
        assert_eq!(style.inline_code.background_color, Some(theme.accent));
        let radius = theme.semantic_tokens().radius.md;
        assert_eq!(style.table.corner_radii.top_left, Some(radius.into()));
        assert_eq!(style.table.corner_radii.top_right, Some(radius.into()));
        assert_eq!(style.table.corner_radii.bottom_left, Some(radius.into()));
        assert_eq!(style.table.corner_radii.bottom_right, Some(radius.into()));
    }

    #[test]
    fn component_text_view_table_respects_square_base_radius_token() {
        let mut theme = Theme::default();
        theme.radius = gpui::px(0.);

        let style = super::base_text_view_style(&theme);
        let square = Some(gpui::px(0.).into());
        assert_eq!(style.table.corner_radii.top_left, square);
        assert_eq!(style.table.corner_radii.top_right, square);
        assert_eq!(style.table.corner_radii.bottom_left, square);
        assert_eq!(style.table.corner_radii.bottom_right, square);
    }

    #[cfg(feature = "tree-sitter")]
    #[gpui::test]
    fn component_initialization_installs_default_code_highlighting(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);

        cx.update(|cx| {
            assert!(gpui_base::TextViewDefaults::global(cx).has_code_block_highlighter());
        });
    }

    #[test]
    fn legacy_text_paths_reexport_base_implementation() {
        let mut style = super::TextViewStyle::default();
        style.highlight_theme = crate::highlighter::HighlightTheme::default_dark();

        let _: super::TextView = super::markdown("# compatible")
            .style(style)
            .selectable(true)
            .scrollable(true);
    }

    #[test]
    fn legacy_text_view_keeps_element_associated_types() {
        fn assert_element_types<T>()
        where
            T: gpui::Element<
                    RequestLayoutState = super::TextViewLayoutState,
                    PrepaintState = super::TextViewPrepaintState,
                >,
        {
        }

        assert_element_types::<super::TextView>();
    }

    #[test]
    fn legacy_default_style_keeps_active_component_theme_colors() {
        let mut theme = Theme::default();
        theme.foreground = gpui::rgb(0xf4f4f5).into();
        theme.link = gpui::rgb(0x38bdf8).into();
        theme.selection = gpui::rgba(0x2563eb66).into();

        let style = super::compat::resolve_component_style(&theme, super::TextViewStyle::default());

        assert_eq!(style.foreground, theme.foreground);
        assert_eq!(style.link, theme.link);
        assert_eq!(style.selection, theme.selection);
    }

    #[test]
    fn legacy_table_refinement_keeps_component_radius() {
        let theme = Theme::default();
        let mut table = gpui::StyleRefinement::default();
        table.overflow.x = Some(gpui::Overflow::Scroll);

        let style = super::compat::resolve_component_style(
            &theme,
            super::TextViewStyle::default().table(table),
        );

        let radius = Some(theme.semantic_tokens().radius.md.into());
        assert_eq!(style.table.corner_radii.top_left, radius);
        assert_eq!(style.table.corner_radii.top_right, radius);
        assert_eq!(style.table.corner_radii.bottom_left, radius);
        assert_eq!(style.table.corner_radii.bottom_right, radius);
        assert_eq!(style.table.overflow.x, Some(gpui::Overflow::Scroll));
    }

    #[test]
    fn legacy_partial_styles_refine_component_theme_defaults() {
        let theme = Theme::default();
        let mut table_head = gpui::StyleRefinement::default();
        table_head.text.font_weight = Some(gpui::FontWeight::BOLD);
        let inline_code = gpui::HighlightStyle {
            font_style: Some(gpui::FontStyle::Italic),
            ..Default::default()
        };

        let style = super::compat::resolve_component_style(
            &theme,
            super::TextViewStyle::default()
                .table_head(table_head)
                .inline_code(inline_code),
        );

        assert_eq!(style.table_head.background, Some(theme.table_head.into()));
        assert_eq!(
            style.table_head.text.color,
            Some(theme.table_head_foreground)
        );
        assert_eq!(
            style.table_head.text.font_weight,
            Some(gpui::FontWeight::BOLD)
        );
        assert_eq!(style.inline_code.background_color, Some(theme.accent));
        assert_eq!(style.inline_code.font_style, Some(gpui::FontStyle::Italic));
    }
}
