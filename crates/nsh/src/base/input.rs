//! Text fields: Input, Textarea.

use gpui_kit::{
    App, AppContext as _, Entity, Focusable as _, InteractiveElement as _, IntoElement as _,
    MouseButton, ParentElement as _, Subscription, Window,
    base::input::{
        Input, InputBase, InputBaseState, InputEvent, InputMode, InputModeKind, InputState,
        Textarea, TextareaMode, TextareaState,
    },
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, Events, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![input(), textarea()]
}

/// A text field's state entity, kept across frames with the subscription
/// sending its events to JS.
struct Field<M: InputModeKind> {
    state: Entity<InputBaseState<M>>,
    _events: Subscription,
}

impl<M: InputModeKind> Field<M> {
    /// Sends `state`'s events; `press_enter` makes the data of Enter's.
    fn new(
        state: Entity<InputBaseState<M>>,
        events: &Events,
        press_enter: impl Fn(&str, bool, bool) -> (&'static str, serde_json::Value) + 'static,
        cx: &mut App,
    ) -> Self {
        let events = events.clone();
        let subscription = cx.subscribe(&state, move |state, event: &InputEvent, cx| {
            let value = state.read(cx).value();
            match event {
                InputEvent::Change => events.emit("change", json!({ "value": value })),
                InputEvent::PressEnter { secondary, shift } => {
                    let (name, data) = press_enter(&value, *shift, *secondary);
                    events.emit(name, data)
                }
                InputEvent::Focus => events.emit("focus", json!({})),
                InputEvent::Blur => events.emit("blur", json!({})),
            }
        });
        Self {
            state,
            _events: subscription,
        }
    }

    /// Brings the state up to date with the props shared by every field
    /// that JS changed.
    fn update(&self, props: &Props, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            if props.changed("value")
                && let Some(value) = props.string("value")
            {
                state.set_value(value, window, cx);
            }
            if props.changed("placeholder") {
                state.set_placeholder(props.string("placeholder").unwrap_or_default(), window, cx);
            }
            state.set_disabled(props.boolean("disabled"), cx);
            state.set_readonly(props.boolean("readonly"), cx);
        });
    }

    /// The input frame; the caller adds the field's text element.
    fn frame(&self, props: &Props, window: &Window, cx: &App) -> InputBase {
        let state = self.state.clone();
        let disabled = props.boolean("disabled");
        props
            .states(
                FRAME_STATES,
                props.styled(InputBase::new(props.id()), cx),
                cx,
            )
            .focused(self.state.focus_handle(cx).is_focused(window))
            .disabled(disabled)
            .styles(|styles| {
                styles
                    .focused(|style| props.styled_by("focusedClass", style, cx))
                    .disabled(|style| props.styled_by("disabledClass", style, cx))
            })
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .when(!disabled, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    state.update(cx, |state, cx| state.focus(window, cx));
                })
            })
    }
}

/// The interaction-state class props of the frame: its focus shows through
/// `focusedClass`, which follows the text field's focus.
const FRAME_STATES: [&str; 3] = ["hoverClass", "", ""];

