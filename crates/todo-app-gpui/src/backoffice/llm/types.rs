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
    Tool, // 新增：工具返回值角色
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
    /// 流式文本块
    TextChunk(String),
    /// 可用工具列表（给模型的工具定义）
    ToolDefinitions(Vec<ToolDefinition>),
}

/// 流式响应类型
pub type ChatStream = Pin<Box<dyn Stream<Item = anyhow::Result<ChatMessage>> + Send>>;

/// 聊天消息结构 - 多模态版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息角色
    pub role: MessageRole,
    /// 多模态内容
    pub contents: Vec<MessageContent>,
    /// 时间戳（必须）
    pub timestamp: DateTime<Utc>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl ChatMessage {
    pub fn user() -> Self {
        Self {
            role: MessageRole::User,
            contents: vec![],
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
    pub fn system() -> Self {
        Self {
            role: MessageRole::System,
            contents: vec![],
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn assistant() -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![],
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn tool_result() -> Self {
        Self {
            role: MessageRole::Tool,
            contents: vec![],
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn tool_result_text(
        tool_name: impl Into<String>,
        content: impl Into<String>,
        success: bool,
    ) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("tool_name".to_string(), tool_name.into());
        metadata.insert("success".to_string(), success.to_string());

        Self {
            role: MessageRole::Tool,
            contents: vec![MessageContent::TextChunk(content.into())],
            timestamp: Utc::now(),
            metadata,
        }
    }

    /// 创建文本消息的便捷方法
    pub fn text(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            contents: vec![MessageContent::text(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// 创建带来源的文本消息
    pub fn text_with_source(
        role: MessageRole,
        content: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            role,
            contents: vec![MessageContent::text(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.into());
                meta
            },
        }
    }

    /// 创建带模型信息的文本消息
    pub fn text_with_model(
        role: MessageRole,
        content: impl Into<String>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            role,
            contents: vec![MessageContent::text(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("model_id".to_string(), model_id.into());
                meta.insert("model_name".to_string(), model_name.into());
                meta
            },
        }
    }

    /// 创建带完整信息的文本消息
    pub fn text_with_full_info(
        role: MessageRole,
        content: impl Into<String>,
        source: impl Into<String>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            role,
            contents: vec![MessageContent::text(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.into());
                meta.insert("model_id".to_string(), model_id.into());
                meta.insert("model_name".to_string(), model_name.into());
                meta
            },
        }
    }

    /// 创建用户文本消息
    pub fn user_text(content: impl Into<String>) -> Self {
        Self::text_with_source(MessageRole::User, content, "user")
    }

    /// 创建用户文本消息（带来源）
    pub fn user_text_with_source(content: impl Into<String>, source: impl Into<String>) -> Self {
        Self::text_with_source(MessageRole::User, content, source)
    }

    /// 创建用户文本消息（带模型信息）
    pub fn user_text_with_model(
        content: impl Into<String>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self::text_with_full_info(MessageRole::User, content, "user", model_id, model_name)
    }

    /// 创建助手文本消息
    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self::text_with_source(MessageRole::Assistant, content, "assistant")
    }

    /// 创建助手文本消息（带来源）
    pub fn assistant_text_with_source(
        content: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::text_with_source(MessageRole::Assistant, content, source)
    }

    /// 创建助手文本消息（带模型信息）
    pub fn assistant_text_with_model(
        content: impl Into<String>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self::text_with_full_info(
            MessageRole::Assistant,
            content,
            "assistant",
            model_id,
            model_name,
        )
    }

    /// 创建助手文本碎片消息（用于流式响应）
    pub fn assistant_chunk(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![MessageContent::chunk(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), "assistant_stream".to_string());
                meta
            },
        }
    }

    /// 创建助手文本碎片消息（带来源）
    pub fn assistant_chunk_with_source(
        content: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![MessageContent::chunk(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.into());
                meta
            },
        }
    }

    /// 创建助手文本碎片消息（带模型信息）
    pub fn assistant_chunk_with_model(
        content: impl Into<String>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![MessageContent::chunk(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), "assistant_stream".to_string());
                meta.insert("model_id".to_string(), model_id.into());
                meta.insert("model_name".to_string(), model_name.into());
                meta
            },
        }
    }

    /// 创建助手文本碎片消息（带完整信息）
    pub fn assistant_chunk_with_full_info(
        content: impl Into<String>,
        source: impl Into<String>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![MessageContent::chunk(content)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.into());
                meta.insert("model_id".to_string(), model_id.into());
                meta.insert("model_name".to_string(), model_name.into());
                meta
            },
        }
    }

    /// 创建系统文本消息
    pub fn system_text(content: impl Into<String>) -> Self {
        Self::text_with_source(MessageRole::System, content, "system")
    }

    /// 创建系统文本消息（带来源）
    pub fn system_text_with_source(content: impl Into<String>, source: impl Into<String>) -> Self {
        Self::text_with_source(MessageRole::System, content, source)
    }

    /// 创建系统文本消息（带模型信息）
    pub fn system_text_with_model(
        content: impl Into<String>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self::text_with_full_info(MessageRole::System, content, "system", model_id, model_name)
    }

    /// 创建工具定义消息（系统消息）
    pub fn tool_definitions(tools: Vec<ToolDefinition>) -> Self {
        Self {
            role: MessageRole::System,
            contents: vec![MessageContent::tool_definitions(tools)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), "tool_system".to_string());
                meta.insert("type".to_string(), "tool_definitions".to_string());
                meta
            },
        }
    }

    /// 创建工具定义消息（带来源）
    pub fn tool_definitions_with_source(
        tools: Vec<ToolDefinition>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::System,
            contents: vec![MessageContent::tool_definitions(tools)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.into());
                meta.insert("type".to_string(), "tool_definitions".to_string());
                meta
            },
        }
    }

    /// 创建工具定义消息（带模型信息）
    pub fn tool_definitions_with_model(
        tools: Vec<ToolDefinition>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::System,
            contents: vec![MessageContent::tool_definitions(tools)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), "tool_system".to_string());
                meta.insert("type".to_string(), "tool_definitions".to_string());
                meta.insert("model_id".to_string(), model_id.into());
                meta.insert("model_name".to_string(), model_name.into());
                meta
            },
        }
    }

    /// 创建工具调用消息
    pub fn tool_call(tool_call: ToolCall) -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![MessageContent::tool_call(tool_call)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), "assistant_tool".to_string());
                meta
            },
        }
    }

    /// 创建工具调用消息（带来源）
    pub fn tool_call_with_source(tool_call: ToolCall, source: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![MessageContent::tool_call(tool_call)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.into());
                meta
            },
        }
    }

    /// 创建工具调用消息（带模型信息）
    pub fn tool_call_with_model(
        tool_call: ToolCall,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            contents: vec![MessageContent::tool_call(tool_call)],
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), "assistant_tool".to_string());
                meta.insert("model_id".to_string(), model_id.into());
                meta.insert("model_name".to_string(), model_name.into());
                meta
            },
        }
    }

    /// 创建多模态消息
    pub fn multimodal(role: MessageRole, contents: Vec<MessageContent>) -> Self {
        Self {
            role,
            contents,
            // id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// 创建多模态消息（带来源）
    pub fn multimodal_with_source(
        role: MessageRole,
        contents: Vec<MessageContent>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            role,
            contents,
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.into());
                meta
            },
        }
    }

    /// 创建多模态消息（带模型信息）
    pub fn multimodal_with_model(
        role: MessageRole,
        contents: Vec<MessageContent>,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            role,
            contents,
            // id: None,
            timestamp: Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("model_id".to_string(), model_id.into());
                meta.insert("model_name".to_string(), model_name.into());
                meta
            },
        }
    }

    /// 设置模型信息
    pub fn with_model(
        mut self,
        model_id: impl Into<String>,
        model_name: impl Into<String>,
    ) -> Self {
        self.metadata
            .insert("model_id".to_string(), model_id.into());
        self.metadata
            .insert("model_name".to_string(), model_name.into());
        self
    }

    pub fn with_tool_definitions(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.contents.push(MessageContent::tool_definitions(tools));
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 设置时间戳
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// 设置当前时间戳
    pub fn with_current_timestamp(mut self) -> Self {
        self.timestamp = Utc::now();
        self
    }

    /// 设置工具结果的执行时间
    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.metadata
            .insert("execution_time_ms".to_string(), time_ms.to_string());
        self
    }

    /// 设置工具结果的错误信息
    pub fn with_tool_error(mut self, error: impl Into<String>) -> Self {
        self.metadata.insert("error".to_string(), error.into());
        self.metadata
            .insert("success".to_string(), "false".to_string());
        self
    }

    /// 设置消息来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.metadata.insert("source".to_string(), source.into());
        self
    }
}

