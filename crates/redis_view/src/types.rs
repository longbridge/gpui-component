//! Redis 核心类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::str::FromStr;
use thiserror::Error;
use rust_i18n::t;

/// Redis 错误类型
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Command error: {message}")]
    Command {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not connected to Redis")]
    NotConnected,

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
}

impl RedisError {
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            source: None,
        }
    }

    pub fn connection_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Connection {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn command(message: impl Into<String>) -> Self {
        Self::Command {
            message: message.into(),
            source: None,
        }
    }

    pub fn command_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Command {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

/// Redis 键类型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RedisKeyType {
    String,
    List,
    Set,
    ZSet,
    Hash,
    Stream,
    None,
}

impl RedisKeyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RedisKeyType::String => "string",
            RedisKeyType::List => "list",
            RedisKeyType::Set => "set",
            RedisKeyType::ZSet => "zset",
            RedisKeyType::Hash => "hash",
            RedisKeyType::Stream => "stream",
            RedisKeyType::None => "none",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RedisKeyType::String => "String",
            RedisKeyType::List => "List",
            RedisKeyType::Set => "Set",
            RedisKeyType::ZSet => "Sorted Set",
            RedisKeyType::Hash => "Hash",
            RedisKeyType::Stream => "Stream",
            RedisKeyType::None => "None",
        }
    }
}

impl FromStr for RedisKeyType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "string" => RedisKeyType::String,
            "list" => RedisKeyType::List,
            "set" => RedisKeyType::Set,
            "zset" => RedisKeyType::ZSet,
            "hash" => RedisKeyType::Hash,
            "stream" => RedisKeyType::Stream,
            _ => RedisKeyType::None,
        })
    }
}

/// Redis 值类型
#[derive(Clone, Debug, PartialEq)]
pub enum RedisValue {
    /// 空值
    Nil,
    /// 字符串
    String(String),
    /// 整数
    Integer(i64),
    /// 浮点数
    Float(f64),
    /// 批量数据（数组）
    Bulk(Vec<RedisValue>),
    /// 状态响应
    Status(String),
    /// 错误响应
    Error(String),
    /// 二进制数据
    Binary(Vec<u8>),
}

impl RedisValue {
    /// 转换为字符串表示
    pub fn as_string(&self) -> Option<&str> {
        match self {
            RedisValue::String(s) => Some(s),
            RedisValue::Status(s) => Some(s),
            _ => None,
        }
    }

    /// 转换为整数
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            RedisValue::Integer(i) => Some(*i),
            RedisValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// 转换为浮点数
    pub fn as_float(&self) -> Option<f64> {
        match self {
            RedisValue::Float(f) => Some(*f),
            RedisValue::Integer(i) => Some(*i as f64),
            RedisValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// 转换为字符串（用于显示）
    pub fn to_display_string(&self) -> String {
        match self {
            RedisValue::Nil => "(nil)".to_string(),
            RedisValue::String(s) => s.clone(),
            RedisValue::Integer(i) => i.to_string(),
            RedisValue::Float(f) => f.to_string(),
            RedisValue::Bulk(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_display_string()).collect();
                format!("[{}]", items.join(", "))
            }
            RedisValue::Status(s) => format!("OK: {}", s),
            RedisValue::Error(e) => format!("ERR: {}", e),
            RedisValue::Binary(b) => {
                if let Ok(s) = String::from_utf8(b.clone()) {
                    s
                } else {
                    format!("<binary: {} bytes>", b.len())
                }
            }
        }
    }

    /// 是否为空
    pub fn is_nil(&self) -> bool {
        matches!(self, RedisValue::Nil)
    }
}

/// 键信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyInfo {
    /// 键名
    pub name: String,
    /// 键类型
    pub key_type: RedisKeyType,
    /// TTL（秒），-1 表示永久，-2 表示不存在
    pub ttl: i64,
    /// 键大小（元素数量或字节数）
    pub size: Option<i64>,
    /// 内存使用量（字节）
    pub memory_usage: Option<i64>,
}

