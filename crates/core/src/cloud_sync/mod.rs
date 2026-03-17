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
mod connection_sync;
pub mod engine;
mod generic_sync;
mod models;
pub mod queue;
mod service;
pub mod state_manager;
pub mod supabase;
pub mod sync_type;
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
pub use sync_type::*;

use crate::storage::{GlobalStorageState, StoredConnection, TeamKeyCacheRepository};

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

// ============================================================================
// 团队选项（供 UI 下拉使用）
// ============================================================================

/// 团队选择项
#[derive(Debug, Clone)]
pub struct TeamOption {
    pub id: String,
    pub name: String,
}

/// 获取可用团队列表（从本地 team_key_cache 缓存读取）
pub fn get_cached_team_options(cx: &App) -> Vec<TeamOption> {
    let Some(storage) = cx.try_global::<GlobalStorageState>() else {
        return Vec::new();
    };
    let Some(repo) = storage.storage.get::<TeamKeyCacheRepository>() else {
        return Vec::new();
    };
    match repo.list() {
        Ok(caches) => caches
            .into_iter()
            .map(|c| TeamOption {
                id: c.team_id,
                name: c.team_name,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

// ============================================================================
// 权限判断
// ============================================================================

/// 判断当前用户是否可编辑指定连接
pub fn can_edit_connection(conn: &StoredConnection, cx: &App) -> bool {
    let Some(team_id) = &conn.team_id else {
        return true; // 个人连接，始终可编辑
    };

    let Some(user) = GlobalCloudUser::get_user(cx) else {
        return false; // 未登录，不可编辑团队连接
    };

    // 创建者可编辑
    if conn.owner_id.as_deref() == Some(&user.id) {
        return true;
    }

    // 团队 owner 可编辑所有
    if let Some(storage) = cx.try_global::<GlobalStorageState>() {
        if let Some(repo) = storage.storage.get::<TeamKeyCacheRepository>() {
            if let Ok(Some(cache)) = repo.get(team_id) {
                return cache.role.as_deref() == Some("owner");
            }
        }
    }

    false
}
