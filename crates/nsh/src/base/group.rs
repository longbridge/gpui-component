//! Controls choosing among a set of items: RadioGroup, ToggleGroup, Tabs.
//!
//! Each builds GPUI Base's group root with one Base item per string of the
//! `items` prop. The selected item is the `value` prop, an index; an item's
//! selection request is sent as `change { value }`. The group is one Tab
//! stop, and GPUI Base moves focus between its items with the arrow keys.

use gpui_kit::{
    Axis, IntoElement as _, ParentElement as _, SharedString,
    base::{Radio, RadioGroup, Tab, Tabs, Toggle, ToggleGroup},
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![radio_group(), toggle_group(), tabs()]
}

/// The interaction-state class props of each item.
const RADIO_STATES: [&str; 3] = ["radioHoverClass", "radioActiveClass", "radioFocusClass"];
const TOGGLE_STATES: [&str; 3] = ["toggleHoverClass", "toggleActiveClass", "toggleFocusClass"];
const TAB_STATES: [&str; 3] = ["tabHoverClass", "tabActiveClass", "tabFocusClass"];

/// An item of the `items` prop.
struct Item {
    index: usize,
    label: SharedString,
    /// Whether it is the `value` prop's index.
    selected: bool,
    /// Whether the group is `disabled` or `disabledItems` lists it.
    disabled: bool,
}

fn items(props: &Props) -> impl Iterator<Item = Item> {
    let value = props.has("value").then(|| props.number("value"));
    let group_disabled = props.boolean("disabled");
    let disabled_items = props
        .json::<Vec<usize>>("disabledItems")
        .unwrap_or_default();
    props
        .strings("items")
        .into_iter()
        .enumerate()
        .map(move |(index, label)| Item {
            index,
            label,
            selected: value == Some(index as f64),
            disabled: group_disabled || disabled_items.contains(&index),
        })
}

/// The `axis` prop.
fn axis(props: &Props, default: Axis) -> Axis {
    match props.string("axis").as_deref() {
        Some("horizontal") => Axis::Horizontal,
        Some("vertical") => Axis::Vertical,
        _ => default,
    }
}

const DISABLED_ITEMS: &str = "disabledItems";
const DISABLED_ITEMS_TYPE: PropType = PropType::Json("number[]");

