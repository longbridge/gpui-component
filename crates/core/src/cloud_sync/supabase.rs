//! Supabase 云端 API 客户端实现
//!
//! 使用 Supabase 作为云端后端服务。
//! 支持 401 响应自动刷新 token 和请求重试。

use crate::cloud_sync::client::*;
use crate::cloud_sync::models::*;
use crate::license::SubscriptionInfo;
use crate::llm::ChatStream;
use anyhow::anyhow;
use async_trait::async_trait;
use futures::AsyncReadExt;
use futures_util::StreamExt;
use futures_util::stream;
use gpui::http_client::{AsyncBody, HttpClient, Method, Request, Response, StatusCode};
use llm_connector::{ChatRequest, StreamingResponse};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::{Mutex as AsyncMutex, Notify};
use tracing::{debug, error, info, warn};

/// Supabase 客户端配置
#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    /// 项目 URL（如 https://xxx.supabase.co）
    pub project_url: String,
    /// API Key（anon key）
    pub api_key: String,
}

/// Supabase 认证状态
#[derive(Debug, Clone)]
struct AuthState {
    /// 访问令牌
    access_token: Option<String>,
    /// 刷新令牌
    refresh_token: Option<String>,
    /// 用户 ID
    user_id: Option<String>,
    /// 令牌过期时间（Unix 时间戳）
    expires_at: i64,
}

/// 会话过期事件回调类型
pub type SessionExpiredCallback = Arc<dyn Fn() + Send + Sync>;
/// 自动刷新成功回调类型
pub type TokenRefreshedCallback = Arc<dyn Fn(AuthResponse) + Send + Sync>;

/// Token 刷新状态
struct RefreshState {
    /// 是否正在刷新
    refreshing: AtomicBool,
    /// 刷新锁（确保同时只有一个刷新操作）
    refresh_lock: AsyncMutex<()>,
    /// 刷新完成通知
    refresh_notify: Notify,
}

impl Default for RefreshState {
    fn default() -> Self {
        Self {
            refreshing: AtomicBool::new(false),
            refresh_lock: AsyncMutex::new(()),
            refresh_notify: Notify::new(),
        }
    }
}

/// Supabase 客户端
pub struct SupabaseClient {
    config: SupabaseConfig,
    http: Arc<dyn HttpClient>,
    auth_state: RwLock<AuthState>,
    refresh_state: RefreshState,
    /// 会话过期回调
    on_session_expired: RwLock<Option<SessionExpiredCallback>>,
    /// 自动刷新成功回调（用于持久化最新 token）
    on_token_refreshed: RwLock<Option<TokenRefreshedCallback>>,
}

impl SupabaseClient {
    /// 创建新的 Supabase 客户端
    pub fn new(config: SupabaseConfig, http: Arc<dyn HttpClient>) -> Self {
        Self {
            config,
            http,
            auth_state: RwLock::new(AuthState {
                access_token: None,
                refresh_token: None,
                user_id: None,
                expires_at: 0,
            }),
            refresh_state: RefreshState::default(),
            on_session_expired: RwLock::new(None),
            on_token_refreshed: RwLock::new(None),
        }
    }

    /// 设置会话过期回调
    pub fn set_session_expired_callback(&self, callback: SessionExpiredCallback) {
        if let Ok(mut cb) = self.on_session_expired.write() {
            *cb = Some(callback);
        }
    }

    /// 设置自动刷新成功回调
    pub fn set_token_refreshed_callback(&self, callback: TokenRefreshedCallback) {
        if let Ok(mut cb) = self.on_token_refreshed.write() {
            *cb = Some(callback);
        }
    }

    /// 触发会话过期回调
    fn notify_session_expired(&self) {
        warn!("[supabase] notifying session expired to UI layer");
        if let Ok(cb) = self.on_session_expired.read() {
            if let Some(callback) = cb.as_ref() {
                callback();
            } else {
                warn!("[supabase] no session expired callback registered");
            }
        }
    }

    /// 触发自动刷新成功回调
    fn notify_token_refreshed(&self, auth: &AuthResponse) {
        if let Ok(cb) = self.on_token_refreshed.read() {
            if let Some(callback) = cb.as_ref() {
                callback(auth.clone());
            }
        }
    }

    /// 设置认证令牌（用于恢复会话）
    pub fn set_auth(&self, access_token: String, refresh_token: String, user_id: String) {
        self.set_auth_with_expiry(access_token, refresh_token, user_id, 0);
    }

    /// 设置认证令牌（包含过期时间）
    pub fn set_auth_with_expiry(
        &self,
        access_token: String,
        refresh_token: String,
        user_id: String,
        expires_at: i64,
    ) {
        if let Ok(mut state) = self.auth_state.write() {
            let token_prefix = if access_token.len() > 20 {
                &access_token[..20]
            } else {
                &access_token
            };
            info!(
                "[supabase] set_auth: user_id={} expires_at={} token_prefix={}...",
                user_id, expires_at, token_prefix
            );
            state.access_token = Some(access_token);
            state.refresh_token = Some(refresh_token);
            state.user_id = Some(user_id);
            state.expires_at = expires_at;
        } else {
            error!("[supabase] set_auth failed: could not acquire auth_state write lock");
        }
    }

    /// 清除认证状态
    pub fn clear_auth(&self) {
        warn!("[supabase] clear_auth called, clearing all auth state");
        if let Ok(mut state) = self.auth_state.write() {
            let user_id = state.user_id.clone().unwrap_or_default();
            info!("[supabase] clearing auth for user_id={}", user_id);
            state.access_token = None;
            state.refresh_token = None;
            state.user_id = None;
            state.expires_at = 0;
        }
    }

    /// 获取当前访问令牌
    fn get_access_token(&self) -> Option<String> {
        self.auth_state
            .read()
            .ok()
            .and_then(|state| state.access_token.clone())
    }

    /// 获取当前刷新令牌
    fn get_refresh_token(&self) -> Option<String> {
        self.auth_state
            .read()
            .ok()
            .and_then(|state| state.refresh_token.clone())
    }

    /// 获取当前用户 ID
    fn get_user_id(&self) -> Option<String> {
        self.auth_state
            .read()
            .ok()
            .and_then(|state| state.user_id.clone())
    }

    /// 构建 Auth API URL
    fn auth_url(&self, path: &str) -> String {
        format!("{}/auth/v1{}", self.config.project_url, path)
    }

    /// 构建 REST API URL
    fn rest_url(&self, table: &str) -> String {
        format!("{}/rest/v1/{}", self.config.project_url, table)
    }

    fn functions_url(&self, function: &str) -> String {
        format!("{}/functions/v1/{}", self.config.project_url, function)
    }

