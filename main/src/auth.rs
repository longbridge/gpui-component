//! 用户认证模块
//!
//! 提供 Supabase 认证集成，包括登录、登出、会话持久化等功能。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::http_client::HttpClient;
use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext as _, AsyncApp, Context, Entity, FontWeight, ParentElement, Styled, Window, px,
};

/// 全局静态会话过期标志
///
/// 用于在 SupabaseClient 的回调中（无法访问 GPUI 全局状态时）通知 UI 层会话已过期。
/// UI 层定期检查此标志，若为 true 则弹出登录对话框。
static SESSION_EXPIRED: AtomicBool = AtomicBool::new(false);
use gpui_component::button::Button;
use gpui_component::dialog::DialogButtonProps;
use gpui_component::{
    ActiveTheme, Disableable, Sizable, WindowExt,
    h_flex,
    input::{Input, InputState},
    v_flex,
};
use one_core::cloud_sync::{
    CloudApiClient, UserInfo,
    supabase::{SessionExpiredCallback, SupabaseClient, SupabaseConfig},
};
use rust_i18n::t;
use tracing::{info, warn};
// ============================================================================
// 全局认证服务
// ============================================================================

/// 全局认证服务包装器
#[derive(Clone)]
pub struct GlobalAuthService(pub Arc<AuthService>);

impl gpui::Global for GlobalAuthService {}

/// 初始化全局认证服务
pub fn init(cx: &mut App) {
    let config = one_core::config::SupabaseConfig::get();
    let supabase_config = SupabaseConfig {
        project_url: config.project_url,
        api_key: config.api_key,
    };
    let http = cx.http_client();
    let service = Arc::new(AuthService::new_with_http(supabase_config, http, cx));
    cx.set_global(GlobalAuthService(service));
}

/// 获取全局认证服务
pub fn get_auth_service(cx: &App) -> Arc<AuthService> {
    cx.global::<GlobalAuthService>().0.clone()
}

/// 检查会话是否已过期（并重置标志）
///
/// UI 层在 render 或定时器中调用此方法检测会话过期。
/// 返回 true 表示会话已过期，需要弹出登录对话框。
pub fn check_and_reset_session_expired() -> bool {
    let expired = SESSION_EXPIRED.swap(false, Ordering::SeqCst);
    if expired {
        warn!("检测到会话过期标志，准备弹出登录对话框");
    }
    expired
}

// ============================================================================
// 认证服务
// ============================================================================

/// 认证服务，管理 Supabase 客户端和用户状态
pub struct AuthService {
    client: Arc<SupabaseClient>,
}

impl AuthService {
    /// 获取云端 API 客户端
    ///
    /// 用于访问云端数据同步功能（如 list_connections）。
    pub fn cloud_client(&self) -> Arc<SupabaseClient> {
        self.client.clone()
    }

    /// 使用配置和 HttpClient 创建认证服务
    fn new_with_http(config: SupabaseConfig, http: Arc<dyn HttpClient>, _cx: &App) -> Self {
        let client = Arc::new(SupabaseClient::new(config, http));

        // 设置会话过期回调：刷新 token 失败时通过静态标志通知 UI
        let callback: SessionExpiredCallback = Arc::new(|| {
            tracing::warn!("会话已过期，需要重新登录");
            SESSION_EXPIRED.store(true, Ordering::SeqCst);
        });
        client.set_session_expired_callback(callback);

        Self { client }
    }

