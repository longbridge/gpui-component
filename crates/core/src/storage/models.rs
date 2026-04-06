use crate::cloud_sync::sync_type::SyncableItem;
use crate::crypto;
use crate::storage::traits::Entity;
use gpui::Global;
use gpui_component::Size::Large;
use gpui_component::{Icon, IconName, Sizable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;

/// 活跃连接状态 - 用于跟踪哪些连接当前已打开
#[derive(Default)]
pub struct ActiveConnections {
    active_ids: HashSet<i64>,
}

impl Global for ActiveConnections {}

impl ActiveConnections {
    pub fn new() -> Self {
        Self {
            active_ids: HashSet::new(),
        }
    }

    pub fn add(&mut self, conn_id: i64) {
        self.active_ids.insert(conn_id);
    }

    pub fn remove(&mut self, conn_id: i64) {
        self.active_ids.remove(&conn_id);
    }

    pub fn is_active(&self, conn_id: i64) -> bool {
        self.active_ids.contains(&conn_id)
    }

    pub fn active_count(&self) -> usize {
        self.active_ids.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConnectionType {
    All,
    Database,
    SshSftp,
    Redis,
    MongoDB,
    ChatDB,
    Serial,
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ConnectionType::All => "All",
            ConnectionType::Database => "Database",
            ConnectionType::SshSftp => "SshSftp",
            ConnectionType::Redis => "Redis",
            ConnectionType::MongoDB => "MongoDB",
            ConnectionType::ChatDB => "ChatDB",
            ConnectionType::Serial => "Serial",
        };
        write!(f, "{}", s)
    }
}

impl ConnectionType {
    pub fn all() -> Vec<ConnectionType> {
        vec![
            ConnectionType::All,
            ConnectionType::Database,
            ConnectionType::SshSftp,
            ConnectionType::Redis,
            ConnectionType::MongoDB,
            ConnectionType::ChatDB,
            ConnectionType::Serial,
        ]
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Database" => ConnectionType::Database,
            "SshSftp" => ConnectionType::SshSftp,
            "Redis" => ConnectionType::Redis,
            "MongoDB" => ConnectionType::MongoDB,
            "ChatDB" => ConnectionType::ChatDB,
            "Serial" => ConnectionType::Serial,
            _ => ConnectionType::Database,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ConnectionType::All => "All",
            ConnectionType::Database => "Database",
            ConnectionType::SshSftp => "SSH/SFTP",
            ConnectionType::Redis => "Redis",
            ConnectionType::MongoDB => "MongoDB",
            ConnectionType::ChatDB => "ChatDB",
            ConnectionType::Serial => "Serial",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            ConnectionType::All => IconName::Server,
            ConnectionType::Database => IconName::Database,
            ConnectionType::SshSftp => IconName::TerminalColor,
            ConnectionType::Redis => IconName::Redis,
            ConnectionType::MongoDB => IconName::MongoDB,
            ConnectionType::ChatDB => IconName::AI,
            ConnectionType::Serial => IconName::SerialPort,
        }
    }
}

/// Database type enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    SQLite,
    DuckDB,
    MSSQL,
    Oracle,
    ClickHouse,
}

