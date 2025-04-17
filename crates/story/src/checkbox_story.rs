use gpui::{
    div, px, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};

use gpui_component::{
    checkbox::Checkbox, h_flex, text::TextView, v_flex, ActiveTheme, Disableable as _, Sizable,
};

use crate::section;

pub struct CheckboxStory {
    focus_handle: gpui::FocusHandle,
    check1: bool,
    check2: bool,
    check3: bool,
    check4: bool,
    check5: bool,
}

impl super::Story for CheckboxStory {
    fn title() -> &'static str {
        "Checkbox"
    }

    fn description() -> &'static str {
        "A control that allows the user to toggle between checked and not checked."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl CheckboxStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            check1: false,
            check2: false,
            check3: true,
            check4: false,
            check5: false,
        }
    }
}

impl Focusable for CheckboxStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CheckboxStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_6().child(
            v_flex()
                .items_start()
                .justify_center()
                .gap_4()
                .child(
                    section("Checkbox")
                        .child(
                            Checkbox::new("check1")
                                .checked(self.check1)
                                .on_click(cx.listener(|v, _, _, _| {
                                    v.check1 = !v.check1;
                                })),
                        )
                        .child(
                            Checkbox::new("check2")
                                .small()
                                .checked(self.check2)
                                .label("With 中文 Label")
                                .on_click(cx.listener(|v, _, _, _| {
                                    v.check2 = !v.check2;
                                })),
                        )
                        .child(
                            Checkbox::new("check3")
                                .checked(self.check3)
                                .label("Remember me")
                                .on_click(cx.listener(|v, _, _, _| {
                                    v.check3 = !v.check3;
                                })),
                        ),
                )
                .child(
                    section("Disabled").child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_6()
                            .child(
                                Checkbox::new("check3")
                                    .label("Disabled Checked")
                                    .checked(true)
                                    .disabled(true),
                            )
                            .child(
                                Checkbox::new("check3_1")
                                    .label("Disabled Unchecked")
                                    .checked(false)
                                    .disabled(true),
                            ),
                    ),
                )
                .child(
                    section("Multi-line Label").child(
                        v_flex()
                            .gap_4()
                            .child(
                                Checkbox::new("longlong-checkbox")
                                    .large()
                                    .w(px(300.))
                                    .checked(self.check4)
                                    .label("The long long label text.")
                                    .child(div().text_color(cx.theme().muted_foreground).child(
                                        "This is a long long label text that \
                                should wrap when the text is too long.",
                                    ))
                                    .on_click(cx.listener(|v, _, _, _| {
                                        v.check4 = !v.check4;
                                    })),
                            )
                            .child(
                                Checkbox::new("longlong-markdown-checkbox")
                                    .w(px(300.))
                                    .checked(self.check5)
                                    .label("Label with description")
                                    .child(div().text_color(cx.theme().muted_foreground).child(
                                        TextView::markdown(
                                            "longlong-markdown-checkbox",
                                            "The [long long label](https://github.com) \
                                    text used markdown, \
                                    it should wrap when the text is too long.",
                                        ),
                                    ))
                                    .on_click(cx.listener(|v, _, _, _| {
                                        v.check5 = !v.check5;
                                    })),
                            ),
                    ),
                ),
        )
    }
}
