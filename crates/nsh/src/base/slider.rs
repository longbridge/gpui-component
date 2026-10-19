//! Controls choosing a number by position: Slider.

use gpui_kit::{
    AppContext as _, Axis, Entity, FocusHandle, IntoElement as _, ParentElement as _, Styled as _,
    Subscription,
    base::{
        Slider, SliderIndicator, SliderThumb, SliderTrack,
        slider::{SliderEvent, SliderState, SliderValue},
    },
    div,
    prelude::FluentBuilder as _,
    relative,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Control, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![slider()]
}

/// The interaction-state class props of the root, which shows the keyboard
/// focus; its thumbs show the pointer's.
const ROOT_STATES: [&str; 3] = ["", "", "focusClass"];

/// The interaction-state class props of a thumb, which never takes focus
/// itself (`thumbFocusClass` marks the thumb the keys move).
const THUMB_STATES: [&str; 3] = ["thumbHoverClass", "thumbActiveClass", ""];

/// The TypeScript type of `value`: one number, or a range's two ends.
const VALUE: &str = "number | [number, number]";

/// Prop `name`, or Base's default where the app set none.
fn number_or(props: &Props, name: &str, default: f32) -> f32 {
    if props.has(name) {
        props.number(name) as f32
    } else {
        default
    }
}

/// The `value` prop as JS writes it.
#[derive(Deserialize)]
#[serde(untagged)]
enum ValueProp {
    Single(f32),
    Range(f32, f32),
}

/// The `value` prop as Base's value, or Base's default where the app set
/// none.
fn value(props: &Props) -> SliderValue {
    match props.json::<ValueProp>("value") {
        Some(ValueProp::Single(value)) => SliderValue::Single(value),
        Some(ValueProp::Range(start, end)) => SliderValue::Range(start, end),
        None => SliderValue::default(),
    }
}

struct SliderControl {
    state: Entity<SliderState>,
    focus: FocusHandle,
    _subscription: Subscription,
}

/// A slider number as JS reads it: `0.3` for a step of `0.1`, not the
/// widened `f32`.
fn number(value: f32) -> Value {
    json!(value.to_string().parse::<f64>().unwrap_or(f64::from(value)))
}

