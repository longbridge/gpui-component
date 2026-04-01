//! ChatMessageRenderer - 共享消息渲染工具
//!
//! 提供通用的消息渲染函数，可被不同的面板复用。
//! SQL 面板可以在此基础上覆盖特定渲染（如 SQL 代码块）。

use crate::ai_chat::panel::CodeBlockActionRegistry;
use crate::ai_chat::types::{ChatMessageUIGeneric, ChatRole, MessageExtension, MessageVariant};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div,
};
use gpui_component::button::Button;
use gpui_component::clipboard::Clipboard;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, button::ButtonVariants, h_flex, text::TextView,
};
use rust_i18n::t;

/// 共享消息渲染器
pub struct ChatMessageRenderer;

impl ChatMessageRenderer {
    /// 渲染用户消息
    pub fn render_user_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        cx: &App,
    ) -> AnyElement {
        div()
            .w_full()
            .px_3()
            .py_2()
            .bg(cx.theme().accent)
            .text_color(cx.theme().accent_foreground)
            .rounded_lg()
            .child(
                TextView::markdown(
                    SharedString::from(format!("user-msg-{}", msg.id)),
                    msg.content.clone(),
                )
                .selectable(true),
            )
            .into_any_element()
    }

    /// 渲染系统消息
    pub fn render_system_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        cx: &App,
    ) -> AnyElement {
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

    /// 渲染状态消息
    pub fn render_status_message(id: &str, title: &str, is_done: bool, cx: &App) -> AnyElement {
        let icon = if is_done {
            IconName::Check
        } else {
            IconName::Loader
        };

        h_flex()
            .id(SharedString::from(id.to_string()))
            .w_full()
            .items_center()
            .gap_2()
            .py_1()
            .child(
                Icon::new(icon)
                    .with_size(Size::Small)
                    .text_color(if is_done {
                        cx.theme().success
                    } else {
                        cx.theme().muted_foreground
                    }),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(title.to_string()),
            )
            .into_any_element()
    }

    /// 渲染 "思考中..." 占位符
    pub fn render_thinking(cx: &App) -> AnyElement {
        div()
            .w_full()
            .py_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("AiChat.thinking").to_string()),
            )
            .into_any_element()
    }

    /// 渲染助手文本消息（带代码块操作按钮）
    pub fn render_assistant_text<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        code_block_actions: &CodeBlockActionRegistry,
        cx: &App,
    ) -> AnyElement {
        if msg.is_streaming && msg.content.is_empty() {
            return Self::render_thinking(cx);
        }

        let view_id = SharedString::from(format!("ai-msg-{}", msg.id));

        if code_block_actions.is_empty() {
            // 无代码块操作，简单渲染
            div()
                .w_full()
                .child(
                    TextView::markdown(view_id, msg.content.clone())
                        .p_3()
                        .selectable(true),
                )
                .into_any_element()
        } else {
            // 有代码块操作，使用 code_block_actions
            let registry = code_block_actions.clone();
            div()
                .w_full()
                .child(
                    TextView::markdown(view_id, msg.content.clone())
                        .code_block_actions(move |code_block, _window, _cx| {
                            let code = code_block.code();
                            let lang = code_block.lang();
                            let lang_str = lang.as_ref().map(|s| s.as_ref());
                            let matched_actions = registry.get_actions_for_lang(lang_str);

                            let mut row = h_flex()
                                .gap_1()
                                .child(Clipboard::new("copy").value(code.clone()));

                            for (idx, action) in matched_actions.iter().enumerate() {
                                let btn_id = SharedString::from(format!("{}-{}", action.id, idx));
                                let callback = action.callback.clone();
                                let icon = action.icon.clone();
                                let label = action.label.clone();
                                let code = code.to_string();
                                let lang = lang.as_ref().map(|s| s.to_string());
                                let mut btn =
                                    Button::new(btn_id).icon(icon).ghost().xsmall().on_click({
                                        let code = code.clone();
                                        let lang = lang.clone();
                                        move |_, window, cx| {
                                            callback(code.clone(), lang.clone(), window, cx);
                                        }
                                    });

                                if let Some(lbl) = label {
                                    btn = btn.tooltip(lbl);
                                }

                                row = row.child(btn);
                            }

                            row
                        })
                        .p_3()
                        .selectable(true),
                )
                .into_any_element()
        }
    }

    /// 渲染单条消息（通用路由）
    pub fn render_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        code_block_actions: &CodeBlockActionRegistry,
        cx: &App,
    ) -> AnyElement {
        match msg.role {
            ChatRole::User => Self::render_user_message(msg, cx),
            ChatRole::Assistant => match &msg.variant {
                MessageVariant::Status { title, is_done } => {
                    Self::render_status_message(&msg.id, title, *is_done, cx)
                }
                MessageVariant::Text => Self::render_assistant_text(msg, code_block_actions, cx),
                MessageVariant::SqlResult => {
                    // SqlResult 需要特殊渲染，默认只显示占位符
                    div()
                        .w_full()
                        .py_2()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("AiChat.sql_result").to_string()),
                        )
                        .into_any_element()
                }
            },
            ChatRole::System => Self::render_system_message(msg, cx),
        }
    }
}
