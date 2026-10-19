//! Toolbar: GPUI Base's Toolbar of Button and Toggle items in ToolbarGroups.
//!
//! The items come from the `items` prop: an item with `pressed` is a Toggle,
//! one without is a Button, and `{ separator: true }` draws the app's
//! separator. Consecutive items naming the same `group` sit in one
//! ToolbarGroup, announced by that name. Base's Toolbar moves focus between
//! the items with Left and Right.

use gpui_kit::{
    AnyElement, App, ElementId, IntoElement as _, ParentElement as _, SharedString,
    base::{Button, Toggle, Toolbar, ToolbarGroup},
    div,
    prelude::FluentBuilder as _,
    svg,
};
use serde::Deserialize;
use serde_json::json;

use crate::{Control, Events, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![toolbar()]
}

/// The interaction-state class props of each item.
const ITEM_STATES: [&str; 3] = ["itemHoverClass", "itemActiveClass", "itemFocusClass"];

/// One entry of the `items` prop.
#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Separator {
        #[allow(dead_code)]
        separator: bool,
    },
    Item(Item),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    id: SharedString,
    label: Option<SharedString>,
    icon: Option<SharedString>,
    accessibility_label: Option<SharedString>,
    pressed: Option<bool>,
    #[serde(default)]
    disabled: bool,
    group: Option<SharedString>,
}

impl Entry {
    fn group(&self) -> Option<&SharedString> {
        match self {
            Entry::Item(item) => item.group.as_ref(),
            Entry::Separator { .. } => None,
        }
    }
}

/// An item: a Toggle when it has `pressed`, else a Button.
fn item(item: Item, props: &Props, events: &Events, cx: &App) -> AnyElement {
    let disabled = props.boolean("disabled") || item.disabled;
    let id = ElementId::Name(item.id.clone());
    let icon = item
        .icon
        .map(|icon| props.styled_by("iconClass", svg().path(icon), cx));
    match item.pressed {
        Some(pressed) => {
            let events = events.clone();
            let item_id = item.id.clone();
            props
                .states(
                    ITEM_STATES,
                    props.styled_by("itemClass", Toggle::new(id), cx),
                    cx,
                )
                .pressed(pressed)
                .disabled(disabled)
                .styles(|styles| {
                    styles
                        .pressed(|style| props.styled_by("pressedClass", style, cx))
                        .disabled(|style| props.styled_by("disabledClass", style, cx))
                })
                .when_some(item.accessibility_label, |this, label| {
                    this.accessibility_label(label)
                })
                .children(icon)
                .children(item.label)
                .on_change(move |pressed, _, _, _| {
                    events.emit("change", json!({ "id": item_id, "pressed": pressed }))
                })
                .into_any_element()
        }
        None => {
            let item_id = item.id.clone();
            props
                .states(
                    ITEM_STATES,
                    props.styled_by("itemClass", Button::new(id), cx),
                    cx,
                )
                .disabled(disabled)
                .styles(|styles| {
                    styles.disabled(|style| props.styled_by("disabledClass", style, cx))
                })
                .when_some(item.accessibility_label, |this, label| {
                    this.accessibility_label(label)
                })
                .children(icon)
                .children(item.label)
                .on_click(events.handler("click", move |_| json!({ "id": item_id })))
                .into_any_element()
        }
    }
}

fn entry(entry: Entry, props: &Props, events: &Events, cx: &App) -> AnyElement {
    match entry {
        Entry::Separator { .. } => props
            .styled_by("separatorClass", div(), cx)
            .into_any_element(),
        Entry::Item(it) => item(it, props, events, cx),
    }
}