impl ChatMessage {
    /// 获取模型ID
    pub fn get_model_id(&self) -> Option<&str> {
        self.metadata.get("model_id").map(|s| s.as_str())
    }

    /// 获取模型名称
    pub fn get_model_name(&self) -> Option<&str> {
        self.metadata.get("model_name").map(|s| s.as_str())
    }

    /// 检查是否来自特定模型
    pub fn is_from_model(&self, model_id: &str) -> bool {
        self.get_model_id() == Some(model_id)
    }

    /// 检查是否使用特定模型名称
    pub fn uses_model_name(&self, model_name: &str) -> bool {
        self.get_model_name() == Some(model_name)
    }

    /// 获取模型信息的完整描述
    pub fn model_info(&self) -> Option<String> {
        match (self.get_model_id(), self.get_model_name()) {
            (Some(id), Some(name)) => Some(format!("{} ({})", name, id)),
            (Some(id), None) => Some(id.to_string()),
            (None, Some(name)) => Some(name.to_string()),
            (None, None) => None,
        }
    }

    /// 获取消息的文本内容（从所有内容中提取）
    pub fn get_text(&self) -> String {
        self.contents
            .iter()
            .map(|content| content.get_text_content())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn get_text_without_tools(&self) -> String {
        self.contents
            .iter()
            .filter(|content| !content.is_tool_call() && !content.is_tool_definitions())
            .map(|content| content.get_text_content())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("")
    }

    /// 检查是否为纯文本消息
    pub fn is_text_only(&self) -> bool {
        self.contents.len() == 1 && self.contents[0].is_text_only()
    }

    /// 检查是否包含工具调用
    pub fn is_tool_call(&self) -> bool {
        self.contents.iter().any(|content| content.is_tool_call())
    }

    /// 检查是否包含工具定义
    pub fn has_tool_definitions(&self) -> bool {
        self.contents
            .iter()
            .any(|content| content.is_tool_definitions())
    }

    /// 获取所有工具定义
    pub fn get_tool_definitions(&self) -> Vec<&ToolDefinition> {
        self.contents
            .iter()
            .filter_map(|content| content.get_tool_definitions())
            .flatten()
            .collect()
    }

    /// 获取所有工具调用
    pub fn get_tool_calls(&self) -> Vec<&ToolCall> {
        self.contents
            .iter()
            .filter_map(|content| content.get_tool_call())
            .collect()
    }

    /// 检查是否包含媒体内容
    pub fn is_media_message(&self) -> bool {
        self.contents
            .iter()
            .any(|content| content.is_media_content())
    }

    /// 添加内容
    pub fn add_content(&mut self, content: MessageContent) {
        self.contents.push(content);
    }

    /// 添加工具调用到现有消息
    pub fn add_tool_call(&mut self, tool_call: ToolCall) {
        self.contents.push(MessageContent::tool_call(tool_call));
    }

    /// 添加媒体内容到现有消息
    pub fn add_media(&mut self, media: MediaContent) {
        self.contents.push(MessageContent::part(media));
    }

    /// 添加文本内容
    pub fn add_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.contents.push(MessageContent::text(text));
        self
    }

