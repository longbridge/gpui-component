use super::*;
use crate::app::AppState;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::TextInput,
    scroll::Scrollbar,
    text::TextView,
    *,
};

impl FocusableCycle for TodoThreadChat {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![self.chat_input.focus_handle(cx)]
    }
}

impl Focusable for TodoThreadChat {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TodoThreadChat {
    fn render_chat_message(&self, message: &ChatMessage) -> impl IntoElement {
        let is_user = matches!(message.role, MessageRole::User);

        h_flex()
            .w_full()
            .py_2()
            .px_3()
            .when(is_user, |this| this.justify_end())
            .when(!is_user, |this| this.justify_start())
            .child(
                div().max_w_full().flex_wrap().child(
                    v_flex()
                        .gap_1()
                        .child(
                            // 消息头部：角色和时间
                            h_flex()
                                .items_center()
                                .gap_2()
                                .when(is_user, |this| this.justify_end())
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(message.role.color())
                                        .font_medium()
                                        .child(message.role.display_name()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(gpui::rgb(0x9CA3AF))
                                        .child(message.timestamp.format("%H:%M").to_string()),
                                )
                                .when_some(message.model.as_ref(), |this, model| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(gpui::rgb(0x6B7280))
                                            .child(format!("({})", model)),
                                    )
                                }),
                        )
                        .child(
                            // 消息内容
                            div()
                                .p_3()
                                .rounded_lg()
                                .text_sm()
                                .when(is_user, |this| {
                                    this.bg(gpui::rgb(0x3B82F6)).text_color(gpui::rgb(0xFFFFFF))
                                })
                                .when(!is_user, |this| {
                                    this.bg(gpui::rgb(0xF3F4F6)).text_color(gpui::rgb(0x374151))
                                })
                                .child(TextView::markdown(
                                    SharedString::new(format!("chat-message-{}", message.id)),
                                    message.content.clone(),
                                )),
                        )
                        .when(!message.tools_used.is_empty(), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x6B7280))
                                    .child(format!("使用工具: {}", message.tools_used.join(", "))),
                            )
                        }),
                ),
            )
    }

    fn render_open_model_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let todo_edit_entity = cx.entity().clone();

        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
           // let providers = todo_edit_entity.read(drawer_cx).model_manager.providers.clone();
            let providers = AppState::state(drawer_cx).llm_provider.providers.clone();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_providers.clone();
            let todoitem = todo_edit_entity.read(drawer_cx).todoitem.clone();
            let mut accordion = Accordion::new("chat-model-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, todo_cx| {
                            todo_edit.toggle_accordion(open_indices, todo_cx);
                        });
                    }
                });

            for (provider_index, provider) in providers.iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_models = provider.models.clone();

                //let has_selected_models = provider_models.iter().any(|model| model.is_selected);
                let has_selected_models = provider_models.iter().any(|model| {
                    todoitem.selected_model.iter().any(|selected| selected.model_id == model.id && selected.provider_id == provider.id)
                });
                let is_expanded = has_selected_models || expanded_providers.contains(&provider_index);

                accordion = accordion.item(|item| {
                    item.open(is_expanded)
                        .icon(IconName::Bot)
                        .title(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .font_medium()
                                                .text_color(gpui::rgb(0x374151))
                                                .child(provider_name.clone()),
                                        )
                                        .when(has_selected_models, |this| {
                                            this.child(
                                                Icon::new(IconName::Check)
                                                    .xsmall()
                                                    .text_color(gpui::rgb(0x10B981)),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(if has_selected_models {
                                            gpui::rgb(0xDCFCE7)
                                        } else {
                                            gpui::rgb(0xEFF6FF)
                                        })
                                        .text_color(if has_selected_models {
                                            gpui::rgb(0x166534)
                                        } else {
                                            gpui::rgb(0x1D4ED8)
                                        })
                                        .rounded_md()
                                        .text_xs()
                                        .child(format!("{} 个模型", provider_models.len())),
                                ),
                        )
                        .content(
                            v_flex()
                                .gap_2()
                                .p_2()
                                .children(provider_models.iter().enumerate().map(
                                    |(model_index, model)| {
                                        let model_name_for_event = model.display_name.clone();
                                        let checkbox_id = SharedString::new(format!(
                                            "chat-model-{}-{}",
                                            provider_index, model_index
                                        ));
                                        let todo_edit_entity_for_event = todo_edit_entity.clone();

                                        div()
                                            .p_1()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .rounded_md()
                                            // .border_1()
                                            // .border_color(gpui::rgb(0xE5E7EB))
                                            .hover(|style| style.bg(gpui::rgb(0xF3F4F6)))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .gap_3()
                                                            .child(
                                                                Checkbox::new(checkbox_id)
                                                                    .checked(todoitem.selected_model.iter().any(|selected|
                                                                            selected.model_id == model.id && selected.provider_id == provider.id
                                                                        ))
                                                                    .label(model.display_name.clone())
                                                                    .on_click({
                                                                        let model_clone = model.clone();
                                                                                let provider_clone = provider.clone();
                                                                        move |checked, window, cx| {
                                                                            let model_name_to_toggle =
                                                                                model_name_for_event.clone();
                                                                            // 更新原始数据
                                                                            todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                todo_edit.toggle_model_selection(*checked,&model_clone, &provider_clone, todo_cx);
                                                                                todo_edit.save( window, todo_cx);
                                                                                todo_cx.notify();
                                                                            });
                                                                            println!("切换模型选择: {}",model_name_to_toggle);
                                                                        }
                                                                    }
                                                                ),
                                                            )
                                                            .child(
                                                                h_flex().gap_1().items_center().children(
                                                                    model.capabilities.iter().enumerate().map(
                                                                        |(cap_index, cap)| {
                                                                            let capability_unique_id = provider_index * 10000
                                                                                + model_index * 1000
                                                                                + cap_index;

                                                                            div()
                                                                                .id(("chat_capability", capability_unique_id))
                                                                                .p_1()
                                                                                .rounded_md()
                                                                                .bg(gpui::rgb(0xF3F4F6))
                                                                                .child(
                                                                                    Icon::new(cap.icon())
                                                                                        .xsmall()
                                                                                        .text_color(gpui::rgb(0x6B7280)),
                                                                                )
                                                                        },
                                                                    ),
                                                                ),
                                                            ),
                                                    ),
                                            )
                                    },
                                ))
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(280.))
                .title("选择模型")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-chat-models")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        todo_edit.todoitem.selected_model=None;
                                        todo_edit.save( window, todo_cx);
                                        todo_cx.notify();
                                    });
                                    // println!("清空所有模型选择");
                                    // window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }

    fn render_open_tool_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let todo_edit_entity = cx.entity().clone();
        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            let providers = AppState::state(drawer_cx).mcp_provider.load_providers().unwrap_or_default();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_tool_providers.clone();
            let todoitem = todo_edit_entity.read(drawer_cx).todoitem.clone();
            let mut accordion = Accordion::new("chat-tool-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, todo_cx| {
                            todo_edit.toggle_tool_accordion(open_indices, todo_cx);
                        });
                    }
                });

            for (provider_index, provider) in providers.iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_tools = provider.tools.clone();

                let has_selected_tools = provider_tools.iter().any(|tool|  todoitem.selected_tools.iter().any(|selected| selected.tool_name == tool.name && selected.provider_id == provider.id));
                let is_expanded = has_selected_tools || expanded_providers.contains(&provider_index);

                accordion = accordion.item(|item| {
                    item.open(is_expanded)
                        .icon(IconName::Wrench)
                        .title(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .font_medium()
                                                .text_color(gpui::rgb(0x374151))
                                                .child(provider_name.clone()),
                                        )
                                        .when(has_selected_tools, |this| {
                                            this.child(
                                                Icon::new(IconName::Check)
                                                    .xsmall()
                                                    .text_color(gpui::rgb(0x10B981)),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(if has_selected_tools {
                                            gpui::rgb(0xDCFCE7)
                                        } else {
                                            gpui::rgb(0xFFF7ED)
                                        })
                                        .text_color(if has_selected_tools {
                                            gpui::rgb(0x166534)
                                        } else {
                                            gpui::rgb(0xEA580C)
                                        })
                                        .rounded_md()
                                        .text_xs()
                                        .child(format!("{} 个工具", provider_tools.len())),
                                ),
                        )
                        .content(
                            v_flex()
                                .gap_2()
                                .p_2()
                                .children(provider_tools.iter().enumerate().map(
                                    |(tool_index, tool)| {
                                        let tool_name_for_event = tool.name.clone();
                                        let checkbox_id = SharedString::new(format!(
                                            "chat-tool-{}-{}",
                                            provider_index, tool_index
                                        ));
                                        let todo_edit_entity_for_event = todo_edit_entity.clone();

                                        div()
                                            .p_1()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .rounded_md()
                                            .hover(|style| style.bg(gpui::rgb(0xF3F4F6)))
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .child(
                                                                h_flex()
                                                                    .items_center()
                                                                    .gap_3()
                                                                    .child(
                                                                        Checkbox::new(checkbox_id)
                                                                            .checked(todoitem.selected_tools.iter().any(|selected|
                                                                            selected.tool_name == tool.name && selected.provider_id == provider.id
                                                                        ))
                                                                            .label(tool.name.clone().to_string())
                                                                            .on_click({
                                                                                let tool_clone = tool.clone();
                                                                                let provider_clone = provider.clone();
                                                                                move |checked, window, cx| {
                                                                                    let tool_name_to_toggle =
                                                                                        tool_name_for_event.clone();

                                                                                    // 更新原始数据
                                                                                    todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                        todo_edit.toggle_tool_selection(*checked,&tool_clone, &provider_clone, todo_cx);
                                                                                        todo_edit.save( window, todo_cx);
                                                                                        todo_cx.notify();
                                                                                    });
                                                                                    println!(
                                                                                        "切换工具选择: {}",
                                                                                        tool_name_to_toggle
                                                                                    );
                                                                                }
                                                                            }
                                                                            ),
                                                                    )
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .pl_6()
                                                            .text_xs()
                                                            .text_color(gpui::rgb(0x6B7280))
                                                            .child(tool.description.clone().unwrap_or_default().to_string()),
                                                    ),
                                            )
                                    },
                                ))
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(280.))
                .title("选择工具集")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-chat-tools")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        todo_edit.todoitem.selected_tools.clear();
                                        todo_edit.save( window, todo_cx);
                                        todo_cx.notify();
                                    });
                                    // println!("清空所有工具选择");
                                    // window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }
}

