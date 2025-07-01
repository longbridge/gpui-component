use crate::models::{
    mcp_config::McpConfigManager, provider_config::LlmProviders, todo_config_path,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Todo状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy, Default)]
pub enum TodoStatus {
    #[default]
    Todo, // 待办
    InProgress, // 进行中
    Done,       // 已完成
    Alert,      // 警报
    Cancelled,  // 已取消
    Suspended,  // 暂停
    Deleted,
}

impl TodoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Todo => "待办",
            TodoStatus::InProgress => "进行中",
            TodoStatus::Alert => "警报",
            TodoStatus::Done => "已完成",
            TodoStatus::Cancelled => "已取消",
            TodoStatus::Suspended => "暂停",
            TodoStatus::Deleted => "已删除",
        }
    }

    pub fn all() -> Vec<&'static str> {
        vec![
            "待办",
            "进行中",
            "警报",
            "已完成",
            "已取消",
            "暂停",
            "已删除",
        ]
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "待办" => TodoStatus::Todo,
            "进行中" => TodoStatus::InProgress,
            "警报" => TodoStatus::Alert,
            "已完成" => TodoStatus::Done,
            "已取消" => TodoStatus::Cancelled,
            "暂停" => TodoStatus::Suspended,
            "已删除" => TodoStatus::Deleted,
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
    #[serde(default)]
    pub model_name: String,
    #[serde(default)]
    pub provider_name: String,
}

/// 选中的MCP工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedTool {
    pub provider_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub provider_name: String,
    #[serde(default)]
    pub description: String,
}

/// Todo项目主结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: TodoStatus,
    // AI配置
    #[serde(default)]
    pub selected_model: Option<SelectedModel>,
    #[serde(default)]
    pub selected_tools: Vec<SelectedTool>,
    // 文件附件
    #[serde(default)]
    pub files: Vec<TodoFile>,
    // 配置选项
    #[serde(default)]
    pub recurring_enabled: bool,
    pub recurring_pattern: Option<String>, // "daily", "weekly", "monthly", "yearly"
    #[serde(default)]
    pub auto_execute: bool,
    #[serde(default)]
    pub enable_notifications: bool,
    #[serde(default)]
    pub push_to_feishu: bool,
    #[serde(default)]
    pub follow: bool, // 是否关注该任务
    #[serde(default)]
    pub needs_recording: bool, // 是否需要录音
    #[serde(default)]
    pub needs_screen_recording: bool, // 是否需要录屏
    // 执行结果
    #[serde(default)]
    pub execution_logs: Vec<String>,
    pub last_execution_result: Option<String>,
    // 时间戳
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
}

