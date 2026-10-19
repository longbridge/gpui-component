//! Controls entering a number: NumberInput, OtpInput.

use gpui_kit::{
    App, AppContext as _, Entity, Focusable as _, IntoElement as _, ParentElement as _,
    StyleRefinement, Subscription, Window,
    base::{
        Button, Input, NumberInput, NumberInputEvent, NumberStep, OtpEvent, OtpInput, OtpState,
        StepAction, StyledExt as _,
        input::{InputEvent, InputState},
    },
    div,
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, Events, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![number_input(), otp_input()]
}

/// The editor of a NumberInput and the subscriptions sending its events.
struct NumberInputControl {
    input: Entity<InputState>,
    _subscriptions: [Subscription; 2],
}

/// The interaction-state class props of both step buttons, which never take
/// focus.
const STEP_STATES: [&str; 3] = ["stepHoverClass", "stepActiveClass", ""];

fn number_input() -> Control {
    Control::stateful(
        "NumberInput",
        |_, events, window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            let change = events.clone();
            let step = events.clone();
            NumberInputControl {
                _subscriptions: [
                    cx.subscribe(&input, move |input, event: &InputEvent, cx| {
                        if let InputEvent::Change = event {
                            change.emit("change", json!({ "value": input.read(cx).value() }));
                        }
                    }),
                    cx.subscribe(&input, move |_, event: &NumberInputEvent, _| {
                        let NumberInputEvent::Step(action) = event;
                        let action = match action {
                            StepAction::Increment => "increment",
                            StepAction::Decrement => "decrement",
                        };
                        step.emit("step", json!({ "action": action }));
                    }),
                ],
                input,
            }
        },
        |control, props, _, window, cx| {
            // The state takes `value` only when JS sets it, never while an
            // edit's `change` is still on its way.
            let value = props.string("value").filter(|_| props.changed("value"));
            control.input.update(cx, |input, cx| {
                if let Some(value) = value.filter(|value| input.value() != *value) {
                    input.set_value(value, window, cx);
                }
                let placeholder = props.string("placeholder").unwrap_or_default();
                if *input.presentation().placeholder() != placeholder {
                    input.set_placeholder(placeholder, window, cx);
                }
                let number = |name| props.has(name).then(|| props.number(name));
                input.set_step(number("step").map(NumberStep::Fixed), window, cx);
                input.set_min(number("min"), window, cx);
                input.set_max(number("max"), window, cx);
                input.set_disabled(props.boolean("disabled"), cx);
            });
            let button = |class: &str, label: &str| {
                let style = props.styled_by(class, StyleRefinement::default(), cx);
                let states = props.state_styles(STEP_STATES, cx);
                let label = props.string(label);
                move |button: Button| {
                    states
                        .apply(button.refine_style(&style))
                        .when_some(label, |this, label| this.child(label))
                }
            };
            props
                .styled(NumberInput::new(&control.input), cx)
                .id(props.id())
                .disabled(props.boolean("disabled"))
                .when(props.boolean("controlsRight"), |this| this.controls_right())
                .when_some(props.string("accessibilityLabel"), |this, label| {
                    this.accessibility_label(label)
                })
                .decrement_button(button("decrementClass", "decrementLabel"))
                .increment_button(button("incrementClass", "incrementLabel"))
                .input(props.styled_by("inputClass", div().child(Input::new(&control.input)), cx))
                .into_any_element()
        },
    )
    .focus(|control: &NumberInputControl, cx| control.input.focus_handle(cx))
    .documentation(
        "GPUI Base's NumberInput: a text field with decrement and increment buttons, stepped by \
         them and by Up and Down. With `step` set it steps the value itself, clamped to `min` and \
         `max`; without it, it sends `step` for the app to apply.",
    )
    .prop("value", PropType::String)
    .prop("placeholder", PropType::String)
    .prop("min", PropType::Number)
    .prop("max", PropType::Number)
    .prop("step", PropType::Number)
    .prop("disabled", PropType::Boolean)
    .prop("controlsRight", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("decrementLabel", PropType::String)
    .prop("incrementLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("inputClass", PropType::String)
    .prop("decrementClass", PropType::String)
    .prop("incrementClass", PropType::String)
    .states(STEP_STATES)
    .event("change", &[("value", PropType::String)])
    .event(
        "step",
        &[("action", PropType::Enum(&["increment", "decrement"]))],
    )
}

/// The code of an OtpInput and the subscription sending its events.
struct OtpInputControl {
    otp: Entity<OtpState>,
    _subscription: Subscription,
}

/// The number of cells when the app sets no `length`.
const OTP_LENGTH: usize = 6;

fn otp_state(props: &Props, events: &Events, window: &mut Window, cx: &mut App) -> OtpInputControl {
    // `OtpState` takes its length only when it is made.
    let length = if props.has("length") {
        props.number("length").max(1.) as usize
    } else {
        OTP_LENGTH
    };
    let otp = cx.new(|cx| OtpState::new(length, window, cx));
    let events = events.clone();
    OtpInputControl {
        _subscription: cx.subscribe(&otp, move |otp, event: &OtpEvent, cx| {
            let value = otp.read(cx).value().clone();
            match event {
                OtpEvent::Change => events.emit("change", json!({ "value": value })),
                OtpEvent::Complete => events.emit("complete", json!({ "value": value })),
                OtpEvent::Focus | OtpEvent::Blur => {}
            }
        }),
        otp,
    }
}

fn otp_input() -> Control {
    Control::stateful("OtpInput", otp_state, |control, props, _, window, cx| {
        // The state takes `value` only when JS sets it (see NumberInput).
        let value = props.string("value").filter(|_| props.changed("value"));
        control.otp.update(cx, |otp, cx| {
            if let Some(value) = value.filter(|value| otp.value() != value) {
                otp.set_value(value, window, cx);
            }
            if otp.is_masked() != props.boolean("masked") {
                otp.set_masked(props.boolean("masked"), window, cx);
            }
        });
        let otp = control.otp.read(cx);
        let disabled = props.boolean("disabled");
        let length = otp.len();
        let value: Vec<char> = otp.value().chars().collect();
        let active = (!disabled && otp.focus_handle(cx).is_focused(window))
            .then(|| value.len().min(length - 1));
        let mask = props.string("maskLabel").unwrap_or_else(|| "•".into());
        props
            .styled(OtpInput::new(&control.otp), cx)
            .id(props.id())
            .disabled(disabled)
            .children((0..length).map(|ix| {
                props
                    .styled_by("cellClass", div(), cx)
                    .when(active == Some(ix), |this| {
                        props.styled_by("activeCellClass", this, cx)
                    })
                    .when_some(value.get(ix), |this, digit| {
                        this.child(if otp.is_masked() {
                            mask.clone()
                        } else {
                            digit.to_string().into()
                        })
                    })
            }))
            .into_any_element()
    })
    .focus(|control: &OtpInputControl, cx| control.otp.focus_handle(cx))
    .documentation(
        "GPUI Base's OtpInput: a fixed-length numeric one-time code typed into a row of cells, \
         one digit each. `length` (6 when unset) is read when the control is made.",
    )
    .prop("value", PropType::String)
    .prop("length", PropType::Number)
    .prop("masked", PropType::Boolean)
    .prop("maskLabel", PropType::String)
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("cellClass", PropType::String)
    .prop("activeCellClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("complete", &[("value", PropType::String)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{TestAppContext, px};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    fn focus(fixture: &Fixture, cx: &mut TestAppContext) {
        let handle = fixture.focus_handle(cx).expect("a Tab stop");
        fixture.update(cx, |window, cx| handle.focus(window, cx));
    }

    /// Clicks a NumberInput's step button, found under the input's id.
    fn step(input: &Fixture, button: &'static str, cx: &mut TestAppContext) {
        input.update(cx, |window, cx| {
            window.within(input.id.clone()).click(button, cx)
        });
    }

    #[gpui_kit::test]
    fn test_number_input_builder(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "NumberInput",
            json!({
                "value": "3",
                "placeholder": "Quantity",
                "min": 0,
                "max": 10,
                "step": 1,
                "controlsRight": true,
                "accessibilityLabel": "Quantity",
                "decrementLabel": "-",
                "incrementLabel": "+",
                "rootClass": "flex items_center w_32 h_7 border_1 border_border",
                "inputClass": "w_full px_2",
                "decrementClass": "w_6 bg_secondary",
                "incrementClass": "w_6 bg_secondary",
            }),
        );
        let size = input.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
    }

    #[gpui_kit::test]
    fn test_number_input_step_buttons_and_keys(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "NumberInput",
            json!({ "value": "1", "min": 0, "max": 2, "step": 1, "decrementLabel": "-", "incrementLabel": "+" }),
        );
        step(&input, "increment", cx);
        assert_eq!(
            input.take_events(),
            vec![("change".into(), json!({ "value": "2" }))]
        );
        // At `max` the value no longer moves.
        step(&input, "increment", cx);
        assert!(input.take_events().is_empty());

        focus(&input, cx);
        input.update(cx, |window, cx| window.press("down", cx));
        input.update(cx, |window, cx| window.press("down", cx));
        input.update(cx, |window, cx| window.press("down", cx));
        assert_eq!(
            input.take_events(),
            vec![
                ("change".into(), json!({ "value": "1" })),
                ("change".into(), json!({ "value": "0" })),
            ]
        );
        input.update(cx, |window, cx| window.press("up", cx));
        assert_eq!(
            input.take_events(),
            vec![("change".into(), json!({ "value": "1" }))]
        );
    }

    #[gpui_kit::test]
    fn test_number_input_typing(cx: &mut TestAppContext) {
        let input = Fixture::new(cx, "NumberInput", json!({ "value": "1" }));
        focus(&input, cx);
        input.update(cx, |window, cx| window.input("5", cx));
        assert_eq!(
            input.take_events(),
            vec![("change".into(), json!({ "value": "15" }))]
        );
        // Base's number mask keeps letters out.
        input.update(cx, |window, cx| window.input("a", cx));
        assert!(input.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_number_input_step_event(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "NumberInput",
            json!({ "value": "1", "decrementLabel": "-", "incrementLabel": "+" }),
        );
        step(&input, "increment", cx);
        step(&input, "decrement", cx);
        assert_eq!(
            input.take_events(),
            vec![
                ("step".into(), json!({ "action": "increment" })),
                ("step".into(), json!({ "action": "decrement" })),
            ]
        );
    }

    #[gpui_kit::test]
    fn test_number_input_controlled_value(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "NumberInput",
            json!({ "value": "1", "step": 0.5, "incrementLabel": "+" }),
        );
        input.set_prop(cx, "value", Some(json!("7")));
        assert!(input.take_events().is_empty());
        step(&input, "increment", cx);
        assert_eq!(
            input.take_events(),
            vec![("change".into(), json!({ "value": "7.5" }))]
        );
    }

    #[gpui_kit::test]
    fn test_number_input_disabled(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "NumberInput",
            json!({ "value": "1", "step": 1, "disabled": true, "incrementLabel": "+" }),
        );
        step(&input, "increment", cx);
        // A disabled control is no Tab stop.
        assert!(input.focus_handle(cx).is_none());
        input.update(cx, |window, cx| window.press("up", cx));
        input.update(cx, |window, cx| window.input("5", cx));
        assert!(input.take_events().is_empty());

        input.set_prop(cx, "disabled", None);
        step(&input, "increment", cx);
        assert_eq!(
            input.take_events(),
            vec![("change".into(), json!({ "value": "2" }))]
        );
    }

    #[gpui_kit::test]
    fn test_otp_input_builder(cx: &mut TestAppContext) {
        let otp = Fixture::new(
            cx,
            "OtpInput",
            json!({
                "value": "12",
                "length": 4,
                "masked": true,
                "maskLabel": "*",
                "rootClass": "flex gap_1",
                "cellClass": "size_7 flex items_center justify_center border_1 border_border",
                "activeCellClass": "border_ring",
            }),
        );
        let size = otp.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
    }

    #[gpui_kit::test]
    fn test_otp_input_typing(cx: &mut TestAppContext) {
        let otp = Fixture::new(
            cx,
            "OtpInput",
            json!({ "length": 3, "rootClass": "flex", "cellClass": "size_7" }),
        );
        // A click focuses the code, as a Tab would.
        otp.update(cx, |window, cx| {
            window.click(otp.id.clone(), cx);
            assert_eq!(window.find(otp.id.clone()).focused(), Some(true));
        });
        for key in ["1", "a", "2", "3", "4"] {
            otp.update(cx, |window, cx| window.press(key, cx));
        }
        assert_eq!(
            otp.take_events(),
            vec![
                ("change".into(), json!({ "value": "1" })),
                ("change".into(), json!({ "value": "12" })),
                ("change".into(), json!({ "value": "123" })),
                ("complete".into(), json!({ "value": "123" })),
            ]
        );
        otp.update(cx, |window, cx| window.press("backspace", cx));
        assert_eq!(
            otp.take_events(),
            vec![("change".into(), json!({ "value": "12" }))]
        );
    }

    #[gpui_kit::test]
    fn test_otp_input_controlled(cx: &mut TestAppContext) {
        let otp = Fixture::new(cx, "OtpInput", json!({ "length": 4 }));
        otp.set_prop(cx, "value", Some(json!("12")));
        focus(&otp, cx);
        otp.update(cx, |window, cx| window.press("3", cx));
        assert_eq!(
            otp.take_events(),
            vec![("change".into(), json!({ "value": "123" }))]
        );

        otp.set_prop(cx, "value", Some(json!("")));
        otp.update(cx, |window, cx| window.press("4", cx));
        assert_eq!(
            otp.take_events(),
            vec![("change".into(), json!({ "value": "4" }))]
        );
    }

    #[gpui_kit::test]
    fn test_otp_input_disabled(cx: &mut TestAppContext) {
        let otp = Fixture::new(
            cx,
            "OtpInput",
            json!({ "length": 4, "disabled": true, "rootClass": "flex", "cellClass": "size_7" }),
        );
        // A disabled control is no Tab stop, and a click does not focus it.
        assert!(otp.focus_handle(cx).is_none());
        otp.update(cx, |window, cx| {
            window.click(otp.id.clone(), cx);
            assert_eq!(window.find(otp.id.clone()).focused(), None);
            window.press("1", cx);
        });
        assert!(otp.take_events().is_empty());
        otp.set_prop(cx, "disabled", None);
        focus(&otp, cx);
        otp.update(cx, |window, cx| window.press("1", cx));
        assert_eq!(
            otp.take_events(),
            vec![("change".into(), json!({ "value": "1" }))]
        );
    }
}