impl DatabaseType {
    pub fn all() -> &'static [DatabaseType] {
        &[
            DatabaseType::MySQL,
            DatabaseType::PostgreSQL,
            DatabaseType::SQLite,
            DatabaseType::DuckDB,
            DatabaseType::MSSQL,
            DatabaseType::Oracle,
            DatabaseType::ClickHouse,
        ]
    }

    pub fn as_str(&self) -> &str {
        match self {
            DatabaseType::MySQL => "MySQL",
            DatabaseType::PostgreSQL => "PostgreSQL",
            DatabaseType::SQLite => "SQLite",
            DatabaseType::DuckDB => "DuckDB",
            DatabaseType::MSSQL => "MSSQL",
            DatabaseType::Oracle => "Oracle",
            DatabaseType::ClickHouse => "ClickHouse",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "MySQL" => Some(DatabaseType::MySQL),
            "PostgreSQL" => Some(DatabaseType::PostgreSQL),
            "SQLite" => Some(DatabaseType::SQLite),
            "DuckDB" => Some(DatabaseType::DuckDB),
            "MSSQL" => Some(DatabaseType::MSSQL),
            "Oracle" => Some(DatabaseType::Oracle),
            "ClickHouse" => Some(DatabaseType::ClickHouse),
            _ => None,
        }
    }

    pub fn as_icon(&self) -> Icon {
        match self {
            DatabaseType::MySQL => IconName::MySQLColor.color().with_size(Large),
            DatabaseType::PostgreSQL => IconName::PostgreSQLColor.color().with_size(Large),
            DatabaseType::SQLite => IconName::SQLiteColor.color().with_size(Large),
            DatabaseType::DuckDB => IconName::DuckDB.color().with_size(Large),
            DatabaseType::MSSQL => IconName::MSSQLColor.color().with_size(Large),
            DatabaseType::Oracle => IconName::OracleColor.color().with_size(Large),
            DatabaseType::ClickHouse => IconName::ClickHouseColor.color().with_size(Large),
        }
    }
    pub fn as_node_icon(&self) -> Icon {
        match self {
            DatabaseType::MySQL => IconName::MySQLLineColor.color().with_size(Large),
            DatabaseType::PostgreSQL => IconName::PostgreSQLLineColor.color().with_size(Large),
            DatabaseType::SQLite => IconName::SQLiteLineColor.color().with_size(Large),
            DatabaseType::DuckDB => IconName::DuckDB.color().with_size(Large),
            DatabaseType::MSSQL => IconName::MSSQLLineColor.color().with_size(Large),
            DatabaseType::Oracle => IconName::OracleLineColor.color().with_size(Large),
            DatabaseType::ClickHouse => IconName::ClickHouseLineColor.color().with_size(Large),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshParams {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: SshAuthMethod,
    /// 连接超时（秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_timeout: Option<u64>,
    /// 心跳间隔（秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keepalive_interval: Option<u64>,
    /// 最大心跳失败次数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keepalive_max: Option<usize>,
    /// 默认工作目录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_directory: Option<String>,
    /// 初始化脚本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_script: Option<String>,
    /// 跳板机配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jump_server: Option<JumpServerConfig>,
    /// 代理配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxyConfig>,
}

/// 跳板机配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JumpServerConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: SshAuthMethod,
}

/// 代理类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyType {
    Socks5,
    Http,
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SshAuthMethod {
    Password {
        password: String,
    },
    PrivateKey {
        key_path: String,
        passphrase: Option<String>,
    },
    Agent,
    AutoPublicKey,
}

/// Redis 连接模式
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedisMode {
    /// 单机模式
    #[default]
    Standalone,
    /// 哨兵模式
    Sentinel,
    /// 集群模式
    Cluster,
}

/// Redis 哨兵配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisSentinelConfig {
    /// 主节点名称
    pub master_name: String,
    /// 哨兵节点列表（host:port）
    pub sentinels: Vec<String>,
    /// 哨兵密码
    pub sentinel_password: Option<String>,
}

/// Redis 集群节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisClusterConfig {
    /// 集群节点列表（host:port）
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisParams {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub username: Option<String>,
    pub db_index: u8,
    /// 连接模式
    #[serde(default)]
    pub mode: RedisMode,
    /// 是否启用 TLS
    #[serde(default)]
    pub use_tls: bool,
    /// 连接超时（秒）
    #[serde(default)]
    pub connect_timeout: Option<u64>,
    /// 哨兵配置
    #[serde(default)]
    pub sentinel: Option<RedisSentinelConfig>,
    /// 集群配置
    #[serde(default)]
    pub cluster: Option<RedisClusterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoDBParams {
    #[serde(default)]
    pub connection_string: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub auth_source: Option<String>,
    #[serde(default)]
    pub replica_set: Option<String>,
    #[serde(default)]
    pub read_preference: Option<String>,
    #[serde(default)]
    pub use_srv_record: bool,
    #[serde(default)]
    pub direct_connection: bool,
    #[serde(default)]
    pub use_tls: bool,
    #[serde(default)]
    pub connect_timeout_seconds: Option<u64>,
    #[serde(default)]
    pub application_name: Option<String>,
}

/// 串口校验位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SerialParity {
    #[default]
    None,
    Odd,
    Even,
}

