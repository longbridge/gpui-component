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

pub use client::*;
pub use conflict::*;
pub use engine::*;
pub use models::*;
pub use queue::*;
pub use service::*;
pub use state_manager::*;