    /// 获取通用请求头
    fn common_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("apikey", self.config.api_key.clone()),
            ("Content-Type", "application/json".to_string()),
        ]
    }

    /// 获取认证请求头
    fn auth_headers(&self) -> Result<Vec<(&'static str, String)>, CloudApiError> {
        let token = self.get_access_token().ok_or_else(|| {
            warn!("[supabase] auth_headers: no access token available");
            CloudApiError::NotAuthenticated
        })?;

        let mut headers = self.common_headers();
        headers.push(("Authorization", format!("Bearer {}", token)));
        Ok(headers)
    }

    /// 构建 HTTP 请求
    fn build_request(
        &self,
        method: Method,
        url: &str,
        headers: Vec<(&'static str, String)>,
        body: Option<Vec<u8>>,
    ) -> Result<Request<AsyncBody>, CloudApiError> {
        let mut builder = Request::builder().method(method).uri(url);

        for (key, value) in headers {
            builder = builder.header(key, value);
        }

        let body = body.map(AsyncBody::from).unwrap_or_else(AsyncBody::empty);

        builder
            .body(body)
            .map_err(|e| CloudApiError::NetworkError(e.to_string()))
    }

    /// 发送请求并读取响应体
    async fn send_request(
        &self,
        request: Request<AsyncBody>,
    ) -> Result<(StatusCode, Vec<u8>), CloudApiError> {
        let method = request.method().clone();
        let uri = request.uri().to_string();
        let start = Instant::now();

        debug!("[supabase] request start: {} {}", method, uri);

        let response = self.http.send(request).await.map_err(|e| {
            error!("[supabase] request failed: {} {} - {}", method, uri, e);
            CloudApiError::NetworkError(e.to_string())
        })?;

        let status = response.status();
        let mut body = response.into_body();
        let mut bytes = Vec::new();
        body.read_to_end(&mut bytes).await.map_err(|e| {
            error!(
                "[supabase] response read failed: {} {} (status {}) - {}",
                method, uri, status, e
            );
            CloudApiError::NetworkError(e.to_string())
        })?;

        debug!(
            "[supabase] request done: {} {} -> {} ({} ms)",
            method,
            uri,
            status,
            start.elapsed().as_millis()
        );

        Ok((status, bytes))
    }

    // ========================================================================
    // Token 刷新拦截器
    // ========================================================================

    /// 内部刷新 token（不通过拦截器，直接调用 API）
    async fn refresh_token_internal(&self) -> Result<AuthResponse, CloudApiError> {
        let refresh_token = self
            .get_refresh_token()
            .ok_or(CloudApiError::NotAuthenticated)?;

        #[derive(Serialize)]
        struct RefreshRequest<'a> {
            refresh_token: &'a str,
        }

        let url = self.auth_url("/token?grant_type=refresh_token");
        debug!("[supabase] refresh token: POST {}", url);
        let body = RefreshRequest {
            refresh_token: &refresh_token,
        };
        let headers = self.common_headers();

        let (status, result) = self
            .post_json::<SupabaseAuthResponse, _>(&url, headers, &body)
            .await?;

        if status.is_success() {
            let auth = result.map_err(|e| CloudApiError::ParseError(e))?;

            // 更新本地认证状态
            let expires_at = auth.expires_at.unwrap_or_else(|| {
                auth.expires_in
                    .map(|e| chrono::Utc::now().timestamp() + e)
                    .unwrap_or(0)
            });

            self.set_auth_with_expiry(
                auth.access_token.clone(),
                auth.refresh_token.clone(),
                auth.user.id.clone(),
                expires_at,
            );

            info!(
                "[supabase] refresh token success: user_id={} expires_at={} expires_in={:?}",
                auth.user.id, expires_at, auth.expires_in
            );
            let auth_response = AuthResponse {
                user_id: auth.user.id,
                email: auth.user.email.unwrap_or_default(),
                access_token: auth.access_token,
                refresh_token: auth.refresh_token,
                expires_at,
            };
            self.notify_token_refreshed(&auth_response);
            Ok(auth_response)
        } else {
            let error_body = match &result {
                Ok(_) => "parsed ok but status failed".to_string(),
                Err(e) => e.clone(),
            };
            error!(
                "[supabase] refresh token failed: status={} body={}",
                status, error_body
            );
            Err(CloudApiError::AuthenticationFailed(
                "令牌刷新失败".to_string(),
            ))
        }
    }

    /// 检查 token 是否即将过期（提前 60 秒刷新）
    fn is_token_expiring_soon(&self) -> bool {
        if let Ok(state) = self.auth_state.read() {
            if state.expires_at == 0 {
                // expires_at 为 0 表示未设置，不做主动刷新
                return false;
            }
            let now = chrono::Utc::now().timestamp();
            let expiring = state.expires_at <= now + 60;
            if expiring {
                warn!(
                    "[supabase] token expiring soon: now={} expires_at={} ({}s remaining)",
                    now,
                    state.expires_at,
                    state.expires_at - now
                );
            }
            expiring
        } else {
            false
        }
    }

    /// 确保 token 有效，必要时刷新
    ///
    /// 如果正在刷新，等待刷新完成；否则执行刷新。
    /// 使用锁确保同时只有一个刷新操作。
    async fn ensure_token_valid(&self) -> Result<(), CloudApiError> {
        // 如果正在刷新，等待刷新完成
        if self.refresh_state.refreshing.load(Ordering::SeqCst) {
            debug!("[supabase] token refresh in progress, waiting");
            self.refresh_state.refresh_notify.notified().await;
            // 刷新完成后检查是否成功
            return if self.get_access_token().is_some() {
                Ok(())
            } else {
                warn!("[supabase] token refresh waited but no access token after refresh");
                Err(CloudApiError::NotAuthenticated)
            };
        }

        // 获取刷新锁
        let _lock = self.refresh_state.refresh_lock.lock().await;

        // 再次检查（可能在等待锁期间已被其他请求刷新）
        if self.refresh_state.refreshing.load(Ordering::SeqCst) {
            drop(_lock);
            debug!("[supabase] token refresh in progress after lock, waiting");
            self.refresh_state.refresh_notify.notified().await;
            return if self.get_access_token().is_some() {
                Ok(())
            } else {
                warn!("[supabase] token refresh waited (after lock) but no access token");
                Err(CloudApiError::NotAuthenticated)
            };
        }

        // 设置刷新中标志
        self.refresh_state.refreshing.store(true, Ordering::SeqCst);
        info!("[supabase] token refresh starting");

        // 执行刷新
        let result = self.refresh_token_internal().await;

        // 清除刷新中标志
        self.refresh_state.refreshing.store(false, Ordering::SeqCst);

        // 通知所有等待的请求
        self.refresh_state.refresh_notify.notify_waiters();

        match result {
            Ok(auth_resp) => {
                info!(
                    "[supabase] token refresh succeeded: user_id={} new_expires_at={}",
                    auth_resp.user_id, auth_resp.expires_at
                );
                Ok(())
            }
            Err(e) => {
                // 刷新失败，清除认证状态并通知 UI
                error!("[supabase] token refresh failed, clearing auth: {}", e);
                self.clear_auth();
                self.notify_session_expired();
                Err(e)
            }
        }
    }

    /// 发送 POST 请求并返回 JSON 响应
    async fn post_json<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: &str,
        headers: Vec<(&'static str, String)>,
        body: &B,
    ) -> Result<(StatusCode, Result<T, String>), CloudApiError> {
        let body_bytes =
            serde_json::to_vec(body).map_err(|e| CloudApiError::ParseError(e.to_string()))?;

        let request = self.build_request(Method::POST, url, headers, Some(body_bytes))?;
        let (status, response_bytes) = self.send_request(request).await?;

        let result = serde_json::from_slice(&response_bytes)
            .map_err(|_| String::from_utf8_lossy(&response_bytes).to_string());

        Ok((status, result))
    }

    /// 发送 GET 请求并返回 JSON 响应
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        headers: Vec<(&'static str, String)>,
    ) -> Result<(StatusCode, Result<T, String>), CloudApiError> {
        let request = self.build_request(Method::GET, url, headers, None)?;
        let (status, response_bytes) = self.send_request(request).await?;

        let result = serde_json::from_slice(&response_bytes)
            .map_err(|_| String::from_utf8_lossy(&response_bytes).to_string());

        Ok((status, result))
    }

    /// 发送 PATCH 请求并返回 JSON 响应
    async fn patch_json<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: &str,
        headers: Vec<(&'static str, String)>,
        body: &B,
    ) -> Result<(StatusCode, Result<T, String>), CloudApiError> {
        let body_bytes =
            serde_json::to_vec(body).map_err(|e| CloudApiError::ParseError(e.to_string()))?;

        let request = self.build_request(Method::PATCH, url, headers, Some(body_bytes))?;
        let (status, response_bytes) = self.send_request(request).await?;

        let result = serde_json::from_slice(&response_bytes)
            .map_err(|_| String::from_utf8_lossy(&response_bytes).to_string());

        Ok((status, result))
    }

    /// 发送 DELETE 请求
    async fn delete_request(
        &self,
        url: &str,
        headers: Vec<(&'static str, String)>,
    ) -> Result<StatusCode, CloudApiError> {
        let request = self.build_request(Method::DELETE, url, headers, None)?;
        let (status, _) = self.send_request(request).await?;
        Ok(status)
    }

    // ========================================================================
    // 带 401 自动重试的请求方法
    // ========================================================================

    /// 发送带 401 自动重试的 GET 请求
    async fn get_json_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<(StatusCode, Result<T, String>), CloudApiError> {
        // 主动检查 token 是否即将过期
        if self.is_token_expiring_soon() {
            info!("[supabase] proactive token refresh before GET {}", url);
            self.ensure_token_valid().await?;
        }

        let headers = self.auth_headers()?;
        let (status, result) = self.get_json::<T>(url, headers).await?;

        if status == StatusCode::UNAUTHORIZED {
            warn!("[supabase] 401 received, refreshing token: GET {}", url);
            // 尝试刷新 token
            self.ensure_token_valid().await?;
            // 重试请求
            let retry_headers = self.auth_headers()?;
            self.get_json::<T>(url, retry_headers).await
        } else {
            Ok((status, result))
        }
    }

    /// 发送带 401 自动重试的 POST 请求
    async fn post_json_with_retry<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: &str,
        extra_headers: Vec<(&'static str, String)>,
        body: &B,
    ) -> Result<(StatusCode, Result<T, String>), CloudApiError> {
        // 主动检查 token 是否即将过期
        if self.is_token_expiring_soon() {
            info!("[supabase] proactive token refresh before POST {}", url);
            self.ensure_token_valid().await?;
        }

        let mut headers = self.auth_headers()?;
        headers.extend(extra_headers.clone());
        let (status, result) = self.post_json::<T, B>(url, headers, body).await?;

        if status == StatusCode::UNAUTHORIZED {
            warn!("[supabase] 401 received, refreshing token: POST {}", url);
            // 尝试刷新 token
            self.ensure_token_valid().await?;
            // 重试请求
            let mut retry_headers = self.auth_headers()?;
            retry_headers.extend(extra_headers);
            self.post_json::<T, B>(url, retry_headers, body).await
        } else {
            Ok((status, result))
        }
    }

    /// 发送流式 POST 请求（带 401 自动重试）
    async fn post_stream_with_retry<B: serde::Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<Response<AsyncBody>, CloudApiError> {
        // 确保 token 有效
        self.ensure_token_valid().await?;

        let body_bytes =
            serde_json::to_vec(body).map_err(|e| CloudApiError::ParseError(e.to_string()))?;

        let mut headers = self.auth_headers()?;
        let mut builder = Request::builder().method(Method::POST).uri(url);
        for (key, value) in &headers {
            builder = builder.header(*key, value);
        }
        builder = builder.header("Accept", "text/event-stream");

        let request = builder
            .body(AsyncBody::from(body_bytes.clone()))
            .map_err(|e| CloudApiError::NetworkError(e.to_string()))?;

        let response = self
            .http
            .send(request)
            .await
            .map_err(|e| CloudApiError::NetworkError(format!("请求失败: {}", e)))?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            warn!(
                "[supabase] 401 received, refreshing token: POST (stream) {}",
                url
            );
            self.ensure_token_valid().await?;
            headers = self.auth_headers()?;
            let mut retry_builder = Request::builder().method(Method::POST).uri(url);
            for (key, value) in &headers {
                retry_builder = retry_builder.header(*key, value);
            }
            retry_builder = retry_builder.header("Accept", "text/event-stream");

            let retry_request = retry_builder
                .body(AsyncBody::from(body_bytes))
                .map_err(|e| CloudApiError::NetworkError(e.to_string()))?;

            let retry_response = self
                .http
                .send(retry_request)
                .await
                .map_err(|e| CloudApiError::NetworkError(format!("请求失败: {}", e)))?;

            let retry_status = retry_response.status();
            if retry_status.is_success() {
                return Ok(retry_response);
            }

            let mut retry_body = retry_response.into_body();
            let mut retry_bytes = Vec::new();
            retry_body
                .read_to_end(&mut retry_bytes)
                .await
                .map_err(|e| CloudApiError::NetworkError(format!("读取错误响应失败: {}", e)))?;
            let retry_error_text = String::from_utf8_lossy(&retry_bytes);
            return Err(CloudApiError::AuthenticationFailed(format!(
                "API 请求失败 ({}): {}",
                retry_status, retry_error_text
            )));
        }

        if !status.is_success() {
            let mut body = response.into_body();
            let mut bytes = Vec::new();
            body.read_to_end(&mut bytes)
                .await
                .map_err(|e| CloudApiError::NetworkError(format!("读取错误响应失败: {}", e)))?;
            let error_text = String::from_utf8_lossy(&bytes);
            return Err(CloudApiError::ServerError(format!(
                "API 请求失败 ({}): {}",
                status, error_text
            )));
        }

        Ok(response)
    }
    /// 发送带 401 自动重试的 PATCH 请求
    async fn patch_json_with_retry<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: &str,
        extra_headers: Vec<(&'static str, String)>,
        body: &B,
    ) -> Result<(StatusCode, Result<T, String>), CloudApiError> {
        // 主动检查 token 是否即将过期
        if self.is_token_expiring_soon() {
            info!("[supabase] proactive token refresh before PATCH {}", url);
            self.ensure_token_valid().await?;
        }

        let mut headers = self.auth_headers()?;
        headers.extend(extra_headers.clone());
        let (status, result) = self.patch_json::<T, B>(url, headers, body).await?;

        if status == StatusCode::UNAUTHORIZED {
            warn!("[supabase] 401 received, refreshing token: PATCH {}", url);
            // 尝试刷新 token
            self.ensure_token_valid().await?;
            // 重试请求
            let mut retry_headers = self.auth_headers()?;
            retry_headers.extend(extra_headers);
            self.patch_json::<T, B>(url, retry_headers, body).await
        } else {
            Ok((status, result))
        }
    }

    /// 发送带 401 自动重试的 DELETE 请求
    #[allow(dead_code)]
    async fn delete_with_retry(&self, url: &str) -> Result<StatusCode, CloudApiError> {
        // 主动检查 token 是否即将过期
        if self.is_token_expiring_soon() {
            info!("[supabase] proactive token refresh before DELETE {}", url);
            self.ensure_token_valid().await?;
        }

        let headers = self.auth_headers()?;
        let status = self.delete_request(url, headers).await?;

        if status == StatusCode::UNAUTHORIZED {
            warn!("[supabase] 401 received, refreshing token: DELETE {}", url);
            // 尝试刷新 token
            self.ensure_token_valid().await?;
            // 重试请求
            let retry_headers = self.auth_headers()?;
            self.delete_request(url, retry_headers).await
        } else {
            Ok(status)
        }
    }

    /// URL 编码
    fn url_encode(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
                _ => {
                    for b in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        result
    }
}

