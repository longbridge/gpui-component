//! License 服务
//!
//! 提供 License 验证、缓存管理和功能权限检查。

use super::error::LicenseError;
use super::models::{
    Feature, LicenseInfo, OfflineLicenseDocument, OfflineLicensePayload, PlanTier, SubscriptionInfo,
};
use super::storage::LicenseStorage;
use crate::cloud_sync::CloudApiError;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, VerifyingKey};
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// License 缓存有效期（7 天，单位：秒）
const LICENSE_CACHE_TTL: i64 = 7 * 24 * 60 * 60;

/// License 服务
///
/// 管理 License 状态、缓存和功能权限检查。
pub struct LicenseService {
    /// 在线订阅 License（内存缓存）
    online_license: RwLock<Option<LicenseInfo>>,
    /// 离线 License（内存缓存）
    offline_license: RwLock<Option<LicenseInfo>>,
    /// 本地缓存存储
    storage: Arc<dyn LicenseStorage>,
}

impl LicenseService {
    /// 创建新的 License 服务
    pub fn new(storage: Arc<dyn LicenseStorage>) -> Self {
        Self {
            online_license: RwLock::new(None),
            offline_license: RwLock::new(None),
            storage,
        }
    }

    fn effective_license(&self) -> Option<LicenseInfo> {
        let online = self
            .online_license
            .read()
            .ok()
            .and_then(|guard| guard.clone());
        let offline = self
            .offline_license
            .read()
            .ok()
            .and_then(|guard| guard.clone());
        Self::select_effective_license(online, offline)
    }

    fn select_effective_license(
        online: Option<LicenseInfo>,
        offline: Option<LicenseInfo>,
    ) -> Option<LicenseInfo> {
        let online = online.filter(|license| !license.is_subscription_expired());
        let offline = offline.filter(|license| !license.is_subscription_expired());

        match (online, offline) {
            (Some(online), Some(offline)) => {
                if Self::license_rank(&offline) > Self::license_rank(&online) {
                    Some(offline)
                } else {
                    Some(online)
                }
            }
            (Some(license), None) | (None, Some(license)) => Some(license),
            (None, None) => None,
        }
    }

    fn license_rank(license: &LicenseInfo) -> (i32, i64) {
        let plan_rank = match license.plan {
            PlanTier::Pro => 1,
            PlanTier::Free => 0,
        };
        let expiry_rank = license.expires_at.unwrap_or(i64::MAX);
        (plan_rank, expiry_rank)
    }

    /// 检查功能是否已解锁
    pub fn is_feature_enabled(&self, feature: Feature) -> bool {
        self.effective_license()
            .map(|license| license.has_feature(feature))
            .unwrap_or(false)
    }

    /// 检查是否是 Pro 用户
    pub fn is_pro(&self) -> bool {
        self.effective_license()
            .map(|license| license.is_pro())
            .unwrap_or(false)
    }

    /// 获取当前付费等级
    pub fn get_plan(&self) -> PlanTier {
        self.effective_license()
            .map(|license| license.plan)
            .unwrap_or(PlanTier::Free)
    }

    /// 获取当前 License 信息的副本
    pub fn get_license(&self) -> Option<LicenseInfo> {
        self.effective_license()
    }

    /// 从本地缓存恢复 License
    ///
    /// 应用启动时调用。如果缓存存在且未过期，则使用缓存的 License。
    pub fn restore_from_cache(&self) -> Option<LicenseInfo> {
        let cached = self.storage.load()?;

        // 检查缓存是否过期
        if cached.is_cache_expired_with_ttl(LICENSE_CACHE_TTL) {
            tracing::info!("[License] 本地缓存已过期，等待登录后刷新");
            return None;
        }

        // 检查订阅是否过期
        if cached.is_subscription_expired() {
            tracing::info!("[License] 订阅已过期，降级为免费版");
            // 订阅过期，创建免费版 License
            let free_license = LicenseInfo::new(cached.user_id, PlanTier::Free, None);
            if let Ok(mut guard) = self.online_license.write() {
                *guard = Some(free_license.clone());
            }
            return Some(free_license);
        }

        tracing::info!(
            "[License] 从缓存恢复成功: plan={:?}, user_id={}",
            cached.plan,
            cached.user_id
        );

        if let Ok(mut guard) = self.online_license.write() {
            *guard = Some(cached.clone());
        }

        Some(cached)
    }