fn input() -> Control {
    Control::stateful(
        "Input",
        |_, events, window, cx| {
            let state = cx.new(|cx| InputState::new(window, cx));
            Field::new(
                state,
                events,
                |value, _, _| ("submit", json!({ "value": value })),
                cx,
            )
        },
        |field, props, _, window, cx| {
            field.update(props, window, cx);
            if props.changed("masked") {
                field.state.update(cx, |state, cx| {
                    state.set_masked(props.boolean("masked"), window, cx)
                });
            }
            field
                .frame(props, window, cx)
                .child(Input::new(&field.state))
                .into_any_element()
        },
    )
    .focus(|field: &Field<InputMode>, cx| field.state.focus_handle(cx))
    .documentation(
        "GPUI Base's Input: a single-line text field; Enter sends `submit`, \
         `masked` hides the value as a password.",
    )
    .prop("value", PropType::String)
    .prop("placeholder", PropType::String)
    .prop("disabled", PropType::Boolean)
    .prop("readonly", PropType::Boolean)
    .prop("masked", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .states(FRAME_STATES)
    .prop("focusedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("submit", &[("value", PropType::String)])
    .event("focus", &[])
    .event("blur", &[])
}

fn textarea() -> Control {
    Control::stateful(
        "Textarea",
        |_, events, window, cx| {
            let state = cx.new(|cx| TextareaState::new(window, cx));
            Field::new(
                state,
                events,
                |value, shift, secondary| {
                    let data = json!({ "value": value, "shift": shift, "secondary": secondary });
                    ("pressEnter", data)
                },
                cx,
            )
        },
        |field, props, _, window, cx| {
            field.update(props, window, cx);
            field.state.update(cx, |state, cx| {
                if props.changed("submitOnEnter") {
                    state.set_submit_on_enter(props.boolean("submitOnEnter"), cx);
                }
                if props.changed("rows") && props.has("rows") {
                    state.set_rows(props.number("rows").max(1.) as usize, cx);
                }
            });
            field
                .frame(props, window, cx)
                .child(Textarea::new(&field.state))
                .into_any_element()
        },
    )
    .focus(|field: &Field<TextareaMode>, cx| field.state.focus_handle(cx))
    .documentation(
        "GPUI Base's Textarea: a multi-line text field. Every Enter sends \
         `pressEnter`; with `submitOnEnter` a plain Enter submits instead of \
         breaking the line, and Shift-Enter still breaks it.",
    )
    .prop("value", PropType::String)
    .prop("placeholder", PropType::String)
    .prop("disabled", PropType::Boolean)
    .prop("readonly", PropType::Boolean)
    .prop("rows", PropType::Number)
    .prop("submitOnEnter", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .states(FRAME_STATES)
    .prop("focusedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event(
        "pressEnter",
        &[
            ("value", PropType::String),
            ("shift", PropType::Boolean),
            ("secondary", PropType::Boolean),
        ],
    )
    .event("focus", &[])
    .event("blur", &[])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{TestAppContext, px};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    fn changes(values: &[&str]) -> Vec<(String, serde_json::Value)> {
        values
            .iter()
            .map(|value| ("change".into(), json!({ "value": value })))
            .collect()
    }

    #[gpui_kit::test]
    fn test_input_builder(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "Input",
            json!({
                "value": "Ada",
                "placeholder": "Name",
                "readonly": false,
                "masked": true,
                "accessibilityLabel": "Account name",
                "rootClass": "w_full px_2 py_1 border_1 border_border rounded_md",
                "focusedClass": "border_ring",
                "disabledClass": "opacity_50",
            }),
        );
        let size = input.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        input.update(cx, |window, _| {
            assert_eq!(window.find(input.id.clone()).label(), Some("Account name"));
        });
        assert!(input.focus_handle(cx).is_some());
        assert!(input.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_input_typing(cx: &mut TestAppContext) {
        let input = Fixture::new(cx, "Input", json!({ "rootClass": "w_full" }));
        // GPUI sends focus changes to an active window only.
        input.update(cx, |window, _| window.activate_window());
        input.update(cx, |window, cx| window.click(input.id.clone(), cx));
        let focus = input.focus_handle(cx).unwrap();
        input.update(cx, |window, _| assert!(focus.is_focused(window)));
        assert_eq!(input.take_events(), vec![("focus".into(), json!({}))]);

        input.update(cx, |window, cx| window.input("H", cx));
        input.update(cx, |window, cx| window.input("i", cx));
        assert_eq!(input.take_events(), changes(&["H", "Hi"]));
        input.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            input.take_events(),
            vec![("submit".into(), json!({ "value": "Hi" }))]
        );
        input.update(cx, |window, cx| window.blur(cx));
        assert_eq!(input.take_events(), vec![("blur".into(), json!({}))]);
    }

    #[gpui_kit::test]
    fn test_input_controlled_value(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "Input",
            json!({ "value": "abc", "rootClass": "w_full" }),
        );
        input.set_prop(cx, "value", Some(json!("xyz")));
        assert!(input.take_events().is_empty());

        input.update(cx, |window, cx| window.click(input.id.clone(), cx));
        input.take_events();
        input.update(cx, |window, cx| window.input("!", cx));
        assert_eq!(input.take_events(), changes(&["xyz!"]));
        // The typed value is now the prop, so the next frame keeps the caret.
        input.update(cx, |window, cx| window.input("?", cx));
        assert_eq!(input.take_events(), changes(&["xyz!?"]));
    }

    #[gpui_kit::test]
    fn test_input_disabled(cx: &mut TestAppContext) {
        let input = Fixture::new(
            cx,
            "Input",
            json!({ "disabled": true, "rootClass": "w_full" }),
        );
        input.update(cx, |window, cx| {
            window.click(input.id.clone(), cx);
            window.input("a", cx);
        });
        // Base's `InputState` tracks focus while disabled too, so the click
        // may focus it; typing changes nothing.
        let events = input.take_events();
        assert!(
            events.iter().all(|(name, _)| name != "change"),
            "{events:?}"
        );
        input.update(cx, |window, cx| window.blur(cx));
        input.take_events();

        input.set_prop(cx, "disabled", None);
        input.update(cx, |window, _| window.activate_window());
        input.update(cx, |window, cx| {
            window.click(input.id.clone(), cx);
            window.input("a", cx);
        });
        assert_eq!(
            input.take_events(),
            vec![
                ("focus".into(), json!({})),
                ("change".into(), json!({ "value": "a" })),
            ]
        );
    }

    #[gpui_kit::test]
    fn test_textarea_builder(cx: &mut TestAppContext) {
        let textarea = Fixture::new(
            cx,
            "Textarea",
            json!({
                "value": "First\nSecond",
                "placeholder": "Notes",
                "rows": 4,
                "submitOnEnter": false,
                "accessibilityLabel": "Notes",
                "rootClass": "w_full p_2 border_1 border_border",
                "focusedClass": "border_ring",
                "disabledClass": "opacity_50",
            }),
        );
        let size = textarea.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        assert!(textarea.focus_handle(cx).is_some());
        assert!(textarea.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_textarea_enter(cx: &mut TestAppContext) {
        let textarea = Fixture::new(
            cx,
            "Textarea",
            json!({ "value": "a", "rootClass": "w_full" }),
        );
        textarea.update(cx, |window, cx| window.click(textarea.id.clone(), cx));
        textarea.take_events();
        textarea.update(cx, |window, cx| window.press("enter", cx));
        let press_enter = |value: &str| {
            (
                "pressEnter".to_string(),
                json!({ "value": value, "shift": false, "secondary": false }),
            )
        };
        assert_eq!(
            textarea.take_events(),
            vec![
                ("change".into(), json!({ "value": "a\n" })),
                press_enter("a\n"),
            ]
        );

        textarea.set_prop(cx, "submitOnEnter", Some(json!(true)));
        textarea.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(textarea.take_events(), vec![press_enter("a\n")]);
    }

    #[gpui_kit::test]
    fn test_textarea_controlled_value(cx: &mut TestAppContext) {
        let textarea = Fixture::new(cx, "Textarea", json!({ "rootClass": "w_full" }));
        textarea.set_prop(cx, "value", Some(json!("one")));
        textarea.update(cx, |window, cx| window.click(textarea.id.clone(), cx));
        textarea.take_events();
        textarea.update(cx, |window, cx| window.input("!", cx));
        let events = textarea.take_events();
        assert_eq!(events.len(), 1);
        assert!(events[0].1["value"].as_str().unwrap().contains("one"));
    }
}
