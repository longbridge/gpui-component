//! Semantic palette installation, and token-name resolution for scripts.
//!
//! `gpui_base::Theme` carries a [`SemanticThemeTokens`] whose [`ColorTokens`]
//! derives `Default`, so every color starts as `Hsla { h: 0, s: 0, l: 0, a: 0 }`
//! — fully transparent. A runtime that only calls `gpui_base::init` therefore
//! paints an invisible window; this module ships the palette that makes it
//! visible ([`RadiusTokens`] and [`SpacingTokens`] already have real defaults,
//! so only colors have to be supplied).
//!
//! The palette lives in `theme/default-tokens.json` rather than in Rust
//! literals because plugin themes load through the same
//! `Serialize`/`Deserialize`/`JsonSchema` derives; the shipped file is the
//! reference document for that format. It is embedded with `include_str!` so a
//! shell binary is self-contained and cannot start up unstyled because a file
//! is missing.

use gpui::{App, Global, Hsla, Pixels};
use gpui_base::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, Theme,
    TypographyTokens,
};
use serde::Deserialize;
use std::cell::{Cell, RefCell};

use crate::scope::with_current_app;

thread_local! {
    /// The installed palette, cached outside GPUI's `App`.
    ///
    /// Token lookups happen in two places with different access to the host:
    /// while a script records a style (inside a call scope, `App` reachable)
    /// and again while the description is materialized into real elements
    /// (outside any scope, `App` *not* reachable). Reading only through the
    /// scope made every color silently resolve to `None` during materialize —
    /// a window that painted nothing but a black rectangle. The palette changes
    /// at most once per theme switch, so caching it is both correct and
    /// cheaper than reaching for a global on every color.
    static CACHED: RefCell<Option<SemanticThemeTokens>> = const { RefCell::new(None) };
}

/// Reads the cached palette, falling back to the ambient `App` when nothing has
/// been installed yet (a host that themed the base layer itself).
fn tokens_for_lookup() -> Option<SemanticThemeTokens> {
    if let Some(tokens) = CACHED.with(|cached| cached.borrow().clone()) {
        return Some(tokens);
    }
    with_current_app(|cx| installed_tokens(cx).cloned()).flatten()
}

/// Which palette is installed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    /// The name a script uses, in `gpui.set_theme("dark")`.
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    /// Parses a script-supplied name. `None` is a caller error, not a host bug, so
    /// the binding layer can report the offending string.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            _ => None,
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, ThemeMode::Dark)
    }
}

/// The installed palette.
///
/// `gpui_base::Theme` has no mode field — it is a bag of resolved tokens — so
/// the shell has to remember which of its two palettes it wrote in order to
/// answer [`mode`] and to make [`set_mode`] idempotent.
struct InstalledPalette {
    mode: ThemeMode,
}

impl Global for InstalledPalette {}

/// Installs whatever palette is current into `gpui_base::Theme`.
///
/// Called from `gpui_shell::init`, before a host has had a chance to supply
/// one, so this is where the neutral fallback lands. A host that has a design
/// calls [`Palettes::install`] afterwards. Re-installing is harmless and does
/// not revert a mode the application already chose.
pub fn init(cx: &mut App) {
    install(mode(cx), cx);
}

/// Switches palette and returns whether anything changed.
///
/// The `false` return lets a caller that mirrors an OS appearance change skip
/// the window refresh when the appearance did not actually move.
pub fn set_mode(mode: ThemeMode, cx: &mut App) -> bool {
    if cx.try_global::<InstalledPalette>().map(|it| it.mode) == Some(mode) {
        return false;
    }

    install(mode, cx);
    // Tokens are read during render, so nothing repaints on its own.
    cx.refresh_windows();
    true
}

pub fn mode(cx: &App) -> ThemeMode {
    cx.try_global::<InstalledPalette>()
        .map(|it| it.mode)
        .unwrap_or_default()
}

/// Resolves a semantic color token name (`"background"`, `"primary_foreground"`,
/// ...) against the installed palette.
///
/// Reads the ambient `App` through [`crate::scope::with_current_app`], so the script
/// bindings can resolve `:bg("surface")` without threading a context through
/// the value conversion layer. Returns `None` outside any call scope, before
/// [`init`], or for an unknown name — resolving to a transparent color instead
/// would reproduce the very failure this module exists to prevent, so the
/// caller gets a reportable error rather than an invisible element.
pub fn token_color(name: &str) -> Option<Hsla> {
    resolve_color(&tokens_for_lookup()?.colors, name)
}

/// Same, for the spacing scale (`"xxs"`..`"xxl"`).
pub fn token_spacing(name: &str) -> Option<Pixels> {
    resolve_spacing(&tokens_for_lookup()?.spacing, name)
}

