//! Controls showing and hiding content: Accordion, Collapsible.
//!
//! Both are controlled: the open state is a prop, and a trigger's request to
//! open or close is sent as an event for JS to set the prop back. Base's
//! [`AccordionTrigger`] is the disclosure button of both, so each trigger
//! reports its expanded state and is activated by a click, Enter or Space.

use std::sync::Arc;

use gpui_kit::{
    App, ElementId, FocusHandle, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString,
    base::{
        Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger, Collapsible,
    },
    div,
    prelude::FluentBuilder as _,
    svg,
};
use serde::Deserialize;
use serde_json::json;

use crate::{Control, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![accordion(), collapsible()]
}

/// The interaction-state class props of a trigger.
const TRIGGER_STATES: [&str; 3] = [
    "triggerHoverClass",
    "triggerActiveClass",
    "triggerFocusClass",
];

/// The id of a control's part named `name`.
fn part(props: &Props, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), name.into())
}

/// A trigger with the classes of its state, a title and the icon of its
/// state, an SVG in the host's assets the app names (`openIcon` falls back
/// to `icon`).
fn trigger(
    props: &Props,
    trigger: AccordionTrigger,
    title: SharedString,
    open: bool,
    disabled: bool,
    cx: &App,
) -> AccordionTrigger {
    let icon = match open {
        true => props.string("openIcon").or_else(|| props.string("icon")),
        false => props.string("icon"),
    };
    let trigger = props.styled_by("triggerClass", trigger, cx);
    let trigger = match disabled {
        true => trigger,
        false => props.state_styles(TRIGGER_STATES, cx).apply(trigger),
    };
    trigger
        .open(open)
        .disabled(disabled)
        .when(open, |this| props.styled_by("openTriggerClass", this, cx))
        .when(disabled, |this| {
            props.styled_by("disabledTriggerClass", this, cx)
        })
        .child(title)
        .when_some(icon, |this, icon| {
            this.child(props.styled_by("iconClass", svg().path(icon), cx))
        })
}

/// One item of Accordion's `items` prop.
#[derive(Deserialize)]
struct Item {
    title: SharedString,
    content: SharedString,
    #[serde(default)]
    disabled: bool,
}

/// The focus handles of an accordion's triggers, by item; one at least, so
/// the control always has a Tab stop to report.
struct Triggers(Vec<FocusHandle>);

impl Triggers {
    fn sync(&mut self, count: usize, cx: &mut App) {
        self.0
            .resize_with(count.max(1), || cx.focus_handle().tab_stop(true));
    }
}

