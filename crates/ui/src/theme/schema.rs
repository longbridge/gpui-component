use std::{rc::Rc, sync::Arc};

use gpui::{SharedString, px};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Colorize, Theme, ThemeColor, ThemeMode,
    highlighter::{HighlightTheme, HighlightThemeStyle},
    try_parse_color,
};

/// Represents a theme configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ThemeSet {
    /// The name of the theme set.
    pub name: SharedString,
    /// The author of the theme.
    pub author: Option<SharedString>,
    /// The URL of the theme.
    pub url: Option<SharedString>,
    /// The theme list of the theme set.
    #[serde(rename = "themes")]
    pub themes: Vec<ThemeConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ThemeConfig {
    /// Whether this theme is the default theme.
    pub is_default: bool,
    /// The name of the theme.
    pub name: SharedString,
    /// The mode of the theme, default is light.
    pub mode: ThemeMode,

    /// The base font size, default is 16.
    #[serde(rename = "font.size")]
    pub font_size: Option<f32>,
    /// The base font family, default is system font: `.SystemUIFont`.
    #[serde(rename = "font.family")]
    pub font_family: Option<SharedString>,
    /// The monospace font family, default is platform specific:
    /// - macOS: `Menlo`
    /// - Windows: `Consolas`
    /// - Linux: `DejaVu Sans Mono`
    #[serde(rename = "mono_font.family")]
    pub mono_font_family: Option<SharedString>,
    /// The monospace font size, default is 13.
    #[serde(rename = "mono_font.size")]
    pub mono_font_size: Option<f32>,

    /// The border radius for general elements, default is 6.
    #[serde(rename = "radius")]
    pub radius: Option<usize>,
    /// The border radius for large elements like Dialogs and Notifications, default is 8.
    #[serde(rename = "radius.lg")]
    pub radius_lg: Option<usize>,
    /// Set shadows in the theme, for example the Input and Button, default is true.
    #[serde(rename = "shadow")]
    pub shadow: Option<bool>,

    /// The colors of the theme.
    pub colors: ThemeConfigColors,
    /// The highlight theme, this part is combilbility with `style` section in Zed theme.
    ///
    /// https://github.com/zed-industries/zed/blob/f50041779dcfd7a76c8aec293361c60c53f02d51/assets/themes/ayu/ayu.json#L9
    pub highlight: Option<HighlightThemeStyle>,
}

