//! License 数据模型
//!
//! 定义付费等级、功能标识和 License 信息结构。

use serde::{Deserialize, Serialize};

/// 付费等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlanTier {
    /// 免费版
    #[default]
    Free,
    /// 专业版
    Pro,
}

impl PlanTier {
    /// 从字符串解析付费等级
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pro" => PlanTier::Pro,
            _ => PlanTier::Free,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanTier::Free => "free",
            PlanTier::Pro => "pro",
        }
    }

    /// 获取该等级包含的功能列表
    pub fn features(&self) -> Vec<Feature> {
        match self {
            PlanTier::Free => vec![],
            PlanTier::Pro => vec![Feature::CloudSync],
        }
    }

    /// 检查该等级是否包含指定功能
    pub fn has_feature(&self, feature: Feature) -> bool {
        self.features().contains(&feature)
    }
}

/// 功能标识
///
/// 定义所有需要 License 控制的付费功能。
/// 设计为可扩展，后续可添加更多功能。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// 云端数据同步
    CloudSync,
    // 预留扩展：
    // /// AI 聊天功能
    // AiChat,
    // /// 高级数据导出
    // AdvancedExport,
    // /// 团队协作
    // TeamCollaboration,
}

impl Feature {
    /// 转换为字符串标识
    pub fn as_str(&self) -> &'static str {
        match self {
            Feature::CloudSync => "cloud_sync",
        }
    }
}

/// License 信息
///
/// 存储用户的订阅状态和已解锁功能。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    /// 用户 ID
    pub user_id: String,
    /// 付费等级
    pub plan: PlanTier,
    /// 已解锁功能列表
    pub features: Vec<Feature>,
    /// 订阅过期时间（Unix 时间戳，秒）
    /// None 表示永久有效或免费用户
    pub expires_at: Option<i64>,
    /// 本地缓存时间（Unix 时间戳，秒）
    pub cached_at: i64,
}

impl LicenseInfo {
    /// 创建新的 License 信息
    pub fn new(user_id: String, plan: PlanTier, expires_at: Option<i64>) -> Self {
        let features = plan.features();
        let cached_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Self {
            user_id,
            plan,
            features,
            expires_at,
            cached_at,
        }
    }

    /// 检查功能是否已解锁
    pub fn has_feature(&self, feature: Feature) -> bool {
        // 先检查订阅是否过期
        if self.is_subscription_expired() {
            return false;
        }
        self.features.contains(&feature)
    }

    /// 检查是否是 Pro 用户
    pub fn is_pro(&self) -> bool {
        !self.is_subscription_expired() && self.plan == PlanTier::Pro
    }

    /// 检查订阅是否已过期
    pub fn is_subscription_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            expires_at <= now
        } else {
            false
        }
    }

    /// 检查本地缓存是否过期（默认 7 天有效期）
    pub fn is_cache_expired(&self) -> bool {
        self.is_cache_expired_with_ttl(7 * 24 * 60 * 60)
    }

    /// 检查本地缓存是否过期（自定义有效期，单位：秒）
    pub fn is_cache_expired_with_ttl(&self, ttl_seconds: i64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now - self.cached_at > ttl_seconds
    }
}

/// 订阅状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    /// 活跃
    Active,
    /// 已取消（但未到期）
    Cancelled,
    /// 已过期
    Expired,
}

impl SubscriptionStatus {
    /// 从字符串解析订阅状态
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => SubscriptionStatus::Active,
            "cancelled" => SubscriptionStatus::Cancelled,
            "expired" => SubscriptionStatus::Expired,
            _ => SubscriptionStatus::Expired,
        }
    }

    /// 检查订阅是否有效（活跃或已取消但未到期）
    pub fn is_valid(&self) -> bool {
        matches!(self, SubscriptionStatus::Active | SubscriptionStatus::Cancelled)
    }
}

/// 服务端返回的订阅信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    /// 付费等级（"free" | "pro"）
    pub plan: String,
    /// 订阅状态（"active" | "cancelled" | "expired"）
    pub status: String,
    /// 过期时间（Unix 时间戳）
    pub expires_at: Option<i64>,
}

impl SubscriptionInfo {
    /// 转换为 LicenseInfo
    pub fn to_license_info(&self, user_id: String) -> LicenseInfo {
        let plan = PlanTier::from_str(&self.plan);
        let status = SubscriptionStatus::from_str(&self.status);

        // 如果订阅无效，降级为免费版
        let effective_plan = if status.is_valid() { plan } else { PlanTier::Free };

        LicenseInfo::new(user_id, effective_plan, self.expires_at)
    }
}

/// 离线 License 载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineLicensePayload {
    /// 版本号
    pub version: u32,
    /// 用户 ID
    pub user_id: String,
    /// 付费等级
    pub plan: PlanTier,
    /// 过期时间（Unix 时间戳）
    pub expires_at: Option<i64>,
    /// 签发时间（Unix 时间戳）
    pub issued_at: i64,
    /// 绑定的设备 ID
    pub device_id: Option<String>,
}

impl OfflineLicensePayload {
    /// 支持的离线 License 版本
    pub const VERSION: u32 = 1;

    /// 转换为 LicenseInfo
    pub fn to_license_info(&self) -> LicenseInfo {
        LicenseInfo::new(self.user_id.clone(), self.plan, self.expires_at)
    }

    /// 检查离线 License 是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            expires_at <= now
        } else {
            false
        }
    }
}

/// 离线 License 文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineLicenseDocument {
    /// base64 编码的 JSON 载荷
    pub payload: String,
    /// base64 编码的签名
    pub signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_tier_features() {
        assert!(PlanTier::Free.features().is_empty());
        assert!(PlanTier::Pro.features().contains(&Feature::CloudSync));
    }

    #[test]
    fn test_license_info_has_feature() {
        let license = LicenseInfo::new("user1".to_string(), PlanTier::Pro, None);
        assert!(license.has_feature(Feature::CloudSync));

        let free_license = LicenseInfo::new("user2".to_string(), PlanTier::Free, None);
        assert!(!free_license.has_feature(Feature::CloudSync));
    }

    #[test]
    fn test_subscription_expiry() {
        // 已过期的订阅
        let expired_license = LicenseInfo::new("user1".to_string(), PlanTier::Pro, Some(0));
        assert!(expired_license.is_subscription_expired());
        assert!(!expired_license.has_feature(Feature::CloudSync));

        // 未过期的订阅
        let future_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 86400;
        let valid_license = LicenseInfo::new("user2".to_string(), PlanTier::Pro, Some(future_time));
        assert!(!valid_license.is_subscription_expired());
        assert!(valid_license.has_feature(Feature::CloudSync));
    }
}
