use futures::Stream;
/// 这个模型是为了提供一个通用的接口，用于处理记忆、工具调用和LLM交互。
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::pin::Pin;

/// 记忆类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
}

/// 记忆条目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub memory_type: MemoryType,
    pub timestamp: Option<u64>,
}

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
pub struct ToolCall {
    /// 工具名称
    pub name: String,
    /// 工具调用参数（JSON 字符串或键值对字符串）
    pub args: String,
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
    /// 工具调用
    ToolCall(ToolCall),
    /// 流式文本碎片
    Chunk(String),
}

/// 消息角色枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 流式响应类型
pub type ChatStream = Pin<Box<dyn Stream<Item = anyhow::Result<ChatMessage>> + Send>>;

/// 聊天消息结构 - 多模态版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息角色
    pub role: MessageRole,
    /// 多模态内容
    pub content: MessageContent,
    /// 消息ID（可选）
    pub id: Option<String>,
    /// 时间戳
    pub timestamp: Option<u64>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl ChatMessage {
    /// 创建文本消息的便捷方法
    pub fn text(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: MessageContent::text(content),
            id: None,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
            metadata: HashMap::new(),
        }
    }

    /// 创建用户文本消息
    pub fn user_text(content: impl Into<String>) -> Self {
        Self::text(MessageRole::User, content)
    }

    /// 创建助手文本消息
    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self::text(MessageRole::Assistant, content)
    }

    pub fn assistant_chunk(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::chunk(content),
            id: None,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
            metadata: HashMap::new(),
        }
    }

    /// 创建系统文本消息
    pub fn system_text(content: impl Into<String>) -> Self {
        Self::text(MessageRole::System, content)
    }

    /// 获取消息的文本内容 - 这是缺少的方法
    pub fn get_text(&self) -> String {
        self.content.get_text_content()
    }

    /// 获取消息的文本内容（别名方法）
    pub fn text_content(&self) -> String {
        self.get_text()
    }

    /// 创建多模态消息
    pub fn multimodal(role: MessageRole, content: MessageContent) -> Self {
        Self {
            role,
            content,
            id: None,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
            metadata: HashMap::new(),
        }
    }

    /// 创建工具调用消息
    pub fn tool_call(tool_call: ToolCall) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::tool_call(tool_call),
            id: None,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
            metadata: HashMap::new(),
        }
    }

    /// 创建工具响应消息
    pub fn tool_response(tool_name: &str, result: &str) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::text(format!("Tool {} result: {}", tool_name, result)),
            id: None,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("tool_name".to_string(), tool_name.to_string());
                meta
            },
        }
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 设置消息ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 检查是否为纯文本消息
    pub fn is_text_only(&self) -> bool {
        self.content.is_text_only()
    }

    /// 检查是否为工具调用消息
    pub fn is_tool_call(&self) -> bool {
        self.content.is_tool_call()
    }

    /// 检查是否为媒体消息
    pub fn is_media_message(&self) -> bool {
        self.content.is_media_content()
    }

    /// 添加工具调用到现有消息
    pub fn set_tool_call(&mut self, tool_call: ToolCall) {
        self.content.set_tool_call(tool_call);
    }

    /// 添加媒体内容到现有消息
    pub fn add_media(&mut self, media: MediaContent) {
        self.content.add_media(media);
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
        Self::Parts(vec![MediaContent::text(text)])
    }

    /// 创建文本流碎片
    pub fn chunk(text: impl Into<String>) -> Self {
        Self::Chunk(text.into())
    }

    /// 创建混合内容
    pub fn mixed(parts: Vec<MediaContent>) -> Self {
        Self::Parts(parts)
    }

    /// 创建工具调用内容
    pub fn tool_call(tool_call: ToolCall) -> Self {
        Self::ToolCall(tool_call)
    }

    /// 添加媒体部分
    pub fn add_media(&mut self, media: MediaContent) {
        match self {
            Self::Parts(parts) => parts.push(media),
            Self::ToolCall(_) | Self::Chunk(_) => {
                // 如果当前是工具调用或碎片，转换为混合类型
                *self = Self::Parts(vec![media]);
            }
        }
    }

    /// 设置工具调用
    pub fn set_tool_call(&mut self, tool_call: ToolCall) {
        *self = Self::ToolCall(tool_call);
    }

    /// 添加文本碎片（用于流式响应的累积）
    pub fn append_chunk(&mut self, chunk: &str) {
        match self {
            Self::Chunk(text) => text.push_str(chunk),
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
            Self::ToolCall(_) => {
                // 如果是工具调用，转换为文本碎片
                *self = Self::Chunk(chunk.to_string());
            }
        }
    }

    /// 获取所有文本内容
    pub fn get_text_content(&self) -> String {
        match self {
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
                format!("Tool: {} with args: {}", tool_call.name, tool_call.args)
            }
            Self::Chunk(text) => text.clone(),
        }
    }

    /// 获取特定媒体类别的内容
    pub fn get_media_by_type(&self, media_type: MediaType) -> Vec<&MediaContent> {
        match self {
            Self::Parts(parts) => parts
                .iter()
                .filter(|part| part.media_type() == media_type)
                .collect(),
            Self::ToolCall(_) | Self::Chunk(_) => vec![], // 工具调用和碎片不包含媒体内容
        }
    }

    /// 检查是否包含特定媒体类别
    pub fn contains_media_type(&self, media_type: MediaType) -> bool {
        match self {
            Self::Parts(parts) => parts.iter().any(|part| part.media_type() == media_type),
            Self::ToolCall(_) | Self::Chunk(_) => false,
        }
    }

    /// 获取工具调用
    pub fn get_tool_call(&self) -> Option<&ToolCall> {
        match self {
            Self::ToolCall(tool_call) => Some(tool_call),
            Self::Parts(_) | Self::Chunk(_) => None,
        }
    }

    /// 获取可变工具调用
    pub fn get_tool_call_mut(&mut self) -> Option<&mut ToolCall> {
        match self {
            Self::ToolCall(tool_call) => Some(tool_call),
            Self::Parts(_) | Self::Chunk(_) => None,
        }
    }

    /// 获取媒体部分
    pub fn get_parts(&self) -> Option<&[MediaContent]> {
        match self {
            Self::Parts(parts) => Some(parts),
            Self::ToolCall(_) | Self::Chunk(_) => None,
        }
    }

    /// 获取可变媒体部分
    pub fn get_parts_mut(&mut self) -> Option<&mut Vec<MediaContent>> {
        match self {
            Self::Parts(parts) => Some(parts),
            Self::ToolCall(_) | Self::Chunk(_) => None,
        }
    }

    /// 获取文本碎片
    pub fn get_chunk(&self) -> Option<&str> {
        match self {
            Self::Chunk(text) => Some(text),
            Self::Parts(_) | Self::ToolCall(_) => None,
        }
    }

    /// 获取可变文本碎片
    pub fn get_chunk_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Chunk(text) => Some(text),
            Self::Parts(_) | Self::ToolCall(_) => None,
        }
    }

    /// 检查是否为工具调用
    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall(_))
    }

    /// 检查是否为媒体内容
    pub fn is_media_content(&self) -> bool {
        matches!(self, Self::Parts(_))
    }

    /// 检查是否为文本碎片
    pub fn is_chunk(&self) -> bool {
        matches!(self, Self::Chunk(_))
    }

    /// 检查是否为纯文本
    pub fn is_text_only(&self) -> bool {
        match self {
            Self::Parts(parts) => parts.len() == 1 && parts[0].is_text(),
            Self::Chunk(_) => true,
            Self::ToolCall(_) => false,
        }
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Parts(parts) => parts.is_empty(),
            Self::ToolCall(_) => false, // 工具调用不为空
            Self::Chunk(text) => text.is_empty(),
        }
    }

    /// 获取内容长度
    pub fn len(&self) -> usize {
        match self {
            Self::Parts(parts) => parts.len(),
            Self::ToolCall(_) => 1,
            Self::Chunk(_) => 1,
        }
    }

    /// 将碎片转换为完整的文本内容
    pub fn finalize_chunk(self) -> Self {
        match self {
            Self::Chunk(text) => Self::text(text),
            other => other,
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: String,
}

impl ToolInfo {
    pub fn format_tool_name(provider_id: &str, tool_name: &str) -> String {
        format!("{}@{}", provider_id, tool_name)
    }
}
