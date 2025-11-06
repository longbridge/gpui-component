use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Window,
};

use gpui_component::{
    WindowExt,
    clipboard::Clipboard,
    h_flex,
    input::{Input, InputState},
    label::Label,
    v_flex,
};

use crate::section;

pub struct ClipboardStory {
    focus_handle: gpui::FocusHandle,
    url_state: Entity<InputState>,
    masked: bool,
}

impl super::Story for ClipboardStory {
    fn title() -> &'static str {
        "Clipboard"
    }

    fn description() -> &'static str {
        "A button that helps you copy text or other content to your clipboard."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ClipboardStory {
    pub(crate) fn new(window: &mut Window, cx: &mut App) -> Self {
        let url_state =
            cx.new(|cx| InputState::new(window, cx).default_value("https://github.com"));

        Self {
            url_state,
            focus_handle: cx.focus_handle(),
            masked: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}
impl Focusable for ClipboardStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for ClipboardStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Clipboard").max_w_md().child(
                    h_flex()
                        .gap_2()
                        .child(Label::new("A clipboard button"))
                        .child(
                            Clipboard::new("clipboard1")
                                .value_fn({
                                    let view = cx.entity().clone();
                                    move |_, cx| {
                                        SharedString::from(format!(
                                            "masked :{}",
                                            view.read(cx).masked
                                        ))
                                    }
                                })
                                .on_copied(|value, window, cx| {
                                    window.push_notification(format!("Copied value: {}", value), cx)
                                }),
                        ),
                ),
            )
            .child(
                section("With in an Input").max_w_md().child(
                    Input::new(&self.url_state).suffix(
                        Clipboard::new("clipboard2")
                            .value_fn({
                                let state = self.url_state.clone();
                                move |_, cx| state.read(cx).value()
                            })
                            .on_copied(|value, window, cx| {
                                window.push_notification(format!("Copied value: {}", value), cx)
                            }),
                    ),
                ),
            )
    }
}
