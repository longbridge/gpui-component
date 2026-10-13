use gpui::{Hsla, Pixels, px};

use crate::SemanticThemeTokens;

/// Presentation seam for [`super::BlockEditor`]. Private fields, `with_*`
/// setters and readers (ARCHITECTURE.md invariant 12). Unset colours are
/// transparent and are filled from the theme tokens by [`Self::resolved`],
/// mirroring `InputEditorStyle::resolved`.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockEditorStyle {
    foreground: Hsla,
    muted_foreground: Hsla,
    caret: Hsla,
    caret_width: Pixels,
    selection: Hsla,
    code_background: Hsla,
    composition_underline: Hsla,
    block_gap: f32,
    indent: Pixels,
}

impl Default for BlockEditorStyle {
    fn default() -> Self {
        let unset = Hsla::transparent_black();
        Self {
            foreground: unset,
            muted_foreground: unset,
            caret: unset,
            caret_width: px(1.5),
            selection: unset,
            code_background: unset,
            composition_underline: unset,
            block_gap: 0.5,
            indent: px(24.),
        }
    }
}

macro_rules! style_field {
    ($with:ident, $get:ident, $ty:ty) => {
        pub fn $with(mut self, value: $ty) -> Self {
            self.$get = value;
            self
        }
        pub fn $get(&self) -> $ty {
            self.$get
        }
    };
}

impl BlockEditorStyle {
    style_field!(with_foreground, foreground, Hsla);
    style_field!(with_muted_foreground, muted_foreground, Hsla);
    style_field!(with_caret, caret, Hsla);
    style_field!(with_caret_width, caret_width, Pixels);
    style_field!(with_selection, selection, Hsla);
    style_field!(with_code_background, code_background, Hsla);
    style_field!(with_composition_underline, composition_underline, Hsla);
    // Vertical gap between blocks, in line heights.
    style_field!(with_block_gap, block_gap, f32);
    // Indent per list or blockquote level.
    style_field!(with_indent, indent, Pixels);

    /// Fill unset colours from the active palette.
    pub fn resolved(&self, tokens: &SemanticThemeTokens) -> Self {
        let colors = &tokens.colors;
        let or = |value: Hsla, fallback: Hsla| if value.a == 0. { fallback } else { value };
        let foreground = or(self.foreground, colors.foreground);
        Self {
            foreground,
            muted_foreground: or(self.muted_foreground, colors.muted_foreground),
            caret: or(self.caret, foreground),
            selection: or(self.selection, colors.selection),
            code_background: or(self.code_background, colors.muted),
            composition_underline: or(self.composition_underline, foreground),
            ..self.clone()
        }
    }
}