#[derive(Debug, Default, Clone, JsonSchema, Serialize, Deserialize)]
pub struct ThemeConfigColors {
    /// Used for accents such as hover background on MenuItem, ListItem, etc.
    #[serde(rename = "accent.background")]
    pub accent: Option<SharedString>,
    /// Used for accent text color.
    #[serde(rename = "accent.foreground")]
    pub accent_foreground: Option<SharedString>,
    /// Accordion background color.
    #[serde(rename = "accordion.background")]
    pub accordion: Option<SharedString>,
    /// Accordion hover background color.
    #[serde(rename = "accordion.hover.background")]
    pub accordion_hover: Option<SharedString>,
    /// Default background color.
    #[serde(rename = "background")]
    pub background: Option<SharedString>,
    /// Default border color
    #[serde(rename = "border")]
    pub border: Option<SharedString>,
    /// Background color for GroupBox.
    #[serde(rename = "group_box.background")]
    pub group_box: Option<SharedString>,
    /// Text color for GroupBox.
    #[serde(rename = "group_box.foreground")]
    pub group_box_foreground: Option<SharedString>,
    /// Title text color for GroupBox.
    #[serde(rename = "group_box.title.foreground")]
    pub group_box_title_foreground: Option<SharedString>,
    /// Input caret color (Blinking cursor).
    #[serde(rename = "caret")]
    pub caret: Option<SharedString>,
    /// Chart 1 color.
    #[serde(rename = "chart.1")]
    pub chart_1: Option<SharedString>,
    /// Chart 2 color.
    #[serde(rename = "chart.2")]
    pub chart_2: Option<SharedString>,
    /// Chart 3 color.
    #[serde(rename = "chart.3")]
    pub chart_3: Option<SharedString>,
    /// Chart 4 color.
    #[serde(rename = "chart.4")]
    pub chart_4: Option<SharedString>,
    /// Chart 5 color.
    #[serde(rename = "chart.5")]
    pub chart_5: Option<SharedString>,
    /// Danger background color.
    #[serde(rename = "danger.background")]
    pub danger: Option<SharedString>,
    /// Danger active background color.
    #[serde(rename = "danger.active.background")]
    pub danger_active: Option<SharedString>,
    /// Danger text color.
    #[serde(rename = "danger.foreground")]
    pub danger_foreground: Option<SharedString>,
    /// Danger hover background color.
    #[serde(rename = "danger.hover.background")]
    pub danger_hover: Option<SharedString>,
    /// Description List label background color.
    #[serde(rename = "description_list.label.background")]
    pub description_list_label: Option<SharedString>,
    /// Description List label foreground color.
    #[serde(rename = "description_list.label.foreground")]
    pub description_list_label_foreground: Option<SharedString>,
    /// Drag border color.
    #[serde(rename = "drag.border")]
    pub drag_border: Option<SharedString>,
    /// Drop target background color.
    #[serde(rename = "drop_target.background")]
    pub drop_target: Option<SharedString>,
    /// Default text color.
    #[serde(rename = "foreground")]
    pub foreground: Option<SharedString>,
    /// Info background color.
    #[serde(rename = "info.background")]
    pub info: Option<SharedString>,
    /// Info active background color.
    #[serde(rename = "info.active.background")]
    pub info_active: Option<SharedString>,
    /// Info text color.
    #[serde(rename = "info.foreground")]
    pub info_foreground: Option<SharedString>,
    /// Info hover background color.
    #[serde(rename = "info.hover.background")]
    pub info_hover: Option<SharedString>,
    /// Border color for inputs such as Input, Select, etc.
    #[serde(rename = "input.border")]
    pub input: Option<SharedString>,
    /// Link text color.
    #[serde(rename = "link")]
    pub link: Option<SharedString>,
    /// Active link text color.
    #[serde(rename = "link.active")]
    pub link_active: Option<SharedString>,
    /// Hover link text color.
    #[serde(rename = "link.hover")]
    pub link_hover: Option<SharedString>,
    /// Background color for List and ListItem.
    #[serde(rename = "list.background")]
    pub list: Option<SharedString>,
    /// Background color for active ListItem.
    #[serde(rename = "list.active.background")]
    pub list_active: Option<SharedString>,
    /// Border color for active ListItem.
    #[serde(rename = "list.active.border")]
    pub list_active_border: Option<SharedString>,
    /// Stripe background color for even ListItem.
    #[serde(rename = "list.even.background")]
    pub list_even: Option<SharedString>,
    /// Background color for List header.
    #[serde(rename = "list.head.background")]
    pub list_head: Option<SharedString>,
    /// Hover background color for ListItem.
    #[serde(rename = "list.hover.background")]
    pub list_hover: Option<SharedString>,
    /// Muted backgrounds such as Skeleton and Switch.
    #[serde(rename = "muted.background")]
    pub muted: Option<SharedString>,
    /// Muted text color, as used in disabled text.
    #[serde(rename = "muted.foreground")]
    pub muted_foreground: Option<SharedString>,
    /// Background color for Popover.
    #[serde(rename = "popover.background")]
    pub popover: Option<SharedString>,
    /// Text color for Popover.
    #[serde(rename = "popover.foreground")]
    pub popover_foreground: Option<SharedString>,
    /// Primary background color.
    #[serde(rename = "primary.background")]
    pub primary: Option<SharedString>,
    /// Active primary background color.
    #[serde(rename = "primary.active.background")]
    pub primary_active: Option<SharedString>,
    /// Primary text color.
    #[serde(rename = "primary.foreground")]
    pub primary_foreground: Option<SharedString>,
    /// Hover primary background color.
    #[serde(rename = "primary.hover.background")]
    pub primary_hover: Option<SharedString>,
    /// Progress bar background color.
    #[serde(rename = "progress.bar.background")]
    pub progress_bar: Option<SharedString>,
    /// Used for focus ring.
    #[serde(rename = "ring")]
    pub ring: Option<SharedString>,
    /// Scrollbar background color.
    #[serde(rename = "scrollbar.background")]
    pub scrollbar: Option<SharedString>,
    /// Scrollbar thumb background color.
    #[serde(rename = "scrollbar.thumb.background")]
    pub scrollbar_thumb: Option<SharedString>,
    /// Scrollbar thumb hover background color.
    #[serde(rename = "scrollbar.thumb.hover.background")]
    pub scrollbar_thumb_hover: Option<SharedString>,
    /// Secondary background color.
    #[serde(rename = "secondary.background")]
    pub secondary: Option<SharedString>,
    /// Active secondary background color.
    #[serde(rename = "secondary.active.background")]
    pub secondary_active: Option<SharedString>,
    /// Secondary text color, used for secondary Button text color or secondary text.
    #[serde(rename = "secondary.foreground")]
    pub secondary_foreground: Option<SharedString>,
    /// Hover secondary background color.
    #[serde(rename = "secondary.hover.background")]
    pub secondary_hover: Option<SharedString>,
    /// Input selection background color.
    #[serde(rename = "selection.background")]
    pub selection: Option<SharedString>,
    /// Sidebar background color.
    #[serde(rename = "sidebar.background")]
    pub sidebar: Option<SharedString>,
    /// Sidebar accent background color.
    #[serde(rename = "sidebar.accent.background")]
    pub sidebar_accent: Option<SharedString>,
    /// Sidebar accent text color.
    #[serde(rename = "sidebar.accent.foreground")]
    pub sidebar_accent_foreground: Option<SharedString>,
    /// Sidebar border color.
    #[serde(rename = "sidebar.border")]
    pub sidebar_border: Option<SharedString>,
    /// Sidebar text color.
    #[serde(rename = "sidebar.foreground")]
    pub sidebar_foreground: Option<SharedString>,
    /// Sidebar primary background color.
    #[serde(rename = "sidebar.primary.background")]
    pub sidebar_primary: Option<SharedString>,
    /// Sidebar primary text color.
    #[serde(rename = "sidebar.primary.foreground")]
    pub sidebar_primary_foreground: Option<SharedString>,
    /// Skeleton background color.
    #[serde(rename = "skeleton.background")]
    pub skeleton: Option<SharedString>,
    /// Slider bar background color.
    #[serde(rename = "slider.background")]
    pub slider_bar: Option<SharedString>,
    /// Slider thumb background color.
    #[serde(rename = "slider.thumb.background")]
    pub slider_thumb: Option<SharedString>,
    /// Success background color.
    #[serde(rename = "success.background")]
    pub success: Option<SharedString>,
    /// Success text color.
    #[serde(rename = "success.foreground")]
    pub success_foreground: Option<SharedString>,
    /// Success hover background color.
    #[serde(rename = "success.hover.background")]
    pub success_hover: Option<SharedString>,
    /// Success active background color.
    #[serde(rename = "success.active.background")]
    pub success_active: Option<SharedString>,
    /// Bullish color for candlestick charts (upward price movement).
    #[serde(rename = "bullish.background")]
    pub bullish: Option<SharedString>,
    /// Bearish color for candlestick charts (downward price movement).
    #[serde(rename = "bearish.background")]
    pub bearish: Option<SharedString>,
    /// Switch background color.
    #[serde(rename = "switch.background")]
    pub switch: Option<SharedString>,
    /// Switch thumb background color.
    #[serde(rename = "switch.thumb.background")]
    pub switch_thumb: Option<SharedString>,
    /// Tab background color.
    #[serde(rename = "tab.background")]
    pub tab: Option<SharedString>,
    /// Tab active background color.
    #[serde(rename = "tab.active.background")]
    pub tab_active: Option<SharedString>,
    /// Tab active text color.
    #[serde(rename = "tab.active.foreground")]
    pub tab_active_foreground: Option<SharedString>,
    /// TabBar background color.
    #[serde(rename = "tab_bar.background")]
    pub tab_bar: Option<SharedString>,
    /// TabBar segmented background color.
    #[serde(rename = "tab_bar.segmented.background")]
    pub tab_bar_segmented: Option<SharedString>,
    /// Tab text color.
    #[serde(rename = "tab.foreground")]
    pub tab_foreground: Option<SharedString>,
    /// Table background color.
    #[serde(rename = "table.background")]
    pub table: Option<SharedString>,
    /// Table active item background color.
    #[serde(rename = "table.active.background")]
    pub table_active: Option<SharedString>,
    /// Table active item border color.
    #[serde(rename = "table.active.border")]
    pub table_active_border: Option<SharedString>,
    /// Stripe background color for even TableRow.
    #[serde(rename = "table.even.background")]
    pub table_even: Option<SharedString>,
    /// Table head background color.
    #[serde(rename = "table.head.background")]
    pub table_head: Option<SharedString>,
    /// Table head text color.
    #[serde(rename = "table.head.foreground")]
    pub table_head_foreground: Option<SharedString>,
    /// Table item hover background color.
    #[serde(rename = "table.hover.background")]
    pub table_hover: Option<SharedString>,
    /// Table row border color.
    #[serde(rename = "table.row.border")]
    pub table_row_border: Option<SharedString>,
    /// TitleBar background color, use for Window title bar.
    #[serde(rename = "title_bar.background")]
    pub title_bar: Option<SharedString>,
    /// TitleBar border color.
    #[serde(rename = "title_bar.border")]
    pub title_bar_border: Option<SharedString>,
    /// Background color for Tiles.
    #[serde(rename = "tiles.background")]
    pub tiles: Option<SharedString>,
    /// Warning background color.
    #[serde(rename = "warning.background")]
    pub warning: Option<SharedString>,
    /// Warning active background color.
    #[serde(rename = "warning.active.background")]
    pub warning_active: Option<SharedString>,
    /// Warning hover background color.
    #[serde(rename = "warning.hover.background")]
    pub warning_hover: Option<SharedString>,
    /// Warning foreground color.
    #[serde(rename = "warning.foreground")]
    pub warning_foreground: Option<SharedString>,
    /// Overlay background color.
    #[serde(rename = "overlay")]
    pub overlay: Option<SharedString>,
    /// Window border color.
    ///
    /// # Platform specific:
    ///
    /// This is only works on Linux, other platforms we can't change the window border color.
    #[serde(rename = "window.border")]
    pub window_border: Option<SharedString>,

