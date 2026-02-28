//! 密钥存储模块
//!
//! 提供统一的密钥持久化接口，支持多种存储后端：
//! - `LocalFileStorage`: 本地文件存储（开发环境推荐）
//! - `KeychainStorage`: 系统 Keychain 存储（生产环境推荐）
//!
//! # 使用方式
//!
//! ```rust,ignore
//! use onetcli_core::key_storage::{KeyStorage, LocalFileStorage, set_key_storage};
//! use std::sync::Arc;
//!
//! // 设置全局存储后端（应用启动时调用一次）
//! set_key_storage(Arc::new(LocalFileStorage));
//!
//! // 或者切换到 Keychain
//! // set_key_storage(Arc::new(KeychainStorage));
//! ```

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Keychain 服务名称
const KEYCHAIN_SERVICE: &str = "com.onetcli.master-key";

/// Keychain 用户名
const KEYCHAIN_USER: &str = "onetcli-user";

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
///
/// 统一封装不同的密钥持久化方式（本地文件、系统 Keychain 等）。
/// 通过 `set_key_storage` 设置全局使用的存储后端。
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
/// 适用于开发环境，避免 Keychain 在 IDE 中反复编译导致的权限问题。
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
// KeychainStorage 实现
// ============================================================================

/// 系统 Keychain 存储实现
///
/// 使用 macOS Keychain / Windows Credential Manager 存储密钥。
/// 适用于已签名的生产环境应用。
pub struct KeychainStorage;

impl KeyStorage for KeychainStorage {
    fn name(&self) -> &'static str {
        "Keychain"
    }

    fn save(&self, master_key: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)
            .map_err(|e| format!("创建 Keychain 条目失败: {}", e))?;

        entry
            .set_password(master_key)
            .map_err(|e| format!("保存到 Keychain 失败: {}", e))?;

        tracing::info!("[Keychain] 主密钥已保存");
        Ok(())
    }

    fn load(&self) -> Option<String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).ok()?;

        match entry.get_password() {
            Ok(key) => {
                tracing::info!("[Keychain] 成功读取密钥");
                Some(key)
            }
            Err(keyring::Error::NoEntry) => {
                tracing::debug!("[Keychain] 没有保存的密钥");
                None
            }
            Err(e) => {
                tracing::warn!("[Keychain] 读取密钥失败: {}", e);
                None
            }
        }
    }

    fn delete(&self) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)
            .map_err(|e| format!("创建 Keychain 条目失败: {}", e))?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::info!("[Keychain] 已删除密钥");
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("从 Keychain 删除失败: {}", e)),
        }
    }

    fn exists(&self) -> bool {
        let entry = match keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER) {
            Ok(e) => e,
            Err(_) => return false,
        };
        entry.get_password().is_ok()
    }
}

// ============================================================================
// 全局存储后端管理
// ============================================================================

/// 设置全局密钥存储后端
///
/// 在应用启动时调用一次，选择使用哪种存储方式。
/// 默认使用 `LocalFileStorage`（开发环境）。
///
/// # 示例
///
/// ```rust,ignore
/// use onetcli_core::key_storage::{set_key_storage, LocalFileStorage, KeychainStorage};
/// use std::sync::Arc;
///
/// // 开发环境：使用本地文件
/// set_key_storage(Arc::new(LocalFileStorage));
///
/// // 生产环境：使用 Keychain
/// set_key_storage(Arc::new(KeychainStorage));
/// ```
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
