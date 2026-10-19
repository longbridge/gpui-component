//! Controls choosing among a set of items: RadioGroup, ToggleGroup, Tabs.
//!
//! Each builds GPUI Base's group root with one Base item per string of the
//! `items` prop. The selected item is the `value` prop, an index; an item's
//! selection request is sent as `change { value }`.

use gpui_kit::{
    Axis, IntoElement as _, ParentElement as _, SharedString,
    base::{Radio, RadioGroup, Tab, Tabs, Toggle, ToggleGroup},
};
use serde_json::json;

use crate::{Control, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![radio_group(), toggle_group(), tabs()]
}

/// The `items` prop, each with whether it is the `value` prop's index.
fn items(props: &Props) -> impl Iterator<Item = (usize, SharedString, bool)> {
    let value = props.has("value").then(|| props.number("value"));
    props
        .strings("items")
        .into_iter()
        .enumerate()
        .map(move |(index, label)| (index, label, value == Some(index as f64)))
}

/// The `axis` prop.
fn axis(props: &Props, default: Axis) -> Axis {
    match props.string("axis").as_deref() {
        Some("horizontal") => Axis::Horizontal,
        Some("vertical") => Axis::Vertical,
        _ => default,
    }
}

fn radio_group() -> Control {
    Control::new("RadioGroup", |props, events, _, cx| {
        let count = props.strings("items").len();
        props
            .styled(RadioGroup::new(props.id()), cx)
            .axis(axis(props, Axis::Vertical))
            .children(items(props).map(|(index, label, checked)| {
                props
                    .styled_by("radioClass", Radio::new(index), cx)
                    .checked(checked)
                    .disabled(props.boolean("disabled"))
                    .set_position(index + 1, count)
                    .styles(|styles| {
                        styles
                            .checked(|style| props.styled_by("checkedClass", style, cx))
                            .disabled(|style| props.styled_by("disabledClass", style, cx))
                    })
                    .child(label)
                    .on_change({
                        let events = events.clone();
                        move |_, _, _, _| events.emit("change", json!({ "value": index }))
                    })
            }))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's RadioGroup of Radio items, one per string of `items`: `value` is the \
         checked item's index, and activating another item (a click, Space or Enter) sends \
         `change { value }`.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::Number)
    .prop("axis", PropType::Enum(&["vertical", "horizontal"]))
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("radioClass", PropType::String)
    .prop("checkedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::Number)])
}

fn toggle_group() -> Control {
    Control::new("ToggleGroup", |props, events, _, cx| {
        props
            .styled(ToggleGroup::new(props.id()), cx)
            .axis(axis(props, Axis::Horizontal))
            .children(items(props).map(|(index, label, pressed)| {
                props
                    .styled_by("toggleClass", Toggle::new(index), cx)
                    .pressed(pressed)
                    .disabled(props.boolean("disabled"))
                    .styles(|styles| {
                        styles
                            .pressed(|style| props.styled_by("pressedClass", style, cx))
                            .disabled(|style| props.styled_by("disabledClass", style, cx))
                    })
                    .child(label)
                    .on_change({
                        let events = events.clone();
                        move |pressed, _, _, _| {
                            let value = if pressed { index as i64 } else { -1 };
                            events.emit("change", json!({ "value": value }))
                        }
                    })
            }))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's ToggleGroup of Toggle items, one per string of `items`, with one pressed \
         at most: `value` is the pressed item's index. Pressing an item (a click, Space or \
         Enter) sends `change { value }` with its index, or -1 when it releases the pressed one.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::Number)
    .prop("axis", PropType::Enum(&["horizontal", "vertical"]))
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("toggleClass", PropType::String)
    .prop("pressedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::Number)])
}

fn tabs() -> Control {
    Control::new("Tabs", |props, events, _, cx| {
        let count = props.strings("items").len();
        props
            .styled(Tabs::new(props.id()), cx)
            .children(items(props).map(|(index, label, selected)| {
                props
                    .styled_by("tabClass", Tab::new(index), cx)
                    .selected(selected)
                    .disabled(props.boolean("disabled"))
                    .set_position(index + 1, count)
                    .styles(|styles| {
                        styles
                            .selected(|style| props.styled_by("selectedClass", style, cx))
                            .disabled(|style| props.styled_by("disabledClass", style, cx))
                    })
                    .child(label)
                    .on_click(events.handler("change", move |_| json!({ "value": index })))
            }))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Tabs list of Tab items, one per string of `items`: `value` is the selected \
         tab's index, and clicking a tab sends `change { value }`. The app shows the selected \
         tab's content itself.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::Number)
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("tabClass", PropType::String)
    .prop("selectedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::Number)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{TestAppContext, px};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    const ITEMS: [&str; 3] = ["Small", "Medium", "Large"];

    fn change(value: i64) -> Vec<(String, serde_json::Value)> {
        vec![("change".into(), json!({ "value": value }))]
    }

    /// Whether each item shows as checked (radios, toggles) or selected (tabs).
    fn states(fixture: &Fixture, cx: &mut TestAppContext, selected: bool) -> Vec<Option<bool>> {
        fixture.update(cx, |window, _| {
            let group = window.within(fixture.id.clone());
            (0..ITEMS.len())
                .map(|index| {
                    let item = group.find(index);
                    if selected {
                        item.selected()
                    } else {
                        item.checked()
                    }
                })
                .collect()
        })
    }

    fn click(fixture: &Fixture, cx: &mut TestAppContext, index: usize) {
        fixture.update(cx, |window, cx| {
            window.within(fixture.id.clone()).click(index, cx)
        });
    }

    fn press(fixture: &Fixture, cx: &mut TestAppContext, key: &str) {
        fixture.update(cx, |window, cx| window.press(key, cx));
    }

    #[gpui_kit::test]
    fn test_radio_group_builder(cx: &mut TestAppContext) {
        let group = Fixture::new(
            cx,
            "RadioGroup",
            json!({
                "items": ITEMS,
                "value": 1,
                "axis": "horizontal",
                "disabled": false,
                "rootClass": "flex gap_2",
                "radioClass": "flex items_center gap_1",
                "checkedClass": "text_primary",
                "disabledClass": "opacity_50",
            }),
        );
        let size = group.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        group.update(cx, |window, _| {
            let radio = window.within(group.id.clone()).find(1);
            assert_eq!(radio.role(), Some(gpui_kit::Role::RadioButton));
            assert_eq!(radio.checked(), Some(true));
        });
    }

    #[gpui_kit::test]
    fn test_radio_group_click_and_keys(cx: &mut TestAppContext) {
        let group = Fixture::new(cx, "RadioGroup", json!({ "items": ITEMS, "value": 0 }));
        click(&group, cx, 2);
        assert_eq!(group.take_events(), change(2));
        // The checked radio cannot uncheck itself.
        click(&group, cx, 2);
        assert!(group.take_events().is_empty());

        // Tab moves focus through the radios; Space and Enter activate.
        press(&group, cx, "tab");
        press(&group, cx, "space");
        assert_eq!(group.take_events(), change(0));
        press(&group, cx, "tab");
        press(&group, cx, "enter");
        assert_eq!(group.take_events(), change(1));
    }

    #[gpui_kit::test]
    fn test_radio_group_controlled(cx: &mut TestAppContext) {
        let group = Fixture::new(cx, "RadioGroup", json!({ "items": ITEMS }));
        assert_eq!(states(&group, cx, false), [Some(false); 3]);
        group.set_prop(cx, "value", Some(json!(2)));
        assert_eq!(
            states(&group, cx, false),
            [Some(false), Some(false), Some(true)]
        );
        // A click checks the radio before JS sets `value` back.
        click(&group, cx, 0);
        assert_eq!(group.take_events(), change(0));
        assert_eq!(
            states(&group, cx, false),
            [Some(true), Some(false), Some(false)]
        );
    }

    #[gpui_kit::test]
    fn test_radio_group_disabled(cx: &mut TestAppContext) {
        let group = Fixture::new(
            cx,
            "RadioGroup",
            json!({ "items": ITEMS, "value": 0, "disabled": true }),
        );
        click(&group, cx, 1);
        press(&group, cx, "tab");
        press(&group, cx, "space");
        assert!(group.take_events().is_empty());
        group.set_prop(cx, "disabled", None);
        click(&group, cx, 1);
        assert_eq!(group.take_events(), change(1));
    }

    #[gpui_kit::test]
    fn test_toggle_group_builder(cx: &mut TestAppContext) {
        let group = Fixture::new(
            cx,
            "ToggleGroup",
            json!({
                "items": ITEMS,
                "value": 0,
                "axis": "vertical",
                "disabled": false,
                "rootClass": "flex",
                "toggleClass": "px_2 py_1 border_1 border_border",
                "pressedClass": "bg_primary text_primary_foreground",
                "disabledClass": "opacity_50",
            }),
        );
        let size = group.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        assert_eq!(
            states(&group, cx, false),
            [Some(true), Some(false), Some(false)]
        );
    }

    #[gpui_kit::test]
    fn test_toggle_group_click_and_keys(cx: &mut TestAppContext) {
        let group = Fixture::new(cx, "ToggleGroup", json!({ "items": ITEMS, "value": 0 }));
        click(&group, cx, 1);
        assert_eq!(group.take_events(), change(1));
        // Pressing the pressed toggle releases it.
        click(&group, cx, 1);
        assert_eq!(group.take_events(), change(-1));

        // Tab moves focus on from the clicked toggle; Space and Enter press.
        press(&group, cx, "tab");
        press(&group, cx, "space");
        assert_eq!(group.take_events(), change(2));
        press(&group, cx, "tab");
        press(&group, cx, "enter");
        assert_eq!(group.take_events(), change(0));
    }

    #[gpui_kit::test]
    fn test_toggle_group_controlled(cx: &mut TestAppContext) {
        let group = Fixture::new(cx, "ToggleGroup", json!({ "items": ITEMS }));
        assert_eq!(states(&group, cx, false), [Some(false); 3]);
        group.set_prop(cx, "value", Some(json!(1)));
        assert_eq!(
            states(&group, cx, false),
            [Some(false), Some(true), Some(false)]
        );
    }

    #[gpui_kit::test]
    fn test_toggle_group_disabled(cx: &mut TestAppContext) {
        let group = Fixture::new(
            cx,
            "ToggleGroup",
            json!({ "items": ITEMS, "disabled": true }),
        );
        click(&group, cx, 0);
        press(&group, cx, "tab");
        press(&group, cx, "space");
        assert!(group.take_events().is_empty());
        group.set_prop(cx, "disabled", None);
        click(&group, cx, 0);
        assert_eq!(group.take_events(), change(0));
    }

    #[gpui_kit::test]
    fn test_tabs_builder(cx: &mut TestAppContext) {
        let tabs = Fixture::new(
            cx,
            "Tabs",
            json!({
                "items": ITEMS,
                "value": 2,
                "disabled": false,
                "rootClass": "flex border_b_1 border_border",
                "tabClass": "px_2 h_7",
                "selectedClass": "border_b_2 border_primary",
                "disabledClass": "opacity_50",
            }),
        );
        let size = tabs.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        tabs.update(cx, |window, _| {
            let tab = window.within(tabs.id.clone()).find(2);
            assert_eq!(tab.role(), Some(gpui_kit::Role::Tab));
            assert_eq!(tab.selected(), Some(true));
        });
    }

    #[gpui_kit::test]
    fn test_tabs_click(cx: &mut TestAppContext) {
        let tabs = Fixture::new(cx, "Tabs", json!({ "items": ITEMS, "value": 0 }));
        click(&tabs, cx, 1);
        assert_eq!(tabs.take_events(), change(1));
    }

    #[gpui_kit::test]
    fn test_tabs_controlled(cx: &mut TestAppContext) {
        let tabs = Fixture::new(cx, "Tabs", json!({ "items": ITEMS, "value": 0 }));
        assert_eq!(
            states(&tabs, cx, true),
            [Some(true), Some(false), Some(false)]
        );
        tabs.set_prop(cx, "value", Some(json!(2)));
        assert_eq!(
            states(&tabs, cx, true),
            [Some(false), Some(false), Some(true)]
        );
    }

    #[gpui_kit::test]
    fn test_tabs_disabled(cx: &mut TestAppContext) {
        let tabs = Fixture::new(cx, "Tabs", json!({ "items": ITEMS, "disabled": true }));
        click(&tabs, cx, 1);
        assert!(tabs.take_events().is_empty());
        tabs.set_prop(cx, "disabled", None);
        click(&tabs, cx, 1);
        assert_eq!(tabs.take_events(), change(1));
    }
}
