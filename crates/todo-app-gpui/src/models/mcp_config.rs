use gpui::SharedString;
use gpui_component::IconName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio,
    Http,
    WebSocket,
    Sse,
}

impl McpTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpTransport::Stdio => "Stdio",
            McpTransport::Http => "HTTP",
            McpTransport::WebSocket => "WebSocket",
            McpTransport::Sse => "SSE",
        }
    }

    pub fn all() -> Vec<SharedString> {
        vec![
            "Stdio".into(),
            "HTTP".into(),
            "WebSocket".into(),
            "SSE".into(),
        ]
    }

    pub fn icon(&self) -> IconName {
        match self {
            McpTransport::Stdio => IconName::Terminal,
            McpTransport::Http => IconName::Globe,
            McpTransport::WebSocket => IconName::Wifi,
            McpTransport::Sse => IconName::Radio,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpCapability {
    Resources,
    Tools,
    Prompts,
    Logging,
    Sampling,
    Completion,
    RootsListChanged,
}

impl McpCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpCapability::Resources => "资源",
            McpCapability::Tools => "工具",
            McpCapability::Prompts => "提示",
            McpCapability::Logging => "日志",
            McpCapability::Sampling => "采样",
            McpCapability::Completion => "补全",
            McpCapability::RootsListChanged => "根目录变更",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            McpCapability::Resources => IconName::Database,
            McpCapability::Tools => IconName::Wrench,
            McpCapability::Prompts => IconName::SquareTerminal,
            McpCapability::Logging => IconName::LetterText,
            McpCapability::Sampling => IconName::Shuffle,
            McpCapability::Completion => IconName::Sparkles,
            McpCapability::RootsListChanged => IconName::FolderTree,
        }
    }

    pub fn color(&self) -> gpui::Rgba {
        match self {
            McpCapability::Resources => gpui::rgb(0x059669),
            McpCapability::Tools => gpui::rgb(0xDC2626),
            McpCapability::Prompts => gpui::rgb(0x7C3AED),
            McpCapability::Logging => gpui::rgb(0xF59E0B),
            McpCapability::Sampling => gpui::rgb(0x06B6D4),
            McpCapability::Completion => gpui::rgb(0xEC4899),
            McpCapability::RootsListChanged => gpui::rgb(0x10B981),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            McpCapability::Resources,
            McpCapability::Tools,
            McpCapability::Prompts,
            McpCapability::Logging,
            McpCapability::Sampling,
            McpCapability::Completion,
            McpCapability::RootsListChanged,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

impl Default for McpParameter {
    fn default() -> Self {
        Self {
            name: String::new(),
            param_type: "string".to_string(),
            description: String::new(),
            required: true,
            default_value: None,
            enum_values: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<McpParameter>,
    pub enabled: bool,
}

impl Default for McpTool {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            parameters: vec![],
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

impl Default for McpArgument {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            required: true,
            default_value: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<McpArgument>,
    pub enabled: bool,
}

impl Default for McpPrompt {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            arguments: vec![],
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: Option<String>,
    pub subscribable: bool,
    pub subscribed: bool,
    pub annotations: HashMap<String, serde_json::Value>,
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
            annotations: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub transport: McpTransport,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: HashMap<String, String>,
    pub capabilities: Vec<McpCapability>,
    pub enabled: bool,
    pub auto_start: bool,
    pub restart_on_failure: bool,
    pub max_restarts: Option<u32>,
    pub timeout: Option<u32>,
    pub tools: Vec<McpTool>,
    pub prompts: Vec<McpPrompt>,
    pub resources: Vec<McpResource>,
    // HTTP/WebSocket specific
    pub base_url: Option<String>,
    pub auth_headers: HashMap<String, String>,
    // Logging
    pub log_level: String,
    pub log_file: Option<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            display_name: String::new(),
            description: String::new(),
            version: "1.0.0".to_string(),
            transport: McpTransport::Stdio,
            command: String::new(),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            capabilities: vec![McpCapability::Resources, McpCapability::Tools],
            enabled: true,
            auto_start: true,
            restart_on_failure: true,
            max_restarts: Some(3),
            timeout: Some(30),
            tools: vec![],
            prompts: vec![],
            resources: vec![],
            base_url: None,
            auth_headers: HashMap::new(),
            log_level: "INFO".to_string(),
            log_file: None,
        }
    }
}

impl McpServerConfig {
    // 预设的常用 MCP 服务器配置
    pub fn filesystem() -> Self {
        Self {
            id: "filesystem".to_string(),
            name: "filesystem".to_string(),
            display_name: "文件系统".to_string(),
            description: "提供文件系统访问功能的MCP服务器".to_string(),
            transport: McpTransport::Stdio,
            command: "node".to_string(),
            args: vec!["filesystem-server.js".to_string()],
            capabilities: vec![
                McpCapability::Resources,
                McpCapability::Tools,
                McpCapability::RootsListChanged,
            ],
            tools: vec![
                McpTool {
                    name: "read_file".to_string(),
                    description: "读取文件内容".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "path".to_string(),
                            param_type: "string".to_string(),
                            description: "文件路径".to_string(),
                            required: true,
                            ..Default::default()
                        }
                    ],
                    enabled: true,
                },
                McpTool {
                    name: "write_file".to_string(),
                    description: "写入文件内容".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "path".to_string(),
                            param_type: "string".to_string(),
                            description: "文件路径".to_string(),
                            required: true,
                            ..Default::default()
                        },
                        McpParameter {
                            name: "content".to_string(),
                            param_type: "string".to_string(),
                            description: "文件内容".to_string(),
                            required: true,
                            ..Default::default()
                        },
                    ],
                    enabled: true,
                },
            ],
            resources: vec![
                McpResource {
                    uri: "file:///".to_string(),
                    name: "根目录".to_string(),
                    description: "系统根目录访问".to_string(),
                    mime_type: Some("inode/directory".to_string()),
                    subscribable: true,
                    subscribed: false,
                    ..Default::default()
                }
            ],
            ..Default::default()
        }
    }

    pub fn database() -> Self {
        Self {
            id: "database".to_string(),
            name: "database".to_string(),
            display_name: "数据库连接".to_string(),
            description: "提供数据库查询和操作功能".to_string(),
            transport: McpTransport::Stdio,
            command: "python".to_string(),
            args: vec!["-m".to_string(), "mcp_database".to_string()],
            env: HashMap::from([
                ("DB_HOST".to_string(), "localhost".to_string()),
                ("DB_PORT".to_string(), "5432".to_string()),
            ]),
            capabilities: vec![
                McpCapability::Tools,
                McpCapability::Resources,
                McpCapability::Logging,
            ],
            tools: vec![
                McpTool {
                    name: "execute_query".to_string(),
                    description: "执行SQL查询".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "sql".to_string(),
                            param_type: "string".to_string(),
                            description: "SQL查询语句".to_string(),
                            required: true,
                            ..Default::default()
                        },
                        McpParameter {
                            name: "params".to_string(),
                            param_type: "array".to_string(),
                            description: "查询参数".to_string(),
                            required: false,
                            ..Default::default()
                        },
                    ],
                    enabled: true,
                },
            ],
            resources: vec![
                McpResource {
                    uri: "db://tables".to_string(),
                    name: "数据库表".to_string(),
                    description: "数据库表列表".to_string(),
                    mime_type: Some("application/json".to_string()),
                    subscribable: true,
                    subscribed: false,
                    ..Default::default()
                }
            ],
            ..Default::default()
        }
    }

    pub fn web_scraper() -> Self {
        Self {
            id: "web-scraper".to_string(),
            name: "web-scraper".to_string(),
            display_name: "网页抓取".to_string(),
            description: "提供网页内容抓取和解析功能".to_string(),
            transport: McpTransport::Http,
            base_url: Some("http://localhost:8080".to_string()),
            capabilities: vec![
                McpCapability::Tools,
                McpCapability::Resources,
            ],
            tools: vec![
                McpTool {
                    name: "scrape_url".to_string(),
                    description: "抓取网页内容".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "url".to_string(),
                            param_type: "string".to_string(),
                            description: "要抓取的网页URL".to_string(),
                            required: true,
                            ..Default::default()
                        },
                        McpParameter {
                            name: "selector".to_string(),
                            param_type: "string".to_string(),
                            description: "CSS选择器".to_string(),
                            required: false,
                            ..Default::default()
                        },
                    ],
                    enabled: true,
                },
            ],
            auth_headers: HashMap::from([
                ("User-Agent".to_string(), "MCP-WebScraper/1.0".to_string()),
            ]),
            ..Default::default()
        }
    }

    pub fn git() -> Self {
        Self {
            id: "git".to_string(),
            name: "git".to_string(),
            display_name: "Git 版本控制".to_string(),
            description: "提供Git版本控制操作功能".to_string(),
            transport: McpTransport::Stdio,
            command: "python".to_string(),
            args: vec!["-m".to_string(), "mcp_git".to_string()],
            capabilities: vec![
                McpCapability::Tools,
                McpCapability::Resources,
                McpCapability::Prompts,
            ],
            tools: vec![
                McpTool {
                    name: "git_status".to_string(),
                    description: "获取Git状态".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "repo_path".to_string(),
                            param_type: "string".to_string(),
                            description: "仓库路径".to_string(),
                            required: false,
                            default_value: Some(".".to_string()),
                            ..Default::default()
                        }
                    ],
                    enabled: true,
                },
                McpTool {
                    name: "git_commit".to_string(),
                    description: "提交更改".to_string(),
                    parameters: vec![
                        McpParameter {
                            name: "message".to_string(),
                            param_type: "string".to_string(),
                            description: "提交信息".to_string(),
                            required: true,
                            ..Default::default()
                        },
                        McpParameter {
                            name: "files".to_string(),
                            param_type: "array".to_string(),
                            description: "要提交的文件列表".to_string(),
                            required: false,
                            ..Default::default()
                        },
                    ],
                    enabled: true,
                },
            ],
            prompts: vec![
                McpPrompt {
                    name: "commit_message".to_string(),
                    description: "生成提交信息".to_string(),
                    arguments: vec![
                        McpArgument {
                            name: "changes".to_string(),
                            description: "更改描述".to_string(),
                            required: true,
                            ..Default::default()
                        }
                    ],
                    enabled: true,
                },
            ],
            resources: vec![
                McpResource {
                    uri: "git://status".to_string(),
                    name: "Git状态".to_string(),
                    description: "当前仓库的Git状态".to_string(),
                    mime_type: Some("application/json".to_string()),
                    subscribable: true,
                    subscribed: false,
                    ..Default::default()
                }
            ],
            ..Default::default()
        }
    }

    // 保存为YAML文件
    pub fn save_to_yaml(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    // 从YAML文件加载
    pub fn load_from_yaml(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    // 批量保存多个配置
    pub fn save_configs_to_yaml(
        configs: &[Self], 
        dir_path: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(dir_path)?;
        
        for config in configs {
            let file_path = format!("{}/{}.yaml", dir_path, config.name);
            config.save_to_yaml(&file_path)?;
        }
        Ok(())
    }

    // 从目录加载所有配置
    pub fn load_configs_from_dir(
        dir_path: &str
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut configs = Vec::new();
        
        if !std::path::Path::new(dir_path).exists() {
            return Ok(configs);
        }

        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") 
                || path.extension().and_then(|s| s.to_str()) == Some("yml") {
                if let Ok(config) = Self::load_from_yaml(path.to_str().unwrap()) {
                    configs.push(config);
                }
            }
        }
        
        Ok(configs)
    }

    // 验证配置
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("服务器名称不能为空".to_string());
        }

        if self.display_name.is_empty() {
            errors.push("显示名称不能为空".to_string());
        }

        match self.transport {
            McpTransport::Stdio => {
                if self.command.is_empty() {
                    errors.push("Stdio传输方式需要指定command".to_string());
                }
            }
            McpTransport::Http | McpTransport::WebSocket => {
                if self.base_url.is_none() {
                    errors.push("HTTP/WebSocket传输方式需要指定base_url".to_string());
                }
            }
            McpTransport::Sse => {
                if self.base_url.is_none() {
                    errors.push("SSE传输方式需要指定base_url".to_string());
                }
            }
        }

        // 验证工具参数
        for tool in &self.tools {
            if tool.name.is_empty() {
                errors.push(format!("工具名称不能为空"));
            }
            
            for param in &tool.parameters {
                if param.name.is_empty() {
                    errors.push(format!("工具 '{}' 的参数名称不能为空", tool.name));
                }
            }
        }

        // 验证提示参数
        for prompt in &self.prompts {
            if prompt.name.is_empty() {
                errors.push("提示名称不能为空".to_string());
            }
            
            for arg in &prompt.arguments {
                if arg.name.is_empty() {
                    errors.push(format!("提示 '{}' 的参数名称不能为空", prompt.name));
                }
            }
        }

        // 验证资源
        for resource in &self.resources {
            if resource.name.is_empty() {
                errors.push("资源名称不能为空".to_string());
            }
            if resource.uri.is_empty() {
                errors.push(format!("资源 '{}' 的URI不能为空", resource.name));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // 获取所有预设配置
    pub fn get_presets() -> Vec<Self> {
        vec![
            Self::filesystem(),
            Self::database(),
            Self::web_scraper(),
            Self::git(),
        ]
    }
}

// 配置管理器
#[derive(Debug)]
pub struct McpConfigManager {
    config_dir: String,
    configs: Vec<McpServerConfig>,
}

impl McpConfigManager {
    pub fn new(config_dir: &str) -> Self {
        Self {
            config_dir: config_dir.to_string(),
            configs: Vec::new(),
        }
    }

    pub fn load_configs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.configs = McpServerConfig::load_configs_from_dir(&self.config_dir)?;
        Ok(())
    }

    pub fn save_configs(&self) -> Result<(), Box<dyn std::error::Error>> {
        McpServerConfig::save_configs_to_yaml(&self.configs, &self.config_dir)
    }

    pub fn add_config(&mut self, config: McpServerConfig) -> Result<(), Vec<String>> {
        config.validate()?;
        
        // 检查重复名称
        if self.configs.iter().any(|c| c.name == config.name) {
            return Err(vec![format!("配置名称 '{}' 已存在", config.name)]);
        }

        self.configs.push(config);
        Ok(())
    }

    pub fn update_config(&mut self, index: usize, config: McpServerConfig) -> Result<(), Vec<String>> {
        config.validate()?;
        
        if index >= self.configs.len() {
            return Err(vec!["配置索引超出范围".to_string()]);
        }

        // 检查重复名称（排除自己）
        if self.configs.iter().enumerate().any(|(i, c)| i != index && c.name == config.name) {
            return Err(vec![format!("配置名称 '{}' 已存在", config.name)]);
        }

        self.configs[index] = config;
        Ok(())
    }

    pub fn remove_config(&mut self, index: usize) -> Result<(), String> {
        if index >= self.configs.len() {
            return Err("配置索引超出范围".to_string());
        }

        self.configs.remove(index);
        Ok(())
    }

    pub fn get_configs(&self) -> &[McpServerConfig] {
        &self.configs
    }

    pub fn get_config(&self, index: usize) -> Option<&McpServerConfig> {
        self.configs.get(index)
    }

    pub fn get_config_by_name(&self, name: &str) -> Option<&McpServerConfig> {
        self.configs.iter().find(|c| c.name == name)
    }

    pub fn init_with_presets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.configs.is_empty() {
            self.configs = McpServerConfig::get_presets();
            self.save_configs()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_serialization() {
        let config = McpServerConfig::filesystem();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: McpServerConfig = serde_yaml::from_str(&yaml).unwrap();
        
        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.display_name, deserialized.display_name);
    }

    #[test]
    fn test_config_validation() {
        let mut config = McpServerConfig::default();
        config.name = "test".to_string();
        config.display_name = "Test".to_string();
        config.command = "node".to_string();
        
        assert!(config.validate().is_ok());
        
        config.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_str().unwrap();
        
        let mut manager = McpConfigManager::new(config_dir);
        let config = McpServerConfig::filesystem();
        
        assert!(manager.add_config(config.clone()).is_ok());
        assert!(manager.save_configs().is_ok());
        
        let mut new_manager = McpConfigManager::new(config_dir);
        assert!(new_manager.load_configs().is_ok());
        assert_eq!(new_manager.get_configs().len(), 1);
        assert_eq!(new_manager.get_configs()[0].name, config.name);
    }
}