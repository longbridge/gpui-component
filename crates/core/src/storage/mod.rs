pub mod connection;
pub mod demo_database;
pub mod manager;
pub mod migration;
pub mod models;
pub mod quick_command;
pub mod repository;
pub mod row_mapping;
pub mod traits;

use gpui::App;
pub use manager::*;
pub use models::*;
pub use quick_command::*;
pub use repository::*;

pub fn init(cx: &mut App) {
    cx.set_global(ActiveConnections::new());
    manager::init(cx);
    repository::init(cx);

    // 首次启动时创建演示数据库
    let storage = cx.global::<GlobalStorageState>().storage.clone();
    if let Some(conn_repo) = storage.get::<ConnectionRepository>() {
        demo_database::try_init_demo(&conn_repo);
    }
}
