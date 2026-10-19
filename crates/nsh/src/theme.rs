//! GPUI Kit's theme as NSH's, so embedded apps follow the host.
//!
//! NSH's theme has GPUI Kit's color names and JSON format: both modes come
//! from the host's theme configs (colors a config leaves out are null, and
//! NSH keeps its own), then the current mode's colors are taken as GPUI Kit
//! resolved them, so NSH's components match the host's.

use gpui_kit::{
    Hsla,
    component::{Theme, ThemeMode},
};
use nsh_platform_gpui::{AdaptiveColor, Appearance, Color};

/// A GPUI color as NSH's (`0xRRGGBBAA`).
fn color(c: Hsla) -> Color {
    let rgba = c.to_rgb();
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    Color(byte(rgba.r) << 24 | byte(rgba.g) << 16 | byte(rgba.b) << 8 | byte(rgba.a))
}

/// The host's light or dark mode.
pub(crate) fn appearance(mode: ThemeMode) -> Appearance {
    if mode.is_dark() {
        Appearance::Dark
    } else {
        Appearance::Light
    }
}

/// `theme`, GPUI Kit's active theme, as NSH's.
pub(crate) fn nsh_theme(theme: &Theme) -> nsh_platform_gpui::Theme {
    let configs = serde_json::json!({ "themes": [&*theme.light_theme, &*theme.dark_theme] });
    let mut nsh = nsh_platform_gpui::Theme::from_json(&configs.to_string()).unwrap_or_else(|err| {
        log::warn!("the host's theme configs: {err}");
        nsh_platform_gpui::Theme::default()
    });
    let dark = theme.mode.is_dark();
    let set = |field: &mut AdaptiveColor, c: Hsla| {
        if dark {
            field.dark = color(c);
        } else {
            field.light = color(c);
        }
    };
    // Every color of NSH's theme has a field of the same name in GPUI Kit's.
    macro_rules! resolved {
        ($($field:ident),* $(,)?) => {
            $( set(&mut nsh.colors.$field, theme.colors.$field); )*
        };
    }
    resolved!(
        accent,
        accent_foreground,
        background,
        foreground,
        border,
        caret,
        danger,
        danger_active,
        danger_foreground,
        group_box,
        group_box_foreground,
        info,
        info_active,
        info_foreground,
        input,
        link,
        link_active,
        list,
        list_active,
        list_active_border,
        list_even,
        list_head,
        muted,
        muted_foreground,
        overlay,
        popover,
        popover_foreground,
        primary,
        primary_active,
        primary_foreground,
        progress_bar,
        ring,
        secondary,
        secondary_active,
        secondary_foreground,
        selection,
        skeleton,
        slider_bar,
        slider_thumb,
        success,
        success_active,
        success_foreground,
        switch,
        switch_thumb,
        tab,
        tab_active,
        tab_active_foreground,
        tab_bar,
        tab_bar_segmented,
        tab_foreground,
        warning,
        warning_active,
        warning_foreground,
    );
    nsh.radius = f32::from(theme.radius);
    nsh.radius_lg = f32::from(theme.radius_lg);
    nsh.font_size = f32::from(theme.font_size);
    nsh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_rgba() {
        assert_eq!(color(gpui_kit::rgb(0x336699).into()), Color(0x336699ff));
        assert_eq!(color(gpui_kit::transparent_black()), Color(0x0000_0000));
    }

    #[test]
    fn test_current_mode_colors_win() {
        let mut theme = Theme {
            mode: ThemeMode::Dark,
            radius: gpui_kit::px(6.0),
            ..Default::default()
        };
        theme.colors.primary = gpui_kit::rgb(0x7c3aed).into();
        let nsh = nsh_theme(&theme);
        assert_eq!(nsh.primary.dark, Color(0x7c3aedff));
        assert_eq!(
            nsh.primary.light,
            nsh_platform_gpui::Theme::default().primary.light
        );
        assert_eq!(nsh.radius, 6.0);
    }
}