// ============================================================================
// Supabase Auth API 响应结构
// ============================================================================

#[derive(Debug, Deserialize)]
struct SupabaseAuthResponse {
    access_token: String,
    refresh_token: String,
    expires_at: Option<i64>,
    expires_in: Option<i64>,
    user: SupabaseUser,
}

/// 注册响应（邮箱确认开启时 token 可能为空）
#[derive(Debug, Deserialize)]
struct SupabaseSignUpResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<i64>,
    expires_in: Option<i64>,
    user: SupabaseUser,
}

#[derive(Debug, Deserialize)]
struct SupabaseUser {
    id: String,
    email: Option<String>,
    user_metadata: Option<serde_json::Value>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SupabaseError {
    error: Option<String>,
    error_description: Option<String>,
    message: Option<String>,
}

// ============================================================================
// 数据库表结构
// ============================================================================

/// 用户配置表记录
#[derive(Debug, Serialize, Deserialize)]
struct UserConfigRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    user_id: String,
    key_verification: String,
    key_version: i32,
    updated_at: Option<String>,
}

/// 用户订阅表记录
#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    user_id: String,
    plan: String,
    status: String,
    expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

impl From<SubscriptionRow> for SubscriptionInfo {
    fn from(row: SubscriptionRow) -> Self {
        SubscriptionInfo {
            plan: row.plan,
            status: row.status,
            expires_at: row
                .expires_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.timestamp()),
        }
    }
}