impl SerialParity {
    pub fn all() -> &'static [SerialParity] {
        &[SerialParity::None, SerialParity::Odd, SerialParity::Even]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SerialParity::None => "None",
            SerialParity::Odd => "Odd",
            SerialParity::Even => "Even",
        }
    }
}

/// 串口流控
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SerialFlowControl {
    #[default]
    None,
    Software,
    Hardware,
}

impl SerialFlowControl {
    pub fn all() -> &'static [SerialFlowControl] {
        &[
            SerialFlowControl::None,
            SerialFlowControl::Software,
            SerialFlowControl::Hardware,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SerialFlowControl::None => "None",
            SerialFlowControl::Software => "XON/XOFF",
            SerialFlowControl::Hardware => "RTS/CTS",
        }
    }
}

/// 串口连接参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialParams {
    /// 串口设备路径，如 /dev/ttyUSB0 或 COM1
    pub port_name: String,
    /// 波特率
    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,
    /// 数据位 (5/6/7/8)
    #[serde(default = "default_data_bits")]
    pub data_bits: u8,
    /// 停止位 (1/2)
    #[serde(default = "default_stop_bits")]
    pub stop_bits: u8,
    /// 校验位
    #[serde(default)]
    pub parity: SerialParity,
    /// 流控
    #[serde(default)]
    pub flow_control: SerialFlowControl,
}

fn default_baud_rate() -> u32 {
    115200
}

fn default_data_bits() -> u8 {
    8
}

fn default_stop_bits() -> u8 {
    1
}

impl Default for SerialParams {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: SerialParity::None,
            flow_control: SerialFlowControl::None,
        }
    }
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectionConfig {
    #[serde(skip)]
    pub id: String,
    pub database_type: DatabaseType,
    #[serde(skip)]
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: Option<String>,
    pub service_name: Option<String>,
    pub sid: Option<String>,
    #[serde(skip)]
    pub workspace_id: Option<i64>,
    #[serde(default)]
    pub extra_params: std::collections::HashMap<String, String>,
}

impl DbConnectionConfig {
    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.extra_params.get(key)
    }

    pub fn get_param_as<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.extra_params.get(key).and_then(|v| v.parse().ok())
    }

    pub fn get_param_bool(&self, key: &str) -> bool {
        self.extra_params
            .get(key)
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }

    pub fn server_info(&self) -> String {
        match self.database_type {
            DatabaseType::SQLite | DatabaseType::DuckDB => format!("{}", self.host),
            _ => format!("{}:{}", self.host, self.port),
        }
    }

    pub fn is_change(&self, other: &DbConnectionConfig) -> bool {
        self.host != other.host
            || self.port != other.port
            || self.username != other.username
            || self.password != other.password
            || self.database != other.database
            || self.service_name != other.service_name
            || self.sid != other.sid
            || self.extra_params != other.extra_params
    }
}

/// Workspace for organizing connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    /// 云端 ID（用于同步）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_id: Option<String>,
}

impl Entity for Workspace {
    fn id(&self) -> Option<i64> {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at
            .expect("created_at 在从数据库读取后应该存在")
    }

    fn updated_at(&self) -> i64 {
        self.updated_at
            .expect("updated_at 在从数据库读取后应该存在")
    }
}

impl Workspace {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            color: None,
            icon: None,
            created_at: None,
            updated_at: None,
            cloud_id: None,
        }
    }
}

impl SyncableItem for Workspace {
    fn local_id(&self) -> Option<i64> {
        self.id
    }

    fn set_local_id(&mut self, id: Option<i64>) {
        self.id = id;
    }

    fn item_name(&self) -> &str {
        &self.name
    }

    fn cloud_id(&self) -> Option<&str> {
        self.cloud_id.as_deref()
    }

    fn set_cloud_id(&mut self, cloud_id: Option<String>) {
        self.cloud_id = cloud_id;
    }

    fn updated_at(&self) -> Option<i64> {
        self.updated_at
    }
}

/// Stored connection with ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredConnection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub name: String,
    pub connection_type: ConnectionType,
    pub params: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<i64>,
    /// 已选中的数据库ID列表（JSON数组），None表示全选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_databases: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    /// 是否启用云同步（默认 true）
    #[serde(default = "default_sync_enabled")]
    pub sync_enabled: bool,
    /// 云端记录 ID（同步成功后获得）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_id: Option<String>,
    /// 最后同步时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    /// 团队归属 ID（None = 个人数据）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// 连接创建者 ID（用户 UUID，用于权限判断）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
}

