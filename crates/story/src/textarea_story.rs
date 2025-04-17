use gpui::{
    actions, App, AppContext as _, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, ParentElement as _, Render, Styled, Window,
};

use crate::section;
use gpui_component::{button::Button, h_flex, input::TextInput, v_flex, FocusableCycle, Sizable};

actions!(input_story, [Tab, TabPrev]);

const CONTEXT: &str = "TextareaStory";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
    ])
}

pub struct TextareaStory {
    textarea: Entity<TextInput>,
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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl TextareaStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let textarea = cx.new(|cx| {
            let mut input = TextInput::new(window, cx)
                .multi_line()
                .rows(10)
                .placeholder("Enter text here...");
            input.set_text(
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
                ),
                window,
                cx,
            );
            input
        });

        Self { textarea }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
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

impl FocusableCycle for TextareaStory {
    fn cycle_focus_handles(&self, _: &mut Window, _: &mut App) -> Vec<FocusHandle> {
        [].to_vec()
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
            .key_context(CONTEXT)
            .id("textarea-story")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("Textarea").child(
                    v_flex()
                        .gap_2()
                        .w_full()
                        .child(self.textarea.clone())
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
    }
}