    /// Base blue color.
    #[serde(rename = "base.blue")]
    blue: Option<String>,
    /// Base light blue color.
    #[serde(rename = "base.blue.light")]
    blue_light: Option<String>,
    /// Base cyan color.
    #[serde(rename = "base.cyan")]
    cyan: Option<String>,
    /// Base light cyan color.
    #[serde(rename = "base.cyan.light")]
    cyan_light: Option<String>,
    /// Base green color.
    #[serde(rename = "base.green")]
    green: Option<String>,
    /// Base light green color.
    #[serde(rename = "base.green.light")]
    green_light: Option<String>,
    /// Base magenta color.
    #[serde(rename = "base.magenta")]
    magenta: Option<String>,
    #[serde(rename = "base.magenta.light")]
    magenta_light: Option<String>,
    /// Base red color.
    #[serde(rename = "base.red")]
    red: Option<String>,
    /// Base light red color.
    #[serde(rename = "base.red.light")]
    red_light: Option<String>,
    /// Base yellow color.
    #[serde(rename = "base.yellow")]
    yellow: Option<String>,
    /// Base light yellow color.
    #[serde(rename = "base.yellow.light")]
    yellow_light: Option<String>,
}

impl ThemeColor {
    /// Create a new `ThemeColor` from a `ThemeConfig`.
    pub(crate) fn apply_config(&mut self, config: &ThemeConfig, default_theme: &ThemeColor) {
        let colors = config.colors.clone();

        macro_rules! apply_color {
            ($config_field:ident) => {
                if let Some(value) = colors.$config_field {
                    if let Ok(color) = try_parse_color(&value) {
                        self.$config_field = color;
                    } else {
                        self.$config_field = default_theme.$config_field;
                    }
                } else {
                    self.$config_field = default_theme.$config_field;
                }
            };
            // With fallback
            ($config_field:ident, fallback = $fallback:expr) => {
                if let Some(value) = colors.$config_field {
                    if let Ok(color) = try_parse_color(&value) {
                        self.$config_field = color;
                    }
                } else {
                    self.$config_field = $fallback;
                }
            };
        }

        apply_color!(background);

        // Base colors for fallback
        apply_color!(red);
        apply_color!(
            red_light,
            fallback = self.background.blend(self.red.opacity(0.8))
        );
        apply_color!(green);
        apply_color!(
            green_light,
            fallback = self.background.blend(self.green.opacity(0.8))
        );
        apply_color!(blue);
        apply_color!(
            blue_light,
            fallback = self.background.blend(self.blue.opacity(0.8))
        );
        apply_color!(magenta);
        apply_color!(
            magenta_light,
            fallback = self.background.blend(self.magenta.opacity(0.8))
        );
        apply_color!(yellow);
        apply_color!(
            yellow_light,
            fallback = self.background.blend(self.yellow.opacity(0.8))
        );
        apply_color!(cyan);
        apply_color!(
            cyan_light,
            fallback = self.background.blend(self.cyan.opacity(0.8))
        );

        apply_color!(border);
        apply_color!(foreground);
        apply_color!(muted);
        apply_color!(
            muted_foreground,
            fallback = self.muted.blend(self.foreground.opacity(0.7))
        );

        // Button colors
        let active_darken = if config.mode.is_dark() { 0.2 } else { 0.1 };
        let hover_opacity = 0.9;
        apply_color!(primary);
        apply_color!(primary_foreground, fallback = self.foreground);
        apply_color!(
            primary_hover,
            fallback = self.background.blend(self.primary.opacity(hover_opacity))
        );
        apply_color!(
            primary_active,
            fallback = self.primary.darken(active_darken)
        );
        apply_color!(secondary);
        apply_color!(secondary_foreground, fallback = self.foreground);
        apply_color!(
            secondary_hover,
            fallback = self.background.blend(self.secondary.opacity(hover_opacity))
        );
        apply_color!(
            secondary_active,
            fallback = self.secondary.darken(active_darken)
        );
        apply_color!(success, fallback = self.green);
        apply_color!(success_foreground, fallback = self.primary_foreground);
        apply_color!(
            success_hover,
            fallback = self.background.blend(self.success.opacity(hover_opacity))
        );
        apply_color!(
            success_active,
            fallback = self.success.darken(active_darken)
        );
        apply_color!(bullish, fallback = self.green);
        apply_color!(bearish, fallback = self.red);
        apply_color!(info, fallback = self.cyan);
        apply_color!(info_foreground, fallback = self.primary_foreground);
        apply_color!(
            info_hover,
            fallback = self.background.blend(self.info.opacity(hover_opacity))
        );
        apply_color!(info_active, fallback = self.info.darken(active_darken));
        apply_color!(warning, fallback = self.yellow);
        apply_color!(warning_foreground, fallback = self.primary_foreground);
        apply_color!(
            warning_hover,
            fallback = self.background.blend(self.warning.opacity(0.9))
        );
        apply_color!(
            warning_active,
            fallback = self.background.blend(self.warning.darken(active_darken))
        );

        // Other colors
        apply_color!(accent, fallback = self.secondary);
        apply_color!(accent_foreground, fallback = self.foreground);
        apply_color!(accordion, fallback = self.background);
        apply_color!(accordion_hover, fallback = self.accent.opacity(0.8));
        apply_color!(
            group_box,
            fallback = self
                .background
                .blend(
                    self.secondary
                        .opacity(if config.mode.is_dark() { 0.3 } else { 0.4 })
                )
        );
        apply_color!(group_box_foreground, fallback = self.foreground);
        apply_color!(caret, fallback = self.primary);
        apply_color!(chart_1, fallback = self.blue.lighten(0.4));
        apply_color!(chart_2, fallback = self.blue.lighten(0.2));
        apply_color!(chart_3, fallback = self.blue);
        apply_color!(chart_4, fallback = self.blue.darken(0.2));
        apply_color!(chart_5, fallback = self.blue.darken(0.4));
        apply_color!(danger, fallback = self.red);
        apply_color!(danger_active, fallback = self.danger.darken(active_darken));
        apply_color!(danger_foreground, fallback = self.primary_foreground);
        apply_color!(
            danger_hover,
            fallback = self.background.blend(self.danger.opacity(0.9))
        );
        apply_color!(
            description_list_label,
            fallback = self.background.blend(self.border.opacity(0.2))
        );
        apply_color!(
            description_list_label_foreground,
            fallback = self.muted_foreground
        );
        apply_color!(drag_border, fallback = self.primary.opacity(0.65));
        apply_color!(drop_target, fallback = self.primary.opacity(0.2));
        apply_color!(input, fallback = self.border);
        apply_color!(link, fallback = self.primary);
        apply_color!(link_active, fallback = self.link);
        apply_color!(link_hover, fallback = self.link);
        apply_color!(list, fallback = self.background);
        apply_color!(
            list_active,
            fallback = self.background.blend(self.primary.opacity(0.1))
        );
        apply_color!(
            list_active_border,
            fallback = self.background.blend(self.primary.opacity(0.6))
        );
        apply_color!(list_even, fallback = self.list);
        apply_color!(list_head, fallback = self.list);
        apply_color!(list_hover, fallback = self.secondary_hover);
        apply_color!(popover, fallback = self.background);
        apply_color!(popover_foreground, fallback = self.foreground);
        apply_color!(progress_bar, fallback = self.primary);
        apply_color!(ring, fallback = self.blue);
        apply_color!(scrollbar, fallback = self.background);
        apply_color!(scrollbar_thumb, fallback = self.accent);
        apply_color!(scrollbar_thumb_hover, fallback = self.scrollbar_thumb);
        apply_color!(selection, fallback = self.primary);
        apply_color!(sidebar, fallback = self.background);
        apply_color!(sidebar_accent, fallback = self.accent);
        apply_color!(sidebar_accent_foreground, fallback = self.accent_foreground);
        apply_color!(sidebar_border, fallback = self.border);
        apply_color!(sidebar_foreground, fallback = self.foreground);
        apply_color!(sidebar_primary, fallback = self.primary);
        apply_color!(
            sidebar_primary_foreground,
            fallback = self.primary_foreground
        );
        apply_color!(skeleton, fallback = self.secondary);
        apply_color!(slider_bar, fallback = self.primary);
        apply_color!(slider_thumb, fallback = self.primary_foreground);
        apply_color!(switch, fallback = self.secondary);
        apply_color!(switch_thumb, fallback = self.background);
        apply_color!(tab, fallback = self.background);
        apply_color!(tab_active, fallback = self.background);
        apply_color!(tab_active_foreground, fallback = self.foreground);
        apply_color!(tab_bar, fallback = self.background);
        apply_color!(tab_bar_segmented, fallback = self.secondary);
        apply_color!(tab_foreground, fallback = self.foreground);
        apply_color!(table, fallback = self.list);
        apply_color!(table_active, fallback = self.list_active);
        apply_color!(table_active_border, fallback = self.list_active_border);
        apply_color!(table_even, fallback = self.list_even);
        apply_color!(table_head, fallback = self.list_head);
        apply_color!(table_head_foreground, fallback = self.muted_foreground);
        apply_color!(table_hover, fallback = self.list_hover);
        apply_color!(table_row_border, fallback = self.border);
        apply_color!(title_bar, fallback = self.background);
        apply_color!(title_bar_border, fallback = self.border);
        apply_color!(tiles, fallback = self.background);
        apply_color!(overlay);
        apply_color!(window_border, fallback = self.border);

        // TODO: Apply default fallback colors to highlight.

        // Ensure opacity for list_active, table_active
        self.list_active = self.list_active.alpha(self.list_active.a.min(0.2));
        self.table_active = self.table_active.alpha(self.table_active.a.min(0.2));
        self.selection = self.selection.alpha(self.selection.a.min(0.3));
    }
}

