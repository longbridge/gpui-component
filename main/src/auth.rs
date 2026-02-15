//! 用户认证模块
//!
//! 提供 Supabase 认证集成，包括登录、登出、会话持久化等功能。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gpui::http_client::HttpClient;
use gpui::prelude::FluentBuilder;
use gpui::{px, App, AppContext as _, AsyncApp, Context, Entity, FontWeight, InteractiveElement, ParentElement, Styled, Window};

/// 全局静态会话过期标志
///
/// 用于在 SupabaseClient 的回调中（无法访问 GPUI 全局状态时）通知 UI 层会话已过期。
/// UI 层定期检查此标志，若为 true 则弹出登录对话框。
static SESSION_EXPIRED: AtomicBool = AtomicBool::new(false);
use gpui_component::{
    input::{Input, InputState},
    v_flex, ActiveTheme, WindowExt,
};
use one_core::cloud_sync::{
    supabase::{SessionExpiredCallback, SupabaseClient, SupabaseConfig},
    CloudApiClient, UserInfo,
};
use rust_i18n::t;
use gpui_component::dialog::DialogButtonProps;
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
    SESSION_EXPIRED.swap(false, Ordering::SeqCst)
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
        let auth_data = load_auth_data()?;
        let (access_token, refresh_token, user_id, expires_at) = auth_data;

        // 检查令牌是否已过期（提前 60 秒刷新）
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let needs_refresh = expires_at <= now + 60;

        if needs_refresh {
            // 令牌已过期或即将过期，必须刷新
            match self.client.refresh_token(&refresh_token).await {
                Ok(auth_resp) => {
                    self.client.set_auth(
                        auth_resp.access_token.clone(),
                        auth_resp.refresh_token.clone(),
                        auth_resp.user_id.clone(),
                    );
                    save_auth_data(
                        &auth_resp.access_token,
                        &auth_resp.refresh_token,
                        &auth_resp.user_id,
                        auth_resp.expires_at,
                    );
                }
                Err(_) => {
                    // 刷新失败，清除本地数据
                    clear_auth_data();
                    return None;
                }
            }
        } else {
            // 令牌未过期，直接使用
            self.client.set_auth(access_token, refresh_token.clone(), user_id);

            // 尝试在后台刷新令牌（可选，提升体验）
            if let Ok(auth_resp) = self.client.refresh_token(&refresh_token).await {
                save_auth_data(
                    &auth_resp.access_token,
                    &auth_resp.refresh_token,
                    &auth_resp.user_id,
                    auth_resp.expires_at,
                );
            }
        }

        // 获取用户信息
        match self.client.get_current_user().await {
            Ok(Some(user)) => Some(user),
            Ok(None) => {
                clear_auth_data();
                None
            }
            Err(_) => {
                // 获取用户信息失败，清除本地数据
                clear_auth_data();
                None
            }
        }
    }

    /// 登出
    pub async fn sign_out(&self) {
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
            let _ = std::fs::write(path, json);
        }
    }
}

/// 从本地加载认证数据
pub fn load_auth_data() -> Option<(String, String, String, i64)> {
    let path = get_auth_file_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;

    let access_token = data.get("access_token")?.as_str()?.to_string();
    let refresh_token = data.get("refresh_token")?.as_str()?.to_string();
    let user_id = data.get("user_id")?.as_str()?.to_string();
    // 兼容旧数据：如果没有 expires_at，默认为 0（会触发刷新）
    let expires_at = data.get("expires_at").and_then(|v| v.as_i64()).unwrap_or(0);

    Some((access_token, refresh_token, user_id, expires_at))
}

/// 清除本地认证数据
pub fn clear_auth_data() {
    if let Some(path) = get_auth_file_path() {
        let _ = std::fs::remove_file(path);
    }
}

// ============================================================================
// 登录/注册对话框
// ============================================================================

