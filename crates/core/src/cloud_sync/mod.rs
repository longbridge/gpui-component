//! 云同步模块
//!
//! 提供连接配置的云端同步功能，支持端到端加密。
//!
//! ## 主要功能
//!
//! - 连接配置的加密上传和解密下载
//! - 主密钥管理（设置、修改、验证）
//! - 同步状态追踪
//! - 冲突检测和解决
//!
//! ## 使用流程
//!
//! 1. 用户登录云端账户
//! 2. 首次使用时设置主密钥，生成 key_verification 上传云端
//! 3. 非首次使用时输入主密钥，从云端获取 key_verification 验证
//! 4. 验证通过后解锁同步服务
//! 5. 执行同步操作（上传/下载/删除）

pub mod client;
pub mod conflict;
pub mod engine;
mod models;
pub mod queue;
mod service;
pub mod state_manager;
pub mod supabase;
mod connection_sync;
mod workspace_sync;

use std::sync::{Arc, RwLock};

use gpui::{App, Global};

pub use client::*;
pub use conflict::*;
pub use engine::*;
pub use models::*;
pub use queue::*;
pub use service::*;
pub use state_manager::*;

// ============================================================================
// 全局用户状态
// ============================================================================

/// 全局当前用户状态（供跨 crate 访问登录态）
#[derive(Clone, Default)]
pub struct GlobalCloudUser {
    user: Arc<RwLock<Option<UserInfo>>>,
}

impl Global for GlobalCloudUser {}

impl GlobalCloudUser {
    /// 获取当前用户
    pub fn get_user(cx: &App) -> Option<UserInfo> {
        if let Some(state) = cx.try_global::<GlobalCloudUser>() {
            state.user.read().ok().and_then(|u| u.clone())
        } else {
            None
        }
    }

    /// 是否已登录
    pub fn is_logged_in(cx: &App) -> bool {
        Self::get_user(cx).is_some()
    }

    /// 设置当前用户
    pub fn set_user(user: Option<UserInfo>, cx: &mut App) {
        if !cx.has_global::<GlobalCloudUser>() {
            cx.set_global(GlobalCloudUser::default());
        }
        if let Some(state) = cx.try_global::<GlobalCloudUser>() {
            if let Ok(mut guard) = state.user.write() {
                *guard = user;
            }
        }
    }
}