    /// 尝试恢复会话
    pub async fn try_restore_session(&self) -> Option<UserInfo> {
        info!("开始尝试恢复会话");
        let auth_data = load_auth_data();
        let Some((access_token, refresh_token, user_id, expires_at)) = auth_data else {
            warn!("恢复会话失败: 本地无认证数据");
            return None;
        };
        info!(
            "已读取本地认证数据: user_id={} expires_at={}",
            user_id, expires_at
        );

        // 检查令牌是否已过期（提前 60 秒刷新）
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let needs_refresh = expires_at <= now + 60;
        info!(
            "令牌过期检查: now={} expires_at={} diff={}s needs_refresh={}",
            now,
            expires_at,
            expires_at - now,
            needs_refresh
        );

        if needs_refresh {
            info!("访问令牌需要刷新: now={} expires_at={}", now, expires_at);
            // 令牌已过期或即将过期，必须刷新
            match self.client.refresh_token(&refresh_token).await {
                Ok(auth_resp) => {
                    // refresh_token 内部已调用 set_auth_with_expiry 更新内存状态
                    save_auth_data(
                        &auth_resp.access_token,
                        &auth_resp.refresh_token,
                        &auth_resp.user_id,
                        auth_resp.expires_at,
                    );
                    info!(
                        "令牌刷新成功: user_id={} new_expires_at={}",
                        auth_resp.user_id, auth_resp.expires_at
                    );
                }
                Err(e) => {
                    // 刷新失败，清除本地数据
                    warn!("令牌刷新失败，清除本地认证数据: {}", e);
                    clear_auth_data();
                    return None;
                }
            }
        } else {
            // 令牌未过期，先设置 auth state（含 expires_at）
            self.client
                .set_auth_with_expiry(access_token, refresh_token.clone(), user_id, expires_at);
            info!(
                "访问令牌有效（剩余 {}s），已设置认证状态",
                expires_at - now
            );

            // 尝试在后台刷新令牌以获取最新 token
            match self.client.refresh_token(&refresh_token).await {
                Ok(auth_resp) => {
                    // refresh_token 内部已调用 set_auth_with_expiry 更新内存状态
                    save_auth_data(
                        &auth_resp.access_token,
                        &auth_resp.refresh_token,
                        &auth_resp.user_id,
                        auth_resp.expires_at,
                    );
                    info!(
                        "后台刷新令牌成功: user_id={} new_expires_at={}",
                        auth_resp.user_id, auth_resp.expires_at
                    );
                }
                Err(e) => {
                    warn!("后台刷新令牌失败（将使用现有 token 继续）: {}", e);
                }
            }
        }

        // 获取用户信息
        match self.client.get_current_user().await {
            Ok(Some(user)) => {
                info!("恢复会话成功: user_id={} email={}", user.id, user.email);
                Some(user)
            }
            Ok(None) => {
                warn!("恢复会话失败: 用户信息为空，清除本地认证数据");
                clear_auth_data();
                None
            }
            Err(e) => {
                // 获取用户信息失败，清除本地数据
                warn!("恢复会话失败: 获取用户信息错误: {}", e);
                clear_auth_data();
                None
            }
        }
    }

    /// 登出
    pub async fn sign_out(&self) {
        info!("用户登出");
        let _ = self.client.sign_out().await;
        clear_auth_data();
    }

    /// 发送 OTP 验证码到邮箱
    pub async fn send_otp(&self, email: &str) -> Result<(), String> {
        self.client
            .sign_in_with_otp(email)
            .await
            .map_err(|e| e.to_string())
    }

    /// 验证 OTP 验证码并登录
    pub async fn verify_otp(&self, email: &str, token: &str) -> Result<UserInfo, String> {
        match self.client.verify_otp(email, token).await {
            Ok(auth_resp) => {
                save_auth_data(
                    &auth_resp.access_token,
                    &auth_resp.refresh_token,
                    &auth_resp.user_id,
                    auth_resp.expires_at,
                );

                // 获取完整用户信息
                match self.client.get_current_user().await {
                    Ok(Some(user)) => Ok(user),
                    Ok(None) => Ok(UserInfo {
                        id: auth_resp.user_id,
                        email: auth_resp.email,
                        username: None,
                        avatar_url: None,
                        created_at: 0,
                    }),
                    Err(e) => Err(e.to_string()),
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

// ============================================================================
// 认证持久化
// ============================================================================

/// 获取认证数据存储路径
fn get_auth_file_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|p| p.join("one-hub").join("auth.json"))
}

/// 保存认证数据到本地
pub fn save_auth_data(access_token: &str, refresh_token: &str, user_id: &str, expires_at: i64) {
    if let Some(path) = get_auth_file_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let data = serde_json::json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "user_id": user_id,
            "expires_at": expires_at,
        });

        if let Ok(json) = serde_json::to_string(&data) {
            match std::fs::write(&path, json) {
                Ok(()) => info!(
                    "认证数据已保存: user_id={} expires_at={} path={}",
                    user_id,
                    expires_at,
                    path.display()
                ),
                Err(e) => warn!("认证数据保存失败: {}", e),
            }
        }
    } else {
        warn!("无法获取认证数据存储路径");
    }
}

/// 从本地加载认证数据
pub fn load_auth_data() -> Option<(String, String, String, i64)> {
    let path = get_auth_file_path()?;
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!("读取认证数据失败: {} path={}", e, path.display());
            }
            return None;
        }
    };
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;

    let access_token = data.get("access_token")?.as_str()?.to_string();
    let refresh_token = data.get("refresh_token")?.as_str()?.to_string();
    let user_id = data.get("user_id")?.as_str()?.to_string();
    // 兼容旧数据：如果没有 expires_at，默认为 0（会触发刷新）
    let expires_at = data.get("expires_at").and_then(|v| v.as_i64()).unwrap_or(0);

    info!(
        "加载本地认证数据: user_id={} expires_at={} token_len={} refresh_token_len={}",
        user_id,
        expires_at,
        access_token.len(),
        refresh_token.len()
    );

    Some((access_token, refresh_token, user_id, expires_at))
}

