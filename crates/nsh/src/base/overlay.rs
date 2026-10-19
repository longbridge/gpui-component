//! Modal overlays a trigger opens: Dialog, AlertDialog, Sheet.
//!
//! Each control is its trigger button in the NSH box, plus the modal GPUI
//! Base draws over the host window while `open` is set. Base owns the
//! modal's focus trap, its Escape and backdrop dismissal and the order of
//! its callbacks; the control reports them as events, and JS answers by
//! setting `open`.

use std::sync::Arc;

use gpui_kit::{
    Action, AnyElement, App, ClickEvent, ElementId, FocusHandle, InteractiveElement as _,
    IntoElement, ParentElement as _, Window,
    base::{
        AlertDialog, AlertDialogBackdrop, AlertDialogDescription, AlertDialogPopup,
        AlertDialogTitle, AlertDialogTrigger, Button, Dialog, DialogBackdrop, DialogChangeReason,
        DialogClose, DialogDescription, DialogPopup, DialogTitle, DialogTrigger, Sheet,
        TestSupportExt as _,
        actions::{Cancel, Confirm},
    },
    deferred, div,
    prelude::FluentBuilder as _,
    svg,
};
use serde_json::json;

use crate::{Control, Events, PropType, Props, ROOT_STATES};

pub(super) fn controls() -> Vec<Control> {
    vec![dialog(), alert_dialog(), sheet()]
}

/// The interaction-state class props of the modal's buttons.
const ACTION_STATES: [&str; 3] = ["actionHoverClass", "actionActiveClass", "actionFocusClass"];
const CANCEL_STATES: [&str; 3] = ["cancelHoverClass", "cancelActiveClass", "cancelFocusClass"];
const CLOSE_STATES: [&str; 3] = ["closeHoverClass", "closeActiveClass", "closeFocusClass"];

/// The values of `openChange`'s `reason`, after Base's [`DialogChangeReason`].
const REASONS: &[&str] = &[
    "triggerPress",
    "backdropPress",
    "cancel",
    "confirm",
    "imperative",
];

fn reason(reason: DialogChangeReason) -> &'static str {
    match reason {
        DialogChangeReason::TriggerPress => "triggerPress",
        DialogChangeReason::BackdropPress => "backdropPress",
        DialogChangeReason::Cancel => "cancel",
        DialogChangeReason::Confirm => "confirm",
        DialogChangeReason::Imperative => "imperative",
    }
}

/// The id of a control's part named `name`.
fn part(props: &Props, name: &'static str) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), name.into())
}

/// A boolean prop whose Base default is `true`, when the app set it.
fn flag(props: &Props, name: &str) -> Option<bool> {
    props.has(name).then(|| props.boolean(name))
}

/// The focus handles of a modal control: its trigger's and the modal's.
struct Focus {
    trigger: FocusHandle,
    modal: FocusHandle,
    /// Whether the modal was open when last drawn.
    open: bool,
    /// Whether the modal held the focus when last drawn.
    held: bool,
}

impl Focus {
    fn new(cx: &mut App) -> Self {
        Self {
            trigger: cx.focus_handle(),
            modal: cx.focus_handle(),
            open: false,
            held: false,
        }
    }

    /// Moves the focus into the modal as it opens, where Base's focus trap
    /// and Escape take it, and back to the trigger as it closes, as GPUI
    /// Kit's window does for its dialogs and sheets.
    fn follow(&mut self, open: bool, window: &mut Window, cx: &mut App) {
        let target = match (self.open, open) {
            (false, true) => Some(self.modal.clone()),
            (true, false) if self.held => Some(self.trigger.clone()),
            _ => None,
        };
        if let Some(target) = target {
            window.defer(cx, move |window, cx| target.focus(window, cx));
        }
        self.held = open && self.modal.contains_focused(window, cx);
        self.open = open;
    }
}

