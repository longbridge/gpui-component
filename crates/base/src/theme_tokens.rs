//! Semantic design tokens shared by application-owned components.
//!
//! These tokens describe visual roles and scales. They intentionally do not
//! contain component names such as `button`, `table`, or `sidebar`.

use gpui::{BoxShadow, FontWeight, Hsla, Pixels, SharedString, point, px};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SemanticThemeTokens {
    pub colors: ColorTokens,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
    pub typography: TypographyTokens,
    pub shadow: ShadowTokens,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ColorTokens {
    pub background: Hsla,
    pub foreground: Hsla,
    pub surface: Hsla,
    pub surface_foreground: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub destructive_foreground: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RadiusTokens {
    pub none: Pixels,
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub xl: Pixels,
    pub full: Pixels,
}

impl Default for RadiusTokens {
    fn default() -> Self {
        Self {
            none: px(0.),
            sm: px(3.),
            md: px(6.),
            lg: px(8.),
            xl: px(12.),
            full: px(9999.),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpacingTokens {
    pub xxs: Pixels,
    pub xs: Pixels,
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub xl: Pixels,
    pub xxl: Pixels,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self {
            xxs: px(2.),
            xs: px(4.),
            sm: px(8.),
            md: px(12.),
            lg: px(16.),
            xl: px(24.),
            xxl: px(32.),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextStyleToken {
    pub size: Pixels,
    pub line_height: Pixels,
    pub weight: FontWeight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TypographyTokens {
    pub sans: SharedString,
    pub mono: SharedString,
    pub xs: TextStyleToken,
    pub sm: TextStyleToken,
    pub md: TextStyleToken,
    pub lg: TextStyleToken,
    pub xl: TextStyleToken,
    pub mono_md: TextStyleToken,
}

impl Default for TypographyTokens {
    fn default() -> Self {
        Self {
            sans: ".SystemUIFont".into(),
            mono: default_mono_font_family(),
            xs: text_style(12., 16.),
            sm: text_style(14., 20.),
            md: text_style(16., 24.),
            lg: text_style(18., 28.),
            xl: text_style(20., 28.),
            mono_md: text_style(13., 20.),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShadowTokens {
    pub sm: Vec<BoxShadow>,
    pub md: Vec<BoxShadow>,
    pub lg: Vec<BoxShadow>,
}

impl ShadowTokens {
    pub fn elevations(color: Hsla) -> Self {
        Self {
            sm: vec![box_shadow(0., 1., 2., 0., color)],
            md: vec![box_shadow(0., 4., 8., -2., color)],
            lg: vec![box_shadow(0., 12., 24., -4., color)],
        }
    }
}

fn text_style(size: f32, line_height: f32) -> TextStyleToken {
    TextStyleToken {
        size: px(size),
        line_height: px(line_height),
        weight: FontWeight::NORMAL,
    }
}

fn default_mono_font_family() -> SharedString {
    if cfg!(target_os = "macos") {
        "Menlo".into()
    } else if cfg!(target_os = "windows") {
        "Consolas".into()
    } else {
        "DejaVu Sans Mono".into()
    }
}

fn box_shadow(x: f32, y: f32, blur: f32, spread: f32, color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(x), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    }
}
