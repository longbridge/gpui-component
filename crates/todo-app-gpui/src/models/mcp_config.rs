use gpui::SharedString;
use gpui_component::IconName;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env::home_dir};

use crate::models::{config_path, mcp_config_path};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum McpCapability {
    Resources,
    Tools,
    Prompts,
}

impl McpCapability {
    pub fn icon(&self) -> IconName {
        match self {
            McpCapability::Resources => IconName::Database,
            McpCapability::Tools => IconName::Wrench,
            McpCapability::Prompts => IconName::SquareTerminal,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            McpCapability::Resources => "资源",
            McpCapability::Tools => "工具",
            McpCapability::Prompts => "提示",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: Option<String>,
    pub subscribable: bool, // 是否支持订阅
    pub subscribed: bool,   // 当前是否已订阅
}

impl Default for McpResource {
    fn default() -> Self {
        Self {
            uri: String::new(),
            name: String::new(),
            description: String::new(),
            mime_type: None,
            subscribable: true,
            subscribed: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<McpParameter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpParameter {
    pub name: String,
    pub param_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpPrompt {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub arguments: Vec<McpArgument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpArgument {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpProviderInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub command: String,
    // #[serde(default)]
    // pub args: Vec<String>,
    #[serde(default)]
    pub transport: McpTransport,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub capabilities: Vec<McpCapability>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub resources: Vec<McpResource>,
    #[serde(default)]
    pub tools: Vec<McpTool>,
    #[serde(default)]
    pub prompts: Vec<McpPrompt>,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
}

impl Default for McpProviderInfo {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            command: String::new(),
            // args: Vec::new(),
            transport: McpTransport::Stdio,
            enabled: true,
            capabilities: vec![McpCapability::Resources, McpCapability::Tools],
            description: String::new(),
            resources: vec![
                McpResource {
                    uri: "file:///home/user/documents".to_string(),
                    name: "文档文件夹".to_string(),
                    description: "用户文档目录访问".to_string(),
                    mime_type: Some("inode/directory".to_string()),
                    subscribable: true,
                    subscribed: false,
                },
                McpResource {
                    uri: "file:///home/user/config.json".to_string(),
                    name: "配置文件".to_string(),
                    description: "应用配置文件".to_string(),
                    mime_type: Some("application/json".to_string()),
                    subscribable: true,
                    subscribed: false,
                },
            ],
            tools: vec![
                McpTool {
                    name: "read_file".to_string(),
                    description: "读取指定文件的内容".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "path".to_string(),
                            param_type: "string".to_string(),
                            description: "要读取的文件路径".to_string(),
                            required: true,
                        },
                        McpParameter {
                            name: "encoding".to_string(),
                            param_type: "string".to_string(),
                            description: "文件编码格式".to_string(),
                            required: false,
                        },
                    ],
                },
                McpTool {
                    name: "write_file".to_string(),
                    description: "写入内容到指定文件".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "path".to_string(),
                            param_type: "string".to_string(),
                            description: "目标文件路径".to_string(),
                            required: true,
                        },
                        McpParameter {
                            name: "content".to_string(),
                            param_type: "string".to_string(),
                            description: "要写入的内容".to_string(),
                            required: true,
                        },
                    ],
                },
            ],
            prompts: vec![
                McpPrompt {
                    name: "code_review".to_string(),
                    description: "对代码进行审查和建议".to_string(),
                    arguments: vec![
                        McpArgument {
                            name: "code".to_string(),
                            description: "要审查的代码内容".to_string(),
                            required: true,
                        },
                        McpArgument {
                            name: "language".to_string(),
                            description: "编程语言类型".to_string(),
                            required: false,
                        },
                    ],
                },
                McpPrompt {
                    name: "explain_concept".to_string(),
                    description: "解释技术概念".to_string(),
                    arguments: vec![McpArgument {
                        name: "concept".to_string(),
                        description: "要解释的概念".to_string(),
                        required: true,
                    }],
                },
            ],
            env_vars: std::collections::HashMap::from([
                (
                    "PATH".to_string(),
                    "/usr/local/bin:/usr/bin:/bin".to_string(),
                ),
                ("NODE_ENV".to_string(), "production".to_string()),
            ]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpProviderManager {
    #[serde(default)]
    pub providers: Vec<McpProviderInfo>,
}

impl McpProviderManager {
    /// 从文件加载配置
    pub fn load() -> Self {
        let config_path = mcp_config_path();
        if !config_path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(config_path) {
            Ok(content) => match serde_yaml::from_str::<Vec<McpProviderInfo>>(&content) {
                Ok(providers) => Self { providers },
                Err(e) => {
                    eprintln!("Failed to parse MCP config: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("Failed to read MCP config file: {}", e);
                Self::default()
            }
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = mcp_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(&self.providers)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 获取所有提供商列表
    pub fn list_providers(&self) -> Vec<McpProviderInfo> {
        self.providers.clone()
    }

    /// 根据ID查询提供商
    pub fn get_provider(&self, id: &str) -> Option<&McpProviderInfo> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// 根据名称查询提供商
    pub fn get_provider_by_name(&self, name: &str) -> Option<&McpProviderInfo> {
        self.providers.iter().find(|p| p.name == name)
    }

    /// 根据索引获取提供商
    pub fn get_provider_by_index(&self, index: usize) -> Option<&McpProviderInfo> {
        self.providers.get(index)
    }

    /// 根据ID查找提供商索引
    pub fn find_provider_index(&self, id: &str) -> Option<usize> {
        self.providers.iter().position(|p| p.id == id)
    }

    /// 添加新的提供商
    pub fn add_provider(&mut self, provider: McpProviderInfo) -> anyhow::Result<String> {
        if self.get_provider_by_name(&provider.name).is_some() {
            return Err(anyhow::anyhow!(
                "Provider '{}' already exists",
                provider.name
            ));
        }

        let id = provider.id.clone();
        self.providers.push(provider);
        Ok(id)
    }

    /// 更新提供商
    pub fn update_provider(&mut self, id: &str, provider: McpProviderInfo) -> anyhow::Result<()> {
        let index = self
            .find_provider_index(id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        // 检查名称冲突
        if let Some(existing) = self.get_provider_by_name(&provider.name) {
            if existing.id != id {
                return Err(anyhow::anyhow!(
                    "Provider name '{}' already exists",
                    provider.name
                ));
            }
        }

        self.providers[index] = provider;
        Ok(())
    }

    /// 根据索引更新提供商
    pub fn update_provider_by_index(
        &mut self,
        index: usize,
        provider: McpProviderInfo,
    ) -> anyhow::Result<()> {
        if index >= self.providers.len() {
            return Err(anyhow::anyhow!("Provider index {} out of bounds", index));
        }

        let old_id = &self.providers[index].id;

        // 检查名称冲突
        if let Some(existing) = self.get_provider_by_name(&provider.name) {
            if existing.id != *old_id {
                return Err(anyhow::anyhow!(
                    "Provider name '{}' already exists",
                    provider.name
                ));
            }
        }

        self.providers[index] = provider;
        Ok(())
    }

    /// 删除提供商
    pub fn delete_provider(&mut self, id: &str) -> anyhow::Result<McpProviderInfo> {
        let index = self
            .find_provider_index(id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        Ok(self.providers.remove(index))
    }

    /// 根据索引删除提供商
    pub fn delete_provider_by_index(&mut self, index: usize) -> anyhow::Result<McpProviderInfo> {
        if index >= self.providers.len() {
            return Err(anyhow::anyhow!("Provider index {} out of bounds", index));
        }

        Ok(self.providers.remove(index))
    }

    /// 启用/禁用提供商
    pub fn toggle_provider(&mut self, id: &str, enabled: bool) -> anyhow::Result<()> {
        let provider = self
            .providers
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider with id '{}' not found", id))?;

        provider.enabled = enabled;
        Ok(())
    }

    /// 根据索引启用/禁用提供商
    pub fn toggle_provider_by_index(&mut self, index: usize, enabled: bool) -> anyhow::Result<()> {
        if index >= self.providers.len() {
            return Err(anyhow::anyhow!("Provider index {} out of bounds", index));
        }

        self.providers[index].enabled = enabled;
        Ok(())
    }

    /// 获取启用的提供商
    pub fn get_enabled_providers(&self) -> Vec<&McpProviderInfo> {
        self.providers
            .iter()
            .filter(|provider| provider.enabled)
            .collect()
    }

    /// 获取提供商数量
    pub fn count(&self) -> usize {
        self.providers.len()
    }

    /// 清空所有提供商
    pub fn clear(&mut self) {
        self.providers.clear();
    }

    /// 批量删除提供商
    pub fn batch_delete(&mut self, ids: &[String]) -> Vec<McpProviderInfo> {
        let mut deleted = Vec::new();

        // 从后往前删除，避免索引变化
        for id in ids {
            if let Some(index) = self.find_provider_index(id) {
                deleted.push(self.providers.remove(index));
            }
        }

        deleted
    }

    /// 根据索引批量删除提供商
    pub fn batch_delete_by_indices(&mut self, mut indices: Vec<usize>) -> Vec<McpProviderInfo> {
        let mut deleted = Vec::new();

        // 从大到小排序索引，从后往前删除
        indices.sort_by(|a, b| b.cmp(a));

        for index in indices {
            if index < self.providers.len() {
                deleted.push(self.providers.remove(index));
            }
        }

        deleted.reverse(); // 恢复原始顺序
        deleted
    }

    /// 搜索提供商
    pub fn search_providers(&self, query: &str) -> Vec<&McpProviderInfo> {
        let query_lower = query.to_lowercase();
        self.providers
            .iter()
            .filter(|provider| {
                provider.name.to_lowercase().contains(&query_lower)
                    || provider.command.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// 移动提供商位置
    pub fn move_provider(&mut self, from_index: usize, to_index: usize) -> anyhow::Result<()> {
        if from_index >= self.providers.len() || to_index >= self.providers.len() {
            return Err(anyhow::anyhow!("Index out of bounds"));
        }

        if from_index != to_index {
            let provider = self.providers.remove(from_index);
            self.providers.insert(to_index, provider);
        }

        Ok(())
    }

    /// 交换两个提供商的位置
    pub fn swap_providers(&mut self, index1: usize, index2: usize) -> anyhow::Result<()> {
        if index1 >= self.providers.len() || index2 >= self.providers.len() {
            return Err(anyhow::anyhow!("Index out of bounds"));
        }

        self.providers.swap(index1, index2);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
}