impl KeyInfo {
    pub fn new(name: String, key_type: RedisKeyType) -> Self {
        Self {
            name,
            key_type,
            ttl: -1,
            size: None,
            memory_usage: None,
        }
    }

    pub fn with_ttl(mut self, ttl: i64) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn with_size(mut self, size: i64) -> Self {
        self.size = Some(size);
        self
    }

    /// TTL 显示文本
    pub fn ttl_display(&self) -> String {
        match self.ttl {
            -2 => t!("RedisTTL.not_found").to_string(),
            -1 => t!("RedisTTL.permanent").to_string(),
            ttl if ttl >= 86400 => {
                t!("RedisTTL.days", days = ttl / 86400).to_string()
            }
            ttl if ttl >= 3600 => {
                t!("RedisTTL.hours", hours = ttl / 3600).to_string()
            }
            ttl if ttl >= 60 => {
                t!("RedisTTL.minutes", minutes = ttl / 60).to_string()
            }
            ttl => t!("RedisTTL.seconds", seconds = ttl).to_string(),
        }
    }
}

/// Hash 字段值对
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HashField {
    pub field: String,
    pub value: String,
}

/// ZSet 成员（带分数）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZSetMember {
    pub member: String,
    pub score: f64,
}

/// Stream 条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamEntry {
    pub id: String,
    pub fields: HashMap<String, String>,
}

/// Redis 连接配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedisConnectionConfig {
    /// 连接 ID
    #[serde(skip)]
    pub id: String,
    /// 连接名称
    #[serde(skip)]
    pub name: String,
    /// 主机地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 密码
    #[serde(default)]
    pub password: Option<String>,
    /// 用户名（Redis 6.0+）
    #[serde(default)]
    pub username: Option<String>,
    /// 默认数据库索引
    #[serde(default)]
    pub db_index: u8,
    /// 是否使用 TLS
    #[serde(default)]
    pub use_tls: bool,
    /// 连接超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 连接模式
    #[serde(default)]
    pub mode: RedisConnectionMode,
}

fn default_timeout() -> u64 {
    10
}

impl Default for RedisConnectionConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            host: "127.0.0.1".to_string(),
            port: 6379,
            password: None,
            username: None,
            db_index: 0,
            use_tls: false,
            timeout: 10,
            mode: RedisConnectionMode::Standalone,
        }
    }
}

impl RedisConnectionConfig {
    /// 构建 Redis 连接 URL
    pub fn to_url(&self) -> String {
        let scheme = if self.use_tls { "rediss" } else { "redis" };
        let auth = match (&self.username, &self.password) {
            (Some(user), Some(pass)) => format!(
                "{}:{}@",
                percent_encode_userinfo(user),
                percent_encode_userinfo(pass)
            ),
            (None, Some(pass)) => format!("default:{}@", percent_encode_userinfo(pass)),
            _ => String::new(),
        };
        format!(
            "{}://{}{}:{}/{}",
            scheme, auth, self.host, self.port, self.db_index
        )
    }

    /// 服务器信息显示
    pub fn server_info(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Redis 连接模式
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedisConnectionMode {
    /// 单机模式
    #[default]
    Standalone,
    /// 哨兵模式
    Sentinel,
    /// 集群模式
    Cluster,
}

impl RedisConnectionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RedisConnectionMode::Standalone => "standalone",
            RedisConnectionMode::Sentinel => "sentinel",
            RedisConnectionMode::Cluster => "cluster",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RedisConnectionMode::Standalone => "Standalone",
            RedisConnectionMode::Sentinel => "Sentinel",
            RedisConnectionMode::Cluster => "Cluster",
        }
    }
}

fn percent_encode_userinfo(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for &byte in value.as_bytes() {
        let needs_encoding = byte <= 0x1F
            || byte == 0x7F
            || matches!(byte, b'@' | b':' | b'/' | b'?' | b'#' | b'%');
        if needs_encoding {
            encoded.push_str(&format!("%{:02X}", byte));
        } else {
            encoded.push(byte as char);
        }
    }
    encoded
}

