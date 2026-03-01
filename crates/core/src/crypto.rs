//! 密码加密模块
//!
//! 使用 AES-256-GCM 对密码进行加密存储。
//! 用户需要设置一个主密钥，该密钥用于派生加密密钥。
//! 如果主密钥丢失，所有加密的密码将无法恢复。
//!
//! 支持两种使用方式：
//! 1. 全局密钥模式：通过 set_master_key 设置全局密钥，使用 encrypt_password/decrypt_password
//! 2. 指定密钥模式：使用 encrypt_with_key/decrypt_with_key 直接传入密钥
//!
//! 密钥持久化：
//! - 支持本地文件存储（开发环境）和系统 Keychain（生产环境）
//! - 通过 key_storage 模块的 KeyStorage trait 统一接口，可灵活切换存储后端
//! - 应用启动时可自动从存储后端恢复密钥

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::key_storage;

/// 加密错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum CryptoError {
    /// 旧密码错误
    InvalidOldPassword,
    /// 新密码为空
    EmptyNewPassword,
    /// 两次输入的新密码不一致
    PasswordMismatch,
    /// 未设置过主密码
    NoPasswordSet,
    /// 保存验证数据失败
    SaveVerificationFailed,
    /// 解密失败
    DecryptionFailed,
    /// 编码失败
    EncodingFailed,
    /// 数据格式错误
    InvalidDataFormat,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::InvalidOldPassword => write!(f, "旧密码错误"),
            CryptoError::EmptyNewPassword => write!(f, "新密码不能为空"),
            CryptoError::PasswordMismatch => write!(f, "两次输入的新密码不一致"),
            CryptoError::NoPasswordSet => write!(f, "未设置过主密码"),
            CryptoError::SaveVerificationFailed => write!(f, "保存验证数据失败"),
            CryptoError::DecryptionFailed => write!(f, "解密失败"),
            CryptoError::EncodingFailed => write!(f, "编码失败"),
            CryptoError::InvalidDataFormat => write!(f, "数据格式错误"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// 加密前缀标识，用于识别已加密的密码
const ENCRYPTED_PREFIX: &str = "ENC:";

/// 验证数据的魔术字符串，用于验证密钥是否正确
const VERIFICATION_MAGIC: &str = "ONEHUB_KEY_VERIFY_V1";

/// 密钥验证文件名
const KEY_VERIFICATION_FILE: &str = "key_verification";

/// 全局加密密钥存储（派生后的密钥）
static ENCRYPTION_KEY: RwLock<Option<[u8; 32]>> = RwLock::new(None);

/// 全局原始主密钥存储（用于云同步等需要原始密钥的场景）
static RAW_MASTER_KEY: RwLock<Option<String>> = RwLock::new(None);

/// 获取数据目录路径
fn get_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join("one-hub"))
}

/// 获取密钥验证文件路径
fn get_verification_file_path() -> Option<PathBuf> {
    get_data_dir().map(|p| p.join(KEY_VERIFICATION_FILE))
}

/// 检查是否已设置过主密钥（验证文件是否存在）
pub fn has_repo_password_set() -> bool {
    get_verification_file_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// 保存密钥验证数据到文件
fn save_verification_data(data: &str) -> bool {
    if let Some(path) = get_verification_file_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, data).is_ok()
    } else {
        false
    }
}

/// 从文件读取密钥验证数据
fn load_verification_data() -> Option<String> {
    get_verification_file_path().and_then(|p| fs::read_to_string(p).ok())
}

/// 从用户主密钥派生 AES-256 密钥
fn derive_key(master_key: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(master_key.as_bytes());
    // 添加盐值增强安全性
    hasher.update(b"onehub_password_encryption_salt_v1");
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// 生成密钥验证数据
///
/// 返回一个加密的魔术字符串，用于验证用户输入的密钥是否正确。
/// 存储格式：base64(nonce + ciphertext)
pub fn generate_key_verification(master_key: &str) -> String {
    let key = derive_key(master_key);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    match cipher.encrypt(nonce, VERIFICATION_MAGIC.as_bytes()) {
        Ok(ciphertext) => {
            let mut combined = Vec::with_capacity(12 + ciphertext.len());
            combined.extend_from_slice(&nonce_bytes);
            combined.extend_from_slice(&ciphertext);
            BASE64.encode(&combined)
        }
        Err(_) => String::new(),
    }
}

/// 验证密钥是否正确
///
/// 通过尝试解密验证数据来验证密钥是否正确。
pub fn verify_master_key(master_key: &str, verification_data: &str) -> bool {
    if verification_data.is_empty() {
        return false;
    }

    let key = derive_key(master_key);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    let combined = match BASE64.decode(verification_data) {
        Ok(data) => data,
        Err(_) => return false,
    };

    if combined.len() < 12 {
        return false;
    }

    let nonce = Nonce::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];

    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => String::from_utf8(plaintext)
            .map(|s| s == VERIFICATION_MAGIC)
            .unwrap_or(false),
        Err(_) => false,
    }
}

