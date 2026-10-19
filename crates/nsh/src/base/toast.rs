//! Toasts: ToastStack.
//!
//! The app keeps its toasts as data (`toasts`), the way it keeps any other
//! list; the control shows them the way GPUI Base expects a host to, as GPUI
//! Kit's notification list and GPUI Shell's root do: a [`ToastManager`]
//! orders them and owns their lifecycle (entry, auto-hide, pause, exit), a
//! [`ToastStack`] lays them out, measures and springs them, and expands on
//! hover or focus (which pauses the timers), and each is a semantic [`Toast`]
//! carrying its transition phase. The control only feeds the manager what JS
//! adds or removes, ticks its clock while a toast has time to track, and
//! reports a dismissal.

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
    time::Duration,
};

use gpui_kit::{
    Anchor, AnyElement, App, ElementId, FocusHandle, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Pixels, Point, SharedString, Size, Window, anchored,
    base::{
        Button, POPUP_PRIORITY, Toast, ToastManager, ToastMotion, ToastOptions, ToastStack,
        ToastStackState, ToastTransitionStatus,
    },
    deferred, div, point,
    prelude::FluentBuilder as _,
    px, svg,
};
use serde::Deserialize;
use serde_json::json;

use crate::{Control, Events, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![toast_stack()]
}

/// How often the lifecycle clock is advanced while a toast has time to
/// track: timeouts are wall-clock, so they are sampled (as GPUI Kit's
/// notification list samples them).
const TICK: Duration = Duration::from_millis(50);

/// Above popups, as in GPUI Shell: a toast reports the outcome of what the
/// user just did, which matters most while a popup is open.
const PRIORITY: usize = POPUP_PRIORITY + 1;

/// The visible toasts when `limit` is unset.
const DEFAULT_LIMIT: usize = 3;

/// The values of the `placement` prop; the first is the default.
const PLACEMENTS: &[&str] = &[
    "topRight",
    "topLeft",
    "topCenter",
    "bottomRight",
    "bottomLeft",
    "bottomCenter",
];

const TOAST_STATES: [&str; 3] = ["toastHoverClass", "", ""];
const CLOSE_STATES: [&str; 3] = ["closeHoverClass", "closeActiveClass", "closeFocusClass"];

/// One entry of the `toasts` prop.
#[derive(Clone, Deserialize)]
struct Entry {
    id: SharedString,
    title: SharedString,
    #[serde(default)]
    description: Option<SharedString>,
    /// Milliseconds before it hides itself; unset or `0` keeps it until it
    /// is dismissed (Base's default).
    #[serde(default)]
    duration: Option<f64>,
}

impl Entry {
    fn timeout(&self) -> Option<Duration> {
        self.duration
            .filter(|ms| *ms > 0.0)
            .map(|ms| Duration::from_secs_f64(ms / 1000.0))
    }
}

/// The toasts Base's manager holds, shared with the clock.
struct Toasts {
    manager: ToastManager<SharedString, Entry>,
    /// The ids of the `toasts` prop as JS last set it.
    shown: Vec<SharedString>,
    /// Whether the clock is running.
    ticking: bool,
}

impl Toasts {
    /// Shows the entries JS added and lets the ones it removed leave.
    fn sync(&mut self, entries: &[Entry], now: std::time::Instant) {
        for entry in entries.iter().filter(|e| !self.shown.contains(&e.id)) {
            let options = ToastOptions {
                timeout: entry.timeout(),
            };
            self.manager
                .push(entry.id.clone(), entry.clone(), options, now);
        }
        for id in &self.shown {
            if !entries.iter().any(|entry| &entry.id == id) {
                self.manager.dismiss(id, now);
            }
        }
        self.shown = entries.iter().map(|entry| entry.id.clone()).collect();
    }

    /// Whether a toast is in transition or counting down.
    fn needs_clock(&self) -> bool {
        self.manager.iter().any(|(_, entry, status)| match status {
            ToastTransitionStatus::Present => entry.timeout().is_some(),
            ToastTransitionStatus::Starting | ToastTransitionStatus::Ending => true,
        })
    }
}

