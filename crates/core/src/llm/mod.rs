pub mod chat_history;
pub mod connector;
pub mod manager;
pub mod onet_cli_provider;
pub mod storage;
pub mod types;

pub use connector::{ChatStream, LlmConnector, LlmProvider};
pub use manager::{GlobalProviderState, ProviderManager};
pub use onet_cli_provider::OnetCliLLMProvider;
pub use types::{ProviderConfig, ProviderType};

pub use llm_connector::types::{ChatRequest, Message, MessageBlock, Role, StreamingResponse};

use gpui::App;

pub fn init(cx: &mut App) {
    storage::init(cx);
    let state = GlobalProviderState::new();
    cx.set_global(state);
}
