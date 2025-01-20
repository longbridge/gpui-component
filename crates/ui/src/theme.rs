use std::ops::{Deref, DerefMut};

use gpui::{
    hsla, point, AppContext, BoxShadow, Global, Hsla, ModelContext, Pixels, SharedString,
    ViewContext, WindowAppearance, WindowContext,
};

use crate::scroll::ScrollbarShow;

pub fn init(cx: &mut AppContext) {
    Theme::sync_system_appearance(cx)
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for AppContext {
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

impl<V> ActiveTheme for ViewContext<'_, V> {
    fn theme(&self) -> &Theme {
        self.deref().theme()
    }
}

impl<V> ActiveTheme for ModelContext<'_, V> {
    fn theme(&self) -> &Theme {
        self.deref().theme()
    }
}

impl ActiveTheme for WindowContext<'_> {
    fn theme(&self) -> &Theme {
        self.deref().theme()
    }
}

/// Make a [gpui::Hsla] color.
///
/// - h: 0..360.0
/// - s: 0.0..100.0
/// - l: 0.0..100.0
pub fn hsl(h: f32, s: f32, l: f32) -> Hsla {
    hsla(h / 360., s / 100.0, l / 100.0, 1.0)
}

/// Make a BoxShadow like CSS
///
/// e.g:
///
/// If CSS is `box-shadow: 0 0 10px 0 rgba(0, 0, 0, 0.1);`
///
/// Then the equivalent in Rust is `box_shadow(0., 0., 10., 0., hsla(0., 0., 0., 0.1))`
pub fn box_shadow(
    x: impl Into<Pixels>,
    y: impl Into<Pixels>,
    blur: impl Into<Pixels>,
    spread: impl Into<Pixels>,
    color: Hsla,
) -> BoxShadow {
    BoxShadow {
        offset: point(x.into(), y.into()),
        blur_radius: blur.into(),
        spread_radius: spread.into(),
        color,
    }
}
pub trait Colorize {
    /// Returns a new color with the given opacity.
    ///
    /// The opacity is a value between 0.0 and 1.0, where 0.0 is fully transparent and 1.0 is fully opaque.
    fn opacity(&self, opacity: f32) -> Hsla;
    /// Returns a new color with each channel divided by the given divisor.
    ///
    /// The divisor in range of 0.0 .. 1.0
    fn divide(&self, divisor: f32) -> Hsla;
    /// Return inverted color
    fn invert(&self) -> Hsla;
    /// Return inverted lightness
    fn invert_l(&self) -> Hsla;
    /// Return a new color with the lightness increased by the given factor.
    ///
    /// factor range: 0.0 .. 1.0
    fn lighten(&self, amount: f32) -> Hsla;
    /// Return a new color with the darkness increased by the given factor.
    ///
    /// factor range: 0.0 .. 1.0
    fn darken(&self, amount: f32) -> Hsla;
    /// Return a new color with the same lightness and alpha but different hue and saturation.
    fn apply(&self, base_color: Hsla) -> Hsla;
}

impl Colorize for Hsla {
    fn opacity(&self, factor: f32) -> Hsla {
        Hsla {
            a: self.a * factor.clamp(0.0, 1.0),
            ..*self
        }
    }

    fn divide(&self, divisor: f32) -> Hsla {
        Hsla {
            a: divisor,
            ..*self
        }
    }

    fn invert(&self) -> Hsla {
        Hsla {
            h: 1.0 - self.h,
            s: 1.0 - self.s,
            l: 1.0 - self.l,
            a: self.a,
        }
    }

    fn invert_l(&self) -> Hsla {
        Hsla {
            l: 1.0 - self.l,
            ..*self
        }
    }

    fn lighten(&self, factor: f32) -> Hsla {
        let l = self.l * (1.0 + factor.clamp(0.0, 1.0));

        Hsla { l, ..*self }
    }

