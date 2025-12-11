use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, div, px,
};

use gpui_component::{
    ActiveTheme, Disableable as _, Sizable, checkbox::Checkbox, h_flex, text::markdown, v_flex,
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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
            check3: false,
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
                            Checkbox::new("1")
                                .checked(self.check1)
                                .label("A normal checkbox")
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.check1 = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Checkbox::new("2")
                                .checked(self.check2)
                                .label("Remember me")
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.check2 = *checked;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    section("Without label").child(
                        Checkbox::new("3")
                            .checked(self.check3)
                            .on_click(cx.listener(|this, checked: &bool, _, _| {
                                this.check3 = *checked;
                            })),
                    ),
                )
                .child(
                    section("Small size").max_w_md().child(
                        Checkbox::new("4")
                            .small()
                            .checked(self.check4)
                            .label("A small checkbox")
                            .on_click(cx.listener(|this, checked: &bool, _, _| {
                                this.check4 = *checked;
                            })),
                    ),
                )
                .child(
                    section("Large size").max_w_md().child(
                        Checkbox::new("check5")
                            .large()
                            .checked(self.check2)
                            .label("A large checkbox")
                            .on_click(cx.listener(|this, checked: &bool, _, _| {
                                this.check2 = *checked;
                            })),
                    ),
                )
                .child(
                    section("Disabled").max_w_md().child(
                        h_flex()
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
                    section("Multi-line").child(
                        v_flex().gap_4().child(
                            Checkbox::new("multi-line-checkbox")
                                .w(px(300.))
                                .checked(self.check4)
                                .label("A multi-line checkbox.")
                                .child(div().text_color(cx.theme().muted_foreground).child(
                                    "This is a long long label text that \
                                should wrap when the text is too long.",
                                ))
                                .on_click(cx.listener(|this, checked: &bool, _, _| {
                                    this.check4 = *checked;
                                })),
                        ),
                    ),
                )
                .child(
                    section("Rich description (Markdown)").child(
                        Checkbox::new("longlong-markdown-checkbox")
                            .w(px(300.))
                            .checked(self.check5)
                            .label("Label with description (Markdown)")
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(markdown(
                                        "The [long long label](https://github.com) \
                            text used **Markdown**, \
                            it should wrap when the text is too long.",
                                    )),
                            )
                            .on_click(cx.listener(|this, checked: &bool, _, _| {
                                this.check5 = *checked;
                            })),
                    ),
                ),
        )
    }
}
