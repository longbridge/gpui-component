//! The shell's default semantic palette, and token-name resolution for Lua.
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

use std::cell::RefCell;
use std::sync::LazyLock;

use gpui::{App, Global, Hsla, Pixels};
use gpui_base::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, Theme,
    TypographyTokens,
};
use serde::Deserialize;

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
    /// The name Lua uses, in `gpui.set_theme("dark")`.
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    /// Parses a Lua-supplied name. `None` is a caller error, not a host bug, so
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

/// Installs the shell's default palette into `gpui_base::Theme`.
///
/// Called from `gpui_shell::init`. Re-installs the currently selected mode, so
/// calling it twice is harmless and a second call does not silently revert a
/// mode the application already chose.
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
/// Reads the ambient `App` through [`crate::scope::with_current_app`], so Lua
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
/// to Lua.
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
struct Palette {
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

#[derive(Debug, Deserialize)]
struct Palettes {
    light: Palette,
    dark: Palette,
}

impl Palettes {
    fn get(&self, mode: ThemeMode) -> &Palette {
        match mode {
            ThemeMode::Light => &self.light,
            ThemeMode::Dark => &self.dark,
        }
    }
}

static PALETTES: LazyLock<Palettes> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../theme/default-tokens.json"))
        .expect("theme/default-tokens.json is compiled into the binary, so a parse error here is a build-time mistake")
});

fn install(mode: ThemeMode, cx: &mut App) {
    let tokens = PALETTES.get(mode).tokens();
    CACHED.with(|cached| *cached.borrow_mut() = Some(tokens.clone()));
    Theme::global_mut(cx).tokens = tokens;
    cx.set_global(InstalledPalette { mode });
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

    #[test]
    fn embedded_palettes_parse() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let palette = PALETTES.get(mode);
            // Scales are omitted from the file, so they must fall back to the
            // Base defaults rather than to zero.
            assert_eq!(palette.radius, RadiusTokens::default());
            assert_eq!(palette.spacing, SpacingTokens::default());
        }
    }

    #[test]
    fn every_color_token_is_opaque_in_both_palettes() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let colors = PALETTES.get(mode).colors;
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
    fn color_token_names_match_the_token_fields() {
        let fields = field_names(&ColorTokens::default());
        assert_eq!(fields, sorted(color_token_names()));
        let colors = PALETTES.get(ThemeMode::Light).colors;
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
        assert!(resolve_color(&PALETTES.get(ThemeMode::Dark).colors, "backgrund").is_none());
        assert!(resolve_spacing(&SpacingTokens::default(), "xxxl").is_none());
        assert!(resolve_radius(&RadiusTokens::default(), "rounded").is_none());
    }

    #[test]
    fn token_lookup_is_none_outside_a_call_scope() {
        // No `CallScope` is open on a test thread, so there is no ambient `App`
        // and a Lua-facing lookup must decline rather than invent a color.
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
