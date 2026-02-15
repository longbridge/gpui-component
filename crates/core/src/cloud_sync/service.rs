//! 云同步服务

use crate::cloud_sync::models::*;
use crate::cloud_sync::queue::{OperationQueue, SyncOperation};
use crate::crypto::{self, CryptoError};
use crate::storage::{ConnectionType, StoredConnection};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 云同步错误类型
#[derive(Debug)]
pub enum SyncError {
    /// 未解锁（未输入主密钥）
    NotUnlocked,
    /// 主密钥错误
    InvalidMasterKey,
    /// 密钥版本不匹配
    KeyVersionMismatch,
    /// 网络错误
    NetworkError(String),
    /// 加解密错误
    CryptoError(CryptoError),
    /// 数据格式错误
    DataFormatError(String),
    /// 存储错误
    StorageError(String),
    /// 未登录
    NotLoggedIn,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NotUnlocked => write!(f, "请先输入主密钥解锁"),
            SyncError::InvalidMasterKey => write!(f, "主密钥错误"),
            SyncError::KeyVersionMismatch => write!(f, "密钥版本不匹配，请重新同步"),
            SyncError::NetworkError(e) => write!(f, "网络错误: {}", e),
            SyncError::CryptoError(e) => write!(f, "加解密错误: {}", e),
            SyncError::DataFormatError(e) => write!(f, "数据格式错误: {}", e),
            SyncError::StorageError(e) => write!(f, "存储错误: {}", e),
            SyncError::NotLoggedIn => write!(f, "请先登录云端账户"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<CryptoError> for SyncError {
    fn from(e: CryptoError) -> Self {
        SyncError::CryptoError(e)
    }
}

/// 获取当前时间戳（毫秒）
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 计算连接数据的校验和
fn calculate_checksum(conn: &StoredConnection) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(conn.name.as_bytes());
    hasher.update(conn.connection_type.to_string().as_bytes());
    hasher.update(conn.params.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 云同步服务
pub struct CloudSyncService {
    /// 主密钥（解锁后存储）
    master_key: Option<String>,
    /// 当前密钥版本
    key_version: u32,
    /// 是否已登录
    logged_in: bool,
    /// 用户 ID
    user_id: Option<String>,
    /// 同步状态缓存
    sync_states: HashMap<i64, SyncState>,
    /// 同步操作队列（按类型分组）
    operation_queues: HashMap<String, OperationQueue>,
}

impl CloudSyncService {
    /// 创建新的云同步服务
    pub fn new() -> Self {
        Self {
            master_key: None,
            key_version: 0,
            logged_in: false,
            user_id: None,
            sync_states: HashMap::new(),
            operation_queues: HashMap::new(),
        }
    }

    /// 检查是否已解锁
    pub fn is_unlocked(&self) -> bool {
        self.master_key.is_some()
    }

    /// 检查是否已登录
    pub fn is_logged_in(&self) -> bool {
        self.logged_in
    }

    /// 获取当前密钥版本
    pub fn key_version(&self) -> u32 {
        self.key_version
    }

    /// 设置登录状态（由外部认证流程调用）
    pub fn set_logged_in(&mut self, user_id: String) {
        self.logged_in = true;
        self.user_id = Some(user_id);
    }

    /// 登出
    pub fn logout(&mut self) {
        self.logged_in = false;
        self.user_id = None;
        self.master_key = None;
        self.key_version = 0;
        self.sync_states.clear();
        self.operation_queues.clear();
    }

    pub fn take_operation_queue(&mut self, key: &str) -> OperationQueue {
        self.operation_queues.remove(key).unwrap_or_default()
    }

    pub fn store_operation_queue(&mut self, key: &str, queue: OperationQueue) {
        if queue.is_empty() {
            self.operation_queues.remove(key);
        } else {
            self.operation_queues.insert(key.to_string(), queue);
        }
    }

    pub fn enqueue_operations(
        &mut self,
        key: &str,
        operations: impl IntoIterator<Item = SyncOperation>,
    ) {
        let queue = self
            .operation_queues
            .entry(key.to_string())
            .or_insert_with(OperationQueue::new);
        queue.enqueue_all(operations);
    }

    /// 直接设置主密钥（不验证）
    ///
    /// 用于从本地 crypto 模块同步密钥状态，跳过云端验证。
    /// 调用者需确保密钥已通过本地验证。
    pub fn set_master_key_directly(&mut self, master_key: String) {
        self.master_key = Some(master_key);
        // key_version 保持默认或之前的值，实际同步时会从云端更新
    }

    /// 解锁同步服务（验证主密钥）
    ///
    /// 需要先从云端获取 key_verification 数据进行验证
    pub fn unlock(
        &mut self,
        master_key: &str,
        cloud_config: &CloudUserConfig,
    ) -> Result<(), SyncError> {
        // 验证主密钥
        if !crypto::verify_master_key(master_key, &cloud_config.key_verification) {
            return Err(SyncError::InvalidMasterKey);
        }

        self.master_key = Some(master_key.to_string());
        self.key_version = cloud_config.key_version;
        Ok(())
    }

    /// 首次设置主密钥
    ///
    /// 返回需要上传到云端的配置数据
    pub fn setup_master_key(&mut self, master_key: &str) -> Result<CloudUserConfig, SyncError> {
        if !self.logged_in {
            return Err(SyncError::NotLoggedIn);
        }

        let verification = crypto::generate_key_verification(master_key);
        let user_id = self.user_id.clone().unwrap_or_default();

        let config = CloudUserConfig {
            user_id,
            key_verification: verification,
            key_version: 1,
            updated_at: current_timestamp(),
        };

        self.master_key = Some(master_key.to_string());
        self.key_version = 1;

        Ok(config)
    }

    /// 准备上传连接到云端
    ///
    /// 返回加密后的云端连接数据
    pub fn prepare_upload(&self, conn: &StoredConnection) -> Result<CloudConnection, SyncError> {
        let master_key = self.master_key.as_ref().ok_or(SyncError::NotUnlocked)?;

        // 加密连接参数中的敏感字段
        let encrypted_params = encrypt_json_passwords_with_key(&conn.params, master_key);

        Ok(CloudConnection {
            id: uuid::Uuid::new_v4().to_string(),
            local_id: conn.id,
            name: conn.name.clone(),
            connection_type: conn.connection_type.to_string(),
            workspace_id: conn.workspace_id.map(|id| id.to_string()),
            encrypted_params,
            key_version: self.key_version,
            updated_at: current_timestamp(),
            checksum: calculate_checksum(conn),
            deleted_at: None,
        })
    }

    /// 解密云端连接数据
    ///
    /// 返回解密后的本地连接数据
    pub fn decrypt_connection(
        &self,
        cloud_conn: &CloudConnection,
    ) -> Result<StoredConnection, SyncError> {
        let master_key = self.master_key.as_ref().ok_or(SyncError::NotUnlocked)?;

        // 检查密钥版本
        if cloud_conn.key_version != self.key_version {
            return Err(SyncError::KeyVersionMismatch);
        }

        // 解密连接参数
        let decrypted_params =
            decrypt_json_passwords_with_key(&cloud_conn.encrypted_params, master_key)?;

        // 解析连接类型
        let connection_type = match cloud_conn.connection_type.as_str() {
            "Database" => ConnectionType::Database,
            "SshSftp" => ConnectionType::SshSftp,
            "Redis" => ConnectionType::Redis,
            "MongoDB" => ConnectionType::MongoDB,
            _ => ConnectionType::Database,
        };

        Ok(StoredConnection {
            id: cloud_conn.local_id,
            name: cloud_conn.name.clone(),
            connection_type,
            workspace_id: cloud_conn.workspace_id.as_ref().and_then(|s| s.parse().ok()),
            params: decrypted_params,
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: Some(cloud_conn.id.clone()),
            last_synced_at: Some(cloud_conn.updated_at),
            created_at: None,
            updated_at: None,
        })
    }

    /// 修改主密钥（重新加密所有云端数据）
    ///
    /// 返回新的用户配置和重新加密的连接列表
    pub fn change_master_key(
        &mut self,
        old_key: &str,
        new_key: &str,
        cloud_connections: &[CloudConnection],
    ) -> Result<(CloudUserConfig, Vec<CloudConnection>), SyncError> {
        // 验证旧密钥
        if self.master_key.as_deref() != Some(old_key) {
            return Err(SyncError::InvalidMasterKey);
        }

        let new_version = self.key_version + 1;
        let mut re_encrypted_connections = Vec::with_capacity(cloud_connections.len());

        // 重新加密每个连接
        for cloud_conn in cloud_connections {
            let re_encrypted_params = re_encrypt_json_passwords(
                &cloud_conn.encrypted_params,
                old_key,
                new_key,
            )?;

            re_encrypted_connections.push(CloudConnection {
                id: cloud_conn.id.clone(),
                local_id: cloud_conn.local_id,
                name: cloud_conn.name.clone(),
                connection_type: cloud_conn.connection_type.clone(),
                workspace_id: cloud_conn.workspace_id.clone(),
                encrypted_params: re_encrypted_params,
                key_version: new_version,
                updated_at: current_timestamp(),
                checksum: cloud_conn.checksum.clone(),
                deleted_at: cloud_conn.deleted_at, // 保留原来的删除状态
            });
        }

        // 生成新的用户配置
        let new_verification = crypto::generate_key_verification(new_key);
        let user_id = self.user_id.clone().unwrap_or_default();

        let new_config = CloudUserConfig {
            user_id,
            key_verification: new_verification,
            key_version: new_version,
            updated_at: current_timestamp(),
        };

        // 更新本地状态
        self.master_key = Some(new_key.to_string());
        self.key_version = new_version;

        Ok((new_config, re_encrypted_connections))
    }

    /// 获取同步状态
    pub fn get_sync_state(&self, connection_id: i64) -> Option<&SyncState> {
        self.sync_states.get(&connection_id)
    }

    /// 更新同步状态
    pub fn update_sync_state(&mut self, state: SyncState) {
        self.sync_states.insert(state.connection_id, state);
    }
}

// ============================================================================
// JSON 密码字段加解密辅助函数
// ============================================================================

/// 使用指定密钥加密 JSON 中的敏感字段
fn encrypt_json_passwords_with_key(json_str: &str, master_key: &str) -> String {
    match serde_json::from_str::<Value>(json_str) {
        Ok(mut value) => {
            encrypt_value_with_key(&mut value, master_key);
            serde_json::to_string(&value).unwrap_or_else(|_| json_str.to_string())
        }
        Err(_) => json_str.to_string(),
    }
}

/// 使用指定密钥解密 JSON 中的敏感字段
fn decrypt_json_passwords_with_key(json_str: &str, master_key: &str) -> Result<String, SyncError> {
    match serde_json::from_str::<Value>(json_str) {
        Ok(mut value) => {
            decrypt_value_with_key(&mut value, master_key)?;
            serde_json::to_string(&value)
                .map_err(|e| SyncError::DataFormatError(e.to_string()))
        }
        Err(e) => Err(SyncError::DataFormatError(e.to_string())),
    }
}

/// 重新加密 JSON 中的敏感字段
fn re_encrypt_json_passwords(
    json_str: &str,
    old_key: &str,
    new_key: &str,
) -> Result<String, SyncError> {
    match serde_json::from_str::<Value>(json_str) {
        Ok(mut value) => {
            re_encrypt_value(&mut value, old_key, new_key)?;
            serde_json::to_string(&value)
                .map_err(|e| SyncError::DataFormatError(e.to_string()))
        }
        Err(e) => Err(SyncError::DataFormatError(e.to_string())),
    }
}

/// 判断字段名是否为敏感字段
fn is_sensitive_field(key: &str) -> bool {
    key == "password" || key == "passphrase"
}

/// 递归加密 JSON Value 中的敏感字段
fn encrypt_value_with_key(value: &mut Value, master_key: &str) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if is_sensitive_field(key) {
                    if let Value::String(s) = val {
                        *s = crypto::encrypt_with_key(s, master_key);
                    }
                } else {
                    encrypt_value_with_key(val, master_key);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                encrypt_value_with_key(item, master_key);
            }
        }
        _ => {}
    }
}