/// The trigger button, the control's root; `on_click` takes its
/// activations.
fn trigger(
    props: &Props,
    focus: &FocusHandle,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Button {
    props
        .states(ROOT_STATES, props.styled(Button::new(props.id()), cx), cx)
        .track_focus(focus)
        .disabled(props.boolean("disabled"))
        .styles(|styles| styles.disabled(|style| props.styled_by("disabledClass", style, cx)))
        .when_some(props.string("label"), |this, label| this.child(label))
        .on_click(on_click)
}

/// Reports the trigger's request to open.
fn request_open(events: &Events) {
    events.emit(
        "openChange",
        json!({ "open": true, "reason": reason(DialogChangeReason::TriggerPress) }),
    );
}

/// The trigger of a dialog: Base's trigger part opens it on a press, and the
/// button's Enter and Space open it from the keyboard.
fn dialog_trigger(
    props: &Props,
    events: &Events,
    focus: &FocusHandle,
    part: impl FnOnce(Button, Events) -> AnyElement,
    cx: &App,
) -> AnyElement {
    let keyboard = events.clone();
    let button = trigger(
        props,
        focus,
        move |event, _, _| {
            if let ClickEvent::Keyboard(_) = event {
                request_open(&keyboard);
            }
        },
        cx,
    );
    if props.boolean("disabled") {
        button.into_any_element()
    } else {
        part(button, events.clone())
    }
}

/// Base's `on_ok` / `on_cancel` decision: reports `name` and lets the modal
/// close.
fn decide(
    events: &Events,
    name: &'static str,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static {
    let events = events.clone();
    move |_, _, _| {
        events.emit(name, json!({}));
        true
    }
}

/// Reports Base's requests to open or close as `openChange`.
fn on_open_change(
    events: &Events,
) -> impl Fn(bool, DialogChangeReason, &mut Window, &mut App) + 'static {
    let events = events.clone();
    move |open, change, _, _| {
        events.emit(
            "openChange",
            json!({ "open": open, "reason": reason(change) }),
        );
    }
}

/// The close button, Base's [`DialogClose`], when `closeLabel` or
/// `closeIcon` names it.
fn close_button(props: &Props, cx: &App) -> Option<DialogClose> {
    let label = props.string("closeLabel");
    let icon = props.string("closeIcon");
    (label.is_some() || icon.is_some()).then(|| {
        DialogClose::new().trigger(|button| {
            props
                .states(CLOSE_STATES, props.styled_by("closeClass", button, cx), cx)
                .when_some(label.clone(), |this, label| this.accessibility_label(label))
                .map(|this| match icon {
                    Some(icon) => {
                        this.child(props.styled_by("closeIconClass", svg().path(icon), cx))
                    }
                    None => this.children(label),
                })
        })
    })
}

/// The header row: `title` and the close button, when there are any.
fn header(props: &Props, title: Option<impl IntoElement>, cx: &App) -> Option<AnyElement> {
    let close = close_button(props, cx);
    (title.is_some() || close.is_some()).then(|| {
        props
            .styled_by("headerClass", div(), cx)
            .children(title)
            .children(close)
            .into_any_element()
    })
}

