pub mod connection;
pub mod manager;
pub mod migration;
pub mod models;
pub mod quick_command;
pub mod repository;
pub mod row_mapping;
pub mod terminal_history;
pub mod traits;

use gpui::App;
pub use manager::*;
pub use models::*;
pub use quick_command::*;
pub use repository::*;
pub use terminal_history::*;

pub fn init(cx: &mut App) {
    cx.set_global(ActiveConnections::new());
    manager::init(cx);
    repository::init(cx);
}
