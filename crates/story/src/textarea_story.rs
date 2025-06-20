use gpui::{
    actions, px, App, AppContext as _, ClickEvent, Context, Entity, Focusable, InteractiveElement,
    IntoElement, ParentElement as _, Render, Styled, Window,
};

use crate::section;
use gpui_component::{
    button::Button,
    h_flex,
    input::{InputState, TextInput},
    v_flex, Sizable,
};

actions!(input_story, [Tab, TabPrev]);

pub fn init(_: &mut App) {}

pub struct TextareaStory {
    textarea: Entity<InputState>,
    textarea_auto_grow: Entity<InputState>,
}

impl super::Story for TextareaStory {
    fn title() -> &'static str {
        "Textarea"
    }

    fn description() -> &'static str {
        "TextInput with multi-line mode."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl TextareaStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let textarea = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line()
                .rows(10)
                .placeholder("Enter text here...").default_value(
                unindent::unindent(
                    r#"Hello 世界，this is GPUI component.

                The GPUI Component is a collection of UI components for GPUI framework, including.

                Button, Input, Checkbox, Radio, Dropdown, Tab, and more...

                Here is an application that is built by using GPUI Component.

                > This application is still under development, not published yet.

                ![image](https://github.com/user-attachments/assets/559a648d-19df-4b5a-b563-b78cc79c8894)

                ![image](https://github.com/user-attachments/assets/5e06ad5d-7ea0-43db-8d13-86a240da4c8d)

                ## Demo

                If you want to see the demo, here is a some demo applications.
                "#,
                )
            )
        });

        let textarea_auto_grow = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(1, 5)
                .placeholder("Enter text here...")
                .default_value("Hello 世界，this is GPUI component.")
        });

        Self {
            textarea,
            textarea_auto_grow,
        }
    }

    fn on_insert_text_to_textarea(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.textarea.update(cx, |input, cx| {
            input.insert("Hello 你好", window, cx);
        });
    }

    fn on_replace_text_to_textarea(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.textarea.update(cx, |input, cx| {
            input.replace("Hello 你好", window, cx);
        });
    }
}

impl Focusable for TextareaStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.textarea.focus_handle(cx)
    }
}

impl Render for TextareaStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("textarea-story")
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("Textarea").child(
                    v_flex()
                        .gap_2()
                        .w_full()
                        .child(TextInput::new(&self.textarea).h(px(320.)))
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new("btn-insert-text")
                                        .xsmall()
                                        .label("Insert Text")
                                        .on_click(cx.listener(Self::on_insert_text_to_textarea)),
                                )
                                .child(
                                    Button::new("btn-replace-text")
                                        .xsmall()
                                        .label("Replace Text")
                                        .on_click(cx.listener(Self::on_replace_text_to_textarea)),
                                ),
                        ),
                ),
            )
            .child(
                section("Textarea Auto Grow").child(
                    v_flex()
                        .w_full()
                        .child(TextInput::new(&self.textarea_auto_grow)),
                ),
            )
    }
}