/// 清除本地认证数据
pub fn clear_auth_data() {
    if let Some(path) = get_auth_file_path() {
        match std::fs::remove_file(&path) {
            Ok(()) => info!("本地认证数据已清除: path={}", path.display()),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    warn!("清除本地认证数据失败: {}", e);
                }
            }
        }
    }
}

// ============================================================================
// 登录/注册对话框
// ============================================================================

/// 倒计时秒数常量
const COUNTDOWN_SECONDS: u32 = 60;

/// 显示 OTP 认证对话框
///
/// 使用 OTP（一次性验证码）方式登录：
/// 1. 用户输入邮箱，点击「发送验证码」按钮
/// 2. 按钮进入倒计时，用户输入收到的验证码
/// 3. 点击「登录」完成验证
pub fn show_auth_dialog<V: 'static>(
    window: &mut Window,
    cx: &mut Context<V>,
    view: Entity<V>,
    on_submit: impl Fn(&mut V, String, String, &mut Context<V>) + 'static,
) {
    let email_input =
        cx.new(|cx| InputState::new(window, cx).placeholder(t!("Auth.email_placeholder")));
    let otp_input =
        cx.new(|cx| InputState::new(window, cx).placeholder(t!("Auth.otp_placeholder")));
    let error_message = cx.new(|_| Option::<String>::None);
    // 倒计时剩余秒数，0 表示未在倒计时
    let countdown_state = cx.new(|_| 0u32);
    let sending_state = cx.new(|_| false);
    // 是否已经发送过验证码（用于显示提示文字）
    let otp_sent_state = cx.new(|_| false);

    let email_for_ok = email_input.clone();
    let otp_for_ok = otp_input.clone();
    let error_for_ok = error_message.clone();

    let email_for_render = email_input.clone();
    let otp_for_render = otp_input.clone();
    let error_for_render = error_message.clone();
    let countdown_for_render = countdown_state.clone();
    let sending_for_render = sending_state.clone();
    let otp_sent_for_render = otp_sent_state.clone();

    let on_submit = std::rc::Rc::new(on_submit);
    let on_submit_clone = on_submit.clone();

    window.open_dialog(cx, move |dialog, _window, cx| {
        let email_ok = email_for_ok.clone();
        let otp_ok = otp_for_ok.clone();
        let error_ok = error_for_ok.clone();

        let email_render = email_for_render.clone();
        let otp_render = otp_for_render.clone();
        let error_render = error_for_render.clone();
        let countdown_render = countdown_for_render.clone();
        let sending_render = sending_for_render.clone();
        let otp_sent_render = otp_sent_for_render.clone();

        let on_submit_ok = on_submit_clone.clone();
        let view_clone = view.clone();

        let countdown_val = *countdown_render.read(cx);
        let is_sending = *sending_render.read(cx);
        let has_sent = *otp_sent_render.read(cx);

        // 发送验证码按钮文案
        let send_button_label = if is_sending {
            t!("Auth.sending_otp").to_string()
        } else if countdown_val > 0 {
            format!("{}s", countdown_val)
        } else if has_sent {
            t!("Auth.resend_otp").to_string()
        } else {
            t!("Auth.send_otp").to_string()
        };

        let send_button_disabled = is_sending || countdown_val > 0;

        dialog
            .title(t!("Auth.login"))
            .width(px(400.))
            .confirm()
            .button_props(DialogButtonProps::default().ok_text(t!("Auth.login")))
            .on_ok(move |_, _window, cx| {
                let email = email_ok.read(cx).text().to_string();
                let otp = otp_ok.read(cx).text().to_string();

                if email.is_empty() {
                    error_ok.update(cx, |msg, cx| {
                        *msg = Some(t!("Auth.email_required").to_string());
                        cx.notify();
                    });
                    return false;
                }

                if otp.is_empty() {
                    error_ok.update(cx, |msg, cx| {
                        *msg = Some(t!("Auth.otp_required").to_string());
                        cx.notify();
                    });
                    return false;
                }

                // 触发验证回调
                view_clone.update(cx, |this, cx| {
                    on_submit_ok(this, email, otp, cx);
                });
                true
            })
            .child(
                v_flex()
                    .gap_4()
                    .p_4()
                    // 邮箱输入
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                gpui::div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(t!("Auth.email").to_string()),
                            )
                            .child(Input::new(&email_render)),
                    )
                    // 验证码输入 + 发送按钮
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                gpui::div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(t!("Auth.otp_code").to_string()),
                            )
                            .child(
                                h_flex().gap_2().child(
                                    Input::new(&otp_render).flex_1(),
                                ).child({
                                    let email_for_send = email_render.clone();
                                    let error_for_send = error_render.clone();
                                    let sending_for_send = sending_render.clone();
                                    let countdown_for_send = countdown_render.clone();
                                    let otp_sent_for_send = otp_sent_render.clone();

                                    Button::new("send-otp")
                                        .xsmall()
                                        .label(send_button_label)
                                        .disabled(send_button_disabled)
                                        .on_click(move |_, _window, cx| {
                                            let email =
                                                email_for_send.read(cx).text().to_string();
                                            if email.is_empty() {
                                                error_for_send.update(cx, |msg, cx| {
                                                    *msg = Some(
                                                        t!("Auth.email_required").to_string(),
                                                    );
                                                    cx.notify();
                                                });
                                                return;
                                            }

                                            // 标记发送中
                                            sending_for_send.update(cx, |s, cx| {
                                                *s = true;
                                                cx.notify();
                                            });

                                            let auth = get_auth_service(cx);
                                            let error_update = error_for_send.clone();
                                            let sending_update = sending_for_send.clone();
                                            let countdown_update = countdown_for_send.clone();
                                            let otp_sent_update = otp_sent_for_send.clone();

                                            cx.spawn(async move |cx: &mut AsyncApp| {
                                                let result = auth.send_otp(&email).await;
                                                cx.update(|cx| {
                                                    sending_update.update(cx, |s, cx| {
                                                        *s = false;
                                                        cx.notify();
                                                    });

                                                    match result {
                                                        Ok(()) => {
                                                            // 标记已发送
                                                            otp_sent_update.update(
                                                                cx,
                                                                |sent, cx| {
                                                                    *sent = true;
                                                                    cx.notify();
                                                                },
                                                            );
                                                            // 启动倒计时
                                                            countdown_update.update(
                                                                cx,
                                                                |c, cx| {
                                                                    *c = COUNTDOWN_SECONDS;
                                                                    cx.notify();
                                                                },
                                                            );
                                                            // 清除错误
                                                            error_update.update(
                                                                cx,
                                                                |msg, cx| {
                                                                    *msg = None;
                                                                    cx.notify();
                                                                },
                                                            );
                                                            // 每秒递减倒计时
                                                            let cd = countdown_update.clone();
                                                            cx.spawn(
                                                                async move |cx: &mut AsyncApp| {
                                                                    for _ in
                                                                        0..COUNTDOWN_SECONDS
                                                                    {
                                                                        cx.background_spawn(
                                                                            async {
                                                                                smol::Timer::after(std::time::Duration::from_secs(1)).await;
                                                                            },
                                                                        )
                                                                        .await;
                                                                        let should_stop =
                                                                            cx.update(|cx| {
                                                                                let mut stop =
                                                                                    false;
                                                                                cd.update(
                                                                                    cx,
                                                                                    |c, cx| {
                                                                                        if *c
                                                                                            > 0
                                                                                        {
                                                                                            *c -=
                                                                                                1;
                                                                                        }
                                                                                        if *c
                                                                                            == 0
                                                                                        {
                                                                                            stop = true;
                                                                                        }
                                                                                        cx.notify();
                                                                                    },
                                                                                );
                                                                                stop
                                                                            });
                                                                        if should_stop {
                                                                            break;
                                                                        }
                                                                    }
                                                                },
                                                            )
                                                            .detach();
                                                        }
                                                        Err(e) => {
                                                            error_update.update(
                                                                cx,
                                                                |msg, cx| {
                                                                    *msg = Some(e);
                                                                    cx.notify();
                                                                },
                                                            );
                                                        }
                                                    }
                                                });
                                            })
                                            .detach();
                                        })
                                }),
                            ),
                    )
                    // 已发送提示
                    .when(has_sent, |this| {
                        this.child(
                            gpui::div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("Auth.otp_sent_hint").to_string()),
                        )
                    })
                    // 错误信息
                    .when_some(error_render.read(cx).clone(), |this, msg| {
                        this.child(
                            gpui::div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(msg),
                        )
                    }),
            )
    });
}
