use crate::{
    highlighter::HighlightTheme, list::ListSettings, notification::NotificationSettings,
    scroll::ScrollbarMode, sheet::SheetSettings,
};
use gpui::{App, Global, Hsla, Pixels, SharedString, Window, WindowAppearance, px};
pub use gpui_base::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

mod color;
mod registry;
mod schema;
mod theme_color;

pub use color::*;
pub use registry::*;
pub use schema::*;
pub use theme_color::*;

pub fn init(cx: &mut App) {
    registry::init(cx);

    // Ensure theme is loaded directly on startup for WASM compatibility
    Theme::change(ThemeMode::Light, None, cx);
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

fn default_true() -> bool {
    true
}

/// The global theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Theme {
    pub colors: ThemeColor,
    /// Component-specific resolved tokens retained for legacy compatibility.
    ///
    /// New application-owned presentation should use [`Self::semantic_tokens`]
    /// rather than extending this legacy surface.
    #[serde(default)]
    pub tokens: ThemeTokens,
    pub highlight_theme: Arc<HighlightTheme>,
    pub light_theme: Rc<ThemeConfig>,
    pub dark_theme: Rc<ThemeConfig>,

    pub mode: ThemeMode,
    /// The font family for the application, default is `.SystemUIFont`.
    pub font_family: SharedString,
    /// The base font size for the application, default is 16px.
    pub font_size: Pixels,
    /// The monospace font family for the application.
    ///
    /// Defaults to:
    ///
    /// - macOS: `Menlo`
    /// - Windows: `Consolas`
    /// - Linux: `DejaVu Sans Mono`
    pub mono_font_family: SharedString,
    /// The monospace font size for the application, default is 13px.
    pub mono_font_size: Pixels,
    /// Radius for the general elements.
    pub radius: Pixels,
    /// Radius for the large elements, e.g.: Dialog, Notification border radius.
    pub radius_lg: Pixels,
    pub shadow: bool,
    /// Whether focused controls draw a ring outside their border, default true.
    ///
    /// The ring is painted outside the element, so any ancestor that clips its
    /// content will cut it off. An application whose layout clips heavily can
    /// turn it off here: focused controls then show only their tinted border,
    /// which costs no space and cannot be clipped.
    #[serde(default = "default_true")]
    pub focus_ring: bool,
    pub transparent: Hsla,
    /// Show the scrollbar mode, default: Scrolling
    #[serde(alias = "scrollbar_show")]
    pub scrollbar_mode: ScrollbarMode,
    /// The notification setting.
    #[serde(skip)]
    pub notification: NotificationSettings,
    /// Tile grid size, default is 4px.
    pub tile_grid_size: Pixels,
    /// The shadow of the tile panel.
    pub tile_shadow: bool,
    /// The border radius of the tile panel, default is 0px.
    pub tile_radius: Pixels,
    /// The list settings.
    pub list: ListSettings,
    /// The sheet settings.
    pub sheet: SheetSettings,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from(&ThemeColor::default())
    }
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

    /// Returns the current theme name.
    pub fn theme_name(&self) -> &SharedString {
        if self.is_dark() {
            &self.dark_theme.name
        } else {
            &self.light_theme.name
        }
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
        let mode = if cx.should_auto_hide_scrollbars() {
            ScrollbarMode::Scrolling
        } else {
            ScrollbarMode::Hover
        };
        Self::set_scrollbar_mode(mode, cx);
    }

    /// Changes the scrollbar display mode and synchronizes the Base projection.
    pub fn set_scrollbar_mode(mode: ScrollbarMode, cx: &mut App) {
        Theme::global_mut(cx).scrollbar_mode = mode;
        gpui_base::Theme::global_mut(cx).scrollbar.mode = mode;
    }

    /// Change the theme mode.
    pub fn change(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App) {
        let mode = mode.into();
        if !cx.has_global::<Theme>() {
            let mut theme = Theme::default();
            theme.light_theme = ThemeRegistry::global(cx).default_light_theme().clone();
            theme.dark_theme = ThemeRegistry::global(cx).default_dark_theme().clone();
            cx.set_global(theme);
        }

        let theme = cx.global_mut::<Theme>();
        theme.mode = mode;
        if mode.is_dark() {
            theme.apply_config(&theme.dark_theme.clone());
        } else {
            theme.apply_config(&theme.light_theme.clone());
        }

        let base_theme = gpui_base::Theme {
            tokens: theme.semantic_tokens(),
            scrollbar: gpui_base::ScrollbarTheme {
                mode: theme.scrollbar_mode,
                styles: gpui_base::ScrollbarStyles::default()
                    .track(|style| style.bg(theme.scrollbar))
                    .track_hover(|style| style.bg(theme.scrollbar))
                    .track_active(|style| style.bg(theme.scrollbar).border_color(theme.border))
                    .thumb(|style| style.bg(theme.tokens.scrollbar_thumb).radius(theme.radius))
                    .thumb_hover(|style| {
                        style
                            .bg(theme.tokens.scrollbar_thumb_hover)
                            .radius(theme.radius)
                    })
                    .thumb_active(|style| {
                        style
                            .bg(theme.tokens.scrollbar_thumb_hover)
                            .radius(theme.radius)
                    }),
            },
            resizable: gpui_base::ResizableTheme {
                handle: theme.border,
                active_handle: theme.drag_border,
            },
        };
        cx.set_global(base_theme);

        if let Some(window) = window {
            window.refresh();
        }
    }

    /// Get the input background color.
    ///
    /// For dark, use a transparent color mixed with the input border: `cx.theme().input`,
    /// otherwise use the `cx.theme().background` color.
    #[inline]
    pub fn input_background(&self) -> Hsla {
        if self.is_dark() {
            self.input.mix_oklab(self.transparent, 0.3)
        } else {
            self.background
        }
    }

    /// Get the editor background color, if not set, use the input background color.
    #[inline]
    pub(crate) fn editor_background(&self) -> Hsla {
        self.highlight_theme
            .style
            .editor_background
            .unwrap_or_else(|| self.input_background())
    }

    /// Returns a snapshot of the semantic design tokens represented by this
    /// theme. The snapshot is computed from the legacy public fields so direct
    /// mutations of those fields are reflected immediately.
    pub fn semantic_tokens(&self) -> SemanticThemeTokens {
        SemanticThemeTokens {
            colors: self.color_tokens(),
            radius: self.radius_tokens(),
            spacing: self.spacing_tokens(),
            typography: self.typography_tokens(),
            shadow: self.shadow_tokens(),
        }
    }

    pub fn color_tokens(&self) -> ColorTokens {
        ColorTokens {
            background: self.background,
            foreground: self.foreground,
            surface: self.popover,
            surface_foreground: self.popover_foreground,
            primary: self.primary,
            primary_foreground: self.primary_foreground,
            secondary: self.secondary,
            secondary_foreground: self.secondary_foreground,
            muted: self.muted,
            muted_foreground: self.muted_foreground,
            accent: self.accent,
            accent_foreground: self.accent_foreground,
            destructive: self.danger,
            destructive_foreground: self.danger_foreground,
            border: self.border,
            input: self.input,
            ring: self.ring,
        }
    }

    pub fn radius_tokens(&self) -> RadiusTokens {
        RadiusTokens {
            none: px(0.),
            sm: self.radius / 2.,
            md: self.radius,
            lg: self.radius_lg,
            xl: self.radius * 2.,
            full: px(9999.),
        }
    }

    pub fn spacing_tokens(&self) -> SpacingTokens {
        SpacingTokens::default()
    }

    pub fn typography_tokens(&self) -> TypographyTokens {
        let mut tokens = TypographyTokens::default();
        tokens.sans = self.font_family.clone();
        tokens.mono = self.mono_font_family.clone();
        tokens.md.size = self.font_size;
        tokens.mono_md.size = self.mono_font_size;
        tokens
    }

    pub fn shadow_tokens(&self) -> ShadowTokens {
        if self.shadow {
            ShadowTokens::elevations(self.transparent.alpha(0.18))
        } else {
            ShadowTokens::default()
        }
    }

    /// Applies the subset of semantic tokens representable by the legacy
    /// theme. Scale-only spacing and elevation details have no legacy storage;
    /// legacy components therefore keep their existing behavior.
    pub fn apply_semantic_tokens(&mut self, tokens: &SemanticThemeTokens) {
        let colors = tokens.colors;
        self.background = colors.background;
        self.foreground = colors.foreground;
        self.popover = colors.surface;
        self.popover_foreground = colors.surface_foreground;
        self.primary = colors.primary;
        self.primary_foreground = colors.primary_foreground;
        self.secondary = colors.secondary;
        self.secondary_foreground = colors.secondary_foreground;
        self.muted = colors.muted;
        self.muted_foreground = colors.muted_foreground;
        self.accent = colors.accent;
        self.accent_foreground = colors.accent_foreground;
        self.danger = colors.destructive;
        self.danger_foreground = colors.destructive_foreground;
        self.border = colors.border;
        self.input = colors.input;
        self.ring = colors.ring;

        self.tokens.background = colors.background.into();
        self.tokens.popover = colors.surface.into();
        self.tokens.primary = colors.primary.into();
        self.tokens.secondary = colors.secondary.into();
        self.tokens.muted = colors.muted.into();
        self.tokens.accent = colors.accent.into();
        self.tokens.danger = colors.destructive.into();

        self.radius = tokens.radius.md;
        self.radius_lg = tokens.radius.lg;
        self.font_family = tokens.typography.sans.clone();
        self.mono_font_family = tokens.typography.mono.clone();
        self.font_size = tokens.typography.md.size;
        self.mono_font_size = tokens.typography.mono_md.size;
        self.shadow = !tokens.shadow.sm.is_empty()
            || !tokens.shadow.md.is_empty()
            || !tokens.shadow.lg.is_empty();
    }

    /// Resolves a standalone semantic configuration over the current legacy
    /// theme without mutating either value.
    pub fn resolve_semantic_config(&self, config: &SemanticThemeConfig) -> SemanticThemeTokens {
        let mut tokens = self.semantic_tokens();
        config.apply_to(&mut tokens);
        tokens
    }

    /// Applies the legacy-representable part of a standalone semantic config
    /// and returns the complete resolved snapshot for application-owned UI.
    pub fn apply_semantic_config(&mut self, config: &SemanticThemeConfig) -> SemanticThemeTokens {
        let tokens = self.resolve_semantic_config(config);
        self.apply_semantic_tokens(&tokens);
        tokens
    }

    /// Parses and applies a standalone `{ "tokens": ... }` semantic theme file.
    pub fn apply_semantic_config_str(
        &mut self,
        content: &str,
    ) -> anyhow::Result<SemanticThemeTokens> {
        let config = serde_json::from_str::<SemanticThemeConfigFile>(content)?;
        Ok(self.apply_semantic_config(&config.tokens))
    }
}

