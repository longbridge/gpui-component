//! Compatibility facade for rich text now owned by `gpui-base`.

pub use gpui_base::text::*;

#[cfg(feature = "tree-sitter")]
use std::{cell::RefCell, collections::HashMap};

#[cfg(feature = "tree-sitter")]
use gpui_base::input::{InputEdit, Point, RopeExt as _};
#[cfg(feature = "tree-sitter")]
use ropey::Rope;

#[cfg(feature = "tree-sitter")]
use crate::highlighter::{LanguageRegistry, SyntaxHighlighter};

#[cfg(test)]
mod window_selection;

/// Derives a rich-text style from the active component theme.
///
/// Syntax highlighting remains opt-in through
/// [`TextView::code_block_highlighter`].
pub fn text_view_style(theme: &crate::Theme) -> TextViewStyle {
    let radius = theme.semantic_tokens().radius.md;
    let mut table = gpui::StyleRefinement::default();
    table.corner_radii.top_left = Some(radius.into());
    table.corner_radii.top_right = Some(radius.into());
    table.corner_radii.bottom_left = Some(radius.into());
    table.corner_radii.bottom_right = Some(radius.into());
    let mut code_block = gpui::StyleRefinement::default();
    code_block.corner_radii = table.corner_radii.clone();

    let mut style = TextViewStyle::default()
        .foreground(theme.foreground)
        .muted_foreground(theme.muted_foreground)
        .link(theme.link)
        .selection(theme.selection)
        .code_block(code_block)
        .table(table);
    style.code_background = theme.muted;
    style.border = theme.border;
    style.inline_code.background_color = Some(theme.muted);
    style.is_dark = theme.is_dark();
    style
}

pub(crate) fn install_text_view_defaults(theme: &crate::Theme, cx: &mut gpui::App) {
    let defaults = gpui_base::TextViewDefaults::new().style(text_view_style(theme));

    #[cfg(feature = "tree-sitter")]
    let defaults = {
        let highlight_theme = theme.highlight_theme.clone();
        defaults.code_block_highlighter(move |block| {
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
        })
    };

    defaults.install(cx);
}

#[cfg(test)]
mod tests {
    use crate::Theme;

    #[test]
    fn component_theme_adapter_maps_text_colors_without_highlighting() {
        let theme = Theme::default();
        let style = super::text_view_style(&theme);

        assert_eq!(style.foreground, theme.foreground);
        assert_eq!(style.muted_foreground, theme.muted_foreground);
        assert_eq!(style.link, theme.link);
        assert_eq!(style.selection, theme.selection);
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

        let style = super::text_view_style(&theme);
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
        let _: gpui_base::TextView = super::markdown("# compatible")
            .style(super::TextViewStyle::default())
            .selectable(true)
            .scrollable(true)
            .code_block_highlighter(|_| Vec::new());
    }
}
