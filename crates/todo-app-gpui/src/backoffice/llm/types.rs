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

/// 消息角色枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
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
    pub fn add_text(&mut self, text: impl Into<String>) {
        self.contents.push(MessageContent::text(text));
    }

    pub fn add_text_chunk(&mut self, text: impl Into<String>) {
        self.contents.push(MessageContent::TextChunk(text.into()));
    }

    /// 设置消息来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.metadata.insert("source".to_string(), source.into());
        self
    }

    /// 获取消息来源
    pub fn get_source(&self) -> Option<&str> {
        self.metadata.get("source").map(|s| s.as_str())
    }

    // /// 设置消息ID
    // pub fn with_id(mut self, id: impl Into<String>) -> Self {
    //     self.id = Some(id.into());
    //     self
    // }

    pub fn with_tool_definitions(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.contents.push(MessageContent::tool_definitions(tools));
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 检查是否来自特定来源
    pub fn is_from_source(&self, source: &str) -> bool {
        self.get_source() == Some(source)
    }

    /// 检查是否来自用户
    pub fn is_from_user(&self) -> bool {
        self.get_source()
            .map(|s| s == "user" || s.starts_with("user_"))
            .unwrap_or(false)
    }

    /// 检查是否来自助手
    pub fn is_from_assistant(&self) -> bool {
        self.get_source()
            .map(|s| s == "assistant" || s.starts_with("assistant_"))
            .unwrap_or(false)
    }

    /// 检查是否来自系统
    pub fn is_from_system(&self) -> bool {
        self.get_source()
            .map(|s| s == "system" || s.starts_with("system_"))
            .unwrap_or(false)
    }

    /// 检查是否来自工具
    pub fn is_from_tool(&self) -> bool {
        self.get_source()
            .map(|s| s.contains("tool"))
            .unwrap_or(false)
    }

    /// 获取来源的类型（提取前缀）
    pub fn source_type(&self) -> Option<&str> {
        self.get_source().map(|s| s.split('_').next().unwrap_or(s))
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

    /// 获取时间戳
    pub fn get_timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// 检查消息是否在指定时间之后
    pub fn is_after(&self, time: DateTime<Utc>) -> bool {
        self.timestamp > time
    }

    /// 检查消息是否在指定时间之前
    pub fn is_before(&self, time: DateTime<Utc>) -> bool {
        self.timestamp < time
    }

    /// 获取消息年龄（距离现在的时间）
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.timestamp
    }

    /// 格式化时间戳为字符串
    pub fn format_timestamp(&self, format: &str) -> String {
        self.timestamp.format(format).to_string()
    }

    /// 获取格式化的时间戳（默认格式）
    pub fn formatted_timestamp(&self) -> String {
        self.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }

    /// 获取 ISO 8601 格式的时间戳
    pub fn iso_timestamp(&self) -> String {
        self.timestamp.to_rfc3339()
    }

    /// 从 Unix 时间戳创建（向后兼容）
    pub fn from_unix_timestamp(timestamp: u64) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(timestamp as i64, 0)
    }

    /// 转换为 Unix 时间戳（向后兼容）
    pub fn to_unix_timestamp(&self) -> u64 {
        self.timestamp.timestamp() as u64
    }

    /// 设置 Unix 时间戳
    pub fn with_unix_timestamp(mut self, timestamp: u64) -> Self {
        if let Some(dt) = Self::from_unix_timestamp(timestamp) {
            self.timestamp = dt;
        }
        self
    }
}
impl MediaContent {
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

    /// 从 MIME 类型推导媒体类别
    pub fn media_type(&self) -> MediaType {
        match self.mime_type.split('/').next().unwrap_or("") {
            "text" => MediaType::Text,
            "image" => MediaType::Image,
            "audio" => MediaType::Audio,
            "video" => MediaType::Video,
            "application" => match self.mime_type.as_str() {
                "application/pdf"
                | "application/msword"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                    MediaType::Document
                }
                _ => MediaType::Application,
            },
            _ => MediaType::Unknown,
        }
    }

    /// 检查是否为文本类型
    pub fn is_text(&self) -> bool {
        self.media_type() == MediaType::Text
    }

    /// 检查是否为图片类型
    pub fn is_image(&self) -> bool {
        self.media_type() == MediaType::Image
    }

    /// 添加描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 添加文件名
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// 添加文件大小
    pub fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
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

    /// 获取特定媒体类别的内容
    pub fn get_media_by_type(&self, media_type: MediaType) -> Vec<&MediaContent> {
        match self {
            Self::Part(media) => {
                if media.media_type() == media_type {
                    vec![media]
                } else {
                    vec![]
                }
            }
            Self::Parts(parts) => parts
                .iter()
                .filter(|part| part.media_type() == media_type)
                .collect(),
            Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => vec![], // 工具相关内容不包含媒体
        }
    }

    /// 检查是否包含特定媒体类别
    pub fn contains_media_type(&self, media_type: MediaType) -> bool {
        match self {
            Self::Part(media) => media.media_type() == media_type,
            Self::Parts(parts) => parts.iter().any(|part| part.media_type() == media_type),
            Self::ToolCall(_) | Self::TextChunk(_) | Self::ToolDefinitions(_) => false,
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
