//! Controls choosing a number by position: Slider.

use gpui_kit::{
    AppContext as _, Axis, Entity, IntoElement as _, ParentElement as _, Styled as _, Subscription,
    base::{
        Slider, SliderIndicator, SliderThumb, SliderTrack,
        slider::{SliderEvent, SliderState},
    },
    div,
    prelude::FluentBuilder as _,
    relative,
};
use serde_json::{Value, json};

use crate::{Control, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![slider()]
}

/// Prop `name`, or Base's default where the app set none.
/// The interaction-state class props of the thumb, which never takes focus.
const THUMB_STATES: [&str; 3] = ["thumbHoverClass", "thumbActiveClass", ""];

fn number_or(props: &Props, name: &str, default: f32) -> f32 {
    if props.has(name) {
        props.number(name) as f32
    } else {
        default
    }
}

struct SliderControl {
    state: Entity<SliderState>,
    _subscription: Subscription,
}

/// A slider value as JS reads it: `0.3` for a step of `0.1`, not the
/// widened `f32`.
fn number(value: f32) -> Value {
    json!(value.to_string().parse::<f64>().unwrap_or(f64::from(value)))
}

fn slider() -> Control {
    Control::stateful(
        "Slider",
        |props, events, _, cx| {
            let state = cx.new(|_| {
                let defaults = SliderState::new();
                SliderState::new()
                    .min(number_or(props, "min", defaults.min_value()))
                    .max(number_or(props, "max", defaults.max_value()))
                    .step(number_or(props, "step", defaults.step_value()))
                    .default_value(number_or(props, "value", defaults.value().end()))
            });
            let events = events.clone();
            let subscription = cx.subscribe(&state, move |_, event: &SliderEvent, _| {
                let (name, value) = match event {
                    SliderEvent::Change(value) => ("change", value),
                    SliderEvent::Release(value) => ("release", value),
                };
                events.emit(name, json!({ "value": number(value.end()) }));
            });
            SliderControl {
                state,
                _subscription: subscription,
            }
        },
        |slider, props, _, window, cx| {
            let changed = |names: &[&str]| names.iter().any(|name| props.changed(name));
            if changed(&["min", "max", "step", "value"]) {
                let defaults = SliderState::new();
                slider.state.update(cx, |state, cx| {
                    if changed(&["min", "max", "step"]) {
                        *state = std::mem::replace(state, SliderState::new())
                            .min(number_or(props, "min", defaults.min_value()))
                            .max(number_or(props, "max", defaults.max_value()))
                            .step(number_or(props, "step", defaults.step_value()));
                    }
                    if props.changed("value") {
                        let value = number_or(props, "value", defaults.value().end());
                        state.set_value(value, window, cx);
                    }
                    cx.notify();
                });
            }
            let state = &slider.state;
            let axis = if props.boolean("vertical") {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            let disabled = props.boolean("disabled");
            let percentage = state.read(cx).percentage();
            props
                .styled(Slider::new(state), cx)
                .axis(axis)
                .disabled(disabled)
                .when_some(props.string("accessibilityLabel"), |this, label| {
                    this.accessibility_label(label)
                })
                .child(
                    props
                        .styled_by("trackClass", SliderTrack::new(state), cx)
                        .axis(axis)
                        .disabled(disabled)
                        .child(
                            props
                                .styled_by(
                                    "indicatorClass",
                                    SliderIndicator::new(state).relative(),
                                    cx,
                                )
                                .child(props.styled_by(
                                    "rangeClass",
                                    div().absolute().map(|this| {
                                        match axis {
                                            Axis::Horizontal => this
                                                .top_0()
                                                .bottom_0()
                                                .left(relative(percentage.start))
                                                .right(relative(1. - percentage.end)),
                                            Axis::Vertical => this
                                                .left_0()
                                                .right_0()
                                                .bottom(relative(percentage.start))
                                                .top(relative(1. - percentage.end)),
                                        }
                                    }),
                                    cx,
                                ))
                                // A zero-size anchor at the value centers the
                                // thumb on it, whatever size the app gives it.
                                .child(
                                    div()
                                        .absolute()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .map(|this| match axis {
                                            Axis::Horizontal => this
                                                .top_0()
                                                .bottom_0()
                                                .w_0()
                                                .left(relative(percentage.end)),
                                            Axis::Vertical => this
                                                .left_0()
                                                .right_0()
                                                .h_0()
                                                .bottom(relative(percentage.end)),
                                        })
                                        .child(
                                            props.states(
                                                THUMB_STATES,
                                                props.styled_by(
                                                    "thumbClass",
                                                    SliderThumb::new(state)
                                                        .axis(axis)
                                                        .disabled(disabled)
                                                        .flex_none(),
                                                    cx,
                                                ),
                                                cx,
                                            ),
                                        ),
                                ),
                        ),
                )
                .into_any_element()
        },
    )
    .documentation(
        "GPUI Base's Slider: a value from `min` to `max` in steps of `step`, set by clicking or \
         dragging the track. `change` reports each value while it moves, `release` the value \
         when the pointer lets go.",
    )
    .prop("value", PropType::Number)
    .prop("min", PropType::Number)
    .prop("max", PropType::Number)
    .prop("step", PropType::Number)
    .prop("vertical", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("trackClass", PropType::String)
    .prop("indicatorClass", PropType::String)
    .prop("rangeClass", PropType::String)
    .prop("thumbClass", PropType::String)
    .states(THUMB_STATES)
    .event("change", &[("value", PropType::Number)])
    .event("release", &[("value", PropType::Number)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{ElementId, Role, TestAppContext, Window, point, px, test::ElementSnapshot};
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    const TRACK: &str = "slider-bar-container";

    fn thumb() -> ElementId {
        ("slider-thumb", 0u32).into()
    }

    /// A 192px wide horizontal slider from 0 to 100, at `value`.
    fn props(value: f64) -> Value {
        json!({
            "value": value,
            "rootClass": "w_full",
            "trackClass": "w_48 h_5 flex items_center",
            "indicatorClass": "w_full h_1 rounded_full bg_muted",
            "rangeClass": "rounded_full bg_primary",
            "thumbClass": "size_4 rounded_full bg_background border_1 border_primary",
        })
    }

    fn slider(cx: &mut TestAppContext, value: f64) -> Fixture {
        Fixture::new(cx, "Slider", props(value))
    }

    /// The Slider's root, which Base identifies by its state's entity.
    fn root(window: &Window) -> ElementSnapshot {
        let id = window
            .find(TRACK)
            .path()
            .iter()
            .find(|id| matches!(id, ElementId::NamedInteger(name, _) if name == "slider"))
            .cloned()
            .expect("the track is inside the slider");
        window.find(id)
    }

    /// The thumb's center, as a fraction of the track's width.
    fn thumb_fraction(slider: &Fixture, cx: &mut TestAppContext) -> f32 {
        slider.update(cx, |window, _| {
            let track = window.find(TRACK).bounds();
            (window.find(thumb()).bounds().center().x - track.left()) / track.size.width
        })
    }

    #[gpui_kit::test]
    fn test_slider_builder(cx: &mut TestAppContext) {
        let mut props = props(30.);
        props["min"] = json!(10);
        props["max"] = json!(50);
        props["step"] = json!(5);
        props["accessibilityLabel"] = json!("Volume");
        let slider = Fixture::new(cx, "Slider", props);
        let size = slider.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        slider.update(cx, |window, _| {
            let root = root(window);
            assert_eq!(root.role(), Some(Role::Slider));
            assert_eq!(root.label(), Some("Volume"));
        });
        assert!((thumb_fraction(&slider, cx) - 0.5).abs() < 0.02);
        assert!(slider.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_slider_click_track(cx: &mut TestAppContext) {
        let slider = slider(cx, 0.);
        slider.update(cx, |window, cx| {
            let track = window.find(TRACK).bounds();
            window.click_at(
                TRACK,
                point(track.size.width * 0.75, track.size.height / 2.),
                cx,
            );
        });
        assert_eq!(
            slider.take_events(),
            vec![
                ("change".into(), json!({ "value": 75.0 })),
                ("release".into(), json!({ "value": 75.0 })),
            ]
        );
        assert!((thumb_fraction(&slider, cx) - 0.75).abs() < 0.02);
    }

    #[gpui_kit::test]
    fn test_slider_drag_thumb(cx: &mut TestAppContext) {
        let slider = slider(cx, 50.);
        slider.update(cx, |window, cx| {
            let track = window.find(TRACK).bounds();
            let from = window.find(thumb()).bounds().center();
            window.drag(
                from,
                point(track.left() + track.size.width * 0.2, from.y),
                cx,
            );
        });
        let events = slider.take_events();
        assert!(events.len() > 2, "{events:?}");
        assert!(
            events[..events.len() - 1]
                .iter()
                .all(|(name, _)| name == "change")
        );
        assert_eq!(
            events.last(),
            Some(&("release".into(), json!({ "value": 20.0 })))
        );
        assert_eq!(events[events.len() - 2].1, json!({ "value": 20.0 }));
        assert!((thumb_fraction(&slider, cx) - 0.2).abs() < 0.02);
    }

    #[gpui_kit::test]
    fn test_slider_step(cx: &mut TestAppContext) {
        let slider = slider(cx, 0.);
        slider.set_prop(cx, "max", Some(json!(1)));
        slider.set_prop(cx, "step", Some(json!(0.1)));
        slider.update(cx, |window, cx| {
            let track = window.find(TRACK).bounds();
            window.click_at(
                TRACK,
                point(track.size.width * 0.32, track.size.height / 2.),
                cx,
            );
        });
        assert_eq!(
            slider.take_events()[0],
            ("change".into(), json!({ "value": 0.3 }))
        );
    }

    #[gpui_kit::test]
    fn test_slider_controlled(cx: &mut TestAppContext) {
        let slider = slider(cx, 10.);
        assert!((thumb_fraction(&slider, cx) - 0.1).abs() < 0.02);
        slider.set_prop(cx, "value", Some(json!(60)));
        assert!((thumb_fraction(&slider, cx) - 0.6).abs() < 0.02);
        slider.set_prop(cx, "max", Some(json!(200)));
        assert!((thumb_fraction(&slider, cx) - 0.3).abs() < 0.02);
        assert!(slider.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_slider_vertical(cx: &mut TestAppContext) {
        let mut props = props(25.);
        props["vertical"] = json!(true);
        props["trackClass"] = json!("h_48 w_5 flex justify_center");
        props["indicatorClass"] = json!("h_full w_1 bg_muted");
        let slider = Fixture::new(cx, "Slider", props);
        let fraction = slider.update(cx, |window, _| {
            let track = window.find(TRACK).bounds();
            (track.bottom() - window.find(thumb()).bounds().center().y) / track.size.height
        });
        assert!((fraction - 0.25).abs() < 0.02);
        slider.update(cx, |window, cx| {
            let track = window.find(TRACK).bounds();
            window.click_at(
                TRACK,
                point(track.size.width / 2., track.size.height * 0.25),
                cx,
            );
        });
        assert_eq!(
            slider.take_events()[0],
            ("change".into(), json!({ "value": 75.0 }))
        );
    }

    #[gpui_kit::test]
    fn test_slider_disabled(cx: &mut TestAppContext) {
        let slider = slider(cx, 0.);
        slider.set_prop(cx, "disabled", Some(json!(true)));
        let click = |slider: &Fixture, cx: &mut TestAppContext| {
            slider.update(cx, |window, cx| {
                let track = window.find(TRACK).bounds();
                window.click_at(
                    TRACK,
                    point(track.size.width * 0.5, track.size.height / 2.),
                    cx,
                );
            })
        };
        click(&slider, cx);
        assert!(slider.take_events().is_empty());
        slider.update(cx, |window, _| {
            assert_eq!(root(window).disabled(), Some(true));
        });
        slider.set_prop(cx, "disabled", None);
        click(&slider, cx);
        assert_eq!(slider.take_events().len(), 2);
    }
}
