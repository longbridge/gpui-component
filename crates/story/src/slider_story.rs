use gpui::{
    hsla, App, AppContext, Context, Entity, Focusable, Hsla, IntoElement, ParentElement, Render,
    SharedString, Styled, Subscription, Window,
};
use gpui_component::{
    clipboard::Clipboard,
    h_flex,
    slider::{Slider, SliderEvent},
    v_flex, Colorize as _, ContextModal, StyledExt,
};

use crate::section;

pub struct SliderStory {
    focus_handle: gpui::FocusHandle,
    slider1: Entity<Slider>,
    slider1_value: f32,
    slider2: Entity<Slider>,
    slider2_value: f32,
    slider_hsl: [Entity<Slider>; 4],
    slider_hsl_value: Hsla,
    _subscritions: Vec<Subscription>,
}

impl super::Story for SliderStory {
    fn title() -> &'static str {
        "Slider"
    }

    fn description() -> &'static str {
        "Displays a slider control for selecting a value within a range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl SliderStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let slider1 = cx.new(|_| {
            Slider::horizontal()
                .min(-255.)
                .max(255.)
                .default_value(15.)
                .step(15.)
        });

        let slider2 = cx.new(|_| Slider::horizontal().min(0.).max(5.).step(1.0));
        let slider_hsl = [
            cx.new(|_| {
                Slider::vertical()
                    .reverse()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.38)
            }),
            cx.new(|_| {
                Slider::vertical()
                    .reverse()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
            cx.new(|_| {
                Slider::vertical()
                    .reverse()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
            cx.new(|_| {
                Slider::vertical()
                    .reverse()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(1.)
            }),
        ];

        let mut _subscritions = vec![
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

        _subscritions.extend(
            slider_hsl
                .iter()
                .map(|slider| {
                    cx.subscribe(slider, |this, _, event: &SliderEvent, cx| match event {
                        SliderEvent::Change(_) => {
                            this.slider_hsl_value = hsla(
                                this.slider_hsl[0].read(cx).value(),
                                this.slider_hsl[1].read(cx).value(),
                                this.slider_hsl[2].read(cx).value(),
                                this.slider_hsl[3].read(cx).value(),
                            );
                            cx.notify();
                        }
                    })
                })
                .collect::<Vec<_>>(),
        );

        slider_hsl[0].update(cx, |slider, cx| {
            cx.emit(SliderEvent::Change(slider.value()));
        });

        Self {
            focus_handle: cx.focus_handle(),
            slider1_value: 0.,
            slider2_value: 0.,
            slider1,
            slider2,
            slider_hsl,
            slider_hsl_value: gpui::red(),
            _subscritions,
        }
    }
}

impl Focusable for SliderStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SliderStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let rgb = SharedString::from(self.slider_hsl_value.to_hex());

        v_flex()
            .items_center()
            .gap_y_3()
            .child(
                section("Horizontal Slider")
                    .max_w_md()
                    .v_flex()
                    .child(self.slider1.clone())
                    .child(format!("Value: {}", self.slider1_value)),
            )
            .child(
                section("Slider (0 - 5)")
                    .max_w_md()
                    .v_flex()
                    .child(self.slider2.clone())
                    .child(format!("Value: {}", self.slider2_value)),
            )
            .child(
                section(
                    h_flex()
                        .gap_2()
                        .justify_between()
                        .child("Color Picker")
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    h_flex()
                                        .text_color(self.slider_hsl_value)
                                        .child(rgb.clone()),
                                )
                                .child(Clipboard::new("copy-hsl").value(rgb).on_copied(
                                    |_, window, cx| {
                                        window.push_notification("Color copied to clipboard.", cx)
                                    },
                                )),
                        ),
                )
                .max_w_md()
                .justify_around()
                .child(
                    v_flex()
                        .h_32()
                        .gap_3()
                        .items_center()
                        .justify_center()
                        .child(self.slider_hsl[0].clone())
                        .child(
                            v_flex()
                                .items_center()
                                .child("Hue")
                                .child(format!("{:.0}", self.slider_hsl_value.h * 360.)),
                        ),
                )
                .child(
                    v_flex()
                        .h_32()
                        .gap_3()
                        .items_center()
                        .justify_center()
                        .child(self.slider_hsl[1].clone())
                        .child(
                            v_flex()
                                .items_center()
                                .child("Saturation")
                                .child(format!("{:.0}", self.slider_hsl_value.s * 100.)),
                        ),
                )
                .child(
                    v_flex()
                        .h_32()
                        .gap_3()
                        .items_center()
                        .justify_center()
                        .child(self.slider_hsl[2].clone())
                        .child(
                            v_flex()
                                .items_center()
                                .child("Lightness")
                                .child(format!("{:.0}", self.slider_hsl_value.l * 100.)),
                        ),
                )
                .child(
                    v_flex()
                        .h_32()
                        .gap_3()
                        .items_center()
                        .justify_center()
                        .child(self.slider_hsl[3].clone())
                        .child(
                            v_flex()
                                .items_center()
                                .child("Alpha")
                                .child(format!("{:.0}", self.slider_hsl_value.a * 100.)),
                        ),
                ),
            )
    }
}
