//! ChatPanel Delegate Trait
//!
//! 定义聊天面板的可扩展接口，允许不同的实现提供自定义行为。

use gpui::{AnyElement, App, Window};
use crate::ai_chat::types::ChatMessageUI;

/// 聊天面板代理 trait
///
/// 通过实现此 trait，可以自定义聊天面板的各种行为，例如：
/// - 自定义输入区域
/// - 自定义消息渲染
/// - 自定义工具栏
///
/// # 示例
///
/// ```rust,ignore
/// struct SqlChatDelegate {
///     // SQL 特有的状态
/// }
///
/// impl ChatPanelDelegate for SqlChatDelegate {
///     fn render_input_area(&self, window: &mut Window, cx: &App) -> AnyElement {
///         // 渲染 SQL 编辑器
///     }
/// }
/// ```
pub trait ChatPanelDelegate: 'static + Sized {
    /// 渲染输入区域
    ///
    /// 返回用于输入消息的 UI 元素。可以是简单的文本框，
    /// 也可以是复杂的 SQL 编辑器。
    fn render_input_area(&self, window: &mut Window, cx: &App) -> AnyElement;

    /// 渲染单条消息
    ///
    /// 默认实现提供基础的消息渲染。可以重写以添加自定义功能，
    /// 如 SQL 执行按钮、代码高亮等。
    fn render_message(&self, msg: &ChatMessageUI, window: &mut Window, cx: &App) -> AnyElement {
        default_render_message(msg, window, cx)
    }

    /// 渲染额外的工具栏项
    ///
    /// 返回要添加到头部工具栏的额外按钮。
    fn extra_toolbar_items(&self, _window: &mut Window, _cx: &App) -> Vec<AnyElement> {
        Vec::new()
    }

    /// 渲染额外的底部工具栏项
    ///
    /// 返回要添加到底部工具栏的额外按钮。
    fn extra_footer_items(&self, _window: &mut Window, _cx: &App) -> Vec<AnyElement> {
        Vec::new()
    }

    /// 获取输入内容
    ///
    /// 从输入区域获取当前的输入文本。
    fn get_input_content(&self) -> String;

    /// 清空输入
    ///
    /// 清空输入区域的内容。
    fn clear_input(&mut self, window: &mut Window, cx: &mut App);

    /// 设置加载状态
    ///
    /// 更新输入区域的加载状态（禁用/启用）。
    fn set_loading(&mut self, loading: bool, window: &mut Window, cx: &mut App);

    /// 处理提交前的钩子
    ///
    /// 在消息提交前调用，可以用于验证或预处理。
    /// 返回 `Some(content)` 表示继续提交，`None` 表示取消。
    fn on_before_submit(&mut self, content: String, _window: &mut Window, _cx: &mut App) -> Option<String> {
        Some(content)
    }

    /// 处理提交后的钩子
    ///
    /// 在消息成功提交后调用。
    fn on_after_submit(&mut self, _content: &str, _window: &mut Window, _cx: &mut App) {}

    /// 处理取消操作
    ///
    /// 当用户点击取消按钮时调用。
    fn on_cancel(&mut self, _window: &mut Window, _cx: &mut App) {}

    /// 处理消息流完成
    ///
    /// 当 AI 响应流式输出完成时调用。
    fn on_stream_complete(&mut self, _message: &ChatMessageUI, _window: &mut Window, _cx: &mut App) {}
}

/// 默认消息渲染实现
///
/// 提供基础的消息渲染，包括用户消息、助手消息和状态消息。
pub fn default_render_message(msg: &ChatMessageUI, _window: &mut Window, cx: &App) -> AnyElement {
    use gpui::{div, InteractiveElement, IntoElement, ParentElement, SharedString, Styled};
    use gpui_component::{h_flex, text::TextView, ActiveTheme, Icon, IconName, Sizable, Size};
    use crate::ai_chat::types::{ChatRole, MessageVariant};

    match msg.role {
        ChatRole::User => {
            div()
                .w_full()
                .px_3()
                .py_2()
                .bg(cx.theme().accent)
                .text_color(cx.theme().accent_foreground)
                .rounded_lg()
                .child(TextView::markdown(
                    SharedString::from(format!("user-msg-{}", msg.id)),
                    msg.content.clone(),
                ))
                .into_any_element()
        }
        ChatRole::Assistant => {
            match &msg.variant {
                MessageVariant::Status { title, is_done } => {
                    let icon = if *is_done {
                        IconName::Check
                    } else {
                        IconName::Loader
                    };

                    h_flex()
                        .id(SharedString::from(msg.id.clone()))
                        .w_full()
                        .items_center()
                        .gap_2()
                        .py_1()
                        .child(
                            Icon::new(icon)
                                .with_size(Size::Small)
                                .text_color(if *is_done {
                                    cx.theme().success
                                } else {
                                    cx.theme().muted_foreground
                                }),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(title.clone()),
                        )
                        .into_any_element()
                }
                MessageVariant::Text => {
                    if msg.is_streaming && msg.content.is_empty() {
                        div()
                            .w_full()
                            .py_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("思考中..."),
                            )
                            .into_any_element()
                    } else {
                        div()
                            .w_full()
                            .child(
                                TextView::markdown(
                                    SharedString::from(format!("ai-msg-{}", msg.id)),
                                    msg.content.clone(),
                                )
                                .p_3()
                                .selectable(true),
                            )
                            .into_any_element()
                    }
                }
                MessageVariant::SqlResult => {
                    // SqlResult 需要特殊渲染，默认实现只显示占位符
                    // 具体的 SQL 结果渲染由 ChatPanelDelegate 实现
                    div()
                        .w_full()
                        .py_2()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("SQL 结果"),
                        )
                        .into_any_element()
                }
            }
        }
        ChatRole::System => {
            h_flex()
                .w_full()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(msg.content.clone()),
                )
                .into_any_element()
        }
    }
}