impl Theme {
    /// Apply the given theme configuration to the current theme.
    pub fn apply_config(&mut self, config: &Rc<ThemeConfig>) {
        if config.mode.is_dark() {
            self.dark_theme = config.clone();
        } else {
            self.light_theme = config.clone();
        }
        if let Some(style) = &config.highlight {
            let highlight_theme = Arc::new(HighlightTheme {
                name: config.name.to_string(),
                appearance: config.mode,
                style: style.clone(),
            });
            self.highlight_theme = highlight_theme.clone();
        }

        let default_theme = if config.mode.is_dark() {
            Self::from(ThemeColor::dark().as_ref())
        } else {
            Self::from(ThemeColor::light().as_ref())
        };

        if let Some(font_size) = config.font_size {
            self.font_size = px(font_size);
        } else {
            self.font_size = default_theme.font_size;
        }
        if let Some(font_family) = &config.font_family {
            self.font_family = font_family.clone();
        } else {
            self.font_family = default_theme.font_family.clone();
        }
        if let Some(mono_font_family) = &config.mono_font_family {
            self.mono_font_family = mono_font_family.clone();
        } else {
            self.mono_font_family = default_theme.mono_font_family.clone();
        }
        if let Some(mono_font_size) = config.mono_font_size {
            self.mono_font_size = px(mono_font_size);
        } else {
            self.mono_font_size = default_theme.mono_font_size;
        }
        if let Some(radius) = config.radius {
            self.radius = px(radius as f32);
        } else {
            self.radius = default_theme.radius;
        }
        if let Some(radius_lg) = config.radius_lg {
            self.radius_lg = px(radius_lg as f32);
        } else {
            self.radius_lg = default_theme.radius_lg;
        }
        if let Some(shadow) = config.shadow {
            self.shadow = shadow;
        } else {
            self.shadow = default_theme.shadow;
        }

        self.colors.apply_config(&config, &default_theme.colors);
        self.mode = config.mode;
    }
}