/// 递归解密 JSON Value 中的敏感字段
fn decrypt_value_with_key(value: &mut Value, master_key: &str) -> Result<(), SyncError> {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if is_sensitive_field(key) {
                    if let Value::String(s) = val {
                        *s = crypto::decrypt_with_key(s, master_key)?;
                    }
                } else {
                    decrypt_value_with_key(val, master_key)?;
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                decrypt_value_with_key(item, master_key)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// 递归重新加密 JSON Value 中的敏感字段
fn re_encrypt_value(value: &mut Value, old_key: &str, new_key: &str) -> Result<(), SyncError> {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if is_sensitive_field(key) {
                    if let Value::String(s) = val {
                        *s = crypto::re_encrypt_data(s, old_key, new_key)?;
                    }
                } else {
                    re_encrypt_value(val, old_key, new_key)?;
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                re_encrypt_value(item, old_key, new_key)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_json() {
        let json = r#"{"host":"localhost","password":"secret123","nested":{"passphrase":"key456"}}"#;
        let master_key = "test_master_key";

        let encrypted = encrypt_json_passwords_with_key(json, master_key);
        assert!(encrypted.contains("ENC:"));

        let decrypted = decrypt_json_passwords_with_key(&encrypted, master_key).unwrap();
        let original: Value = serde_json::from_str(json).unwrap();
        let result: Value = serde_json::from_str(&decrypted).unwrap();

        assert_eq!(original["host"], result["host"]);
        assert_eq!(original["password"], result["password"]);
        assert_eq!(original["nested"]["passphrase"], result["nested"]["passphrase"]);
    }

    #[test]
    fn test_re_encrypt_json() {
        let json = r#"{"password":"secret"}"#;
        let old_key = "old_key";
        let new_key = "new_key";

        // 先用旧密钥加密
        let encrypted = encrypt_json_passwords_with_key(json, old_key);

        // 重新加密
        let re_encrypted = re_encrypt_json_passwords(&encrypted, old_key, new_key).unwrap();

        // 用新密钥解密
        let decrypted = decrypt_json_passwords_with_key(&re_encrypted, new_key).unwrap();
        let result: Value = serde_json::from_str(&decrypted).unwrap();

        assert_eq!(result["password"], "secret");
    }
}
