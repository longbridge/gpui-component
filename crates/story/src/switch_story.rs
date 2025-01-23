use gpui::{
    div, px, Div, IntoElement, ParentElement, Render, SharedString, Styled, View, ViewContext,
    VisualContext as _, WindowContext,
};

use ui::{
    checkbox::Checkbox,
    h_flex,
    label::Label,
    radio::{Radio, RadioGroup},
    switch::Switch,
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

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl SwitchStory {
    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }

    fn new(cx: &mut ViewContext<Self>) -> Self {
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

impl gpui::FocusableView for SwitchStory {
    fn focus_handle(&self, _: &gpui::AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SwitchStory {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.theme();

        fn title(title: impl Into<SharedString>) -> Div {
            v_flex().flex_1().gap_2().child(Label::new(title).text_xl())
        }

        fn card(cx: &ViewContext<SwitchStory>) -> Div {
            let theme = cx.theme();

            h_flex()
                .items_center()
                .gap_4()
                .p_4()
                .w_full()
                .rounded_lg()
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
                                .on_click(cx.listener(move |view, checked, cx| {
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
                                .on_click(cx.listener(move |view, checked, cx| {
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
                                .child(Switch::new("switch3").disabled(true).on_click(|v, _| {
                                    println!("Switch value changed: {:?}", v);
                                }))
                                .child(
                                    Switch::new("switch3_1")
                                        .label("Airplane Mode")
                                        .checked(true)
                                        .disabled(true)
                                        .on_click(|ev, _| {
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
                                    .on_click(cx.listener(move |view, checked, cx| {
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
                                cx.listener(|v, _, _| {
                                    v.check1 = !v.check1;
                                }),
                            ))
                            .child(
                                Checkbox::new("check2")
                                    .checked(self.check2)
                                    .label("Subscribe to newsletter")
                                    .on_click(cx.listener(|v, _, _| {
                                        v.check2 = !v.check2;
                                    })),
                            )
                            .child(
                                Checkbox::new("check3")
                                    .checked(self.check3)
                                    .label("Remember me")
                                    .on_click(cx.listener(|v, _, _| {
                                        v.check3 = !v.check3;
                                    })),
                            )
                            .child(div().w(px(300.)).child(
                                Checkbox::new("longlong-checkbox").label(
                                    "The long long label text, \
                                    it should ellipsis when the text is too long.",
                                ),
                            )),
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
                                cx.listener(|this, v, _cx| {
                                    this.radio_check1 = *v;
                                }),
                            ))
                            .child(
                                Radio::new("radio2")
                                    .label("Radio")
                                    .checked(self.radio_check2)
                                    .on_click(cx.listener(|this, v, _cx| {
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
                                div().w(px(200.)).child(
                                    Radio::new("radio3")
                                        .label("A long long long text radio label")
                                        .checked(true)
                                        .disabled(true),
                                ),
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
                                    .on_change(cx.listener(|this, selected_ix: &usize, _cx| {
                                        this.radio_group_checked = Some(*selected_ix);
                                    })),
                            ),
                        )
                        .child(
                            section("Radio Group Vertical", cx).flex_1().child(
                                RadioGroup::vertical()
                                    .disabled(true)
                                    .child(Radio::new("one1").label("United States"))
                                    .child(Radio::new("one2").label("Canada"))
                                    .child(Radio::new("one3").label("Mexico"))
                                    .selected_index(self.radio_group_checked)
                                    .on_change(cx.listener(|this, selected_ix: &usize, _cx| {
                                        this.radio_group_checked = Some(*selected_ix);
                                    })),
                            ),
                        ),
                ),
        )
    }
}