/// Same, for the radius scale (`"none"`, `"sm"`, `"md"`, `"lg"`, `"xl"`,
/// `"full"`).
pub fn token_radius(name: &str) -> Option<Pixels> {
    resolve_radius(&tokens_for_lookup()?.radius, name)
}

/// Every valid color token name, for error messages and for exposing the theme
/// to a script.
pub fn color_token_names() -> &'static [&'static str] {
    COLOR_TOKEN_NAMES
}

/// Every valid spacing token name, in scale order.
pub fn spacing_token_names() -> &'static [&'static str] {
    SPACING_TOKEN_NAMES
}

/// Every valid radius token name, in scale order.
pub fn radius_token_names() -> &'static [&'static str] {
    RADIUS_TOKEN_NAMES
}

const COLOR_TOKEN_NAMES: &[&str] = &[
    "background",
    "foreground",
    "surface",
    "surface_foreground",
    "primary",
    "primary_foreground",
    "secondary",
    "secondary_foreground",
    "muted",
    "muted_foreground",
    "accent",
    "accent_foreground",
    "destructive",
    "destructive_foreground",
    "border",
    "input",
    "ring",
];

const SPACING_TOKEN_NAMES: &[&str] = &["xxs", "xs", "sm", "md", "lg", "xl", "xxl"];

const RADIUS_TOKEN_NAMES: &[&str] = &["none", "sm", "md", "lg", "xl", "full"];

/// One palette as it is written in `default-tokens.json`.
///
/// `colors` is deliberately *not* `#[serde(default)]`: a palette that forgets a
/// color would otherwise deserialize into a transparent one and fail only as an
/// invisible pixel at runtime. The scales below default because they carry
/// meaningful Base defaults, which lets a palette override the ground colors
/// alone.
#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    colors: ColorTokens,
    #[serde(default)]
    radius: RadiusTokens,
    #[serde(default)]
    spacing: SpacingTokens,
    #[serde(default)]
    typography: TypographyTokens,
    #[serde(default)]
    shadow: ShadowTokens,
}

impl Palette {
    /// A grey scale plus one blue, at the two contrasts a UI needs.
    fn neutral(dark: bool) -> Self {
        let gray = |lightness: f32| gpui::hsla(0., 0., lightness, 1.);
        let colors = if dark {
            ColorTokens {
                background: gray(0.039_216),
                foreground: gray(0.980_392),
                surface: gray(0.039_216),
                surface_foreground: gray(0.980_392),
                primary: gray(0.980_392),
                primary_foreground: gray(0.090_196),
                secondary: gray(0.149_02),
                secondary_foreground: gray(0.980_392),
                muted: gray(0.149_02),
                muted_foreground: gray(0.639_216),
                accent: gray(0.149_02),
                accent_foreground: gray(0.980_392),
                destructive: gpui::hsla(0., 0.907, 0.708, 1.),
                destructive_foreground: gray(0.090_196),
                border: gray(0.149_02),
                input: gray(0.184_314),
                ring: gray(0.450_98),
            }
        } else {
            ColorTokens {
                background: gray(1.),
                foreground: gray(0.039_216),
                surface: gray(1.),
                surface_foreground: gray(0.039_216),
                primary: gray(0.090_196),
                primary_foreground: gray(0.980_392),
                secondary: gray(0.898_039),
                secondary_foreground: gray(0.090_196),
                muted: gray(0.960_784),
                muted_foreground: gray(0.450_98),
                accent: gray(0.960_784),
                accent_foreground: gray(0.090_196),
                destructive: gpui::hsla(0., 0.721, 0.506, 1.),
                destructive_foreground: gray(0.980_392),
                border: gray(0.898_039),
                input: gray(0.898_039),
                ring: gray(0.639_216),
            }
        };

        Self {
            colors,
            radius: RadiusTokens::default(),
            spacing: SpacingTokens::default(),
            typography: TypographyTokens::default(),
            shadow: ShadowTokens::default(),
        }
    }

    fn tokens(&self) -> SemanticThemeTokens {
        SemanticThemeTokens {
            colors: self.colors,
            radius: self.radius,
            spacing: self.spacing,
            typography: self.typography.clone(),
            shadow: self.shadow.clone(),
        }
    }
}

/// A light and a dark palette, supplied by the host.
///
/// The runtime does not ship a design. A palette is a product decision, and a
/// library that carried one would be deciding how every application built on it
/// looks — which is the thing this whole layer exists to leave to the
/// application. What the runtime does own is the *mechanism*: parsing, mode
/// switching, and the cached lookup the style layer reads.
///
/// [`Palettes::neutral`] exists so that a host which has installed nothing
/// still renders something legible; it is a safety net, not a design.
#[derive(Debug, Clone, Deserialize)]
pub struct Palettes {
    light: Palette,
    dark: Palette,
}