fn default_sync_enabled() -> bool {
    true
}

impl Entity for StoredConnection {
    fn id(&self) -> Option<i64> {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at
            .expect("created_at 在从数据库读取后应该存在")
    }

    fn updated_at(&self) -> i64 {
        self.updated_at
            .expect("updated_at 在从数据库读取后应该存在")
    }
}

impl SyncableItem for StoredConnection {
    fn local_id(&self) -> Option<i64> {
        self.id
    }

    fn set_local_id(&mut self, id: Option<i64>) {
        self.id = id;
    }

    fn item_name(&self) -> &str {
        &self.name
    }

    fn cloud_id(&self) -> Option<&str> {
        self.cloud_id.as_deref()
    }

    fn set_cloud_id(&mut self, cloud_id: Option<String>) {
        self.cloud_id = cloud_id;
    }

    fn updated_at(&self) -> Option<i64> {
        self.updated_at
    }

    fn is_sync_enabled(&self) -> bool {
        self.sync_enabled
    }

    fn last_synced_at(&self) -> Option<i64> {
        self.last_synced_at
    }

    fn team_id(&self) -> Option<&str> {
        self.team_id.as_deref()
    }
}

impl StoredConnection {
    pub fn new_database(
        name: String,
        params: DbConnectionConfig,
        workspace_id: Option<i64>,
    ) -> Self {
        Self {
            id: None,
            name,
            connection_type: ConnectionType::Database,
            params: serde_json::to_string(&params).expect("DbConnectionConfig 序列化不应失败"),
            workspace_id,
            selected_databases: if let Some(database) = &params.database {
                Some(format!("[\"{}\"]", database))
            } else {
                None
            },
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    pub fn new_ssh(name: String, params: SshParams, workspace_id: Option<i64>) -> Self {
        Self {
            id: None,
            name,
            connection_type: ConnectionType::SshSftp,
            params: serde_json::to_string(&params).expect("SshParams 序列化不应失败"),
            workspace_id,
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    pub fn new_redis(name: String, params: RedisParams, workspace_id: Option<i64>) -> Self {
        Self {
            id: None,
            name,
            connection_type: ConnectionType::Redis,
            params: serde_json::to_string(&params).expect("RedisParams 序列化不应失败"),
            workspace_id,
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    pub fn new_mongodb(name: String, params: MongoDBParams, workspace_id: Option<i64>) -> Self {
        Self {
            id: None,
            name,
            connection_type: ConnectionType::MongoDB,
            params: serde_json::to_string(&params).expect("MongoDBParams 序列化不应失败"),
            workspace_id,
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    pub fn to_ssh_params(&self) -> Result<SshParams, serde_json::Error> {
        serde_json::from_str(&self.params)
    }

    pub fn to_redis_params(&self) -> Result<RedisParams, serde_json::Error> {
        serde_json::from_str(&self.params)
    }

    pub fn to_mongodb_params(&self) -> Result<MongoDBParams, serde_json::Error> {
        serde_json::from_str(&self.params)
    }

    pub fn new_serial(name: String, params: SerialParams, workspace_id: Option<i64>) -> Self {
        Self {
            id: None,
            name,
            connection_type: ConnectionType::Serial,
            params: serde_json::to_string(&params).expect("SerialParams 序列化不应失败"),
            workspace_id,
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    pub fn to_serial_params(&self) -> Result<SerialParams, serde_json::Error> {
        serde_json::from_str(&self.params)
    }

    pub fn to_db_connection(&self) -> Result<DbConnectionConfig, serde_json::Error> {
        let mut params: DbConnectionConfig = serde_json::from_str(&self.params)?;
        params.name = self.name.clone();
        params.workspace_id = self.workspace_id;
        params.id = self.id.unwrap_or(0).to_string();
        Ok(params)
    }

    pub fn from_db_connection(connection: DbConnectionConfig) -> Self {
        let name = connection.name.clone();
        let workspace_id = connection.workspace_id.clone();
        Self::new_database(name, connection, workspace_id)
    }

    /// 获取已选中的数据库列表，None表示全选
    pub fn get_selected_databases(&self) -> Option<Vec<String>> {
        self.selected_databases
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
    }

    /// 设置已选中的数据库列表，None表示全选
    pub fn set_selected_databases(&mut self, databases: Option<Vec<String>>) {
        self.selected_databases =
            databases.map(|dbs| serde_json::to_string(&dbs).unwrap_or_default());
    }

    /// 对 params 中的敏感字段进行加密，返回加密后的 params 字符串。
    /// 敏感字段包括：password、passphrase 以及嵌套结构中的同名字段。
    pub fn encrypt_params(&self) -> String {
        encrypt_json_passwords(&self.params)
    }

    /// 对 params 中的加密字段进行解密，返回解密后的 params 字符串。
    pub fn decrypt_params(&self) -> String {
        decrypt_json_passwords(&self.params)
    }

    /// 返回一个新的 StoredConnection，其 params 中的密码字段已解密
    pub fn with_decrypted_params(&self) -> Self {
        let mut cloned = self.clone();
        cloned.params = cloned.decrypt_params();
        cloned
    }
}

/// 递归加密 JSON 中所有名为 password 或 passphrase 的字符串字段
fn encrypt_json_passwords(json_str: &str) -> String {
    match serde_json::from_str::<Value>(json_str) {
        Ok(mut value) => {
            encrypt_value(&mut value);
            serde_json::to_string(&value).unwrap_or_else(|_| json_str.to_string())
        }
        Err(_) => json_str.to_string(),
    }
}

/// 递归解密 JSON 中所有名为 password 或 passphrase 的字符串字段
fn decrypt_json_passwords(json_str: &str) -> String {
    match serde_json::from_str::<Value>(json_str) {
        Ok(mut value) => {
            decrypt_value(&mut value);
            serde_json::to_string(&value).unwrap_or_else(|_| json_str.to_string())
        }
        Err(_) => json_str.to_string(),
    }
}

/// 判断字段名是否为敏感字段
fn is_sensitive_field(key: &str) -> bool {
    key == "password"
        || key == "passphrase"
        || key.ends_with("_password")
        || key.ends_with("_passphrase")
}

/// 递归遍历 JSON Value，加密敏感字段
fn encrypt_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if is_sensitive_field(key) {
                    if let Value::String(s) = val {
                        *s = crypto::encrypt_password(s);
                    }
                } else {
                    encrypt_value(val);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                encrypt_value(item);
            }
        }
        _ => {}
    }
}

/// 递归遍历 JSON Value，解密敏感字段
fn decrypt_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if is_sensitive_field(key) {
                    if let Value::String(s) = val {
                        *s = crypto::decrypt_password(s);
                    }
                } else {
                    decrypt_value(val);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                decrypt_value(item);
            }
        }
        _ => {}
    }
}

/// 检测 params 中是否存在“已加密字段解密失败”的情况。
///
/// 规则：敏感字段（password/passphrase）若以 ENC: 开头，且解密结果为空，视为失败。
pub fn has_decrypt_failure_in_sensitive_fields(json_str: &str) -> bool {
    match serde_json::from_str::<Value>(json_str) {
        Ok(value) => has_decrypt_failure_in_value(&value),
        Err(_) => false,
    }
}

fn has_decrypt_failure_in_value(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, val)| {
            if is_sensitive_field(key) {
                if let Value::String(s) = val {
                    return crypto::is_encrypted(s) && crypto::decrypt_password(s).is_empty();
                }
                false
            } else {
                has_decrypt_failure_in_value(val)
            }
        }),
        Value::Array(arr) => arr.iter().any(has_decrypt_failure_in_value),
        _ => false,
    }
}

/// Generic key-value storage model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

impl KeyValue {
    pub fn new(key: String, value: String) -> Self {
        Self {
            id: None,
            key,
            value,
            created_at: None,
            updated_at: None,
        }
    }
}

pub fn parse_db_type(s: &str) -> DatabaseType {
    match s {
        "MySQL" => DatabaseType::MySQL,
        "PostgreSQL" => DatabaseType::PostgreSQL,
        "SQLite" => DatabaseType::SQLite,
        "DuckDB" => DatabaseType::DuckDB,
        _ => DatabaseType::MySQL,
    }
}

#[cfg(test)]
mod serial_tests {
    use super::*;

