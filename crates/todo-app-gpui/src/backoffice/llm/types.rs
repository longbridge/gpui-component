use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::pin::Pin;

/// 媒体数据存储方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaData {
    /// 直接内嵌的文本内容
    Text(String),
    /// Base64 编码的二进制数据（适用于小文件）
    Base64(String),
    /// 文件路径引用
    FilePath(String),
    /// URL 引用
    Url(String),
    /// 二进制数据（仅在内存中使用，不序列化）
    #[serde(skip)]
    Binary(Vec<u8>),
}

/// 媒体类型枚举（从 MIME 类型推导）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    Text,
    Image,
    Audio,
    Video,
    Document,
    Application,
    Unknown,
}

/// 媒体内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaContent {
    /// MIME 类型（直接表示具体格式）
    pub mime_type: String,
    /// 内容数据
    pub data: MediaData,
    /// 可选的描述信息
    pub description: Option<String>,
    /// 文件名（如果适用）
    pub filename: Option<String>,
    /// 文件大小（字节）
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: String,
}

impl ToolDefinition {
    pub fn format_tool_name(provider_id: &str, tool_name: &str) -> String {
        format!("{}@{}", provider_id, tool_name)
    }
}

/// 工具函数执行状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ToolExecutionStatus {
    /// 待执行
    #[default]
    Pending,
    /// 执行中
    Running,
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
}

/// 工具函数 - 统一的工具调用和结果结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "tool_use")]
pub struct ToolFunction {
    /// 唯一标识符（可选）
    pub id: Option<String>,
    /// 工具名称 (provider_id@tool_name 格式)
    pub name: String,
    /// 工具调用参数（JSON 字符串）
    #[serde(default)]
    pub arguments: String,
    /// 执行结果（可选）
    #[serde(default)]
    pub result: Option<String>,
    /// 执行状态
    #[serde(skip)]
    pub status: ToolExecutionStatus,
    /// 错误信息（如果执行失败）
    #[serde(default)]
    pub error: Option<String>,
    /// 执行开始时间
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    /// 执行完成时间
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    /// 执行元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ToolFunction {
    /// 创建新的工具函数调用
    pub fn new(name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            arguments: arguments.into(),
            result: None,
            status: ToolExecutionStatus::Pending,
            error: None,
            started_at: None,
            completed_at: None,
            metadata: HashMap::new(),
        }
    }

    /// 创建带ID的工具函数调用
    pub fn with_id(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(id.into()),
            name: name.into(),
            arguments: arguments.into(),
            result: None,
            status: ToolExecutionStatus::Pending,
            error: None,
            started_at: None,
            completed_at: None,
            metadata: HashMap::new(),
        }
    }

    /// 获取工具ID部分
    pub fn tool_id(&self) -> &str {
        self.name.split('@').next().unwrap_or(&self.name)
    }

    /// 获取工具名称部分
    pub fn tool_name(&self) -> &str {
        self.name.split('@').nth(1).unwrap_or(&self.name)
    }

    /// 获取提供者ID
    pub fn provider_id(&self) -> Option<&str> {
        if self.name.contains('@') {
            Some(self.name.split('@').next().unwrap_or(&self.name))
        } else {
            None
        }
    }

    /// 开始执行
    pub fn start_execution(&mut self) -> &mut Self {
        self.status = ToolExecutionStatus::Running;
        self.started_at = Some(Utc::now());
        self
    }

    /// 设置执行成功
    pub fn set_success(&mut self, result: impl Into<String>) -> &mut Self {
        self.result = Some(result.into());
        self.status = ToolExecutionStatus::Success;
        self.completed_at = Some(Utc::now());
        self.error = None;
        self
    }

    /// 设置执行失败
    pub fn set_failed(&mut self, error: impl Into<String>) -> &mut Self {
        self.error = Some(error.into());
        self.status = ToolExecutionStatus::Failed;
        self.completed_at = Some(Utc::now());
        self
    }

    /// 检查是否已完成
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            ToolExecutionStatus::Success | ToolExecutionStatus::Failed
        )
    }

    /// 检查是否成功
    pub fn is_success(&self) -> bool {
        matches!(self.status, ToolExecutionStatus::Success)
    }

    /// 检查是否失败
    pub fn is_failed(&self) -> bool {
        matches!(self.status, ToolExecutionStatus::Failed)
    }

    /// 检查是否正在执行
    pub fn is_running(&self) -> bool {
        matches!(self.status, ToolExecutionStatus::Running)
    }

    /// 检查是否待执行
    pub fn is_pending(&self) -> bool {
        matches!(self.status, ToolExecutionStatus::Pending)
    }

    /// 获取执行结果或错误信息
    pub fn get_output(&self) -> Option<&str> {
        self.result.as_deref().or(self.error.as_deref())
    }
}