struct State {
    toasts: Rc<RefCell<Toasts>>,
    layout: ToastStackState,
    /// The stack's focus scope: focus inside it expands the stack and
    /// pauses the timers.
    focus: FocusHandle,
}

/// The id of a control's part named `name`.
fn part(props: &Props, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), name.into())
}

/// The anchor of the `placement` prop, and its point in a viewport of `size`.
fn placement(props: &Props, size: Size<Pixels>) -> (Anchor, Point<Pixels>) {
    let (width, height) = (size.width, size.height);
    match props.string("placement").as_deref() {
        Some("topLeft") => (Anchor::TopLeft, point(px(0.), px(0.))),
        Some("topCenter") => (Anchor::TopCenter, point(width / 2., px(0.))),
        Some("bottomRight") => (Anchor::BottomRight, point(width, height)),
        Some("bottomLeft") => (Anchor::BottomLeft, point(px(0.), height)),
        Some("bottomCenter") => (Anchor::BottomCenter, point(width / 2., height)),
        _ => (Anchor::TopRight, point(width, px(0.))),
    }
}

/// Runs the lifecycle clock until no toast has time to track, reporting the
/// toasts whose time ran out.
fn start_clock(
    toasts: &Rc<RefCell<Toasts>>,
    layout: &ToastStackState,
    events: &Events,
    window: &mut Window,
    cx: &mut App,
) {
    {
        let mut toasts = toasts.borrow_mut();
        if toasts.ticking || !toasts.needs_clock() {
            return;
        }
        toasts.ticking = true;
    }
    let weak: Weak<RefCell<Toasts>> = Rc::downgrade(toasts);
    let layout = layout.clone();
    let events = events.clone();
    window
        .spawn(cx, async move |cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                // Dropped with the control.
                let Some(toasts) = weak.upgrade() else {
                    break;
                };
                let running = cx.update(|_, cx| {
                    // Paused while the user reads the stack (hover or focus).
                    let advance = toasts
                        .borrow_mut()
                        .manager
                        .advance(cx.background_executor().now(), layout.is_expanded());
                    for id in &advance.ending {
                        events.emit("dismiss", json!({ "id": id }));
                    }
                    if advance.changed {
                        events.redraw();
                    }
                    let mut toasts = toasts.borrow_mut();
                    toasts.ticking = toasts.needs_clock();
                    toasts.ticking
                });
                if !matches!(running, Ok(true)) {
                    toasts.borrow_mut().ticking = false;
                    break;
                }
            }
        })
        .detach();
}

/// One toast: its title, description and close button.
fn toast(
    state: &State,
    props: &Props,
    events: &Events,
    entry: Entry,
    status: ToastTransitionStatus,
    cx: &App,
) -> AnyElement {
    let id = entry.id.clone();
    let phase_class = match status {
        ToastTransitionStatus::Starting => Some("toastStartingClass"),
        ToastTransitionStatus::Present => None,
        ToastTransitionStatus::Ending => Some("toastEndingClass"),
    };
    props
        .states(
            TOAST_STATES,
            props.styled_by(
                "toastClass",
                Toast::new(part(props, format!("toast-{id}"))),
                cx,
            ),
            cx,
        )
        .transition_status(status)
        .when_some(phase_class, |this, class| props.styled_by(class, this, cx))
        // The app below takes no clicks through a toast.
        .occlude()
        .child(props.styled_by("titleClass", div().child(entry.title), cx))
        .when_some(entry.description, |this, description| {
            this.child(props.styled_by("descriptionClass", div().child(description), cx))
        })
        .when_some(props.string("closeIcon"), |this, icon| {
            let toasts = state.toasts.clone();
            let layout = state.layout.clone();
            let events = events.clone();
            this.child(
                props
                    .states(
                        CLOSE_STATES,
                        props.styled_by(
                            "closeClass",
                            Button::new(part(props, format!("close-{id}"))),
                            cx,
                        ),
                        cx,
                    )
                    .accessibility_label(
                        props.string("closeLabel").unwrap_or_else(|| "Close".into()),
                    )
                    .child(props.styled_by("closeIconClass", svg().path(icon), cx))
                    .on_click(move |_, window, cx| {
                        let now = cx.background_executor().now();
                        if toasts.borrow_mut().manager.dismiss(&id, now) {
                            events.emit("dismiss", json!({ "id": id }));
                            start_clock(&toasts, &layout, &events, window, cx);
                        }
                    }),
            )
        })
        .into_any_element()
}

