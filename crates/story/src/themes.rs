use std::path::PathBuf;

use gpui::{Action, App, SharedString};
use gpui_component::{ActiveTheme, Theme, ThemeMode, ThemeRegistry, scroll::ScrollbarShow};
use serde::{Deserialize, Serialize};

const STATE_FILE: &str = "target/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
  theme: SharedString,
  scrollbar_show: Option<ScrollbarShow>,
}

impl Default for State {
  fn default() -> Self {
    Self {
      theme: "Default Light".into(),
      scrollbar_show: None,
    }
  }
}

pub fn init(cx: &mut App) {
  // Load last theme state
  let json = std::fs::read_to_string(STATE_FILE).unwrap_or(String::default());
  tracing::info!("Load themes...");
  let state = serde_json::from_str::<State>(&json).unwrap_or_default();
  if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
    if let Some(theme) = ThemeRegistry::global(cx)
      .themes()
      .get(&state.theme)
      .cloned()
    {
      Theme::global_mut(cx).apply_config(&theme);
    }
  }) {
    tracing::error!("Failed to watch themes directory: {}", err);
  }

  if let Some(scrollbar_show) = state.scrollbar_show {
    Theme::global_mut(cx).scrollbar_show = scrollbar_show;
  }
  cx.refresh_windows();

  cx.observe_global::<Theme>(|cx| {
    let state = State {
      theme: cx.theme().theme_name().clone(),
      scrollbar_show: Some(cx.theme().scrollbar_show),
    };

    if let Ok(json) = serde_json::to_string_pretty(&state) {
      // Ignore write errors - if STATE_FILE doesn't exist or can't be written, do nothing
      let _ = std::fs::write(STATE_FILE, json);
    }
  })
  .detach();

  cx.on_action(|switch: &SwitchTheme, cx| {
    let theme_name = switch.0.clone();
    if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
      Theme::global_mut(cx).apply_config(&theme_config);
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
