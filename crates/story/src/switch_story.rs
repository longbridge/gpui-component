use gpui::{
    px, App, AppContext, Context, Div, Entity, Focusable, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

use gpui_component::{
    checkbox::Checkbox,
    h_flex,
    label::Label,
    radio::{Radio, RadioGroup},
    switch::Switch,
    text::TextView,
    v_flex, ActiveTheme, Disableable as _, Side, Sizable, StyledExt,
};

use crate::section;

pub struct SwitchStory {
    focus_handle: gpui::FocusHandle,
    switch1: bool,
    switch2: bool,
    switch3: bool,
    check1: bool,
    check2: bool,
    check3: bool,
    radio_check1: bool,
    radio_check2: bool,
    radio_group_checked: Option<usize>,
}

impl super::Story for SwitchStory {
    fn title() -> &'static str {
        "Switch"
    }

    fn description() -> &'static str {
        "Switch, Radio, Checkbox components testing and examples"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl SwitchStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            switch1: true,
            switch2: false,
            switch3: true,
            check1: false,
            check2: false,
            check3: true,
            radio_check1: false,
            radio_check2: true,
            radio_group_checked: None,
        }
    }
}

impl Focusable for SwitchStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SwitchStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        fn title(title: impl Into<SharedString>) -> Div {
            v_flex().flex_1().gap_2().child(Label::new(title).text_xl())
        }

        fn card(cx: &Context<SwitchStory>) -> Div {
            let theme = cx.theme();

            h_flex()
                .items_center()
                .gap_4()
                .p_4()
                .w_full()
                .rounded(cx.theme().radius)
                .border_1()
                .border_color(theme.border)
        }

        v_flex().gap_6().child(
            v_flex()
                .items_start()
                .justify_center()
                .gap_4()
                .child(
                    card(cx)
                        .child(
                            title("Marketing emails").child(
                                Label::new(
                                    "Receive emails about new products, features, and more.",
                                )
                                .text_color(theme.muted_foreground),
                            ),
                        )
                        .child(
                            Switch::new("switch1")
                                .checked(self.switch1)
                                .label_side(Side::Left)
                                .label("Subscribe")
                                .on_click(cx.listener(move |view, checked, _, cx| {
                                    view.switch1 = *checked;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    card(cx)
                        .child(
                            title("Security emails").child(
                                Label::new(
                                    "Receive emails about your account security. \
                                    When turn off, you never receive email again.",
                                )
                                .text_color(theme.muted_foreground),
                            ),
                        )
                        .child(
                            Switch::new("switch2")
                                .checked(self.switch2)
                                .on_click(cx.listener(move |view, checked, _, cx| {
                                    view.switch2 = *checked;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    card(cx)
                        .v_flex()
                        .items_start()
                        .child(title("Disabled Switches"))
                        .child(
                            h_flex()
                                .items_center()
                                .gap_6()
                                .child(Switch::new("switch3").disabled(true).on_click(|v, _, _| {
                                    println!("Switch value changed: {:?}", v);
                                }))
                                .child(
                                    Switch::new("switch3_1")
                                        .w(px(200.))
                                        .label("Airplane Mode")
                                        .checked(true)
                                        .disabled(true)
                                        .on_click(|ev, _, _| {
                                            println!("Switch value changed: {:?}", ev);
                                        }),
                                ),
                        ),
                )
                .child(
                    card(cx)
                        .v_flex()
                        .items_start()
                        .child(title("Small Switches"))
                        .child(
                            h_flex().items_center().gap_6().child(
                                Switch::new("switch3")
                                    .checked(self.switch3)
                                    .label("Small Size")
                                    .small()
                                    .on_click(cx.listener(move |view, checked, _, cx| {
                                        view.switch3 = *checked;
                                        cx.notify();
                                    })),
                            ),
                        ),
                )
                .child(
                    section("Checkbox", cx).child(
                        h_flex()
                            .w_full()
                            .items_start()
                            .gap_6()
                            .child(Checkbox::new("check1").checked(self.check1).on_click(
                                cx.listener(|v, _, _, _| {
                                    v.check1 = !v.check1;
                                }),
                            ))
                            .child(
                                Checkbox::new("check2")
                                    .checked(self.check2)
                                    .label("Subscribe to newsletter")
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
                            )
                            .child(Checkbox::new("longlong-checkbox").w(px(300.)).label(
                                "The long long label text, \
                                 it should wrap when the text is too long.",
                            ))
                            .child(
                                Checkbox::new("longlong-markdown-checkbox")
                                    .w(px(300.))
                                    .label(
                                        TextView::markdown(
                                            "longlong-markdown-checkbox",
                                            "The [long long label](https://github.com) text used markdown, \
                                             it should wrap when the text is too long.",
                                        )
                                        .inline(),
                                    ),
                            ),
                    ),
                )
                .child(
                    section("Disabled Checkbox", cx).child(
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
                    section("Radio", cx).child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .items_start()
                            .child(Radio::new("radio1").checked(self.radio_check1).on_click(
                                cx.listener(|this, v, _, _| {
                                    this.radio_check1 = *v;
                                }),
                            ))
                            .child(
                                Radio::new("radio2")
                                    .label("Radio")
                                    .checked(self.radio_check2)
                                    .on_click(cx.listener(|this, v, _, _| {
                                        this.radio_check2 = *v;
                                    })),
                            )
                            .child(
                                Radio::new("radio3")
                                    .label("Disabled Radio")
                                    .checked(true)
                                    .disabled(true),
                            )
                            .child(
                                Radio::new("radio3")
                                    .label(
                                        "The long long label text, \
                                         it should wrap when the text is too long.",
                                    )
                                    .w(px(300.))
                                    .checked(true)
                                    .disabled(true),
                            ),
                    ),
                )
                .child(
                    h_flex()
                        .items_start()
                        .gap_4()
                        .w_full()
                        .child(
                            section("Radio Group", cx).flex_1().child(
                                RadioGroup::horizontal()
                                    .children(["One", "Two", "Three"])
                                    .selected_index(self.radio_group_checked)
                                    .on_change(cx.listener(|this, selected_ix: &usize, _, _| {
                                        this.radio_group_checked = Some(*selected_ix);
                                    })),
                            ),
                        )
                        .child(
                            section("Radio Group Vertical (With container style)", cx)
                                .flex_1()
                                .child(
                                    RadioGroup::vertical()
                                        .w(px(220.))
                                        .p_2()
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .rounded_md()
                                        .disabled(true)
                                        .child(Radio::new("one1").label("United States"))
                                        .child(Radio::new("one2").label("Canada"))
                                        .child(Radio::new("one3").label("Mexico"))
                                        .selected_index(self.radio_group_checked)
                                        .on_change(cx.listener(
                                            |this, selected_ix: &usize, _, _| {
                                                this.radio_group_checked = Some(*selected_ix);
                                            },
                                        )),
                                ),
                        ),
                ),
        )
    }
}
