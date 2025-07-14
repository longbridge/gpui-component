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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具名称
    #[serde(default)]
    pub name: String,
    /// 工具调用参数（JSON 字符串或键值对字符串）
    #[serde(default)]
    pub arguments: String,
}

impl ToolCall {
    pub fn id(&self) -> &str {
        self.name.split('@').next().unwrap_or(&self.name)
    }
    pub fn tool_name(&self) -> &str {
        self.name.split('@').nth(1).unwrap_or(&self.name)
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
    /// 多媒体内容（文本、图片、音频等）
    Parts(Vec<MediaContent>),
    /// 单个媒体内容
    Part(MediaContent),
    /// 工具调用
    ToolCall(ToolCall),
    /// 工具调用结果
    ToolResult(String,String),
    /// 流式文本块
    TextChunk(String),
    /// 可用工具列表（给模型的工具定义）
    ToolDefinitions(Vec<ToolDefinition>),
}

impl MessageContent {
    /// 创建文本内容
    pub fn text(content: impl Into<String>) -> Self {
        Self::Part(MediaContent::text(content))
    }

    /// 创建文本块（用于流式响应）
    pub fn chunk(content: impl Into<String>) -> Self {
        Self::TextChunk(content.into())
    }

    /// 创建工具调用内容
    pub fn tool_call(tool_call: ToolCall) -> Self {
        Self::ToolCall(tool_call)
    }

    /// 创建工具结果内容
    pub fn tool_result(name: impl Into<String>,result: impl Into<String>) -> Self {
        Self::ToolResult(name.into(),result.into())
    }

    /// 创建工具定义内容
    pub fn tool_definitions(tools: Vec<ToolDefinition>) -> Self {
        Self::ToolDefinitions(tools)
    }

    /// 创建单个媒体内容
    pub fn media(media: MediaContent) -> Self {
        Self::Part(media)
    }

    /// 创建多媒体内容
    pub fn multimodal(parts: Vec<MediaContent>) -> Self {
        Self::Parts(parts)
    }

    /// 检查是否为文本内容
    pub fn is_text(&self) -> bool {
        match self {
            Self::Part(media) => media.is_text(),
            Self::Parts(parts) => parts.len() == 1 && parts[0].is_text(),
            Self::TextChunk(_) => true,
            Self::ToolCall(_) | Self::ToolResult(_,_) | Self::ToolDefinitions(_) => false,
        }
    }