    /// 从离线 License 缓存恢复
    pub fn restore_offline_from_cache(
        &self,
        public_key: &[u8],
        device_id: Option<&str>,
    ) -> Option<LicenseInfo> {
        let document = self.storage.load_offline()?;
        match Self::verify_offline_license_document(&document, public_key, device_id) {
            Ok(payload) => {
                let license = payload.to_license_info();
                if let Ok(mut guard) = self.offline_license.write() {
                    *guard = Some(license.clone());
                }
                tracing::info!(
                    "[License] 离线 License 恢复成功: plan={:?}, user_id={}",
                    license.plan,
                    license.user_id
                );
                Some(license)
            }
            Err(e) => {
                tracing::warn!("[License] 离线 License 恢复失败: {}", e);
                None
            }
        }
    }

    /// 混合模式恢复（优先离线 License，其次在线缓存）
    pub fn restore_hybrid(
        &self,
        public_key: &[u8],
        device_id: Option<&str>,
    ) -> Option<LicenseInfo> {
        let _ = self.restore_offline_from_cache(public_key, device_id);
        let _ = self.restore_from_cache();
        self.effective_license()
    }

    /// 导入离线 License（传入文件内容）
    pub fn import_offline_license(
        &self,
        license_content: &str,
        public_key: &[u8],
        device_id: Option<&str>,
    ) -> Result<LicenseInfo, LicenseError> {
        let document: OfflineLicenseDocument = serde_json::from_str(license_content)?;
        let payload = Self::verify_offline_license_document(&document, public_key, device_id)?;
        let license = payload.to_license_info();

        if let Ok(mut guard) = self.offline_license.write() {
            *guard = Some(license.clone());
        }

        self.storage.save_offline(&document)?;

        tracing::info!(
            "[License] 离线 License 导入成功: plan={:?}, user_id={}",
            license.plan,
            license.user_id
        );

        Ok(license)
    }

    /// 导入离线 License（从文件路径读取）
    pub fn import_offline_license_from_path<P: AsRef<Path>>(
        &self,
        path: P,
        public_key: &[u8],
        device_id: Option<&str>,
    ) -> Result<LicenseInfo, LicenseError> {
        let content = fs::read_to_string(path)?;
        self.import_offline_license(&content, public_key, device_id)
    }

    /// 使用订阅信息更新 License
    ///
    /// 登录成功后调用，从服务端获取订阅信息后更新本地状态。
    pub fn update_from_subscription(
        &self,
        user_id: String,
        subscription: Option<SubscriptionInfo>,
    ) -> Result<LicenseInfo, LicenseError> {
        let license = match subscription {
            Some(sub) => {
                tracing::info!(
                    "[License] 从服务端获取订阅信息: plan={}, status={}",
                    sub.plan,
                    sub.status
                );
                sub.to_license_info(user_id)
            }
            None => {
                tracing::info!("[License] 用户无订阅记录，使用免费版");
                LicenseInfo::new(user_id, PlanTier::Free, None)
            }
        };

        // 更新内存缓存
        if let Ok(mut guard) = self.online_license.write() {
            *guard = Some(license.clone());
        }

        // 保存到本地缓存
        self.storage.save(&license)?;

        tracing::info!(
            "[License] 更新完成: plan={:?}, features={:?}",
            license.plan,
            license.features
        );

        Ok(license)
    }

    /// 设置为免费用户（用于未登录状态）
    pub fn set_free(&self) {
        if let Ok(mut guard) = self.online_license.write() {
            *guard = None;
        }
    }

    /// 清除在线 License
    ///
    /// 登出时调用，清除内存和本地缓存。
    pub fn clear(&self) {
        if let Ok(mut guard) = self.online_license.write() {
            *guard = None;
        }

        if let Err(e) = self.storage.delete() {
            tracing::warn!("[License] 清除本地缓存失败: {}", e);
        }

        tracing::info!("[License] 已清除");
    }

    /// 清除离线 License
    pub fn clear_offline_license(&self) {
        if let Ok(mut guard) = self.offline_license.write() {
            *guard = None;
        }

        if let Err(e) = self.storage.delete_offline() {
            tracing::warn!("[License] 清除离线 License 失败: {}", e);
        }
    }

    fn verify_offline_license_document(
        document: &OfflineLicenseDocument,
        public_key: &[u8],
        device_id: Option<&str>,
    ) -> Result<OfflineLicensePayload, LicenseError> {
        let payload_base64 = document.payload.trim();
        let signature_base64 = document.signature.trim();

        let payload_bytes = BASE64.decode(payload_base64.as_bytes()).map_err(|e| {
            LicenseError::InvalidOfflineLicense(format!("payload 解码失败: {}", e))
        })?;
        let signature_bytes = BASE64.decode(signature_base64.as_bytes()).map_err(|e| {
            LicenseError::InvalidOfflineLicense(format!("签名解码失败: {}", e))
        })?;

        let public_key_bytes: [u8; 32] = public_key.try_into().map_err(|_| {
            LicenseError::InvalidOfflineLicense("公钥长度错误".to_string())
        })?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key_bytes).map_err(|_| {
                LicenseError::InvalidOfflineLicense("公钥无效".to_string())
            })?;
        let signature = Signature::from_slice(&signature_bytes).map_err(|_| {
            LicenseError::InvalidOfflineLicense("签名格式错误".to_string())
        })?;

