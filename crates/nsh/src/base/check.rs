//! Controls that hold a checked state: Switch, Checkbox, Radio.

use gpui_kit::{
    IntoElement as _, ParentElement as _,
    base::{Checkbox, CheckboxIndicator, CheckboxState, Radio, Switch, SwitchThumb, SwitchTrack},
    prelude::FluentBuilder as _,
    svg,
};
use serde_json::json;

use crate::{Control, PropType, ROOT_STATES};

pub(super) fn controls() -> Vec<Control> {
    vec![switch(), checkbox(), radio()]
}

fn switch() -> Control {
    Control::focusable("Switch", |focus, props, events, _, cx| {
        let checked = props.boolean("checked");
        let disabled = props.boolean("disabled");
        props
            .states(ROOT_STATES, props.styled(Switch::new(props.id()), cx), cx)
            .track_focus(focus)
            .checked(checked)
            .disabled(disabled)
            .styles(|styles| {
                styles
                    .checked(|style| props.styled_by("checkedClass", style, cx))
                    .disabled(|style| props.styled_by("disabledClass", style, cx))
            })
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .on_change({
                let events = events.clone();
                move |checked, _, _, _| events.emit("change", json!({ "checked": checked }))
            })
            .child(
                props
                    .styled_by("trackClass", SwitchTrack::new((props.id(), "track")), cx)
                    .checked(checked)
                    .disabled(disabled)
                    .styles(|styles| {
                        styles
                            .checked(|style| props.styled_by("trackCheckedClass", style, cx))
                            .disabled(|style| props.styled_by("trackDisabledClass", style, cx))
                    })
                    .child(
                        props
                            .styled_by("thumbClass", SwitchThumb::new(checked), cx)
                            .disabled(disabled)
                            .styles(|styles| {
                                styles
                                    .checked(|style| {
                                        props.styled_by("thumbCheckedClass", style, cx)
                                    })
                                    .disabled(|style| {
                                        props.styled_by("thumbDisabledClass", style, cx)
                                    })
                            }),
                    ),
            )
            .when_some(props.string("label"), |this, label| this.child(label))
            .into_any_element()
    })
    .documentation("GPUI Base's Switch, a track holding a thumb: toggled by a click or Space.")
    .prop("checked", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("label", PropType::String)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .states(ROOT_STATES)
    .prop("checkedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .prop("trackClass", PropType::String)
    .prop("trackCheckedClass", PropType::String)
    .prop("trackDisabledClass", PropType::String)
    .prop("thumbClass", PropType::String)
    .prop("thumbCheckedClass", PropType::String)
    .prop("thumbDisabledClass", PropType::String)
    .event("change", &[("checked", PropType::Boolean)])
}

fn checkbox() -> Control {
    Control::focusable("Checkbox", |focus, props, events, _, cx| {
        let state = if props.boolean("indeterminate") {
            CheckboxState::Indeterminate
        } else if props.boolean("checked") {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        let disabled = props.boolean("disabled");
        props
            .states(ROOT_STATES, props.styled(Checkbox::new(props.id()), cx), cx)
            .track_focus(focus)
            .state(state)
            .disabled(disabled)
            .styles(|styles| {
                styles
                    .checked(|style| props.styled_by("checkedClass", style, cx))
                    .indeterminate(|style| props.styled_by("indeterminateClass", style, cx))
                    .disabled(|style| props.styled_by("disabledClass", style, cx))
            })
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .on_change({
                let events = events.clone();
                move |state, _, _, _| {
                    events.emit(
                        "change",
                        json!({
                            "checked": state == CheckboxState::Checked,
                            "indeterminate": state == CheckboxState::Indeterminate,
                        }),
                    )
                }
            })
            .child(
                props
                    .styled_by("indicatorClass", CheckboxIndicator::new(), cx)
                    .state(state)
                    .disabled(disabled)
                    .styles(|styles| {
                        styles
                            .checked(|style| props.styled_by("indicatorCheckedClass", style, cx))
                            .indeterminate(|style| {
                                props.styled_by("indicatorIndeterminateClass", style, cx)
                            })
                            .disabled(|style| props.styled_by("indicatorDisabledClass", style, cx))
                    })
                    // Base leaves the mark to the composer: an SVG from the
                    // host's assets, named by the app.
                    .when_some(
                        match state {
                            CheckboxState::Checked => props.string("checkedIcon"),
                            CheckboxState::Indeterminate => props.string("indeterminateIcon"),
                            CheckboxState::Unchecked => None,
                        },
                        |this, icon| this.child(props.styled_by("iconClass", svg().path(icon), cx)),
                    ),
            )
            .when_some(props.string("label"), |this, label| this.child(label))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Checkbox, an indicator and a label: toggled by a click or Space; \
         an indeterminate checkbox becomes checked. `checkedIcon` and `indeterminateIcon` \
         name SVGs in the host's assets (`icons/check.svg`) drawn in the indicator.",
    )
    .prop("checked", PropType::Boolean)
    .prop("indeterminate", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("label", PropType::String)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .states(ROOT_STATES)
    .prop("checkedClass", PropType::String)
    .prop("indeterminateClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .prop("indicatorClass", PropType::String)
    .prop("indicatorCheckedClass", PropType::String)
    .prop("indicatorIndeterminateClass", PropType::String)
    .prop("indicatorDisabledClass", PropType::String)
    .prop("checkedIcon", PropType::String)
    .prop("indeterminateIcon", PropType::String)
    .prop("iconClass", PropType::String)
    .event(
        "change",
        &[
            ("checked", PropType::Boolean),
            ("indeterminate", PropType::Boolean),
        ],
    )
}

fn radio() -> Control {
    Control::focusable("Radio", |focus, props, events, _, cx| {
        props
            .states(ROOT_STATES, props.styled(Radio::new(props.id()), cx), cx)
            .track_focus(focus)
            .checked(props.boolean("checked"))
            .disabled(props.boolean("disabled"))
            .styles(|styles| {
                styles
                    .checked(|style| props.styled_by("checkedClass", style, cx))
                    .disabled(|style| props.styled_by("disabledClass", style, cx))
            })
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .on_change({
                let events = events.clone();
                move |checked, _, _, _| events.emit("change", json!({ "checked": checked }))
            })
            .when_some(props.string("label"), |this, label| this.child(label))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Radio: a click or Space on an unchecked radio requests checked; \
         the app unchecks the others of its group.",
    )
    .prop("checked", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("label", PropType::String)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .states(ROOT_STATES)
    .prop("checkedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("checked", PropType::Boolean)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Role, TestAppContext, px};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    fn assert_measured(fixture: &Fixture, cx: &mut TestAppContext) {
        let size = fixture.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
    }

    #[gpui_kit::test]
    fn test_switch_builder(cx: &mut TestAppContext) {
        let switch = Fixture::new(
            cx,
            "Switch",
            json!({
                "checked": true,
                "label": "Wi-Fi",
                "accessibilityLabel": "Wi-Fi",
                "rootClass": "flex items_center gap_2",
                "checkedClass": "text_primary",
                "disabledClass": "opacity_50",
                "trackClass": "w_9 h_5 p_0p5 rounded_full bg_muted",
                "trackCheckedClass": "bg_primary justify_end",
                "trackDisabledClass": "opacity_50",
                "thumbClass": "size_4 rounded_full bg_background",
                "thumbCheckedClass": "shadow_sm",
                "thumbDisabledClass": "opacity_50",
            }),
        );
        assert_measured(&switch, cx);
        switch.update(cx, |window, _| {
            let element = window.find(switch.id.clone());
            assert_eq!(element.role(), Some(Role::Switch));
            assert_eq!(element.checked(), Some(true));
            assert_eq!(element.label(), Some("Wi-Fi"));
        });
    }

    #[gpui_kit::test]
    fn test_switch_click_and_keyboard(cx: &mut TestAppContext) {
        let switch = Fixture::new(cx, "Switch", json!({ "label": "Wi-Fi" }));
        switch.update(cx, |window, cx| window.click(switch.id.clone(), cx));
        assert_eq!(
            switch.take_events(),
            vec![("change".into(), json!({ "checked": true }))]
        );
        // The event's `checked` updated the prop, so the switch shows it.
        switch.update(cx, |window, _| {
            assert_eq!(window.find(switch.id.clone()).checked(), Some(true))
        });
        switch.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(
            switch.take_events(),
            vec![("change".into(), json!({ "checked": false }))]
        );
    }

    #[gpui_kit::test]
    fn test_switch_controlled(cx: &mut TestAppContext) {
        let switch = Fixture::new(cx, "Switch", json!({ "label": "Wi-Fi" }));
        switch.set_prop(cx, "checked", Some(json!(true)));
        switch.update(cx, |window, _| {
            assert_eq!(window.find(switch.id.clone()).checked(), Some(true))
        });
        switch.set_prop(cx, "checked", None);
        switch.update(cx, |window, _| {
            assert_eq!(window.find(switch.id.clone()).checked(), Some(false))
        });
    }

    #[gpui_kit::test]
    fn test_switch_disabled(cx: &mut TestAppContext) {
        let switch = Fixture::new(cx, "Switch", json!({ "label": "Wi-Fi", "disabled": true }));
        switch.update(cx, |window, cx| {
            assert_eq!(window.find(switch.id.clone()).disabled(), Some(true));
            window.click(switch.id.clone(), cx)
        });
        assert!(switch.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_checkbox_builder(cx: &mut TestAppContext) {
        let checkbox = Fixture::new(
            cx,
            "Checkbox",
            json!({
                "checked": true,
                "label": "Remember me",
                "accessibilityLabel": "Remember me",
                "rootClass": "flex items_center gap_2",
                "checkedClass": "text_primary",
                "indeterminateClass": "text_primary",
                "disabledClass": "opacity_50",
                "indicatorClass": "size_4 rounded_sm border_1 border_border",
                "indicatorCheckedClass": "bg_primary",
                "indicatorIndeterminateClass": "bg_muted",
                "indicatorDisabledClass": "opacity_50",
                "checkedIcon": "icons/check.svg",
                "indeterminateIcon": "icons/minus.svg",
                "iconClass": "size_3 text_primary_foreground",
            }),
        );
        assert_measured(&checkbox, cx);
        checkbox.update(cx, |window, _| {
            let element = window.find(checkbox.id.clone());
            assert_eq!(element.role(), Some(Role::CheckBox));
            assert_eq!(element.checked(), Some(true));
            assert_eq!(element.label(), Some("Remember me"));
        });
    }

    #[gpui_kit::test]
    fn test_checkbox_click_and_keyboard(cx: &mut TestAppContext) {
        let checkbox = Fixture::new(cx, "Checkbox", json!({ "label": "Remember me" }));
        checkbox.update(cx, |window, cx| window.click(checkbox.id.clone(), cx));
        assert_eq!(
            checkbox.take_events(),
            vec![(
                "change".into(),
                json!({ "checked": true, "indeterminate": false })
            )]
        );
        checkbox.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(
            checkbox.take_events(),
            vec![(
                "change".into(),
                json!({ "checked": false, "indeterminate": false })
            )]
        );
    }

    #[gpui_kit::test]
    fn test_checkbox_indeterminate(cx: &mut TestAppContext) {
        let checkbox = Fixture::new(
            cx,
            "Checkbox",
            json!({ "label": "Select all", "indeterminate": true }),
        );
        checkbox.update(cx, |window, _| {
            assert_eq!(window.find(checkbox.id.clone()).indeterminate(), Some(true))
        });
        // An indeterminate checkbox becomes checked, and the event clears
        // `indeterminate` at once.
        checkbox.update(cx, |window, cx| window.click(checkbox.id.clone(), cx));
        assert_eq!(
            checkbox.take_events(),
            vec![(
                "change".into(),
                json!({ "checked": true, "indeterminate": false })
            )]
        );
        checkbox.update(cx, |window, _| {
            assert_eq!(window.find(checkbox.id.clone()).checked(), Some(true))
        });
    }

    #[gpui_kit::test]
    fn test_checkbox_controlled(cx: &mut TestAppContext) {
        let checkbox = Fixture::new(cx, "Checkbox", json!({ "label": "Remember me" }));
        checkbox.set_prop(cx, "checked", Some(json!(true)));
        checkbox.update(cx, |window, _| {
            assert_eq!(window.find(checkbox.id.clone()).checked(), Some(true))
        });
        checkbox.set_prop(cx, "checked", Some(json!(false)));
        checkbox.update(cx, |window, _| {
            assert_eq!(window.find(checkbox.id.clone()).checked(), Some(false))
        });
    }

    #[gpui_kit::test]
    fn test_checkbox_disabled(cx: &mut TestAppContext) {
        let checkbox = Fixture::new(
            cx,
            "Checkbox",
            json!({ "label": "Remember me", "disabled": true }),
        );
        checkbox.update(cx, |window, cx| window.click(checkbox.id.clone(), cx));
        assert!(checkbox.take_events().is_empty());
        checkbox.set_prop(cx, "disabled", None);
        checkbox.update(cx, |window, cx| window.click(checkbox.id.clone(), cx));
        assert_eq!(checkbox.take_events().len(), 1);
    }

    #[gpui_kit::test]
    fn test_radio_builder(cx: &mut TestAppContext) {
        let radio = Fixture::new(
            cx,
            "Radio",
            json!({
                "checked": true,
                "label": "Standard",
                "accessibilityLabel": "Standard shipping",
                "rootClass": "flex items_center gap_2",
                "checkedClass": "text_primary",
                "disabledClass": "opacity_50",
            }),
        );
        assert_measured(&radio, cx);
        radio.update(cx, |window, _| {
            let element = window.find(radio.id.clone());
            assert_eq!(element.role(), Some(Role::RadioButton));
            assert_eq!(element.checked(), Some(true));
            assert_eq!(element.label(), Some("Standard shipping"));
        });
    }

    #[gpui_kit::test]
    fn test_radio_click_and_keyboard(cx: &mut TestAppContext) {
        let radio = Fixture::new(cx, "Radio", json!({ "label": "Standard" }));
        radio.update(cx, |window, cx| window.click(radio.id.clone(), cx));
        assert_eq!(
            radio.take_events(),
            vec![("change".into(), json!({ "checked": true }))]
        );
        // A checked radio cannot uncheck itself.
        radio.update(cx, |window, cx| window.click(radio.id.clone(), cx));
        assert!(radio.take_events().is_empty());
        radio.set_prop(cx, "checked", Some(json!(false)));
        radio.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(
            radio.take_events(),
            vec![("change".into(), json!({ "checked": true }))]
        );
    }

    #[gpui_kit::test]
    fn test_radio_controlled(cx: &mut TestAppContext) {
        let radio = Fixture::new(cx, "Radio", json!({ "label": "Standard" }));
        radio.set_prop(cx, "checked", Some(json!(true)));
        radio.update(cx, |window, _| {
            assert_eq!(window.find(radio.id.clone()).checked(), Some(true))
        });
        radio.set_prop(cx, "checked", Some(json!(false)));
        radio.update(cx, |window, _| {
            assert_eq!(window.find(radio.id.clone()).checked(), Some(false))
        });
    }

    #[gpui_kit::test]
    fn test_radio_disabled(cx: &mut TestAppContext) {
        let radio = Fixture::new(
            cx,
            "Radio",
            json!({ "label": "Standard", "disabled": true }),
        );
        radio.update(cx, |window, cx| window.click(radio.id.clone(), cx));
        assert!(radio.take_events().is_empty());
    }
}