fn radio_group() -> Control {
    Control::focusable("RadioGroup", |focus, props, events, _, cx| {
        let count = props.strings("items").len();
        props
            .styled(RadioGroup::new(props.id()), cx)
            .track_focus(focus)
            .axis(axis(props, Axis::Vertical))
            .children(items(props).map(|item| {
                let index = item.index;
                props
                    .states(
                        RADIO_STATES,
                        props.styled_by("radioClass", Radio::new(index), cx),
                        cx,
                    )
                    .checked(item.selected)
                    .disabled(item.disabled)
                    .set_position(index + 1, count)
                    .styles(|styles| {
                        styles
                            .checked(|style| props.styled_by("checkedClass", style, cx))
                            .disabled(|style| props.styled_by("disabledClass", style, cx))
                    })
                    .child(item.label)
                    .on_change({
                        let events = events.clone();
                        move |_, _, _, _| events.emit("change", json!({ "value": index }))
                    })
            }))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's RadioGroup of Radio items, one per string of `items`: `value` is the \
         checked item's index, and activating another item sends `change { value }`. The group \
         is one Tab stop, on the checked item. Any arrow key moves focus to the previous or next \
         enabled item and checks it; Space and Enter check the focused item.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::Number)
    .prop("axis", PropType::Enum(&["vertical", "horizontal"]))
    .prop("disabled", PropType::Boolean)
    .prop(DISABLED_ITEMS, DISABLED_ITEMS_TYPE)
    .prop("rootClass", PropType::String)
    .prop("radioClass", PropType::String)
    .states(RADIO_STATES)
    .prop("checkedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::Number)])
}

fn toggle_group() -> Control {
    Control::focusable("ToggleGroup", |focus, props, events, _, cx| {
        props
            .styled(ToggleGroup::new(props.id()), cx)
            .track_focus(focus)
            .axis(axis(props, Axis::Horizontal))
            .children(items(props).map(|item| {
                let index = item.index;
                props
                    .states(
                        TOGGLE_STATES,
                        props.styled_by("toggleClass", Toggle::new(index), cx),
                        cx,
                    )
                    .pressed(item.selected)
                    .disabled(item.disabled)
                    .styles(|styles| {
                        styles
                            .pressed(|style| props.styled_by("pressedClass", style, cx))
                            .disabled(|style| props.styled_by("disabledClass", style, cx))
                    })
                    .child(item.label)
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
         Enter) sends `change { value }` with its index, or -1 when it releases the pressed one. \
         The group is one Tab stop, on the item focused last. The arrow keys of its `axis` move \
         focus to the previous or next enabled item, and Home and End to the first and last, \
         without pressing anything.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::Number)
    .prop("axis", PropType::Enum(&["horizontal", "vertical"]))
    .prop("disabled", PropType::Boolean)
    .prop(DISABLED_ITEMS, DISABLED_ITEMS_TYPE)
    .prop("rootClass", PropType::String)
    .prop("toggleClass", PropType::String)
    .states(TOGGLE_STATES)
    .prop("pressedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::Number)])
}

fn tabs() -> Control {
    Control::focusable("Tabs", |focus, props, events, _, cx| {
        let count = props.strings("items").len();
        props
            .styled(Tabs::new(props.id()), cx)
            .track_focus(focus)
            .axis(axis(props, Axis::Horizontal))
            .when(props.has("activateOnFocus"), |this| {
                this.activate_on_focus(props.boolean("activateOnFocus"))
            })
            .children(items(props).map(|item| {
                let index = item.index;
                props
                    .states(
                        TAB_STATES,
                        props.styled_by("tabClass", Tab::new(index), cx),
                        cx,
                    )
                    .selected(item.selected)
                    .disabled(item.disabled)
                    .set_position(index + 1, count)
                    .styles(|styles| {
                        styles
                            .selected(|style| props.styled_by("selectedClass", style, cx))
                            .disabled(|style| props.styled_by("disabledClass", style, cx))
                    })
                    .child(item.label)
                    .on_click(events.handler("change", move |_| json!({ "value": index })))
            }))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Tabs list of Tab items, one per string of `items`: `value` is the selected \
         tab's index, and clicking a tab sends `change { value }` (a click leaves focus where it \
         was). The list is one Tab stop, on the selected tab. The arrow keys of its `axis` move \
         focus to the previous or next enabled tab, and Home and End to the first and last; \
         moving focus selects the tab unless `activateOnFocus` is false, when Space or Enter \
         selects it. The app shows the selected tab's content itself.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::Number)
    .prop("axis", PropType::Enum(&["horizontal", "vertical"]))
    .prop("activateOnFocus", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop(DISABLED_ITEMS, DISABLED_ITEMS_TYPE)
    .prop("rootClass", PropType::String)
    .prop("tabClass", PropType::String)
    .states(TAB_STATES)
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

    /// The index of the item with keyboard focus.
    fn focused(fixture: &Fixture, cx: &mut TestAppContext) -> Option<usize> {
        fixture.update(cx, |window, _| {
            let group = window.within(fixture.id.clone());
            (0..ITEMS.len()).find(|index| group.find(*index).focused() == Some(true))
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

    /// Focuses the control as NSH's Tab does: through its focus handle.
    fn tab_in(fixture: &Fixture, cx: &mut TestAppContext) {
        let focus = fixture.focus_handle(cx).expect("the group is a Tab stop");
        fixture.update(cx, |window, cx| window.focus(&focus, cx));
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
                "disabledItems": [2],
                "rootClass": "flex gap_2",
                "radioClass": "flex items_center gap_1",
                "radioHoverClass": "bg_accent",
                "radioActiveClass": "opacity_80",
                "radioFocusClass": "border_1 border_ring",
                "checkedClass": "text_primary",
                "disabledClass": "opacity_50",
            }),
        );
        let size = group.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        group.update(cx, |window, _| {
            let group = window.within(group.id.clone());
            let radio = group.find(1);
            assert_eq!(radio.role(), Some(gpui_kit::Role::RadioButton));
            assert_eq!(radio.checked(), Some(true));
            assert_eq!(group.find(2).disabled(), Some(true));
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

        // Every arrow moves focus and checks, wrapping at the ends.
        press(&group, cx, "down");
        assert_eq!(group.take_events(), change(0));
        assert_eq!(focused(&group, cx), Some(0));
        press(&group, cx, "left");
        assert_eq!(group.take_events(), change(2));
        press(&group, cx, "up");
        assert_eq!(group.take_events(), change(1));
        press(&group, cx, "right");
        assert_eq!(group.take_events(), change(2));
        assert_eq!(focused(&group, cx), Some(2));
    }

    #[gpui_kit::test]
    fn test_radio_group_tab_stop_and_disabled_items(cx: &mut TestAppContext) {
        let group = Fixture::new(
            cx,
            "RadioGroup",
            json!({ "items": ITEMS, "value": 2, "disabledItems": [1] }),
        );
        // Tab lands on the checked radio.
        tab_in(&group, cx);
        assert_eq!(focused(&group, cx), Some(2));
        // The disabled radio is skipped.
        press(&group, cx, "up");
        assert_eq!(group.take_events(), change(0));
        assert_eq!(focused(&group, cx), Some(0));
        click(&group, cx, 1);
        assert!(group.take_events().is_empty());
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
        // The Tab stop follows the checked radio.
        group.set_prop(cx, "value", Some(json!(1)));
        group.update(cx, |window, cx| window.blur(cx));
        tab_in(&group, cx);
        assert_eq!(focused(&group, cx), Some(1));
    }

    #[gpui_kit::test]
    fn test_radio_group_disabled(cx: &mut TestAppContext) {
        let group = Fixture::new(
            cx,
            "RadioGroup",
            json!({ "items": ITEMS, "value": 0, "disabled": true }),
        );
        assert!(group.focus_handle(cx).is_none());
        click(&group, cx, 1);
        press(&group, cx, "down");
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
                "disabledItems": [1],
                "rootClass": "flex",
                "toggleClass": "px_2 py_1 border_1 border_border",
                "toggleHoverClass": "bg_accent",
                "toggleActiveClass": "opacity_80",
                "toggleFocusClass": "border_ring",
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

        // The axis' arrows and Home/End move focus without pressing.
        press(&group, cx, "right");
        assert_eq!(focused(&group, cx), Some(2));
        press(&group, cx, "right");
        assert_eq!(focused(&group, cx), Some(0));
        press(&group, cx, "end");
        assert_eq!(focused(&group, cx), Some(2));
        press(&group, cx, "home");
        assert_eq!(focused(&group, cx), Some(0));
        press(&group, cx, "down");
        assert_eq!(focused(&group, cx), Some(0));
        assert!(group.take_events().is_empty());

        // Space and Enter press the focused toggle.
        press(&group, cx, "space");
        assert_eq!(group.take_events(), change(0));
        press(&group, cx, "end");
        press(&group, cx, "enter");
        assert_eq!(group.take_events(), change(2));
    }

    #[gpui_kit::test]
    fn test_toggle_group_tab_stop_and_vertical_axis(cx: &mut TestAppContext) {
        let group = Fixture::new(
            cx,
            "ToggleGroup",
            json!({ "items": ITEMS, "axis": "vertical", "disabledItems": [1] }),
        );
        tab_in(&group, cx);
        assert_eq!(focused(&group, cx), Some(0));
        press(&group, cx, "right");
        assert_eq!(focused(&group, cx), Some(0));
        press(&group, cx, "down");
        assert_eq!(focused(&group, cx), Some(2));
        // Tab returns to the toggle focused last.
        group.update(cx, |window, cx| window.blur(cx));
        tab_in(&group, cx);
        assert_eq!(focused(&group, cx), Some(2));
        assert!(group.take_events().is_empty());
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
        assert!(group.focus_handle(cx).is_none());
        click(&group, cx, 0);
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
                "axis": "vertical",
                "activateOnFocus": false,
                "disabled": false,
                "disabledItems": [0],
                "rootClass": "flex flex_col border_r_1 border_border",
                "tabClass": "px_2 h_7",
                "tabHoverClass": "bg_accent",
                "tabActiveClass": "opacity_80",
                "tabFocusClass": "border_1 border_ring",
                "selectedClass": "border_r_2 border_primary",
                "disabledClass": "opacity_50",
            }),
        );
        let size = tabs.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        tabs.update(cx, |window, _| {
            let list = window.within(tabs.id.clone());
            let tab = list.find(2);
            assert_eq!(tab.role(), Some(gpui_kit::Role::Tab));
            assert_eq!(tab.selected(), Some(true));
            assert_eq!(list.find(0).disabled(), Some(true));
        });
    }

    #[gpui_kit::test]
    fn test_tabs_click_and_keys(cx: &mut TestAppContext) {
        let tabs = Fixture::new(cx, "Tabs", json!({ "items": ITEMS, "value": 0 }));
        // A click selects the tab and leaves focus where it was.
        click(&tabs, cx, 1);
        assert_eq!(tabs.take_events(), change(1));
        assert_eq!(focused(&tabs, cx), None);

        // Tab lands on the selected tab; moving focus selects.
        tab_in(&tabs, cx);
        assert_eq!(focused(&tabs, cx), Some(1));
        press(&tabs, cx, "right");
        assert_eq!(tabs.take_events(), change(2));
        press(&tabs, cx, "right");
        assert_eq!(tabs.take_events(), change(0));
        press(&tabs, cx, "end");
        assert_eq!(tabs.take_events(), change(2));
        press(&tabs, cx, "home");
        assert_eq!(tabs.take_events(), change(0));
        // A horizontal list leaves Up and Down alone.
        press(&tabs, cx, "down");
        assert!(tabs.take_events().is_empty());
        assert_eq!(focused(&tabs, cx), Some(0));
    }

    #[gpui_kit::test]
    fn test_tabs_vertical_without_activation_on_focus(cx: &mut TestAppContext) {
        let tabs = Fixture::new(
            cx,
            "Tabs",
            json!({
                "items": ITEMS,
                "value": 0,
                "axis": "vertical",
                "activateOnFocus": false,
                "disabledItems": [1],
            }),
        );
        tab_in(&tabs, cx);
        press(&tabs, cx, "down");
        assert_eq!(focused(&tabs, cx), Some(2));
        assert!(tabs.take_events().is_empty());
        press(&tabs, cx, "enter");
        assert_eq!(tabs.take_events(), change(2));
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
        tab_in(&tabs, cx);
        assert_eq!(focused(&tabs, cx), Some(2));
    }

    #[gpui_kit::test]
    fn test_tabs_disabled(cx: &mut TestAppContext) {
        let tabs = Fixture::new(cx, "Tabs", json!({ "items": ITEMS, "disabled": true }));
        assert!(tabs.focus_handle(cx).is_none());
        click(&tabs, cx, 1);
        assert!(tabs.take_events().is_empty());
        tabs.set_prop(cx, "disabled", None);
        click(&tabs, cx, 1);
        assert_eq!(tabs.take_events(), change(1));
    }
}
