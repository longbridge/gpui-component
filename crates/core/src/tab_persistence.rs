use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use gpui::{App, AsyncApp, Entity, Task, Window};

use crate::storage::get_config_dir;
use crate::tab_container::{TabContainer, TabContainerState, TabContentRegistry};

const TAB_STATE_FILE: &str = "tab_state.json";
const SAVE_DELAY_SECS: u64 = 5;

fn get_tab_state_path() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir.join(TAB_STATE_FILE))
}

pub fn save_tab_state(state: &TabContainerState) -> Result<()> {
    let path = get_tab_state_path()?;
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    tracing::info!("Tab state saved to {:?}", path);
    Ok(())
}

pub fn load_tab_state() -> Result<TabContainerState> {
    let path = get_tab_state_path()?;
    if !path.exists() {
        return Ok(TabContainerState::default());
    }
    let json = std::fs::read_to_string(&path).context("Failed to read tab state file")?;
    let state = serde_json::from_str(&json).context("Failed to parse tab state JSON")?;
    tracing::info!("Tab state loaded from {:?}", path);
    Ok(state)
}

pub fn tab_state_exists() -> bool {
    get_tab_state_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn load_tabs(
    tab_container: &Entity<TabContainer>,
    registry: &TabContentRegistry,
    window: &mut Window,
    cx: &mut App,
) -> Result<()> {
    let state = load_tab_state()?;

    if state.tabs.is_empty() {
        tracing::info!("Saved tab state is empty");
        return Ok(());
    }

    tab_container.update(cx, |container, cx| {
        container.load(state, registry, window, cx);
    });

    tracing::info!("Tabs restored from saved state");
    Ok(())
}

pub fn schedule_save(
    tab_container: Entity<TabContainer>,
    last_layout_state: &mut Option<TabContainerState>,
    cx: &mut App,
) -> Task<()> {
    let last_state = last_layout_state.clone();

    cx.spawn(async move |cx: &mut AsyncApp| {
        cx.background_executor()
            .timer(Duration::from_secs(SAVE_DELAY_SECS))
            .await;

        if let Some(t) = cx.update(move |cx| {
            let current_state = tab_container.read(cx).dump(cx);

            if Some(&current_state) == last_state.as_ref() {
                tracing::debug!("Tab state unchanged, skipping save");
                return None;
            }

            if let Err(err) = save_tab_state(&current_state) {
                tracing::error!("Failed to save tab state: {:?}", err);
            }

            Some(current_state)
        }) {
            tracing::info!("Tab state saved, {:?}", t)
        }
    })
}