    fn darken(&self, factor: f32) -> Hsla {
        let l = self.l * (1.0 - factor.clamp(0.0, 1.0));

        Hsla { l, ..*self }
    }

    fn apply(&self, new_color: Hsla) -> Hsla {
        Hsla {
            h: new_color.h,
            s: new_color.s,
            l: self.l,
            a: self.a,
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ThemeColor {
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub accordion: Hsla,
    pub accordion_active: Hsla,
    pub accordion_hover: Hsla,
    pub background: Hsla,
    pub border: Hsla,
    pub window_border: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    pub danger: Hsla,
    pub danger_active: Hsla,
    pub danger_foreground: Hsla,
    pub danger_hover: Hsla,
    pub drag_border: Hsla,
    pub drop_target: Hsla,
    pub foreground: Hsla,
    pub input: Hsla,
    pub link: Hsla,
    pub link_active: Hsla,
    pub link_hover: Hsla,
    pub list: Hsla,
    pub list_active: Hsla,
    pub list_active_border: Hsla,
    pub list_even: Hsla,
    pub list_head: Hsla,
    pub list_hover: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub panel: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub primary: Hsla,
    pub primary_active: Hsla,
    pub primary_foreground: Hsla,
    pub primary_hover: Hsla,
    pub progress_bar: Hsla,
    pub ring: Hsla,
    pub scrollbar: Hsla,
    pub scrollbar_thumb: Hsla,
    pub scrollbar_thumb_hover: Hsla,
    pub secondary: Hsla,
    pub secondary_active: Hsla,
    pub secondary_foreground: Hsla,
    pub secondary_hover: Hsla,
    pub selection: Hsla,
    pub skeleton: Hsla,
    pub slider_bar: Hsla,
    pub slider_thumb: Hsla,
    pub tab: Hsla,
    pub tab_active: Hsla,
    pub tab_active_foreground: Hsla,
    pub tab_bar: Hsla,
    pub tab_foreground: Hsla,
    pub table: Hsla,
    pub table_active: Hsla,
    pub table_active_border: Hsla,
    pub table_even: Hsla,
    pub table_head: Hsla,
    pub table_head_foreground: Hsla,
    pub table_hover: Hsla,
    pub table_row_border: Hsla,
    pub title_bar: Hsla,
    pub title_bar_border: Hsla,
    pub sidebar: Hsla,
    pub sidebar_accent: Hsla,
    pub sidebar_accent_foreground: Hsla,
    pub sidebar_border: Hsla,
    pub sidebar_foreground: Hsla,
    pub sidebar_primary: Hsla,
    pub sidebar_primary_foreground: Hsla,
}

impl ThemeColor {
    pub fn light() -> Self {
        Self {
            accent: hsl(240.0, 5.0, 96.0),
            accent_foreground: hsl(240.0, 5.9, 10.0),
            accordion: hsl(0.0, 0.0, 100.0),
            accordion_active: hsl(240.0, 5.9, 90.0),
            accordion_hover: hsl(240.0, 4.8, 95.9).opacity(0.7),
            background: hsl(0.0, 0.0, 100.),
            border: hsl(240.0, 5.9, 90.0),
            window_border: hsl(240.0, 5.9, 78.0),
            card: hsl(0.0, 0.0, 100.0),
            card_foreground: hsl(240.0, 10.0, 3.9),
            danger: hsl(0.0, 84.2, 60.2),
            danger_active: hsl(0.0, 84.2, 47.0),
            danger_foreground: hsl(0.0, 0.0, 98.0),
            danger_hover: hsl(0.0, 84.2, 65.0),
            drag_border: crate::blue_500(),
            drop_target: hsl(235.0, 30., 44.0).opacity(0.25),
            foreground: hsl(240.0, 10., 3.9),
            input: hsl(240.0, 5.9, 90.0),
            link: hsl(221.0, 83.0, 53.0),
            link_active: hsl(221.0, 83.0, 53.0).darken(0.2),
            link_hover: hsl(221.0, 83.0, 53.0).lighten(0.2),
            list: hsl(0.0, 0.0, 100.),
            list_active: hsl(211.0, 97.0, 85.0).opacity(0.2),
            list_active_border: hsl(211.0, 97.0, 85.0),
            list_even: hsl(240.0, 5.0, 96.0),
            list_head: hsl(0.0, 0.0, 100.),
            list_hover: hsl(240.0, 4.8, 95.0),
            muted: hsl(240.0, 4.8, 95.9),
            muted_foreground: hsl(240.0, 3.8, 46.1),
            panel: hsl(0.0, 0.0, 100.0),
            popover: hsl(0.0, 0.0, 100.0),
            popover_foreground: hsl(240.0, 10.0, 3.9),
            primary: hsl(223.0, 5.9, 10.0),
            primary_active: hsl(223.0, 1.9, 25.0),
            primary_foreground: hsl(223.0, 0.0, 98.0),
            primary_hover: hsl(223.0, 5.9, 15.0),
            progress_bar: hsl(223.0, 5.9, 10.0),
            ring: hsl(240.0, 5.9, 65.0),
            scrollbar: hsl(0., 0., 97.).opacity(0.75),
            scrollbar_thumb: hsl(0., 0., 69.).opacity(0.9),
            scrollbar_thumb_hover: hsl(0., 0., 59.),
            secondary: hsl(240.0, 5.9, 96.9),
            secondary_active: hsl(240.0, 5.9, 90.),
            secondary_foreground: hsl(240.0, 59.0, 10.),
            secondary_hover: hsl(240.0, 5.9, 98.),
            selection: hsl(211.0, 97.0, 85.0),
            skeleton: hsl(223.0, 5.9, 10.0).opacity(0.1),
            slider_bar: hsl(223.0, 5.9, 10.0),
            slider_thumb: hsl(0.0, 0.0, 100.0),
            tab: gpui::transparent_black(),
            tab_active: hsl(0.0, 0.0, 100.0),
            tab_active_foreground: hsl(240.0, 10., 3.9),
            tab_bar: hsl(240.0, 4.8, 95.9),
            tab_foreground: hsl(240.0, 10., 3.9),
            table: hsl(0.0, 0.0, 100.),
            table_active: hsl(211.0, 97.0, 85.0).opacity(0.2),
            table_active_border: hsl(211.0, 97.0, 85.0),
            table_even: hsl(240.0, 5.0, 96.0),
            table_head: hsl(0.0, 0.0, 100.),
            table_head_foreground: hsl(240.0, 10., 3.9).opacity(0.7),
            table_hover: hsl(240.0, 4.8, 95.0),
            table_row_border: hsl(240.0, 7.7, 94.5),
            title_bar: hsl(0.0, 0.0, 100.),
            title_bar_border: hsl(240.0, 5.9, 90.0),
            sidebar: hsl(0.0, 0.0, 98.0),
            sidebar_accent: hsl(240.0, 4.8, 92.),
            sidebar_accent_foreground: hsl(240.0, 5.9, 10.0),
            sidebar_border: hsl(220.0, 13.0, 91.0),
            sidebar_foreground: hsl(240.0, 5.3, 26.1),
            sidebar_primary: hsl(240.0, 5.9, 10.0),
            sidebar_primary_foreground: hsl(0.0, 0.0, 98.0),
        }
    }

    pub fn dark() -> Self {
        Self {
            accent: hsl(240.0, 3.7, 15.9),
            accent_foreground: hsl(0.0, 0.0, 78.0),
            accordion: hsl(299.0, 2., 11.),
            accordion_active: hsl(240.0, 3.7, 16.9),
            accordion_hover: hsl(240.0, 3.7, 15.9).opacity(0.7),
            background: hsl(0.0, 0.0, 8.0),
            border: hsl(240.0, 3.7, 16.9),
            window_border: hsl(240.0, 3.7, 28.0),
            card: hsl(0.0, 0.0, 8.0),
            card_foreground: hsl(0.0, 0.0, 78.0),
            danger: hsl(0.0, 62.8, 30.6),
            danger_active: hsl(0.0, 62.8, 20.6),
            danger_foreground: hsl(0.0, 0.0, 78.0),
            danger_hover: hsl(0.0, 62.8, 35.6),
            drag_border: crate::blue_500(),
            drop_target: hsl(235.0, 30., 44.0).opacity(0.1),
            foreground: hsl(0., 0., 78.),
            input: hsl(240.0, 3.7, 15.9),
            link: hsl(221.0, 83.0, 53.0),
            link_active: hsl(221.0, 83.0, 53.0).darken(0.2),
            link_hover: hsl(221.0, 83.0, 53.0).lighten(0.2),
            list: hsl(0.0, 0.0, 8.0),
            list_active: hsl(240.0, 3.7, 15.0).opacity(0.2),
            list_active_border: hsl(240.0, 5.9, 35.5),
            list_even: hsl(240.0, 3.7, 10.0),
            list_head: hsl(0.0, 0.0, 8.0),
            list_hover: hsl(240.0, 3.7, 15.9),
            muted: hsl(240.0, 3.7, 15.9),
            muted_foreground: hsl(240.0, 5.0, 64.9),
            panel: hsl(299.0, 2., 11.),
            popover: hsl(0.0, 0.0, 10.),
            popover_foreground: hsl(0.0, 0.0, 78.0),
            primary: hsl(223.0, 0.0, 98.0),
            primary_active: hsl(223.0, 0.0, 80.0),
            primary_foreground: hsl(223.0, 5.9, 10.0),
            primary_hover: hsl(223.0, 0.0, 90.0),
            progress_bar: hsl(223.0, 0.0, 98.0),
            ring: hsl(240.0, 4.9, 83.9),
            scrollbar: hsl(240., 1., 15.).opacity(0.75),
            scrollbar_thumb: hsl(0., 0., 48.).opacity(0.9),
            scrollbar_thumb_hover: hsl(0., 0., 68.),
            secondary: hsl(240.0, 0., 13.0),
            secondary_active: hsl(240.0, 0., 10.),
            secondary_foreground: hsl(0.0, 0.0, 78.0),
            secondary_hover: hsl(240.0, 0., 15.),
            selection: hsl(211.0, 97.0, 22.0),
            skeleton: hsla(223.0, 0.0, 98.0, 0.1),
            slider_bar: hsl(223.0, 0.0, 98.0),
            slider_thumb: hsl(0.0, 0.0, 8.0),
            tab: gpui::transparent_black(),
            tab_active: hsl(0.0, 0.0, 8.0),
            tab_active_foreground: hsl(0., 0., 78.),
            tab_bar: hsl(299.0, 0., 5.5),
            tab_foreground: hsl(0., 0., 78.),
            table: hsl(0.0, 0.0, 8.0),
            table_active: hsl(240.0, 3.7, 15.0).opacity(0.2),
            table_active_border: hsl(240.0, 5.9, 35.5),
            table_even: hsl(240.0, 3.7, 10.0),
            table_head: hsl(0.0, 0.0, 8.0),
            table_head_foreground: hsl(0., 0., 78.).opacity(0.7),
            table_hover: hsl(240.0, 3.7, 15.9).opacity(0.5),
            table_row_border: hsl(240.0, 3.7, 16.9).opacity(0.5),
            title_bar: hsl(0., 0., 9.7),
            title_bar_border: hsl(240.0, 3.7, 15.9),
            sidebar: hsl(240.0, 0.0, 10.0),
            sidebar_accent: hsl(240.0, 3.7, 15.9),
            sidebar_accent_foreground: hsl(240.0, 4.8, 95.9),
            sidebar_border: hsl(240.0, 3.7, 15.9),
            sidebar_foreground: hsl(240.0, 4.8, 95.9),
            sidebar_primary: hsl(0.0, 0.0, 98.0),
            sidebar_primary_foreground: hsl(240.0, 5.9, 10.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    colors: ThemeColor,

    pub mode: ThemeMode,
    pub font_family: SharedString,
    pub font_size: f32,
    pub radius: f32,
    pub shadow: bool,
    pub transparent: Hsla,
    /// Show the scrollbar mode, default: Scrolling
    pub scrollbar_show: ScrollbarShow,
}

impl Deref for Theme {
    type Target = ThemeColor;

    fn deref(&self) -> &Self::Target {
        &self.colors
    }
}

impl DerefMut for Theme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.colors
    }
}

impl Global for Theme {}

impl Theme {
    /// Returns the global theme reference
    pub fn global(cx: &AppContext) -> &Theme {
        cx.global::<Theme>()
    }

    /// Returns the global theme mutable reference
    pub fn global_mut(cx: &mut AppContext) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    /// Apply a mask color to the theme.
    pub fn apply_color(&mut self, mask_color: Hsla) {
        self.title_bar = self.title_bar.apply(mask_color);
        self.title_bar_border = self.title_bar_border.apply(mask_color);
        self.background = self.background.apply(mask_color);
        self.foreground = self.foreground.apply(mask_color);
        self.card = self.card.apply(mask_color);
        self.card_foreground = self.card_foreground.apply(mask_color);
        self.popover = self.popover.apply(mask_color);
        self.popover_foreground = self.popover_foreground.apply(mask_color);
        self.primary = self.primary.apply(mask_color);
        self.primary_hover = self.primary_hover.apply(mask_color);
        self.primary_active = self.primary_active.apply(mask_color);
        self.primary_foreground = self.primary_foreground.apply(mask_color);
        self.secondary = self.secondary.apply(mask_color);
        self.secondary_hover = self.secondary_hover.apply(mask_color);
        self.secondary_active = self.secondary_active.apply(mask_color);
        self.secondary_foreground = self.secondary_foreground.apply(mask_color);
        // self.danger = self.danger.apply(mask_color);
        // self.danger_hover = self.danger_hover.apply(mask_color);
        // self.danger_active = self.danger_active.apply(mask_color);
        // self.danger_foreground = self.danger_foreground.apply(mask_color);
        self.muted = self.muted.apply(mask_color);
        self.muted_foreground = self.muted_foreground.apply(mask_color);
        self.accent = self.accent.apply(mask_color);
        self.accent_foreground = self.accent_foreground.apply(mask_color);
        self.border = self.border.apply(mask_color);
        self.input = self.input.apply(mask_color);
        self.ring = self.ring.apply(mask_color);
        // self.selection = self.selection.apply(mask_color);
        self.scrollbar = self.scrollbar.apply(mask_color);
        self.scrollbar_thumb = self.scrollbar_thumb.apply(mask_color);
        self.scrollbar_thumb_hover = self.scrollbar_thumb_hover.apply(mask_color);
        self.panel = self.panel.apply(mask_color);
        self.drag_border = self.drag_border.apply(mask_color);
        self.drop_target = self.drop_target.apply(mask_color);
        self.tab_bar = self.tab_bar.apply(mask_color);
        self.tab = self.tab.apply(mask_color);
        self.tab_active = self.tab_active.apply(mask_color);
        self.tab_foreground = self.tab_foreground.apply(mask_color);
        self.tab_active_foreground = self.tab_active_foreground.apply(mask_color);
        self.progress_bar = self.progress_bar.apply(mask_color);
        self.slider_bar = self.slider_bar.apply(mask_color);
        self.slider_thumb = self.slider_thumb.apply(mask_color);
        self.list = self.list.apply(mask_color);
        self.list_even = self.list_even.apply(mask_color);
        self.list_head = self.list_head.apply(mask_color);
        self.list_active = self.list_active.apply(mask_color);
        self.list_active_border = self.list_active_border.apply(mask_color);
        self.list_hover = self.list_hover.apply(mask_color);
        self.table = self.table.apply(mask_color);
        self.table_even = self.table_even.apply(mask_color);
        self.table_active = self.table_active.apply(mask_color);
        self.table_active_border = self.table_active_border.apply(mask_color);
        self.table_hover = self.table_hover.apply(mask_color);
        self.table_row_border = self.table_row_border.apply(mask_color);
        self.table_head = self.table_head.apply(mask_color);
        self.table_head_foreground = self.table_head_foreground.apply(mask_color);
        self.link = self.link.apply(mask_color);
        self.link_hover = self.link_hover.apply(mask_color);
        self.link_active = self.link_active.apply(mask_color);
        self.skeleton = self.skeleton.apply(mask_color);
        self.accordion = self.accordion.apply(mask_color);
        self.accordion_hover = self.accordion_hover.apply(mask_color);
        self.accordion_active = self.accordion_active.apply(mask_color);
        self.title_bar = self.title_bar.apply(mask_color);
        self.title_bar_border = self.title_bar_border.apply(mask_color);
        self.sidebar = self.sidebar.apply(mask_color);
        self.sidebar_accent = self.sidebar_accent.apply(mask_color);
        self.sidebar_accent_foreground = self.sidebar_accent_foreground.apply(mask_color);
        self.sidebar_border = self.sidebar_border.apply(mask_color);
        self.sidebar_foreground = self.sidebar_foreground.apply(mask_color);
        self.sidebar_primary = self.sidebar_primary.apply(mask_color);
        self.sidebar_primary_foreground = self.sidebar_primary_foreground.apply(mask_color);
    }

    /// Sync the theme with the system appearance
    pub fn sync_system_appearance(cx: &mut AppContext) {
        match cx.window_appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                Self::change(ThemeMode::Dark, cx)
            }
            WindowAppearance::Light | WindowAppearance::VibrantLight => {
                Self::change(ThemeMode::Light, cx)
            }
        }
    }

    pub fn change(mode: ThemeMode, cx: &mut AppContext) {
        let colors = match mode {
            ThemeMode::Light => ThemeColor::light(),
            ThemeMode::Dark => ThemeColor::dark(),
        };

        let mut theme = Theme::from(colors);
        theme.mode = mode;

        cx.set_global(theme);
        cx.refresh();
    }
}

impl From<ThemeColor> for Theme {
    fn from(colors: ThemeColor) -> Self {
        Theme {
            mode: ThemeMode::default(),
            transparent: Hsla::transparent_black(),
            font_size: 16.0,
            font_family: if cfg!(target_os = "macos") {
                ".SystemUIFont".into()
            } else if cfg!(target_os = "windows") {
                "Segoe UI".into()
            } else {
                "FreeMono".into()
            },
            radius: 4.0,
            shadow: true,
            scrollbar_show: ScrollbarShow::default(),
            colors,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }
}

#[cfg(test)]
mod tests {
    use crate::theme::Colorize as _;

    #[test]
    fn test_lighten() {
        let color = super::hsl(240.0, 5.0, 30.0);
        let color = color.lighten(0.5);
        assert_eq!(color.l, 0.45000002);
        let color = color.lighten(0.5);
        assert_eq!(color.l, 0.675);
        let color = color.lighten(0.1);
        assert_eq!(color.l, 0.7425);
    }

    #[test]
    fn test_darken() {
        let color = super::hsl(240.0, 5.0, 96.0);
        let color = color.darken(0.5);
        assert_eq!(color.l, 0.48);
        let color = color.darken(0.5);
        assert_eq!(color.l, 0.24);
    }
}
