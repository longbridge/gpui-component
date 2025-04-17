use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Window,
};

use gpui_component::{clipboard::Clipboard, label::Label, link::Link, v_flex, ContextModal};

use crate::section;

pub struct ClipboardStory {
    focus_handle: gpui::FocusHandle,
    masked: bool,
}

impl super::Story for ClipboardStory {
    fn title() -> &'static str {
        "Clipboard"
    }

    fn description() -> &'static str {
        "A button that helps you copy text or other content to your clipboard."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl ClipboardStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
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
        v_flex().gap_6().child(
            section("Copy to Clipboard")
                .max_w_md()
                .child(
                    Clipboard::new("clipboard1")
                        .content(|_, _| Label::new("Click icon to copy"))
                        .value_fn({
                            let view = cx.entity().clone();
                            move |_, cx| {
                                SharedString::from(format!("masked :{}", view.read(cx).masked))
                            }
                        })
                        .on_copied(|value, window, cx| {
                            window.push_notification(format!("Copied value: {}", value), cx)
                        }),
                )
                .child(
                    Clipboard::new("clipboard2")
                        .content(|_, _| {
                            Link::new("link1")
                                .href("https://github.com")
                                .child("GitHub")
                        })
                        .value("https://github.com")
                        .on_copied(|value, window, cx| {
                            window.push_notification(format!("Copied value: {}", value), cx)
                        }),
                ),
        )
    }
}