#[cfg(test)]
mod semantic_token_tests {
    use gpui::{Hsla, px};

    use super::Theme;

    #[test]
    fn semantic_colors_are_a_live_projection_of_legacy_fields() {
        let mut theme = Theme::default();
        let primary = Hsla::default().alpha(0.42);
        theme.primary = primary;

        assert_eq!(theme.color_tokens().primary, primary);
        assert_eq!(theme.semantic_tokens().colors.primary, primary);
    }

    #[test]
    fn applying_semantic_tokens_only_updates_generic_legacy_colors() {
        let mut theme = Theme::default();
        let component_color = theme.button_primary;
        let mut tokens = theme.semantic_tokens();
        tokens.colors.primary = Hsla::default().alpha(0.25);
        tokens.colors.destructive = Hsla::default().alpha(0.75);
        tokens.radius.md = px(10.);

        theme.apply_semantic_tokens(&tokens);

        assert_eq!(theme.primary, tokens.colors.primary);
        assert_eq!(theme.tokens.primary.color, tokens.colors.primary);
        assert_eq!(theme.danger, tokens.colors.destructive);
        assert_eq!(theme.radius, px(10.));
        assert_eq!(theme.button_primary, component_color);
    }

    #[test]
    fn disabled_legacy_shadows_project_to_empty_elevations() {
        let mut theme = Theme::default();
        theme.shadow = false;

        let shadows = theme.shadow_tokens();
        assert!(shadows.sm.is_empty());
        assert!(shadows.md.is_empty());
        assert!(shadows.lg.is_empty());
    }
}

