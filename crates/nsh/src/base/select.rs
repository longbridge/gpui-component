//! Controls choosing one item of a list in a popup: Select, Combobox.

use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, AppContext as _, ElementId, Entity, FocusHandle, InteractiveElement as _,
    IntoElement as _, MouseButton, ParentElement as _, Role, SharedString,
    StatefulInteractiveElement as _, Subscription, Window,
    base::{
        Button, Combobox, InputBase, Popup, Select,
        input::{InputEvent, InputState},
    },
    div,
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, Events, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![select(), combobox()]
}

/// The focus handles of a select's trigger and of its options, by item.
struct Focus {
    trigger: FocusHandle,
    options: Vec<FocusHandle>,
}

impl Focus {
    fn new(cx: &mut App) -> Self {
        Self {
            trigger: cx.focus_handle(),
            options: Vec::new(),
        }
    }

    /// One option handle for each of `count` items.
    fn sync(&mut self, count: usize, cx: &mut App) {
        self.options.resize_with(count, || cx.focus_handle());
    }
}

/// The id of a control's part named `name`.
fn part(props: &Props, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), name.into())
}

/// Reports the choice of `item` and closes the popup.
fn choose(events: &Events, item: &SharedString) {
    events.emit("change", json!({ "value": item }));
    events.emit("openChange", json!({ "open": false }));
}

/// The trigger, which a click opens or closes. Opening focuses `focus_on_open`,
/// as Base's Combobox showcase focuses its query field.
fn trigger(props: &Props, events: &Events, focus_on_open: Option<FocusHandle>, cx: &App) -> Button {
    let open = props.boolean("open");
    let events = events.clone();
    props
        .styled_by("triggerClass", Button::new(part(props, "trigger")), cx)
        // The root is the Tab stop; a press focuses it through the trigger.
        .focusable(false)
        .disabled(props.boolean("disabled"))
        .styles(|styles| styles.disabled(|style| props.styled_by("disabledClass", style, cx)))
        .map(
            |this| match props.string("value").filter(|value| !value.is_empty()) {
                Some(value) => this.child(value),
                None => this.child(props.styled_by(
                    "placeholderClass",
                    div().child(props.string("placeholder").unwrap_or_default()),
                    cx,
                )),
            },
        )
        .on_click(move |_, window, cx| {
            events.emit("openChange", json!({ "open": !open }));
            if let Some(handle) = focus_on_open.as_ref().filter(|_| !open) {
                handle.focus(window, cx);
            }
        })
}

/// The list of `items` (with their indices), styled by class prop `rootClass`,
/// the option of the `value` item selected.
fn options(
    props: &Props,
    class: &str,
    events: &Events,
    focus: &Focus,
    items: impl Iterator<Item = (usize, SharedString)>,
    cx: &App,
) -> AnyElement {
    let value = props.string("value");
    props
        .styled_by(class, div().id(part(props, "list")).role(Role::ListBox), cx)
        .children(items.map(|(ix, item)| {
            let selected = value.as_ref() == Some(&item);
            let events = events.clone();
            props
                .styled_by(
                    "itemClass",
                    Button::new(part(props, format!("item-{ix}"))),
                    cx,
                )
                .role(Role::ListBoxOption)
                .track_focus(&focus.options[ix])
                .selected(selected)
                .aria_selected(selected)
                .styles(|styles| {
                    styles.selected(|style| props.styled_by("selectedItemClass", style, cx))
                })
                .child(item.clone())
                .on_click(move |_, _, _| choose(&events, &item))
        }))
        .into_any_element()
}

/// Reports Base's requests to open or close as `openChange`.
fn on_open_change(events: &Events) -> impl Fn(bool, &mut Window, &mut App) + 'static {
    let handler = events.handler("openChange", |open: &bool| json!({ "open": open }));
    move |open, window, cx| handler(&open, window, cx)
}

