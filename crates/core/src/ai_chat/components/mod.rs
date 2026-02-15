//! AI Chat 可复用组件
//!
//! 此模块包含可在不同聊天面板中复用的 UI 组件：
//!
//! - `ModelSettings` / `ModelSettingsPanel`: 模型参数设置
//! - `SendButton`: 发送/终止按钮
//! - `ProviderSelect`: Provider 选择器组件
//! - `SessionList`: 会话列表组件

mod model_settings;
mod send_button;
mod provider_select;
mod session_list;

pub use model_settings::{ModelSettings, ModelSettingsPanel, ModelSettingsEvent, ModelSettingsLabels};
pub use send_button::{SendButton, SendButtonEvent, SendButtonState};
pub use provider_select::{ProviderItem, ModelItem, ProviderSelectState, ProviderSelectEvent};
pub use session_list::{
    SessionData, SessionListConfig, SessionListItem, SessionListDelegate,
    SessionListHost,
};
