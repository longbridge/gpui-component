use gpui::div;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, prelude::FluentBuilder as _,
};

use gpui_component::group_box::{GroupBox, GroupBoxVariants as _};
use gpui_component::label::Label;
use gpui_component::tag::Tag;
use gpui_component::{ActiveTheme, IconName, StyledExt, h_flex};
use gpui_component::{
    Sizable,
    button::{Button, ButtonVariants},
    collapsible::Collapsible,
    v_flex,
};

use crate::section;

pub struct CollapsibleStory {
    focus_handle: FocusHandle,
    item1_open: bool,
    item2_open: bool,
}

impl super::Story for CollapsibleStory {
    fn title() -> &'static str {
        "Collapsible"
    }

    fn description() -> &'static str {
        "An interactive element that expands/collapses."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CollapsibleStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            item1_open: false,
            item2_open: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Focusable for CollapsibleStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CollapsibleStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = [
            ["TSLA.US", "$423.00", "+30.25%"],
            ["NVDA.US", "$312.00", "+12.12%"],
            ["AAPL.US", "$145.00", "-8.50%"],
        ];

        v_flex()
            .gap_6()
            .child(
                section("Expland Paragraphs").v_flex().child(
                    Collapsible::new()
                        .max_w_128()
                        .gap_1()
                        .open(self.item1_open)
                        .child(
                            "This is a collapsible component. \
            Click the header to expand or collapse the content.",
                        )
                        .content(
                            "This is the full content of the Collapsible component. \
                        It is only visible when the component is expanded. \n\
                        You can put any content you like here, including text, images, \
                        or other UI elements.
                        ",
                        )
                        .child(
                            h_flex().justify_center().child(
                                Button::new("toggle1")
                                    .icon(IconName::ChevronDown)
                                    .label("Show more")
                                    .when(self.item1_open, |this| {
                                        this.icon(IconName::ChevronUp).label("Show less")
                                    })
                                    .xsmall()
                                    .link()
                                    .on_click({
                                        cx.listener(move |this, _, _, cx| {
                                            this.item1_open = !this.item1_open;
                                            cx.notify();
                                        })
                                    }),
                            ),
                        ),
                ),
            )
            .child(
                section("Card").child(
                    GroupBox::new()
                        .outline()
                        .w_80()
                        .title("Collapsible in a Card")
                        .child(
                            Collapsible::new()
                                .gap_1()
                                .open(self.item2_open)
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .child(
                                            v_flex().child("Total Return").child(
                                                h_flex()
                                                    .gap_1()
                                                    .child(
                                                        Label::new("123.5%")
                                                            .text_2xl()
                                                            .font_semibold(),
                                                    )
                                                    .child(
                                                        Tag::info()
                                                            .child("+4.5%")
                                                            .outline()
                                                            .rounded_full()
                                                            .small(),
                                                    ),
                                            ),
                                        )
                                        .child(
                                            Button::new("toggle2")
                                                .small()
                                                .outline()
                                                .icon(IconName::ChevronDown)
                                                .label("Details")
                                                .when(self.item2_open, |this| {
                                                    this.icon(IconName::ChevronUp)
                                                })
                                                .on_click({
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.item2_open = !this.item2_open;
                                                        cx.notify();
                                                    })
                                                }),
                                        ),
                                )
                                .content(v_flex().gap_2().children(items.iter().map(|item| {
                                    let is_up = item[2].starts_with('+');

                                    h_flex().justify_between().child(item[0]).child(
                                        h_flex()
                                            .flex_1()
                                            .justify_end()
                                            .gap_4()
                                            .child(div().w_16().justify_end().child(item[1]))
                                            .child(
                                                Label::new(item[2])
                                                    .text_xs()
                                                    .w_16()
                                                    .justify_end()
                                                    .when(is_up, |this| {
                                                        this.text_color(cx.theme().green)
                                                    })
                                                    .when(!is_up, |this| {
                                                        this.text_color(cx.theme().red)
                                                    }),
                                            ),
                                    )
                                }))),
                        ),
                ),
            )
    }
}
