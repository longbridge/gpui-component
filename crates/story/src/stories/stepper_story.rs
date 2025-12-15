use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div,
};
use gpui_component::{
    StyledExt,
    checkbox::Checkbox,
    h_flex,
    stepper::{Stepper, StepperItem},
    text::markdown,
    v_flex,
};

use crate::section;

pub struct StepperStory {
    focus_handle: gpui::FocusHandle,
    stepper1_step: usize,
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
            stepper1_step: 1,
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
                h_flex().child(
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
                        .w_112()
                        .disabled(self.disabled)
                        .step(self.stepper1_step)
                        .items([
                            StepperItem::new()
                                .label("Step 1")
                                .description(div().child("This is the description for step 1.")),
                            StepperItem::new()
                                .label("Step 2")
                                .description("This is description 2."),
                            StepperItem::new().label("Step 3").description(markdown(
                                "**This is description 3.** You can use _Markdown_ here!",
                            )),
                        ])
                        .on_click(cx.listener(|this, step, _, cx| {
                            this.stepper1_step = *step;
                            cx.notify();
                        })),
                ),
            )
    }
}
