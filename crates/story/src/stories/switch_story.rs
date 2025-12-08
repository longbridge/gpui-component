use gpui::{
    App, AppContext, Context, Div, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, SharedString, Styled, Window, px,
};

use gpui_component::{
    ActiveTheme, Disableable as _, Sizable, h_flex, label::Label, switch::Switch, v_flex,
};

use crate::section;

pub struct SwitchStory {
    focus_handle: FocusHandle,
    switch1: bool,
    switch2: bool,
    switch3: bool,
}

impl super::Story for SwitchStory {
    fn title() -> &'static str {
        "Switch"
    }

    fn description() -> &'static str {
        "A control that allows the user to toggle between checked and not checked."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
            h_flex()
                .items_center()
                .gap_4()
                .p_4()
                .w_full()
                .rounded(cx.theme().radius)
                .border_1()
                .border_color(cx.theme().border)
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
                            h_flex().gap_2().child("Subscribe").child(
                                Switch::new("switch1")
                                    .checked(self.switch1)
                                    .on_click(cx.listener(move |view, checked, _, cx| {
                                        view.switch1 = *checked;
                                        cx.notify();
                                    })),
                            ),
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
                    section("Disabled")
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
                )
                .child(
                    section("Small Size").child(
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
    }
}