/// 连接表记录
#[derive(Debug, Serialize, Deserialize)]
struct ConnectionRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    user_id: String,
    local_id: Option<i64>,
    name: String,
    connection_type: String,
    workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_databases: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remark: Option<String>,
    encrypted_params: String,
    key_version: i32,
    checksum: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    /// 软删除时间戳（ISO 8601 格式）
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<String>,
}

impl From<ConnectionRow> for CloudConnection {
    fn from(row: ConnectionRow) -> Self {
        CloudConnection {
            id: row.id.unwrap_or_default(),
            local_id: row.local_id,
            name: row.name,
            connection_type: row.connection_type,
            workspace_id: row.workspace_id,
            selected_databases: row.selected_databases,
            remark: row.remark,
            encrypted_params: row.encrypted_params,
            key_version: row.key_version as u32,
            updated_at: row
                .updated_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0),
            checksum: row.checksum,
            deleted_at: row
                .deleted_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.timestamp_millis()),
        }
    }
}

impl From<&CloudConnection> for ConnectionRow {
    fn from(conn: &CloudConnection) -> Self {
        ConnectionRow {
            id: if conn.id.is_empty() {
                None
            } else {
                Some(conn.id.clone())
            },
            user_id: String::new(), // 会在插入时设置
            local_id: conn.local_id,
            name: conn.name.clone(),
            connection_type: conn.connection_type.clone(),
            workspace_id: conn.workspace_id.clone(),
            selected_databases: conn.selected_databases.clone(),
            remark: conn.remark.clone(),
            encrypted_params: conn.encrypted_params.clone(),
            key_version: conn.key_version as i32,
            checksum: conn.checksum.clone(),
            updated_at: None,
            created_at: None,
            deleted_at: conn.deleted_at.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
            }),
        }
    }
}

/// 工作空间表记录
#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    user_id: String,
    local_id: Option<i64>,
    name: String,
    color: Option<String>,
    icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    /// 软删除时间戳（ISO 8601 格式）
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<String>,
}

impl From<WorkspaceRow> for CloudWorkspace {
    fn from(row: WorkspaceRow) -> Self {
        CloudWorkspace {
            id: row.id.unwrap_or_default(),
            local_id: row.local_id,
            name: row.name,
            color: row.color,
            icon: row.icon,
            updated_at: row
                .updated_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0),
            deleted_at: row
                .deleted_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.timestamp_millis()),
        }
    }
}

impl From<&CloudWorkspace> for WorkspaceRow {
    fn from(ws: &CloudWorkspace) -> Self {
        WorkspaceRow {
            id: if ws.id.is_empty() {
                None
            } else {
                Some(ws.id.clone())
            },
            user_id: String::new(), // 会在插入时设置
            local_id: ws.local_id,
            name: ws.name.clone(),
            color: ws.color.clone(),
            icon: ws.icon.clone(),
            updated_at: None,
            created_at: None,
            deleted_at: ws.deleted_at.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
            }),
        }
    }
}

/// 模型列表记录
#[derive(Debug, Serialize, Deserialize)]
struct ModelListRow {
    model: String,
    #[serde(default)]
    enabled: Option<bool>,
}

impl From<ModelListRow> for String {
    fn from(row: ModelListRow) -> Self {
        row.model
    }
}

#[async_trait]
impl CloudApiClient for SupabaseClient {
    // ========================================================================
    // 认证相关
    // ========================================================================

    async fn sign_in_with_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, CloudApiError> {
        #[derive(Serialize)]
        struct SignInRequest<'a> {
            email: &'a str,
            password: &'a str,
        }

        let url = self.auth_url("/token?grant_type=password");
        let body = SignInRequest { email, password };
        let headers = self.common_headers();