    /// 检查是否为工具调用
    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall(_))
    }

    /// 检查是否为工具结果
    pub fn is_tool_result(&self) -> bool {
        matches!(self, Self::ToolResult(_,_))
    }

    /// 检查是否为工具定义
    pub fn is_tool_definitions(&self) -> bool {
        matches!(self, Self::ToolDefinitions(_))
    }

    /// 检查是否为文本块
    pub fn is_text_chunk(&self) -> bool {
        matches!(self, Self::TextChunk(_))
    }

    /// 检查是否为媒体内容
    pub fn is_media(&self) -> bool {
        matches!(self, Self::Part(_) | Self::Parts(_))
    }

    /// 获取文本内容
    pub fn get_text(&self) -> String {
        match self {
            Self::Part(media) => {
                if let MediaData::Text(text) = &media.data {
                    text.clone()
                } else {
                    String::new()
                }
            }
            Self::Parts(parts) => {
                parts
                    .iter()
                    .filter_map(|part| {
                        if let MediaData::Text(text) = &part.data {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Self::TextChunk(text) => text.clone(),
            Self::ToolResult(_name,result) => result.clone(),
            Self::ToolCall(tool_call) => {
                format!("Tool: {} with args: {}", tool_call.name, tool_call.arguments)
            }
            Self::ToolDefinitions(tools) => {
                tools
                    .iter()
                    .map(|tool| format!("Tool: {} - {}", tool.name, tool.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    /// 获取工具调用
    pub fn get_tool_call(&self) -> Option<&ToolCall> {
        match self {
            Self::ToolCall(tool_call) => Some(tool_call),
            _ => None,
        }
    }

    /// 获取工具结果
    pub fn get_tool_result(&self) -> Option<(&str,&str)> {
        match self {
            Self::ToolResult(name,result) => Some((name,result)),
            _ => None,
        }
    }

    /// 获取工具定义
    pub fn get_tool_definitions(&self) -> Option<&[ToolDefinition]> {
        match self {
            Self::ToolDefinitions(tools) => Some(tools),
            _ => None,
        }
    }

    /// 获取媒体内容
    pub fn get_media(&self) -> Vec<&MediaContent> {
        match self {
            Self::Part(media) => vec![media],
            Self::Parts(parts) => parts.iter().collect(),
            _ => vec![],
        }
    }

    /// 获取可变的工具结果
    pub fn get_tool_result_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::ToolResult(_,result) => Some(result),
            _ => None,
        }
    }

    /// 获取可变的工具调用
    pub fn get_tool_call_mut(&mut self) -> Option<&mut ToolCall> {
        match self {
            Self::ToolCall(tool_call) => Some(tool_call),
            _ => None,
        }
    }

    /// 获取可变的工具定义
    pub fn get_tool_definitions_mut(&mut self) -> Option<&mut Vec<ToolDefinition>> {
        match self {
            Self::ToolDefinitions(tools) => Some(tools),
            _ => None,
        }
    }

    /// 添加文本到现有内容
    pub fn append_text(&mut self, text: &str) {
        match self {
            Self::TextChunk(existing) => existing.push_str(text),
            Self::ToolResult(_,result) => result.push_str(text),
            Self::Part(media) => {
                if let MediaData::Text(existing) = &mut media.data {
                    existing.push_str(text);
                } else {
                    *self = Self::TextChunk(text.to_string());
                }
            }
            Self::Parts(parts) => {
                if let Some(MediaContent {
                    data: MediaData::Text(existing),
                    ..
                }) = parts.first_mut()
                {
                    existing.push_str(text);
                } else {
                    parts.insert(0, MediaContent::text(text));
                }
            }
            Self::ToolCall(_) | Self::ToolDefinitions(_) => {
                *self = Self::TextChunk(text.to_string());
            }
        }
    }

    /// 添加媒体内容
    pub fn add_media(&mut self, media: MediaContent) {
        match self {
            Self::Parts(parts) => parts.push(media),
            Self::Part(existing) => {
                let existing_media = existing.clone();
                *self = Self::Parts(vec![existing_media, media]);
            }
            _ => {
                *self = Self::Part(media);
            }
        }
    }

    /// 添加工具定义
    pub fn add_tool_definition(&mut self, tool: ToolDefinition) {
        match self {
            Self::ToolDefinitions(tools) => tools.push(tool),
            _ => {
                *self = Self::ToolDefinitions(vec![tool]);
            }
        }
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Part(_) => false,
            Self::Parts(parts) => parts.is_empty(),
            Self::ToolCall(_) => false,
            Self::ToolResult(_,result) => result.is_empty(),
            Self::ToolDefinitions(tools) => tools.is_empty(),
            Self::TextChunk(text) => text.is_empty(),
        }
    }

    /// 获取内容长度
    pub fn len(&self) -> usize {
        match self {
            Self::Part(_) => 1,
            Self::Parts(parts) => parts.len(),
            Self::ToolCall(_) => 1,
            Self::ToolResult(_,_) => 1,
            Self::ToolDefinitions(tools) => tools.len(),
            Self::TextChunk(_) => 1,
        }
    }

    /// 转换为最终的文本内容
    pub fn finalize(self) -> Self {
        match self {
            Self::TextChunk(text) => Self::text(text),
            other => other,
        }
    }

    /// 克隆并转换为指定类型
    pub fn as_text_chunk(&self) -> Option<String> {
        match self {
            Self::TextChunk(text) => Some(text.clone()),
            _ => None,
        }
    }

    /// 克隆并转换为工具调用
    pub fn as_tool_call(&self) -> Option<ToolCall> {
        match self {
            Self::ToolCall(tool_call) => Some(tool_call.clone()),
            _ => None,
        }
    }

    /// 克隆并转换为工具结果
    pub fn as_tool_result(&self) -> Option<String> {
        match self {
            Self::ToolResult(name,result) => Some(format!("{}\n{}",name, result)),
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
            MediaData::Text(text) => Some(text),
            _ => None,
        }
    }

    /// 获取可变文本内容
    pub fn get_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.data {
            MediaData::Text(text) => Some(text),
            _ => None,
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
        Self {
            role,
            contents: Vec::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// 创建空消息
    pub fn assistant() -> Self {
        Self {
            role:MessageRole::Assistant,
            contents: Vec::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn system() -> Self {
        Self {
            role:MessageRole::System,
            contents: Vec::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
    pub fn user() -> Self {
        Self {
            role:MessageRole::User,
            contents: Vec::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
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
        self.contents.push(MessageContent::text(text));
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

     pub fn with_metadata(mut self, key: impl Into<String>,value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 添加内容
    pub fn add_content(&mut self, content: MessageContent)->&mut Self {
        self.contents.push(content);
        self
    }

    /// 添加多个内容
    pub fn add_contents(&mut self, contents: Vec<MessageContent>)->&mut Self {
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

    /// 获取所有工具调用
    pub fn get_tool_calls(&self) -> Vec<ToolCall> {
        let tool_calls: Vec<ToolCall> = self.contents
            .iter()
            .filter_map(|content| content.get_tool_call())
            .cloned()
            .collect();
        
        tool_calls
    }


    /// 获取第一个工具调用
    pub fn get_first_tool_call(&self) -> Option<&ToolCall> {
        self.contents
            .iter()
            .find_map(|content| content.get_tool_call())
    }

    /// 获取所有工具定义
    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.contents
            .iter()
            .filter_map(|content| content.get_tool_definitions())
            .flatten()
            .cloned()
            .collect()
    }

    /// 检查是否包含工具调用
    pub fn has_tool_calls(&self) -> bool {
        self.contents.iter().any(|content| content.is_tool_call())
    }

    /// 检查是否包含工具定义
    pub fn has_tool_definitions(&self) -> bool {
        self.contents.iter().any(|content| content.is_tool_definitions())
    }

    /// 检查是否为工具调用消息
    pub fn is_tool_call(&self) -> bool {
        self.contents.iter().any(|content| content.is_tool_call())
    }

    /// 检查是否为工具结果消息
    pub fn is_tool_result(&self) -> bool {
        self.contents.iter().any(|content| content.is_tool_result()) ||
        self.metadata.get("message_type") == Some(&"tool_result".to_string())
    }

    /// 检查是否为纯文本消息
    pub fn is_text_only(&self) -> bool {
        self.contents.len() == 1 && self.contents[0].is_text()
    }

    /// 检查是否为多模态消息
    pub fn is_multimodal(&self) -> bool {
        self.contents.len() > 1 || 
        self.contents.iter().any(|content| content.is_media())
    }

    /// 检查是否为流式文本块
    pub fn is_text_chunk(&self) -> bool {
        self.contents.len() == 1 && self.contents[0].is_text_chunk()
    }

    /// 检查是否为空消息
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty() || self.contents.iter().all(|content| content.is_empty())
    }

    /// 获取消息长度（内容数量）
    pub fn len(&self) -> usize {
        self.contents.len()
    }

    /// 获取文本长度
    pub fn text_len(&self) -> usize {
        self.get_text().len()
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

    /// 获取消息类型
    pub fn get_message_type(&self) -> Option<&str> {
        self.metadata.get("message_type").map(|s| s.as_str())
    }

    /// 设置消息来源
    pub fn set_source(&mut self, source: impl Into<String>) {
        self.metadata.insert("source".to_string(), source.into());
    }

    /// 设置模型信息
    pub fn set_model(&mut self, model_id: impl Into<String>, model_name: impl Into<String>)->&mut Self {
        self.metadata.insert("model_id".to_string(), model_id.into());
        self.metadata.insert("model_name".to_string(), model_name.into());
        self
    }

    /// 添加元数据
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>)->&mut Self {
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
    pub fn clear_metadata(&mut self) ->&mut Self{
        self.metadata.clear();
        self
    }

    /// 更新时间戳为当前时间
    pub fn update_timestamp(&mut self)->&mut Self {
        self.timestamp = Utc::now();
        self
    }

    pub fn append_text(&mut self, text: &str)->&mut Self {
        if let Some(last_content) = self.contents.last_mut() {
            last_content.append_text(text);
        } else {
            self.contents.push(MessageContent::text(text));
        }
        self
    }

    pub fn append_chunk(&mut self, chunk: &str) {
        if let Some(last_content) = self.contents.last_mut() {
            if last_content.is_text_chunk() {
                last_content.append_text(chunk);
            } else {
                self.contents.push(MessageContent::chunk(chunk));
            }
        } else {
            self.contents.push(MessageContent::chunk(chunk));
        }
    }

    pub fn finalize_chunks(&mut self) {
        for content in &mut self.contents {
            if content.is_text_chunk() {
                *content = content.clone().finalize();
            }
        }
    }


    /// 检查消息是否匹配指定角色
    pub fn is_role(&self, role: MessageRole) -> bool {
        self.role == role
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

    /// 验证消息是否有效
    pub fn validate(&self) -> Result<(), String> {
        if self.contents.is_empty() {
            return Err("消息内容不能为空".to_string());
        }

        for (i, content) in self.contents.iter().enumerate() {
            if content.is_empty() {
                return Err(format!("第 {} 个内容为空", i + 1));
            }
        }

        Ok(())
    }

    /// 清理消息（移除空内容）
    pub fn cleanup(&mut self) {
        self.contents.retain(|content| !content.is_empty());
    }

    /// 合并相邻的文本内容
    pub fn merge_text_contents(&mut self) {
        let mut merged_contents = Vec::new();
        let mut current_text = String::new();
        let mut has_text = false;

        for content in &self.contents {
            if content.is_text() || content.is_text_chunk() {
                current_text.push_str(&content.get_text());
                current_text.push('\n');
                has_text = true;
            } else {
                if has_text {
                    merged_contents.push(MessageContent::text(current_text.trim()));
                    current_text.clear();
                    has_text = false;
                }
                merged_contents.push(content.clone());
            }
        }

        if has_text {
            merged_contents.push(MessageContent::text(current_text.trim()));
        }

        self.contents = merged_contents;
    }
}


/// 聊天流类型别名
pub type ChatStream = Pin<Box<dyn Stream<Item = anyhow::Result<ChatMessage>> + Send>>;

