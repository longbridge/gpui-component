use crate::config::mcp_config_path;
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub enum McpTransport {
    #[default]
    Stdio,
    Sse,
    Streamable,
}

impl McpTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpTransport::Stdio => "Stdio",
            McpTransport::Sse => "Sse",
            McpTransport::Streamable => "Streamable",
        }
    }

    pub fn all() -> Vec<SharedString> {
        vec!["Stdio".into(), "Sse".into(), "Streamable".into()]
    }
}

// 纯配置信息
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub transport: McpTransport,
    pub enabled: bool,
    pub description: String,
    pub env_vars: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct McpConfigManager;

impl McpConfigManager {
    pub fn config_path() -> PathBuf {
        mcp_config_path()
    }
    /// 从文件加载所有提供商配置
    pub fn load_servers() -> anyhow::Result<Vec<McpServerConfig>> {
        let config_path = mcp_config_path();
        if !config_path.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(config_path)?;
        let providers = serde_yaml::from_str::<Vec<McpServerConfig>>(&content)?;
        Ok(providers)
    }

    /// 保存所有提供商配置到文件
    pub fn save_servers(servers: &[McpServerConfig]) -> anyhow::Result<()> {
        let config_path = mcp_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(servers)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 根据ID查询单个提供商配置
    pub fn get_server(id: &str) -> anyhow::Result<Option<McpServerConfig>> {
        let providers = Self::load_servers()?;
        Ok(providers.into_iter().find(|p| p.id == id))
    }

    /// 添加新的提供商配置
    pub fn add_server(server: McpServerConfig) -> anyhow::Result<()> {
        let mut providers = Self::load_servers()?;
        providers.push(server);
        Self::save_servers(&providers)
    }

    /// 更新提供商配置
    pub fn update_server(id: &str, server: McpServerConfig) -> anyhow::Result<bool> {
        let mut providers = Self::load_servers()?;

        if let Some(provider) = providers.iter_mut().find(|p| p.id == id) {
            *provider = server;
            Self::save_servers(&providers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 删除提供商配置
    pub fn remove_server(id: &str) -> anyhow::Result<bool> {
        let mut providers = Self::load_servers()?;
        let original_len = providers.len();

        providers.retain(|p| p.id != id);

        if providers.len() != original_len {
            Self::save_servers(&providers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 启用/禁用提供商
    pub fn set_server_enabled(id: &str, enabled: bool) -> anyhow::Result<bool> {
        let mut providers = Self::load_servers()?;

        if let Some(provider) = providers.iter_mut().find(|p| p.id == id) {
            provider.enabled = enabled;
            Self::save_servers(&providers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取所有启用的提供商
    pub fn get_enabled_servers() -> anyhow::Result<Vec<McpServerConfig>> {
        let providers = Self::load_servers()?;
        Ok(providers.into_iter().filter(|p| p.enabled).collect())
    }
}