        verifying_key
            .verify_strict(&payload_bytes, &signature)
            .map_err(|_| LicenseError::InvalidOfflineLicense("签名校验失败".to_string()))?;

        let payload: OfflineLicensePayload = serde_json::from_slice(&payload_bytes)?;

        if payload.version != OfflineLicensePayload::VERSION {
            return Err(LicenseError::InvalidOfflineLicense(format!(
                "不支持的版本: {}",
                payload.version
            )));
        }

        if payload.is_expired() {
            return Err(LicenseError::LicenseExpired);
        }

        if let Some(expected_device_id) = payload.device_id.as_deref() {
            match device_id {
                Some(actual_device_id) if actual_device_id == expected_device_id => {}
                _ => return Err(LicenseError::OfflineLicenseDeviceMismatch),
            }
        }

        Ok(payload)
    }
}

/// 从 CloudApiError 转换为 LicenseError
impl From<CloudApiError> for LicenseError {
    fn from(err: CloudApiError) -> Self {
        match err {
            CloudApiError::NotAuthenticated => LicenseError::NotAuthenticated,
            CloudApiError::NetworkError(msg) => LicenseError::NetworkError(msg),
            CloudApiError::ServerError(msg) => LicenseError::ServerError(msg),
            CloudApiError::ParseError(msg) => LicenseError::ParseError(msg),
            _ => LicenseError::ServerError(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license::storage::LicenseStorage;
    use std::sync::RwLock as StdRwLock;

    /// 测试用的内存存储
    struct MemoryLicenseStorage {
        data: StdRwLock<Option<LicenseInfo>>,
    }

    impl MemoryLicenseStorage {
        fn new() -> Self {
            Self {
                data: StdRwLock::new(None),
            }
        }
    }

    impl LicenseStorage for MemoryLicenseStorage {
        fn name(&self) -> &'static str {
            "内存存储"
        }

        fn save(&self, license: &LicenseInfo) -> Result<(), LicenseError> {
            if let Ok(mut guard) = self.data.write() {
                *guard = Some(license.clone());
            }
            Ok(())
        }

        fn load(&self) -> Option<LicenseInfo> {
            self.data.read().ok().and_then(|guard| guard.clone())
        }

        fn delete(&self) -> Result<(), LicenseError> {
            if let Ok(mut guard) = self.data.write() {
                *guard = None;
            }
            Ok(())
        }

        fn exists(&self) -> bool {
            self.data
                .read()
                .ok()
                .map(|guard| guard.is_some())
                .unwrap_or(false)
        }
    }

    #[test]
    fn test_license_service_basic() {
        let storage = Arc::new(MemoryLicenseStorage::new());
        let service = LicenseService::new(storage);

        // 初始状态：无 License
        assert!(!service.is_pro());
        assert!(!service.is_feature_enabled(Feature::CloudSync));
        assert_eq!(service.get_plan(), PlanTier::Free);
    }

    #[test]
    fn test_update_from_subscription() {
        let storage = Arc::new(MemoryLicenseStorage::new());
        let service = LicenseService::new(storage);

        // 更新为 Pro 订阅
        let sub = SubscriptionInfo {
            plan: "pro".to_string(),
            status: "active".to_string(),
            expires_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    + 86400,
            ),
        };

        service
            .update_from_subscription("user1".to_string(), Some(sub))
            .unwrap();

        assert!(service.is_pro());
        assert!(service.is_feature_enabled(Feature::CloudSync));
        assert_eq!(service.get_plan(), PlanTier::Pro);
    }

    #[test]
    fn test_clear_license() {
        let storage = Arc::new(MemoryLicenseStorage::new());
        let service = LicenseService::new(storage.clone());

        // 先设置 Pro
        let sub = SubscriptionInfo {
            plan: "pro".to_string(),
            status: "active".to_string(),
            expires_at: None,
        };
        service
            .update_from_subscription("user1".to_string(), Some(sub))
            .unwrap();

        // 清除
        service.clear();

        assert!(!service.is_pro());
        assert!(!storage.exists());
    }

    #[test]
    fn test_restore_from_cache() {
        let storage = Arc::new(MemoryLicenseStorage::new());

        // 预先保存一个有效的 License
        let license = LicenseInfo::new("user1".to_string(), PlanTier::Pro, None);
        storage.save(&license).unwrap();

        // 创建服务并恢复
        let service = LicenseService::new(storage);
        let restored = service.restore_from_cache();

        assert!(restored.is_some());
        assert!(service.is_pro());
    }
}
