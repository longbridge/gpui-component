use gpui::App;

pub mod agent;
pub mod ai_chat;
pub mod cloud_sync;
pub mod config;
pub mod connection_notifier;
pub mod crypto;
pub mod gpui_tokio;
pub mod key_storage;
pub mod license;
pub mod llm;
pub mod popup_window;
pub mod storage;
pub mod tab_container;
pub mod tab_persistence;
pub mod themes;
pub mod utils;

pub use crate::agent::{Agent, AgentContext, AgentDescriptor, AgentDispatcher, AgentEvent, AgentResult, AgentRegistry, SessionAffinity};
pub use crate::ai_chat::{AiChatColors, AiChatPanel, AiChatPanelEvent, ChatMessageUI, ChatMessageUIGeneric, ChatRole, CodeBlockAction, CodeBlockActionBuilder, CodeBlockActionCallback, CodeBlockActionRegistry, LanguageMatcher, MessageExtension, MessageVariant, NoExtension, ProviderItem};
pub use crate::ai_chat::{ChatEngine, ChatMessageRenderer, CoreStreamEvent, StreamError, ChatStreamProcessor};

pub fn init(cx: &mut App) {
    gpui_tokio::init(cx);
    themes::init(cx);
    storage::init(cx);
    llm::init(cx);
    agent::init(cx);
    connection_notifier::init(cx);
}
