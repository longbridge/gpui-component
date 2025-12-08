use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, div, px,
};

use gpui_component::{
    ActiveTheme, Sizable, h_flex,
    radio::{Radio, RadioGroup},
    v_flex,
};

use crate::section;

pub struct RadioStory {
    focus_handle: gpui::FocusHandle,
    radio_check1: bool,
    radio_check2: bool,
    radio_group_checked: Option<usize>,
}

impl super::Story for RadioStory {
    fn title() -> &'static str {
        "Radio"
    }

    fn description() -> &'static str {
        "A set of checkable buttons—known as radio buttons—where no more than one of the buttons can be checked at a time."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl RadioStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            radio_check1: false,
            radio_check2: true,
            radio_group_checked: Some(1),
        }
    }
}

impl Focusable for RadioStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RadioStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Radio")
                    .max_w_md()
                    .child(
                        Radio::new("radio1")
                            .checked(self.radio_check1)
                            .on_click(cx.listener(|this, checked, _, _| {
                                this.radio_check1 = *checked;
                            })),
                    )
                    .child(
                        Radio::new("radio2")
                            .label("Radio 2")
                            .checked(self.radio_check2)
                            .on_click(cx.listener(|this, checked, _, _| {
                                this.radio_check2 = *checked;
                            })),
                    ),
            )
            .child(
                section("Disabled")
                    .child(Radio::new("a").label("Disabled").disabled(true))
                    .child(
                        Radio::new("b")
                            .label("Disabled with Checked")
                            .checked(true)
                            .disabled(true),
                    ),
            )
            .child(
                section("Multi-line Label").child(
                    Radio::new("radio3")
                        .label("The long long label text.")
                        .child(
                            div()
                                .text_color(cx.theme().muted_foreground)
                                .child("This line should wrap when the text is too long."),
                        )
                        .w(px(300.))
                        .checked(true)
                        .disabled(true),
                ),
            )
            .child(
                section("Sizeable").child(
                    h_flex()
                        .h_full()
                        .gap_x_4()
                        .child(
                            Radio::new("xsmall")
                                .label("Small")
                                .xsmall()
                                .checked(self.radio_check2)
                                .on_click(cx.listener(|this, v, _, _| {
                                    this.radio_check2 = *v;
                                })),
                        )
                        .child(
                            Radio::new("large")
                                .label("Large")
                                .large()
                                .checked(self.radio_check2)
                                .on_click(cx.listener(|this, v, _, _| {
                                    this.radio_check2 = *v;
                                })),
                        ),
                ),
            )
            .child(
                section("Radio Group").max_w_md().child(
                    v_flex().child(
                        RadioGroup::horizontal("radio_group_1")
                            .children(["One", "Two", "Three"])
                            .selected_index(self.radio_group_checked)
                            .on_click(cx.listener(|this, selected_ix: &usize, _, cx| {
                                this.radio_group_checked = Some(*selected_ix);
                                cx.notify();
                            })),
                    ),
                ),
            )
            .child(
                section("Radio Group Vertical (With container style)")
                    .max_w_md()
                    .child(
                        v_flex().items_center().content_center().child(
                            RadioGroup::vertical("radio_group_2")
                                .w(px(220.))
                                .p_2()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_md()
                                .disabled(true)
                                .child(Radio::new("one1").label("United States"))
                                .child(Radio::new("one2").label("Canada"))
                                .child(Radio::new("one3").label("Mexico"))
                                .selected_index(Some(1)),
                        ),
                    ),
            )
    }
}