/// 设置主密钥
///
/// 用户提供的主密钥将被哈希后存储在内存中，用于后续的加密/解密操作。
/// 如果是首次设置，会生成并保存验证数据。
/// 同时会将密钥保存到存储后端，以便应用重启后自动恢复。
pub fn set_master_key(master_key: &str) {
    let key = derive_key(master_key);
    if let Ok(mut guard) = ENCRYPTION_KEY.write() {
        *guard = Some(key);
    }

    // 同时保存原始主密钥（用于云同步等场景）
    if let Ok(mut guard) = RAW_MASTER_KEY.write() {
        *guard = Some(master_key.to_string());
    }

    // 如果尚未设置过密码，生成并保存验证数据
    if !has_repo_password_set() {
        let verification = generate_key_verification(master_key);
        save_verification_data(&verification);
    }

    // 保存到存储后端
    let storage = key_storage::get_key_storage();
    if let Err(e) = storage.save(master_key) {
        tracing::warn!("[{}] 保存密钥失败: {}", storage.name(), e);
    }
}

/// 验证并设置主密钥
///
/// 如果已设置过密码，需要先验证密钥是否正确。
/// 返回 Ok(()) 表示设置成功，Err 表示密码错误。
pub fn verify_and_set_master_key(master_key: &str) -> Result<(), &'static str> {
    if has_repo_password_set() {
        // 已设置过密码，需要验证
        if let Some(verification_data) = load_verification_data() {
            if !verify_master_key(master_key, &verification_data) {
                return Err("密码错误");
            }
        }
    }

    // 验证通过或首次设置，设置密钥
    set_master_key(master_key);
    Ok(())
}

/// 清除主密钥
pub fn clear_master_key() {
    if let Ok(mut guard) = ENCRYPTION_KEY.write() {
        *guard = None;
    }
    if let Ok(mut guard) = RAW_MASTER_KEY.write() {
        *guard = None;
    }
    // 同时清除存储后端中的密钥
    let storage = key_storage::get_key_storage();
    let _ = storage.delete();
}

