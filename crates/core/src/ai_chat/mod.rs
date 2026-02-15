//! AI Chat Panel - 通用 AI 助手对话面板
//!
//! 此模块提供可复用的 AI 聊天界面组件，可以在不同的应用中使用。
//!
//! # 模块结构
//!
//! - `types`: 共享类型定义（ChatMessageUI, ChatRole 等）
//! - `delegate`: 面板代理 trait，用于自定义行为
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
//! ## 自定义面板行为
//!
//! 实现 `ChatPanelDelegate` trait 来自定义面板行为：
//!
//! ```rust,ignore
//! use one_core::ai_chat::{ChatPanelDelegate, ChatMessageUI};
//!
//! struct MyDelegate { /* ... */ }
//!
//! impl ChatPanelDelegate for MyDelegate {
//!     fn render_input_area(&self, window: &mut Window, cx: &App) -> AnyElement {
//!         // 自定义输入区域
//!     }
//!     // ...
//! }
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

mod panel;
mod types;
mod delegate;
pub mod components;
pub mod ask_ai;
pub mod services;

// 导出面板相关
pub use panel::*;

// 导出共享类型
pub use types::{
    ChatMessageUI,
    ChatRole,
    MessageVariant,
    ProviderSelectItem,
    ModelSelectItem,
    MESSAGE_RENDER_LIMIT,
    MESSAGE_RENDER_STEP,
};

// 导出代理 trait
pub use delegate::{ChatPanelDelegate, default_render_message};

// 重导出常用组件（方便使用）
pub use components::{ProviderItem, ModelItem};

// 导出服务层
pub use services::{SessionService, SessionError, extract_session_name};
