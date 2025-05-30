use gpui::{
    div, prelude::FluentBuilder as _, px, rems, AnyElement, App, Entity, FocusHandle,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::button::Button;
use crate::ActiveTheme;
use crate::IconName;
use crate::Size;
use crate::StyledExt;

use super::text_input::TextInput;
use super::InputState;

// 提示：请确保按照上述说明修改 InputState 结构以包含 `attachments: Vec<std::path::PathBuf>` 字段。

#[derive(IntoElement)]
pub struct ChatInput {
    text_input_state: Entity<InputState>,
}

impl ChatInput {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            text_input_state: state.clone(),
        }
    }
}

impl RenderOnce for ChatInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_input_state_entity = self.text_input_state.clone();

        div() // 主容器
            .id("chat_input_bar")
            .flex_col()
            .p(px(8.))
            .gap(px(8.))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .bg(cx.theme().background)
            .w_full()
            .h_full()
            .child(
                // 顶部附件块 (水平排列)
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        // "添加上下文" 按钮
                        Button::new("add_context_btn").label("添加上下文").on_click(
                            |_event, _win, _app| {
                                println!("Add context clicked");
                            },
                        ),
                    )
                    .child(
                        // "mcp.rs:9 当前文件" 按钮/标签
                        Button::new("current_file_btn")
                            .label("mcp.rs:9 当前文件")
                            .on_click(|_event, _win, _app| {
                                println!("Current file tag clicked");
                            }),
                    ),
            )
            .child(
                // 中间：主要文本输入区域 (垂直伸展)
                div().flex_grow().w_full().child(
                    div().w_full().child(
                        TextInput::new(&self.text_input_state)
                            .appearance(false)
                            .no_gap(),
                    ),
                ),
            )
            .child(
                // 底部按钮块 (水平排列)
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(Button::new("mic_btn").icon(IconName::ArrowDown).on_click(
                        |_event, _win, _app| {
                            println!("Mic button clicked");
                        },
                    ))
                    .child(
                        Button::new("at_btn")
                            .label("@")
                            .on_click(|_event, _win, _app| {
                                println!("@ button clicked");
                            }),
                    )
                    .child(div().flex_grow())
                    .child(
                        Button::new("ask_btn")
                            .label("Ask")
                            .on_click(|_event, _win, _app| {
                                println!("Ask button clicked");
                            }),
                    )
                    .child(Button::new("gpt_model_btn").label("GPT-4.1").on_click(
                        |_event, _win, _app| {
                            println!("GPT model button clicked");
                        },
                    ))
                    .child(
                        Button::new("send_btn")
                            .icon(IconName::SquareTerminal)
                            .on_click({
                                // 为发送按钮的闭包克隆实体引用
                                let state_for_send = self.text_input_state.clone();
                                move |_event, event_window, event_app| {
                                    let current_text = state_for_send.read(event_app).text.clone();

                                    if !current_text.is_empty() {
                                        println!("Send Text: {}", current_text);

                                        state_for_send.update(event_app, |state, model_cx| {
                                            // 清理文本和附件
                                            state.clean(event_window, model_cx); // 假设 clean 会清理文本
                                            model_cx.notify();
                                        });
                                    }
                                }
                            }),
                    ),
            )
    }
}

// 备注:
// 1. InputState 初始化: `ChatInput::new` 中 `InputState` 的创建方式 (例如 `InputState::default(entity_cx)`)
//    和设置占位符 (`state.placeholder = ...`) 及焦点句柄 (`state.focus_handle = ...`) 的方式
//    取决于您项目中 `InputState` 的实际 API。请根据您的 `InputState` 定义进行调整。
// 2. 图标: `IconName::Paperclip`, `IconName::Mic`, `IconName::Send` 等图标需要在您的 `IconName` 枚举中定义。
// 3. 按钮样式: 您可能需要使用 `ButtonVariants` 或其他样式方法来调整按钮的外观以匹配图片。
// 4. 回调函数: `on_click` 闭包的参数签名 (`|_event, _win, _app|`) 取决于您的 `Button` 组件的实现。
//    `text_input.rs` 中的内部按钮使用 `|_, window, cx|`。
// 5. 主题和样式: `cx.theme().border`, `cx.theme().background`, `cx.theme().radius`
//    依赖于 `ActiveTheme` 的实现。`StyledExt` 提供的 `p()`, `gap()`, `rounded()`, `w_full()`, `flex_grow()`
//    也需要可用。