fn accordion() -> Control {
    Control::stateful(
        "Accordion",
        |_, _, _, cx| Triggers(vec![cx.focus_handle().tab_stop(true)]),
        |triggers, props, events, _, cx| {
            let items = props.json::<Vec<Item>>("items").unwrap_or_default();
            let value = props.json::<Vec<usize>>("value").unwrap_or_default();
            triggers.sync(items.len(), cx);
            props
                .styled(Accordion::new(props.id()), cx)
                .multiple(props.boolean("multiple"))
                .on_change(events.handler("change", |value: &[usize]| json!({ "value": value })))
                .items(items.into_iter().enumerate().map(|(index, item)| {
                    let open = value.contains(&index);
                    let disabled = props.boolean("disabled") || item.disabled;
                    let base = AccordionTrigger::new(index)
                        .when(!disabled, |this| this.track_focus(&triggers.0[index]));
                    props
                        .styled_by("itemClass", AccordionItem::new(), cx)
                        .open(open)
                        .disabled(disabled)
                        .when(open, |this| props.styled_by("openItemClass", this, cx))
                        .header(
                            props
                                .styled_by(
                                    "headerClass",
                                    AccordionHeader::new(trigger(
                                        props, base, item.title, open, disabled, cx,
                                    )),
                                    cx,
                                )
                                .id(part(props, format!("header-{index}"))),
                        )
                        .panel(
                            props
                                .styled_by("panelClass", AccordionPanel::new(), cx)
                                .id(part(props, format!("panel-{index}")))
                                .child(item.content),
                        )
                }))
                .into_any_element()
        },
    )
    .focus(|triggers: &Triggers, _| triggers.0[0].clone())
    .documentation(
        "GPUI Base's Accordion: one item per entry of `items`, each a header whose trigger \
         shows or hides the item's panel. `value` lists the open items' indexes; a trigger's \
         click, Enter or Space sends `change { value }`, the open items Base requests: the \
         item opened (alone, unless `multiple`) or closed. `icon` and `openIcon` name SVGs in the host's assets \
         (`icons/chevron-down.svg`) drawn after each title.",
    )
    .prop(
        "items",
        PropType::Json("{ title: string; content: string; disabled?: boolean }[]"),
    )
    .prop("value", PropType::Json("number[]"))
    .prop("multiple", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("icon", PropType::String)
    .prop("openIcon", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("itemClass", PropType::String)
    .prop("openItemClass", PropType::String)
    .prop("headerClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .states(TRIGGER_STATES)
    .prop("openTriggerClass", PropType::String)
    .prop("disabledTriggerClass", PropType::String)
    .prop("iconClass", PropType::String)
    .prop("panelClass", PropType::String)
    .event("change", &[("value", PropType::Json("number[]"))])
}

fn collapsible() -> Control {
    Control::focusable("Collapsible", |focus, props, events, _, cx| {
        let open = props.boolean("open");
        let disabled = props.boolean("disabled");
        let base = AccordionTrigger::new(part(props, "trigger"))
            .when(!disabled, |this| {
                this.track_focus(&focus.clone().tab_stop(true))
            })
            .on_change({
                let events = events.clone();
                move |open, _, _, _| events.emit("openChange", json!({ "open": open }))
            });
        props
            .styled(Collapsible::new(), cx)
            .open(open)
            .child(trigger(
                props,
                base,
                props.string("title").unwrap_or_default(),
                open,
                disabled,
                cx,
            ))
            .content(
                props
                    .styled_by("panelClass", div(), cx)
                    .when_some(props.string("content"), |this, content| this.child(content)),
            )
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Collapsible: a trigger showing `title` above `content`, shown while `open`. \
         The trigger's click, Enter or Space sends `openChange { open }`. `icon` and `openIcon` \
         name SVGs in the host's assets (`icons/chevron-right.svg`) drawn after the title.",
    )
    .prop("title", PropType::String)
    .prop("content", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("icon", PropType::String)
    .prop("openIcon", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .states(TRIGGER_STATES)
    .prop("openTriggerClass", PropType::String)
    .prop("disabledTriggerClass", PropType::String)
    .prop("iconClass", PropType::String)
    .prop("panelClass", PropType::String)
    .event("openChange", &[("open", PropType::Boolean)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{App, ElementId, Role, TestAppContext, Window, px};
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    const LONG: &str = "GPUI Base owns the behavior of every control: pointer and keyboard \
                        interaction, focus, accessibility and state. The app owns the look.";

    fn items() -> Value {
        json!([
            { "title": "Behavior", "content": LONG },
            { "title": "Look", "content": "Class strings." },
            { "title": "Locked", "content": "Never shown.", "disabled": true },
        ])
    }

    fn change(value: &[usize]) -> Vec<(String, Value)> {
        vec![("change".into(), json!({ "value": value }))]
    }

    /// Whether each of `count` accordion triggers shows as expanded.
    fn expanded(fixture: &Fixture, cx: &mut TestAppContext, count: usize) -> Vec<Option<bool>> {
        fixture.update(cx, |window, _| {
            let accordion = window.within(fixture.id.clone());
            (0..count)
                .map(|index| accordion.find(index).expanded())
                .collect()
        })
    }

    fn click_trigger(fixture: &Fixture, cx: &mut TestAppContext, index: usize) {
        fixture.update(cx, |window, cx| {
            window.within(fixture.id.clone()).click(index, cx)
        });
    }

    #[gpui_kit::test]
    fn test_accordion_builder(cx: &mut TestAppContext) {
        let accordion = Fixture::new(
            cx,
            "Accordion",
            json!({
                "items": items(),
                "value": [0],
                "multiple": true,
                "icon": "icons/chevron-down.svg",
                "openIcon": "icons/chevron-up.svg",
                "rootClass": "flex flex_col w_full",
                "itemClass": "border_b_1 border_border",
                "openItemClass": "bg_muted",
                "headerClass": "flex",
                "triggerClass": "flex flex_1 items_center justify_between py_2 font_medium",
                "triggerHoverClass": "bg_accent",
                "triggerActiveClass": "bg_muted",
                "triggerFocusClass": "border_ring",
                "openTriggerClass": "text_foreground",
                "disabledTriggerClass": "opacity_50",
                "iconClass": "size_4 text_muted_foreground",
                "panelClass": "pb_2 text_sm",
            }),
        );
        let size = accordion.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        assert_eq!(
            expanded(&accordion, cx, 3),
            vec![Some(true), Some(false), Some(false)]
        );
        accordion.update(cx, |window, _| {
            let scope = window.within(accordion.id.clone());
            assert_eq!(scope.find(0).role(), Some(Role::Button));
            assert_eq!(
                scope.find(part_of(&accordion, "header-0")).role(),
                Some(Role::Heading)
            );
            assert_eq!(
                scope.find(part_of(&accordion, "panel-0")).role(),
                Some(Role::Region)
            );
            assert!(scope.try_find(part_of(&accordion, "panel-1")).is_none());
        });

        // The open panel's long content wraps in a narrow box.
        let wide = accordion.measure_at(cx, Some(px(800.)));
        let narrow = accordion.measure_at(cx, Some(px(160.)));
        assert!(narrow.height > wide.height);
    }

    /// The id of an accordion's part, as [`super::part`] makes it.
    fn part_of(fixture: &Fixture, name: &'static str) -> ElementId {
        ElementId::NamedChild(std::sync::Arc::new(fixture.id.clone()), name.into())
    }

    #[gpui_kit::test]
    fn test_accordion_change(cx: &mut TestAppContext) {
        let accordion = Fixture::new(cx, "Accordion", json!({ "items": items() }));
        click_trigger(&accordion, cx, 0);
        assert_eq!(accordion.take_events(), change(&[0]));
        assert_eq!(
            expanded(&accordion, cx, 3),
            vec![Some(true), Some(false), Some(false)]
        );

        // One item at most: opening another closes the first.
        click_trigger(&accordion, cx, 1);
        assert_eq!(accordion.take_events(), change(&[1]));
        click_trigger(&accordion, cx, 1);
        assert_eq!(accordion.take_events(), change(&[]));

        accordion.set_prop(cx, "multiple", Some(json!(true)));
        click_trigger(&accordion, cx, 1);
        click_trigger(&accordion, cx, 0);
        let events = accordion.take_events();
        assert_eq!(events[1], change(&[0, 1])[0]);
        assert_eq!(
            expanded(&accordion, cx, 3),
            vec![Some(true), Some(true), Some(false)]
        );
    }

    #[gpui_kit::test]
    fn test_accordion_keyboard(cx: &mut TestAppContext) {
        let accordion = Fixture::new(cx, "Accordion", json!({ "items": items() }));
        let first = accordion.focus_handle(cx).expect("a Tab stop");
        accordion.update(cx, |window, cx| first.focus(window, cx));
        accordion.update(cx, |window: &mut Window, cx: &mut App| {
            assert_eq!(
                window.within(accordion.id.clone()).find(0).focused(),
                Some(true)
            );
            window.press("enter", cx);
        });
        assert_eq!(accordion.take_events(), change(&[0]));
        accordion.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(accordion.take_events(), change(&[]));

        // Tab moves to the next trigger.
        accordion.update(cx, |window, cx| window.press("tab", cx));
        accordion.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(accordion.take_events(), change(&[1]));
    }

    #[gpui_kit::test]
    fn test_accordion_controlled(cx: &mut TestAppContext) {
        let accordion = Fixture::new(cx, "Accordion", json!({ "items": items() }));
        accordion.set_prop(cx, "value", Some(json!([1, 2])));
        assert_eq!(
            expanded(&accordion, cx, 3),
            vec![Some(false), Some(true), Some(true)]
        );
        accordion.set_prop(cx, "value", None);
        assert_eq!(
            expanded(&accordion, cx, 3),
            vec![Some(false), Some(false), Some(false)]
        );
    }

    #[gpui_kit::test]
    fn test_accordion_disabled(cx: &mut TestAppContext) {
        let accordion = Fixture::new(cx, "Accordion", json!({ "items": items() }));
        click_trigger(&accordion, cx, 2);
        assert!(accordion.take_events().is_empty());

        accordion.set_prop(cx, "disabled", Some(json!(true)));
        assert!(accordion.focus_handle(cx).is_none());
        click_trigger(&accordion, cx, 0);
        assert!(accordion.take_events().is_empty());
        accordion.set_prop(cx, "disabled", None);
        click_trigger(&accordion, cx, 0);
        assert_eq!(accordion.take_events(), change(&[0]));
    }

    fn collapsible(cx: &mut TestAppContext, props: Value) -> (Fixture, ElementId) {
        let fixture = Fixture::new(cx, "Collapsible", props);
        let trigger = part_of(&fixture, "trigger");
        (fixture, trigger)
    }

    fn open_change(open: bool) -> Vec<(String, Value)> {
        vec![("openChange".into(), json!({ "open": open }))]
    }

    #[gpui_kit::test]
    fn test_collapsible_builder(cx: &mut TestAppContext) {
        let (collapsible, trigger) = collapsible(
            cx,
            json!({
                "title": "Details",
                "content": LONG,
                "open": true,
                "icon": "icons/chevron-right.svg",
                "openIcon": "icons/chevron-down.svg",
                "rootClass": "flex flex_col gap_1",
                "triggerClass": "flex items_center gap_2 font_medium",
                "triggerHoverClass": "text_primary",
                "triggerActiveClass": "opacity_70",
                "triggerFocusClass": "border_ring",
                "openTriggerClass": "text_foreground",
                "disabledTriggerClass": "opacity_50",
                "iconClass": "size_4",
                "panelClass": "text_sm text_muted_foreground",
            }),
        );
        let size = collapsible.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        collapsible.update(cx, |window, _| {
            let found = window.find(trigger.clone());
            assert_eq!(found.role(), Some(Role::Button));
            assert_eq!(found.expanded(), Some(true));
        });
        let wide = collapsible.measure_at(cx, Some(px(800.)));
        let narrow = collapsible.measure_at(cx, Some(px(160.)));
        assert!(narrow.height > wide.height);

        // Closed, only the trigger shows.
        collapsible.set_prop(cx, "open", None);
        assert!(collapsible.measure_at(cx, Some(px(160.))).height < narrow.height);
    }

    #[gpui_kit::test]
    fn test_collapsible_open_change(cx: &mut TestAppContext) {
        let (collapsible, trigger) =
            collapsible(cx, json!({ "title": "Details", "content": LONG }));
        collapsible.update(cx, |window, cx| window.click(trigger.clone(), cx));
        assert_eq!(collapsible.take_events(), open_change(true));
        collapsible.update(cx, |window, _| {
            assert_eq!(window.find(trigger.clone()).expanded(), Some(true));
        });

        // The click focused the trigger.
        collapsible.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(collapsible.take_events(), open_change(false));
        collapsible.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(collapsible.take_events(), open_change(true));
    }

    #[gpui_kit::test]
    fn test_collapsible_controlled(cx: &mut TestAppContext) {
        let (collapsible, trigger) = collapsible(cx, json!({ "title": "Details" }));
        collapsible.set_prop(cx, "open", Some(json!(true)));
        collapsible.update(cx, |window, _| {
            assert_eq!(window.find(trigger.clone()).expanded(), Some(true));
        });
        collapsible.set_prop(cx, "open", Some(json!(false)));
        collapsible.update(cx, |window, _| {
            assert_eq!(window.find(trigger.clone()).expanded(), Some(false));
        });
    }

    #[gpui_kit::test]
    fn test_collapsible_disabled(cx: &mut TestAppContext) {
        let (collapsible, trigger) =
            collapsible(cx, json!({ "title": "Details", "disabled": true }));
        assert!(collapsible.focus_handle(cx).is_none());
        collapsible.update(cx, |window, cx| window.click(trigger.clone(), cx));
        assert!(collapsible.take_events().is_empty());
        collapsible.set_prop(cx, "disabled", None);
        collapsible.update(cx, |window, cx| window.click(trigger.clone(), cx));
        assert_eq!(collapsible.take_events(), open_change(true));
    }
}
