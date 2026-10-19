//! Controls choosing one item of a list in a popup: Select, Combobox.

use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui_kit::{
    AnyElement, App, AppContext as _, ElementId, Entity, FocusHandle, InteractiveElement as _,
    IntoElement as _, MouseButton, ParentElement as _, Role, ScrollHandle, SharedString,
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

/// What a select keeps between frames: its focus handles, the option Base
/// last asked to highlight, and the list's scroll position.
struct List {
    trigger: FocusHandle,
    /// The open list's focus; a combobox's query field takes it instead.
    content: FocusHandle,
    /// The highlighted option, by its index among the options shown.
    highlighted: Rc<Cell<Option<usize>>>,
    scroll: ScrollHandle,
}

impl List {
    fn new(cx: &mut App) -> Self {
        Self {
            trigger: cx.focus_handle(),
            content: cx.focus_handle(),
            highlighted: Rc::default(),
            scroll: ScrollHandle::new(),
        }
    }

    /// The highlighted index among `count` options shown. A closed list
    /// highlights nothing, so it opens on its selection.
    fn highlighted(&self, open: bool, count: usize) -> Option<usize> {
        if !open {
            self.highlighted.set(None);
        }
        self.highlighted.get().filter(|ix| *ix < count)
    }
}

/// The id of a control's part named `name`.
fn part(props: &Props, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), name.into())
}

/// Reports the choice of `item`, closes the popup and gives the trigger
/// back the focus, as Base's dismissal does.
fn choose(
    events: &Events,
    item: Option<&SharedString>,
    trigger: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(item) = item {
        events.emit("change", json!({ "value": item }));
    }
    events.emit("openChange", json!({ "open": false }));
    trigger.focus(window, cx);
}

/// The interaction-state class props of the trigger and of each option.
/// The trigger never takes focus itself: its `triggerFocusedClass` shows
/// while the root, which holds the focus, has it. Options never take focus
/// either: the keyboard highlights them (`highlightedItemClass`).
const TRIGGER_STATES: [&str; 3] = [
    "triggerHoverClass",
    "triggerActiveClass",
    "triggerFocusedClass",
];
const ITEM_STATES: [&str; 3] = ["itemHoverClass", "itemActiveClass", ""];

/// The trigger, which a click opens or closes. Opening focuses `content`.
fn trigger(
    props: &Props,
    events: &Events,
    content: &FocusHandle,
    focused: bool,
    cx: &App,
) -> Button {
    let open = props.boolean("open");
    let events = events.clone();
    let content = content.clone();
    props
        .states(
            TRIGGER_STATES,
            props.styled_by("triggerClass", Button::new(part(props, "trigger")), cx),
            cx,
        )
        // The root holds the focus, so its keyboard focus shows here.
        .when(focused, |this| {
            props.styled_by("triggerFocusedClass", this, cx)
        })
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
            if !open {
                content.focus(window, cx);
            }
        })
}

/// Records the option Base asks to highlight, and draws it highlighted.
fn on_highlight(list: &List) -> impl Fn(usize, &mut Window, &mut App) + 'static {
    let highlighted = list.highlighted.clone();
    move |ix, window, _| {
        if highlighted.replace(Some(ix)) != Some(ix) {
            window.refresh();
        }
    }
}

/// The list of `shown` options, styled by class prop `class`, the option
/// of the `value` item selected and the one at `highlighted` highlighted.
/// It takes the focus of `list.content` when `focused`.
#[allow(clippy::too_many_arguments)]
fn options(
    props: &Props,
    class: &str,
    focused: bool,
    events: &Events,
    list: &List,
    shown: &Rc<[SharedString]>,
    highlighted: Option<usize>,
    cx: &App,
) -> AnyElement {
    let value = props.string("value");
    props
        .styled_by(
            class,
            div()
                .id(part(props, "list"))
                .role(Role::ListBox)
                .when(focused, |this| this.track_focus(&list.content))
                .track_scroll(&list.scroll)
                .overflow_y_scroll(),
            cx,
        )
        .children(shown.iter().enumerate().map(|(ix, item)| {
            let selected = value.as_ref() == Some(item);
            let is_highlighted = highlighted == Some(ix);
            let events = events.clone();
            let trigger = list.trigger.clone();
            let item = item.clone();
            props
                .states(
                    ITEM_STATES,
                    props.styled_by(
                        "itemClass",
                        Button::new(part(props, format!("item-{item}"))),
                        cx,
                    ),
                    cx,
                )
                .role(Role::ListBoxOption)
                .focusable(false)
                .selected(selected)
                .aria_selected(selected)
                .styles(|styles| {
                    styles.selected(|style| props.styled_by("selectedItemClass", style, cx))
                })
                .when(is_highlighted, |this| {
                    props
                        .styled_by("highlightedItemClass", this, cx)
                        .aria_active_descendant()
                })
                .child(item.clone())
                .on_click(move |_, window, cx| choose(&events, Some(&item), &trigger, window, cx))
        }))
        .into_any_element()
}

