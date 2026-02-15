//! License 模块
//!
//! 提供付费功能控制和订阅管理。
//!
//! # 概述
//!
//! 本模块实现混合模式的 License 验证系统：
//! - 服务端验证：登录后从 Supabase 获取订阅状态
//! - 本地缓存：支持离线使用（7 天有效期）
//!
//! # 使用方式
//!
//! ```rust,ignore
//! use one_core::license::{LicenseService, Feature, PlanTier};
//! use std::sync::Arc;
//!
//! // 创建服务
//! let storage = Arc::new(LocalLicenseStorage);
//! let service = LicenseService::new(storage);
//!
//! // 尝试从缓存恢复
//! service.restore_from_cache();
//!
//! // 检查功能权限
//! if service.is_feature_enabled(Feature::CloudSync) {
//!     // 执行云同步
//! }
//! ```

mod error;
mod models;
mod service;
mod storage;

pub use error::LicenseError;
pub use models::{
    Feature, LicenseInfo, OfflineLicenseDocument, OfflineLicensePayload, PlanTier, SubscriptionInfo,
    SubscriptionStatus,
};
pub use service::LicenseService;
pub use storage::{LicenseStorage, LocalLicenseStorage};