/// Redis 服务器信息
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RedisServerInfo {
    /// Redis 版本
    pub version: String,
    /// 运行模式
    pub mode: String,
    /// 操作系统
    pub os: String,
    /// 已连接客户端数
    pub connected_clients: i64,
    /// 已用内存
    pub used_memory: i64,
    /// 已用内存（人类可读）
    pub used_memory_human: String,
    /// 键总数
    pub total_keys: i64,
    /// 运行时间（秒）
    pub uptime_in_seconds: i64,
}

/// Redis 数据库信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedisDatabaseInfo {
    /// 数据库索引
    pub index: u8,
    /// 键数量
    pub keys: i64,
    /// 过期键数量
    pub expires: i64,
    /// 平均 TTL
    pub avg_ttl: i64,
}

/// 树形节点类型
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedisNodeType {
    /// 连接节点
    Connection,
    /// 数据库节点（0-15）
    Database(u8),
    /// 命名空间节点（虚拟分组）
    Namespace,
    /// 键节点
    Key(RedisKeyType),
    /// 加载更多
    LoadMore,
}

/// Redis 树形节点
#[derive(Clone, Debug)]
pub struct RedisNode {
    /// 节点唯一 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 节点类型
    pub node_type: RedisNodeType,
    /// 连接 ID
    pub connection_id: String,
    /// 数据库索引
    pub db_index: u8,
    /// 完整键名（仅对 Key 节点有效）
    pub full_key: Option<String>,
    /// 子节点
    pub children: Vec<RedisNode>,
    /// 子节点是否已加载
    pub children_loaded: bool,
    /// 键数量（对于 Namespace 节点）
    pub key_count: Option<i64>,
}

impl RedisNode {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        node_type: RedisNodeType,
        connection_id: impl Into<String>,
        db_index: u8,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            node_type,
            connection_id: connection_id.into(),
            db_index,
            full_key: None,
            children: Vec::new(),
            children_loaded: false,
            key_count: None,
        }
    }

    pub fn with_full_key(mut self, key: impl Into<String>) -> Self {
        self.full_key = Some(key.into());
        self
    }

    pub fn with_key_count(mut self, count: i64) -> Self {
        self.key_count = Some(count);
        self
    }

    pub fn set_children(&mut self, children: Vec<RedisNode>) {
        self.children = children;
        self.children_loaded = true;
    }

    /// 是否可展开
    pub fn is_expandable(&self) -> bool {
        matches!(
            self.node_type,
            RedisNodeType::Connection | RedisNodeType::Database(_) | RedisNodeType::Namespace
        )
    }
}

/// SCAN 操作结果
#[derive(Clone, Debug)]
pub struct ScanResult {
    /// 下一个游标
    pub cursor: u64,
    /// 扫描到的键列表
    pub keys: Vec<String>,
    /// 是否扫描完成
    pub finished: bool,
}

impl ScanResult {
    pub fn new(cursor: u64, keys: Vec<String>) -> Self {
        Self {
            cursor,
            keys,
            finished: cursor == 0,
        }
    }
}

/// 键值详情（用于显示）
#[derive(Clone, Debug)]
pub struct KeyValueDetail {
    /// 键信息
    pub key_info: KeyInfo,
    /// 值内容
    pub value: KeyValueContent,
}

/// 键值内容
#[derive(Clone, Debug)]
pub enum KeyValueContent {
    /// String 类型
    String(String),
    /// List 类型
    List(Vec<String>),
    /// Set 类型
    Set(Vec<String>),
    /// ZSet 类型
    ZSet(Vec<ZSetMember>),
    /// Hash 类型
    Hash(Vec<HashField>),
    /// Stream 类型
    Stream(Vec<StreamEntry>),
    /// 空或不存在
    None,
}

impl KeyValueContent {
    /// 获取元素数量
    pub fn len(&self) -> usize {
        match self {
            KeyValueContent::String(_) => 1,
            KeyValueContent::List(v) => v.len(),
            KeyValueContent::Set(v) => v.len(),
            KeyValueContent::ZSet(v) => v.len(),
            KeyValueContent::Hash(v) => v.len(),
            KeyValueContent::Stream(v) => v.len(),
            KeyValueContent::None => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