impl Render for TodoThreadChat {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_model = self.get_model_display_text(cx);
        let selected_tool = self.get_tool_display_text(cx);
        let has_tools = !selected_tool.is_empty();
        let has_models = !selected_model.is_empty();
        v_flex()
            .key_context(CONTEXT)
            .id("todo-thread-view")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::send_message))
            .size_full()
            .p_2()
            .gap_2()
            .child(
                div().size_full().min_h_32().child(
                    div()
                        .relative()
                        .size_full()
                        .child(
                            v_flex()
                                // .border_1()
                                // .border_color(gpui::rgb(0xE5E7EB))
                                .relative()
                                .size_full()
                                .child(
                                    v_flex()
                                        .id("id-todo-thread-chat")
                                        .p_1()
                                        .gap_1()
                                        .overflow_y_scroll()
                                        .track_scroll(&self.scroll_handle)
                                        .children(
                                            self.chat_messages
                                                .iter()
                                                .map(|msg| self.render_chat_message(msg)),
                                        )
                                        .when(self.is_loading, |this| {
                                            this.child(
                                                h_flex().justify_start().py_2().child(
                                                    div()
                                                        .p_3()
                                                        .bg(gpui::rgb(0xF3F4F6))
                                                        .rounded_lg()
                                                        .text_color(gpui::rgb(0x6B7280))
                                                        .child("AI正在思考中..."),
                                                ),
                                            )
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .child(
                                    Scrollbar::vertical(&self.scroll_state, &self.scroll_handle)
                                        .scroll_size(self.scroll_size),
                                ),
                        ),
                ),
            )
            .child(
                // 聊天输入区域 - 固定在底部
                v_flex()
                    .p_1()
                    .gap_0()
                    .border_1()
                    .border_1()
                    .rounded_lg()
                    .border_color(gpui::rgb(0xE5E7EB))
                    .when(has_models || has_tools, |this| {
                        this.child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .bg(gpui::rgb(0xF9FAFB))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(gpui::rgb(0x6B7280))
                                        .child(selected_model),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(gpui::rgb(0x6B7280))
                                        .child(selected_tool),
                                ),
                        )
                    })
                    .child(
                        h_flex()
                            .items_center()
                            .justify_start()
                            .gap_1()
                            // .p_1()
                            .bg(gpui::rgb(0xF9FAFB))
                            .child(
                                h_flex().justify_start().items_center().gap_2().child(
                                    Button::new("show-chat-model-drawer")
                                        .icon(
                                            Icon::new(IconName::Database)
                                                .xsmall()
                                                .when(has_models, |this| {
                                                    this.text_color(green_500())
                                                }),
                                        )
                                        .ghost()
                                        .small()
                                        .justify_center()
                                        .tooltip("选择模型")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.render_open_model_drawer_at(
                                                Placement::Left,
                                                window,
                                                cx,
                                            )
                                        })),
                                ),
                            )
                            .child(
                                h_flex().justify_start().items_center().gap_2().child(
                                    Button::new("show-chat-tool-drawer")
                                        .icon(
                                            Icon::new(IconName::Wrench)
                                                .xsmall()
                                                .when(has_tools, |this| {
                                                    this.text_color(green_500())
                                                }),
                                        )
                                        .ghost()
                                        .small()
                                        .justify_center()
                                        .tooltip("选择工具")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.render_open_tool_drawer_at(
                                                Placement::Left,
                                                window,
                                                cx,
                                            )
                                        })),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            // .p_1()
                            .child(
                                // 多行输入框
                                div()
                                    .w_full()
                                    .text_sm()
                                    .child(TextInput::new(&self.chat_input).bordered(false)),
                            )
                            .child(
                                h_flex().justify_end().child(
                                    Button::new("send-message")
                                        .with_variant(ButtonVariant::Primary)
                                        .icon(IconName::Send)
                                        // .label("发送")
                                        .disabled(self.is_loading)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            // window.dispatch_action(Box::new(SendMessage), cx);
                                            this.send_message(&SendMessage, window, cx);
                                        })),
                                ),
                            ),
                    ),
            )
    }
}
