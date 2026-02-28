//! AI Chat Panel - 通用 AI 助手对话面板
//!
//! 此模块提供可复用的 AI 聊天界面组件，可以在不同的应用中使用。
//!
//! # 模块结构
//!
//! - `types`: 共享类型定义（ChatMessageUI, ChatRole, MessageExtension 等）
//! - `engine`: 共享业务逻辑引擎（ChatEngine）
//! - `rendering`: 共享消息渲染工具（ChatMessageRenderer）
//! - `stream`: 流式处理器（ChatStreamProcessor, StreamEvent）
//! - `panel`: 默认的聊天面板实现
//! - `components`: 可复用的 UI 组件（Provider 选择器、模型设置、发送按钮等）
//! - `ask_ai`: AI 提问通知机制
//!
//! # 使用方式
//!
//! ## 直接使用默认面板
//!
//! ```rust,ignore
//! use one_core::ai_chat::AiChatPanel;
//!
//! let panel = cx.new(|cx| AiChatPanel::new(window, cx));
//! ```
//!
//! ## 使用 ChatEngine 构建自定义面板
//!
//! ```rust,ignore
//! use one_core::ai_chat::engine::ChatEngine;
//! use one_core::ai_chat::types::NoExtension;
//!
//! let engine = ChatEngine::<NoExtension>::new(storage_manager);
//! ```
//!
//! ## 使用可复用组件
//!
//! ```rust,ignore
//! use one_core::ai_chat::components::{
//!     ProviderSelectState, ModelSettings, SendButton,
//! };
//!
//! // Provider/Model 选择器
//! let provider_state = ProviderSelectState::new(window, cx, |event, cx| {
//!     // 处理选择事件
//! });
//!
//! // 模型设置面板
//! let settings = ModelSettings::default();
//! let panel = cx.new(|cx| ModelSettingsPanel::new(settings, window, cx));
//!
//! // 发送按钮
//! let button = SendButton::render(&state, || submit(), || cancel());
//! ```

pub mod ask_ai;
pub mod components;
pub mod engine;
mod panel;
pub mod rendering;
pub mod services;
pub mod stream;
mod types;

// 导出面板相关
pub use panel::*;

// 导出共享类型
pub use types::{
    ChatMessageUI, ChatMessageUIGeneric, ChatRole, MESSAGE_RENDER_LIMIT, MESSAGE_RENDER_STEP,
    MessageExtension, MessageVariant, ModelSelectItem, NoExtension, ProviderSelectItem,
};

// 导出引擎
pub use engine::ChatEngine;

// 导出渲染器
pub use rendering::ChatMessageRenderer;

// 导出流式处理器
pub use stream::{ChatStreamProcessor, StreamError, StreamEvent as CoreStreamEvent};

// 重导出常用组件（方便使用）
pub use components::{ModelItem, ProviderItem};

// 导出服务层
pub use services::{SessionError, SessionService, extract_session_name};