/// A footer button sending Base's `action` to the modal. It dispatches from
/// the modal's own focus node, which reaches the modal whatever holds the
/// focus.
fn footer_button(
    props: &Props,
    modal: &FocusHandle,
    (name, class, states): (&'static str, &str, [&str; 3]),
    label: impl IntoElement,
    action: impl Action,
    cx: &App,
) -> Button {
    let modal = modal.clone();
    props
        .states(
            states,
            props.styled_by(class, Button::new(part(props, name)), cx),
            cx,
        )
        .child(label)
        .on_click(move |_, window, cx| modal.dispatch_action(&action, window, cx))
}

/// The footer row: the cancel and action buttons the props name.
fn footer(props: &Props, modal: &FocusHandle, cx: &App) -> Option<AnyElement> {
    let cancel = props.string("cancelLabel").map(|label| {
        footer_button(
            props,
            modal,
            ("cancel", "cancelClass", CANCEL_STATES),
            label,
            Cancel,
            cx,
        )
    });
    let action = props.string("actionLabel").map(|label| {
        footer_button(
            props,
            modal,
            ("action", "actionClass", ACTION_STATES),
            label,
            Confirm { secondary: false },
            cx,
        )
    });
    (cancel.is_some() || action.is_some()).then(|| {
        props
            .styled_by("footerClass", div(), cx)
            .children(cancel)
            .children(action)
            .into_any_element()
    })
}

fn dialog() -> Control {
    Control::stateful(
        "Dialog",
        |_, _, _, cx| Focus::new(cx),
        |focus, props, events, window, cx| {
            let open = props.boolean("open");
            focus.follow(open, window, cx);
            div()
                .child(dialog_trigger(
                    props,
                    events,
                    &focus.trigger,
                    |button, events| {
                        DialogTrigger::new(button)
                            .on_open(move |_, _| request_open(&events))
                            .into_any_element()
                    },
                    cx,
                ))
                .child(
                    Dialog::new(cx)
                        .open(open)
                        .focus_handle(focus.modal.clone())
                        .when_some(flag(props, "closeOnEscape"), |this, value| {
                            this.close_on_escape(value)
                        })
                        .when_some(flag(props, "closeOnBackdropPress"), |this, value| {
                            this.close_on_backdrop_press(value)
                        })
                        .on_ok(decide(events, "action"))
                        .on_cancel(decide(events, "cancel"))
                        .on_open_change(on_open_change(events))
                        .backdrop(props.styled_by("backdropClass", DialogBackdrop::new(), cx))
                        .popup(
                            props
                                .styled_by("popupClass", DialogPopup::new(), cx)
                                .children(header(
                                    props,
                                    props.string("title").map(|title| {
                                        props
                                            .styled_by("titleClass", DialogTitle::new(), cx)
                                            .child(title)
                                    }),
                                    cx,
                                ))
                                .when_some(props.string("description"), |this, description| {
                                    this.child(
                                        props
                                            .styled_by(
                                                "descriptionClass",
                                                DialogDescription::new(),
                                                cx,
                                            )
                                            .child(description),
                                    )
                                })
                                .children(footer(props, &focus.modal, cx)),
                        ),
                )
                .into_any_element()
        },
    )
    .focus(|focus: &Focus, _| focus.trigger.clone())
    .documentation(
        "GPUI Base's Dialog: a trigger button that opens a modal over the window. Base traps \
         the focus inside it; Escape, the cancel and close buttons and a press on the backdrop \
         cancel it, Enter and the action button confirm it. Each reports `cancel` or `action`, \
         then `openChange` with the reason.",
    )
    .prop("label", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("title", PropType::String)
    .prop("description", PropType::String)
    .prop("actionLabel", PropType::String)
    .prop("cancelLabel", PropType::String)
    .prop("closeLabel", PropType::String)
    .prop("closeIcon", PropType::String)
    .prop("closeOnEscape", PropType::Boolean)
    .prop("closeOnBackdropPress", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .states(ROOT_STATES)
    .prop("disabledClass", PropType::String)
    .prop("backdropClass", PropType::String)
    .prop("popupClass", PropType::String)
    .prop("headerClass", PropType::String)
    .prop("titleClass", PropType::String)
    .prop("descriptionClass", PropType::String)
    .prop("footerClass", PropType::String)
    .prop("actionClass", PropType::String)
    .states(ACTION_STATES)
    .prop("cancelClass", PropType::String)
    .states(CANCEL_STATES)
    .prop("closeClass", PropType::String)
    .states(CLOSE_STATES)
    .prop("closeIconClass", PropType::String)
    .event(
        "openChange",
        &[
            ("open", PropType::Boolean),
            ("reason", PropType::Enum(REASONS)),
        ],
    )
    .event("action", &[])
    .event("cancel", &[])
}

fn alert_dialog() -> Control {
    Control::stateful(
        "AlertDialog",
        |_, _, _, cx| Focus::new(cx),
        |focus, props, events, window, cx| {
            let open = props.boolean("open");
            focus.follow(open, window, cx);
            div()
                .child(dialog_trigger(
                    props,
                    events,
                    &focus.trigger,
                    |button, events| {
                        AlertDialogTrigger::new(button)
                            .on_open(move |_, _| request_open(&events))
                            .into_any_element()
                    },
                    cx,
                ))
                .child(
                    AlertDialog::new(cx)
                        .open(open)
                        .focus_handle(focus.modal.clone())
                        .when_some(flag(props, "closeOnEscape"), |this, value| {
                            this.close_on_escape(value)
                        })
                        .on_ok(decide(events, "action"))
                        .on_cancel(decide(events, "cancel"))
                        .on_open_change(on_open_change(events))
                        .backdrop(props.styled_by("backdropClass", AlertDialogBackdrop::new(), cx))
                        .popup(
                            props
                                .styled_by("popupClass", AlertDialogPopup::new(), cx)
                                .when_some(props.string("title"), |this, title| {
                                    this.child(
                                        props
                                            .styled_by("titleClass", AlertDialogTitle::new(), cx)
                                            .child(title),
                                    )
                                })
                                .when_some(props.string("description"), |this, description| {
                                    this.child(
                                        props
                                            .styled_by(
                                                "descriptionClass",
                                                AlertDialogDescription::new(),
                                                cx,
                                            )
                                            .child(description),
                                    )
                                })
                                .children(footer(props, &focus.modal, cx)),
                        ),
                )
                .into_any_element()
        },
    )
    .focus(|focus: &Focus, _| focus.trigger.clone())
    .documentation(
        "GPUI Base's AlertDialog: a trigger button that opens a modal asking to confirm. Base \
         traps the focus inside it and keeps it open on a press on the backdrop; Escape and the \
         cancel button cancel it, Enter and the action button confirm it. Each reports `cancel` \
         or `action`, then `openChange` with the reason.",
    )
    .prop("label", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("title", PropType::String)
    .prop("description", PropType::String)
    .prop("actionLabel", PropType::String)
    .prop("cancelLabel", PropType::String)
    .prop("closeOnEscape", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .states(ROOT_STATES)
    .prop("disabledClass", PropType::String)
    .prop("backdropClass", PropType::String)
    .prop("popupClass", PropType::String)
    .prop("titleClass", PropType::String)
    .prop("descriptionClass", PropType::String)
    .prop("footerClass", PropType::String)
    .prop("actionClass", PropType::String)
    .states(ACTION_STATES)
    .prop("cancelClass", PropType::String)
    .states(CANCEL_STATES)
    .event(
        "openChange",
        &[
            ("open", PropType::Boolean),
            ("reason", PropType::Enum(REASONS)),
        ],
    )
    .event("action", &[])
    .event("cancel", &[])
}

/// Base's Sheet is a host with no placement of its own: `popupClass` pins
/// the surface to an edge of the window (`absolute top_0 right_0 h_full
/// w_80`), and Base draws it over the window's other content.
fn sheet() -> Control {
    Control::stateful(
        "Sheet",
        |_, _, _, cx| Focus::new(cx),
        |focus, props, events, window, cx| {
            let open = props.boolean("open");
            focus.follow(open, window, cx);
            let opening = events.clone();
            let closing = events.clone();
            div()
                .child(trigger(
                    props,
                    &focus.trigger,
                    move |_, _, _| opening.emit("openChange", json!({ "open": true })),
                    cx,
                ))
                .when(open, |this| {
                    this.child(
                        deferred(
                            Sheet::new(cx)
                                .focus_handle(focus.modal.clone())
                                .when_some(flag(props, "closeOnBackdropPress"), |this, value| {
                                    this.overlay_closable(value)
                                })
                                .on_close(move |_, _, _| {
                                    closing.emit("openChange", json!({ "open": false }))
                                })
                                .overlay(props.styled_by("backdropClass", div(), cx))
                                .surface(
                                    props
                                        .styled_by(
                                            "popupClass",
                                            // Presses on the surface stay off
                                            // the backdrop behind it.
                                            div().id(part(props, "popup")).test_support().occlude(),
                                            cx,
                                        )
                                        .children(header(
                                            props,
                                            props.string("title").map(|title| {
                                                props
                                                    .styled_by("titleClass", div(), cx)
                                                    .child(title)
                                            }),
                                            cx,
                                        ))
                                        .when_some(
                                            props.string("description"),
                                            |this, description| {
                                                this.child(
                                                    props
                                                        .styled_by("descriptionClass", div(), cx)
                                                        .child(description),
                                                )
                                            },
                                        ),
                                ),
                        )
                        .with_priority(10),
                    )
                })
                .into_any_element()
        },
    )
    .focus(|focus: &Focus, _| focus.trigger.clone())
    .documentation(
        "GPUI Base's Sheet: a trigger button that opens a modal surface at an edge of the \
         window, which `popupClass` places. Base traps the focus inside it; Escape, the close \
         button and a press on the backdrop close it and report `openChange`.",
    )
    .prop("label", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("title", PropType::String)
    .prop("description", PropType::String)
    .prop("closeLabel", PropType::String)
    .prop("closeIcon", PropType::String)
    .prop("closeOnBackdropPress", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .states(ROOT_STATES)
    .prop("disabledClass", PropType::String)
    .prop("backdropClass", PropType::String)
    .prop("popupClass", PropType::String)
    .prop("headerClass", PropType::String)
    .prop("titleClass", PropType::String)
    .prop("descriptionClass", PropType::String)
    .prop("closeClass", PropType::String)
    .states(CLOSE_STATES)
    .prop("closeIconClass", PropType::String)
    .event("openChange", &[("open", PropType::Boolean)])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui_kit::{ElementId, Role, TestAppContext, point, px};
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    /// The id of the fixture control's part `name`.
    fn part(fixture: &Fixture, name: &str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), name.to_owned().into())
    }

    fn event(name: &str, data: Value) -> (String, Value) {
        (name.into(), data)
    }

    fn opened() -> (String, Value) {
        event(
            "openChange",
            json!({ "open": true, "reason": "triggerPress" }),
        )
    }

    /// The window-wide host Base draws a dialog or an alert dialog in, as
    /// its focus trap names it.
    fn dialog_host() -> ElementId {
        "dialog-0".into()
    }

    fn sheet_host() -> ElementId {
        "sheet".into()
    }

    /// The window's close button, Base's `DialogClose` trigger.
    fn close() -> ElementId {
        "close".into()
    }

    /// Presses the backdrop at the window's top-left corner, clear of the
    /// popup.
    fn press_backdrop(fixture: &Fixture, host: ElementId, cx: &mut TestAppContext) {
        fixture.update(cx, |window, cx| {
            window.click_at(host, point(px(4.), px(4.)), cx)
        });
    }

    fn dialog_props() -> Value {
        json!({
            "label": "Edit profile",
            "title": "Edit profile",
            "description": "Make changes to your profile.",
            "actionLabel": "Save",
            "cancelLabel": "Cancel",
            "closeLabel": "Close",
        })
    }

    #[gpui_kit::test]
    fn test_dialog_builder(cx: &mut TestAppContext) {
        let dialog = Fixture::new(
            cx,
            "Dialog",
            json!({
                "label": "Edit profile",
                "open": true,
                "title": "Edit profile",
                "description": "Make changes to your profile.",
                "actionLabel": "Save",
                "cancelLabel": "Cancel",
                "closeLabel": "Close",
                "closeIcon": "icons/close.svg",
                "closeOnEscape": true,
                "closeOnBackdropPress": true,
                "rootClass": "h_8 px_3 rounded_md border_1 border_border",
                "hoverClass": "bg_accent",
                "disabledClass": "opacity_50",
                "backdropClass": "size_full bg_background opacity_80",
                "popupClass": "w_96 p_6 gap_4 flex flex_col rounded_lg border_1 bg_background",
                "headerClass": "flex justify_between",
                "titleClass": "font_semibold",
                "descriptionClass": "text_sm text_muted_foreground",
                "footerClass": "flex justify_end gap_2",
                "actionClass": "h_8 px_3 bg_primary",
                "actionHoverClass": "opacity_90",
                "cancelClass": "h_8 px_3 border_1",
                "cancelHoverClass": "bg_accent",
                "closeClass": "size_6 rounded_md",
                "closeHoverClass": "bg_accent",
                "closeIconClass": "size_4",
            }),
        );
        let size = dialog.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        dialog.update(cx, |window, _| {
            assert_eq!(window.find(dialog_host()).role(), Some(Role::Dialog));
            assert_eq!(window.find(close()).label(), Some("Close"));
            assert!(window.try_find(part(&dialog, "action")).is_some());
            assert!(window.try_find(part(&dialog, "cancel")).is_some());
        });
    }

    #[gpui_kit::test]
    fn test_dialog_click_and_escape(cx: &mut TestAppContext) {
        let dialog = Fixture::new(cx, "Dialog", dialog_props());
        dialog.update(cx, |window, cx| window.click(dialog.id.clone(), cx));
        assert_eq!(dialog.take_events(), vec![opened()]);
        // Opening moved the focus into the modal, where Escape reaches Base.
        dialog.update(cx, |window, cx| {
            assert_eq!(window.find(dialog_host()).focused(), Some(true));
            window.press("escape", cx);
        });
        assert_eq!(
            dialog.take_events(),
            vec![
                event("cancel", json!({})),
                event("openChange", json!({ "open": false, "reason": "cancel" })),
            ]
        );
        dialog.update(cx, |window, _| {
            assert!(window.try_find(dialog_host()).is_none());
            // Closing gave the focus back to the trigger.
            assert_eq!(window.find(dialog.id.clone()).focused(), Some(true));
        });
    }

    #[gpui_kit::test]
    fn test_dialog_buttons(cx: &mut TestAppContext) {
        let dialog = Fixture::new(cx, "Dialog", dialog_props());
        let cancelled = vec![
            event("cancel", json!({})),
            event("openChange", json!({ "open": false, "reason": "cancel" })),
        ];
        dialog.set_prop(cx, "open", Some(json!(true)));
        dialog.update(cx, |window, cx| window.click(part(&dialog, "action"), cx));
        assert_eq!(
            dialog.take_events(),
            vec![
                event("action", json!({})),
                event("openChange", json!({ "open": false, "reason": "confirm" })),
            ]
        );
        dialog.set_prop(cx, "open", Some(json!(true)));
        dialog.update(cx, |window, cx| window.click(part(&dialog, "cancel"), cx));
        assert_eq!(dialog.take_events(), cancelled);
        dialog.set_prop(cx, "open", Some(json!(true)));
        dialog.update(cx, |window, cx| window.click(close(), cx));
        assert_eq!(dialog.take_events(), cancelled);
    }

    #[gpui_kit::test]
    fn test_dialog_keyboard(cx: &mut TestAppContext) {
        let dialog = Fixture::new(cx, "Dialog", dialog_props());
        let trigger = dialog.focus_handle(cx).unwrap();
        dialog.update(cx, |window, cx| trigger.focus(window, cx));
        dialog.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(dialog.take_events(), vec![opened()]);
        // Base's focus trap keeps Tab among the modal's buttons.
        let buttons = [close(), part(&dialog, "cancel"), part(&dialog, "action")];
        for _ in 0..4 {
            dialog.update(cx, |window, cx| window.press("tab", cx));
            dialog.update(cx, |window, _| {
                assert!(
                    buttons
                        .iter()
                        .any(|id| window.find(id.clone()).focused() == Some(true))
                );
            });
        }
        let action = part(&dialog, "action");
        while !dialog.update(cx, |window, _| {
            window.find(action.clone()).focused() == Some(true)
        }) {
            dialog.update(cx, |window, cx| window.press("tab", cx));
        }
        dialog.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            dialog.take_events(),
            vec![
                event("action", json!({})),
                event("openChange", json!({ "open": false, "reason": "confirm" })),
            ]
        );
    }

    #[gpui_kit::test]
    fn test_dialog_backdrop(cx: &mut TestAppContext) {
        let dialog = Fixture::new(cx, "Dialog", dialog_props());
        dialog.set_prop(cx, "open", Some(json!(true)));
        press_backdrop(&dialog, dialog_host(), cx);
        assert_eq!(
            dialog.take_events(),
            vec![
                event("cancel", json!({})),
                event(
                    "openChange",
                    json!({ "open": false, "reason": "backdropPress" })
                ),
            ]
        );

        dialog.set_prop(cx, "closeOnBackdropPress", Some(json!(false)));
        dialog.set_prop(cx, "closeOnEscape", Some(json!(false)));
        dialog.set_prop(cx, "open", Some(json!(true)));
        press_backdrop(&dialog, dialog_host(), cx);
        dialog.update(cx, |window, cx| window.press("escape", cx));
        assert!(dialog.take_events().is_empty());
        dialog.update(cx, |window, _| {
            assert!(window.try_find(dialog_host()).is_some())
        });
    }

    #[gpui_kit::test]
    fn test_dialog_controlled(cx: &mut TestAppContext) {
        let dialog = Fixture::new(cx, "Dialog", dialog_props());
        dialog.update(cx, |window, _| {
            assert!(window.try_find(dialog_host()).is_none())
        });
        dialog.set_prop(cx, "open", Some(json!(true)));
        dialog.update(cx, |window, _| {
            assert!(window.try_find(dialog_host()).is_some())
        });
        dialog.set_prop(cx, "open", Some(json!(false)));
        dialog.update(cx, |window, _| {
            assert!(window.try_find(dialog_host()).is_none())
        });
        assert!(dialog.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_dialog_disabled(cx: &mut TestAppContext) {
        let mut props = dialog_props();
        props["disabled"] = json!(true);
        let dialog = Fixture::new(cx, "Dialog", props);
        assert!(dialog.focus_handle(cx).is_none());
        dialog.update(cx, |window, cx| {
            assert_eq!(window.find(dialog.id.clone()).disabled(), Some(true));
            window.click(dialog.id.clone(), cx);
            window.press("enter", cx);
        });
        assert!(dialog.take_events().is_empty());
    }

    fn alert_props() -> Value {
        json!({
            "label": "Delete",
            "title": "Delete the file?",
            "description": "This cannot be undone.",
            "actionLabel": "Delete",
            "cancelLabel": "Cancel",
        })
    }

    #[gpui_kit::test]
    fn test_alert_dialog_builder(cx: &mut TestAppContext) {
        let alert = Fixture::new(
            cx,
            "AlertDialog",
            json!({
                "label": "Delete",
                "open": true,
                "title": "Delete the file?",
                "description": "This cannot be undone.",
                "actionLabel": "Delete",
                "cancelLabel": "Cancel",
                "closeOnEscape": true,
                "rootClass": "h_8 px_3 bg_destructive text_destructive_foreground",
                "hoverClass": "opacity_90",
                "disabledClass": "opacity_50",
                "backdropClass": "size_full bg_background opacity_80",
                "popupClass": "w_96 p_6 gap_4 flex flex_col bg_background",
                "titleClass": "font_semibold",
                "descriptionClass": "text_sm",
                "footerClass": "flex justify_end gap_2",
                "actionClass": "h_8 px_3 bg_destructive",
                "actionHoverClass": "opacity_90",
                "cancelClass": "h_8 px_3 border_1",
                "cancelHoverClass": "bg_accent",
            }),
        );
        let size = alert.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        alert.update(cx, |window, _| {
            assert_eq!(window.find(dialog_host()).role(), Some(Role::AlertDialog));
            assert!(window.try_find(part(&alert, "action")).is_some());
            assert!(window.try_find(part(&alert, "cancel")).is_some());
        });
    }

    #[gpui_kit::test]
    fn test_alert_dialog_interaction(cx: &mut TestAppContext) {
        let alert = Fixture::new(cx, "AlertDialog", alert_props());
        alert.update(cx, |window, cx| window.click(alert.id.clone(), cx));
        assert_eq!(alert.take_events(), vec![opened()]);
        // A press on the backdrop leaves an alert open.
        press_backdrop(&alert, dialog_host(), cx);
        assert!(alert.take_events().is_empty());
        alert.update(cx, |window, cx| window.press("escape", cx));
        assert_eq!(
            alert.take_events(),
            vec![
                event("cancel", json!({})),
                event("openChange", json!({ "open": false, "reason": "cancel" })),
            ]
        );
        alert.set_prop(cx, "open", Some(json!(true)));
        alert.update(cx, |window, cx| window.click(part(&alert, "action"), cx));
        assert_eq!(
            alert.take_events(),
            vec![
                event("action", json!({})),
                event("openChange", json!({ "open": false, "reason": "confirm" })),
            ]
        );
        alert.update(cx, |window, _| {
            assert!(window.try_find(dialog_host()).is_none())
        });
    }

    #[gpui_kit::test]
    fn test_alert_dialog_keyboard(cx: &mut TestAppContext) {
        let alert = Fixture::new(cx, "AlertDialog", alert_props());
        let trigger = alert.focus_handle(cx).unwrap();
        alert.update(cx, |window, cx| trigger.focus(window, cx));
        alert.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(alert.take_events(), vec![opened()]);
        // Enter on the modal confirms.
        alert.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            alert.take_events(),
            vec![
                event("action", json!({})),
                event("openChange", json!({ "open": false, "reason": "confirm" })),
            ]
        );
        alert.update(cx, |window, _| assert!(trigger.is_focused(window)));
    }

    #[gpui_kit::test]
    fn test_alert_dialog_controlled_and_disabled(cx: &mut TestAppContext) {
        let alert = Fixture::new(cx, "AlertDialog", alert_props());
        alert.set_prop(cx, "open", Some(json!(true)));
        alert.update(cx, |window, _| {
            assert!(window.try_find(dialog_host()).is_some())
        });
        alert.set_prop(cx, "open", Some(json!(false)));
        alert.set_prop(cx, "disabled", Some(json!(true)));
        alert.update(cx, |window, cx| {
            assert!(window.try_find(dialog_host()).is_none());
            window.click(alert.id.clone(), cx);
        });
        assert!(alert.take_events().is_empty());
    }

    fn sheet_props() -> Value {
        json!({
            "label": "Open settings",
            "title": "Settings",
            "description": "Change how the app looks.",
            "closeLabel": "Close",
            "popupClass": "absolute top_0 right_0 h_full w_80 p_4",
        })
    }

    #[gpui_kit::test]
    fn test_sheet_builder(cx: &mut TestAppContext) {
        let sheet = Fixture::new(
            cx,
            "Sheet",
            json!({
                "label": "Open settings",
                "open": true,
                "title": "Settings",
                "description": "Change how the app looks.",
                "closeLabel": "Close",
                "closeIcon": "icons/close.svg",
                "closeOnBackdropPress": true,
                "rootClass": "h_8 px_3 rounded_md border_1",
                "hoverClass": "bg_accent",
                "disabledClass": "opacity_50",
                "backdropClass": "absolute inset_0 bg_background opacity_80",
                "popupClass": "absolute top_0 right_0 h_full w_80 p_4 bg_background border_l_1",
                "headerClass": "flex justify_between",
                "titleClass": "font_semibold",
                "descriptionClass": "text_sm",
                "closeClass": "size_6 rounded_md",
                "closeHoverClass": "bg_accent",
                "closeIconClass": "size_4",
            }),
        );
        let size = sheet.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        sheet.update(cx, |window, _| {
            assert!(window.try_find(sheet_host()).is_some());
            let popup = window.find(part(&sheet, "popup"));
            assert_eq!(popup.bounds().size.width, px(320.));
            assert_eq!(window.find(close()).label(), Some("Close"));
        });
    }

    #[gpui_kit::test]
    fn test_sheet_interaction(cx: &mut TestAppContext) {
        let closed = vec![event("openChange", json!({ "open": false }))];
        let sheet = Fixture::new(cx, "Sheet", sheet_props());
        sheet.update(cx, |window, cx| window.click(sheet.id.clone(), cx));
        assert_eq!(
            sheet.take_events(),
            vec![event("openChange", json!({ "open": true }))]
        );
        sheet.update(cx, |window, cx| {
            assert_eq!(window.find(sheet_host()).focused(), Some(true));
            window.press("escape", cx);
        });
        assert_eq!(sheet.take_events(), closed);
        sheet.update(cx, |window, _| {
            assert!(window.try_find(sheet_host()).is_none());
            assert_eq!(window.find(sheet.id.clone()).focused(), Some(true));
        });

        sheet.set_prop(cx, "open", Some(json!(true)));
        // The surface keeps presses on itself.
        sheet.update(cx, |window, cx| window.click(part(&sheet, "popup"), cx));
        assert!(sheet.take_events().is_empty());
        sheet.update(cx, |window, cx| window.click(close(), cx));
        assert_eq!(sheet.take_events(), closed);
        sheet.set_prop(cx, "open", Some(json!(true)));
        press_backdrop(&sheet, sheet_host(), cx);
        assert_eq!(sheet.take_events(), closed);

        sheet.set_prop(cx, "closeOnBackdropPress", Some(json!(false)));
        sheet.set_prop(cx, "open", Some(json!(true)));
        press_backdrop(&sheet, sheet_host(), cx);
        assert!(sheet.take_events().is_empty());
        sheet.update(cx, |window, _| {
            assert!(window.try_find(sheet_host()).is_some())
        });
    }

    #[gpui_kit::test]
    fn test_sheet_keyboard(cx: &mut TestAppContext) {
        let sheet = Fixture::new(cx, "Sheet", sheet_props());
        let trigger = sheet.focus_handle(cx).unwrap();
        sheet.update(cx, |window, cx| trigger.focus(window, cx));
        sheet.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            sheet.take_events(),
            vec![event("openChange", json!({ "open": true }))]
        );
        // Base's focus trap keeps Tab on the close button.
        sheet.update(cx, |window, cx| window.press("tab", cx));
        sheet.update(cx, |window, cx| window.press("tab", cx));
        sheet.update(cx, |window, cx| {
            assert_eq!(window.find(close()).focused(), Some(true));
            window.press("space", cx);
        });
        assert_eq!(
            sheet.take_events(),
            vec![event("openChange", json!({ "open": false }))]
        );
        sheet.update(cx, |window, _| assert!(trigger.is_focused(window)));
    }

    #[gpui_kit::test]
    fn test_sheet_controlled_and_disabled(cx: &mut TestAppContext) {
        let sheet = Fixture::new(cx, "Sheet", sheet_props());
        sheet.set_prop(cx, "open", Some(json!(true)));
        sheet.update(cx, |window, _| {
            assert!(window.try_find(sheet_host()).is_some())
        });
        sheet.set_prop(cx, "open", Some(json!(false)));
        sheet.set_prop(cx, "disabled", Some(json!(true)));
        assert!(sheet.focus_handle(cx).is_none());
        sheet.update(cx, |window, cx| {
            assert!(window.try_find(sheet_host()).is_none());
            window.click(sheet.id.clone(), cx);
        });
        assert!(sheet.take_events().is_empty());
    }
}
