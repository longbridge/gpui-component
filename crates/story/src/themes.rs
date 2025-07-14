use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use gpui::{div, Action, App, InteractiveElement as _, ParentElement as _, Render, SharedString};
use gpui_component::{
    button::{Button, ButtonVariants},
    highlighter::HighlightTheme,
    popup_menu::PopupMenuExt,
    IconName, Sizable, Theme, ThemeColor, ThemeConfig,
};
use serde::{Deserialize, Serialize};

const STATE_FILE: &str = "target/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppState {
    theme: SharedString,
}

pub fn init(cx: &mut App) {
    // Load last theme state
    let json = std::fs::read_to_string(STATE_FILE).unwrap_or(String::default());
    if let Ok(state) = serde_json::from_str::<AppState>(&json) {
        tracing::info!("apply theme: {:?}", state.theme);
        if let Some(theme) = THEMES.get(&state.theme) {
            Theme::global_mut(cx).apply_config(theme);
        }
    }
}

static THEMES: LazyLock<HashMap<SharedString, ThemeConfig>> = LazyLock::new(|| {
    fn parse_themes(source: &str) -> Vec<ThemeConfig> {
        serde_json::from_str(source).unwrap()
    }

    let mut themes = HashMap::new();
    for source in [
        include_str!("./themes/adventure.json"),
        include_str!("./themes/ayu.json"),
        include_str!("./themes/catppuccin.json"),
        include_str!("./themes/macos-classic.json"),
        include_str!("./themes/solarized.json"),
        include_str!("./themes/tokyonight.json"),
    ] {
        for sub_theme in parse_themes(source) {
            themes.insert(sub_theme.name.clone(), sub_theme);
        }
    }
    themes
});

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
struct SwitchTheme(SharedString);

pub struct ThemeSwitcher {
    current_theme_name: SharedString,
}

impl ThemeSwitcher {
    pub fn new() -> Self {
        Self {
            current_theme_name: "default-light".into(),
        }
    }
}

impl Render for ThemeSwitcher {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .id("theme-switcher")
            .on_action(cx.listener(|this, switch: &SwitchTheme, _, cx| {
                this.current_theme_name = switch.0.clone();
                let theme_name = this.current_theme_name.clone();

                if let Some(theme_config) = THEMES.get(&theme_name) {
                    Theme::global_mut(cx).apply_config(theme_config);
                } else if theme_name == "default-light" {
                    Theme::global_mut(cx).set_default_light();
                } else if theme_name == "default-dark" {
                    Theme::global_mut(cx).set_default_dark();
                }

                // Save AppState
                let state = AppState {
                    theme: theme_name.clone(),
                };
                let json = serde_json::to_string_pretty(&state).unwrap();
                std::fs::write(STATE_FILE, json).unwrap();

                cx.notify();
            }))
            .child(
                Button::new("btn")
                    .icon(IconName::Palette)
                    .ghost()
                    .small()
                    .popup_menu({
                        let current_theme_id = self.current_theme_name.clone();
                        move |menu, _, _| {
                            let mut menu = menu
                                .menu_with_check(
                                    "Default Light",
                                    current_theme_id == "default-light",
                                    Box::new(SwitchTheme("default-light".into())),
                                )
                                .menu_with_check(
                                    "Default Dark",
                                    current_theme_id == "default-dark",
                                    Box::new(SwitchTheme("default-dark".into())),
                                );

                            let mut names = THEMES.keys().collect::<Vec<&SharedString>>();
                            names.sort();

                            for theme_name in names {
                                let is_selected = *theme_name == current_theme_id;
                                menu = menu.menu_with_check(
                                    theme_name.clone(),
                                    is_selected,
                                    Box::new(SwitchTheme(theme_name.clone())),
                                );
                            }

                            menu
                        }
                    }),
            )
    }
}
