use std::path::PathBuf;

use gpui::{
    div, px, Action, App, InteractiveElement as _, ParentElement as _, Render, SharedString,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    popup_menu::PopupMenuExt,
    IconName, Sizable, Theme, ThemeRegistry,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

const STATE_FILE: &str = "target/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    theme: SharedString,
}

pub fn init(cx: &mut App) {
    // Load last theme state
    let json = std::fs::read_to_string(STATE_FILE).unwrap_or(String::default());
    tracing::info!("Load themes...");
    if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
        if let Ok(state) = serde_json::from_str::<State>(&json) {
            tracing::info!("apply theme: {:?}", state.theme);
            AppState::global_mut(cx).theme_name = Some(state.theme.clone());

            if let Some(theme) = ThemeRegistry::global(cx)
                .themes()
                .get(&state.theme)
                .cloned()
            {
                Theme::global_mut(cx).apply_config(&theme);
            }
        }
    }) {
        tracing::error!("Failed to watch themes directory: {}", err);
    }
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
struct SwitchTheme(SharedString);

pub struct ThemeSwitcher {
    current_theme_name: SharedString,
}

impl ThemeSwitcher {
    pub fn new(cx: &mut App) -> Self {
        let theme_name = AppState::global(cx)
            .theme_name
            .clone()
            .unwrap_or("default-light".into());

        Self {
            current_theme_name: theme_name,
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
                if let Some(theme_config) =
                    ThemeRegistry::global(cx).themes().get(&theme_name).cloned()
                {
                    Theme::global_mut(cx).apply_config(&theme_config);
                }

                // Save AppState
                let state = State {
                    theme: theme_name.clone(),
                };
                AppState::global_mut(cx).theme_name = Some(theme_name.clone());
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
                        move |menu, _, cx| {
                            let mut menu = menu.scrollable().max_h(px(600.));

                            let names = ThemeRegistry::global(cx)
                                .sorted_themes()
                                .iter()
                                .map(|theme| theme.name.clone())
                                .collect::<Vec<SharedString>>();

                            for theme_name in names {
                                let is_selected = theme_name == current_theme_id;
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
