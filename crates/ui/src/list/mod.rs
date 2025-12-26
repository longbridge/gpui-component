pub(crate) mod cache;
mod delegate;
mod list;
mod list_item;
mod loading;
mod separator_item;

pub use delegate::*;
pub use list::*;
pub use list_item::*;
use schemars::JsonSchema;
pub use separator_item::*;
use serde::{Deserialize, Serialize};

/// Settings for List.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSettings {
    /// Whether to use active highlight style on ListItem, default
    pub active_highlight: bool,
}

impl Default for ListSettings {
    fn default() -> Self {
        Self {
            active_highlight: true,
        }
    }
}
