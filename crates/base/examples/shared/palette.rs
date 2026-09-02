use std::cell::Cell;

use gpui::{App, Rgba, Window, WindowAppearance};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExamplePalette {
    pub canvas: u32,
    pub surface: u32,
    pub elevated: u32,
    pub foreground: u32,
    pub muted_foreground: u32,
    pub subtle_foreground: u32,
    pub border: u32,
    pub strong_border: u32,
    pub hover: u32,
    pub selected: u32,
    pub accent: u32,
    pub accent_foreground: u32,
}

impl ExamplePalette {
    pub fn from_window(window: &Window) -> Self {
        Self::for_dark(matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ))
    }

    pub const fn for_dark(dark: bool) -> Self {
        if dark {
            Self {
                canvas: 0x0e0e0e,
                surface: 0x171717,
                elevated: 0x262626,
                foreground: 0xffffff,
                muted_foreground: 0xa3a3a3,
                subtle_foreground: 0x737373,
                border: 0x404040,
                strong_border: 0xd4d4d4,
                hover: 0x262626,
                selected: 0x303030,
                accent: 0xffffff,
                accent_foreground: 0x171717,
            }
        } else {
            Self {
                canvas: 0xffffff,
                surface: 0xffffff,
                elevated: 0xf5f5f5,
                foreground: 0x171717,
                muted_foreground: 0x525252,
                subtle_foreground: 0x737373,
                border: 0xd4d4d4,
                strong_border: 0x171717,
                hover: 0xf5f5f5,
                selected: 0xf0f0f0,
                accent: 0x171717,
                accent_foreground: 0xffffff,
            }
        }
    }

    pub const fn resolve(self, light_color: u32) -> u32 {
        match light_color {
            0xffffff => self.surface,
            0x171717 => self.foreground,
            0x262626 => self.foreground,
            0x404040 => self.strong_border,
            0x525252 => self.muted_foreground,
            0x737373 => self.subtle_foreground,
            0xa3a3a3 => self.subtle_foreground,
            0x71717a => self.muted_foreground,
            0xd4d4d4 | 0xd4d4d8 | 0xe5e5e5 | 0xe5e7eb => self.border,
            0xf0f0f0 => self.selected,
            0xf4f4f5 | 0xf5f5f5 => self.hover,
            0x007fff if self.canvas == 0x0e0e0e => 0x79c0ff,
            0x036a07 if self.canvas == 0x0e0e0e => 0x7ee787,
            0x0433ff if self.canvas == 0x0e0e0e => 0x79c0ff,
            0xc5060b if self.canvas == 0x0e0e0e => 0xff7b72,
            0x0000a2 | 0x6f42c1 if self.canvas == 0x0e0e0e => 0xd2a8ff,
            0x333333 if self.canvas == 0x0e0e0e => 0xc9d1d9,
            color => color,
        }
    }
}

thread_local! {
    static ACTIVE: Cell<ExamplePalette> = const { Cell::new(ExamplePalette::for_dark(false)) };
}

pub fn activate(window: &Window, cx: &mut App) {
    let palette = ExamplePalette::from_window(window);
    ACTIVE.set(palette);
    apply_base_theme(palette, cx);
}

fn apply_base_theme(palette: ExamplePalette, cx: &mut App) {
    let dark = palette == ExamplePalette::for_dark(true);
    let theme = gpui_base::Theme::global_mut(cx);
    theme.appearance = if dark {
        gpui_base::ThemeAppearance::Dark
    } else {
        gpui_base::ThemeAppearance::Light
    };

    let colors = &mut theme.tokens.colors;
    colors.background = gpui::rgb(palette.canvas).into();
    colors.foreground = gpui::rgb(palette.foreground).into();
    colors.surface = gpui::rgb(palette.surface).into();
    colors.surface_foreground = gpui::rgb(palette.foreground).into();
    colors.primary = gpui::rgb(palette.accent).into();
    colors.primary_foreground = gpui::rgb(palette.accent_foreground).into();
    colors.secondary = gpui::rgb(palette.elevated).into();
    colors.secondary_foreground = gpui::rgb(palette.foreground).into();
    colors.muted = gpui::rgb(palette.elevated).into();
    colors.muted_foreground = gpui::rgb(palette.muted_foreground).into();
    colors.accent = gpui::rgb(palette.hover).into();
    colors.accent_foreground = gpui::rgb(palette.foreground).into();
    colors.border = gpui::rgb(palette.border).into();
    colors.input = gpui::rgb(palette.border).into();
    colors.ring = gpui::rgb(palette.accent).into();
}

pub fn example_rgb(color: u32) -> Rgba {
    gpui::rgb(ACTIVE.get().resolve(color))
}

pub fn canvas() -> Rgba {
    gpui::rgb(ACTIVE.get().canvas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_palettes_keep_semantic_contrast() {
        let light = ExamplePalette::for_dark(false);
        let dark = ExamplePalette::for_dark(true);

        assert_eq!(light.canvas, 0xffffff);
        assert_eq!(light.foreground, 0x171717);
        assert_eq!(dark.canvas, 0x0e0e0e);
        assert_eq!(dark.foreground, 0xffffff);
        assert_ne!(light.surface, dark.surface);
        assert_ne!(light.border, dark.border);
        assert_eq!(light.resolve(0xffffff), 0xffffff);
        assert_eq!(dark.resolve(0xffffff), 0x171717);
        assert_eq!(dark.resolve(0x171717), 0xffffff);
        assert_eq!(dark.resolve(0x2563eb), 0x2563eb);
    }

    #[gpui::test]
    fn dark_palette_projects_dark_base_theme(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| apply_base_theme(ExamplePalette::for_dark(true), cx));

        cx.update(|cx| {
            let theme = gpui_base::Theme::global(cx);
            assert_eq!(theme.appearance, gpui_base::ThemeAppearance::Dark);
            assert_eq!(theme.tokens.colors.foreground, gpui::rgb(0xffffff).into());
            assert_eq!(theme.tokens.colors.border, gpui::rgb(0x404040).into());
        });
    }
}
