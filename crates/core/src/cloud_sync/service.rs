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
    /// 团队密钥缓存：team_id -> team_key
    team_keys: HashMap<String, String>,
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
            team_keys: HashMap::new(),
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
        self.team_keys.clear();
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

    /// 修改主密钥（重新加密所有云端同步数据）
    ///
    /// 返回新的用户配置和重新加密的同步数据列表
    pub fn change_master_key(
        &mut self,
        old_key: &str,
        new_key: &str,
        cloud_data_list: &[CloudSyncData],
    ) -> Result<(CloudUserConfig, Vec<CloudSyncData>), SyncError> {
        // 验证旧密钥
        if self.master_key.as_deref() != Some(old_key) {
            return Err(SyncError::InvalidMasterKey);
        }

        let new_version = self.key_version + 1;
        let mut re_encrypted_list = Vec::with_capacity(cloud_data_list.len());

        // 重新加密每条同步数据
        for data in cloud_data_list {
            let re_encrypted = self.re_encrypt_sync_data(data, old_key, new_key, new_version)?;
            re_encrypted_list.push(re_encrypted);
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

        Ok((new_config, re_encrypted_list))
    }

    /// 获取同步状态
    pub fn get_sync_state(&self, connection_id: i64) -> Option<&SyncState> {
        self.sync_states.get(&connection_id)
    }

    /// 更新同步状态
    pub fn update_sync_state(&mut self, state: SyncState) {
        self.sync_states.insert(state.connection_id, state);
    }

    // ========================================================================
    // 团队密钥管理
    // ========================================================================

    /// 设置团队密钥
    pub fn set_team_key(&mut self, team_id: &str, team_key: String) {
        self.team_keys.insert(team_id.to_string(), team_key);
    }

    /// 获取团队密钥
    pub fn get_team_key(&self, team_id: &str) -> Option<&String> {
        self.team_keys.get(team_id)
    }

    /// 移除团队密钥
    pub fn remove_team_key(&mut self, team_id: &str) {
        self.team_keys.remove(team_id);
    }

    /// 检查团队密钥是否已解锁
    pub fn is_team_unlocked(&self, team_id: &str) -> bool {
        self.team_keys.contains_key(team_id)
    }

    /// 验证团队密钥
    pub fn verify_team_key(&self, team_key: &str, key_verification: &str) -> bool {
        crypto::verify_master_key(team_key, key_verification)
    }

    /// 生成团队密钥验证数据
    pub fn generate_team_key_verification(&self, team_key: &str) -> String {
        crypto::generate_key_verification(team_key)
    }

    /// 获取用户 ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    // ========================================================================
    // 统一 blob 加密/解密（新版 sync_data）
    // ========================================================================

    /// 选择加密密钥：个人数据用 master_key，团队数据用 team_key
    fn select_encrypt_key(&self, team_id: Option<&str>) -> Result<&str, SyncError> {
        match team_id {
            Some(tid) => self
                .team_keys
                .get(tid)
                .map(|s| s.as_str())
                .ok_or(SyncError::NotUnlocked),
            None => self.master_key.as_deref().ok_or(SyncError::NotUnlocked),
        }
    }

    /// 选择密钥版本
    fn select_key_version(&self, team_id: Option<&str>, teams: &[Team]) -> u32 {
        match team_id {
            Some(tid) => teams
                .iter()
                .find(|t| t.id == tid)
                .map(|t| t.key_version)
                .unwrap_or(1),
            None => self.key_version,
        }
    }

    /// 加密整体明文 JSON 为 blob
    pub fn encrypt_blob(
        &self,
        plaintext: &str,
        team_id: Option<&str>,
    ) -> Result<String, SyncError> {
        let key = self.select_encrypt_key(team_id)?;
        Ok(crypto::encrypt_with_key(plaintext, key))
    }

    /// 解密整体 blob 为明文 JSON
    pub fn decrypt_blob(
        &self,
        encrypted: &str,
        team_id: Option<&str>,
    ) -> Result<String, SyncError> {
        let key = self.select_encrypt_key(team_id)?;
        crypto::decrypt_with_key(encrypted, key).map_err(SyncError::CryptoError)
    }

    /// 计算明文数据的 SHA-256 校验和
    pub fn calculate_blob_checksum(plaintext: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(plaintext.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 准备上传连接到 sync_data（整体 blob 加密）
    pub fn prepare_sync_data_upload(
        &self,
        conn: &StoredConnection,
        team_id: Option<&str>,
        teams: &[Team],
    ) -> Result<CloudSyncData, SyncError> {
        let plain_data = ConnectionPlainData {
            name: conn.name.clone(),
            connection_type: conn.connection_type.to_string(),
            workspace_cloud_id: None, // 由调用者设置
            selected_databases: conn.selected_databases.clone(),
            remark: conn.remark.clone(),
            params: serde_json::from_str(&conn.params)
                .unwrap_or(Value::Object(serde_json::Map::new())),
            owner_id: conn.owner_id.clone(),
        };

        let plaintext = serde_json::to_string(&plain_data)
            .map_err(|e| SyncError::DataFormatError(e.to_string()))?;

        let checksum = Self::calculate_blob_checksum(&plaintext);
        let encrypted_data = self.encrypt_blob(&plaintext, team_id)?;
        let key_version = self.select_key_version(team_id, teams);

        Ok(CloudSyncData {
            id: uuid::Uuid::new_v4().to_string(),
            owner_id: self.user_id.clone().unwrap_or_default(),
            team_id: team_id.map(|s| s.to_string()),
            data_type: data_type::CONNECTION.to_string(),
            encrypted_data,
            key_version,
            checksum,
            version: 1,
            updated_at: current_timestamp(),
            deleted_at: None,
        })
    }

    /// 准备上传工作空间到 sync_data（整体 blob 加密）
    pub fn prepare_workspace_sync_data_upload(
        &self,
        ws: &crate::storage::Workspace,
        team_id: Option<&str>,
        teams: &[Team],
    ) -> Result<CloudSyncData, SyncError> {
        let plain_data = WorkspacePlainData {
            name: ws.name.clone(),
            color: ws.color.clone(),
            icon: ws.icon.clone(),
        };

        let plaintext = serde_json::to_string(&plain_data)
            .map_err(|e| SyncError::DataFormatError(e.to_string()))?;

        let checksum = Self::calculate_blob_checksum(&plaintext);
        let encrypted_data = self.encrypt_blob(&plaintext, team_id)?;
        let key_version = self.select_key_version(team_id, teams);

        Ok(CloudSyncData {
            id: uuid::Uuid::new_v4().to_string(),
            owner_id: self.user_id.clone().unwrap_or_default(),
            team_id: team_id.map(|s| s.to_string()),
            data_type: data_type::WORKSPACE.to_string(),
            encrypted_data,
            key_version,
            checksum,
            version: 1,
            updated_at: current_timestamp(),
            deleted_at: None,
        })
    }

    /// 解密 sync_data 中的连接数据
    pub fn decrypt_sync_data_connection(
        &self,
        cloud_data: &CloudSyncData,
    ) -> Result<StoredConnection, SyncError> {
        let plaintext =
            self.decrypt_blob(&cloud_data.encrypted_data, cloud_data.team_id.as_deref())?;
        let plain_data: ConnectionPlainData = serde_json::from_str(&plaintext)
            .map_err(|e| SyncError::DataFormatError(e.to_string()))?;

        let connection_type = ConnectionType::from_str(&plain_data.connection_type);
        let params = serde_json::to_string(&plain_data.params).unwrap_or_else(|_| "{}".to_string());

        Ok(StoredConnection {
            id: None,
            name: plain_data.name,
            connection_type,
            workspace_id: None, // 由调用者根据 workspace_cloud_id 解析
            params,
            selected_databases: plain_data.selected_databases,
            remark: plain_data.remark,
            sync_enabled: true,
            cloud_id: Some(cloud_data.id.clone()),
            last_synced_at: Some(cloud_data.updated_at),
            created_at: None,
            updated_at: None,
            team_id: cloud_data.team_id.clone(),
            owner_id: plain_data.owner_id,
        })
    }

    /// 解密 sync_data 中的工作空间数据
    pub fn decrypt_sync_data_workspace(
        &self,
        cloud_data: &CloudSyncData,
    ) -> Result<crate::storage::Workspace, SyncError> {
        let plaintext =
            self.decrypt_blob(&cloud_data.encrypted_data, cloud_data.team_id.as_deref())?;
        let plain_data: WorkspacePlainData = serde_json::from_str(&plaintext)
            .map_err(|e| SyncError::DataFormatError(e.to_string()))?;

        Ok(crate::storage::Workspace {
            id: None,
            name: plain_data.name,
            color: plain_data.color,
            icon: plain_data.icon,
            created_at: None,
            updated_at: Some(cloud_data.updated_at / 1000),
            cloud_id: Some(cloud_data.id.clone()),
        })
    }

    /// 重新加密同步数据（密钥轮换时使用）
    pub fn re_encrypt_sync_data(
        &self,
        cloud_data: &CloudSyncData,
        old_key: &str,
        new_key: &str,
        new_key_version: u32,
    ) -> Result<CloudSyncData, SyncError> {
        let plaintext = crypto::decrypt_with_key(&cloud_data.encrypted_data, old_key)
            .map_err(SyncError::CryptoError)?;
        let encrypted_data = crypto::encrypt_with_key(&plaintext, new_key);

        Ok(CloudSyncData {
            id: cloud_data.id.clone(),
            owner_id: cloud_data.owner_id.clone(),
            team_id: cloud_data.team_id.clone(),
            data_type: cloud_data.data_type.clone(),
            encrypted_data,
            key_version: new_key_version,
            checksum: cloud_data.checksum.clone(),
            version: cloud_data.version,
            updated_at: current_timestamp(),
            deleted_at: cloud_data.deleted_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_encrypt_decrypt() {
        let mut service = CloudSyncService::new();
        service.set_master_key_directly("test_blob_key".to_string());

        let plaintext = r#"{"name":"test","params":{"host":"localhost","password":"secret"}}"#;
        let encrypted = service.encrypt_blob(plaintext, None).unwrap();
        assert!(encrypted.starts_with("ENC:"));

        let decrypted = service.decrypt_blob(&encrypted, None).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_team_key_management() {
        let mut service = CloudSyncService::new();
        assert!(!service.is_team_unlocked("team-1"));

        service.set_team_key("team-1", "team_key_123".to_string());
        assert!(service.is_team_unlocked("team-1"));
        assert_eq!(service.get_team_key("team-1").unwrap(), "team_key_123");

        service.remove_team_key("team-1");
        assert!(!service.is_team_unlocked("team-1"));
    }

    #[test]
    fn test_blob_checksum() {
        let data = r#"{"name":"test","host":"localhost"}"#;
        let checksum1 = CloudSyncService::calculate_blob_checksum(data);
        let checksum2 = CloudSyncService::calculate_blob_checksum(data);
        assert_eq!(checksum1, checksum2);

        let different_data = r#"{"name":"test","host":"127.0.0.1"}"#;
        let checksum3 = CloudSyncService::calculate_blob_checksum(different_data);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_team_blob_encrypt_decrypt() {
        let mut service = CloudSyncService::new();
        service.set_team_key("team-1", "team_secret_key".to_string());

        let plaintext = r#"{"name":"team connection"}"#;
        let encrypted = service.encrypt_blob(plaintext, Some("team-1")).unwrap();
        let decrypted = service.decrypt_blob(&encrypted, Some("team-1")).unwrap();
        assert_eq!(plaintext, decrypted);
    }
}
