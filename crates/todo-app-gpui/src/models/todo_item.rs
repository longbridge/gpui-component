use crate::models::{
    mcp_config::{McpProviderInfo, McpProviderManager},
    provider_config::{LlmProviderInfo, LlmProviderManager, ModelInfo},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Todo状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub enum TodoStatus {
    Todo,       // 待办
    InProgress, // 进行中
    Done,       // 已完成
    Cancelled,  // 已取消
}

impl TodoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Todo => "待办",
            TodoStatus::InProgress => "进行中",
            TodoStatus::Done => "已完成",
            TodoStatus::Cancelled => "已取消",
        }
    }

    pub fn all() -> Vec<&'static str> {
        vec!["待办", "进行中", "已完成", "已取消"]
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "待办" => TodoStatus::Todo,
            "进行中" => TodoStatus::InProgress,
            "已完成" => TodoStatus::Done,
            "已取消" => TodoStatus::Cancelled,
            _ => TodoStatus::Todo,
        }
    }
}

/// 上传文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoFile {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub mime_type: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub uploaded_at: DateTime<Utc>,
}

/// 选中的模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedModel {
    pub provider_id: String,
    pub model_id: String,
    pub model_name: String,
    pub provider_name: String,
}

/// 选中的MCP工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedTool {
    pub provider_id: String,
    pub tool_name: String,
    pub provider_name: String,
    pub description: String,
}

/// Todo项目主结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TodoStatus,
    // pub priority: TodoPriority,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub due_date: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub reminder_date: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub completed_at: Option<DateTime<Utc>>,

    // AI配置
    pub selected_models: Vec<SelectedModel>,
    pub selected_tools: Vec<SelectedTool>,

    // 文件附件
    pub files: Vec<TodoFile>,

    // 配置选项
    pub recurring_enabled: bool,
    pub recurring_pattern: Option<String>, // "daily", "weekly", "monthly", "yearly"
    pub auto_execute: bool,
    pub enable_notifications: bool,
    pub push_to_feishu: bool,

    // 执行结果
    pub execution_logs: Vec<String>,
    pub last_execution_result: Option<String>,
}

impl Default for Todo {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: String::new(),
            description: String::new(),
            status: TodoStatus::Todo,
            // priority: TodoPriority::Medium,
            created_at: now,
            updated_at: now,
            due_date: None,
            reminder_date: None,
            completed_at: None,
            selected_models: Vec::new(),
            selected_tools: Vec::new(),
            files: Vec::new(),
            recurring_enabled: false,
            recurring_pattern: None,
            auto_execute: false,
            enable_notifications: true,
            push_to_feishu: false,
            execution_logs: Vec::new(),
            last_execution_result: None,
        }
    }
}

impl Todo {
    /// 创建新的Todo项目
    pub fn new(title: String, description: String) -> Self {
        Self {
            title,
            description,
            ..Default::default()
        }
    }

    /// 标记为完成
    pub fn mark_completed(&mut self) {
        self.status = TodoStatus::Done;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 标记为未完成
    pub fn mark_incomplete(&mut self) {
        if self.status == TodoStatus::Done {
            self.status = TodoStatus::Todo;
            self.completed_at = None;
            self.updated_at = Utc::now();
        }
    }

    /// 切换完成状态
    pub fn toggle_completed(&mut self) {
        if self.status == TodoStatus::Done {
            self.mark_incomplete();
        } else {
            self.mark_completed();
        }
    }

    /// 更新状态
    pub fn set_status(&mut self, status: TodoStatus) {
        let old_status = self.status;
        self.status = status;
        if status == TodoStatus::Done && self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        } else if status != TodoStatus::Done && old_status == TodoStatus::Done {
            self.completed_at = None;
        }
        self.updated_at = Utc::now();
    }

    /// 添加选中的模型
    pub fn add_selected_model(
        &mut self,
        provider_manager: &LlmProviderManager,
        provider_id: &str,
        model_id: &str,
    ) -> anyhow::Result<()> {
        if let Some(provider) = provider_manager.get_provider(provider_id) {
            if let Some(model) = provider.models.iter().find(|m| m.id == model_id) {
                let selected_model = SelectedModel {
                    provider_id: provider_id.to_string(),
                    model_id: model_id.to_string(),
                    model_name: model.display_name.clone(),
                    provider_name: provider.name.clone(),
                };

                // 检查是否已存在
                if !self
                    .selected_models
                    .iter()
                    .any(|m| m.provider_id == provider_id && m.model_id == model_id)
                {
                    self.selected_models.push(selected_model);
                    self.updated_at = Utc::now();
                }
                return Ok(());
            }
        }
        Err(anyhow::anyhow!(
            "Model not found: {}/{}",
            provider_id,
            model_id
        ))
    }