        let (status, result) = self
            .post_json::<SupabaseAuthResponse, _>(&url, headers, &body)
            .await?;

        if status.is_success() {
            let auth = result.map_err(|e| CloudApiError::ParseError(e))?;

            // 保存认证状态
            let expires_at = auth.expires_at.unwrap_or_else(|| {
                auth.expires_in
                    .map(|e| chrono::Utc::now().timestamp() + e)
                    .unwrap_or(0)
            });

            self.set_auth_with_expiry(
                auth.access_token.clone(),
                auth.refresh_token.clone(),
                auth.user.id.clone(),
                expires_at,
            );

            info!(
                "[supabase] sign_in_with_password success: user_id={} expires_at={}",
                auth.user.id, expires_at
            );

            Ok(AuthResponse {
                user_id: auth.user.id,
                email: auth.user.email.unwrap_or_default(),
                access_token: auth.access_token,
                refresh_token: auth.refresh_token,
                expires_at,
            })
        } else {
            let error_msg = match result {
                Err(e) => {
                    // 尝试解析错误响应
                    serde_json::from_str::<SupabaseError>(&e)
                        .ok()
                        .and_then(|err| err.error_description.or(err.message).or(err.error))
                        .unwrap_or_else(|| "认证失败".to_string())
                }
                Ok(_) => "认证失败".to_string(),
            };
            Err(CloudApiError::AuthenticationFailed(error_msg))
        }
    }

    async fn sign_in_with_oauth(
        &self,
        provider: &str,
        redirect_url: &str,
    ) -> Result<OAuthResponse, CloudApiError> {
        // 构建 OAuth 授权 URL
        let auth_url = format!(
            "{}/auth/v1/authorize?provider={}&redirect_to={}",
            self.config.project_url,
            provider,
            Self::url_encode(redirect_url)
        );

        Ok(OAuthResponse { auth_url })
    }

    async fn sign_up(&self, email: &str, password: &str) -> Result<AuthResponse, CloudApiError> {
        #[derive(Serialize)]
        struct SignUpRequest<'a> {
            email: &'a str,
            password: &'a str,
        }

        let url = self.auth_url("/signup");
        let body = SignUpRequest { email, password };
        let headers = self.common_headers();

        let (status, result) = self
            .post_json::<SupabaseSignUpResponse, _>(&url, headers, &body)
            .await?;

        if status.is_success() {
            let auth = result.map_err(|e| CloudApiError::ParseError(e))?;

            // 检查是否有 token（邮箱确认关闭时会直接返回 token）
            match (auth.access_token, auth.refresh_token) {
                (Some(access_token), Some(refresh_token)) => {
                    // 有 token，直接登录成功
                    let expires_at = auth.expires_at.unwrap_or_else(|| {
                        auth.expires_in
                            .map(|e| chrono::Utc::now().timestamp() + e)
                            .unwrap_or(0)
                    });

                    self.set_auth_with_expiry(
                        access_token.clone(),
                        refresh_token.clone(),
                        auth.user.id.clone(),
                        expires_at,
                    );

                    Ok(AuthResponse {
                        user_id: auth.user.id,
                        email: auth.user.email.unwrap_or_default(),
                        access_token,
                        refresh_token,
                        expires_at,
                    })
                }
                _ => {
                    // 没有 token，需要邮箱确认
                    Err(CloudApiError::EmailConfirmationRequired(
                        auth.user.email.unwrap_or_else(|| email.to_string()),
                    ))
                }
            }
        } else {
            let error_msg = match result {
                Err(e) => {
                    // 尝试解析错误响应
                    serde_json::from_str::<SupabaseError>(&e)
                        .ok()
                        .and_then(|err| err.error_description.or(err.message).or(err.error))
                        .unwrap_or_else(|| "注册失败".to_string())
                }
                Ok(_) => "注册失败".to_string(),
            };
            Err(CloudApiError::AuthenticationFailed(error_msg))
        }
    }

    async fn sign_out(&self) -> Result<(), CloudApiError> {
        let headers = self.auth_headers()?;
        let url = self.auth_url("/logout");

        let request = self.build_request(Method::POST, &url, headers, None)?;
        let _ = self.send_request(request).await?;

        self.clear_auth();
        Ok(())
    }

    async fn get_current_user(&self) -> Result<Option<UserInfo>, CloudApiError> {
        let url = self.auth_url("/user");
        debug!("[supabase] get_current_user: GET {}", url);

        // 主动检查 token 是否即将过期
        if self.is_token_expiring_soon() {
            info!("[supabase] proactive token refresh before get_current_user");
            self.ensure_token_valid().await?;
        }

        // 使用带重试的 GET 请求
        let result = async {
            let headers = match self.auth_headers() {
                Ok(h) => h,
                Err(_) => {
                    debug!("[supabase] get_current_user skipped: not authenticated");
                    return Ok(None);
                }
            };
            let (status, result) = self.get_json::<SupabaseUser>(&url, headers).await?;

            if status == StatusCode::UNAUTHORIZED {
                warn!("[supabase] get_current_user 401, refreshing token");
                // 尝试刷新 token 并重试
                self.ensure_token_valid().await?;
                let retry_headers = self.auth_headers()?;
                let (retry_status, retry_result) =
                    self.get_json::<SupabaseUser>(&url, retry_headers).await?;
                Ok::<_, CloudApiError>(Some((retry_status, retry_result)))
            } else {
                Ok(Some((status, result)))
            }
        }
        .await?;

        let Some((status, result)) = result else {
            return Ok(None);
        };

        if status.is_success() {
            let user = result.map_err(|e| CloudApiError::ParseError(e))?;

            let created_at = user
                .created_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);

            Ok(Some(UserInfo {
                id: user.id,
                email: user.email.unwrap_or_default(),
                username: user
                    .user_metadata
                    .as_ref()
                    .and_then(|m| m.get("username"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                avatar_url: user
                    .user_metadata
                    .as_ref()
                    .and_then(|m| m.get("avatar_url"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                created_at,
            }))
        } else if status == StatusCode::UNAUTHORIZED {
            // 重试后仍然 401
            self.clear_auth();
            self.notify_session_expired();
            Ok(None)
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取用户信息失败: {}",
                status
            )))
        }
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthResponse, CloudApiError> {
        #[derive(Serialize)]
        struct RefreshRequest<'a> {
            refresh_token: &'a str,
        }

        let url = self.auth_url("/token?grant_type=refresh_token");
        info!("[supabase] refresh_token (trait): POST {}", url);
        let body = RefreshRequest { refresh_token };
        let headers = self.common_headers();

        let (status, result) = self
            .post_json::<SupabaseAuthResponse, _>(&url, headers, &body)
            .await?;

        if status.is_success() {
            let auth = result.map_err(|e| CloudApiError::ParseError(e))?;

            let expires_at = auth.expires_at.unwrap_or_else(|| {
                auth.expires_in
                    .map(|e| chrono::Utc::now().timestamp() + e)
                    .unwrap_or(0)
            });

            // 使用 set_auth_with_expiry 确保内存中 expires_at 正确
            self.set_auth_with_expiry(
                auth.access_token.clone(),
                auth.refresh_token.clone(),
                auth.user.id.clone(),
                expires_at,
            );

            info!(
                "[supabase] refresh_token (trait) success: user_id={} expires_at={} expires_in={:?}",
                auth.user.id, expires_at, auth.expires_in
            );

            Ok(AuthResponse {
                user_id: auth.user.id,
                email: auth.user.email.unwrap_or_default(),
                access_token: auth.access_token,
                refresh_token: auth.refresh_token,
                expires_at,
            })
        } else {
            let error_body = match &result {
                Ok(_) => "parsed ok but status failed".to_string(),
                Err(e) => e.clone(),
            };
            error!(
                "[supabase] refresh_token (trait) failed: status={} body={}",
                status, error_body
            );
            Err(CloudApiError::AuthenticationFailed(
                "令牌刷新失败".to_string(),
            ))
        }
    }

    async fn sign_in_with_otp(&self, email: &str) -> Result<(), CloudApiError> {
        #[derive(Serialize)]
        struct OtpRequest<'a> {
            email: &'a str,
        }

        let url = self.auth_url("/otp");
        let body = OtpRequest { email };
        let headers = self.common_headers();

        let (status, result) = self
            .post_json::<serde_json::Value, _>(&url, headers, &body)
            .await?;

        if status.is_success() {
            Ok(())
        } else {
            let error_msg = match result {
                Err(e) => serde_json::from_str::<SupabaseError>(&e)
                    .ok()
                    .and_then(|err| err.error_description.or(err.message).or(err.error))
                    .unwrap_or_else(|| "发送验证码失败".to_string()),
                Ok(_) => "发送验证码失败".to_string(),
            };
            Err(CloudApiError::AuthenticationFailed(error_msg))
        }
    }

    async fn verify_otp(&self, email: &str, token: &str) -> Result<AuthResponse, CloudApiError> {
        #[derive(Serialize)]
        struct VerifyOtpRequest<'a> {
            email: &'a str,
            token: &'a str,
            #[serde(rename = "type")]
            otp_type: &'a str,
        }

        let url = self.auth_url("/verify");
        info!("[supabase] verify_otp: email={}", email);
        let body = VerifyOtpRequest {
            email,
            token,
            otp_type: "email",
        };
        let headers = self.common_headers();

        let (status, result) = self
            .post_json::<SupabaseAuthResponse, _>(&url, headers, &body)
            .await?;

        if status.is_success() {
            let auth = result.map_err(|e| CloudApiError::ParseError(e))?;

            let expires_at = auth.expires_at.unwrap_or_else(|| {
                auth.expires_in
                    .map(|e| chrono::Utc::now().timestamp() + e)
                    .unwrap_or(0)
            });

            self.set_auth_with_expiry(
                auth.access_token.clone(),
                auth.refresh_token.clone(),
                auth.user.id.clone(),
                expires_at,
            );

            info!(
                "[supabase] verify_otp success: user_id={} expires_at={}",
                auth.user.id, expires_at
            );

            Ok(AuthResponse {
                user_id: auth.user.id,
                email: auth.user.email.unwrap_or_default(),
                access_token: auth.access_token,
                refresh_token: auth.refresh_token,
                expires_at,
            })
        } else {
            let error_msg = match result {
                Err(e) => serde_json::from_str::<SupabaseError>(&e)
                    .ok()
                    .and_then(|err| err.error_description.or(err.message).or(err.error))
                    .unwrap_or_else(|| "验证码验证失败".to_string()),
                Ok(_) => "验证码验证失败".to_string(),
            };
            Err(CloudApiError::AuthenticationFailed(error_msg))
        }
    }

    // ========================================================================
    // 用户配置相关
    // ========================================================================

    async fn get_user_config(&self) -> Result<Option<CloudUserConfig>, CloudApiError> {
        let url = format!("{}?&select=*", self.rest_url("user_configs"),);

        let (status, result) = self.get_json_with_retry::<Vec<UserConfigRow>>(&url).await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;

            Ok(rows.into_iter().next().map(|row| CloudUserConfig {
                user_id: row.user_id,
                key_verification: row.key_verification,
                key_version: row.key_version as u32,
                updated_at: row
                    .updated_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0),
            }))
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取用户配置失败: {}",
                status
            )))
        }
    }

    async fn save_user_config(&self, config: &CloudUserConfig) -> Result<(), CloudApiError> {
        let user_id = self.get_user_id().ok_or(CloudApiError::NotAuthenticated)?;

        let row = UserConfigRow {
            id: None,
            user_id: user_id.clone(),
            key_verification: config.key_verification.clone(),
            key_version: config.key_version as i32,
            updated_at: None,
        };

        // 使用 upsert（插入或更新）
        // PostgREST upsert 需要指定冲突列和 Prefer 头
        let url = format!("{}?on_conflict=user_id", self.rest_url("user_configs"));
        let extra_headers = vec![("Prefer", "resolution=merge-duplicates".to_string())];

        let (status, _) = self
            .post_json_with_retry::<serde_json::Value, _>(&url, extra_headers, &row)
            .await?;

        if status.is_success() || status == StatusCode::CREATED {
            Ok(())
        } else {
            Err(CloudApiError::ServerError("保存用户配置失败".to_string()))
        }
    }

    // ========================================================================
    // 订阅相关
    // ========================================================================

    async fn get_subscription(&self) -> Result<Option<SubscriptionInfo>, CloudApiError> {
        let url = format!("{}?&select=*", self.rest_url("user_subscriptions"));

        let (status, result) = self
            .get_json_with_retry::<Vec<SubscriptionRow>>(&url)
            .await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;
            Ok(rows.into_iter().next().map(SubscriptionInfo::from))
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取订阅信息失败: {}",
                status
            )))
        }
    }

    // ========================================================================
    // 连接数据同步
    // ========================================================================

    async fn list_models(&self) -> Result<Vec<String>, CloudApiError> {
        let url = format!(
            "{}?select=model,enabled&enabled=eq.true&order=created_at.desc",
            self.rest_url("model_list"),
        );

        let (status, result) = self.get_json_with_retry::<Vec<ModelListRow>>(&url).await?;

        if status.is_success() {
            let rows = result.map_err(CloudApiError::ParseError)?;
            Ok(rows.into_iter().map(String::from).collect())
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取模型列表失败: {}",
                status
            )))
        }
    }

    async fn list_connections(&self) -> Result<Vec<CloudConnection>, CloudApiError> {
        let url = format!(
            "{}?select=*&order=updated_at.desc",
            self.rest_url("connections")
        );

        let (status, result) = self.get_json_with_retry::<Vec<ConnectionRow>>(&url).await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;
            Ok(rows.into_iter().map(CloudConnection::from).collect())
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取连接列表失败: {}",
                status
            )))
        }
    }

    async fn get_connection(&self, id: &str) -> Result<Option<CloudConnection>, CloudApiError> {
        let url = format!("{}?id=eq.{}&select=*", self.rest_url("connections"), id);

        let (status, result) = self.get_json_with_retry::<Vec<ConnectionRow>>(&url).await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;
            Ok(rows.into_iter().next().map(CloudConnection::from))
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取连接失败: {}",
                status
            )))
        }
    }

    async fn create_connection(
        &self,
        connection: &CloudConnection,
    ) -> Result<CloudConnection, CloudApiError> {
        let user_id = self.get_user_id().ok_or(CloudApiError::NotAuthenticated)?;

        let mut row = ConnectionRow::from(connection);
        row.user_id = user_id;
        row.id = Some(uuid::Uuid::new_v4().to_string());

        let url = self.rest_url("connections");
        let extra_headers = vec![("Prefer", "return=representation".to_string())];

        let (status, result) = self
            .post_json_with_retry::<Vec<ConnectionRow>, _>(&url, extra_headers, &row)
            .await?;

        if status.is_success() || status == StatusCode::CREATED {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;

            rows.into_iter()
                .next()
                .map(CloudConnection::from)
                .ok_or_else(|| CloudApiError::ParseError("创建连接后未返回数据".to_string()))
        } else {
            // 尝试提取服务端返回的详细错误信息
            let error_detail = match result {
                Err(e) => e,
                Ok(_) => format!("HTTP {}", status.as_u16()),
            };
            Err(CloudApiError::ServerError(format!(
                "创建连接失败: {}",
                error_detail
            )))
        }
    }

    async fn update_connection(
        &self,
        connection: &CloudConnection,
    ) -> Result<CloudConnection, CloudApiError> {
        let user_id = self.get_user_id().ok_or(CloudApiError::NotAuthenticated)?;

        let mut row = ConnectionRow::from(connection);
        row.user_id = user_id;

        let url = format!("{}?id=eq.{}", self.rest_url("connections"), connection.id);
        let extra_headers = vec![("Prefer", "return=representation".to_string())];

        let (status, result) = self
            .patch_json_with_retry::<Vec<ConnectionRow>, _>(&url, extra_headers, &row)
            .await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;

            rows.into_iter()
                .next()
                .map(CloudConnection::from)
                .ok_or_else(|| CloudApiError::NotFound("连接不存在".to_string()))
        } else {
            Err(CloudApiError::ServerError("更新连接失败".to_string()))
        }
    }

    async fn delete_connection(&self, id: &str) -> Result<(), CloudApiError> {
        // 使用软删除：设置 deleted_at 字段而非真正删除记录
        #[derive(Serialize)]
        struct SoftDeletePayload {
            deleted_at: String,
        }

        let now = chrono::Utc::now().to_rfc3339();
        let payload = SoftDeletePayload { deleted_at: now };

        let url = format!("{}?id=eq.{}", self.rest_url("connections"), id);
        let extra_headers = vec![("Prefer", "return=minimal".to_string())];

        let (status, _) = self
            .patch_json_with_retry::<serde_json::Value, _>(&url, extra_headers, &payload)
            .await?;

        if status.is_success() || status == StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(CloudApiError::ServerError("删除连接失败".to_string()))
        }
    }

    async fn batch_sync(&self, request: &SyncRequest) -> Result<SyncResponse, CloudApiError> {
        let mut response = SyncResponse {
            uploaded_ids: Vec::new(),
            downloaded: Vec::new(),
            deleted_ids: Vec::new(),
            errors: Vec::new(),
        };

        // 批量上传
        for conn in &request.uploads {
            match self.create_connection(conn).await {
                Ok(created) => {
                    response.uploaded_ids.push((conn.local_id, created.id));
                }
                Err(e) => {
                    response
                        .errors
                        .push(format!("上传失败 {}: {}", conn.name, e));
                }
            }
        }

        // 批量下载
        for id in &request.download_ids {
            match self.get_connection(id).await {
                Ok(Some(conn)) => {
                    response.downloaded.push(conn);
                }
                Ok(None) => {
                    response.errors.push(format!("连接不存在: {}", id));
                }
                Err(e) => {
                    response.errors.push(format!("下载失败 {}: {}", id, e));
                }
            }
        }

        // 批量删除
        for id in &request.delete_ids {
            match self.delete_connection(id).await {
                Ok(()) => {
                    response.deleted_ids.push(id.clone());
                }
                Err(e) => {
                    response.errors.push(format!("删除失败 {}: {}", id, e));
                }
            }
        }

        Ok(response)
    }

    // ========================================================================
    // 工作空间数据同步
    // ========================================================================

    async fn get_connections_since(
        &self,
        since_timestamp: i64,
    ) -> Result<Vec<CloudConnection>, CloudApiError> {
        // 将时间戳转换为 ISO 8601 格式
        let since_datetime = chrono::DateTime::from_timestamp_millis(since_timestamp)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();

        let url = format!(
            "{}?updated_at=gt.{}&select=*&order=updated_at.desc",
            self.rest_url("connections"),
            Self::url_encode(&since_datetime)
        );

        let (status, result) = self.get_json_with_retry::<Vec<ConnectionRow>>(&url).await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;
            Ok(rows.into_iter().map(CloudConnection::from).collect())
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取更新连接失败: {}",
                status
            )))
        }
    }

    async fn list_workspaces(&self) -> Result<Vec<CloudWorkspace>, CloudApiError> {
        let url = format!(
            "{}?select=*&order=updated_at.desc",
            self.rest_url("workspaces"),
        );

        let (status, result) = self.get_json_with_retry::<Vec<WorkspaceRow>>(&url).await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;
            Ok(rows.into_iter().map(CloudWorkspace::from).collect())
        } else {
            Err(CloudApiError::ServerError(format!(
                "获取工作空间列表失败: {}",
                status
            )))
        }
    }

    async fn create_workspace(
        &self,
        workspace: &CloudWorkspace,
    ) -> Result<CloudWorkspace, CloudApiError> {
        let user_id = self.get_user_id().ok_or(CloudApiError::NotAuthenticated)?;

        let mut row = WorkspaceRow::from(workspace);
        row.user_id = user_id;
        row.id = Some(uuid::Uuid::new_v4().to_string());

        let url = self.rest_url("workspaces");
        let extra_headers = vec![("Prefer", "return=representation".to_string())];

        let (status, result) = self
            .post_json_with_retry::<Vec<WorkspaceRow>, _>(&url, extra_headers, &row)
            .await?;

        if status.is_success() || status == StatusCode::CREATED {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;

            rows.into_iter()
                .next()
                .map(CloudWorkspace::from)
                .ok_or_else(|| CloudApiError::ParseError("创建工作空间后未返回数据".to_string()))
        } else {
            let error_detail = match result {
                Err(e) => e,
                Ok(_) => format!("HTTP {}", status.as_u16()),
            };
            Err(CloudApiError::ServerError(format!(
                "创建工作空间失败: {}",
                error_detail
            )))
        }
    }

    async fn update_workspace(
        &self,
        workspace: &CloudWorkspace,
    ) -> Result<CloudWorkspace, CloudApiError> {
        let user_id = self.get_user_id().ok_or(CloudApiError::NotAuthenticated)?;

        let mut row = WorkspaceRow::from(workspace);
        row.user_id = user_id;

        let url = format!("{}?id=eq.{}", self.rest_url("workspaces"), workspace.id);
        let extra_headers = vec![("Prefer", "return=representation".to_string())];

        let (status, result) = self
            .patch_json_with_retry::<Vec<WorkspaceRow>, _>(&url, extra_headers, &row)
            .await?;

        if status.is_success() {
            let rows = result.map_err(|e| CloudApiError::ParseError(e))?;

            rows.into_iter()
                .next()
                .map(CloudWorkspace::from)
                .ok_or_else(|| CloudApiError::NotFound("工作空间不存在".to_string()))
        } else {
            Err(CloudApiError::ServerError("更新工作空间失败".to_string()))
        }
    }

    async fn delete_workspace(&self, id: &str) -> Result<(), CloudApiError> {
        // 使用软删除：设置 deleted_at 字段而非真正删除记录
        #[derive(Serialize)]
        struct SoftDeletePayload {
            deleted_at: String,
        }

        let now = chrono::Utc::now().to_rfc3339();
        let payload = SoftDeletePayload { deleted_at: now };

        let url = format!("{}?id=eq.{}", self.rest_url("workspaces"), id);
        let extra_headers = vec![("Prefer", "return=minimal".to_string())];

        let (status, _) = self
            .patch_json_with_retry::<serde_json::Value, _>(&url, extra_headers, &payload)
            .await?;

        if status.is_success() || status == StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(CloudApiError::ServerError("删除工作空间失败".to_string()))
        }
    }

    async fn chat(&self, request: &ChatRequest) -> Result<String, CloudApiError> {
        let url = self.functions_url("ai-proxy");

        // 使用 serde_json::Value 手动解析，兼容 content 为字符串或数组两种格式
        let (status, response) = self
            .post_json_with_retry::<serde_json::Value, ChatRequest>(&url, vec![], request)
            .await?;

        let json_val = response.map_err(|e| CloudApiError::ParseError(e))?;

        if !status.is_success() {
            return Err(CloudApiError::ServerError("接口请求失败".to_string()));
        }

        // 从顶层 content 字段取值（部分代理会填充此字段）
        if let Some(content) = json_val.get("content").and_then(|v| v.as_str()) {
            if !content.is_empty() {
                return Ok(content.to_string());
            }
        }

        // 从 choices[0].message 中提取
        let choices = json_val.get("choices").and_then(|v| v.as_array());
        let Some(choices) = choices else {
            return Ok(String::new());
        };
        let Some(first_choice) = choices.first() else {
            return Ok(String::new());
        };
        let Some(message) = first_choice.get("message") else {
            return Ok(String::new());
        };

        // content 可能是字符串（OpenAI 标准格式）或数组（多模态格式）
        if let Some(content) = message.get("content") {
            if let Some(text) = content.as_str() {
                return Ok(text.to_string());
            }
            if let Some(arr) = content.as_array() {
                for block in arr {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            return Ok(text.to_string());
                        }
                    }
                }
            }
        }

        // 按优先级尝试 reasoning 字段
        for key in &["reasoning_content", "reasoning", "thought", "thinking"] {
            if let Some(val) = message.get(*key).and_then(|v| v.as_str()) {
                if !val.is_empty() {
                    return Ok(val.to_string());
                }
            }
        }

        Ok(String::new())
    }

    async fn chat_stream(&self, request: &ChatRequest) -> Result<ChatStream, CloudApiError> {
        // 设置 stream = true
        let mut stream_request = request.clone();
        stream_request.stream = Some(true);

        let url = self.functions_url("ai-proxy");
        let response = self.post_stream_with_retry(&url, &stream_request).await?;

        let body = response.into_body();

        // 创建 SSE 解析流
        let stream = stream::unfold((body, String::new()), |(mut body, mut buffer)| async move {
            let mut chunk = vec![0u8; 4096];
            match body.read(&mut chunk).await {
                Ok(0) => None, // EOF
                Ok(n) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk[..n]));

                    // 解析 SSE 事件
                    let mut results = Vec::new();
                    while let Some(pos) = buffer.find("\n\n") {
                        let event = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        // 解析 data: 前缀
                        for line in event.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }
                                // 直接解析为 StreamingResponse
                                if let Ok(mut response) =
                                    serde_json::from_str::<StreamingResponse>(data)
                                {
                                    if response.choices.is_empty() {
                                        continue;
                                    }
                                    // 按优先级获取内容：content > reasoning_content > reasoning > thought > thinking
                                    if response.content.is_empty() {
                                        if let Some(choice) = response.choices.first() {
                                            let content = choice
                                                .delta
                                                .content
                                                .as_ref()
                                                .filter(|s| !s.is_empty())
                                                .or(choice.delta.reasoning_content.as_ref())
                                                .or(choice.delta.reasoning.as_ref())
                                                .or(choice.delta.thought.as_ref())
                                                .or(choice.delta.thinking.as_ref())
                                                .cloned()
                                                .unwrap_or_default();
                                            response.content = content;
                                        }
                                    }
                                    results.push(Ok(response));
                                }
                            }
                        }
                    }

                    if results.is_empty() {
                        Some((stream::iter(vec![]), (body, buffer)))
                    } else {
                        Some((stream::iter(results), (body, buffer)))
                    }
                }
                Err(e) => Some((
                    stream::iter(vec![Err(anyhow!("读取流失败: {}", e))]),
                    (body, buffer),
                )),
            }
        })
        .flatten();

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_row_conversion() {
        let cloud_conn = CloudConnection {
            id: "test-id".to_string(),
            local_id: Some(123),
            name: "Test Connection".to_string(),
            connection_type: "Database".to_string(),
            workspace_id: Some("ws-1".to_string()),
            selected_databases: None,
            remark: None,
            encrypted_params: "ENC:xxx".to_string(),
            key_version: 1,
            updated_at: 1234567890,
            checksum: "abc123".to_string(),
            deleted_at: None,
        };

        let row = ConnectionRow::from(&cloud_conn);
        assert_eq!(row.id, Some("test-id".to_string()));
        assert_eq!(row.local_id, Some(123));
        assert_eq!(row.name, "Test Connection");

        let converted: CloudConnection = row.into();
        assert_eq!(converted.id, "test-id");
        assert_eq!(converted.local_id, Some(123));
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(SupabaseClient::url_encode("hello"), "hello");
        assert_eq!(SupabaseClient::url_encode("hello world"), "hello%20world");
        assert_eq!(SupabaseClient::url_encode("a+b=c"), "a%2Bb%3Dc");
    }
}