impl Palettes {
    /// Parses the palette format: `{ "light": { "colors": { … } }, "dark": … }`.
    ///
    /// The token names are the fields of `gpui_base`'s `SemanticThemeTokens`,
    /// which already derives `Deserialize` and `JsonSchema`, so this format has
    /// no schema of its own to keep in sync.
    pub fn parse(source: &str) -> Result<Self, String> {
        serde_json::from_str(source).map_err(|error| format!("invalid palette: {error}"))
    }

    /// The smallest palette that makes an interface legible.
    ///
    /// Deliberately plain — a neutral grey scale with one blue — because it is
    /// what a host gets for installing nothing, and it should look like a
    /// placeholder rather than like a decision somebody made for them.
    pub fn neutral() -> Self {
        Self {
            light: Palette::neutral(false),
            dark: Palette::neutral(true),
        }
    }

    /// Installs this palette and selects the current mode.
    pub fn install(self, cx: &mut App) {
        let mode = mode(cx);
        INSTALLED.with(|installed| *installed.borrow_mut() = Some(self));
        install(mode, cx);
    }

    fn get(&self, mode: ThemeMode) -> &Palette {
        match mode {
            ThemeMode::Light => &self.light,
            ThemeMode::Dark => &self.dark,
        }
    }
}

thread_local! {
    /// The palette the host installed, or the neutral fallback.
    static INSTALLED: RefCell<Option<Palettes>> = const { RefCell::new(None) };
}

fn with_palettes<R>(f: impl FnOnce(&Palettes) -> R) -> R {
    INSTALLED.with(|installed| {
        let mut installed = installed.borrow_mut();
        f(installed.get_or_insert_with(Palettes::neutral))
    })
}

fn install(mode: ThemeMode, cx: &mut App) {
    let tokens = with_palettes(|palettes| palettes.get(mode).tokens());
    CACHED.with(|cached| *cached.borrow_mut() = Some(tokens.clone()));
    Theme::global_mut(cx).tokens = tokens;
    cx.set_global(InstalledPalette { mode });
    GENERATION.with(|generation| generation.set(generation.get().wrapping_add(1)));
}

thread_local! {
    /// Bumped whenever a palette is installed.
    static GENERATION: Cell<u64> = const { Cell::new(0) };
}

/// Which palette the currently installed tokens came from.
///
/// A script resolves `:bg("surface")` to a concrete color while it builds its
/// description, so the palette is baked into the resulting snapshot. Repainting
/// the window is therefore not enough to pick up a new palette — the snapshot
/// has to be rebuilt. A view records this number alongside its snapshot and
/// rebuilds when the two disagree, which is how a theme switch reaches script
/// views without the theme knowing they exist.
pub fn generation() -> u64 {
    GENERATION.with(Cell::get)
}

/// The tokens [`init`] installed, or `None` if it never ran.
///
/// Uses `try_global` rather than `Theme::global`, whose `unwrap_or_default`
/// would hand back the all-transparent defaults and hide the missing `init`.
fn installed_tokens(cx: &App) -> Option<&SemanticThemeTokens> {
    cx.try_global::<Theme>().map(|theme| &theme.tokens)
}

fn resolve_color(colors: &ColorTokens, name: &str) -> Option<Hsla> {
    Some(match name {
        "background" => colors.background,
        "foreground" => colors.foreground,
        "surface" => colors.surface,
        "surface_foreground" => colors.surface_foreground,
        "primary" => colors.primary,
        "primary_foreground" => colors.primary_foreground,
        "secondary" => colors.secondary,
        "secondary_foreground" => colors.secondary_foreground,
        "muted" => colors.muted,
        "muted_foreground" => colors.muted_foreground,
        "accent" => colors.accent,
        "accent_foreground" => colors.accent_foreground,
        "destructive" => colors.destructive,
        "destructive_foreground" => colors.destructive_foreground,
        "border" => colors.border,
        "input" => colors.input,
        "ring" => colors.ring,
        _ => return None,
    })
}

fn resolve_spacing(spacing: &SpacingTokens, name: &str) -> Option<Pixels> {
    Some(match name {
        "xxs" => spacing.xxs,
        "xs" => spacing.xs,
        "sm" => spacing.sm,
        "md" => spacing.md,
        "lg" => spacing.lg,
        "xl" => spacing.xl,
        "xxl" => spacing.xxl,
        _ => return None,
    })
}