/// Reports Base's requests to open or close as `openChange`.
fn on_open_change(events: &Events) -> impl Fn(bool, &mut Window, &mut App) + 'static {
    let handler = events.handler("openChange", |open: &bool| json!({ "open": open }));
    move |open, window, cx| handler(&open, window, cx)
}

/// Chooses the highlighted option, or keeps the selected one, when Base
/// confirms.
fn on_confirm(
    events: &Events,
    list: &List,
    item: Option<SharedString>,
) -> impl Fn(&mut Window, &mut App) + 'static {
    let events = events.clone();
    let trigger = list.trigger.clone();
    move |window, cx| choose(&events, item.as_ref(), &trigger, window, cx)
}

/// Reports a press outside the open popup as a request to close it.
fn on_dismiss(events: &Events) -> impl Fn(&mut Window, &mut App) + 'static {
    let events = events.clone();
    move |_, _| events.emit("openChange", json!({ "open": false }))
}

fn select() -> Control {
    Control::stateful(
        "Select",
        |_, _, _, cx| List::new(cx),
        |list, props, events, window, cx| {
            let shown: Rc<[SharedString]> = props.strings("items").into();
            let value = props.string("value");
            let selected = value
                .as_ref()
                .and_then(|value| shown.iter().position(|item| item == value));
            let open = props.boolean("open");
            let highlighted = list.highlighted(open, shown.len());
            props
                .styled(Select::new(props.id()), cx)
                .open(open)
                .disabled(props.boolean("disabled"))
                .focus_handle(&list.trigger)
                .content_focus_handle(&list.content)
                .options(shown.iter().cloned())
                .selected_index(selected)
                .highlighted_index(highlighted)
                .on_highlight(on_highlight(list))
                .scroll_handle(&list.scroll)
                .when_some(props.string("accessibilityLabel"), |this, label| {
                    this.accessibility_label(label)
                })
                .when_some(value, |this, value| this.accessibility_value(value))
                .on_open_change(on_open_change(events))
                .on_confirm(on_confirm(
                    events,
                    list,
                    highlighted.map(|ix| shown[ix].clone()),
                ))
                .child(
                    Popup::new(
                        part(props, "popup"),
                        trigger(
                            props,
                            events,
                            &list.content,
                            list.trigger.is_focused(window),
                            cx,
                        ),
                    )
                    .on_dismiss(on_dismiss(events))
                    .when(open, |this| {
                        this.content(options(
                            props,
                            "contentClass",
                            true,
                            events,
                            list,
                            &shown,
                            highlighted,
                            cx,
                        ))
                    }),
                )
                .into_any_element()
        },
    )
    .focus(|list: &List, _| list.trigger.clone())
    .documentation(
        "GPUI Base's Select: a trigger showing `value` that opens a list of `items` in a popup. \
         Click, Enter, Space, an arrow key or typing opens it. In the list, the arrow keys, Home, \
         End, Page Up and Page Down move the highlighted option (as does the pointer), typing the \
         start of an item highlights it (Space included, within a search), Enter or a click \
         chooses, and Escape, Tab or a click outside closes it.",
    )
    .prop("items", PropType::Strings)
    .prop("value", PropType::String)
    .prop("placeholder", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .states(TRIGGER_STATES)
    .prop("placeholderClass", PropType::String)
    .prop("contentClass", PropType::String)
    .prop("itemClass", PropType::String)
    .states(ITEM_STATES)
    .prop("selectedItemClass", PropType::String)
    .prop("highlightedItemClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("openChange", &[("open", PropType::Boolean)])
}

/// A combobox's list and the state of its query field.
struct ComboboxState {
    list: List,
    query: Entity<InputState>,
    _changes: Subscription,
}

fn combobox() -> Control {
    Control::stateful(
        "Combobox",
        |_, events, window, cx| {
            let query = cx.new(|cx| InputState::new(window, cx));
            let mut list = List::new(cx);
            list.content = query.read(cx).presentation().focus_handle().clone();
            let events = events.clone();
            let highlighted = list.highlighted.clone();
            let changes = cx.subscribe(&query, move |query, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    // Other options show: the highlight starts over.
                    highlighted.set(None);
                    events.emit("queryChange", json!({ "query": query.read(cx).value() }));
                }
            });
            ComboboxState {
                list,
                query,
                _changes: changes,
            }
        },
        |state, props, events, window, cx| {
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
            let list = &state.list;
            // The items whose text contains the query, ignoring case.
            let needle = query_text.to_lowercase();
            let shown: Rc<[SharedString]> = props
                .strings("items")
                .into_iter()
                .filter(|item| item.to_lowercase().contains(&needle))
                .collect();
            let value = props.string("value");
            let selected = value
                .as_ref()
                .and_then(|value| shown.iter().position(|item| item == value));
            let open = props.boolean("open");
            let highlighted = list.highlighted(open, shown.len());
            props
                .styled(Combobox::new(props.id()), cx)
                .open(open)
                .disabled(props.boolean("disabled"))
                .focus_handle(&list.trigger)
                .content_focus_handle(&list.content)
                .options(shown.iter().cloned())
                .selected_index(selected)
                .highlighted_index(highlighted)
                .on_highlight(on_highlight(list))
                .scroll_handle(&list.scroll)
                .on_open_change(on_open_change(events))
                .on_confirm(on_confirm(
                    events,
                    list,
                    highlighted.map(|ix| shown[ix].clone()),
                ))
                .child(
                    Popup::new(
                        part(props, "popup"),
                        trigger(
                            props,
                            events,
                            &list.content,
                            list.trigger.is_focused(window),
                            cx,
                        ),
                    )
                    .on_dismiss(on_dismiss(events))
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
                                        .focused(list.content.is_focused(window))
                                        .styles(|styles| {
                                            styles.focused(|style| {
                                                props.styled_by("inputFocusedClass", style, cx)
                                            })
                                        })
                                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                            query.update(cx, |query, cx| query.focus(window, cx));
                                        })
                                        .child(state.query.clone()),
                                )
                                .child(options(
                                    props,
                                    "listClass",
                                    false,
                                    events,
                                    list,
                                    &shown,
                                    highlighted,
                                    cx,
                                )),
                        )
                    }),
                )
                .into_any_element()
        },
    )
    .focus(|state: &ComboboxState, _| state.list.trigger.clone())
    .documentation(
        "GPUI Base's Combobox: a trigger showing `value` that opens a query field over the \
         `items` containing the query (ignoring case). Opening focuses the query field, which \
         keeps the focus: Up, Down, Page Up and Page Down move the highlighted option, Enter or \
         a click chooses, and Escape, Tab or a click outside closes it.",
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
    .states(TRIGGER_STATES)
    .prop("placeholderClass", PropType::String)
    .prop("contentClass", PropType::String)
    .prop("inputClass", PropType::String)
    .prop("inputFocusedClass", PropType::String)
    .prop("listClass", PropType::String)
    .prop("itemClass", PropType::String)
    .states(ITEM_STATES)
    .prop("selectedItemClass", PropType::String)
    .prop("highlightedItemClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("openChange", &[("open", PropType::Boolean)])
    .event("queryChange", &[("query", PropType::String)])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui_kit::{
        ElementId, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, PlatformInput,
        TestAppContext, point, px,
    };
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    /// The id of the fixture control's part `name`.
    fn part(fixture: &Fixture, name: &str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), name.to_owned().into())
    }

    fn event(name: &str, data: Value) -> (String, Value) {
        (name.into(), data)
    }

    fn opened(open: bool) -> (String, Value) {
        event("openChange", json!({ "open": open }))
    }

    fn changed(value: &str) -> (String, Value) {
        event("change", json!({ "value": value }))
    }

    /// Presses and releases the mouse at a window position far from the
    /// control.
    fn press_outside(window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
        let position = point(px(700.), px(500.));
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    }

    /// A control with the focus, unless disabled (no Tab stop then).
    fn focused(cx: &mut TestAppContext, name: &str, props: Value) -> Fixture {
        let fixture = Fixture::new(cx, name, props);
        if let Some(focus) = fixture.focus_handle(cx) {
            fixture.update(cx, |window, cx| focus.focus(window, cx));
        }
        fixture
    }

    const FRUITS: [&str; 5] = ["Apple", "Apricot", "Banana", "Blueberry", "Cherry"];

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
                "triggerHoverClass": "bg_accent",
                "triggerActiveClass": "bg_accent",
                "triggerFocusedClass": "border_ring",
                "placeholderClass": "text_muted_foreground",
                "contentClass": "p_1 bg_background",
                "itemClass": "px_2 py_1",
                "itemHoverClass": "bg_accent",
                "itemActiveClass": "bg_accent",
                "selectedItemClass": "font_semibold",
                "highlightedItemClass": "bg_accent",
                "disabledClass": "opacity_50",
            }),
        );
        let size = select.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        select.update(cx, |window, _| {
            let root = window.find(select.id.clone());
            assert_eq!(root.label(), Some("Fruit"));
            assert_eq!(root.value(), Some("Banana"));
            assert_eq!(root.expanded(), Some(true));
            assert_eq!(
                window.find(part(&select, "item-Banana")).selected(),
                Some(true)
            );
            assert_ne!(
                window.find(part(&select, "item-Apple")).selected(),
                Some(true)
            );
        });
    }

    #[gpui_kit::test]
    fn test_select_click(cx: &mut TestAppContext) {
        let select = focused(
            cx,
            "Select",
            json!({ "items": ["Apple", "Banana"], "placeholder": "Fruit" }),
        );
        select.update(cx, |window, cx| window.click(part(&select, "trigger"), cx));
        assert_eq!(select.take_events(), vec![opened(true)]);
        select.update(cx, |window, cx| {
            window.click(part(&select, "item-Banana"), cx)
        });
        assert_eq!(select.take_events(), vec![changed("Banana"), opened(false)]);
        let trigger = select.focus_handle(cx).unwrap();
        select.update(cx, |window, _| {
            assert!(window.try_find(part(&select, "item-Banana")).is_none());
            assert_eq!(window.find(select.id.clone()).value(), Some("Banana"));
            // Choosing hands the focus back to the trigger.
            assert!(trigger.is_focused(window));
        });
    }

    /// The arrows open the list on the selection and move the highlight,
    /// Home and End jump to the ends, Enter chooses the highlighted option
    /// and gives the trigger the focus back.
    #[gpui_kit::test]
    fn test_select_keyboard(cx: &mut TestAppContext) {
        let select = focused(cx, "Select", json!({ "items": FRUITS, "value": "Banana" }));
        let trigger = select.focus_handle(cx).unwrap();
        select.update(cx, |window, cx| window.press("down", cx));
        assert_eq!(select.take_events(), vec![opened(true)]);
        for key in ["down", "down", "up"] {
            select.update(cx, |window, cx| window.press(key, cx));
        }
        select.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            select.take_events(),
            vec![changed("Blueberry"), opened(false)]
        );
        select.update(cx, |window, _| assert!(trigger.is_focused(window)));

        for key in ["enter", "end", "home", "enter"] {
            select.update(cx, |window, cx| window.press(key, cx));
        }
        assert_eq!(
            select.take_events(),
            vec![opened(true), changed("Apple"), opened(false)]
        );

        // Escape closes and keeps the value.
        for key in ["space", "down", "escape"] {
            select.update(cx, |window, cx| window.press(key, cx));
        }
        assert_eq!(select.take_events(), vec![opened(true), opened(false)]);
        select.update(cx, |window, _| {
            assert!(window.try_find(part(&select, "item-Apple")).is_none());
            assert!(trigger.is_focused(window));
        });
    }

    /// A closed list opens on its selection: a highlight moved before
    /// Escape is gone. Enter with nothing highlighted keeps the value.
    #[gpui_kit::test]
    fn test_select_reopens_on_the_selection(cx: &mut TestAppContext) {
        let select = focused(cx, "Select", json!({ "items": FRUITS, "value": "Apple" }));
        for key in ["down", "end", "escape", "enter", "enter"] {
            select.update(cx, |window, cx| window.press(key, cx));
        }
        assert_eq!(
            select.take_events(),
            vec![opened(true), opened(false), opened(true), opened(false)]
        );
        for key in ["enter", "down", "enter"] {
            select.update(cx, |window, cx| window.press(key, cx));
        }
        assert_eq!(
            select.take_events(),
            vec![opened(true), changed("Apricot"), opened(false)]
        );
    }

    /// Typing the start of an item opens the list on it; repeating a letter
    /// steps through the items starting with it.
    #[gpui_kit::test]
    fn test_select_typeahead(cx: &mut TestAppContext) {
        let select = focused(cx, "Select", json!({ "items": FRUITS }));
        select.update(cx, |window, cx| window.press("b", cx));
        assert_eq!(select.take_events(), vec![opened(true)]);
        select.update(cx, |window, cx| window.press("b", cx));
        select.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            select.take_events(),
            vec![changed("Blueberry"), opened(false)]
        );

        for key in ["a", "p", "r", "enter"] {
            select.update(cx, |window, cx| window.press(key, cx));
        }
        assert_eq!(
            select.take_events(),
            vec![opened(true), changed("Apricot"), opened(false)]
        );
    }

    /// Base highlights the option the pointer moves over, which Enter chooses.
    #[gpui_kit::test]
    fn test_select_hover_highlights(cx: &mut TestAppContext) {
        let select = focused(cx, "Select", json!({ "items": FRUITS }));
        select.update(cx, |window, cx| window.press("enter", cx));
        select.update(cx, |window, cx| {
            window.hover(part(&select, "item-Cherry"), cx)
        });
        select.update(cx, |window, cx| window.press("up", cx));
        select.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            select.take_events(),
            vec![opened(true), changed("Blueberry"), opened(false)]
        );
    }

    #[gpui_kit::test]
    fn test_select_click_outside(cx: &mut TestAppContext) {
        let select = focused(cx, "Select", json!({ "items": FRUITS }));
        select.update(cx, |window, cx| window.click(part(&select, "trigger"), cx));
        select.update(cx, press_outside);
        assert_eq!(select.take_events(), vec![opened(true), opened(false)]);
        select.update(cx, |window, _| {
            assert!(window.try_find(part(&select, "item-Apple")).is_none());
        });
        // A closed select asks nothing of a press outside.
        select.update(cx, press_outside);
        assert!(select.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_select_controlled(cx: &mut TestAppContext) {
        let select = Fixture::new(cx, "Select", json!({ "items": ["Apple", "Banana"] }));
        select.set_prop(cx, "value", Some(json!("Apple")));
        select.set_prop(cx, "open", Some(json!(true)));
        select.update(cx, |window, _| {
            assert_eq!(window.find(select.id.clone()).value(), Some("Apple"));
            assert_eq!(
                window.find(part(&select, "item-Apple")).selected(),
                Some(true)
            );
        });
        select.set_prop(cx, "open", Some(json!(false)));
        select.update(cx, |window, _| {
            assert!(window.try_find(part(&select, "item-Apple")).is_none());
        });
        assert!(select.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_select_disabled(cx: &mut TestAppContext) {
        let select = focused(
            cx,
            "Select",
            json!({ "items": ["Apple"], "disabled": true }),
        );
        assert!(select.focus_handle(cx).is_none());
        select.update(cx, |window, cx| {
            assert_eq!(window.find(select.id.clone()).disabled(), Some(true));
            window.click(part(&select, "trigger"), cx);
            window.press("down", cx);
            window.press("enter", cx);
            window.press("a", cx);
        });
        assert!(select.take_events().is_empty());
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
                "triggerHoverClass": "bg_accent",
                "triggerActiveClass": "bg_accent",
                "triggerFocusedClass": "border_ring",
                "placeholderClass": "text_muted_foreground",
                "contentClass": "p_1",
                "inputClass": "px_2 h_7",
                "inputFocusedClass": "border_ring",
                "listClass": "p_1",
                "itemClass": "px_2 py_1",
                "itemHoverClass": "bg_accent",
                "itemActiveClass": "bg_accent",
                "selectedItemClass": "font_semibold",
                "highlightedItemClass": "bg_accent",
                "disabledClass": "opacity_50",
            }),
        );
        let size = combobox.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        combobox.update(cx, |window, _| {
            assert!(window.try_find(part(&combobox, "query")).is_some());
            // Only the items containing the query, ignoring case.
            assert!(window.try_find(part(&combobox, "item-Apple")).is_none());
            assert!(window.try_find(part(&combobox, "item-Banana")).is_some());
            assert!(window.try_find(part(&combobox, "item-Cherry")).is_none());
        });
    }

    #[gpui_kit::test]
    fn test_combobox_click_and_type(cx: &mut TestAppContext) {
        let combobox = focused(
            cx,
            "Combobox",
            json!({ "items": ["Apple", "Banana", "Cherry"] }),
        );
        combobox.update(cx, |window, cx| {
            window.click(part(&combobox, "trigger"), cx)
        });
        assert_eq!(combobox.take_events(), vec![opened(true)]);
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
            assert!(window.try_find(part(&combobox, "item-Apple")).is_none());
        });
        combobox.update(cx, |window, cx| {
            window.click(part(&combobox, "item-Cherry"), cx)
        });
        assert_eq!(
            combobox.take_events(),
            vec![changed("Cherry"), opened(false)]
        );
    }

    /// The query field keeps the focus while Up and Down move the highlight
    /// through the matching items; Enter chooses it.
    #[gpui_kit::test]
    fn test_combobox_keyboard(cx: &mut TestAppContext) {
        let combobox = focused(cx, "Combobox", json!({ "items": FRUITS }));
        let trigger = combobox.focus_handle(cx).unwrap();
        combobox.update(cx, |window, cx| window.press("down", cx));
        assert_eq!(combobox.take_events(), vec![opened(true)]);
        combobox.update(cx, |window, cx| window.input("b", cx));
        assert_eq!(
            combobox.take_events(),
            vec![event("queryChange", json!({ "query": "b" }))]
        );
        // "b" matches Banana and Blueberry.
        for key in ["down", "down", "down", "up", "enter"] {
            combobox.update(cx, |window, cx| window.press(key, cx));
        }
        assert_eq!(
            combobox.take_events(),
            vec![changed("Banana"), opened(false)]
        );
        combobox.update(cx, |window, _| assert!(trigger.is_focused(window)));

        combobox.update(cx, |window, cx| window.press("down", cx));
        combobox.update(cx, |window, cx| window.press("escape", cx));
        assert_eq!(combobox.take_events(), vec![opened(true), opened(false)]);
        // Base's dismissal gives focus back to the trigger.
        combobox.update(cx, |window, _| assert!(trigger.is_focused(window)));
    }

    #[gpui_kit::test]
    fn test_combobox_click_outside(cx: &mut TestAppContext) {
        let combobox = focused(cx, "Combobox", json!({ "items": FRUITS }));
        combobox.update(cx, |window, cx| {
            window.click(part(&combobox, "trigger"), cx)
        });
        // A press on the query field is inside the popup.
        combobox.update(cx, |window, cx| window.click(part(&combobox, "query"), cx));
        assert_eq!(combobox.take_events(), vec![opened(true)]);
        combobox.update(cx, press_outside);
        assert_eq!(combobox.take_events(), vec![opened(false)]);
    }

    #[gpui_kit::test]
    fn test_combobox_controlled(cx: &mut TestAppContext) {
        let combobox = Fixture::new(
            cx,
            "Combobox",
            json!({ "items": ["Apple", "Banana"], "open": true }),
        );
        combobox.update(cx, |window, _| {
            assert!(window.try_find(part(&combobox, "item-Apple")).is_some());
        });
        combobox.set_prop(cx, "query", Some(json!("nan")));
        combobox.set_prop(cx, "value", Some(json!("Banana")));
        combobox.update(cx, |window, _| {
            assert!(window.try_find(part(&combobox, "item-Apple")).is_none());
            assert_eq!(
                window.find(part(&combobox, "item-Banana")).selected(),
                Some(true)
            );
        });
        // Setting the query is not the user's change.
        assert!(combobox.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_combobox_disabled(cx: &mut TestAppContext) {
        let combobox = focused(
            cx,
            "Combobox",
            json!({ "items": ["Apple"], "disabled": true }),
        );
        combobox.update(cx, |window, cx| {
            window.click(part(&combobox, "trigger"), cx);
            window.press("down", cx);
        });
        assert!(combobox.take_events().is_empty());
    }
}
