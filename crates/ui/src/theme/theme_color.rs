use std::{ops::Deref, sync::Arc};

use crate::{ThemeMode, theme::DEFAULT_THEME_COLORS};

use gpui::{Background, Fill, Hsla};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A theme token that keeps a solid representative color and its renderable background.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ThemeToken {
    pub color: Hsla,
    pub background: Background,
}

impl ThemeToken {
    pub fn new(color: Hsla, background: Background) -> Self {
        Self { color, background }
    }
}

impl Deref for ThemeToken {
    type Target = Hsla;

    fn deref(&self) -> &Self::Target {
        &self.color
    }
}

impl From<Hsla> for ThemeToken {
    fn from(color: Hsla) -> Self {
        Self {
            color,
            background: color.into(),
        }
    }
}

impl From<ThemeToken> for Hsla {
    fn from(token: ThemeToken) -> Self {
        token.color
    }
}

impl From<ThemeToken> for Background {
    fn from(token: ThemeToken) -> Self {
        token.background
    }
}

impl From<ThemeToken> for Fill {
    fn from(token: ThemeToken) -> Self {
        Fill::Color(token.background)
    }
}

/// Theme colors used throughout the UI components.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct ThemeColor {
    /// Used for accents such as hover background on MenuItem, ListItem, etc.
    pub accent: ThemeToken,
    /// Used for accent text color.
    pub accent_foreground: ThemeToken,
    /// Accordion background color.
    pub accordion: ThemeToken,
    /// Accordion hover background color.
    pub accordion_hover: ThemeToken,
    /// Default background color.
    pub background: ThemeToken,
    /// Default border color
    pub border: ThemeToken,
    /// Default Button background color.
    pub button: ThemeToken,
    /// Default Button active background color.
    pub button_active: ThemeToken,
    /// Default Button text color.
    pub button_foreground: ThemeToken,
    /// Default Button hover background color.
    pub button_hover: ThemeToken,
    /// Button danger background color, fallback to `danger`.
    pub button_danger: ThemeToken,
    /// Button danger active background color, fallback to `danger_active`.
    pub button_danger_active: ThemeToken,
    /// Button danger text color, fallback to `danger_foreground`.
    pub button_danger_foreground: ThemeToken,
    /// Button danger hover background color, fallback to `danger_hover`.
    pub button_danger_hover: ThemeToken,
    /// Button info background color, fallback to `info`.
    pub button_info: ThemeToken,
    /// Button info active background color, fallback to `info_active`.
    pub button_info_active: ThemeToken,
    /// Button info text color, fallback to `info_foreground`.
    pub button_info_foreground: ThemeToken,
    /// Button info hover background color, fallback to `info_hover`.
    pub button_info_hover: ThemeToken,
    /// Button primary background color, fallback to `primary`.
    pub button_primary: ThemeToken,
    /// Button primary active background color, fallback to `primary_active`.
    pub button_primary_active: ThemeToken,
    /// Button primary text color, fallback to `primary_foreground`.
    pub button_primary_foreground: ThemeToken,
    /// Button primary hover background color, fallback to `primary_hover`.
    pub button_primary_hover: ThemeToken,
    /// Button secondary background color, fallback to `secondary`.
    pub button_secondary: ThemeToken,
    /// Button secondary active background color, fallback to `secondary_active`.
    pub button_secondary_active: ThemeToken,
    /// Button secondary text color, fallback to `secondary_foreground`.
    pub button_secondary_foreground: ThemeToken,
    /// Button secondary hover background color, fallback to `secondary_hover`.
    pub button_secondary_hover: ThemeToken,
    /// Button success background color, fallback to `success`.
    pub button_success: ThemeToken,
    /// Button success active background color, fallback to `success_active`.
    pub button_success_active: ThemeToken,
    /// Button success text color, fallback to `success_foreground`.
    pub button_success_foreground: ThemeToken,
    /// Button success hover background color, fallback to `success_hover`.
    pub button_success_hover: ThemeToken,
    /// Button warning background color, fallback to `warning`.
    pub button_warning: ThemeToken,
    /// Button warning active background color, fallback to `warning_active`.
    pub button_warning_active: ThemeToken,
    /// Button warning text color, fallback to `warning_foreground`.
    pub button_warning_foreground: ThemeToken,
    /// Button warning hover background color, fallback to `warning_hover`.
    pub button_warning_hover: ThemeToken,
    /// Background color for GroupBox.
    pub group_box: ThemeToken,
    /// Text color for GroupBox.
    pub group_box_foreground: ThemeToken,
    /// Input caret color (Blinking cursor).
    pub caret: ThemeToken,
    /// Chart 1 color.
    pub chart_1: ThemeToken,
    /// Chart 2 color.
    pub chart_2: ThemeToken,
    /// Chart 3 color.
    pub chart_3: ThemeToken,
    /// Chart 4 color.
    pub chart_4: ThemeToken,
    /// Chart 5 color.
    pub chart_5: ThemeToken,
    /// Bullish color for candlestick charts (upward price movement).
    pub chart_bullish: ThemeToken,
    /// Bearish color for candlestick charts (downward price movement).
    pub chart_bearish: ThemeToken,
    /// Danger background color.
    pub danger: ThemeToken,
    /// Danger active background color.
    pub danger_active: ThemeToken,
    /// Danger text color.
    pub danger_foreground: ThemeToken,
    /// Danger hover background color.
    pub danger_hover: ThemeToken,
    /// Description List label background color.
    pub description_list_label: ThemeToken,
    /// Description List label foreground color.
    pub description_list_label_foreground: ThemeToken,
    /// Drag border color.
    pub drag_border: ThemeToken,
    /// Drop target background color.
    pub drop_target: ThemeToken,
    /// Default text color.
    pub foreground: ThemeToken,
    /// Info background color.
    pub info: ThemeToken,
    /// Info active background color.
    pub info_active: ThemeToken,
    /// Info text color.
    pub info_foreground: ThemeToken,
    /// Info hover background color.
    pub info_hover: ThemeToken,
    /// Border color for inputs such as Input, Select, etc.
    pub input: ThemeToken,
    /// Link text color.
    pub link: ThemeToken,
    /// Active link text color.
    pub link_active: ThemeToken,
    /// Hover link text color.
    pub link_hover: ThemeToken,
    /// Background color for List and ListItem.
    pub list: ThemeToken,
    /// Background color for active ListItem.
    pub list_active: ThemeToken,
    /// Border color for active ListItem.
    pub list_active_border: ThemeToken,
    /// Stripe background color for even ListItem.
    pub list_even: ThemeToken,
    /// Background color for List header.
    pub list_head: ThemeToken,
    /// Hover background color for ListItem.
    pub list_hover: ThemeToken,
    /// Muted backgrounds such as Skeleton and Switch.
    pub muted: ThemeToken,
    /// Muted text color, as used in disabled text.
    pub muted_foreground: ThemeToken,
    /// Background color for Popover.
    pub popover: ThemeToken,
    /// Text color for Popover.
    pub popover_foreground: ThemeToken,
    /// Primary background color.
    pub primary: ThemeToken,
    /// Active primary background color.
    pub primary_active: ThemeToken,
    /// Primary text color.
    pub primary_foreground: ThemeToken,
    /// Hover primary background color.
    pub primary_hover: ThemeToken,
    /// Progress bar background color.
    pub progress_bar: ThemeToken,
    /// Used for focus ring.
    pub ring: ThemeToken,
    /// Scrollbar background color.
    pub scrollbar: ThemeToken,
    /// Scrollbar thumb background color.
    pub scrollbar_thumb: ThemeToken,
    /// Scrollbar thumb hover background color.
    pub scrollbar_thumb_hover: ThemeToken,
    /// Secondary background color.
    pub secondary: ThemeToken,
    /// Active secondary background color.
    pub secondary_active: ThemeToken,
    /// Secondary text color, used for secondary Button text color or secondary text.
    pub secondary_foreground: ThemeToken,
    /// Hover secondary background color.
    pub secondary_hover: ThemeToken,
    /// Input selection background color.
    pub selection: ThemeToken,
    /// Sidebar background color.
    pub sidebar: ThemeToken,
    /// Sidebar accent background color.
    pub sidebar_accent: ThemeToken,
    /// Sidebar accent text color.
    pub sidebar_accent_foreground: ThemeToken,
    /// Sidebar border color.
    pub sidebar_border: ThemeToken,
    /// Sidebar text color.
    pub sidebar_foreground: ThemeToken,
    /// Sidebar primary background color.
    pub sidebar_primary: ThemeToken,
    /// Sidebar primary text color.
    pub sidebar_primary_foreground: ThemeToken,
    /// Skeleton background color.
    pub skeleton: ThemeToken,
    /// Slider bar background color.
    pub slider_bar: ThemeToken,
    /// Slider thumb background color.
    pub slider_thumb: ThemeToken,
    /// Success background color.
    pub success: ThemeToken,
    /// Success text color.
    pub success_foreground: ThemeToken,
    /// Success hover background color.
    pub success_hover: ThemeToken,
    /// Success active background color.
    pub success_active: ThemeToken,
    /// Switch background color.
    pub switch: ThemeToken,
    /// Switch thumb background color.
    pub switch_thumb: ThemeToken,
    /// Tab background color.
    pub tab: ThemeToken,
    /// Tab active background color.
    pub tab_active: ThemeToken,
    /// Tab active text color.
    pub tab_active_foreground: ThemeToken,
    /// TabBar background color.
    pub tab_bar: ThemeToken,
    /// TabBar segmented background color.
    pub tab_bar_segmented: ThemeToken,
    /// Tab text color.
    pub tab_foreground: ThemeToken,
    /// Table background color.
    pub table: ThemeToken,
    /// Table active item background color.
    pub table_active: ThemeToken,
    /// Table active item border color.
    pub table_active_border: ThemeToken,
    /// Stripe background color for even TableRow.
    pub table_even: ThemeToken,
    /// Table head background color.
    pub table_head: ThemeToken,
    /// Table head text color.
    pub table_head_foreground: ThemeToken,
    /// Table footer background color.
    pub table_foot: ThemeToken,
    /// Table footer text color.
    pub table_foot_foreground: ThemeToken,
    /// Table item hover background color.
    pub table_hover: ThemeToken,
    /// Table row border color.
    pub table_row_border: ThemeToken,
    /// TitleBar background color, use for Window title bar.
    pub title_bar: ThemeToken,
    /// TitleBar border color.
    pub title_bar_border: ThemeToken,
    /// StatusBar background color, use for the bottom status bar.
    pub status_bar: ThemeToken,
    /// StatusBar border color.
    pub status_bar_border: ThemeToken,
    /// Background color for Tiles.
    pub tiles: ThemeToken,
    /// Warning background color.
    pub warning: ThemeToken,
    /// Warning active background color.
    pub warning_active: ThemeToken,
    /// Warning hover background color.
    pub warning_hover: ThemeToken,
    /// Warning foreground color.
    pub warning_foreground: ThemeToken,
    /// Overlay background color.
    pub overlay: ThemeToken,
    /// Window border color.
    ///
    /// # Platform specific:
    ///
    /// This is only works on Linux, other platforms we can't change the window border color.
    pub window_border: ThemeToken,

    /// The base red color.
    pub red: ThemeToken,
    /// The base red light color.
    pub red_light: ThemeToken,
    /// The base green color.
    pub green: ThemeToken,
    /// The base green light color.
    pub green_light: ThemeToken,
    /// The base blue color.
    pub blue: ThemeToken,
    /// The base blue light color.
    pub blue_light: ThemeToken,
    /// The base yellow color.
    pub yellow: ThemeToken,
    /// The base yellow light color.
    pub yellow_light: ThemeToken,
    /// The base magenta color.
    pub magenta: ThemeToken,
    /// The base magenta light color.
    pub magenta_light: ThemeToken,
    /// The base cyan color.
    pub cyan: ThemeToken,
    /// The base cyan light color.
    pub cyan_light: ThemeToken,
}

impl ThemeColor {
    /// Get the default light theme colors.
    pub fn light() -> Arc<Self> {
        DEFAULT_THEME_COLORS[&ThemeMode::Light].0.clone()
    }

    /// Get the default dark theme colors.
    pub fn dark() -> Arc<Self> {
        DEFAULT_THEME_COLORS[&ThemeMode::Dark].0.clone()
    }
}
