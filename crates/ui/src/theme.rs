use std::ops::{Deref, DerefMut};

use gpui::{
    hsla, point, px, App, BoxShadow, Global, Hsla, Pixels, SharedString, Window, WindowAppearance,
};

use crate::{scroll::ScrollbarShow, Colorize as _};

pub fn init(cx: &mut App) {
    Theme::sync_system_appearance(None, cx)
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    #[inline]
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

/// Make a [gpui::Hsla] color.
///
/// - h: 0..360.0
/// - s: 0.0..100.0
/// - l: 0.0..100.0
#[inline]
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
#[inline]
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ThemeColor {
    /// Used for accents such as hover background on MenuItem, ListItem, etc.
    pub accent: Hsla,
    /// Used for accent text color.
    pub accent_foreground: Hsla,
    /// Accordion background color.
    pub accordion: Hsla,
    /// Accordion active background color.
    pub accordion_active: Hsla,
    /// Accordion hover background color.
    pub accordion_hover: Hsla,
    /// Default background color.
    pub background: Hsla,
    /// Default border color
    pub border: Hsla,
    /// Background color for Card.
    pub card: Hsla,
    /// Text color for Card.
    pub card_foreground: Hsla,
    /// Input caret color (Blinking cursor).
    pub caret: Hsla,
    /// Danger background color.
    pub danger: Hsla,
    /// Danger active background color.
    pub danger_active: Hsla,
    /// Danger text color.
    pub danger_foreground: Hsla,
    /// Danger hover background color.
    pub danger_hover: Hsla,
    /// Drag border color.
    pub drag_border: Hsla,
    /// Drop target background color.
    pub drop_target: Hsla,
    /// Default text color.
    pub foreground: Hsla,
    /// Border color for inputs such as Input, Dropdown, etc.
    pub input: Hsla,
    /// Link text color.
    pub link: Hsla,
    /// Active link text color.
    pub link_active: Hsla,
    /// Hover link text color.
    pub link_hover: Hsla,
    /// Background color for List and ListItem.
    pub list: Hsla,
    /// Background color for active ListItem.
    pub list_active: Hsla,
    /// Border color for active ListItem.
    pub list_active_border: Hsla,
    /// Stripe background color for even ListItem.
    pub list_even: Hsla,
    /// Background color for List header.
    pub list_head: Hsla,
    /// Hover background color for ListItem.
    pub list_hover: Hsla,
    /// Muted backgrounds such as Skeleton and Switch.
    pub muted: Hsla,
    /// Muted text color, as used in disabled text.
    pub muted_foreground: Hsla,
    /// Background color for Popover.
    pub popover: Hsla,
    /// Text color for Popover.
    pub popover_foreground: Hsla,
    /// Primary background color.
    pub primary: Hsla,
    /// Active primary background color.
    pub primary_active: Hsla,
    /// Primary text color.
    pub primary_foreground: Hsla,
    /// Hover primary background color.
    pub primary_hover: Hsla,
    /// Progress bar background color.
    pub progress_bar: Hsla,
    /// Used for focus ring.
    pub ring: Hsla,
    /// Scrollbar background color.
    pub scrollbar: Hsla,
    /// Scrollbar thumb background color.
    pub scrollbar_thumb: Hsla,
    /// Scrollbar thumb hover background color.
    pub scrollbar_thumb_hover: Hsla,
    /// Secondary background color.
    pub secondary: Hsla,
    /// Active secondary background color.
    pub secondary_active: Hsla,
    /// Secondary text color, used for secondary Button text color or secondary text.
    pub secondary_foreground: Hsla,
    /// Hover secondary background color.
    pub secondary_hover: Hsla,
    /// Input selection background color.
    pub selection: Hsla,
    /// Sidebar background color.
    pub sidebar: Hsla,
    /// Sidebar accent background color.
    pub sidebar_accent: Hsla,
    /// Sidebar accent text color.
    pub sidebar_accent_foreground: Hsla,
    /// Sidebar border color.
    pub sidebar_border: Hsla,
    /// Sidebar text color.
    pub sidebar_foreground: Hsla,
    /// Sidebar primary background color.
    pub sidebar_primary: Hsla,
    /// Sidebar primary text color.
    pub sidebar_primary_foreground: Hsla,
    /// Skeleton background color.
    pub skeleton: Hsla,
    /// Slider bar background color.
    pub slider_bar: Hsla,
    /// Slider thumb background color.
    pub slider_thumb: Hsla,
    /// Tab background color.
    pub tab: Hsla,
    /// Tab active background color.
    pub tab_active: Hsla,
    /// Tab active text color.
    pub tab_active_foreground: Hsla,
    /// TabBar background color.
    pub tab_bar: Hsla,
    /// Tab text color.
    pub tab_foreground: Hsla,
    /// Table background color.
    pub table: Hsla,
    /// Table active item background color.
    pub table_active: Hsla,
    /// Table active item border color.
    pub table_active_border: Hsla,
    /// Stripe background color for even TableRow.
    pub table_even: Hsla,
    /// Table head background color.
    pub table_head: Hsla,
    /// Table head text color.
    pub table_head_foreground: Hsla,
    /// Table item hover background color.
    pub table_hover: Hsla,
    /// Table row border color.
    pub table_row_border: Hsla,
    /// TitleBar background color, use for Window title bar.
    pub title_bar: Hsla,
    /// TitleBar border color.
    pub title_bar_border: Hsla,
    /// Window border color.
    ///
    /// # Platform specific:
    ///
    /// This is only works on Linux, other platforms we can't change the window border color.
    pub window_border: Hsla,
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
            card: hsl(0.0, 0.0, 100.0),
            card_foreground: hsl(240.0, 10.0, 3.9),
            caret: hsl(240.0, 10., 3.9),
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
            sidebar: hsl(0.0, 0.0, 98.0),
            sidebar_accent: hsl(240.0, 4.8, 92.),
            sidebar_accent_foreground: hsl(240.0, 5.9, 10.0),
            sidebar_border: hsl(220.0, 13.0, 91.0),
            sidebar_foreground: hsl(240.0, 5.3, 26.1),
            sidebar_primary: hsl(240.0, 5.9, 10.0),
            sidebar_primary_foreground: hsl(0.0, 0.0, 98.0),
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
            window_border: hsl(240.0, 5.9, 78.0),
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
            card: hsl(0.0, 0.0, 8.0),
            card_foreground: hsl(0.0, 0.0, 78.0),
            caret: hsl(0., 0., 78.),
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
            sidebar: hsl(240.0, 0.0, 10.0),
            sidebar_accent: hsl(240.0, 3.7, 15.9),
            sidebar_accent_foreground: hsl(240.0, 4.8, 95.9),
            sidebar_border: hsl(240.0, 3.7, 15.9),
            sidebar_foreground: hsl(240.0, 4.8, 95.9),
            sidebar_primary: hsl(0.0, 0.0, 98.0),
            sidebar_primary_foreground: hsl(240.0, 5.9, 10.0),
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
            window_border: hsl(240.0, 3.7, 28.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    colors: ThemeColor,

    pub mode: ThemeMode,
    pub font_family: SharedString,
    pub font_size: Pixels,
    pub radius: Pixels,
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
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Returns the global theme mutable reference
    pub fn global_mut(cx: &mut App) -> &mut Theme {
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
        self.caret = self.caret.apply(mask_color);
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
    pub fn sync_system_appearance(window: Option<&mut Window>, cx: &mut App) {
        match cx.window_appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                Self::change(ThemeMode::Dark, window, cx)
            }
            WindowAppearance::Light | WindowAppearance::VibrantLight => {
                Self::change(ThemeMode::Light, window, cx)
            }
        }
    }

    pub fn change(mode: ThemeMode, window: Option<&mut Window>, cx: &mut App) {
        let colors = match mode {
            ThemeMode::Light => ThemeColor::light(),
            ThemeMode::Dark => ThemeColor::dark(),
        };

        let mut theme = Theme::from(colors);
        theme.mode = mode;

        cx.set_global(theme);
        if let Some(window) = window {
            window.refresh();
        }
    }
}

impl From<ThemeColor> for Theme {
    fn from(colors: ThemeColor) -> Self {
        Theme {
            mode: ThemeMode::default(),
            transparent: Hsla::transparent_black(),
            font_size: px(16.),
            font_family: if cfg!(target_os = "macos") {
                ".SystemUIFont".into()
            } else if cfg!(target_os = "windows") {
                "Segoe UI".into()
            } else {
                "FreeMono".into()
            },
            radius: px(4.),
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
    #[inline]
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }
}
