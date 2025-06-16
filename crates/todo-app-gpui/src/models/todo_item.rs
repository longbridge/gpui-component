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

/// Todo优先级枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub enum TodoPriority {
    Low,    // 低
    Medium, // 中
    High,   // 高
    Urgent, // 紧急
}

impl TodoPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoPriority::Low => "低",
            TodoPriority::Medium => "中",
            TodoPriority::High => "高",
            TodoPriority::Urgent => "紧急",
        }
    }

    pub fn all() -> Vec<&'static str> {
        vec!["低", "中", "高", "紧急"]
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "低" => TodoPriority::Low,
            "中" => TodoPriority::Medium,
            "高" => TodoPriority::High,
            "紧急" => TodoPriority::Urgent,
            _ => TodoPriority::Medium,
        }
    }

    pub fn color(&self) -> gpui::Rgba {
        match self {
            TodoPriority::Low => gpui::rgb(0x6B7280),
            TodoPriority::Medium => gpui::rgb(0x3B82F6),
            TodoPriority::High => gpui::rgb(0xF59E0B),
            TodoPriority::Urgent => gpui::rgb(0xEF4444),
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
    pub priority: TodoPriority,
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
            priority: TodoPriority::Medium,
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

    /// 设置优先级
    pub fn set_priority(&mut self, priority: TodoPriority) {
        self.priority = priority;
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

    /// 根据优先级筛选Todo
    pub fn get_todos_by_priority(&self, priority: TodoPriority) -> Vec<Todo> {
        self.todos
            .values()
            .filter(|todo| todo.priority == priority)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_creation() {
        let todo = Todo::new("Test Todo".to_string(), "This is a test todo".to_string());

        assert_eq!(todo.title, "Test Todo");
        assert_eq!(todo.description, "This is a test todo");
        assert_eq!(todo.status, TodoStatus::Todo);
        assert_eq!(todo.priority, TodoPriority::Medium);
    }

    #[test]
    fn test_todo_completion() {
        let mut todo = Todo::new("Test Todo".to_string(), "This is a test todo".to_string());

        todo.mark_completed();
        assert_eq!(todo.status, TodoStatus::Done);
        assert!(todo.completed_at.is_some());

        todo.mark_incomplete();
        assert_eq!(todo.status, TodoStatus::Todo);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn test_todo_toggle_completion() {
        let mut todo = Todo::new("Test Todo".to_string(), "Test".to_string());

        // Initially not completed
        assert_eq!(todo.status, TodoStatus::Todo);
        assert!(todo.completed_at.is_none());

        // Toggle to completed
        todo.toggle_completed();
        assert_eq!(todo.status, TodoStatus::Done);
        assert!(todo.completed_at.is_some());

        // Toggle back to incomplete
        todo.toggle_completed();
        assert_eq!(todo.status, TodoStatus::Todo);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn test_todo_status_transitions() {
        let mut todo = Todo::new("Test Todo".to_string(), "Test".to_string());

        // Set to in progress
        todo.set_status(TodoStatus::InProgress);
        assert_eq!(todo.status, TodoStatus::InProgress);
        assert!(todo.completed_at.is_none());

        // Set to done
        todo.set_status(TodoStatus::Done);
        assert_eq!(todo.status, TodoStatus::Done);
        assert!(todo.completed_at.is_some());

        // Set to cancelled
        todo.set_status(TodoStatus::Cancelled);
        assert_eq!(todo.status, TodoStatus::Cancelled);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn test_todo_priority() {
        let mut todo = Todo::new("Test Todo".to_string(), "Test".to_string());

        assert_eq!(todo.priority, TodoPriority::Medium);

        todo.set_priority(TodoPriority::High);
        assert_eq!(todo.priority, TodoPriority::High);

        todo.set_priority(TodoPriority::Urgent);
        assert_eq!(todo.priority, TodoPriority::Urgent);
    }

    #[test]
    fn test_todo_files() {
        let mut todo = Todo::new("Test Todo".to_string(), "Test".to_string());

        assert!(todo.files.is_empty());

        todo.add_file(
            "test.txt".to_string(),
            "/path/to/test.txt".to_string(),
            Some(1024),
            Some("text/plain".to_string()),
        );

        assert_eq!(todo.files.len(), 1);
        assert_eq!(todo.files[0].name, "test.txt");
        assert_eq!(todo.files[0].path, "/path/to/test.txt");
        assert_eq!(todo.files[0].size, Some(1024));

        todo.remove_file("/path/to/test.txt");
        assert!(todo.files.is_empty());
    }

    #[test]
    fn test_todo_execution_logs() {
        let mut todo = Todo::new("Test Todo".to_string(), "Test".to_string());

        assert!(todo.execution_logs.is_empty());

        todo.add_execution_log("Started execution".to_string());
        todo.add_execution_log("Completed successfully".to_string());

        assert_eq!(todo.execution_logs.len(), 2);
        assert!(todo.execution_logs[0].contains("Started execution"));
        assert!(todo.execution_logs[1].contains("Completed successfully"));

        todo.set_execution_result("Success".to_string());
        assert_eq!(todo.last_execution_result, Some("Success".to_string()));
    }

    #[test]
    fn test_todo_overdue_check() {
        let mut todo = Todo::new("Test Todo".to_string(), "Test".to_string());

        // No due date set
        assert!(!todo.is_overdue());

        // Set due date in the past
        todo.due_date = Some(Utc::now() - chrono::Duration::days(1));
        assert!(todo.is_overdue());

        // Completed todos are not overdue
        todo.mark_completed();
        assert!(!todo.is_overdue());

        // Future due date
        todo.mark_incomplete();
        todo.due_date = Some(Utc::now() + chrono::Duration::days(1));
        assert!(!todo.is_overdue());
    }

    #[test]
    fn test_todo_reminder_check() {
        let mut todo = Todo::new("Test Todo".to_string(), "Test".to_string());

        // No reminder date set
        assert!(!todo.needs_reminder());

        // Set reminder date in the past
        todo.reminder_date = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(todo.needs_reminder());

        // Completed todos don't need reminders
        todo.mark_completed();
        assert!(!todo.needs_reminder());

        // Future reminder date
        todo.mark_incomplete();
        todo.reminder_date = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(!todo.needs_reminder());
    }

    #[test]
    fn test_todo_manager() {
        let mut manager = TodoManager::default();

        let todo = Todo::new("Test Todo".to_string(), "This is a test todo".to_string());

        let id = manager.add_todo(todo.clone()).unwrap();
        assert_eq!(manager.count(), 1);

        let retrieved = manager.get_todo(&id).unwrap();
        assert_eq!(retrieved.title, "Test Todo");

        manager.delete_todo(&id).unwrap();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_todo_manager_filtering() {
        let mut manager = TodoManager::default();

        let mut todo1 = Todo::new("Todo 1".to_string(), "Description 1".to_string());
        todo1.set_status(TodoStatus::Done);
        todo1.set_priority(TodoPriority::High);

        let mut todo2 = Todo::new("Todo 2".to_string(), "Description 2".to_string());
        todo2.set_status(TodoStatus::InProgress);
        todo2.set_priority(TodoPriority::Low);

        let mut todo3 = Todo::new("Todo 3".to_string(), "Description 3".to_string());
        todo3.set_priority(TodoPriority::High);

        manager.add_todo(todo1).unwrap();
        manager.add_todo(todo2).unwrap();
        manager.add_todo(todo3).unwrap();

        // Test filtering by status
        let completed = manager.get_todos_by_status(TodoStatus::Done);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].title, "Todo 1");

        let in_progress = manager.get_todos_by_status(TodoStatus::InProgress);
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].title, "Todo 2");

        let todo_status = manager.get_todos_by_status(TodoStatus::Todo);
        assert_eq!(todo_status.len(), 1);
        assert_eq!(todo_status[0].title, "Todo 3");

        // Test filtering by priority
        let high_priority = manager.get_todos_by_priority(TodoPriority::High);
        assert_eq!(high_priority.len(), 2);

        let low_priority = manager.get_todos_by_priority(TodoPriority::Low);
        assert_eq!(low_priority.len(), 1);
        assert_eq!(low_priority[0].title, "Todo 2");
    }

    #[test]
    fn test_todo_manager_search() {
        let mut manager = TodoManager::default();

        let todo1 = Todo::new("Buy groceries".to_string(), "Milk, bread, eggs".to_string());
        let todo2 = Todo::new(
            "Walk the dog".to_string(),
            "30 minutes in the park".to_string(),
        );
        let todo3 = Todo::new(
            "Write code".to_string(),
            "Implement new features".to_string(),
        );

        manager.add_todo(todo1).unwrap();
        manager.add_todo(todo2).unwrap();
        manager.add_todo(todo3).unwrap();

        // Search by title
        let results = manager.search_todos("dog");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Walk the dog");

        // Search by description
        let results = manager.search_todos("milk");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Buy groceries");

        // Case insensitive search
        let results = manager.search_todos("CODE");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Write code");

        // No matches
        let results = manager.search_todos("xyz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_todo_manager_batch_operations() {
        let mut manager = TodoManager::default();

        let todo1 = Todo::new("Todo 1".to_string(), "Description 1".to_string());
        let todo2 = Todo::new("Todo 2".to_string(), "Description 2".to_string());
        let todo3 = Todo::new("Todo 3".to_string(), "Description 3".to_string());

        let id1 = manager.add_todo(todo1).unwrap();
        let id2 = manager.add_todo(todo2).unwrap();
        let id3 = manager.add_todo(todo3).unwrap();

        assert_eq!(manager.count(), 3);

        // Batch delete
        let deleted = manager.batch_delete(&[id1.clone(), id3.clone()]);
        assert_eq!(deleted.len(), 2);
        assert_eq!(manager.count(), 1);

        // Verify remaining todo
        let remaining = manager.get_todo(&id2).unwrap();
        assert_eq!(remaining.title, "Todo 2");

        // Clear all
        manager.clear();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_todo_manager_statistics() {
        let mut manager = TodoManager::default();

        let mut todo1 = Todo::new("Todo 1".to_string(), "Description 1".to_string());
        todo1.set_status(TodoStatus::Done);

        let mut todo2 = Todo::new("Todo 2".to_string(), "Description 2".to_string());
        todo2.set_status(TodoStatus::InProgress);

        let mut todo3 = Todo::new("Todo 3".to_string(), "Description 3".to_string());
        todo3.set_status(TodoStatus::Cancelled);

        let mut todo4 = Todo::new("Todo 4".to_string(), "Description 4".to_string());
        todo4.due_date = Some(Utc::now() - chrono::Duration::days(1)); // Overdue

        manager.add_todo(todo1).unwrap();
        manager.add_todo(todo2).unwrap();
        manager.add_todo(todo3).unwrap();
        manager.add_todo(todo4).unwrap();

        let stats = manager.get_statistics();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.todo, 1);
        assert_eq!(stats.cancelled, 1);
        assert_eq!(stats.overdue, 1);
    }

    #[test]
    fn test_todo_status_enum() {
        assert_eq!(TodoStatus::Todo.as_str(), "待办");
        assert_eq!(TodoStatus::InProgress.as_str(), "进行中");
        assert_eq!(TodoStatus::Done.as_str(), "已完成");
        assert_eq!(TodoStatus::Cancelled.as_str(), "已取消");

        assert_eq!(TodoStatus::from_str("待办"), TodoStatus::Todo);
        assert_eq!(TodoStatus::from_str("进行中"), TodoStatus::InProgress);
        assert_eq!(TodoStatus::from_str("已完成"), TodoStatus::Done);
        assert_eq!(TodoStatus::from_str("已取消"), TodoStatus::Cancelled);
        assert_eq!(TodoStatus::from_str("unknown"), TodoStatus::Todo);

        let all_statuses = TodoStatus::all();
        assert_eq!(all_statuses.len(), 4);
        assert!(all_statuses.contains(&"待办"));
        assert!(all_statuses.contains(&"进行中"));
        assert!(all_statuses.contains(&"已完成"));
        assert!(all_statuses.contains(&"已取消"));
    }

    #[test]
    fn test_todo_priority_enum() {
        assert_eq!(TodoPriority::Low.as_str(), "低");
        assert_eq!(TodoPriority::Medium.as_str(), "中");
        assert_eq!(TodoPriority::High.as_str(), "高");
        assert_eq!(TodoPriority::Urgent.as_str(), "紧急");

        assert_eq!(TodoPriority::from_str("低"), TodoPriority::Low);
        assert_eq!(TodoPriority::from_str("中"), TodoPriority::Medium);
        assert_eq!(TodoPriority::from_str("高"), TodoPriority::High);
        assert_eq!(TodoPriority::from_str("紧急"), TodoPriority::Urgent);
        assert_eq!(TodoPriority::from_str("unknown"), TodoPriority::Medium);

        let all_priorities = TodoPriority::all();
        assert_eq!(all_priorities.len(), 4);
        assert!(all_priorities.contains(&"低"));
        assert!(all_priorities.contains(&"中"));
        assert!(all_priorities.contains(&"高"));
        assert!(all_priorities.contains(&"紧急"));
    }

    #[test]
    fn test_todo_update() {
        let mut manager = TodoManager::default();

        let todo = Todo::new(
            "Original Title".to_string(),
            "Original Description".to_string(),
        );
        let id = manager.add_todo(todo).unwrap();

        // Update the todo
        let mut updated_todo = manager.get_todo(&id).unwrap().clone();
        updated_todo.title = "Updated Title".to_string();
        updated_todo.description = "Updated Description".to_string();

        manager.update_todo(&id, updated_todo).unwrap();

        let retrieved = manager.get_todo(&id).unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.description, "Updated Description");

        // Test updating non-existent todo
        let result = manager.update_todo("non-existent", Todo::default());
        assert!(result.is_err());
    }
}