/// 检查是否已设置主密钥
pub fn has_master_key() -> bool {
    ENCRYPTION_KEY
        .read()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// 获取原始主密钥
///
/// 返回设置的原始主密钥字符串，用于云同步等需要原始密钥的场景。
/// 如果未设置主密钥，返回 None。
pub fn get_raw_master_key() -> Option<String> {
    RAW_MASTER_KEY.read().ok().and_then(|guard| guard.clone())
}

/// 加密密码
///
/// 如果未设置主密钥，返回原始密码。
/// 加密后的密码格式：`ENC:base64(nonce + ciphertext)`
pub fn encrypt_password(password: &str) -> String {
    if password.is_empty() {
        return password.to_string();
    }

    // 如果已经是加密的，直接返回
    if password.starts_with(ENCRYPTED_PREFIX) {
        return password.to_string();
    }

    let key = match ENCRYPTION_KEY.read() {
        Ok(guard) => match *guard {
            Some(k) => k,
            None => return password.to_string(),
        },
        Err(_) => return password.to_string(),
    };

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    // 生成随机 nonce (12 字节)
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    match cipher.encrypt(nonce, password.as_bytes()) {
        Ok(ciphertext) => {
            // 将 nonce 和密文拼接后进行 base64 编码
            let mut combined = Vec::with_capacity(12 + ciphertext.len());
            combined.extend_from_slice(&nonce_bytes);
            combined.extend_from_slice(&ciphertext);
            format!("{}{}", ENCRYPTED_PREFIX, BASE64.encode(&combined))
        }
        Err(_) => password.to_string(),
    }
}

/// 解密密码
///
/// 如果密码未加密（不以 `ENC:` 开头），返回原始密码。
/// 如果未设置主密钥或解密失败，返回空字符串。
pub fn decrypt_password(encrypted: &str) -> String {
    if encrypted.is_empty() {
        return encrypted.to_string();
    }

    // 如果不是加密的，直接返回
    if !encrypted.starts_with(ENCRYPTED_PREFIX) {
        return encrypted.to_string();
    }

    let key = match ENCRYPTION_KEY.read() {
        Ok(guard) => match *guard {
            Some(k) => k,
            None => return String::new(),
        },
        Err(_) => return String::new(),
    };

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    // 解码 base64
    let encoded = &encrypted[ENCRYPTED_PREFIX.len()..];
    let combined = match BASE64.decode(encoded) {
        Ok(data) => data,
        Err(_) => return String::new(),
    };

    if combined.len() < 12 {
        return String::new();
    }

    // 分离 nonce 和密文
    let nonce = Nonce::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];

    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => String::from_utf8(plaintext).unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// 检查密码是否已加密
pub fn is_encrypted(password: &str) -> bool {
    password.starts_with(ENCRYPTED_PREFIX)
}

// ============================================================================
// 指定密钥模式的加解密函数（用于云同步和密钥迁移）
// ============================================================================

/// 使用指定密钥加密密码
///
/// 不依赖全局密钥状态，直接使用传入的主密钥进行加密。
/// 适用于云同步场景和密钥迁移场景。
pub fn encrypt_with_key(plaintext: &str, master_key: &str) -> String {
    if plaintext.is_empty() {
        return plaintext.to_string();
    }

    // 如果已经是加密的，直接返回
    if plaintext.starts_with(ENCRYPTED_PREFIX) {
        return plaintext.to_string();
    }

    let key = derive_key(master_key);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    match cipher.encrypt(nonce, plaintext.as_bytes()) {
        Ok(ciphertext) => {
            let mut combined = Vec::with_capacity(12 + ciphertext.len());
            combined.extend_from_slice(&nonce_bytes);
            combined.extend_from_slice(&ciphertext);
            format!("{}{}", ENCRYPTED_PREFIX, BASE64.encode(&combined))
        }
        Err(_) => plaintext.to_string(),
    }
}

/// 使用指定密钥解密密码
///
/// 不依赖全局密钥状态，直接使用传入的主密钥进行解密。
/// 适用于云同步场景和密钥迁移场景。
pub fn decrypt_with_key(encrypted: &str, master_key: &str) -> Result<String, CryptoError> {
    if encrypted.is_empty() {
        return Ok(encrypted.to_string());
    }

    // 如果不是加密的，直接返回
    if !encrypted.starts_with(ENCRYPTED_PREFIX) {
        return Ok(encrypted.to_string());
    }

    let key = derive_key(master_key);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    let encoded = &encrypted[ENCRYPTED_PREFIX.len()..];
    let combined = BASE64
        .decode(encoded)
        .map_err(|_| CryptoError::EncodingFailed)?;

    if combined.len() < 12 {
        return Err(CryptoError::InvalidDataFormat);
    }

    let nonce = Nonce::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];

    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => String::from_utf8(plaintext).map_err(|_| CryptoError::EncodingFailed),
        Err(_) => Err(CryptoError::DecryptionFailed),
    }
}

/// 重新加密数据（从旧密钥迁移到新密钥）
///
/// 用于修改主密钥时重新加密已加密的数据。
pub fn re_encrypt_data(
    encrypted_data: &str,
    old_key: &str,
    new_key: &str,
) -> Result<String, CryptoError> {
    // 如果数据未加密，直接用新密钥加密
    if !encrypted_data.starts_with(ENCRYPTED_PREFIX) {
        return Ok(encrypt_with_key(encrypted_data, new_key));
    }

    // 用旧密钥解密
    let plaintext = decrypt_with_key(encrypted_data, old_key)?;

    // 用新密钥重新加密
    Ok(encrypt_with_key(&plaintext, new_key))
}

// ============================================================================
// 修改主密钥相关函数
// ============================================================================

/// 修改主密钥
///
/// 验证旧密钥后更新验证数据和内存中的密钥。
/// 注意：此函数不负责重新加密已有数据，调用方需要自行处理数据迁移。
///
/// # 参数
/// - `old_key`: 当前的主密钥
/// - `new_key`: 新的主密钥
/// - `confirm_new_key`: 确认新密钥（需要与 new_key 一致）
///
/// # 返回
/// - `Ok(())`: 修改成功
/// - `Err(CryptoError)`: 修改失败的原因
pub fn change_master_key(
    old_key: &str,
    new_key: &str,
    confirm_new_key: &str,
) -> Result<(), CryptoError> {
    // 检查是否已设置过密码
    if !has_repo_password_set() {
        return Err(CryptoError::NoPasswordSet);
    }

    // 验证旧密钥
    if let Some(verification_data) = load_verification_data() {
        if !verify_master_key(old_key, &verification_data) {
            return Err(CryptoError::InvalidOldPassword);
        }
    } else {
        return Err(CryptoError::NoPasswordSet);
    }

    // 验证新密钥不为空
    if new_key.is_empty() {
        return Err(CryptoError::EmptyNewPassword);
    }

    // 验证两次新密钥一致
    if new_key != confirm_new_key {
        return Err(CryptoError::PasswordMismatch);
    }

    // 生成新的验证数据
    let new_verification = generate_key_verification(new_key);
    if !save_verification_data(&new_verification) {
        return Err(CryptoError::SaveVerificationFailed);
    }

    // 更新内存中的密钥
    let key = derive_key(new_key);
    if let Ok(mut guard) = ENCRYPTION_KEY.write() {
        *guard = Some(key);
    }

    // 同时更新原始主密钥
    if let Ok(mut guard) = RAW_MASTER_KEY.write() {
        *guard = Some(new_key.to_string());
    }

    // 更新存储后端中的密钥
    let storage = key_storage::get_key_storage();
    if let Err(e) = storage.save(new_key) {
        tracing::warn!("[{}] 更新密钥失败: {}", storage.name(), e);
    }

    Ok(())
}