/// 消息角色枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// 消息内容 - 支持多模态和工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Part(MediaContent),
    /// 工具函数调用（新的统一结构）
    ToolFunction(ToolFunction),
    /// 流式文本块
    TextChunk(String),
    /// 工具定义
    ToolDefinitions(Vec<ToolDefinition>),
}

impl MessageContent {
    /// 检查是否为工具函数
    pub fn is_tool_function(&self) -> bool {
        matches!(self, Self::ToolFunction(_))
    }

    /// 获取工具函数
    pub fn get_tool_function(&self) -> Option<&ToolFunction> {
        match self {
            Self::ToolFunction(tf) => Some(tf),
            _ => None,
        }
    }

    /// 获取可变工具函数
    pub fn get_tool_function_mut(&mut self) -> Option<&mut ToolFunction> {
        match self {
            Self::ToolFunction(tf) => Some(tf),
            _ => None,
        }
    }

    /// 检查是否为工具定义
    pub fn is_tool_definitions(&self) -> bool {
        matches!(self, Self::ToolDefinitions(_))
    }

    /// 检查是否为文本块
    pub fn is_text_chunk(&self) -> bool {
        matches!(self, Self::TextChunk(_))
    }

    /// 获取文本内容
    pub fn get_text(&self) -> String {
        match self {
            Self::Part(part) => part.get_text().unwrap_or_default().to_string(),
            Self::TextChunk(text) => text.clone(),
            Self::ToolDefinitions(tools) => tools
                .iter()
                .map(|tool| format!("Tool: {} - {}", tool.name, tool.description))
                .collect::<Vec<_>>()
                .join("\n"),
            Self::ToolFunction(tool_function) => {
                format!(
                    "Tool: {}\nArguments: {}\nResult: {}",
                    tool_function.name,
                    tool_function.arguments,
                    tool_function.result.as_deref().unwrap_or("")
                )
            }
        }
    }

    /// 获取工具定义
    pub fn get_tool_definitions(&self) -> Option<&[ToolDefinition]> {
        match self {
            Self::ToolDefinitions(tools) => Some(tools),
            _ => None,
        }
    }

    /// 添加文本到现有内容
    pub fn append_text(&mut self, text: &str) -> &mut Self {
        match self {
            Self::TextChunk(existing) => existing.push_str(text),

            Self::Part(part) => {
                if let Some(existing_text) = part.get_text_mut() {
                    existing_text.push_str(text);
                } else {
                    unimplemented!("Cannot append text to non-text content");
                }
            }
            _ => {
                unimplemented!("Cannot append text to non-text content");
            }
        }
        self
    }

    /// 添加工具定义
    pub fn add_tool_definition(&mut self, tool: ToolDefinition) -> &mut Self {
        match self {
            Self::ToolDefinitions(tools) => tools.push(tool),
            _ => {
                *self = Self::ToolDefinitions(vec![tool]);
            }
        }
        self
    }

    /// 克隆并转换为指定类型
    pub fn as_text_chunk(&self) -> Option<String> {
        match self {
            Self::TextChunk(text) => Some(text.clone()),
            _ => None,
        }
    }
}

impl MediaContent {
    /// 创建文本媒体内容
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            mime_type: "text/plain".to_string(),
            data: MediaData::Text(content.into()),
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 检查是否为文本媒体
    pub fn is_text(&self) -> bool {
        matches!(self.data, MediaData::Text(_))
    }

    /// 获取文本内容
    pub fn get_text(&self) -> Option<&str> {
        match &self.data {
            MediaData::Text(text) => Some(text.as_str()),
            MediaData::Base64(base64) => Some(base64.as_str()),
            MediaData::FilePath(path) => Some(path.as_str()),
            MediaData::Url(url) => Some(url.as_str()),
            MediaData::Binary(_) => Some("二进制数据"), // 二进制数据不支持直接获取文本
        }
    }

    /// 获取可变文本内容
    pub fn get_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.data {
            MediaData::Text(text) => Some(text),
            MediaData::Base64(base64) => Some(base64),
            MediaData::FilePath(path) => Some(path),
            MediaData::Url(url) => Some(url),
            MediaData::Binary(_) => None, // 二进制数据不支持直接获取文本
        }
    }
}

/// 聊天消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息角色
    pub role: MessageRole,
    /// 消息内容列表（支持多模态）
    pub contents: Vec<MessageContent>,
    /// 消息时间戳
    pub timestamp: DateTime<Utc>,
    /// 消息元数据（存储额外信息）
    pub metadata: HashMap<String, String>,
}