    #[test]
    fn serial_params_serialize_deserialize() {
        let params = SerialParams {
            port_name: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: SerialParity::None,
            flow_control: SerialFlowControl::None,
        };
        let json = serde_json::to_string(&params).unwrap();
        let p2: SerialParams = serde_json::from_str(&json).unwrap();
        assert_eq!(p2.port_name, "/dev/ttyUSB0");
        assert_eq!(p2.baud_rate, 115200);
        assert_eq!(p2.data_bits, 8);
        assert_eq!(p2.stop_bits, 1);
        assert_eq!(p2.parity, SerialParity::None);
        assert_eq!(p2.flow_control, SerialFlowControl::None);
    }

    #[test]
    fn serial_params_defaults_from_minimal_json() {
        let json = r#"{"port_name":"/dev/tty0"}"#;
        let p: SerialParams = serde_json::from_str(json).unwrap();
        assert_eq!(p.port_name, "/dev/tty0");
        assert_eq!(p.baud_rate, 115200);
        assert_eq!(p.data_bits, 8);
        assert_eq!(p.stop_bits, 1);
        assert_eq!(p.parity, SerialParity::None);
        assert_eq!(p.flow_control, SerialFlowControl::None);
    }

    #[test]
    fn stored_connection_serial_roundtrip() {
        let params = SerialParams {
            port_name: "/dev/cu.usbserial-1420".to_string(),
            baud_rate: 9600,
            data_bits: 7,
            stop_bits: 2,
            parity: SerialParity::Even,
            flow_control: SerialFlowControl::Hardware,
        };
        let conn = StoredConnection::new_serial("我的串口".to_string(), params, Some(42));
        assert_eq!(conn.connection_type, ConnectionType::Serial);
        assert_eq!(conn.name, "我的串口");
        assert_eq!(conn.workspace_id, Some(42));

        let rt = conn.to_serial_params().unwrap();
        assert_eq!(rt.port_name, "/dev/cu.usbserial-1420");
        assert_eq!(rt.baud_rate, 9600);
        assert_eq!(rt.data_bits, 7);
        assert_eq!(rt.stop_bits, 2);
        assert_eq!(rt.parity, SerialParity::Even);
        assert_eq!(rt.flow_control, SerialFlowControl::Hardware);
    }

