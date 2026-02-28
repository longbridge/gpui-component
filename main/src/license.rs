//! 全局 License 服务模块
//!
//! 提供全局 License 状态管理和便捷访问函数。

use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use gpui::{App, Context, ParentElement, Styled, Window, px};
use gpui_component::{ActiveTheme, Icon, IconName, WindowExt, dialog::DialogButtonProps, v_flex};
use one_core::license::{Feature, LicenseService, LocalLicenseStorage};
use rust_i18n::t;

// ============================================================================
// 全局 License 服务
// ============================================================================

const OFFLINE_LICENSE_PUBLIC_KEY_BASE64: &str = "";

/// 全局 License 服务包装器
#[derive(Clone)]
pub struct GlobalLicenseService(pub Arc<LicenseService>);

impl gpui::Global for GlobalLicenseService {}

/// 初始化全局 License 服务
///
/// 应在应用启动时、认证服务初始化后调用。
pub fn init(cx: &mut App) {
    let storage = Arc::new(LocalLicenseStorage);
    let service = Arc::new(LicenseService::new(storage));

    // 尝试从本地缓存恢复
    let restored = match offline_license_public_key() {
        Ok(public_key) => service.restore_hybrid(&public_key, None),
        Err(_) => service.restore_from_cache(),
    };

    if let Some(license) = restored {
        tracing::info!(
            "[License] 从缓存恢复: plan={:?}, user_id={}",
            license.plan,
            license.user_id
        );
    }

    cx.set_global(GlobalLicenseService(service));
}

/// 获取全局 License 服务
pub fn get_license_service(cx: &App) -> Arc<LicenseService> {
    cx.global::<GlobalLicenseService>().0.clone()
}

pub fn offline_license_public_key() -> Result<[u8; 32], String> {
    if OFFLINE_LICENSE_PUBLIC_KEY_BASE64.trim().is_empty() {
        return Err("未配置离线 License 公钥".to_string());
    }

    let decoded = BASE64
        .decode(OFFLINE_LICENSE_PUBLIC_KEY_BASE64.as_bytes())
        .map_err(|e| format!("离线 License 公钥解码失败: {}", e))?;

    let bytes: [u8; 32] = decoded
        .try_into()
        .map_err(|_| "离线 License 公钥长度错误".to_string())?;

    Ok(bytes)
}

/// 便捷方法：检查功能是否已启用
pub fn is_feature_enabled(feature: Feature, cx: &App) -> bool {
    if let Some(global) = cx.try_global::<GlobalLicenseService>() {
        global.0.is_feature_enabled(feature)
    } else {
        false
    }
}

// ============================================================================
// 升级提示对话框
// ============================================================================

/// 显示升级到 Pro 的对话框
pub fn show_upgrade_dialog<V: 'static>(window: &mut Window, cx: &mut Context<V>) {
    window.open_dialog(cx, |dialog, _window, cx| {
        dialog
            .title(t!("License.upgrade_title").to_string())
            .width(px(420.))
            .child(
                v_flex()
                    .gap_4()
                    .p_4()
                    // 主要提示文字
                    .child(
                        gpui::div()
                            .text_base()
                            .text_color(cx.theme().foreground)
                            .child(t!("License.upgrade_message").to_string()),
                    )
                    // 功能列表
                    .child(v_flex().gap_2().child(feature_item(
                        IconName::Refresh,
                        t!("License.feature_cloud_sync").to_string(),
                        cx,
                    ))),
            )
            .button_props(DialogButtonProps::default().ok_text(t!("License.upgrade_button")))
            .on_ok(|_, _, cx| {
                // 打开购买页面
                cx.open_url("https://onehub.app/pricing");
                true
            })
    });
}

/// 渲染功能项
fn feature_item(icon: IconName, text: String, cx: &App) -> impl gpui::IntoElement {
    gpui::div()
        .flex()
        .items_center()
        .gap_2()
        .child(Icon::new(icon).size_4().text_color(cx.theme().primary))
        .child(gpui::div().text_sm().child(text))
}
