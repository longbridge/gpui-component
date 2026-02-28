//! Ask AI - 询问AI功能的可复用组件和全局事件机制

use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, Global, IntoElement, SharedString,
};
use gpui_component::{
    IconName, Sizable, Size,
    button::{Button, ButtonVariants},
};
use rust_i18n::t;

/// AI 请求事件 - 直接发送格式化后的消息
#[derive(Clone, Debug)]
pub enum AskAiEvent {
    Request(String),
}

/// 全局 AI 通知器
pub struct AskAiNotifier;

impl EventEmitter<AskAiEvent> for AskAiNotifier {}

/// 全局包装器，存储 Entity<AskAiNotifier>
#[derive(Clone)]
pub struct GlobalAskAiNotifier(pub Entity<AskAiNotifier>);

impl Global for GlobalAskAiNotifier {}

/// 初始化全局 AI 通知器
pub fn init_ask_ai_notifier(cx: &mut App) {
    let notifier = cx.new(|_| AskAiNotifier);
    cx.set_global(GlobalAskAiNotifier(notifier));
}

/// 获取全局 AI 通知器 Entity
pub fn get_ask_ai_notifier(cx: &App) -> Option<Entity<AskAiNotifier>> {
    cx.try_global::<GlobalAskAiNotifier>().map(|g| g.0.clone())
}

/// 辅助函数：发送 AI 请求事件 - Context 版本
pub fn emit_ask_ai_event<T>(message: String, cx: &mut Context<T>) {
    if let Some(notifier) = cx.try_global::<GlobalAskAiNotifier>().cloned() {
        notifier.0.update(cx, |_, cx| {
            cx.emit(AskAiEvent::Request(message));
        });
    }
}

/// 辅助函数：发送 AI 请求事件 - App 版本
pub fn emit_ask_ai_event_app(message: String, cx: &mut App) {
    if let Some(notifier) = cx.try_global::<GlobalAskAiNotifier>().cloned() {
        notifier.0.update(cx, |_, cx| {
            cx.emit(AskAiEvent::Request(message));
        });
    }
}

/// 询问AI按钮 - 可复用组件
/// 点击时组装提示词并发送全局事件
pub struct AskAiButton {
    id: SharedString,
    sql: String,
    error_message: String,
    context: Option<String>,
    size: Size,
}

impl AskAiButton {
    pub fn new(
        id: impl Into<SharedString>,
        sql: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            sql: sql.into(),
            error_message: error_message.into(),
            context: None,
            size: Size::Small,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    fn format_message(&self) -> String {
        let mut message = t!(
            "AiChat.ask_ai_template",
            sql = self.sql.trim(),
            error = self.error_message.trim()
        )
        .to_string();

        if let Some(ctx) = &self.context {
            message.push_str(&t!("AiChat.ask_ai_context", context = ctx).to_string());
        }

        message.push_str(&t!("AiChat.ask_ai_request_help").to_string());
        message
    }
}

impl IntoElement for AskAiButton {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let message = self.format_message();

        Button::new(self.id)
            .icon(IconName::Bot)
            .label(t!("AiChat.ask_ai_button").to_string())
            .ghost()
            .with_size(self.size)
            .on_click(move |_event, _window, cx| {
                emit_ask_ai_event_app(message.clone(), cx);
            })
            .into_any_element()
    }
}
