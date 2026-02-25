//! 发送按钮组件 - 统一的发送/终止按钮
//!
//! 提供可复用的发送/终止按钮组件，支持：
//! - 正常状态显示发送按钮
//! - 加载状态显示终止按钮
//! - 自定义标签

use gpui::{AnyElement, App, IntoElement, Window};
use gpui_component::{
    button::{Button, ButtonVariants},
    IconName, Sizable, Size,
};
use rust_i18n::t;

// ============================================================================
// 事件定义
// ============================================================================

/// 发送按钮事件
#[derive(Clone, Debug)]
pub enum SendButtonEvent {
    /// 提交
    Submit,
    /// 取消
    Cancel,
}

// ============================================================================
// 发送按钮状态
// ============================================================================

/// 发送按钮状态
#[derive(Clone, Debug, Default)]
pub struct SendButtonState {
    /// 是否正在加载
    pub is_loading: bool,
    /// 发送按钮标签
    pub send_label: String,
    /// 终止按钮标签
    pub cancel_label: String,
}

impl SendButtonState {
    /// 创建新的发送按钮状态
    pub fn new() -> Self {
        Self {
            is_loading: false,
            send_label: t!("AiChat.send").to_string(),
            cancel_label: t!("AiChat.cancel").to_string(),
        }
    }

    /// 设置加载状态
    pub fn with_loading(mut self, is_loading: bool) -> Self {
        self.is_loading = is_loading;
        self
    }

    /// 设置发送标签
    pub fn with_send_label(mut self, label: impl Into<String>) -> Self {
        self.send_label = label.into();
        self
    }

    /// 设置终止标签
    pub fn with_cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// 切换加载状态
    pub fn set_loading(&mut self, is_loading: bool) {
        self.is_loading = is_loading;
    }
}

// ============================================================================
// 发送按钮组件
// ============================================================================

/// 发送按钮组件
///
/// 根据加载状态自动切换发送/终止按钮。
pub struct SendButton;

impl SendButton {
    /// 渲染发送按钮
    ///
    /// # 参数
    /// - `state`: 按钮状态
    /// - `on_submit`: 提交回调
    /// - `on_cancel`: 取消回调
    pub fn render<F, G>(state: &SendButtonState, on_submit: F, on_cancel: G) -> AnyElement
    where
        F: Fn(&mut Window, &mut App) + 'static,
        G: Fn(&mut Window, &mut App) + 'static,
    {
        if state.is_loading {
            Button::new("send-cancel")
                .with_size(Size::Small)
                .danger()
                .icon(IconName::CircleX)
                .label(state.cancel_label.clone())
                .on_click(move |_, window, cx| on_cancel(window, cx))
                .into_any_element()
        } else {
            Button::new("send-submit")
                .with_size(Size::Small)
                .primary()
                .icon(IconName::ArrowRight)
                .label(state.send_label.clone())
                .on_click(move |_, window, cx| on_submit(window, cx))
                .into_any_element()
        }
    }

    /// 渲染带有自定义 ID 的发送按钮
    pub fn render_with_id<F, G>(
        id: impl Into<gpui::ElementId>,
        state: &SendButtonState,
        on_submit: F,
        on_cancel: G,
    ) -> AnyElement
    where
        F: Fn(&mut Window, &mut App) + 'static,
        G: Fn(&mut Window, &mut App) + 'static,
    {
        let id = id.into();
        if state.is_loading {
            Button::new(id)
                .with_size(Size::Small)
                .danger()
                .icon(IconName::CircleX)
                .label(state.cancel_label.clone())
                .on_click(move |_, window, cx| on_cancel(window, cx))
                .into_any_element()
        } else {
            Button::new(id)
                .with_size(Size::Small)
                .primary()
                .icon(IconName::ArrowRight)
                .label(state.send_label.clone())
                .on_click(move |_, window, cx| on_submit(window, cx))
                .into_any_element()
        }
    }
}