fn toolbar() -> Control {
    Control::new("Toolbar", |props, events, _, cx| {
        let mut entries = props
            .json::<Vec<Entry>>("items")
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .peekable();
        let mut children = Vec::new();
        while let Some((index, first)) = entries.next() {
            let Some(group) = first.group().cloned() else {
                children.push(entry(first, props, events, cx));
                continue;
            };
            let mut members = vec![entry(first, props, events, cx)];
            while let Some((_, next)) = entries.next_if(|(_, next)| next.group() == Some(&group)) {
                members.push(entry(next, props, events, cx));
            }
            children.push(
                props
                    .styled_by("groupClass", ToolbarGroup::new(index), cx)
                    .label(group)
                    .children(members)
                    .into_any_element(),
            );
        }
        props
            .styled(Toolbar::new(props.id()), cx)
            .disabled(props.boolean("disabled"))
            .children(children)
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Toolbar: a row of items, each a Toggle when it has `pressed` (sending \
         `change { id, pressed }`) or a Button (sending `click { id }`), and \
         `{ separator: true }` entries drawn by `separatorClass`. Consecutive items naming the \
         same `group` sit in one ToolbarGroup announced by that name. `icon` names an SVG in \
         the host's assets (`icons/bold.svg`). Left and Right move focus between the items.",
    )
    .prop(
        "items",
        PropType::Json(
            "({ id: string; label?: string; icon?: string; accessibilityLabel?: string; \
             pressed?: boolean; disabled?: boolean; group?: string } | { separator: true })[]",
        ),
    )
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("groupClass", PropType::String)
    .prop("itemClass", PropType::String)
    .states(ITEM_STATES)
    .prop("pressedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .prop("separatorClass", PropType::String)
    .prop("iconClass", PropType::String)
    .event("click", &[("id", PropType::String)])
    .event(
        "change",
        &[("id", PropType::String), ("pressed", PropType::Boolean)],
    )
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Role, TestAppContext, px};
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    fn items() -> Value {
        json!([
            { "id": "undo", "icon": "icons/undo.svg", "accessibilityLabel": "Undo", "group": "History" },
            { "id": "redo", "icon": "icons/redo.svg", "accessibilityLabel": "Redo", "group": "History" },
            { "separator": true },
            { "id": "bold", "label": "B", "pressed": true, "group": "Format" },
            { "id": "italic", "label": "I", "pressed": false, "group": "Format" },
            { "id": "share", "label": "Share", "disabled": true },
        ])
    }

    /// A Toolbar of `items()` with `props`; the icon-only items need a size.
    fn fixture(cx: &mut TestAppContext, mut props: Value) -> Fixture {
        props["items"] = items();
        props["iconClass"] = json!("size_4");
        Fixture::new(cx, "Toolbar", props)
    }

    fn click(toolbar: &Fixture, cx: &mut TestAppContext, id: &'static str) {
        toolbar.update(cx, |window, cx| window.click(id, cx));
    }

    fn press(toolbar: &Fixture, cx: &mut TestAppContext, key: &str) {
        toolbar.update(cx, |window, cx| window.press(key, cx));
    }

    fn focused(toolbar: &Fixture, cx: &mut TestAppContext, id: &'static str) -> Option<bool> {
        toolbar.update(cx, |window, _| window.find(id).focused())
    }

    #[gpui_kit::test]
    fn test_toolbar_builder(cx: &mut TestAppContext) {
        let toolbar = Fixture::new(
            cx,
            "Toolbar",
            json!({
                "items": items(),
                "disabled": false,
                "rootClass": "flex items_center gap_1 p_1",
                "groupClass": "gap_1",
                "itemClass": "h_8 px_2 rounded_md",
                "itemHoverClass": "bg_accent",
                "itemActiveClass": "bg_muted",
                "itemFocusClass": "border_ring",
                "pressedClass": "bg_accent",
                "disabledClass": "opacity_50",
                "separatorClass": "w_px h_5 bg_border",
                "iconClass": "size_4",
            }),
        );
        let size = toolbar.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        toolbar.update(cx, |window, _| {
            let undo = window.find("undo");
            assert_eq!(undo.role(), Some(Role::Button));
            assert_eq!(undo.label(), Some("Undo"));
            assert_eq!(window.find("bold").checked(), Some(true));
            assert_eq!(window.find("italic").checked(), Some(false));
            assert_eq!(window.find("share").disabled(), Some(true));
        });
        // Each run of a `group` is one ToolbarGroup, keyed by its first
        // entry's index; `share` has none.
        toolbar.update(cx, |window, _| {
            assert!(window.within(0usize).try_find("redo").is_some());
            assert!(window.within(3usize).try_find("italic").is_some());
            assert!(window.within(3usize).try_find("share").is_none());
        });
    }

    #[gpui_kit::test]
    fn test_toolbar_click_and_change(cx: &mut TestAppContext) {
        let toolbar = fixture(cx, json!({}));
        click(&toolbar, cx, "undo");
        assert_eq!(
            toolbar.take_events(),
            vec![("click".into(), json!({ "id": "undo" }))]
        );
        click(&toolbar, cx, "bold");
        assert_eq!(
            toolbar.take_events(),
            vec![("change".into(), json!({ "id": "bold", "pressed": false }))]
        );
        // Space on the focused toggle asks to flip it.
        press(&toolbar, cx, "right");
        press(&toolbar, cx, "space");
        assert_eq!(
            toolbar.take_events(),
            vec![("change".into(), json!({ "id": "italic", "pressed": true }))]
        );
    }

    #[gpui_kit::test]
    fn test_toolbar_keyboard(cx: &mut TestAppContext) {
        let toolbar = fixture(cx, json!({}));
        click(&toolbar, cx, "undo");
        toolbar.take_events();
        // Right moves into the next group, past the separator; Left wraps
        // from the first item to the last focusable one.
        press(&toolbar, cx, "right");
        assert_eq!(focused(&toolbar, cx, "redo"), Some(true));
        press(&toolbar, cx, "right");
        assert_eq!(focused(&toolbar, cx, "bold"), Some(true));
        press(&toolbar, cx, "left");
        press(&toolbar, cx, "left");
        assert_eq!(focused(&toolbar, cx, "undo"), Some(true));
        press(&toolbar, cx, "left");
        assert_eq!(focused(&toolbar, cx, "italic"), Some(true));
        press(&toolbar, cx, "enter");
        assert_eq!(
            toolbar.take_events(),
            vec![("change".into(), json!({ "id": "italic", "pressed": true }))]
        );
    }

    #[gpui_kit::test]
    fn test_toolbar_controlled(cx: &mut TestAppContext) {
        let toolbar = fixture(cx, json!({}));
        let mut changed = items();
        changed[3]["pressed"] = json!(false);
        changed[4]["pressed"] = json!(true);
        changed[5]["disabled"] = json!(false);
        toolbar.set_prop(cx, "items", Some(changed));
        toolbar.update(cx, |window, _| {
            assert_eq!(window.find("bold").checked(), Some(false));
            assert_eq!(window.find("italic").checked(), Some(true));
        });
        click(&toolbar, cx, "share");
        assert_eq!(
            toolbar.take_events(),
            vec![("click".into(), json!({ "id": "share" }))]
        );
    }

    #[gpui_kit::test]
    fn test_toolbar_disabled(cx: &mut TestAppContext) {
        let toolbar = fixture(cx, json!({ "disabled": true }));
        click(&toolbar, cx, "undo");
        click(&toolbar, cx, "bold");
        click(&toolbar, cx, "share");
        assert!(toolbar.take_events().is_empty());
        toolbar.update(cx, |window, _| {
            assert_eq!(window.find("bold").disabled(), Some(true));
        });

        toolbar.set_prop(cx, "disabled", None);
        click(&toolbar, cx, "undo");
        click(&toolbar, cx, "share");
        assert_eq!(
            toolbar.take_events(),
            vec![("click".into(), json!({ "id": "undo" }))]
        );
    }
}