fn toast_stack() -> Control {
    Control::stateful(
        "ToastStack",
        |_, _, _, cx| State {
            toasts: Rc::new(RefCell::new(Toasts {
                manager: ToastManager::new(ToastMotion::sonner()),
                shown: Vec::new(),
                ticking: false,
            })),
            layout: ToastStackState::default(),
            focus: cx.focus_handle().tab_stop(true),
        },
        |state, props, events, window, cx| {
            let entries: Vec<Entry> = props.json("toasts").unwrap_or_default();
            if props.changed("toasts") {
                let now = cx.background_executor().now();
                state.toasts.borrow_mut().sync(&entries, now);
            }
            start_clock(&state.toasts, &state.layout, events, window, cx);

            let limit = match props.number("limit") {
                limit if limit >= 1.0 => limit as usize,
                _ => DEFAULT_LIMIT,
            };
            // A toast JS still lists shows its current entry; one leaving
            // shows the entry it had.
            let items = state
                .toasts
                .borrow()
                .manager
                .visible(limit)
                .map(|(id, entry, status)| {
                    let entry = entries.iter().find(|e| &e.id == id).unwrap_or(entry);
                    (entry.clone(), status)
                })
                .collect::<Vec<_>>();
            let (anchor, position) = placement(props, window.viewport_size());
            // The control's own box stays empty: the toasts are in the
            // window's overlay layer, at a corner of the window.
            div()
                .when(!items.is_empty(), |this| {
                    this.child(
                        deferred(
                            anchored().anchor(anchor).position(position).child(
                                props.styled(div(), cx).child(
                                    items
                                        .into_iter()
                                        .fold(
                                            ToastStack::new(
                                                part(props, "stack"),
                                                state.layout.clone(),
                                            ),
                                            |stack, (entry, status)| {
                                                stack.item(
                                                    part(props, format!("item-{}", entry.id)),
                                                    toast(state, props, events, entry, status, cx),
                                                )
                                            },
                                        )
                                        .placement(anchor)
                                        .focus_handle(state.focus.clone()),
                                ),
                            ),
                        )
                        .with_priority(PRIORITY),
                    )
                })
                .into_any_element()
        },
    )
    .documentation(
        "GPUI Base's toast stack, at a corner of the window. Each entry of `toasts` shows once, \
         when its `id` first appears; removing it lets it leave. GPUI Base owns the timing (an \
         entry's `duration` in milliseconds, none or 0 until dismissed), the stacking and its \
         motion: the stack expands on hover or focus, which pauses the timers. `dismiss` reports \
         a toast whose time ran out or whose close button was pressed (shown when `closeIcon` \
         names an SVG); remove its entry then. `rootClass` styles the toast region (give it a \
         width; its padding is the distance from the window's edges).",
    )
    .prop(
        "toasts",
        PropType::Json("{ id: string; title: string; description?: string; duration?: number }[]"),
    )
    .prop("placement", PropType::Enum(PLACEMENTS))
    .prop("limit", PropType::Number)
    .prop("closeIcon", PropType::String)
    .prop("closeLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("toastClass", PropType::String)
    .states(TOAST_STATES)
    .prop("toastStartingClass", PropType::String)
    .prop("toastEndingClass", PropType::String)
    .prop("titleClass", PropType::String)
    .prop("descriptionClass", PropType::String)
    .prop("closeClass", PropType::String)
    .states(CLOSE_STATES)
    .prop("closeIconClass", PropType::String)
    .event("dismiss", &[("id", PropType::String)])
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use gpui_kit::{ElementId, Role, TestAppContext, px};
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    /// Base's sonner motion: entry, then exit.
    const ENTER: Duration = Duration::from_millis(400);
    const EXIT: Duration = Duration::from_millis(200);

    fn part(fixture: &Fixture, name: &str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), name.to_owned().into())
    }

    fn dismissed(id: &str) -> (String, Value) {
        ("dismiss".into(), json!({ "id": id }))
    }

    /// Lets `time` pass and draws.
    fn wait(fixture: &Fixture, cx: &mut TestAppContext, time: Duration) {
        cx.executor().advance_clock(time);
        fixture.update(cx, |_, _| {});
    }

    fn shown(fixture: &Fixture, cx: &mut TestAppContext, id: &str) -> bool {
        let toast = part(fixture, &format!("toast-{id}"));
        fixture.update(cx, |window, _| window.try_find(toast).is_some())
    }

    fn stack(cx: &mut TestAppContext, props: Value) -> Fixture {
        let mut all = json!({
            "rootClass": "w_80 p_4",
            "closeIcon": "icons/close.svg",
            "closeClass": "size_6",
        });
        all.as_object_mut()
            .unwrap()
            .extend(props.as_object().unwrap().clone());
        Fixture::new(cx, "ToastStack", all)
    }

    #[gpui_kit::test]
    fn test_toast_stack_builder(cx: &mut TestAppContext) {
        let stack = Fixture::new(
            cx,
            "ToastStack",
            json!({
                "toasts": [
                    { "id": "a", "title": "Saved", "description": "All changes saved." },
                    { "id": "b", "title": "Sent", "duration": 4000 },
                ],
                "placement": "topRight",
                "limit": 3,
                "closeIcon": "icons/close.svg",
                "closeLabel": "Dismiss",
                "rootClass": "w_80 p_4",
                "toastClass": "p_4 rounded_lg border_1 border_border bg_background",
                "toastHoverClass": "border_ring",
                "toastStartingClass": "opacity_0",
                "toastEndingClass": "opacity_0",
                "titleClass": "text_sm font_semibold",
                "descriptionClass": "text_sm text_muted_foreground",
                "closeClass": "size_6 rounded_md",
                "closeHoverClass": "bg_accent",
                "closeActiveClass": "bg_muted",
                "closeFocusClass": "border_ring",
                "closeIconClass": "size_4",
            }),
        );
        // The toasts are in the overlay layer, not in the control's box.
        assert_eq!(stack.measure(cx).height, px(0.));
        wait(&stack, cx, ENTER);
        stack.update(cx, |window, _| {
            let newest = window.find(part(&stack, "toast-b"));
            assert_eq!(newest.role(), Some(Role::Alert));
            // At the window's top right, inside the region's padding.
            let bounds = newest.bounds();
            assert!(bounds.size.height > px(0.));
            assert_eq!(bounds.right(), px(800. - 16.));
            assert_eq!(bounds.top(), px(16.));
            assert!(window.try_find(part(&stack, "toast-a")).is_some());
            let close = window.find(part(&stack, "close-a"));
            assert_eq!(close.label(), Some("Dismiss"));
        });
    }

    #[gpui_kit::test]
    fn test_toast_stack_placement(cx: &mut TestAppContext) {
        let stack = stack(
            cx,
            json!({ "toasts": [{ "id": "a", "title": "Saved" }], "placement": "bottomLeft" }),
        );
        wait(&stack, cx, Duration::from_secs(1));
        stack.update(cx, |window, _| {
            let bounds = window.find(part(&stack, "toast-a")).bounds();
            assert_eq!(bounds.left(), px(16.));
            assert_eq!(bounds.bottom(), px(600. - 16.));
        });
    }

    #[gpui_kit::test]
    fn test_toast_stack_close(cx: &mut TestAppContext) {
        let stack = stack(cx, json!({ "toasts": [{ "id": "a", "title": "Saved" }] }));
        wait(&stack, cx, ENTER);
        stack.update(cx, |window, cx| window.click(part(&stack, "close-a"), cx));
        assert_eq!(stack.take_events(), vec![dismissed("a")]);
        // It stays for its exit, then leaves.
        assert!(shown(&stack, cx, "a"));
        wait(&stack, cx, EXIT);
        assert!(!shown(&stack, cx, "a"));
        assert!(stack.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_toast_stack_timeout(cx: &mut TestAppContext) {
        let stack = stack(
            cx,
            json!({ "toasts": [
                { "id": "a", "title": "Saved", "duration": 1000 },
                { "id": "b", "title": "Pinned" },
            ] }),
        );
        wait(&stack, cx, ENTER);
        wait(&stack, cx, Duration::from_millis(900));
        assert!(stack.take_events().is_empty());
        wait(&stack, cx, Duration::from_millis(100));
        assert_eq!(stack.take_events(), vec![dismissed("a")]);
        wait(&stack, cx, EXIT);
        assert!(!shown(&stack, cx, "a"));
        // Without a duration it stays until dismissed.
        wait(&stack, cx, Duration::from_secs(60));
        assert!(shown(&stack, cx, "b"));
        assert!(stack.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_toast_stack_hover_pauses(cx: &mut TestAppContext) {
        let stack = stack(
            cx,
            json!({ "toasts": [{ "id": "a", "title": "Saved", "duration": 1000 }] }),
        );
        wait(&stack, cx, ENTER);
        stack.update(cx, |window, cx| window.hover(part(&stack, "toast-a"), cx));
        wait(&stack, cx, Duration::from_secs(2));
        assert!(stack.take_events().is_empty());
        assert!(shown(&stack, cx, "a"));
    }

    #[gpui_kit::test]
    fn test_toast_stack_controlled(cx: &mut TestAppContext) {
        let stack = stack(cx, json!({ "toasts": [] }));
        assert!(!shown(&stack, cx, "a"));
        stack.set_prop(cx, "toasts", Some(json!([{ "id": "a", "title": "Saved" }])));
        assert!(shown(&stack, cx, "a"));
        // Showing it again with the same id is the same toast.
        stack.set_prop(
            cx,
            "toasts",
            Some(json!([
                { "id": "a", "title": "Saved" },
                { "id": "b", "title": "Sent" },
            ])),
        );
        assert!(shown(&stack, cx, "a") && shown(&stack, cx, "b"));
        // Removing an entry lets it leave, which is no dismissal.
        stack.set_prop(cx, "toasts", Some(json!([{ "id": "b", "title": "Sent" }])));
        wait(&stack, cx, EXIT);
        assert!(!shown(&stack, cx, "a") && shown(&stack, cx, "b"));
        assert!(stack.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_toast_stack_limit(cx: &mut TestAppContext) {
        let stack = stack(
            cx,
            json!({ "limit": 1, "toasts": [
                { "id": "a", "title": "Saved" },
                { "id": "b", "title": "Sent" },
            ] }),
        );
        // The newest shows; the older one waits for it to leave.
        assert!(!shown(&stack, cx, "a") && shown(&stack, cx, "b"));
        stack.set_prop(cx, "toasts", Some(json!([{ "id": "a", "title": "Saved" }])));
        wait(&stack, cx, EXIT);
        assert!(shown(&stack, cx, "a") && !shown(&stack, cx, "b"));
    }

    #[gpui_kit::test]
    fn test_toast_stack_keyboard(cx: &mut TestAppContext) {
        let stack = stack(cx, json!({ "toasts": [{ "id": "a", "title": "Saved" }] }));
        wait(&stack, cx, ENTER);
        // Tab traversal (Root's Tab is `focus_next`) reaches the stack, then
        // its close button.
        let close = part(&stack, "close-a");
        for _ in 0..2 {
            stack.update(cx, |window, cx| window.focus_next(cx));
        }
        stack.update(cx, |window, _| {
            assert_eq!(window.find(close).focused(), Some(true));
        });
        stack.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(stack.take_events(), vec![dismissed("a")]);
    }
}
