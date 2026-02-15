//! License 本地存储
//!
//! 提供 License 信息的本地缓存存储功能。

use super::error::LicenseError;
use super::models::{LicenseInfo, OfflineLicenseDocument};
use std::fs;
use std::path::PathBuf;

/// License 存储后端 trait
pub trait LicenseStorage: Send + Sync {
    /// 存储后端名称
    fn name(&self) -> &'static str;

    /// 保存 License 信息
    fn save(&self, license: &LicenseInfo) -> Result<(), LicenseError>;

    /// 加载 License 信息
    fn load(&self) -> Option<LicenseInfo>;

    /// 删除存储的 License 信息
    fn delete(&self) -> Result<(), LicenseError>;

    /// 检查是否存在已保存的 License
    fn exists(&self) -> bool;

    /// 保存离线 License 文件
    fn save_offline(&self, license: &OfflineLicenseDocument) -> Result<(), LicenseError> {
        let _ = license;
        Err(LicenseError::StorageError(
            "当前存储不支持离线 License".to_string(),
        ))
    }

    /// 加载离线 License 文件
    fn load_offline(&self) -> Option<OfflineLicenseDocument> {
        None
    }

    /// 删除离线 License 文件
    fn delete_offline(&self) -> Result<(), LicenseError> {
        Ok(())
    }

    /// 检查是否存在离线 License 文件
    fn offline_exists(&self) -> bool {
        false
    }
}

/// 本地文件存储实现
///
/// 将 License 信息以 JSON 格式保存到本地文件。
pub struct LocalLicenseStorage;

impl LocalLicenseStorage {
    /// 获取 License 缓存文件路径
    fn get_license_path() -> Option<PathBuf> {
        dirs::data_dir().map(|p| p.join("one-hub").join("license.json"))
    }

    /// 获取离线 License 文件路径
    fn get_offline_license_path() -> Option<PathBuf> {
        dirs::data_dir().map(|p| p.join("one-hub").join("offline_license.json"))
    }
}

impl LicenseStorage for LocalLicenseStorage {
    fn name(&self) -> &'static str {
        "本地文件"
    }

    fn save(&self, license: &LicenseInfo) -> Result<(), LicenseError> {
        let path = Self::get_license_path()
            .ok_or_else(|| LicenseError::StorageError("无法获取存储路径".to_string()))?;

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 序列化并保存
        let json = serde_json::to_string_pretty(license)?;
        fs::write(&path, json)?;

        tracing::info!("[License 存储] 已保存到本地缓存");
        Ok(())
    }

    fn load(&self) -> Option<LicenseInfo> {
        let path = Self::get_license_path()?;

        if !path.exists() {
            tracing::debug!("[License 存储] 本地缓存不存在");
            return None;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<LicenseInfo>(&content) {
                Ok(license) => {
                    tracing::info!("[License 存储] 从本地缓存加载成功");
                    Some(license)
                }
                Err(e) => {
                    tracing::warn!("[License 存储] 解析缓存失败: {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("[License 存储] 读取缓存失败: {}", e);
                None
            }
        }
    }

    fn delete(&self) -> Result<(), LicenseError> {
        if let Some(path) = Self::get_license_path() {
            if path.exists() {
                fs::remove_file(&path)?;
                tracing::info!("[License 存储] 已删除本地缓存");
            }
        }
        Ok(())
    }

    fn exists(&self) -> bool {
        Self::get_license_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn save_offline(&self, license: &OfflineLicenseDocument) -> Result<(), LicenseError> {
        let path = Self::get_offline_license_path()
            .ok_or_else(|| LicenseError::StorageError("无法获取存储路径".to_string()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(license)?;
        fs::write(&path, json)?;

        tracing::info!("[License 存储] 已保存离线 License");
        Ok(())
    }

    fn load_offline(&self) -> Option<OfflineLicenseDocument> {
        let path = Self::get_offline_license_path()?;

        if !path.exists() {
            tracing::debug!("[License 存储] 离线 License 文件不存在");
            return None;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<OfflineLicenseDocument>(&content) {
                Ok(license) => {
                    tracing::info!("[License 存储] 离线 License 加载成功");
                    Some(license)
                }
                Err(e) => {
                    tracing::warn!("[License 存储] 解析离线 License 失败: {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("[License 存储] 读取离线 License 失败: {}", e);
                None
            }
        }
    }

    fn delete_offline(&self) -> Result<(), LicenseError> {
        if let Some(path) = Self::get_offline_license_path() {
            if path.exists() {
                fs::remove_file(&path)?;
                tracing::info!("[License 存储] 已删除离线 License");
            }
        }
        Ok(())
    }

    fn offline_exists(&self) -> bool {
        Self::get_offline_license_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license::models::PlanTier;
    use std::env;
    use std::fs;

    /// 使用临时目录进行测试的存储实现
    struct TestLicenseStorage {
        path: PathBuf,
    }

    impl TestLicenseStorage {
        fn new() -> Self {
            let path = env::temp_dir()
                .join("onetcli-test")
                .join(format!("license-{}.json", std::process::id()));
            Self { path }
        }
    }

    impl Drop for TestLicenseStorage {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    impl LicenseStorage for TestLicenseStorage {
        fn name(&self) -> &'static str {
            "测试存储"
        }

        fn save(&self, license: &LicenseInfo) -> Result<(), LicenseError> {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(license)?;
            fs::write(&self.path, json)?;
            Ok(())
        }

        fn load(&self) -> Option<LicenseInfo> {
            let content = fs::read_to_string(&self.path).ok()?;
            serde_json::from_str(&content).ok()
        }

        fn delete(&self) -> Result<(), LicenseError> {
            if self.path.exists() {
                fs::remove_file(&self.path)?;
            }
            Ok(())
        }

        fn exists(&self) -> bool {
            self.path.exists()
        }
    }

    #[test]
    fn test_save_and_load() {
        let storage = TestLicenseStorage::new();
        let license = LicenseInfo::new("user123".to_string(), PlanTier::Pro, None);

        // 保存
        storage.save(&license).unwrap();
        assert!(storage.exists());

        // 加载
        let loaded = storage.load().unwrap();
        assert_eq!(loaded.user_id, "user123");
        assert_eq!(loaded.plan, PlanTier::Pro);

        // 删除
        storage.delete().unwrap();
        assert!(!storage.exists());
    }
}
