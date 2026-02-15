//! License 错误类型

use std::fmt;

/// License 相关错误
#[derive(Debug, Clone)]
pub enum LicenseError {
    /// 未登录
    NotAuthenticated,
    /// 网络错误
    NetworkError(String),
    /// 服务端错误
    ServerError(String),
    /// 数据解析错误
    ParseError(String),
    /// 存储错误
    StorageError(String),
    /// License 已过期
    LicenseExpired,
    /// 功能未授权
    FeatureNotAuthorized(String),
    /// 离线 License 无效
    InvalidOfflineLicense(String),
    /// 离线 License 设备不匹配
    OfflineLicenseDeviceMismatch,
}

impl fmt::Display for LicenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LicenseError::NotAuthenticated => write!(f, "未登录"),
            LicenseError::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            LicenseError::ServerError(msg) => write!(f, "服务端错误: {}", msg),
            LicenseError::ParseError(msg) => write!(f, "数据解析错误: {}", msg),
            LicenseError::StorageError(msg) => write!(f, "存储错误: {}", msg),
            LicenseError::LicenseExpired => write!(f, "License 已过期"),
            LicenseError::FeatureNotAuthorized(feature) => {
                write!(f, "功能未授权: {}", feature)
            }
            LicenseError::InvalidOfflineLicense(msg) => write!(f, "离线 License 无效: {}", msg),
            LicenseError::OfflineLicenseDeviceMismatch => write!(f, "离线 License 设备不匹配"),
        }
    }
}

impl std::error::Error for LicenseError {}

impl From<std::io::Error> for LicenseError {
    fn from(err: std::io::Error) -> Self {
        LicenseError::StorageError(err.to_string())
    }
}

impl From<serde_json::Error> for LicenseError {
    fn from(err: serde_json::Error) -> Self {
        LicenseError::ParseError(err.to_string())
    }
}