impl From<&ThemeColor> for Theme {
    fn from(colors: &ThemeColor) -> Self {
        Theme {
            mode: ThemeMode::default(),
            transparent: Hsla::transparent_black(),
            font_family: ".SystemUIFont".into(),
            font_size: px(16.),
            mono_font_family: if cfg!(target_os = "macos") {
                // https://en.wikipedia.org/wiki/Menlo_(typeface)
                "Menlo".into()
            } else if cfg!(target_os = "windows") {
                "Consolas".into()
            } else {
                "DejaVu Sans Mono".into()
            },
            mono_font_size: px(13.),
            radius: px(6.),
            radius_lg: px(8.),
            shadow: true,
            focus_ring: true,
            scrollbar_mode: ScrollbarMode::default(),
            notification: NotificationSettings::default(),
            tile_grid_size: px(8.),
            tile_shadow: true,
            tile_radius: px(0.),
            list: ListSettings::default(),
            colors: *colors,
            tokens: ThemeTokens::from(colors),
            light_theme: Rc::new(ThemeConfig::default()),
            dark_theme: Rc::new(ThemeConfig::default()),
            highlight_theme: HighlightTheme::default_light(),
            sheet: SheetSettings::default(),
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Light,
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

#[cfg(test)]
mod base_theme_projection_tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn base_theme_tracks_initialization_and_mode_changes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            assert_styled_projection(cx);

            Theme::change(ThemeMode::Dark, None, cx);
            assert_styled_projection(cx);

            Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);
            assert_eq!(Theme::global(cx).scrollbar_mode, ScrollbarMode::Always);
            assert_eq!(
                gpui_base::Theme::global(cx).scrollbar.mode,
                gpui_base::ScrollbarMode::Always
            );
        });
    }

    fn assert_styled_projection(cx: &App) {
        let theme = Theme::global(cx);
        let base = gpui_base::Theme::global(cx);

        assert_eq!(base.tokens, theme.semantic_tokens());
        assert_eq!(base.scrollbar.mode, theme.scrollbar_mode);
        assert_eq!(base.resizable.handle, theme.border);
        assert_eq!(base.resizable.active_handle, theme.drag_border);
    }
}
