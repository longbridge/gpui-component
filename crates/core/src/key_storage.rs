//! 密钥存储模块
//!
//! 提供统一的密钥持久化接口。
//! 当前仅保留 `LocalFileStorage`：将主密钥使用程序内置固定 key
//! 进行 AES-256-GCM 加密后写入本地文件。

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// 本地加密密钥文件名
const KEY_STORAGE_FILE: &str = "key_storage";

/// 本地文件存储使用的固定加密密钥（用于加密本地保存的主密钥）
const LOCAL_STORAGE_FIXED_KEY: &[u8; 32] = b"onehub-local-dev-key-2025-fixed!";

/// 全局密钥存储后端
static KEY_STORAGE: RwLock<Option<Arc<dyn KeyStorage>>> = RwLock::new(None);

// ============================================================================
// KeyStorage trait
// ============================================================================

/// 密钥存储后端 trait
pub trait KeyStorage: Send + Sync {
    /// 存储后端名称，用于日志标识
    fn name(&self) -> &'static str;

    /// 保存主密钥
    fn save(&self, master_key: &str) -> Result<(), String>;

    /// 加载主密钥
    fn load(&self) -> Option<String>;

    /// 删除存储的密钥
    fn delete(&self) -> Result<(), String>;

    /// 检查是否存在已保存的密钥
    fn exists(&self) -> bool;
}

// ============================================================================
// LocalFileStorage 实现
// ============================================================================

/// 本地文件存储实现
///
/// 使用固定密钥对主密钥进行 AES-256-GCM 加密后保存到本地文件。
pub struct LocalFileStorage;

impl KeyStorage for LocalFileStorage {
    fn name(&self) -> &'static str {
        "本地文件"
    }

    fn save(&self, master_key: &str) -> Result<(), String> {
        let path = get_key_storage_path().ok_or_else(|| "无法获取密钥存储路径".to_string())?;

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let cipher = Aes256Gcm::new_from_slice(LOCAL_STORAGE_FIXED_KEY)
            .map_err(|e| format!("创建加密器失败: {}", e))?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, master_key.as_bytes())
            .map_err(|e| format!("加密密钥失败: {}", e))?;

        let mut data = nonce_bytes.to_vec();
        data.extend(ciphertext);

        fs::write(&path, &data).map_err(|e| format!("写入密钥文件失败: {}", e))?;

        tracing::info!("[本地文件] 主密钥已保存");
        Ok(())
    }

    fn load(&self) -> Option<String> {
        let path = get_key_storage_path()?;

        if !path.exists() {
            return None;
        }

        let data = fs::read(&path).ok()?;
        if data.len() < 12 {
            tracing::warn!("[本地文件] 密钥文件格式无效");
            return None;
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        let cipher = Aes256Gcm::new_from_slice(LOCAL_STORAGE_FIXED_KEY).ok()?;
        let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
        let master_key = String::from_utf8(plaintext).ok()?;

        tracing::info!("[本地文件] 成功读取密钥");
        Some(master_key)
    }

    fn delete(&self) -> Result<(), String> {
        if let Some(path) = get_key_storage_path() {
            if path.exists() {
                fs::remove_file(&path).map_err(|e| format!("删除密钥文件失败: {}", e))?;
            }
        }
        Ok(())
    }

    fn exists(&self) -> bool {
        get_key_storage_path().map(|p| p.exists()).unwrap_or(false)
    }
}

// ============================================================================
// 全局存储后端管理
// ============================================================================

/// 设置全局密钥存储后端
pub fn set_key_storage(storage: Arc<dyn KeyStorage>) {
    if let Ok(mut guard) = KEY_STORAGE.write() {
        tracing::info!("[密钥存储] 切换到「{}」后端", storage.name());
        *guard = Some(storage);
    }
}

/// 获取当前密钥存储后端
///
/// 如果未设置，默认返回 `LocalFileStorage`。
pub fn get_key_storage() -> Arc<dyn KeyStorage> {
    KEY_STORAGE
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or_else(|| Arc::new(LocalFileStorage))
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取数据目录路径
fn get_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join("one-hub"))
}

/// 获取本地密钥存储文件路径
fn get_key_storage_path() -> Option<PathBuf> {
    get_data_dir().map(|p| p.join(KEY_STORAGE_FILE))
}
