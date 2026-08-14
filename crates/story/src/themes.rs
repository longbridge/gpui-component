use gpui::{Action, App, SharedString};
use gpui_component::{Theme, ThemeConfig, ThemeMode, ThemeRegistry, scroll::ScrollbarMode};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[cfg(not(target_family = "wasm"))]
use gpui_component::ActiveTheme;

#[cfg(target_family = "wasm")]
use crate::embedded_themes;

const STATE_FILE: &str = "target/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    theme: SharedString,
    #[serde(alias = "scrollbar_show")]
    scrollbar_mode: Option<ScrollbarMode>,
    #[serde(default)]
    show_fps_monitor: Option<bool>,
}

fn apply_theme_config(theme_config: std::rc::Rc<ThemeConfig>, cx: &mut App) {
    let mode = theme_config.mode;
    Theme::global_mut(cx).apply_config(&theme_config);
    Theme::change(mode, None, cx);
}

impl Default for State {
    fn default() -> Self {
        Self {
            theme: "Default Light".into(),
            scrollbar_mode: None,
            show_fps_monitor: None,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn save_state(cx: &mut App) {
    let state = State {
        theme: cx.theme().theme_name().clone(),
        scrollbar_mode: Some(cx.theme().scrollbar_mode),
        show_fps_monitor: Some(AppState::global(cx).show_fps_monitor),
    };

    if let Ok(json) = serde_json::to_string_pretty(&state) {
        // Ignore write errors - if STATE_FILE doesn't exist or can't be written, do nothing
        let _ = std::fs::write(STATE_FILE, json);
    }
}

pub fn init(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        tracing::info!("Loading embedded themes for WASM...");
        let embedded = embedded_themes::embedded_themes();
        let registry = ThemeRegistry::global_mut(cx);

        for (name, content) in embedded {
            if let Err(e) = registry.load_themes_from_str(content) {
                tracing::error!("Failed to load embedded theme {}: {}", name, e);
            } else {
                tracing::info!("Loaded embedded theme: {}", name);
            }
        }
    }

    let state = if cfg!(not(target_family = "wasm")) {
        let json = std::fs::read_to_string(STATE_FILE).unwrap_or(String::default());
        serde_json::from_str::<State>(&json).unwrap_or_default()
    } else {
        State::default()
    };

    #[cfg(not(target_family = "wasm"))]
    if let Err(err) =
        ThemeRegistry::watch_dir(std::path::PathBuf::from("./themes"), cx, move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx)
                .themes()
                .get(&state.theme)
                .cloned()
            {
                apply_theme_config(theme, cx);
            }
        })
    {
        tracing::error!("Failed to watch themes directory: {}", err);
    }

    if let Some(scrollbar_mode) = state.scrollbar_mode {
        Theme::set_scrollbar_mode(scrollbar_mode, cx);
    }
    if let Some(show_fps_monitor) = state.show_fps_monitor {
        AppState::global_mut(cx).show_fps_monitor = show_fps_monitor;
    }
    cx.refresh_windows();

    // Both globals carry persisted settings, so either changing writes the file.
    #[cfg(not(target_family = "wasm"))]
    {
        cx.observe_global::<Theme>(save_state).detach();
        cx.observe_global::<AppState>(save_state).detach();
    }

    cx.on_action(|switch: &SwitchTheme, cx| {
        let theme_name = switch.0.clone();
        if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            apply_theme_config(theme_config, cx);
        }
        cx.refresh_windows();
    });
    cx.on_action(|switch: &SwitchThemeMode, cx| {
        let mode = switch.0;
        Theme::change(mode, None, cx);
        cx.refresh_windows();
    });
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub(crate) struct SwitchTheme(pub(crate) SharedString);

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub(crate) struct SwitchThemeMode(pub(crate) ThemeMode);

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use gpui_component::ThemeConfig;
    use std::rc::Rc;

    #[gpui::test]
    fn applying_custom_theme_updates_base_component_colors(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let config = Rc::new(
            serde_json::from_value::<ThemeConfig>(serde_json::json!({
                "name": "Custom Dark",
                "mode": "dark",
                "colors": {
                    "popover.background": "#102030",
                    "popover.foreground": "#f0e0d0",
                    "border": "#405060",
                    "drag_border": "#708090",
                    "ring": "#a0b0c0"
                }
            }))
            .unwrap(),
        );

        cx.update(|cx| apply_theme_config(config, cx));

        cx.read(|cx| {
            let theme = cx.theme();
            let base = gpui_base::Theme::global(cx);
            assert_eq!(base.tokens.colors.surface, theme.popover);
            assert_eq!(
                base.tokens.colors.surface_foreground,
                theme.popover_foreground
            );
            assert_eq!(base.resizable.handle, theme.border);
            assert_eq!(base.resizable.active_handle, theme.drag_border);
            assert_eq!(base.tokens.colors.ring, theme.ring);
        });
    }
}
