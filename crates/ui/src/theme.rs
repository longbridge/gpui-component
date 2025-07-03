use gpui::{
    hsla, point, px, transparent_black, App, BoxShadow, Global, Hsla, Pixels, SharedString, Window,
    WindowAppearance,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use crate::{colors::*, scroll::ScrollbarShow};

pub fn init(cx: &mut App) {
    Theme::sync_system_appearance(None, cx);
    Theme::sync_scrollbar_appearance(cx);
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
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
    /// Chart 1 color.
    pub chart_1: Hsla,
    /// Chart 2 color.
    pub chart_2: Hsla,
    /// Chart 3 color.
    pub chart_3: Hsla,
    /// Chart 4 color.
    pub chart_4: Hsla,
    /// Chart 5 color.
    pub chart_5: Hsla,
    /// Danger background color.
    pub danger: Hsla,
    /// Danger active background color.
    pub danger_active: Hsla,
    /// Danger text color.
    pub danger_foreground: Hsla,
    /// Danger hover background color.
    pub danger_hover: Hsla,
    /// Description List label background color.
    pub description_list_label: Hsla,
    /// Description List label foreground color.
    pub description_list_label_foreground: Hsla,
    /// Drag border color.
    pub drag_border: Hsla,
    /// Drop target background color.
    pub drop_target: Hsla,
    /// Default text color.
    pub foreground: Hsla,
    /// Info background color.
    pub info: Hsla,
    /// Info active background color.
    pub info_active: Hsla,
    /// Info text color.
    pub info_foreground: Hsla,
    /// Info hover background color.
    pub info_hover: Hsla,
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
    /// Success background color.
    pub success: Hsla,
    /// Success text color.
    pub success_foreground: Hsla,
    /// Success hover background color.
    pub success_hover: Hsla,
    /// Success active background color.
    pub success_active: Hsla,
    /// Switch background color.
    pub switch: Hsla,
    /// Tab background color.
    pub tab: Hsla,
    /// Tab active background color.
    pub tab_active: Hsla,
    /// Tab active text color.
    pub tab_active_foreground: Hsla,
    /// TabBar background color.
    pub tab_bar: Hsla,
    /// TabBar segmented background color.
    pub tab_bar_segmented: Hsla,
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
    /// Background color for Tiles.
    pub tiles: Hsla,
    /// Warning background color.
    pub warning: Hsla,
    /// Warning active background color.
    pub warning_active: Hsla,
    /// Warning hover background color.
    pub warning_hover: Hsla,
    /// Warning foreground color.
    pub warning_foreground: Hsla,
    /// Overlay background color.
    pub overlay: Hsla,
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
            accent: neutral_100(),
            accent_foreground: neutral_900(),
            accordion: white(),
            accordion_active: neutral_200(),
            accordion_hover: neutral_100(),
            background: white(),
            border: neutral_200(),
            card: white(),
            card_foreground: neutral_950(),
            caret: neutral_950(),
            chart_1: blue_300(),
            chart_2: blue_500(),
            chart_3: blue_600(),
            chart_4: blue_700(),
            chart_5: blue_800(),
            danger: red_500(),
            danger_active: red_600(),
            danger_foreground: red_50(),
            danger_hover: red_500().opacity(0.9),
            description_list_label: neutral_100(),
            description_list_label_foreground: neutral_900(),
            drag_border: blue_500(),
            drop_target: blue_500().opacity(0.25),
            foreground: neutral_950(),
            info: sky_500(),
            info_active: sky_600(),
            info_hover: sky_500().opacity(0.9),
            info_foreground: sky_50(),
            input: neutral_200(),
            link: neutral_950(),
            link_active: neutral_950(),
            link_hover: neutral_800(),
            list: white(),
            list_active: blue_200().opacity(0.2),
            list_active_border: blue_400(),
            list_even: neutral_50(),
            list_head: neutral_50(),
            list_hover: neutral_100(),
            muted: neutral_100(),
            muted_foreground: neutral_500(),
            popover: white(),
            popover_foreground: neutral_950(),
            primary: neutral_900(),
            primary_active: neutral_950(),
            primary_foreground: neutral_50(),
            primary_hover: neutral_800(),
            progress_bar: neutral_900(),
            ring: neutral_950(),
            scrollbar: neutral_50().opacity(0.5),
            scrollbar_thumb: neutral_400().opacity(0.9),
            scrollbar_thumb_hover: neutral_400(),
            secondary: neutral_100().darken(0.05),
            secondary_active: neutral_200(),
            secondary_foreground: neutral_900(),
            secondary_hover: neutral_100().darken(0.05).opacity(0.5),
            selection: blue_200(),
            sidebar: neutral_50(),
            sidebar_accent: neutral_200(),
            sidebar_accent_foreground: neutral_900(),
            sidebar_border: neutral_200(),
            sidebar_foreground: neutral_900(),
            sidebar_primary: neutral_900(),
            sidebar_primary_foreground: neutral_50(),
            skeleton: neutral_50().opacity(0.1),
            slider_bar: neutral_900(),
            slider_thumb: white(),
            success: green_500(),
            success_active: green_600(),
            success_hover: green_500().opacity(0.9),
            success_foreground: gray_50(),
            switch: neutral_300(),
            tab: gpui::transparent_black(),
            tab_active: white(),
            tab_active_foreground: neutral_900(),
            tab_bar: neutral_100(),
            tab_bar_segmented: neutral_100(),
            tab_foreground: neutral_700(),
            table: white(),
            table_active: blue_200().opacity(0.2),
            table_active_border: blue_400(),
            table_even: neutral_50(),
            table_head: neutral_50(),
            table_head_foreground: neutral_500(),
            table_hover: neutral_100(),
            table_row_border: neutral_200().opacity(0.7),
            tiles: neutral_50(),
            title_bar: white(),
            title_bar_border: neutral_200(),
            warning: yellow_500(),
            warning_active: yellow_600(),
            warning_hover: yellow_500().opacity(0.9),
            warning_foreground: gray_50(),
            overlay: black().opacity(0.05),
            window_border: neutral_200(),
        }
    }

    pub fn dark() -> Self {
        Self {
            accent: neutral_900(),
            accent_foreground: neutral_50(),
            accordion: neutral_950(),
            accordion_active: neutral_800(),
            accordion_hover: neutral_800().opacity(0.9),
            background: neutral_950(),
            border: neutral_800(),
            card: neutral_950(),
            card_foreground: neutral_50(),
            caret: neutral_50(),
            chart_1: blue_300(),
            chart_2: blue_500(),
            chart_3: blue_600(),
            chart_4: blue_700(),
            chart_5: blue_800(),
            danger: red_900(),
            danger_active: red_900().darken(0.2),
            danger_foreground: red_50(),
            danger_hover: red_900().lighten(0.1),
            description_list_label: neutral_900(),
            description_list_label_foreground: neutral_100(),
            drag_border: blue_500(),
            drop_target: blue_500().opacity(0.1),
            foreground: neutral_50(),
            info: sky_900(),
            info_active: sky_900().darken(0.2),
            info_foreground: sky_50(),
            info_hover: sky_900().lighten(0.1),
            input: neutral_800(),
            link: neutral_50(),
            link_active: neutral_50().darken(0.2),
            link_hover: neutral_50().lighten(0.2),
            list: neutral_950(),
            list_active: blue_800().opacity(0.2),
            list_active_border: blue_700(),
            list_even: neutral_900().opacity(0.8),
            list_head: neutral_900().opacity(0.8),
            list_hover: neutral_800(),
            muted: neutral_800(),
            muted_foreground: neutral_500(),
            popover: neutral_950(),
            popover_foreground: neutral_50(),
            primary: neutral_50(),
            primary_active: neutral_300(),
            primary_foreground: neutral_900(),
            primary_hover: neutral_200(),
            progress_bar: neutral_100(),
            ring: neutral_300(),
            scrollbar: neutral_900().opacity(0.5),
            scrollbar_thumb: neutral_700().opacity(0.9),
            scrollbar_thumb_hover: neutral_700(),
            secondary: neutral_800(),
            secondary_active: neutral_800().darken(0.2),
            secondary_foreground: neutral_50(),
            secondary_hover: neutral_800().lighten(0.1),
            selection: blue_700(),
            sidebar: neutral_950(),
            sidebar_accent: neutral_800(),
            sidebar_accent_foreground: neutral_100(),
            sidebar_border: neutral_800(),
            sidebar_foreground: neutral_100(),
            sidebar_primary: neutral_100(),
            sidebar_primary_foreground: neutral_950(),
            skeleton: neutral_900(),
            slider_bar: neutral_50(),
            slider_thumb: neutral_950(),
            success: green_900(),
            success_active: green_900().darken(0.2),
            success_foreground: green_50(),
            success_hover: green_900().lighten(0.1),
            switch: neutral_700(),
            tab: transparent_black(),
            tab_active: neutral_950(),
            tab_active_foreground: neutral_50(),
            tab_bar: neutral_900(),
            tab_bar_segmented: neutral_900(),
            tab_foreground: neutral_300(),
            table: neutral_950(),
            table_active: blue_800().opacity(0.2),
            table_active_border: blue_700(),
            table_even: neutral_900().opacity(0.8),
            table_head: neutral_900().opacity(0.8),
            table_head_foreground: neutral_600(),
            table_hover: neutral_800(),
            table_row_border: neutral_800().opacity(0.7),
            tiles: neutral_900(),
            title_bar: neutral_950(),
            title_bar_border: neutral_800(),
            warning: yellow_900(),
            warning_active: yellow_900().darken(0.2),
            warning_foreground: yellow_50(),
            warning_hover: yellow_900().lighten(0.1),
            overlay: white().opacity(0.03),
            window_border: neutral_800(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub colors: ThemeColor,

    pub mode: ThemeMode,
    pub font_family: SharedString,
    pub font_size: Pixels,
    /// Radius for the general elements.
    pub radius: Pixels,
    /// Radius for the large elements, e.g.: Modal, Notification border radius.
    pub radius_lg: Pixels,
    pub shadow: bool,
    pub transparent: Hsla,
    /// Show the scrollbar mode, default: Scrolling
    pub scrollbar_show: ScrollbarShow,
    /// Tile grid size, default is 4px.
    pub tile_grid_size: Pixels,
    /// The shadow of the tile panel.
    pub tile_shadow: bool,
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
    #[inline(always)]
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Returns the global theme mutable reference
    #[inline(always)]
    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    /// Returns true if the theme is dark.
    #[inline(always)]
    pub fn is_dark(&self) -> bool {
        self.mode.is_dark()
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
        self.tab_bar_segmented = self.tab_bar_segmented.apply(mask_color);
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
        self.tiles = self.tiles.apply(mask_color);
        self.description_list_label = self.description_list_label.apply(mask_color);
        self.description_list_label_foreground =
            self.description_list_label_foreground.apply(mask_color);
    }

    /// Sync the theme with the system appearance
    pub fn sync_system_appearance(window: Option<&mut Window>, cx: &mut App) {
        // Better use window.appearance() for avoid error on Linux.
        // https://github.com/longbridge/gpui-component/issues/104
        let appearance = window
            .as_ref()
            .map(|window| window.appearance())
            .unwrap_or_else(|| cx.window_appearance());

        Self::change(appearance, window, cx);
    }

    /// Sync the Scrollbar showing behavior with the system
    pub fn sync_scrollbar_appearance(cx: &mut App) {
        if cx.should_auto_hide_scrollbars() {
            cx.global_mut::<Theme>().scrollbar_show = ScrollbarShow::Scrolling;
        } else {
            cx.global_mut::<Theme>().scrollbar_show = ScrollbarShow::Hover;
        }
    }

    pub fn change(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App) {
        let mode = mode.into();
        let colors = match mode {
            ThemeMode::Light => ThemeColor::light(),
            ThemeMode::Dark => ThemeColor::dark(),
        };

        if !cx.has_global::<Theme>() {
            let theme = Theme::from(colors);
            cx.set_global(theme);
        }

        let theme = cx.global_mut::<Theme>();

        theme.mode = mode;
        theme.colors = colors;

        if let Some(window) = window {
            window.refresh();
        }
    }
}

impl From<ThemeColor> for Theme {
    fn from(colors: ThemeColor) -> Self {
        let mode = ThemeMode::default();
        Theme {
            mode,
            transparent: Hsla::transparent_black(),
            font_size: px(16.),
            font_family: if cfg!(target_os = "macos") {
                ".SystemUIFont".into()
            } else if cfg!(target_os = "windows") {
                "Segoe UI".into()
            } else {
                "FreeMono".into()
            },
            radius: px(6.),
            radius_lg: px(8.),
            shadow: true,
            scrollbar_show: ScrollbarShow::default(),
            tile_grid_size: px(8.),
            tile_shadow: true,
            colors,
        }
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    #[inline(always)]
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Return lower_case theme name: `light`, `dark`.
    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }
}

impl From<WindowAppearance> for ThemeMode {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
        }
    }
}
