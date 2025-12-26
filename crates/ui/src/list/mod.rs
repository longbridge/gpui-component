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

/// Configuration options for List.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListConfig {
    /// Whether to use active highlight style on ListItem, default
    pub active_highlight: bool,
}

impl Default for ListConfig {
    fn default() -> Self {
        Self {
            active_highlight: true,
        }
    }
}
