//! 云端 API 客户端抽象层
//!
//! 定义云端 API 的通用接口，支持多种后端实现（如 Supabase）。

use crate::cloud_sync::models::*;
use crate::license::SubscriptionInfo;
use crate::llm::ChatStream;
use async_trait::async_trait;
use llm_connector::ChatRequest;
use std::fmt;

/// 云端 API 错误类型
#[derive(Debug, Clone)]
pub enum CloudApiError {
    /// 未登录
    NotAuthenticated,
    /// 认证失败
    AuthenticationFailed(String),
    /// 需要邮箱确认（注册成功但需验证邮箱）
    EmailConfirmationRequired(String),
    /// 网络错误
    NetworkError(String),
    /// 服务端错误
    ServerError(String),
    /// 数据解析错误
    ParseError(String),
    /// 资源未找到
    NotFound(String),
    /// 冲突
    Conflict(String),
    /// 未知错误
    Unknown(String),
}

impl fmt::Display for CloudApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloudApiError::NotAuthenticated => write!(f, "未登录"),
            CloudApiError::AuthenticationFailed(msg) => write!(f, "认证失败: {}", msg),
            CloudApiError::EmailConfirmationRequired(msg) => write!(f, "需要邮箱确认: {}", msg),
            CloudApiError::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            CloudApiError::ServerError(msg) => write!(f, "服务端错误: {}", msg),
            CloudApiError::ParseError(msg) => write!(f, "数据解析错误: {}", msg),
            CloudApiError::NotFound(msg) => write!(f, "资源未找到: {}", msg),
            CloudApiError::Conflict(msg) => write!(f, "冲突: {}", msg),
            CloudApiError::Unknown(msg) => write!(f, "未知错误: {}", msg),
        }
    }
}

impl CloudApiError {
    /// 判断是否为认证相关错误（应触发清除认证状态）
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            CloudApiError::NotAuthenticated | CloudApiError::AuthenticationFailed(_)
        )
    }
}

impl std::error::Error for CloudApiError {}

/// 云端 API 客户端 trait
///
/// 定义与云端服务交互的通用接口。
#[async_trait]
pub trait CloudApiClient: Send + Sync {
    // ========================================================================
    // 认证相关
    // ========================================================================

    /// 使用邮箱密码登录
    async fn sign_in_with_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, CloudApiError>;

    /// 使用 OAuth 登录（如 GitHub、Google）
    async fn sign_in_with_oauth(
        &self,
        provider: &str,
        redirect_url: &str,
    ) -> Result<OAuthResponse, CloudApiError>;

    /// 注册新用户
    async fn sign_up(&self, email: &str, password: &str) -> Result<AuthResponse, CloudApiError>;

    /// 登出
    async fn sign_out(&self) -> Result<(), CloudApiError>;

    /// 获取当前登录用户信息
    async fn get_current_user(&self) -> Result<Option<UserInfo>, CloudApiError>;

    /// 刷新访问令牌
    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthResponse, CloudApiError>;

    /// 发送 OTP 验证码到邮箱
    async fn sign_in_with_otp(&self, email: &str) -> Result<(), CloudApiError>;

    /// 验证 OTP 验证码并登录
    async fn verify_otp(&self, email: &str, token: &str) -> Result<AuthResponse, CloudApiError>;

    // ========================================================================
    // 用户配置相关（密钥验证数据）
    // ========================================================================

    /// 获取用户的加密配置
    async fn get_user_config(&self) -> Result<Option<CloudUserConfig>, CloudApiError>;

    /// 保存用户的加密配置
    async fn save_user_config(&self, config: &CloudUserConfig) -> Result<(), CloudApiError>;

    // ========================================================================
    // 订阅相关
    // ========================================================================

    /// 获取用户订阅信息
    ///
    /// 返回用户当前的订阅计划和状态，用于 License 功能控制。
    async fn get_subscription(&self) -> Result<Option<SubscriptionInfo>, CloudApiError>;

    // ========================================================================
    // OnetCli 模型列表
    // ========================================================================

    /// 获取当前可用模型列表
    async fn list_models(&self) -> Result<Vec<String>, CloudApiError>;

    // ========================================================================
    // 统一同步数据（sync_data 表）
    // ========================================================================

    /// 获取同步数据列表
    ///
    /// 可选过滤：data_type, team_id, since_timestamp
    async fn list_sync_data(
        &self,
        data_type: Option<&str>,
        team_id: Option<&str>,
        since: Option<i64>,
    ) -> Result<Vec<CloudSyncData>, CloudApiError>;

    /// 创建同步数据
    async fn create_sync_data(&self, data: &CloudSyncData) -> Result<CloudSyncData, CloudApiError>;

    /// 更新同步数据（乐观并发控制）
    async fn update_sync_data(&self, data: &CloudSyncData) -> Result<CloudSyncData, CloudApiError>;

    /// 软删除同步数据
    async fn delete_sync_data(&self, id: &str) -> Result<(), CloudApiError>;

    // ========================================================================
    // 团队管理
    // ========================================================================

    /// 获取当前用户所在的所有团队
    async fn list_teams(&self) -> Result<Vec<Team>, CloudApiError>;

    /// 创建团队
    async fn create_team(&self, team: &Team) -> Result<Team, CloudApiError>;

    /// 更新团队信息
    async fn update_team(&self, team: &Team) -> Result<Team, CloudApiError>;

    /// 删除团队
    async fn delete_team(&self, id: &str) -> Result<(), CloudApiError>;

    /// 获取团队成员列表
    async fn list_team_members(&self, team_id: &str) -> Result<Vec<TeamMember>, CloudApiError>;

    /// 添加团队成员
    async fn add_team_member(&self, member: &TeamMember) -> Result<TeamMember, CloudApiError>;

    /// 通过邮箱添加团队成员
    ///
    /// 使用服务端 RPC 函数根据邮箱查找用户并添加为团队成员。
    /// 解决了客户端无法直接访问 auth.users 表的问题。
    async fn add_team_member_by_email(
        &self,
        team_id: &str,
        email: &str,
    ) -> Result<TeamMember, CloudApiError>;

    /// 移除团队成员
    async fn remove_team_member(&self, member_id: &str) -> Result<(), CloudApiError>;

    // ========================================================================
    // AI 聊天
    // ========================================================================

    /// 聊天
    async fn chat(&self, request: &ChatRequest) -> Result<String, CloudApiError>;

    /// 聊天流
    async fn chat_stream(&self, request: &ChatRequest) -> Result<ChatStream, CloudApiError>;
}

/// 认证响应
#[derive(Debug, Clone)]
pub struct AuthResponse {
    /// 用户 ID
    pub user_id: String,
    /// 用户邮箱
    pub email: String,
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 令牌过期时间（Unix 时间戳）
    pub expires_at: i64,
}

/// OAuth 响应
#[derive(Debug, Clone)]
pub struct OAuthResponse {
    /// 授权 URL
    pub auth_url: String,
}

/// 用户信息
#[derive(Debug, Clone)]
pub struct UserInfo {
    /// 用户 ID
    pub id: String,
    /// 用户邮箱
    pub email: String,
    /// 用户名（可选）
    pub username: Option<String>,
    /// 头像 URL（可选）
    pub avatar_url: Option<String>,
    /// 创建时间
    pub created_at: i64,
}
