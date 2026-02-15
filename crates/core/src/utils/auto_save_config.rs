use gpui::Global;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// SQL编辑器自动保存全局配置
///
/// 此结构体存储自动保存功能的配置参数，可以在运行时动态更新。
/// 通过 gpui::Global trait 使其在整个应用中可访问。
pub struct AutoSaveConfig {
    /// 是否启用自动保存功能
    enabled: AtomicBool,
    /// 自动保存间隔（毫秒），使用原子类型以支持运行时动态修改
    interval_ms: AtomicU64,
}

impl AutoSaveConfig {
    /// 创建新的自动保存配置
    ///
    /// # 参数
    /// * `enabled` - 是否启用自动保存
    /// * `interval_seconds` - 自动保存间隔（秒）
    pub fn new(enabled: bool, interval_seconds: f64) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            interval_ms: AtomicU64::new((interval_seconds * 1000.0) as u64),
        }
    }

    /// 检查自动保存是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// 设置自动保存启用状态
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// 获取自动保存间隔（毫秒）
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms.load(Ordering::Relaxed)
    }

    /// 设置自动保存间隔（秒）
    pub fn set_interval_seconds(&self, interval_seconds: f64) {
        let interval_ms = (interval_seconds * 1000.0) as u64;
        self.interval_ms.store(interval_ms, Ordering::Relaxed);
    }
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        // 默认启用，间隔5秒
        Self::new(true, 5.0)
    }
}

impl Global for AutoSaveConfig {}
