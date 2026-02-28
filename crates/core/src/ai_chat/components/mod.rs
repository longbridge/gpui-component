//! AI Chat 可复用组件
//!
//! 此模块包含可在不同聊天面板中复用的 UI 组件：
//!
//! - `ModelSettings` / `ModelSettingsPanel`: 模型参数设置
//! - `SendButton`: 发送/终止按钮
//! - `ProviderSelect`: Provider 选择器组件
//! - `SessionList`: 会话列表组件

mod model_settings;
mod provider_select;
mod send_button;
mod session_list;

pub use model_settings::{
    ModelSettings, ModelSettingsEvent, ModelSettingsLabels, ModelSettingsPanel,
};
pub use provider_select::{ModelItem, ProviderItem, ProviderSelectEvent, ProviderSelectState};
pub use send_button::{SendButton, SendButtonEvent, SendButtonState};
pub use session_list::{
    SessionData, SessionListConfig, SessionListDelegate, SessionListHost, SessionListItem,
};