/// Chooses the item whose option has focus when Base confirms.
fn on_confirm(
    events: &Events,
    focus: &Focus,
    items: Vec<SharedString>,
) -> impl Fn(&mut Window, &mut App) + 'static {
    let events = events.clone();
    let options = focus.options.clone();
    move |window, _| {
        if let Some(ix) = options.iter().position(|handle| handle.is_focused(window)) {
            choose(&events, &items[ix]);
        }
    }
}

fn select() -> Control {
    Control::stateful(
        "Select",
        |_, _, _, cx| Focus::new(cx),
        |focus, props, events, _, cx| {
            let items = props.strings("items");
            focus.sync(items.len(), cx);
            let value = props.string("value");
            // Keyboard opening lands on the selected option, or the first.
            let content = value
                .as_ref()
                .and_then(|value| items.iter().position(|item| item == value))
                .or((!items.is_empty()).then_some(0))
                .map(|ix| focus.options[ix].clone());
            let open = props.boolean("open");
            props
                .styled(Select::new(props.id()), cx)
                .open(open)
                .disabled(props.boolean("disabled"))
                .focus_handle(&focus.trigger)
                .when_some(content.clone(), |this, content| {
                    this.content_focus_handle(&content)
                })
                .when_some(props.string("accessibilityLabel"), |this, label| {
                    this.accessibility_label(label)
                })
                .when_some(value, |this, value| this.accessibility_value(value))
                .on_open_change(on_open_change(events))
                .on_confirm(on_confirm(events, focus, items.clone()))
                .child(
                    Popup::new(part(props, "popup"), trigger(props, events, None, cx)).when(
                        open,
                        |this| {
                            this.content(options(
                                props,
                                "contentClass",
                                events,
                                focus,
                                items.into_iter().enumerate(),
                                cx,
                            ))
                        },
                    ),
                )
                .into_any_element()
        },
    )
    .focus(|focus: &Focus, _| focus.trigger.clone())
    .documentation(
        "GPUI Base's Select: a trigger showing `value` that opens a list of `items` in a popup. \
         Click, Enter, Space or an arrow key opens it and focuses the selected option; Tab moves \
         between options, Enter or a click chooses one, and Escape closes it.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::String)
    .prop("placeholder", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .prop("placeholderClass", PropType::String)
    .prop("contentClass", PropType::String)
    .prop("itemClass", PropType::String)
    .prop("selectedItemClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("openChange", &[("open", PropType::Boolean)])
}

/// A combobox's focus handles and the state of its query field.
struct ComboboxState {
    focus: Focus,
    query: Entity<InputState>,
    _changes: Subscription,
}

fn combobox() -> Control {
    Control::stateful(
        "Combobox",
        |_, events, window, cx| {
            let query = cx.new(|cx| InputState::new(window, cx));
            let events = events.clone();
            let changes = cx.subscribe(&query, move |query, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    events.emit("queryChange", json!({ "query": query.read(cx).value() }));
                }
            });
            ComboboxState {
                focus: Focus::new(cx),
                query,
                _changes: changes,
            }
        },
        |state, props, events, window, cx| {
            let items = props.strings("items");
            state.focus.sync(items.len(), cx);
            let query_text = props.string("query").unwrap_or_default();
            let placeholder = props.string("queryPlaceholder").unwrap_or_default();
            state.query.update(cx, |query, cx| {
                if props.changed("query") {
                    query.set_value(query_text.clone(), window, cx);
                }
                if props.changed("queryPlaceholder") {
                    query.set_placeholder(placeholder, window, cx);
                }
            });
            let content = state.query.read(cx).presentation().focus_handle().clone();
            // The items whose text contains the query, ignoring case.
            let needle = query_text.to_lowercase();
            let matches = items
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, item)| item.to_lowercase().contains(&needle));
            let open = props.boolean("open");
            let focus = &state.focus;
            props
                .styled(Combobox::new(props.id()), cx)
                .open(open)
                .disabled(props.boolean("disabled"))
                .focus_handle(&focus.trigger)
                .content_focus_handle(&content)
                .on_open_change(on_open_change(events))
                .on_confirm(on_confirm(events, focus, items.clone()))
                .child(
                    Popup::new(
                        part(props, "popup"),
                        trigger(props, events, Some(content.clone()), cx),
                    )
                    .when(open, |this| {
                        let query = state.query.clone();
                        this.content(
                            props
                                .styled_by("contentClass", div(), cx)
                                .child(
                                    props
                                        .styled_by(
                                            "inputClass",
                                            InputBase::new(part(props, "query")),
                                            cx,
                                        )
                                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                            query.update(cx, |query, cx| query.focus(window, cx));
                                        })
                                        .child(state.query.clone()),
                                )
                                .child(options(props, "listClass", events, focus, matches, cx)),
                        )
                    }),
                )
                .into_any_element()
        },
    )
    .focus(|state: &ComboboxState, _| state.focus.trigger.clone())
    .documentation(
        "GPUI Base's Combobox: a trigger showing `value` that opens a query field over the \
         `items` containing the query (ignoring case). Opening focuses the query field; Tab moves \
         to the options, Enter or a click chooses one, and Escape closes it.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::String)
    .prop("placeholder", PropType::String)
    .prop("query", PropType::String)
    .prop("queryPlaceholder", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .prop("placeholderClass", PropType::String)
    .prop("contentClass", PropType::String)
    .prop("inputClass", PropType::String)
    .prop("listClass", PropType::String)
    .prop("itemClass", PropType::String)
    .prop("selectedItemClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("openChange", &[("open", PropType::Boolean)])
    .event("queryChange", &[("query", PropType::String)])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui_kit::{ElementId, TestAppContext};
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    /// The id of the fixture control's part `name`.
    fn part(fixture: &Fixture, name: &str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), name.to_owned().into())
    }

    fn event(name: &str, data: Value) -> (String, Value) {
        (name.into(), data)
    }

    fn select(cx: &mut TestAppContext, props: Value) -> Fixture {
        let fixture = Fixture::new(cx, "Select", props);
        let focus = fixture.focus_handle(cx).expect("Select is a Tab stop");
        fixture.update(cx, |window, cx| focus.focus(window, cx));
        fixture
    }

    #[gpui_kit::test]
    fn test_select_builder(cx: &mut TestAppContext) {
        let select = Fixture::new(
            cx,
            "Select",
            json!({
                "items": ["Apple", "Banana"],
                "value": "Banana",
                "placeholder": "Fruit",
                "open": true,
                "accessibilityLabel": "Fruit",
                "rootClass": "w_40",
                "triggerClass": "px_3 py_1 border_1 border_border",
                "placeholderClass": "text_muted_foreground",
                "contentClass": "p_1 bg_popover",
                "itemClass": "px_2 py_1",
                "selectedItemClass": "bg_accent",
                "disabledClass": "opacity_50",
            }),
        );
        let size = select.measure(cx);
        assert!(size.width > gpui_kit::px(0.) && size.height > gpui_kit::px(0.));
        select.update(cx, |window, _| {
            let root = window.find(select.id.clone());
            assert_eq!(root.label(), Some("Fruit"));
            assert_eq!(root.value(), Some("Banana"));
            assert!(window.find(part(&select, "item-1")).selected() == Some(true));
            assert_ne!(window.find(part(&select, "item-0")).selected(), Some(true));
        });
    }

    #[gpui_kit::test]
    fn test_select_click(cx: &mut TestAppContext) {
        let select = select(
            cx,
            json!({ "items": ["Apple", "Banana"], "placeholder": "Fruit" }),
        );
        let trigger = part(&select, "trigger");
        select.update(cx, |window, cx| window.click(trigger.clone(), cx));
        assert_eq!(
            select.take_events(),
            vec![event("openChange", json!({ "open": true }))]
        );
        select.update(cx, |window, cx| window.click(part(&select, "item-1"), cx));
        assert_eq!(
            select.take_events(),
            vec![
                event("change", json!({ "value": "Banana" })),
                event("openChange", json!({ "open": false })),
            ]
        );
        select.update(cx, |window, _| {
            assert!(window.try_find(part(&select, "item-1")).is_none());
            assert_eq!(window.find(select.id.clone()).value(), Some("Banana"));
        });
    }

    #[gpui_kit::test]
    fn test_select_keyboard(cx: &mut TestAppContext) {
        let select = select(
            cx,
            json!({ "items": ["Apple", "Banana", "Cherry"], "value": "Banana" }),
        );
        select.update(cx, |window, cx| window.press("down", cx));
        assert_eq!(
            select.take_events(),
            vec![event("openChange", json!({ "open": true }))]
        );
        select.update(cx, |window, cx| {
            assert!(window.find(part(&select, "item-1")).focused() == Some(true));
            window.press("tab", cx);
        });
        select.update(cx, |window, cx| {
            assert!(window.find(part(&select, "item-2")).focused() == Some(true));
            window.press("enter", cx);
        });
        assert_eq!(
            select.take_events(),
            vec![
                event("change", json!({ "value": "Cherry" })),
                event("openChange", json!({ "open": false })),
            ]
        );
        // Choosing leaves focus where Base's confirmation put it; the
        // trigger takes it back as a Tab or a click would give it.
        let trigger = select.focus_handle(cx).unwrap();
        select.update(cx, |window, cx| trigger.focus(window, cx));
        select.update(cx, |window, cx| window.press("enter", cx));
        select.update(cx, |window, cx| window.press("escape", cx));
        assert_eq!(
            select.take_events(),
            vec![
                event("openChange", json!({ "open": true })),
                event("openChange", json!({ "open": false })),
            ]
        );
        select.update(cx, |window, _| {
            assert!(window.try_find(part(&select, "item-0")).is_none());
            assert!(trigger.is_focused(window));
        });
    }

    #[gpui_kit::test]
    fn test_select_controlled(cx: &mut TestAppContext) {
        let select = Fixture::new(cx, "Select", json!({ "items": ["Apple", "Banana"] }));
        select.set_prop(cx, "value", Some(json!("Apple")));
        select.set_prop(cx, "open", Some(json!(true)));
        select.update(cx, |window, _| {
            assert_eq!(window.find(select.id.clone()).value(), Some("Apple"));
            assert!(window.find(part(&select, "item-0")).selected() == Some(true));
        });
        select.set_prop(cx, "open", Some(json!(false)));
        select.update(cx, |window, _| {
            assert!(window.try_find(part(&select, "item-0")).is_none());
        });
        assert!(select.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_select_disabled(cx: &mut TestAppContext) {
        let select = select(cx, json!({ "items": ["Apple"], "disabled": true }));
        select.update(cx, |window, cx| {
            assert!(window.find(select.id.clone()).disabled() == Some(true));
            window.click(part(&select, "trigger"), cx);
            window.press("down", cx);
            window.press("enter", cx);
        });
        assert!(select.take_events().is_empty());
    }

    fn combobox(cx: &mut TestAppContext, props: Value) -> Fixture {
        let fixture = Fixture::new(cx, "Combobox", props);
        let focus = fixture.focus_handle(cx).expect("Combobox is a Tab stop");
        fixture.update(cx, |window, cx| focus.focus(window, cx));
        fixture
    }

    #[gpui_kit::test]
    fn test_combobox_builder(cx: &mut TestAppContext) {
        let combobox = Fixture::new(
            cx,
            "Combobox",
            json!({
                "items": ["Apple", "Banana", "Cherry"],
                "value": "Cherry",
                "placeholder": "Fruit",
                "query": "an",
                "queryPlaceholder": "Search",
                "open": true,
                "rootClass": "w_40",
                "triggerClass": "px_3 py_1",
                "placeholderClass": "text_muted_foreground",
                "contentClass": "p_1",
                "inputClass": "px_2 h_7",
                "listClass": "p_1",
                "itemClass": "px_2 py_1",
                "selectedItemClass": "bg_accent",
                "disabledClass": "opacity_50",
            }),
        );
        let size = combobox.measure(cx);
        assert!(size.width > gpui_kit::px(0.) && size.height > gpui_kit::px(0.));
        combobox.update(cx, |window, _| {
            assert!(window.try_find(part(&combobox, "query")).is_some());
            // Only the items containing the query, ignoring case.
            assert!(window.try_find(part(&combobox, "item-0")).is_none());
            assert!(window.try_find(part(&combobox, "item-1")).is_some());
            assert!(window.try_find(part(&combobox, "item-2")).is_none());
        });
    }

    #[gpui_kit::test]
    fn test_combobox_click_and_type(cx: &mut TestAppContext) {
        let combobox = combobox(cx, json!({ "items": ["Apple", "Banana", "Cherry"] }));
        combobox.update(cx, |window, cx| {
            window.click(part(&combobox, "trigger"), cx)
        });
        assert_eq!(
            combobox.take_events(),
            vec![event("openChange", json!({ "open": true }))]
        );
        // Opening focused the query field, which takes the typing.
        combobox.update(cx, |window, cx| window.input("C", cx));
        combobox.update(cx, |window, cx| window.input("H", cx));
        assert_eq!(
            combobox.take_events(),
            vec![
                event("queryChange", json!({ "query": "C" })),
                event("queryChange", json!({ "query": "CH" })),
            ]
        );
        combobox.update(cx, |window, _| {
            assert!(window.try_find(part(&combobox, "item-0")).is_none());
        });
        combobox.update(cx, |window, cx| window.click(part(&combobox, "item-2"), cx));
        assert_eq!(
            combobox.take_events(),
            vec![
                event("change", json!({ "value": "Cherry" })),
                event("openChange", json!({ "open": false })),
            ]
        );
    }

    #[gpui_kit::test]
    fn test_combobox_keyboard(cx: &mut TestAppContext) {
        let combobox = combobox(cx, json!({ "items": ["Apple", "Banana"] }));
        combobox.update(cx, |window, cx| window.press("down", cx));
        assert_eq!(
            combobox.take_events(),
            vec![event("openChange", json!({ "open": true }))]
        );
        combobox.update(cx, |window, cx| window.input("a", cx));
        assert_eq!(
            combobox.take_events(),
            vec![event("queryChange", json!({ "query": "a" }))]
        );
        combobox.update(cx, |window, cx| window.press("tab", cx));
        combobox.update(cx, |window, cx| {
            assert!(window.find(part(&combobox, "item-0")).focused() == Some(true));
            window.press("enter", cx);
        });
        assert_eq!(
            combobox.take_events(),
            vec![
                event("change", json!({ "value": "Apple" })),
                event("openChange", json!({ "open": false })),
            ]
        );
        let trigger = combobox.focus_handle(cx).unwrap();
        combobox.update(cx, |window, cx| trigger.focus(window, cx));
        combobox.update(cx, |window, cx| window.press("down", cx));
        combobox.update(cx, |window, cx| window.press("escape", cx));
        assert_eq!(
            combobox.take_events(),
            vec![
                event("openChange", json!({ "open": true })),
                event("openChange", json!({ "open": false })),
            ]
        );
        // Base's dismissal gives focus back to the trigger.
        combobox.update(cx, |window, _| assert!(trigger.is_focused(window)));
    }

    #[gpui_kit::test]
    fn test_combobox_controlled(cx: &mut TestAppContext) {
        let combobox = Fixture::new(
            cx,
            "Combobox",
            json!({ "items": ["Apple", "Banana"], "open": true }),
        );
        combobox.update(cx, |window, _| {
            assert!(window.try_find(part(&combobox, "item-0")).is_some());
        });
        combobox.set_prop(cx, "query", Some(json!("nan")));
        combobox.set_prop(cx, "value", Some(json!("Banana")));
        combobox.update(cx, |window, _| {
            assert!(window.try_find(part(&combobox, "item-0")).is_none());
            assert!(window.find(part(&combobox, "item-1")).selected() == Some(true));
        });
        // Setting the query is not the user's change.
        assert!(combobox.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_combobox_disabled(cx: &mut TestAppContext) {
        let combobox = combobox(cx, json!({ "items": ["Apple"], "disabled": true }));
        combobox.update(cx, |window, cx| {
            window.click(part(&combobox, "trigger"), cx);
            window.press("down", cx);
        });
        assert!(combobox.take_events().is_empty());
    }
}
