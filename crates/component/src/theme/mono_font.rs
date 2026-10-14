//! Keeps the theme's default monospace family on a font the machine has,
//! preferring the one the user configured for the system.
//!
//! GPUI panics on the first line it lays out in a family it cannot find, and
//! `Font::fallbacks` does not help there: it is a cascade list for missing
//! glyphs, consulted only after the family itself has loaded. The theme's
//! platform default is a bare name, so a machine without that font would crash
//! on its first code block. This probe swaps the default for an installed
//! alternative before any text is laid out.
//!
//! On Linux the system's own monospace font is the first choice. Desktops
//! there configure it through fontconfig's `monospace` alias — the font the
//! terminal and editors use — and it varies far more than any default could
//! anticipate: Omarchy sets JetBrains Mono, GNOME Adwaita Mono, others DejaVu
//! Sans Mono. Only when the system names nothing installed do the platform
//! default and the fixed alternates apply.
//!
//! Only the platform default is probed. A family the application or a theme
//! file chose explicitly is used as-is: it is usually embedded through
//! `add_fonts`, and Windows lists families under their localized names, so an
//! English name for a CJK font would look missing when it is not.

use gpui::{App, SharedString};
use gpui_base::TypographyTokens;
use std::sync::OnceLock;

/// Monospace families tried, in order, when the platform default is missing.
const MONO_FONT_ALTERNATES: &[&str] = if cfg!(target_os = "macos") {
    &["Monaco", "Courier New"]
} else if cfg!(target_os = "windows") {
    &["Cascadia Mono", "Courier New"]
} else {
    &["Noto Sans Mono", "Liberation Mono", "Ubuntu Mono"]
};

/// The virtual family GPUI resolves on every platform; the last resort when
/// neither the default nor an alternate is installed.
const SYSTEM_UI_FONT: &str = ".SystemUIFont";

/// The monospace family the theme picks when none is configured.
pub(super) fn default_mono_font_family() -> SharedString {
    TypographyTokens::default().mono
}

/// Replaces the platform-default monospace family on the global theme with one
/// that is installed. A family chosen explicitly is left alone.
pub(super) fn resolve_default_mono_font(cx: &mut App) {
    if cx.global::<super::Theme>().mono_font_family != default_mono_font_family() {
        return;
    }
    let family = installed_default_mono_font_family(cx);
    cx.global_mut::<super::Theme>().mono_font_family = family;
}

/// The installed stand-in for the platform default, resolved once per process.
fn installed_default_mono_font_family(cx: &App) -> SharedString {
    static RESOLVED: OnceLock<SharedString> = OnceLock::new();
    RESOLVED
        .get_or_init(|| {
            let default = default_mono_font_family();
            let system = system_mono_font_family();
            let family = first_installed(
                system.as_deref(),
                &default,
                MONO_FONT_ALTERNATES,
                installed_font_names(cx),
            );
            // Following the system, or standing in for a default the machine
            // does not have, is the probe working; only running out of
            // monospace families altogether is worth a warning.
            if family == SYSTEM_UI_FONT {
                tracing::warn!("No monospace font is installed, using {SYSTEM_UI_FONT:?} instead.");
            } else if system.as_deref() == Some(family.as_ref()) {
                tracing::info!("Monospace font follows the system's {family:?}.");
            } else if family != default {
                tracing::info!(
                    "Monospace font {default:?} is not installed, using {family:?} instead."
                );
            }
            family
        })
        .clone()
}

/// The family the system's `monospace` alias names, as fontconfig resolves it.
///
/// Asked of `fc-match`, which ships with fontconfig wherever GPUI's Linux text
/// system finds its fonts, rather than through fontconfig's C API: this runs
/// once per process, and a missing tool only means falling back to the
/// alternates.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn system_mono_font_family() -> Option<String> {
    let output = std::process::Command::new("fc-match")
        .args(["--format=%{family[0]}", "monospace"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let family = String::from_utf8(output.stdout).ok()?;
    let family = family.trim();
    (!family.is_empty()).then(|| family.to_owned())
}

/// Other platforms have no user-configured monospace alias to follow.
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
fn system_mono_font_family() -> Option<String> {
    None
}

/// The families installed on the machine, listed once per process.
///
/// Enumerating fonts costs around a hundred milliseconds on macOS, and the
/// answer only depends on the system fonts, which do not change while the
/// process runs.
pub(super) fn installed_font_names(cx: &App) -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| cx.text_system().all_font_names())
}

/// The first installed of `system`, `default` and the alternates, else
/// `.SystemUIFont`.
fn first_installed(
    system: Option<&str>,
    default: &str,
    alternates: &[&str],
    installed: &[String],
) -> SharedString {
    system
        .into_iter()
        .chain(std::iter::once(default))
        .chain(alternates.iter().copied())
        .find(|candidate| installed.iter().any(|name| name == candidate))
        .unwrap_or(SYSTEM_UI_FONT)
        .to_string()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn keeps_the_default_when_it_is_installed() {
        let installed = names(&["Monaco", "Menlo", ".SystemUIFont"]);
        assert_eq!(
            first_installed(None, "Menlo", &["Monaco"], &installed),
            SharedString::from("Menlo")
        );
    }

    #[test]
    fn falls_through_the_alternates_in_order() {
        let installed = names(&["Courier New", "Monaco", ".SystemUIFont"]);
        assert_eq!(
            first_installed(None, "Menlo", &["Monaco", "Courier New"], &installed),
            SharedString::from("Monaco")
        );
    }

    #[test]
    fn falls_back_to_the_system_font_when_nothing_is_installed() {
        let installed = names(&["Helvetica", ".SystemUIFont"]);
        assert_eq!(
            first_installed(None, "Menlo", &["Monaco"], &installed),
            SharedString::from(".SystemUIFont")
        );
    }

    #[test]
    fn prefers_the_system_font_when_it_is_installed() {
        let installed = names(&["DejaVu Sans Mono", "JetBrainsMono Nerd Font"]);
        assert_eq!(
            first_installed(
                Some("JetBrainsMono Nerd Font"),
                "DejaVu Sans Mono",
                &["Noto Sans Mono"],
                &installed
            ),
            SharedString::from("JetBrainsMono Nerd Font")
        );
    }

    #[test]
    fn ignores_a_system_font_that_is_not_installed() {
        let installed = names(&["Noto Sans Mono"]);
        assert_eq!(
            first_installed(
                Some("Missing Mono"),
                "DejaVu Sans Mono",
                &["Noto Sans Mono"],
                &installed
            ),
            SharedString::from("Noto Sans Mono")
        );
    }
}
