use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Subscription, Window,
};
use gpui_component::{
    IconName, Selectable as _, Sizable, Size, StyledExt,
    button::{Button, ButtonGroup},
    checkbox::Checkbox,
    h_flex,
    stepper::{Stepper, StepperItem},
    v_flex,
};

use crate::section;

pub struct StepperStory {
    focus_handle: gpui::FocusHandle,
    size: Size,
    stepper0_step: usize,
    stepper1_step: usize,
    stepper2_step: usize,
    stepper3_step: usize,
    disabled: bool,
    _subscritions: Vec<Subscription>,
}

impl super::Story for StepperStory {
    fn title() -> &'static str {
        "Stepper"
    }

    fn description() -> &'static str {
        "A step-by-step process for users to navigate through a series of steps."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl StepperStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            size: Size::default(),
            stepper0_step: 1,
            stepper1_step: 0,
            stepper2_step: 2,
            stepper3_step: 0,
            disabled: false,
            _subscritions: vec![],
        }
    }
}

impl Focusable for StepperStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StepperStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_y_3()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        ButtonGroup::new("toggle-size")
                            .outline()
                            .compact()
                            .child(
                                Button::new("xsmall")
                                    .label("XSmall")
                                    .selected(self.size == Size::XSmall),
                            )
                            .child(
                                Button::new("small")
                                    .label("Small")
                                    .selected(self.size == Size::Small),
                            )
                            .child(
                                Button::new("medium")
                                    .label("Medium")
                                    .selected(self.size == Size::Medium),
                            )
                            .child(
                                Button::new("large")
                                    .label("Large")
                                    .selected(self.size == Size::Large),
                            )
                            .on_click(cx.listener(|this, selecteds: &Vec<usize>, _, cx| {
                                let size = match selecteds[0] {
                                    0 => Size::XSmall,
                                    1 => Size::Small,
                                    2 => Size::Medium,
                                    3 => Size::Large,
                                    _ => unreachable!(),
                                };
                                this.size = size;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("disabled")
                            .checked(self.disabled)
                            .label("Disabled")
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.disabled = *check;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Horizontal Stepper").max_w_md().v_flex().child(
                    Stepper::new("stepper0")
                        .w_full()
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper0_step)
                        .items([
                            StepperItem::new().child("Step 1"),
                            StepperItem::new().child("Step 2"),
                            StepperItem::new().child("Step 3"),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper0_step = *step;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Icon Stepper").max_w_md().v_flex().child(
                    Stepper::new("stepper1")
                        .w_full()
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper1_step)
                        .items([
                            StepperItem::new()
                                .icon(IconName::Calendar)
                                .child("Order Details"),
                            StepperItem::new().icon(IconName::Inbox).child("Shipping"),
                            StepperItem::new().icon(IconName::Frame).child("Preview"),
                            StepperItem::new().icon(IconName::Info).child("Finish"),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper1_step = *step;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Vertical Stepper").max_w_md().v_flex().child(
                    Stepper::new("stepper3")
                        .vertical()
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper2_step)
                        .items_center()
                        .items([
                            StepperItem::new()
                                .pb_8()
                                .icon(IconName::Building2)
                                .child(v_flex().child("Step 1").child("Description for step 1.")),
                            StepperItem::new()
                                .pb_8()
                                .icon(IconName::Asterisk)
                                .child(v_flex().child("Step 2").child("Description for step 2.")),
                            StepperItem::new()
                                .pb_8()
                                .icon(IconName::Folder)
                                .child(v_flex().child("Step 3").child("Description for step 3.")),
                            StepperItem::new()
                                .icon(IconName::CircleCheck)
                                .child(v_flex().child("Step 4").child("Description for step 4.")),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper2_step = *step;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Text Center").max_w_md().v_flex().child(
                    Stepper::new("stepper4")
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .selected_index(self.stepper3_step)
                        .text_center(true)
                        .items([
                            StepperItem::new().child(
                                v_flex()
                                    .items_center()
                                    .child("Step 1")
                                    .child("Desc for step 1."),
                            ),
                            StepperItem::new().child(
                                v_flex()
                                    .items_center()
                                    .child("Step 2")
                                    .child("Desc for step 2."),
                            ),
                            StepperItem::new().child(
                                v_flex()
                                    .items_center()
                                    .child("Step 3")
                                    .child("Desc for step 3."),
                            ),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper3_step = *step;
                            cx.notify();
                        })),
                ),
            )
    }
}
