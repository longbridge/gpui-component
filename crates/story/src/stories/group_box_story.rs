use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    StyleRefinement, Styled, Window, relative,
};

use gpui_component::{
    ActiveTheme as _, StyledExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    radio::{Radio, RadioGroup},
    switch::Switch,
    text::markdown,
    v_flex,
};

use crate::section;

pub struct GroupBoxStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for GroupBoxStory {
    fn title() -> &'static str {
        "GroupBox"
    }

    fn description() -> &'static str {
        "A styled container element that with an optional title \
        to groups related content together."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl GroupBoxStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for GroupBoxStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GroupBoxStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_6().child(
            v_flex()
                .items_start()
                .justify_center()
                .gap_4()
                .child(
                    section("Default Style").w_128().child(
                        GroupBox::new()
                            .child("Subscriptions")
                            .child(Checkbox::new("all").label("All"))
                            .child(Checkbox::new("news-letter").label("News Letter"))
                            .child(Checkbox::new("account-activity").label("Account Activity"))
                            .child(Button::new("ok").primary().label("Update Subscriptions")),
                    ),
                )
                .child(
                    section("Fill Style").w_128().child(
                        GroupBox::new()
                            .id("activity")
                            .fill()
                            .title("Contributions & activity")
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child("Make profile private and hide activity")
                                    .child(Switch::new("toggle-0").checked(true)),
                            )
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child("Include private contributions on my profile")
                                    .child(Switch::new("toggle-1").checked(false)),
                            )
                            .child(Button::new("btn-1").primary().label("Save")),
                    ),
                )
                .child(
                    section("Outline Style").w_128().child(
                        GroupBox::new()
                            .id("appearance")
                            .outline()
                            .title("Appearance")
                            .child(
                                RadioGroup::vertical("theme")
                                    .child(Radio::new("light").label("Light"))
                                    .child(Radio::new("dark").label("Dark"))
                                    .child(Radio::new("system").label("System")),
                            ),
                    ),
                )
                .child(
                    section("Without Title").w_128().child(
                        GroupBox::new().outline().child(
                            h_flex()
                                .justify_between()
                                .child("Make profile private and hide activity")
                                .child(Switch::new("toggle-1").checked(true)),
                        ),
                    ),
                )
                .child(
                    section("Custom style").w_128().child(
                        GroupBox::new()
                            .outline()
                            .bg(cx.theme().group_box)
                            .rounded_xl()
                            .p_5()
                            .title("This is a custom style")
                            .title_style(
                                StyleRefinement::default()
                                    .font_semibold()
                                    .line_height(relative(1.0))
                                    .px_3(),
                            )
                            .content_style(
                                StyleRefinement::default()
                                    .rounded_xl()
                                    .py_3()
                                    .px_4()
                                    .border_2(),
                            )
                            .child(markdown(
                                "You can use `title_style` to customize the style \
                                of the title. \n \
                                And any style in `GroupBox` will apply to the content container.",
                            )),
                    ),
                ),
        )
    }
}
