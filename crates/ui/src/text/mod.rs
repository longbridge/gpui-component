//! Compatibility facade for rich text now owned by `gpui-base`.

pub use gpui_base::text::*;

#[cfg(test)]
mod window_selection;

/// Derives a rich-text style from the active component theme.
///
/// Syntax highlighting remains opt-in through
/// [`TextView::code_block_highlighter`].
pub fn text_view_style(theme: &crate::Theme) -> TextViewStyle {
    let mut style = TextViewStyle::default()
        .foreground(theme.foreground)
        .muted_foreground(theme.muted_foreground)
        .link(theme.link)
        .selection(theme.selection);
    style.code_background = theme.muted;
    style.border = theme.border;
    style
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