    /// 移除选中的模型
    pub fn remove_selected_model(&mut self, provider_id: &str, model_id: &str) {
        self.selected_models
            .retain(|m| !(m.provider_id == provider_id && m.model_id == model_id));
        self.updated_at = Utc::now();
    }

    /// 清空所有选中的模型
    pub fn clear_selected_models(&mut self) {
        self.selected_models.clear();
        self.updated_at = Utc::now();
    }

    /// 添加选中的工具
    pub fn add_selected_tool(
        &mut self,
        mcp_manager: &McpProviderManager,
        provider_id: &str,
        tool_name: &str,
    ) -> anyhow::Result<()> {
        if let Some(provider) = mcp_manager.get_provider(provider_id) {
            if let Some(tool) = provider.tools.iter().find(|t| t.name == tool_name) {
                let selected_tool = SelectedTool {
                    provider_id: provider_id.to_string(),
                    tool_name: tool_name.to_string(),
                    provider_name: provider.name.clone(),
                    description: tool.description.clone(),
                };

                // 检查是否已存在
                if !self
                    .selected_tools
                    .iter()
                    .any(|t| t.provider_id == provider_id && t.tool_name == tool_name)
                {
                    self.selected_tools.push(selected_tool);
                    self.updated_at = Utc::now();
                }
                return Ok(());
            }
        }
        Err(anyhow::anyhow!(
            "Tool not found: {}/{}",
            provider_id,
            tool_name
        ))
    }

    /// 移除选中的工具
    pub fn remove_selected_tool(&mut self, provider_id: &str, tool_name: &str) {
        self.selected_tools
            .retain(|t| !(t.provider_id == provider_id && t.tool_name == tool_name));
        self.updated_at = Utc::now();
    }

    /// 清空所有选中的工具
    pub fn clear_selected_tools(&mut self) {
        self.selected_tools.clear();
        self.updated_at = Utc::now();
    }

    /// 添加文件
    pub fn add_file(
        &mut self,
        name: String,
        path: String,
        size: Option<u64>,
        mime_type: Option<String>,
    ) {
        let file = TodoFile {
            name,
            path,
            size,
            mime_type,
            uploaded_at: Utc::now(),
        };
        self.files.push(file);
        self.updated_at = Utc::now();
    }

    /// 移除文件
    pub fn remove_file(&mut self, path: &str) {
        self.files.retain(|f| f.path != path);
        self.updated_at = Utc::now();
    }

    /// 添加执行日志
    pub fn add_execution_log(&mut self, log: String) {
        self.execution_logs.push(format!(
            "[{}] {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            log
        ));
        self.updated_at = Utc::now();
    }

    /// 设置执行结果
    pub fn set_execution_result(&mut self, result: String) {
        self.last_execution_result = Some(result);
        self.updated_at = Utc::now();
    }

    /// 检查是否过期
    pub fn is_overdue(&self) -> bool {
        if let Some(due_date) = self.due_date {
            due_date < Utc::now() && self.status != TodoStatus::Done
        } else {
            false
        }
    }

    /// 检查是否需要提醒
    pub fn needs_reminder(&self) -> bool {
        if let Some(reminder_date) = self.reminder_date {
            reminder_date <= Utc::now() && self.status != TodoStatus::Done
        } else {
            false
        }
    }

    /// 获取模型能力总结
    pub fn get_model_capabilities_summary(
        &self,
        provider_manager: &LlmProviderManager,
    ) -> Vec<String> {
        let mut capabilities = Vec::new();
        for selected_model in &self.selected_models {
            if let Some(provider) = provider_manager.get_provider(&selected_model.provider_id) {
                if let Some(model) = provider
                    .models
                    .iter()
                    .find(|m| m.id == selected_model.model_id)
                {
                    for capability in &model.capabilities {
                        let cap_str = capability.as_str().to_string();
                        if !capabilities.contains(&cap_str) {
                            capabilities.push(cap_str);
                        }
                    }
                }
            }
        }
        capabilities
    }