    pub fn add_text_chunk(&mut self, text: impl Into<String>) -> &mut Self {
        self.contents.push(MessageContent::TextChunk(text.into()));
        self
    }

    /// 获取消息来源
    pub fn get_source(&self) -> Option<&str> {
        self.metadata.get("source").map(|s| s.as_str())
    }

    /// 检查是否来自特定来源
    pub fn is_from_source(&self, source: &str) -> bool {
        self.get_source() == Some(source)
    }

    /// 检查是否为工具结果消息
    pub fn is_tool_result(&self) -> bool {
        self.role == MessageRole::Tool
    }

    /// 获取工具名称（从元数据中）
    pub fn get_tool_name(&self) -> Option<&str> {
        self.metadata.get("tool_name").map(|s| s.as_str())
    }

    /// 检查工具执行是否成功
    pub fn is_tool_success(&self) -> bool {
        self.metadata
            .get("success")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false)
    }

    /// 获取工具执行错误信息
    pub fn get_tool_error(&self) -> Option<&str> {
        self.metadata.get("error").map(|s| s.as_str())
    }

    /// 获取工具执行时间
    pub fn get_execution_time(&self) -> Option<u64> {
        self.metadata
            .get("execution_time_ms")
            .and_then(|s| s.parse::<u64>().ok())
    }

    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