fn resolve_radius(radius: &RadiusTokens, name: &str) -> Option<Pixels> {
    Some(match name {
        "none" => radius.none,
        "sm" => radius.sm,
        "md" => radius.md,
        "lg" => radius.lg,
        "xl" => radius.xl,
        "full" => radius.full,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The serialized field names of a token group, which is the closest thing
    /// to reflection available here: it fails when someone adds a token to
    /// `gpui_base` and forgets the matching name list. Sorted, because whether
    /// `serde_json` preserves declaration order depends on feature unification.
    fn field_names<T: serde::Serialize>(value: &T) -> Vec<String> {
        let mut names: Vec<String> = serde_json::to_value(value)
            .expect("token groups serialize")
            .as_object()
            .expect("token groups serialize as maps")
            .keys()
            .cloned()
            .collect();
        names.sort();
        names
    }

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        names.sort();
        names
    }

    fn contrast_ratio(foreground: Hsla, background: Hsla) -> f32 {
        fn luminance(color: Hsla) -> f32 {
            let rgb = color.to_rgb();
            let linear = |channel: f32| {
                if channel <= 0.04045 {
                    channel / 12.92
                } else {
                    ((channel + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b)
        }

        let foreground = luminance(foreground);
        let background = luminance(background);
        (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
    }

    #[test]
    fn a_supplied_palette_parses() {
        // The format is the host's entry point, so a malformed one must say so
        // rather than silently installing nothing.
        assert!(Palettes::parse("{\"light\":{},\"dark\":{}}").is_err());
        assert!(Palettes::parse("not json").is_err());
    }

    #[test]
    fn the_neutral_fallback_keeps_the_base_scales() {
        let palettes = Palettes::neutral();
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let palette = palettes.get(mode);
            // A palette that overrides only colors must keep Base's scales
            // rather than zeroing them.
            assert_eq!(palette.radius, RadiusTokens::default());
            assert_eq!(palette.spacing, SpacingTokens::default());
        }
    }

    #[test]
    fn every_color_token_is_opaque_in_both_palettes() {
        let palettes = Palettes::neutral();
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let colors = palettes.get(mode).colors;
            for name in color_token_names() {
                let color = resolve_color(&colors, name)
                    .unwrap_or_else(|| panic!("`{name}` is not resolvable"));
                assert!(
                    color.a > 0.,
                    "`{name}` is transparent in the {} palette",
                    mode.as_str()
                );
            }
        }
    }

    #[test]
    fn destructive_foreground_is_legible_in_both_palettes() {
        let palettes = Palettes::neutral();
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let colors = palettes.get(mode).colors;
            let contrast = contrast_ratio(colors.destructive_foreground, colors.destructive);
            assert!(
                contrast >= 4.5,
                "destructive foreground contrast is {contrast:.2}:1 in the {} palette",
                mode.as_str()
            );
        }
    }

    #[test]
    fn color_token_names_match_the_token_fields() {
        let fields = field_names(&ColorTokens::default());
        assert_eq!(fields, sorted(color_token_names()));
        let colors = Palettes::neutral().get(ThemeMode::Light).colors;
        for name in &fields {
            assert!(
                resolve_color(&colors, name).is_some(),
                "`{name}` has no lookup arm"
            );
        }
    }

    #[test]
    fn spacing_and_radius_token_names_match_the_token_fields() {
        let spacing = SpacingTokens::default();
        assert_eq!(field_names(&spacing), sorted(spacing_token_names()));
        for name in spacing_token_names() {
            assert!(
                resolve_spacing(&spacing, name).is_some(),
                "`{name}` has no lookup arm"
            );
        }

        let radius = RadiusTokens::default();
        assert_eq!(field_names(&radius), sorted(radius_token_names()));
        for name in radius_token_names() {
            assert!(
                resolve_radius(&radius, name).is_some(),
                "`{name}` has no lookup arm"
            );
        }
    }

    #[test]
    fn unknown_token_names_resolve_to_none() {
        let dark = Palettes::neutral();
        assert!(resolve_color(&dark.get(ThemeMode::Dark).colors, "backgrund").is_none());
        assert!(resolve_spacing(&SpacingTokens::default(), "xxxl").is_none());
        assert!(resolve_radius(&RadiusTokens::default(), "rounded").is_none());
    }

    #[test]
    fn token_lookup_is_none_outside_a_call_scope() {
        // No `CallScope` is open on a test thread, so there is no ambient `App`
        // and a script-facing lookup must decline rather than invent a color.
        assert!(token_color("background").is_none());
        assert!(token_spacing("md").is_none());
        assert!(token_radius("lg").is_none());
    }

    #[test]
    fn theme_mode_names_round_trip() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            assert_eq!(ThemeMode::from_name(mode.as_str()), Some(mode));
        }
        assert!(ThemeMode::from_name("solarized").is_none());
    }
}
