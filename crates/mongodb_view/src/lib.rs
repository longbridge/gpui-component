//! MongoDB 视图层
//!
//! 提供 MongoDB 的核心功能和用户界面组件，包括：
//! - 连接管理和全局状态
//! - 树形视图和集合文档浏览
//! - 连接表单

use gpui::App;

rust_i18n::i18n!("locales", fallback = "en");

pub mod collection_view;
pub mod connection;
pub mod manager;
pub mod mongo_form_window;
pub mod mongo_tab;
pub mod mongo_tree_event;
pub mod mongo_tree_view;
pub mod sidebar;
pub mod types;

pub use collection_view::CollectionView;
pub use connection::{MongoConnection, MongoConnectionImpl};
pub use manager::{GlobalMongoState, MongoManager};
pub use mongo_form_window::{MongoFormWindow, MongoFormWindowConfig};
pub use mongo_tab::MongoTabView;
pub use mongo_tree_view::{MongoTreeView, MongoTreeViewEvent};
pub use sidebar::{MongoSidebar, MongoSidebarEvent};
pub use types::*;

/// 初始化 MongoDB 模块
pub fn init(cx: &mut App) {
    cx.set_global(GlobalMongoState::new());
}
