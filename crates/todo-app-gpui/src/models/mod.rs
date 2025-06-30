pub mod mcp_config;
pub mod profile_config;
pub mod prompts;
pub mod provider_config;
pub mod todo_item;

use std::{env::home_dir, path::PathBuf};

const MCP_CONFIG_FILE: &str = "config/mcp_providers.yml";
const PROFILE_CONFIG_FILE: &str = "config/profile.yml";
const CONFIG_FILE: &str = "config/llm_providers.yml";
const TODO_CONFIG_FILE: &str = "config/todos.yml";

pub fn home() -> PathBuf {
    let home = home_dir().unwrap().join(".xTo-Do");
    std::fs::create_dir_all(&home).ok(); // 确保目录存在
    home
}

pub fn config_path(file: &str) -> PathBuf {
    home().join(file)
}

pub fn mcp_config_path() -> PathBuf {
    config_path(MCP_CONFIG_FILE)
}
pub fn profile_config_path() -> PathBuf {
    config_path(PROFILE_CONFIG_FILE)
}
pub fn provider_config_path() -> PathBuf {
    config_path(CONFIG_FILE)
}
pub fn todo_config_path() -> PathBuf {
    config_path(TODO_CONFIG_FILE)
}
