//! Redis 视图层
//!
//! 提供 Redis 的核心功能和用户界面组件，包括：
//! - 连接管理和全局状态
//! - 树形视图和键值编辑器
//! - 连接表单
//! - CLI 终端视图

use gpui::App;

rust_i18n::i18n!("locales", fallback = "zh-CN");

// 核心模块
pub mod connection;
pub mod manager;
pub mod types;

// 视图模块
pub(crate) mod create_key_dialog;
pub mod key_value_view;
pub mod redis_cli_element;
pub mod redis_cli_view;
pub mod redis_form_window;
pub mod redis_tab;
mod redis_tree_event;
pub mod redis_tree_view;
pub mod sidebar;

// 核心导出
pub use connection::{RedisConnection, RedisConnectionImpl};
pub use manager::{GlobalRedisState, RedisManager};
pub use types::*;

// 视图导出
pub use key_value_view::{KeyValueView, KeyValueViewEvent};
pub use redis_cli_view::{RedisCliView, RedisCliViewEvent};
pub use redis_form_window::{RedisFormWindow, RedisFormWindowConfig};
pub use redis_tab::RedisTabView;
pub use redis_tree_view::{RedisTreeView, RedisTreeViewEvent};
pub use sidebar::{RedisSidebar, RedisSidebarEvent};

/// 初始化 Redis 模块
///
/// 注册全局状态和其他必要的初始化操作
pub fn init(cx: &mut App) {
    cx.set_global(GlobalRedisState::new());
    redis_cli_view::init(cx);
}