impl ChatMessage {
    /// 创建新的聊天消息
    pub fn new(role: MessageRole, contents: Vec<MessageContent>) -> Self {
        Self {
            role,
            contents,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// 创建空消息
    pub fn empty(role: MessageRole) -> Self {
        Self::new(role, vec![])
    }

    /// 创建空消息
    pub fn assistant() -> Self {
        Self::new(MessageRole::Assistant, vec![])
    }

    pub fn system() -> Self {
        Self::new(MessageRole::System, vec![])
    }
    pub fn user() -> Self {
        Self::new(MessageRole::User, vec![])
    }

    pub fn with_role(mut self, role: MessageRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_content(mut self, content: MessageContent) -> Self {
        self.contents.push(content);
        self
    }

    pub fn with_contents(mut self, contents: Vec<MessageContent>) -> Self {
        self.contents = contents;
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.contents
            .push(MessageContent::Part(MediaContent::text(text)));
        self
    }

    pub fn with_text_chunk(mut self, text: impl Into<String>) -> Self {
        self.contents.push(MessageContent::TextChunk(text.into()));
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.metadata.insert("source".to_string(), source.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl ChatMessage {
    /// 添加内容
    pub fn add_content(&mut self, content: MessageContent) -> &mut Self {
        self.contents.push(content);
        self
    }

    /// 添加多个内容
    pub fn add_contents(&mut self, contents: Vec<MessageContent>) -> &mut Self {
        self.contents.extend(contents);
        self
    }

    /// 获取第一个内容
    pub fn first_content(&self) -> Option<&MessageContent> {
        self.contents.first()
    }

    /// 获取最后一个内容
    pub fn last_content(&self) -> Option<&MessageContent> {
        self.contents.last()
    }

    /// 获取可变的第一个内容
    pub fn first_content_mut(&mut self) -> Option<&mut MessageContent> {
        self.contents.first_mut()
    }

    /// 获取可变的最后一个内容
    pub fn last_content_mut(&mut self) -> Option<&mut MessageContent> {
        self.contents.last_mut()
    }

    /// 获取所有文本内容
    pub fn get_text(&self) -> String {
        self.contents
            .iter()
            .map(|content| content.get_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// 获取所有工具定义
    pub fn get_tool_definitions(&self) -> Vec<&ToolDefinition> {
        self.contents
            .iter()
            .filter_map(|content| content.get_tool_definitions())
            .flatten()
            .collect()
    }

    /// 检查是否包含工具调用
    pub fn has_tool_function(&self) -> bool {
        self.contents
            .iter()
            .any(|content| content.is_tool_function())
    }

    /// 检查是否包含工具定义
    pub fn has_tool_definitions(&self) -> bool {
        self.contents
            .iter()
            .any(|content| content.is_tool_definitions())
    }

    /// 检查是否为空消息
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    /// 获取消息长度（内容数量）
    pub fn len(&self) -> usize {
        self.contents.len()
    }

    /// 获取消息来源
    pub fn get_source(&self) -> Option<&str> {
        self.metadata.get("source").map(|s| s.as_str())
    }

    /// 获取模型ID
    pub fn get_model_id(&self) -> Option<&str> {
        self.metadata.get("model_id").map(|s| s.as_str())
    }

    /// 获取模型名称
    pub fn get_model_name(&self) -> Option<&str> {
        self.metadata.get("model_name").map(|s| s.as_str())
    }

    /// 设置消息来源
    pub fn set_source(&mut self, source: impl Into<String>) -> &mut Self {
        self.metadata.insert("source".to_string(), source.into());
        self
    }

    /// 设置模型信息
    pub fn set_model(
        &mut self,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> &mut Self {
        self.metadata
            .insert("model_id".to_string(), model_id.into());
        self.metadata
            .insert("model_name".to_string(), model_name.into());
        self
    }

    /// 添加元数据
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 获取元数据
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// 移除元数据
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }

    /// 清空元数据
    pub fn clear_metadata(&mut self) -> &mut Self {
        self.metadata.clear();
        self
    }

    /// 更新时间戳为当前时间
    pub fn update_timestamp(&mut self) -> &mut Self {
        self.timestamp = Utc::now();
        self
    }

    pub fn append_chunk(&mut self, chunk: &str) {
        if let Some(last_content) = self.contents.last_mut() {
            if last_content.is_text_chunk() {
                last_content.append_text(chunk);
            } else {
                self.contents
                    .push(MessageContent::TextChunk(chunk.to_string()));
            }
        } else {
            self.contents
                .push(MessageContent::TextChunk(chunk.to_string()));
        }
    }

    /// 检查消息是否为用户消息
    pub fn is_user(&self) -> bool {
        self.role == MessageRole::User
    }

    /// 检查消息是否为助手消息
    pub fn is_assistant(&self) -> bool {
        self.role == MessageRole::Assistant
    }

    /// 检查消息是否为系统消息
    pub fn is_system(&self) -> bool {
        self.role == MessageRole::System
    }

    /// 获取消息摘要（用于显示）
    pub fn get_summary(&self, max_length: usize) -> String {
        let text = self.get_text();
        if text.len() > max_length {
            format!("{}...", &text[..max_length])
        } else {
            text
        }
    }

    /// 清理消息（移除空内容）
    pub fn cleanup(&mut self) -> &mut Self {
        self
    }
}

/// 聊天流类型别名
pub type ChatStream = Pin<Box<dyn Stream<Item = anyhow::Result<MessageContent>> + Send>>;
