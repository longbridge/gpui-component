//! `Toggle` — a button that stays down.
//!
//! Controlled like every other base control: `pressed(...)` in,
//! `on_change(...)` out, carrying the value the script would have to flip
//! anyway. Nothing about it draws — a toggle with no styling is an invisible
//! hit target with a button role — so the pressed look is the script's, usually
//! through `.when(pressed, el => …)`.
//!
//! `pressed` rather than `checked` because that is what base calls it, and
//! because the distinction is real to a screen reader: this announces as a
//! button in a toggled state, not as a checkbox.

use std::rc::Rc;

use gpui::{AnyElement, SharedString, StyleRefinement};
use gpui_base::Toggle;

use crate::{
    engine::ShellRuntime,
    materialize::{
        Behavior, Children, StateStyles, dispatch_change, finish, tracked_focus, warn_ignored_key,
        warn_unhonoured_a11y, with_active_and_focus, with_hover, with_input_handlers,
    },
};

/// One toggle button. Controlled: `pressed(...)` in, `on_change(...)` out.
pub(in crate::materialize) fn toggle(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Toggle");
    // `Toggle::render` announces `Role::Button` itself; the pressed state
    // reaches a screen reader through `pressed`, not through `aria_selected`.
    warn_unhonoured_a11y(
        &behavior,
        "Toggle",
        &["track_focus", "tab_index", "tab_stop"],
    );
    let mut toggle = Toggle::new(SharedString::from(id))
        .pressed(behavior.pressed)
        .disabled(behavior.disabled);

    if let Some(focus) = tracked_focus(runtime, &behavior, "Toggle") {
        toggle = toggle.track_focus(&focus);
    }
    if let Some(index) = behavior.tab_index {
        toggle = toggle.tab_index(index);
    }
    if let Some(stop) = behavior.tab_stop {
        toggle = toggle.tab_stop(stop);
    }

    if let Some(label) = behavior.accessibility_label.clone() {
        toggle = toggle.accessibility_label(label);
    }

    if let Some(callback) = behavior.on_change {
        let runtime = Rc::downgrade(runtime);
        toggle = toggle.on_change(move |pressed, _, window, cx| {
            dispatch_change(&runtime, callback, pressed, window, cx);
        });
    }

    let toggle = with_hover(toggle, &states);
    let toggle = with_active_and_focus(toggle, &states);
    let toggle = with_input_handlers(toggle, &behavior, runtime);
    finish(toggle, refinement, children)
}