    /// 获取工具能力总结
    pub fn get_tool_capabilities_summary(&self) -> Vec<String> {
        self.selected_tools
            .iter()
            .map(|tool| tool.description.clone())
            .collect()
    }
}

/// Todo管理器
const TODO_CONFIG_FILE: &str = "config/todos.yml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoManager {
    #[serde(flatten, default)]
    pub todos: HashMap<String, Todo>,
}

impl TodoManager {
    /// 从文件加载配置
    pub fn load() -> Self {
        let config_path = Path::new(TODO_CONFIG_FILE);
        if !config_path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(config_path) {
            Ok(content) => match serde_yaml::from_str::<Self>(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to parse Todo config: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("Failed to read Todo config file: {}", e);
                Self::default()
            }
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Path::new(TODO_CONFIG_FILE);

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 获取所有Todo列表
    pub fn list_todos(&self) -> Vec<Todo> {
        let mut todos: Vec<Todo> = self.todos.values().cloned().collect();
        // 按更新时间倒序排列
        todos.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        todos
    }

    /// 根据ID查询Todo
    pub fn get_todo(&self, id: &str) -> Option<&Todo> {
        self.todos.get(id)
    }

    /// 根据状态筛选Todo
    pub fn get_todos_by_status(&self, status: TodoStatus) -> Vec<Todo> {
        self.todos
            .values()
            .filter(|todo| todo.status == status)
            .cloned()
            .collect()
    }

    /// 获取过期的Todo
    pub fn get_overdue_todos(&self) -> Vec<Todo> {
        self.todos
            .values()
            .filter(|todo| todo.is_overdue())
            .cloned()
            .collect()
    }

    /// 获取需要提醒的Todo
    pub fn get_reminder_todos(&self) -> Vec<Todo> {
        self.todos
            .values()
            .filter(|todo| todo.needs_reminder())
            .cloned()
            .collect()
    }

    /// 添加新的Todo
    pub fn add_todo(&mut self, todo: Todo) -> anyhow::Result<String> {
        let id = todo.id.clone();
        self.todos.insert(id.clone(), todo);
        Ok(id)
    }

    /// 更新Todo
    pub fn update_todo(&mut self, id: &str, mut todo: Todo) -> anyhow::Result<()> {
        if !self.todos.contains_key(id) {
            return Err(anyhow::anyhow!("Todo with id '{}' not found", id));
        }

        todo.updated_at = Utc::now();
        self.todos.insert(id.to_string(), todo);
        Ok(())
    }

    /// 删除Todo
    pub fn delete_todo(&mut self, id: &str) -> anyhow::Result<Todo> {
        self.todos
            .remove(id)
            .ok_or_else(|| anyhow::anyhow!("Todo with id '{}' not found", id))
    }

    /// 批量删除Todo
    pub fn batch_delete(&mut self, ids: &[String]) -> Vec<Todo> {
        let mut deleted = Vec::new();
        for id in ids {
            if let Some(todo) = self.todos.remove(id) {
                deleted.push(todo);
            }
        }
        deleted
    }

    /// 搜索Todo
    pub fn search_todos(&self, query: &str) -> Vec<Todo> {
        let query_lower = query.to_lowercase();
        self.todos
            .values()
            .filter(|todo| {
                todo.title.to_lowercase().contains(&query_lower)
                    || todo.description.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// 获取Todo统计信息
    pub fn get_statistics(&self) -> TodoStatistics {
        let total = self.todos.len();
        let completed = self
            .todos
            .values()
            .filter(|t| t.status == TodoStatus::Done)
            .count();
        let in_progress = self
            .todos
            .values()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();
        let todo = self
            .todos
            .values()
            .filter(|t| t.status == TodoStatus::Todo)
            .count();
        let cancelled = self
            .todos
            .values()
            .filter(|t| t.status == TodoStatus::Cancelled)
            .count();
        let overdue = self.todos.values().filter(|t| t.is_overdue()).count();

        TodoStatistics {
            total,
            completed,
            in_progress,
            todo,
            cancelled,
            overdue,
        }
    }

    /// 清空所有Todo
    pub fn clear(&mut self) {
        self.todos.clear();
    }

    /// 获取Todo数量
    pub fn count(&self) -> usize {
        self.todos.len()
    }
}

/// Todo统计信息
#[derive(Debug, Clone)]
pub struct TodoStatistics {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub todo: usize,
    pub cancelled: usize,
    pub overdue: usize,
}
