use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, button::Button, h_flex, progress::{Progress, ProgressCircle}, v_flex,
};

use crate::section;

pub struct ProgressStory {
    focus_handle: gpui::FocusHandle,
    value: f32,
}

impl super::Story for ProgressStory {
    fn title() -> &'static str {
        "Progress"
    }

    fn description() -> &'static str {
        "Displays an indicator showing the completion progress of a task, typically displayed as a progress bar."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ProgressStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: 25.,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

impl Focusable for ProgressStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProgressStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .items_center()
            .gap_y_3()
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .justify_center()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("button-1").small().label("0%").on_click(
                                cx.listener(|this, _, _, _| {
                                    this.set_value(0.);
                                }),
                            ))
                            .child(Button::new("button-2").small().label("25%").on_click(
                                cx.listener(|this, _, _, _| {
                                    this.set_value(25.);
                                }),
                            ))
                            .child(Button::new("button-3").small().label("75%").on_click(
                                cx.listener(|this, _, _, _| {
                                    this.set_value(75.);
                                }),
                            ))
                            .child(Button::new("button-4").small().label("100%").on_click(
                                cx.listener(|this, _, _, _| {
                                    this.set_value(100.);
                                }),
                            )),
                    )
                    .child(
                        h_flex()
                            .gap_x_2()
                            .child(
                                Button::new("circle-button-5")
                                    .icon(IconName::Minus)
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.set_value((this.value - 1.).max(0.));
                                    })),
                            )
                            .child(
                                Button::new("circle-button-6")
                                    .icon(IconName::Plus)
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.set_value((this.value + 1.).min(100.));
                                    })),
                            ),
                    ),
            )
            .child(
                section("Progress Bar")
                    .max_w_md()
                    .child(Progress::new("progress-1").value(self.value)),
            )
            .child(
                section("Custom Style").max_w_md().child(
                    Progress::new("progress-2")
                        .value(32.)
                        .h(px(16.))
                        .rounded(px(2.))
                        .color(cx.theme().green_light)
                        .border_2()
                        .border_color(cx.theme().green),
                ),
            )
            .child(
                section("Circle Progress").max_w_md().child(
                    ProgressCircle::new("circle-progress-1")
                        .value(self.value)
                        .size_16(),
                ),
            )
            .child(
                section("With size").max_w_md().child(
                    h_flex()
                        .gap_2()
                        .child(
                            ProgressCircle::new("circle-progress-1")
                                .value(self.value)
                                .large(),
                        )
                        .child(
                            ProgressCircle::new("circle-progress-1")
                                .value(self.value),
                        )
                        .child(
                            ProgressCircle::new("circle-progress-1")
                                .value(self.value)
                                .small(),
                        )
                        .child(
                            ProgressCircle::new("circle-progress-1")
                                .value(self.value)
                                .xsmall(),
                        ),
                ),
            )
            .child(
                section("With Label").max_w_md().child(
                    h_flex()
                        .gap_2()
                        .child(
                            ProgressCircle::new("circle-progress-1")
                                .color(cx.theme().primary)
                                .value(self.value)
                                .size_4(),
                        )
                        .child(format!("Downloading... {}%", self.value as u8)),
                ),
            )
            .child(
                section("Circle with Color").max_w_md().child(
                    ProgressCircle::new("circle-progress-1")
                        .color(cx.theme().yellow)
                        .value(self.value)
                        .size_12(),
                ),
            )
    }
}