/// OTP 登录步骤
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OtpStep {
    /// 输入邮箱
    EnterEmail,
    /// 输入验证码
    EnterCode,
}

/// 显示 OTP 认证对话框
///
/// 使用 OTP（一次性验证码）方式登录：
/// 1. 用户输入邮箱
/// 2. 点击发送验证码
/// 3. 输入收到的验证码
/// 4. 验证成功后登录
pub fn show_auth_dialog<V: 'static>(
    window: &mut Window,
    cx: &mut Context<V>,
    view: Entity<V>,
    on_submit: impl Fn(&mut V, String, String, &mut Context<V>) + 'static,
) {
    let email_input = cx.new(|cx| {
        InputState::new(window, cx).placeholder(t!("Auth.email_placeholder"))
    });
    let otp_input = cx.new(|cx| {
        InputState::new(window, cx).placeholder(t!("Auth.otp_placeholder"))
    });
    let error_message = cx.new(|_| Option::<String>::None);
    let step_state = cx.new(|_| OtpStep::EnterEmail);
    let sending_state = cx.new(|_| false);

    let email_for_ok = email_input.clone();
    let otp_for_ok = otp_input.clone();
    let error_for_ok = error_message.clone();
    let step_for_ok = step_state.clone();

    let email_for_render = email_input.clone();
    let otp_for_render = otp_input.clone();
    let error_for_render = error_message.clone();
    let step_for_render = step_state.clone();
    let sending_for_render = sending_state.clone();

    let on_submit = std::rc::Rc::new(on_submit);
    let on_submit_clone = on_submit.clone();

    window.open_dialog(cx, move |dialog, _window, cx| {
        let email_ok = email_for_ok.clone();
        let otp_ok = otp_for_ok.clone();
        let error_ok = error_for_ok.clone();
        let step_ok = step_for_ok.clone();

        let email_render = email_for_render.clone();
        let otp_render = otp_for_render.clone();
        let error_render = error_for_render.clone();
        let step_render = step_for_render.clone();
        let sending_render = sending_for_render.clone();

        let on_submit_ok = on_submit_clone.clone();
        let view_clone = view.clone();

        let current_step = *step_render.read(cx);

        let title = match current_step {
            OtpStep::EnterEmail => t!("Auth.login").to_string(),
            OtpStep::EnterCode => t!("Auth.verify_otp").to_string(),
        };

        let ok_button_text = match current_step {
            OtpStep::EnterEmail => t!("Auth.send_otp").to_string(),
            OtpStep::EnterCode => t!("Auth.login").to_string(),
        };

        let email_for_send = email_render.clone();
        let error_for_send = error_render.clone();
        let sending_for_send = sending_render.clone();

        dialog
            .title(title)
            .width(px(400.))
            .confirm()
            .button_props(DialogButtonProps::default().ok_text(ok_button_text))
            .on_ok(move |_, _window, cx| {
                let current_step = *step_ok.read(cx);
                let email = email_ok.read(cx).text().to_string();

                match current_step {
                    OtpStep::EnterEmail => {
                        if email.is_empty() {
                            error_ok.update(cx, |msg, cx| {
                                *msg = Some(t!("Auth.email_required").to_string());
                                cx.notify();
                            });
                            return false;
                        }

                        // 发送 OTP 验证码
                        let auth = get_auth_service(cx);
                        let email_clone = email.clone();
                        let step_update = step_ok.clone();
                        let error_update = error_ok.clone();

                        cx.spawn(async move |cx: &mut AsyncApp| {
                            match auth.send_otp(&email_clone).await {
                                Ok(()) => {
                                    cx.update(|cx| {
                                        step_update.update(cx, |step, cx| {
                                            *step = OtpStep::EnterCode;
                                            cx.notify();
                                        });
                                    }).ok();
                                }
                                Err(e) => {
                                    cx.update(|cx| {
                                        error_update.update(cx, |msg, cx| {
                                            *msg = Some(e);
                                            cx.notify();
                                        });
                                    }).ok();
                                }
                            }
                        })
                        .detach();

                        // 保持对话框打开
                        false
                    }
                    OtpStep::EnterCode => {
                        let otp = otp_ok.read(cx).text().to_string();

                        if otp.is_empty() {
                            error_ok.update(cx, |msg, cx| {
                                *msg = Some(t!("Auth.otp_required").to_string());
                                cx.notify();
                            });
                            return false;
                        }

                        // 触发验证回调
                        view_clone.update(cx, |this, cx| {
                            on_submit_ok(this, email.clone(), otp.clone(), cx);
                        });
                        true
                    }
                }
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
                            .child(
                                Input::new(&email_render)
                                    .when(current_step == OtpStep::EnterCode, |input| {
                                        input.disabled(true)
                                    }),
                            ),
                    )
                    // OTP 输入（仅在第二步显示）
                    .when(current_step == OtpStep::EnterCode, |this| {
                        this.child(
                            v_flex()
                                .gap_1()
                                .child(
                                    gpui::div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(t!("Auth.otp_code").to_string()),
                                )
                                .child(Input::new(&otp_render)),
                        )
                            .child(
                                gpui::div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("Auth.otp_sent_hint").to_string()),
                            )
                            // 重新发送验证码链接
                            .child({
                                let is_sending = *sending_render.read(cx);
                                gpui::div()
                                    .id("resend-otp")
                                    .text_sm()
                                    .text_color(cx.theme().link)
                                    .cursor_pointer()
                                    .when(is_sending, |this| {
                                        this.text_color(cx.theme().muted_foreground)
                                            .cursor_default()
                                    })
                                    .child(if is_sending {
                                        t!("Auth.sending_otp").to_string()
                                    } else {
                                        t!("Auth.resend_otp").to_string()
                                    })
                                    .when(!is_sending, |this| {
                                        let email_send = email_for_send.clone();
                                        let error_send = error_for_send.clone();
                                        let sending_send = sending_for_send.clone();
                                        this.on_mouse_down(
                                            gpui::MouseButton::Left,
                                            move |_, _window, cx| {
                                                let email = email_send.read(cx).text().to_string();
                                                if email.is_empty() {
                                                    return;
                                                }

                                                sending_send.update(cx, |s, cx| {
                                                    *s = true;
                                                    cx.notify();
                                                });

                                                let auth = get_auth_service(cx);
                                                let error_update = error_send.clone();
                                                let sending_update = sending_send.clone();

                                                cx.spawn(async move |cx: &mut AsyncApp| {
                                                    let result = auth.send_otp(&email).await;
                                                    cx.update(|cx| {
                                                        sending_update.update(cx, |s, cx| {
                                                            *s = false;
                                                            cx.notify();
                                                        });
                                                        if let Err(e) = result {
                                                            error_update.update(cx, |msg, cx| {
                                                                *msg = Some(e);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }).ok();
                                                })
                                                    .detach();
                                            },
                                        )
                                    })
                            })
                    })
                    // 错误信息
                    .when_some(error_render.read(cx).clone(), |this, msg| {
                        this.child(
                            gpui::div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(msg),
                        )
                    })
                    // 返回输入邮箱链接（仅在第二步显示）
                    .when(current_step == OtpStep::EnterCode, |this| {
                        let step_back = step_render.clone();
                        let error_back = error_render.clone();
                        this.child(
                            gpui::div()
                                .id("back-to-email")
                                .text_sm()
                                .text_color(cx.theme().link)
                                .cursor_pointer()
                                .child(t!("Auth.change_email").to_string())
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    move |_, _window, cx| {
                                        step_back.update(cx, |step, cx| {
                                            *step = OtpStep::EnterEmail;
                                            cx.notify();
                                        });
                                        error_back.update(cx, |msg, cx| {
                                            *msg = None;
                                            cx.notify();
                                        });
                                    },
                                ),
                        )
                    }),
            )
    });
}