impl Default for Todo {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: String::new(),
            description: String::new(),
            status: TodoStatus::Todo,
            created_at: now,
            updated_at: now,
            due_date: None,
            reminder_date: None,
            completed_at: None,
            selected_model: None,
            selected_tools: Vec::new(),
            files: Vec::new(),
            recurring_enabled: false,
            recurring_pattern: None,
            auto_execute: false,
            enable_notifications: true,
            push_to_feishu: false,
            follow: false,
            needs_recording: false,        // 默认不需要录音
            needs_screen_recording: false, // 默认不需要录屏
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
    pub fn add_selected_model(&mut self, provider_id: &str, model_id: &str) -> anyhow::Result<()> {
        if let Some(provider) = LlmProviders::get_provider(provider_id) {
            if let Some(model) = provider.models.iter().find(|m| m.id == model_id) {
                let selected_model = SelectedModel {
                    provider_id: provider_id.to_string(),
                    model_id: model_id.to_string(),
                    model_name: model.display_name.clone(),
                    provider_name: provider.name.clone(),
                };
                self.selected_model = Some(selected_model);
                self.updated_at = Utc::now();
                // 检查是否已存在
                // if !self
                //     .selected_model
                //     .iter()
                //     .any(|m| m.provider_id == provider_id && m.model_id == model_id)
                // {
                //     self.selected_model.push(selected_model);
                //     self.updated_at = Utc::now();
                // }
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
    pub fn remove_selected_model(&mut self) {
        self.selected_model = None;
        self.updated_at = Utc::now();
    }

    /// 清空所有选中的模型
    pub fn clear_selected_model(&mut self) {
        self.selected_model = None;
        self.updated_at = Utc::now();
    }

    /// 添加选中的工具
    pub fn add_selected_tool(&mut self, provider_id: &str, tool_name: &str) -> anyhow::Result<()> {
        if let Ok(Some(provider)) = McpConfigManager::get_server(provider_id) {
            let selected_tool = SelectedTool {
                provider_id: provider_id.to_string(),
                tool_name: tool_name.to_string(),
                provider_name: provider.name.clone(),
                description: String::new(),
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
    pub fn get_model_capabilities_summary(&self) -> Vec<String> {
        let mut capabilities = Vec::new();
        for selected_model in &self.selected_model {
            if let Some(provider) = LlmProviders::get_provider(&selected_model.provider_id) {
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

    /// 设置关注状态
    pub fn set_follow(&mut self, follow: bool) {
        self.follow = follow;
        self.updated_at = Utc::now();
    }

    /// 切换关注状态
    pub fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        self.updated_at = Utc::now();
    }

    /// 设置是否需要录音
    pub fn set_needs_recording(&mut self, needs_recording: bool) {
        self.needs_recording = needs_recording;
        self.updated_at = Utc::now();
    }

    /// 切换录音需求状态
    pub fn toggle_needs_recording(&mut self) {
        self.needs_recording = !self.needs_recording;
        self.updated_at = Utc::now();
    }

    /// 设置是否需要录屏
    pub fn set_needs_screen_recording(&mut self, needs_screen_recording: bool) {
        self.needs_screen_recording = needs_screen_recording;
        self.updated_at = Utc::now();
    }

    /// 切换录屏需求状态
    pub fn toggle_needs_screen_recording(&mut self) {
        self.needs_screen_recording = !self.needs_screen_recording;
        self.updated_at = Utc::now();
    }

    /// 检查是否需要媒体记录（录音或录屏）
    pub fn needs_media_recording(&self) -> bool {
        self.needs_recording || self.needs_screen_recording
    }

    pub fn copy(&self) -> Self {
        let mut copy = self.clone();
        copy.id = uuid::Uuid::new_v4().to_string(); // 生成新的ID
        copy
    }
}

/// Todo管理器 - 无状态管理器，所有方法都是静态的
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoManager;

impl TodoManager {
    /// 从文件加载配置
    fn load() -> Vec<Todo> {
        let config_path = todo_config_path();
        if !config_path.exists() {
            return Vec::new();
        }

        match std::fs::read_to_string(config_path) {
            Ok(content) => match serde_yaml::from_str::<Vec<Todo>>(&content) {
                Ok(todos) => todos,
                Err(e) => {
                    eprintln!("Failed to parse Todo config: {}", e);
                    Vec::new()
                }
            },
            Err(e) => {
                eprintln!("Failed to read Todo config file: {}", e);
                Vec::new()
            }
        }
    }

    /// 保存配置到文件
    pub fn save(todos: &[Todo]) -> anyhow::Result<()> {
        let config_path = todo_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(todos)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 获取所有Todo列表
    pub fn list_todos() -> Vec<Todo> {
        // 按更新时间倒序排列
        let mut result = Self::load();
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        result
    }

    /// 根据ID查询Todo
    pub fn get_todo(id: &str) -> Option<Todo> {
        Self::list_todos().into_iter().find(|todo| todo.id == id)
    }

    /// 根据状态筛选Todo
    pub fn get_todos_by_status(status: TodoStatus) -> Vec<Todo> {
        Self::list_todos()
            .iter()
            .filter(|todo| todo.status == status)
            .cloned()
            .collect()
    }

    /// 获取过期的Todo
    pub fn get_overdue_todos() -> Vec<Todo> {
        Self::list_todos()
            .iter()
            .filter(|todo| todo.is_overdue())
            .cloned()
            .collect()
    }

    /// 获取需要提醒的Todo
    pub fn get_reminder_todos() -> Vec<Todo> {
        Self::list_todos()
            .iter()
            .filter(|todo| todo.needs_reminder())
            .cloned()
            .collect()
    }

    /// 获取关注的Todo列表
    pub fn get_followed_todos() -> Vec<Todo> {
        Self::list_todos()
            .iter()
            .filter(|todo| todo.follow)
            .cloned()
            .collect()
    }

    /// 获取需要录音的Todo列表
    pub fn get_recording_todos() -> Vec<Todo> {
        Self::list_todos()
            .iter()
            .filter(|todo| todo.needs_recording)
            .cloned()
            .collect()
    }

    /// 获取需要录屏的Todo列表
    pub fn get_screen_recording_todos() -> Vec<Todo> {
        Self::list_todos()
            .iter()
            .filter(|todo| todo.needs_screen_recording)
            .cloned()
            .collect()
    }

    /// 获取需要媒体记录的Todo列表
    pub fn get_media_recording_todos() -> Vec<Todo> {
        Self::list_todos()
            .iter()
            .filter(|todo| todo.needs_media_recording())
            .cloned()
            .collect()
    }

    /// 更新Todo - 返回新的Vec
    pub fn update_todo(mut todo: Todo) -> anyhow::Result<()> {
        let mut todos = Self::list_todos();
        if let Some(position) = todos.iter().position(|t| t.id == todo.id) {
            todo.updated_at = Utc::now();
            todos[position] = todo;
        } else {
            todos.push(todo);
        }
        Self::save(todos.as_slice())?;
        Ok(())
    }

    /// 删除Todo - 标记为已删除状态
    pub fn delete_todo(id: &str) -> anyhow::Result<Option<Todo>> {
        let mut todos = Self::list_todos();
        if let Some(position) = todos.iter().position(|t| t.id == id) {
            todos[position].status = TodoStatus::Deleted;
            Self::save(todos.as_slice())?;
            return Ok(Some(todos[position].clone()));
        }
        Ok(None)
    }

    /// 复制Todo
    pub fn copy_todo(id: &str) -> anyhow::Result<Option<Todo>> {
        let mut todos = Self::list_todos();
        if let Some(position) = todos.iter().position(|t| t.id == id) {
            let copy = todos[position].copy();
            todos.push(copy.clone());
            Self::save(todos.as_slice())?;
            return Ok(Some(copy));
        }
        Ok(None)
    }

    /// 搜索Todo
    pub fn search_todos(query: &str) -> Vec<Todo> {
        let todos = Self::list_todos();
        let query_lower = query.to_lowercase();
        todos
            .iter()
            .filter(|todo| {
                todo.title.to_lowercase().contains(&query_lower)
                    || todo.description.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// 获取Todo统计信息
    pub fn get_statistics() -> TodoStatistics {
        let mut todos = Self::list_todos();
        let total = todos.len();
        let completed = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Done)
            .count();
        let in_progress = todos
            .iter()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();
        let todo = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Todo)
            .count();
        let cancelled = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Cancelled)
            .count();
        let suspended = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Suspended)
            .count();
        let followed = todos.iter().filter(|t| t.follow).count();
        let overdue = todos.iter().filter(|t| t.is_overdue()).count();
        let needs_recording = todos.iter().filter(|t| t.needs_recording).count();
        let needs_screen_recording = todos.iter().filter(|t| t.needs_screen_recording).count();

        TodoStatistics {
            total,
            completed,
            in_progress,
            todo,
            cancelled,
            suspended,
            followed,
            overdue,
            needs_recording,
            needs_screen_recording,
        }
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
    pub suspended: usize,
    pub followed: usize,
    pub overdue: usize,
    pub needs_recording: usize,
    pub needs_screen_recording: usize,
}