impl MediaContent {
    /// 检查是否为文本内容
    pub fn is_text(&self) -> bool {
        matches!(self.data, MediaData::Text(_)) || self.mime_type.starts_with("text/")
    }

    /// 检查是否为图像内容
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    /// 检查是否为音频内容
    pub fn is_audio(&self) -> bool {
        self.mime_type.starts_with("audio/")
    }

    /// 检查是否为视频内容
    pub fn is_video(&self) -> bool {
        self.mime_type.starts_with("video/")
    }

    /// 检查是否为应用程序类型
    pub fn is_application(&self) -> bool {
        self.mime_type.starts_with("application/")
    }

    /// 检查是否为二进制数据
    pub fn is_binary(&self) -> bool {
        matches!(self.data, MediaData::Binary(_))
    }

    /// 检查是否为 Base64 编码
    pub fn is_base64(&self) -> bool {
        matches!(self.data, MediaData::Base64(_))
    }

    /// 检查是否为文件路径引用
    pub fn is_file_path(&self) -> bool {
        matches!(self.data, MediaData::FilePath(_))
    }

    /// 检查是否为 URL 引用
    pub fn is_url(&self) -> bool {
        matches!(self.data, MediaData::Url(_))
    }

    /// 获取媒体类型
    pub fn get_media_type(&self) -> MediaType {
        if self.is_text() {
            MediaType::Text
        } else if self.is_image() {
            MediaType::Image
        } else if self.is_audio() {
            MediaType::Audio
        } else if self.is_video() {
            MediaType::Video
        } else if self.is_application() {
            MediaType::Application
        } else {
            MediaType::Unknown
        }
    }

    /// 获取文本内容（如果是文本类型）
    pub fn get_text(&self) -> Option<&str> {
        match &self.data {
            MediaData::Text(text) => Some(text),
            _ => None,
        }
    }