    #[test]
    fn connection_type_serial_methods() {
        assert_eq!(ConnectionType::Serial.label(), "串口");
        assert_eq!(ConnectionType::from_str("Serial"), ConnectionType::Serial);
        assert_eq!(format!("{}", ConnectionType::Serial), "Serial");
        assert!(ConnectionType::all().contains(&ConnectionType::Serial));
    }

    #[test]
    fn serial_enums_defaults_and_labels() {
        assert_eq!(SerialParity::default(), SerialParity::None);
        assert_eq!(SerialFlowControl::default(), SerialFlowControl::None);
        assert_eq!(SerialParity::all().len(), 3);
        assert_eq!(SerialFlowControl::all().len(), 3);
        assert_eq!(SerialParity::Odd.label(), "Odd");
        assert_eq!(SerialParity::Even.label(), "Even");
        assert_eq!(SerialFlowControl::Software.label(), "XON/XOFF");
        assert_eq!(SerialFlowControl::Hardware.label(), "RTS/CTS");
    }

    #[test]
    fn ssh_auth_method_agent_serialize_deserialize() {
        let auth = SshAuthMethod::Agent;
        let json = serde_json::to_string(&auth).expect("Agent 认证方式应可序列化");
        let parsed: SshAuthMethod =
            serde_json::from_str(&json).expect("Agent 认证方式应可反序列化");
        assert!(matches!(parsed, SshAuthMethod::Agent));
    }

    #[test]
    fn ssh_auth_method_auto_publickey_serialize_deserialize() {
        let auth = SshAuthMethod::AutoPublicKey;
        let json = serde_json::to_string(&auth).expect("自动公钥认证方式应可序列化");
        let parsed: SshAuthMethod =
            serde_json::from_str(&json).expect("自动公钥认证方式应可反序列化");
        assert!(matches!(parsed, SshAuthMethod::AutoPublicKey));
    }
}