/// A slider value as JS reads it, in the shape of the `value` prop.
fn value_json(value: &SliderValue) -> Value {
    match *value {
        SliderValue::Single(value) => number(value),
        SliderValue::Range(start, end) => json!([number(start), number(end)]),
    }
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
                    .default_value(value(props))
            });
            let events = events.clone();
            let subscription = cx.subscribe(&state, move |_, event: &SliderEvent, _| {
                let (name, value) = match event {
                    SliderEvent::Change(value) => ("change", value),
                    SliderEvent::Release(value) => ("release", value),
                };
                events.emit(name, json!({ "value": value_json(value) }));
            });
            SliderControl {
                state,
                focus: cx.focus_handle(),
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
                        state.set_value(value(props), window, cx);
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
            let (is_range, percentage, keyboard_on_start) = {
                let state = state.read(cx);
                (
                    state.value().is_range(),
                    state.percentage(),
                    state.is_keyboard_on_start(),
                )
            };
            let focused = !disabled && slider.focus.is_focused(window);
            // A zero-size anchor at `position` centers a thumb on it, whatever
            // size the app gives the thumb.
            let thumb = |position: f32, start: bool| {
                div()
                    .absolute()
                    .flex()
                    .items_center()
                    .justify_center()
                    .map(|this| match axis {
                        Axis::Horizontal => this.top_0().bottom_0().w_0().left(relative(position)),
                        Axis::Vertical => this.left_0().right_0().h_0().bottom(relative(position)),
                    })
                    .child(
                        props.states(
                            THUMB_STATES,
                            props
                                .styled_by(
                                    "thumbClass",
                                    SliderThumb::new(state)
                                        .axis(axis)
                                        .start(start)
                                        .disabled(disabled)
                                        .flex_none(),
                                    cx,
                                )
                                .when(focused && start == keyboard_on_start, |this| {
                                    props.styled_by("thumbFocusClass", this, cx)
                                }),
                            cx,
                        ),
                    )
            };
            props
                .states(ROOT_STATES, props.styled(Slider::new(state), cx), cx)
                .track_focus(&slider.focus)
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
                                .when(is_range, |this| this.child(thumb(percentage.start, true)))
                                .child(thumb(percentage.end, false)),
                        ),
                )
                .into_any_element()
        },
    )
    .focus(|slider: &SliderControl, _| slider.focus.clone())
    .documentation(
        "GPUI Base's Slider: a value from `min` to `max` in steps of `step`, set by clicking or \
         dragging the track, or with the keyboard while focused: the arrow keys move a step, \
         Page Up and Page Down ten, Home and End to `min` and `max`. A `[start, end]` value is \
         a range with two thumbs; the keys move the thumb the pointer moved last (the end one \
         at first). `change` reports each value while it moves, `release` the value when the \
         pointer lets go or a key is pressed; both carry `value` in the shape the app gave it.",
    )
    .prop("value", PropType::Json(VALUE))
    .prop("min", PropType::Number)
    .prop("max", PropType::Number)
    .prop("step", PropType::Number)
    .prop("vertical", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .states(ROOT_STATES)
    .prop("trackClass", PropType::String)
    .prop("indicatorClass", PropType::String)
    .prop("rangeClass", PropType::String)
    .prop("thumbClass", PropType::String)
    .states(THUMB_STATES)
    .prop("thumbFocusClass", PropType::String)
    .event("change", &[("value", PropType::Json(VALUE))])
    .event("release", &[("value", PropType::Json(VALUE))])
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

    /// The thumb of a range's start.
    fn start_thumb() -> ElementId {
        ("slider-thumb", 1u32).into()
    }

    /// A 192px wide horizontal slider from 0 to 100, at `value`.
    fn props(value: f64) -> Value {
        props_with(json!(value))
    }

    /// [`props`] with a value of either shape.
    fn props_with(value: Value) -> Value {
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

    /// The center of `thumb`, as a fraction of the track's width.
    fn fraction_of(slider: &Fixture, cx: &mut TestAppContext, thumb: ElementId) -> f32 {
        slider.update(cx, |window, _| {
            let track = window.find(TRACK).bounds();
            (window.find(thumb).bounds().center().x - track.left()) / track.size.width
        })
    }

    /// The thumb's center, as a fraction of the track's width.
    fn thumb_fraction(slider: &Fixture, cx: &mut TestAppContext) -> f32 {
        fraction_of(slider, cx, thumb())
    }

    /// Clicks the track at `fraction` of its width.
    fn click_track(slider: &Fixture, cx: &mut TestAppContext, fraction: f32) {
        slider.update(cx, |window, cx| {
            let track = window.find(TRACK).bounds();
            window.click_at(
                TRACK,
                point(track.size.width * fraction, track.size.height / 2.),
                cx,
            );
        });
    }

    /// A key press is one whole change: `change`, then `release`.
    fn pressed(value: Value) -> Vec<(String, Value)> {
        vec![
            ("change".into(), json!({ "value": value })),
            ("release".into(), json!({ "value": value })),
        ]
    }

    #[gpui_kit::test]
    fn test_slider_builder(cx: &mut TestAppContext) {
        let mut props = props(30.);
        props["min"] = json!(10);
        props["max"] = json!(50);
        props["step"] = json!(5);
        props["accessibilityLabel"] = json!("Volume");
        props["focusClass"] = json!("rounded_md");
        props["thumbHoverClass"] = json!("bg_accent");
        props["thumbActiveClass"] = json!("bg_accent");
        props["thumbFocusClass"] = json!("border_ring");
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
        assert!(
            slider.focus_handle(cx).is_none(),
            "a disabled slider is no Tab stop"
        );
        slider.update(cx, |window, cx| {
            assert_eq!(root(window).disabled(), Some(true));
            window.press("right", cx);
        });
        assert!(slider.take_events().is_empty());
        slider.set_prop(cx, "disabled", None);
        click(&slider, cx);
        assert_eq!(slider.take_events().len(), 2);
    }

    #[gpui_kit::test]
    fn test_slider_keyboard(cx: &mut TestAppContext) {
        let slider = slider(cx, 50.);
        let focus = slider.focus_handle(cx).expect("the slider is a Tab stop");
        slider.update(cx, |window, cx| focus.focus(window, cx));
        for (key, value) in [
            ("right", 51.0),
            ("up", 52.0),
            ("left", 51.0),
            ("down", 50.0),
            ("pageup", 60.0),
            ("pagedown", 50.0),
            ("home", 0.0),
            ("end", 100.0),
        ] {
            slider.update(cx, |window, cx| window.press(key, cx));
            assert_eq!(slider.take_events(), pressed(json!(value)), "{key}");
        }
        // At the maximum, nothing changes.
        slider.update(cx, |window, cx| window.press("right", cx));
        assert!(slider.take_events().is_empty());
        assert!((thumb_fraction(&slider, cx) - 1.).abs() < 0.02);
    }

    #[gpui_kit::test]
    fn test_slider_click_focuses(cx: &mut TestAppContext) {
        let slider = slider(cx, 0.);
        let focus = slider.focus_handle(cx).unwrap();
        click_track(&slider, cx, 0.4);
        slider.take_events();
        slider.update(cx, |window, cx| {
            assert!(focus.is_focused(window));
            window.press("right", cx);
        });
        assert_eq!(slider.take_events(), pressed(json!(41.0)));
    }

    #[gpui_kit::test]
    fn test_slider_range(cx: &mut TestAppContext) {
        let slider = Fixture::new(cx, "Slider", props_with(json!([20, 80])));
        assert!((fraction_of(&slider, cx, start_thumb()) - 0.2).abs() < 0.02);
        assert!((thumb_fraction(&slider, cx) - 0.8).abs() < 0.02);

        // A click moves the nearer thumb.
        click_track(&slider, cx, 0.1);
        assert_eq!(slider.take_events(), pressed(json!([10.0, 80.0])));
        // The keys move the thumb the pointer moved last.
        slider.update(cx, |window, cx| window.press("left", cx));
        assert_eq!(slider.take_events(), pressed(json!([9.0, 80.0])));
        click_track(&slider, cx, 0.9);
        assert_eq!(slider.take_events(), pressed(json!([9.0, 90.0])));
        slider.update(cx, |window, cx| window.press("pageup", cx));
        assert_eq!(slider.take_events(), pressed(json!([9.0, 100.0])));
        // The start thumb stops at the end one.
        click_track(&slider, cx, 0.3);
        slider.take_events();
        slider.update(cx, |window, cx| window.press("end", cx));
        assert_eq!(slider.take_events(), pressed(json!([100.0, 100.0])));
    }

    #[gpui_kit::test]
    fn test_slider_range_drag(cx: &mut TestAppContext) {
        let slider = Fixture::new(cx, "Slider", props_with(json!([20, 80])));
        slider.update(cx, |window, cx| {
            let track = window.find(TRACK).bounds();
            let from = window.find(start_thumb()).bounds().center();
            window.drag(
                from,
                point(track.left() + track.size.width * 0.5, from.y),
                cx,
            );
        });
        let events = slider.take_events();
        assert_eq!(
            events.last(),
            Some(&("release".into(), json!({ "value": [50.0, 80.0] })))
        );
        assert!((fraction_of(&slider, cx, start_thumb()) - 0.5).abs() < 0.02);
    }

    #[gpui_kit::test]
    fn test_slider_range_controlled(cx: &mut TestAppContext) {
        let slider = slider(cx, 10.);
        slider.set_prop(cx, "value", Some(json!([30, 60])));
        assert!((fraction_of(&slider, cx, start_thumb()) - 0.3).abs() < 0.02);
        assert!((thumb_fraction(&slider, cx) - 0.6).abs() < 0.02);
        slider.set_prop(cx, "value", Some(json!(40)));
        slider.update(cx, |window, _| {
            assert!(window.try_find(start_thumb()).is_none());
        });
        assert!((thumb_fraction(&slider, cx) - 0.4).abs() < 0.02);
        assert!(slider.take_events().is_empty());
    }
}