    /// 获取可变文本内容（如果是文本类型）
    pub fn get_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.data {
            MediaData::Text(text) => Some(text),
            _ => None,
        }
    }

    /// 获取二进制数据（如果是二进制类型）
    pub fn get_binary(&self) -> Option<&[u8]> {
        match &self.data {
            MediaData::Binary(data) => Some(data),
            _ => None,
        }
    }

    /// 获取 Base64 数据（如果是 Base64 类型）
    pub fn get_base64(&self) -> Option<&str> {
        match &self.data {
            MediaData::Base64(data) => Some(data),
            _ => None,
        }
    }

    /// 获取文件路径（如果是文件路径类型）
    pub fn get_file_path(&self) -> Option<&str> {
        match &self.data {
            MediaData::FilePath(path) => Some(path),
            _ => None,
        }
    }

    /// 获取 URL（如果是 URL 类型）
    pub fn get_url(&self) -> Option<&str> {
        match &self.data {
            MediaData::Url(url) => Some(url),
            _ => None,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置文件名
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// 设置文件大小
    pub fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    /// 检查是否有描述
    pub fn has_description(&self) -> bool {
        self.description.is_some()
    }

    /// 检查是否有文件名
    pub fn has_filename(&self) -> bool {
        self.filename.is_some()
    }

    /// 检查是否有文件大小信息
    pub fn has_size(&self) -> bool {
        self.size_bytes.is_some()
    }

    /// 获取显示名称（优先文件名，否则使用描述或默认值）
    pub fn display_name(&self) -> String {
        self.filename
            .as_ref()
            .or(self.description.as_ref())
            .cloned()
            .unwrap_or_else(|| format!("{} content", self.mime_type))
    }

    /// 估算内容大小（字节）
    pub fn estimate_size(&self) -> Option<u64> {
        if let Some(size) = self.size_bytes {
            return Some(size);
        }

        match &self.data {
            MediaData::Text(text) => Some(text.len() as u64),
            MediaData::Binary(data) => Some(data.len() as u64),
            MediaData::Base64(data) => {
                // Base64 解码后的大小约为原始大小的 3/4
                Some((data.len() as f64 * 0.75) as u64)
            }
            MediaData::FilePath(_) | MediaData::Url(_) => None, // 无法估算远程内容大小
        }
    }

    /// 检查是否为支持的图像格式
    pub fn is_supported_image(&self) -> bool {
        matches!(
            self.mime_type.as_str(),
            "image/jpeg" | "image/jpg" | "image/png" | "image/gif" | "image/webp" | "image/svg+xml"
        )
    }

    /// 检查是否为支持的音频格式
    pub fn is_supported_audio(&self) -> bool {
        matches!(
            self.mime_type.as_str(),
            "audio/mpeg" | "audio/mp3" | "audio/wav" | "audio/ogg" | "audio/m4a"
        )
    }

    /// 检查是否为支持的视频格式
    pub fn is_supported_video(&self) -> bool {
        matches!(
            self.mime_type.as_str(),
            "video/mp4" | "video/mpeg" | "video/quicktime" | "video/webm"
        )
    }

    /// 检查是否为支持的文档格式
    pub fn is_supported_document(&self) -> bool {
        matches!(
            self.mime_type.as_str(),
            "application/pdf"
                | "application/msword"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                | "text/plain"
                | "text/markdown"
                | "text/html"
        )
    }
    /// 创建文本内容
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            mime_type: "text/plain".to_string(),
            data: MediaData::Text(content.into()),
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建图片内容
    pub fn image_jpeg(data: MediaData) -> Self {
        Self {
            mime_type: "image/jpeg".to_string(),
            data,
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建图片内容
    pub fn image_png(data: MediaData) -> Self {
        Self {
            mime_type: "image/png".to_string(),
            data,
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建音频内容
    pub fn audio_mp3(data: MediaData) -> Self {
        Self {
            mime_type: "audio/mpeg".to_string(),
            data,
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建视频内容
    pub fn video_mp4(data: MediaData) -> Self {
        Self {
            mime_type: "video/mp4".to_string(),
            data,
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 从 MIME 类型和数据创建
    pub fn from_mime_type(mime_type: impl Into<String>, data: MediaData) -> Self {
        Self {
            mime_type: mime_type.into(),
            data,
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建二进制内容
    pub fn binary(data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            mime_type: mime_type.into(),
            data: MediaData::Binary(data),
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建 Base64 内容
    pub fn base64(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            mime_type: mime_type.into(),
            data: MediaData::Base64(data.into()),
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建文件路径引用
    pub fn file_path(path: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            mime_type: mime_type.into(),
            data: MediaData::FilePath(path.into()),
            description: None,
            filename: None,
            size_bytes: None,
        }
    }

    /// 创建 URL 引用
    pub fn url(url: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            mime_type: mime_type.into(),
            data: MediaData::Url(url.into()),
            description: None,
            filename: None,
            size_bytes: None,
        }
    }
}

impl MessageContent {
    /// 创建纯文本内容
    pub fn text(text: impl Into<String>) -> Self {
        Self::Part(MediaContent::text(text))
    }

    /// 创建单个媒体内容
    pub fn part(media: MediaContent) -> Self {
        Self::Part(media)
    }

    /// 创建文本流碎片
    pub fn chunk(text: impl Into<String>) -> Self {
        Self::TextChunk(text.into())
    }

    /// 创建混合内容
    pub fn mixed(parts: Vec<MediaContent>) -> Self {
        Self::Parts(parts)
    }

    /// 创建工具调用内容
    pub fn tool_call(tool_call: ToolCall) -> Self {
        Self::ToolCall(tool_call)
    }

    /// 创建工具定义列表
    pub fn tool_definitions(tools: Vec<ToolDefinition>) -> Self {
        Self::ToolDefinitions(tools)
    }

    /// 添加媒体部分
    pub fn add_media(&mut self, media: MediaContent) {
        match self {
            Self::Parts(parts) => parts.push(media),
            Self::Part(existing) => {
                // 将单个媒体转换为多媒体列表
                let existing_media = existing.clone();
                *self = Self::Parts(vec![existing_media, media]);
            }
            Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => {
                // 如果当前是工具调用、碎片或工具定义，转换为单个媒体
                *self = Self::Part(media);
            }
        }
    }

    /// 设置工具调用
    pub fn set_tool_call(&mut self, tool_call: ToolCall) {
        *self = Self::ToolCall(tool_call);
    }

    /// 设置工具定义列表
    pub fn set_tool_definitions(&mut self, tools: Vec<ToolDefinition>) {
        *self = Self::ToolDefinitions(tools)
    }

    /// 添加工具定义
    pub fn add_tool_definition(&mut self, tool: ToolDefinition) {
        match self {
            Self::ToolDefinitions(tools) => tools.push(tool),
            _ => {
                // 如果当前不是工具定义列表，转换为工具定义列表
                *self = Self::ToolDefinitions(vec![tool]);
            }
        }
    }

    /// 添加文本碎片（用于流式响应的累积）
    pub fn append_chunk(&mut self, chunk: &str) {
        match self {
            Self::TextChunk(text) => text.push_str(chunk),
            Self::Part(media) => {
                if let MediaData::Text(text) = &mut media.data {
                    text.push_str(chunk);
                } else {
                    // 如果不是文本媒体，转换为文本碎片
                    *self = Self::TextChunk(chunk.to_string());
                }
            }
            Self::Parts(parts) => {
                if let Some(MediaContent {
                    data: MediaData::Text(text),
                    ..
                }) = parts.first_mut()
                {
                    text.push_str(chunk);
                } else {
                    parts.insert(0, MediaContent::text(chunk));
                }
            }
            Self::ToolCall(_) | Self::ToolDefinitions(_) => {
                // 如果是工具调用或工具定义，转换为文本碎片
                *self = Self::TextChunk(chunk.to_string());
            }
        }
    }

    /// 获取所有文本内容
    pub fn get_text_content(&self) -> String {
        match self {
            Self::Part(media) => {
                if media.is_text() {
                    if let MediaData::Text(text) = &media.data {
                        text.clone()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| {
                    if part.is_text() {
                        if let MediaData::Text(text) = &part.data {
                            Some(text.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Self::ToolCall(tool_call) => {
                // 返回工具调用的描述
                format!(
                    "Tool: {} with args: {}",
                    tool_call.name, tool_call.arguments
                )
            }
            Self::ToolDefinitions(tools) => {
                // 返回工具定义的描述
                tools
                    .iter()
                    .map(|tool| format!("Tool: {} - {}", tool.name, tool.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Self::TextChunk(text) => text.clone(),
        }
    }

    /// 获取工具调用
    pub fn get_tool_call(&self) -> Option<&ToolCall> {
        match self {
            Self::ToolCall(tool_call) => Some(tool_call),
            Self::Part(_) | Self::Parts(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => None,
        }
    }

    /// 获取可变工具调用
    pub fn get_tool_call_mut(&mut self) -> Option<&mut ToolCall> {
        match self {
            Self::ToolCall(tool_call) => Some(tool_call),
            Self::Part(_) | Self::Parts(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => None,
        }
    }

    /// 获取工具定义列表
    pub fn get_tool_definitions(&self) -> Option<&[ToolDefinition]> {
        match self {
            Self::ToolDefinitions(tools) => Some(tools),
            Self::Part(_) | Self::Parts(_) | Self::TextChunk(_) | Self::ToolCall(_) => None,
        }
    }

    /// 获取可变工具定义列表
    pub fn get_tool_definitions_mut(&mut self) -> Option<&mut Vec<ToolDefinition>> {
        match self {
            Self::ToolDefinitions(tools) => Some(tools),
            Self::Part(_) | Self::Parts(_) | Self::TextChunk(_) | Self::ToolCall(_) => None,
        }
    }

    /// 获取单个媒体部分
    pub fn get_part(&self) -> Option<&MediaContent> {
        match self {
            Self::Part(media) => Some(media),
            Self::Parts(_) | Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => {
                None
            }
        }
    }

    /// 获取可变单个媒体部分
    pub fn get_part_mut(&mut self) -> Option<&mut MediaContent> {
        match self {
            Self::Part(media) => Some(media),
            Self::Parts(_) | Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => {
                None
            }
        }
    }

    /// 获取媒体部分列表
    pub fn get_parts(&self) -> Option<&[MediaContent]> {
        match self {
            Self::Parts(parts) => Some(parts),
            Self::Part(_) | Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => {
                None
            }
        }
    }

    /// 获取可变媒体部分列表
    pub fn get_parts_mut(&mut self) -> Option<&mut Vec<MediaContent>> {
        match self {
            Self::Parts(parts) => Some(parts),
            Self::Part(_) | Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => {
                None
            }
        }
    }

    /// 获取所有媒体内容（包括单个和多个）
    pub fn get_all_media(&self) -> Vec<&MediaContent> {
        match self {
            Self::Part(media) => vec![media],
            Self::Parts(parts) => parts.iter().collect(),
            Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => vec![],
        }
    }

    /// 获取文本碎片
    pub fn get_chunk(&self) -> Option<&str> {
        match self {
            Self::TextChunk(text) => Some(text),
            Self::Part(_) | Self::Parts(_) | Self::ToolCall(_) | Self::ToolDefinitions(_) => None,
        }
    }

    /// 获取可变文本碎片
    pub fn get_chunk_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::TextChunk(text) => Some(text),
            Self::Part(_) | Self::Parts(_) | Self::ToolCall(_) | Self::ToolDefinitions(_) => None,
        }
    }

    /// 检查是否为文本碎片
    pub fn is_chunk(&self) -> bool {
        matches!(self, Self::TextChunk(_))
    }

    /// 检查是否为工具调用
    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall(_))
    }

    /// 检查是否为工具定义列表
    pub fn is_tool_definitions(&self) -> bool {
        matches!(self, Self::ToolDefinitions(_))
    }

    /// 检查是否为媒体内容（单个或多个）
    pub fn is_media_content(&self) -> bool {
        matches!(self, Self::Part(_) | Self::Parts(_))
    }

    /// 检查是否为单个媒体内容
    pub fn is_single_media(&self) -> bool {
        matches!(self, Self::Part(_))
    }

    /// 检查是否为多个媒体内容
    pub fn is_multi_media(&self) -> bool {
        matches!(self, Self::Parts(_))
    }

    /// 检查是否为纯文本
    pub fn is_text_only(&self) -> bool {
        match self {
            Self::Part(media) => media.is_text(),
            Self::Parts(parts) => parts.len() == 1 && parts[0].is_text(),
            Self::TextChunk(_) => true,
            Self::ToolCall(_) | Self::ToolDefinitions(_) => false,
        }
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Part(_) => false, // 单个媒体不为空
            Self::Parts(parts) => parts.is_empty(),
            Self::ToolCall(_) => false, // 工具调用不为空
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
            Self::ToolDefinitions(tools) => tools.len(),
            Self::TextChunk(_) => 1,
        }
    }

    /// 将碎片转换为完整的文本内容
    pub fn finalize_chunk(self) -> Self {
        match self {
            Self::TextChunk(text) => Self::text(text),
            other => other,
        }
    }
}

impl From<Vec<ToolDefinition>> for MessageContent {
    fn from(tools: Vec<ToolDefinition>) -> Self {
        Self::ToolDefinitions(tools)
    }
}

impl From<MediaContent> for MessageContent {
    fn from(media: MediaContent) -> Self {
        Self::Part(media)
    }
}

impl From<ToolCall> for MessageContent {
    fn from(tool_call: ToolCall) -> Self {
        Self::ToolCall(tool_call)
    }
}

impl From<&str> for MessageContent {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

impl From<String> for MessageContent {
    fn from(text: String) -> Self {
        Self::text(text)
    }
}

impl From<Vec<MediaContent>> for MessageContent {
    fn from(parts: Vec<MediaContent>) -> Self {
        Self::Parts(parts)
    }
}
