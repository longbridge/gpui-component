use gpui::{
    div, px, IntoElement, ParentElement, Render, Styled, Subscription, View, ViewContext,
    VisualContext, WindowContext,
};
use ui::{
    button::Button,
    divider::Divider,
    h_flex,
    indicator::Indicator,
    progress::Progress,
    skeleton::Skeleton,
    slider::{Slider, SliderEvent},
    v_flex, IconName, Sizable,
};

pub struct ProgressStory {
    focus_handle: gpui::FocusHandle,
    value: f32,
    slider1: View<Slider>,
    slider1_value: f32,
    slider2: View<Slider>,
    slider2_value: f32,
    _subscritions: Vec<Subscription>,
}

impl super::Story for ProgressStory {
    fn title() -> &'static str {
        "Progress"
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl ProgressStory {
    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }

    fn new(cx: &mut ViewContext<Self>) -> Self {
        let slider1 = cx.new_view(|_| {
            Slider::horizontal()
                .min(-255.)
                .max(255.)
                .default_value(15.)
                .step(15.)
        });

        let slider2 = cx.new_view(|_| Slider::horizontal().min(0.).max(5.).step(1.0));

        let _subscritions = vec![
            cx.subscribe(&slider1, |this, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(value) => {
                    this.slider1_value = *value;
                    cx.notify();
                }
            }),
            cx.subscribe(&slider2, |this, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(value) => {
                    this.slider2_value = *value;
                    cx.notify();
                }
            }),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            value: 50.,
            slider1_value: 15.,
            slider2_value: 1.,
            slider1,
            slider2,
            _subscritions,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

impl gpui::FocusableView for ProgressStory {
    fn focus_handle(&self, _: &gpui::AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProgressStory {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .items_center()
            .gap_y_3()
            .child(
                h_flex()
                    .gap_x_2()
                    .child(Button::new("button-1").label("0%").on_click(cx.listener(
                        |this, _, _| {
                            this.set_value(0.);
                        },
                    )))
                    .child(Button::new("button-2").label("25%").on_click(cx.listener(
                        |this, _, _| {
                            this.set_value(25.);
                        },
                    )))
                    .child(Button::new("button-3").label("75%").on_click(cx.listener(
                        |this, _, _| {
                            this.set_value(75.);
                        },
                    )))
                    .child(Button::new("button-4").label("100%").on_click(cx.listener(
                        |this, _, _| {
                            this.set_value(100.);
                        },
                    ))),
            )
            .child(div().w_1_2().child(Progress::new().value(self.value)))
            .child(
                h_flex()
                    .gap_x_2()
                    .child(
                        Button::new("button-5")
                            .icon(IconName::Minus)
                            .on_click(cx.listener(|this, _, _| {
                                this.set_value((this.value - 1.).max(0.));
                            })),
                    )
                    .child(
                        Button::new("button-6")
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|this, _, _| {
                                this.set_value((this.value + 1.).min(100.));
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_x_2()
                    .child(Indicator::new().xsmall())
                    .child(Indicator::new().small())
                    .child(Indicator::new())
                    .child(
                        Indicator::new()
                            .large()
                            .icon(IconName::LoaderCircle)
                            .color(ui::blue_500()),
                    )
                    .child(Indicator::new().with_size(px(64.))),
            )
            .child(
                Divider::horizontal()
                    .mt_10()
                    .label("Slider")
                    .color(ui::gray_300()),
            )
            .child(self.slider1.clone())
            .child(format!("Slider 1: {}", self.slider1_value))
            .child(
                v_flex()
                    .gap_3()
                    .w(px(200.))
                    .child(self.slider2.clone())
                    .child(format!("Slider 2: {}", self.slider2_value)),
            )
            .child(
                h_flex()
                    .mt_5()
                    .gap_4()
                    .child(Skeleton::new().size_12().rounded_full())
                    .child(
                        v_flex()
                            .gap_2()
                            .child(Skeleton::new().w(px(250.)).h_4())
                            .child(Skeleton::new().w(px(240.)).h_4()),
                    ),
            )
    }
}