/// 重置主密钥
///
/// 删除验证文件，允许用户重新设置密钥。
/// 警告：这将导致所有已加密的密码无法解密！
pub fn reset_repo_password() -> bool {
    // 清除内存中的密钥
    clear_master_key();

    // 删除验证文件
    if let Some(path) = get_verification_file_path() {
        if path.exists() {
            return fs::remove_file(path).is_ok();
        }
    }
    true
}

/// 获取当前密钥的验证数据
///
/// 用于云端存储，验证用户输入的密钥是否正确。
pub fn get_current_verification_data() -> Option<String> {
    load_verification_data()
}

/// 从存储后端恢复主密钥
///
/// 尝试从当前配置的存储后端读取密钥。
/// 如果密钥存在且验证通过，则自动设置到内存。
/// 返回是否成功恢复。
pub fn try_restore_master_key() -> bool {
    let storage = key_storage::get_key_storage();
    tracing::info!("[密钥恢复] 尝试从「{}」恢复主密钥...", storage.name());

    // 如果没有设置过密码验证文件，不需要恢复
    if !has_repo_password_set() {
        tracing::info!("[密钥恢复] 未设置过主密钥，跳过恢复");
        return false;
    }

    // 从存储后端读取
    let master_key = match storage.load() {
        Some(key) => key,
        None => {
            tracing::warn!(
                "[密钥恢复] 从「{}」读取失败，需要用户手动输入",
                storage.name()
            );
            return false;
        }
    };

    // 验证密钥是否正确
    if let Some(verification_data) = load_verification_data() {
        if !verify_master_key(&master_key, &verification_data) {
            tracing::warn!("[密钥恢复] 密钥验证失败，可能密码已修改");
            let _ = storage.delete();
            return false;
        }
    }

    // 直接设置密钥到内存（不再保存，避免循环）
    let key = derive_key(&master_key);
    if let Ok(mut guard) = ENCRYPTION_KEY.write() {
        *guard = Some(key);
    }
    if let Ok(mut guard) = RAW_MASTER_KEY.write() {
        *guard = Some(master_key);
    }

    tracing::info!("[密钥恢复] 主密钥恢复成功");
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn test_mutex() -> &'static Mutex<()> {
        static MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        MUTEX.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_encrypt_decrypt() {
        let _guard = test_mutex().lock().unwrap();
        set_master_key("test_key_123");

        let original = "my_secret_password";
        let encrypted = encrypt_password(original);

        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert_ne!(encrypted, original);

        let decrypted = decrypt_password(&encrypted);
        assert_eq!(decrypted, original);

        clear_master_key();
    }

    #[test]
    fn test_key_verification() {
        let _guard = test_mutex().lock().unwrap();
        let master_key = "test_key_123";
        let verification = generate_key_verification(master_key);

        assert!(verify_master_key(master_key, &verification));
        assert!(!verify_master_key("wrong_key", &verification));
    }

    #[test]
    fn test_empty_password() {
        let _guard = test_mutex().lock().unwrap();
        set_master_key("test_key");

        let encrypted = encrypt_password("");
        assert_eq!(encrypted, "");

        let decrypted = decrypt_password("");
        assert_eq!(decrypted, "");

        clear_master_key();
    }

    #[test]
    fn test_no_master_key() {
        let _guard = test_mutex().lock().unwrap();
        clear_master_key();

        let original = "password123";
        let result = encrypt_password(original);
        assert_eq!(result, original); // 未设置密钥时不加密

        let decrypted = decrypt_password(&format!("{}abc", ENCRYPTED_PREFIX));
        assert_eq!(decrypted, ""); // 未设置密钥时解密返回空

        clear_master_key();
    }

    #[test]
    fn test_already_encrypted() {
        let _guard = test_mutex().lock().unwrap();
        set_master_key("test_key");

        let original = "password";
        let encrypted = encrypt_password(original);
        let double_encrypted = encrypt_password(&encrypted);

        // 已加密的不应再次加密
        assert_eq!(encrypted, double_encrypted);

        clear_master_key();
    }
}
